# Rune Tools - MCP Servers as Plugins

This document is the single source of truth for how `rune-kit` (host) and
`rune-tools` (plugins) fit together. If you are new to this codebase, read
this top to bottom before writing any plugin code — it supersedes any
older per-plugin conventions you might find by example, since several
existing plugins (`rune-fs`, `rune-print`) predate parts of this design and
are mid-migration.

---

## 1. Core Architecture

The Rune Ecosystem separates **untrusted/variable capability code** from
**the trusted host**, using two execution models instead of one:

| Model | Runs as | Used for |
|---|---|---|
| **WASM plugin** | `.wasm` compiled to `wasm32-wasip1`, loaded by `rune-kit` into an Extism/Wasmtime sandbox | Logic that does real in-process compute (parsing, encoding, rendering, string/graph manipulation) and benefits from memory isolation |
| **Native sidecar** | A native OS binary, spawned as a child process by `rune-kit`, spoken to over stdio JSON | Logic whose real work is unavoidably native: linking a native library that can't target `wasm32-wasip1` (e.g. CUPS/WinSpool via `rust-printers`), or shelling out to external binaries (`ffmpeg`, `yt-dlp`, `gallery-dl`) where the WASM layer would just be a pass-through anyway |

**Rule of thumb:** if removing the WASM layer wouldn't remove any real
sandboxing benefit — because the dangerous part (subprocess exec, native
library calls) already happens on the host side of a host_fn — build a
native sidecar instead of a WASM plugin. Don't pay the WASM/host_fn
serialization tax for a pass-through.

Both models expose the **same three-verb contract** to `rune-kit`:
`info`, `list_tools`, `call_tool`. `rune-kit-core::McpRouter` treats them
uniformly via a `PluginInstance` enum (§4) — from the MCP client's point of
view, namespacing and dispatch behave identically regardless of which
model backs a given plugin.

---

## 2. Repository Layout

```text
rune-tools/                            # plugin workspace
├── Cargo.toml                         # [workspace] members + shared deps
└── plugins/
    ├── rune-filesystem/                # WASM-only (pure compute: fs walk, paging)
    ├── rune-time/                      # WASM-only (pure compute)
    ├── rune-fetch/                     # WASM-only (HTML→Markdown, network via host_fn)
    ├── rune-git/                       # candidate for native sidecar — review (see §7)
    ├── rune-media/                     # NATIVE SIDECAR (ffmpeg/yt-dlp/gallery-dl/aria2c/spotdl)
    ├── rune-print/                     # HYBRID — WASM renders, native sidecar dispatches
    ├── rune-memory/                    # WASM-only (pure compute)
    └── rune-sequentialthinking/        # WASM-only (pure compute)
```

Every plugin, regardless of model, follows the same internal module split
(§3) so that domain logic is always unit-testable with plain `cargo test`,
with no WASM toolchain or host process involved.

---

## 3. Standard Plugin Module Layout

```text
plugins/rune-<name>/
├── Cargo.toml                  # see §5/§6 for WASM-only vs sidecar config
├── src/
│   ├── lib.rs                  # WASM FFI boundary ONLY — gated #[cfg(target_arch = "wasm32")]
│   ├── bin/
│   │   └── native_sidecar.rs   # native entry point — ONLY present for sidecar plugins (§6)
│   ├── definitions.rs          # pure ToolDefinition declarations & JSON schemas
│   ├── operations.rs           # pure execution router & domain logic
│   └── types.rs                # request/response deserialization structs
└── tests/
    ├── contract_tests.rs       # schema/routing/type-rejection macro tests
    └── operations_tests.rs     # domain logic unit tests
```

**Non-negotiable rule:** `definitions.rs`, `operations.rs`, and `types.rs`
must never import `extism_pdk` or anything WASM-specific. This is what
lets `cargo test -p rune-<name>` run on your laptop with zero WASM
toolchain, and it's what lets a native sidecar `main.rs` reuse the exact
same `operations::execute_tool` your WASM `lib.rs` calls — there is only
ever one implementation of the domain logic, never a fork.

If a plugin needs I/O that differs between WASM and native execution
(subprocess exec, network calls), isolate that difference behind a single
function with two `#[cfg(...)]` bodies in `operations.rs`, the way
`run_binary_raw`/`get_config` already do in `rune-media`. Domain code
above that function should never branch on target arch.

---

## 4. Host Runtime (`rune-kit-core`)

### 4.1 `PluginInstance` — uniform dispatch over both models

```rust
// crates/rune-kit-core/src/lib.rs (or protocol.rs)
pub enum PluginInstance {
    Wasm(WasmPluginInstance),
    Native(NativeSidecar),
}

impl PluginInstance {
    pub fn list_tools(&mut self) -> Result<Vec<ToolDefinition>, RuntimeError> {
        match self {
            Self::Wasm(w) => w.list_tools(),
            Self::Native(n) => n.list_tools(),
        }
    }
    pub fn call_tool(&mut self, name: &str, args: Value) -> Result<Value, RuntimeError> {
        match self {
            Self::Wasm(w) => w.call_tool(name, args),
            Self::Native(n) => n.call_tool(name, args),
        }
    }
}
```

`McpRouter.instances: HashMap<String, PluginInstance>` — `tools/list` and
`tools/call` in `protocol.rs` are written once against this enum and never
need to know which model backs a given namespace.

### 4.2 `WasmPluginInstance` (existing)

Loads a `.wasm` file into an Extism `Plugin`, calling `mcp_info` /
`mcp_list_tools` / `mcp_call_tool` as guest exports. Unchanged from the
original design — see `runtime.rs`.

### 4.3 `NativeSidecar` (new)

Spawns a native binary as a persistent child process, talking newline-
delimited JSON over stdin/stdout using the shared `rune-sidecar` protocol.

```rust
// crates/rune-kit-core/src/native_sidecar.rs
pub struct NativeSidecar {
    child: std::process::Child,
    stdin: std::process::ChildStdin,
    stdout: std::io::BufReader<std::process::ChildStdout>,
}

impl NativeSidecar {
    pub fn spawn(binary_path: &str) -> Result<Self, RuntimeError> { /* ... */ }
    fn call(&mut self, method: &str, params: Value) -> Result<Value, RuntimeError> { /* ... */ }
    pub fn list_tools(&mut self) -> Result<Vec<ToolDefinition>, RuntimeError> { /* calls "list_tools" */ }
    pub fn call_tool(&mut self, name: &str, arguments: Value) -> Result<Value, RuntimeError> { /* calls "call_tool" */ }
}
```

Spawn `stderr` with `Stdio::inherit()` so sidecar logs surface directly in
`rune-kit`'s own output rather than being swallowed.

### 4.4 Host capabilities — typed, not generic exec

`host_cmd_exec(program, args)` as a **general-purpose, plugin-chosen**
capability is deprecated for new work. It is a standing security hole:
any WASM guest can ask the host to run any program with any arguments,
and any code path that further interpolates guest-supplied strings into a
*shell or script* (not just argv) is a command-injection vector — this is
exactly how the PowerShell-based printer socket fallback in `rune-print`
became exploitable.

New host functions must be **typed and single-purpose**:

```rust
fn host_tcp_send(req: TcpSendRequest) -> TcpSendResponse;     // structured fields, never a script string
fn host_http_request(req: HttpRequest) -> HttpResponse;       // replaces ad-hoc curl shell-outs
fn host_exec(req: ExecRequest) -> ExecResponse;                // ONLY if a WASM plugin genuinely still
                                                                 // needs subprocess exec; program name is
                                                                 // checked against an allowlist declared in
                                                                 // the plugin's manifest before spawning
```

`host_exec`'s allowlist is declared per plugin (see §4.5) — a plugin
manifest that doesn't declare `exec.allowed_binaries` gets no exec
capability at all, full stop.

Native sidecars do **not** need `host_exec` — they call
`std::process::Command` directly, since they're already unsandboxed
native processes you built and ship yourself, not arbitrary loaded WASM.
Moving a plugin from WASM+`host_cmd_exec` to a native sidecar is itself a
security simplification: it removes that plugin's calls from the generic
exec surface entirely.

### 4.5 Capability manifest (default-deny)

Every plugin ships a small manifest read at install time by
`PackageManager`, declaring exactly what it needs. `rune-kit` grants only
what's declared — no more `with_allowed_host("*")` by default.

```toml
# plugins/rune-<name>/plugin.toml
[capabilities]
network_hosts = []                # explicit hostnames/IPs, templated from config where needed
filesystem = { mode = "scoped", root_param = "allowed_dir" }
exec = { allowed_binaries = [] }  # empty = no host_exec capability
```

---

## 5. Building a WASM-only Plugin

Use this when the plugin's logic is real in-process compute with no
native library dependency and no external binary to shell out to
(`rune-filesystem`, `rune-time`, `rune-memory`, `rune-sequentialthinking`
are the reference examples).

### 5.1 Register in workspace `Cargo.toml`

```toml
[workspace]
members = [
    "crates/rune-pdk",
    "crates/rune-sidecar",
    "plugins/rune-filesystem",
    "plugins/rune-<name>",
]
```

### 5.2 `plugins/rune-<name>/Cargo.toml`

```toml
[package]
name = "rune-<name>"
description = "MCP plugin for <functionality>"
version = "0.1.0"
edition.workspace = true
license.workspace = true

[lib]
crate-type = ["cdylib", "rlib"]   # rlib is required even for WASM-only plugins — it's what lets
                                   # tests/*.rs link against definitions/operations natively

[dependencies]
rune-pdk = { path = "../../crates/rune-pdk" }
serde.workspace = true
serde_json.workspace = true

[target.'cfg(target_arch = "wasm32")'.dependencies]
extism-pdk.workspace = true

[dev-dependencies]
tempfile.workspace = true
```

### 5.3 `src/types.rs` — request/response structs

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalculationPayload {
    pub expression: String,
    #[serde(default)]
    pub precision: Option<usize>,
}
```

### 5.4 `src/definitions.rs` — tool schemas

Every parameter needs a `description` and a `type`; tool and parameter
names are `snake_case`. If a tool can return a large or unbounded result
(file contents, byte streams, long lists), it **must** use the paging
envelope from §7 rather than returning everything in one call.

```rust
use rune_pdk::ToolDefinition;
use serde_json::json;

pub fn tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "calculate".to_string(),
            description: "Evaluates a mathematical expression and returns the formatted result.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "expression": { "type": "string", "description": "Expression to evaluate (e.g. '2 + 2')" },
                    "precision": { "type": "number", "description": "Optional decimal precision for output" }
                },
                "required": ["expression"]
            }),
        }
    ]
}
```

### 5.5 `src/operations.rs` — router + domain logic

```rust
use crate::types::CalculationPayload;
use rune_pdk::ToolCallRequest;
use serde_json::{Value, json};

pub fn execute_tool(request: ToolCallRequest) -> Result<Value, String> {
    match request.name.as_str() {
        "calculate" => {
            let expr = request.arguments["expression"]
                .as_str()
                .ok_or_else(|| "Missing 'expression' parameter".to_string())?;
            let precision = request.arguments.get("precision").and_then(|v| v.as_u64()).map(|v| v as usize).unwrap_or(2);
            let result_val = evaluate_expression(expr)?;
            Ok(json!({ "expression": expr, "result": format!("{:.precision$}", result_val, precision = precision) }))
        }
        unknown => Err(format!("Unknown tool: {}", unknown)),
    }
}

fn evaluate_expression(expr: &str) -> Result<f64, String> {
    if expr.trim().is_empty() {
        return Err("Expression cannot be empty".to_string());
    }
    Ok(42.0)
}
```

**Never** materialize an unbounded amount of data in one call here (e.g.
`fs::read_to_string` on an arbitrarily large file, or a full multi-page
raster buffer) — see §7 for why and what to do instead.

### 5.6 `src/lib.rs` — WASM FFI boundary only

```rust
pub mod definitions;
pub mod operations;
pub mod types;

#[cfg(target_arch = "wasm32")]
use rune_pdk::ToolCallRequest;
#[cfg(target_arch = "wasm32")]
use serde_json::json;

#[cfg(target_arch = "wasm32")]
#[extism_pdk::plugin_fn]
pub fn mcp_info(_: ()) -> extism_pdk::FnResult<String> {
    let info = json!({
        "name": env!("CARGO_PKG_NAME"),
        "version": env!("CARGO_PKG_VERSION"),
        "description": option_env!("CARGO_PKG_DESCRIPTION")
    });
    Ok(serde_json::to_string(&info)?)
}

#[cfg(target_arch = "wasm32")]
#[extism_pdk::plugin_fn]
pub fn mcp_list_tools(_: ()) -> extism_pdk::FnResult<String> {
    Ok(serde_json::to_string(&definitions::tool_definitions())?)
}

#[cfg(target_arch = "wasm32")]
#[extism_pdk::plugin_fn]
pub fn mcp_call_tool(input: String) -> extism_pdk::FnResult<String> {
    let request: ToolCallRequest = serde_json::from_str(&input)?;
    let result = operations::execute_tool(request);
    let output = match result {
        Ok(val) => json!({ "status": "success", "result": val }),
        Err(err) => json!({ "status": "error", "error": err }),
    };
    Ok(serde_json::to_string(&output)?)
}
```

Nothing in this file should ever be reachable when compiling natively —
if you find yourself wanting to call something from `lib.rs` in a native
test, that logic belongs in `operations.rs` instead.

---

## 6. Building a Native Sidecar Plugin

Use this when the plugin's core job is either (a) linking a native
library that cannot target `wasm32-wasip1` (e.g. `rust-printers` needing
CUPS/WinSpool), or (b) shelling out to external binaries as its dominant
behavior (`rune-media` → `ffmpeg`/`yt-dlp`/`gallery-dl`/`aria2c`/`spotdl`).

Everything in §3 and §5.3–§5.5 (`types.rs`, `definitions.rs`,
`operations.rs`) is written **identically** to a WASM plugin — a sidecar
is a different entry point over the same domain logic, not a different
architecture underneath.

### 6.1 `Cargo.toml` — add the native `[[bin]]` target

```toml
[package]
name = "rune-<name>"
version = "0.1.0"
edition.workspace = true
license.workspace = true

[lib]
crate-type = ["cdylib", "rlib"]

[[bin]]
name = "rune-<name>-native"
path = "src/bin/native_sidecar.rs"

[dependencies]
rune-pdk = { path = "../../crates/rune-pdk" }
serde.workspace = true
serde_json.workspace = true

[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
rune-sidecar = { path = "../../crates/rune-sidecar" }

[target.'cfg(target_arch = "wasm32")'.dependencies]
extism-pdk.workspace = true   # omit entirely if the plugin has NO wasm build at all
```

If the plugin has no reason to ever run as WASM (true for `rune-media`),
you can drop the `cdylib` target and the wasm dependency block entirely —
keep `rlib` only, and skip `lib.rs`'s `#[cfg(target_arch = "wasm32")]`
handlers.

### 6.2 `src/bin/native_sidecar.rs` — the entire native entry point

```rust
use rune_<name>::{definitions, operations};
use rune_pdk::ToolCallRequest;
use rune_sidecar::{run_stdio, SidecarHandler};
use serde_json::{json, Value};

struct Handler;

impl SidecarHandler for Handler {
    fn info(&self) -> Value {
        json!({
            "name": env!("CARGO_PKG_NAME"),
            "version": env!("CARGO_PKG_VERSION"),
            "description": option_env!("CARGO_PKG_DESCRIPTION")
        })
    }
    fn list_tools(&self) -> Vec<rune_pdk::ToolDefinition> {
        definitions::tool_definitions()
    }
    fn call_tool(&self, req: ToolCallRequest) -> Result<Value, String> {
        operations::execute_tool(req)
    }
}

fn main() -> std::io::Result<()> {
    run_stdio(Handler)
}
```

`rune-sidecar::run_stdio` (in `crates/rune-sidecar`) implements the
newline-delimited JSON loop (`info` / `list_tools` / `call_tool` methods)
that `NativeSidecar` on the host side speaks to — it exists once, shared
by every sidecar plugin, so no plugin hand-rolls its own stdio protocol.

### 6.3 Registering the sidecar with `rune-kit`

`PackageManager`/plugin config marks the plugin's execution model (native
binary path vs `.wasm` path) in its lockfile entry; `McpRouter::register`
constructs `PluginInstance::Native(NativeSidecar::spawn(path)?)` instead
of `PluginInstance::Wasm(...)` for that namespace. Namespacing, tool
listing, and dispatch in `protocol.rs` are otherwise unchanged (§4.1).

### 6.4 Native subprocess calls stay in `operations.rs`, not the sidecar entry point

Keep `run_binary`/`Command::new` calls inside `operations.rs`'s
`#[cfg(not(target_arch = "wasm32"))]` branch, exactly as `rune-media`
already does — `src/bin/native_sidecar.rs` should never call
`std::process::Command` directly. This keeps the domain logic testable
via `cargo test -p rune-<name>` without spawning real subprocesses in
tests that don't need to.

---

## 7. Data & Memory Handling — the Paging Rule

**No single `call_tool` response may require materializing more than a
bounded amount of data in memory before returning**, regardless of
execution model. This is not a style preference — it's what previously
caused large files to appear truncated: a WASM guest has a finite linear
memory ceiling, and a response has to exist there in full (often twice —
once as raw bytes, again as serialized JSON) before it can cross back to
the host. Native sidecars don't have the WASM ceiling, but the same rule
still keeps stdio messages small and responsive, so apply it universally.

Any tool whose result could be large or unbounded (file contents, byte
streams, directory trees, long lists, raster/media payloads) must use a
paging envelope instead of returning everything at once:

```rust
// crates/rune-pdk — shared by every plugin, never redefined locally
#[derive(Serialize, Deserialize)]
pub struct Page<T> {
    pub items: T,
    pub cursor: Option<String>,   // opaque continuation token
    pub has_more: bool,
}
```

Concretely: text reads page by line range (`lineOffset`/`lineLimit`,
`hasMore`/`nextLineOffset`); binary/media reads page by byte offset
(`offset`/`length`, `paging.hasMore`/`paging.nextOffset`), capped at a
fixed per-call ceiling (e.g. `const MAX_CHUNK_BYTES: usize = 512 * 1024;`
declared once per plugin, next to the tool that needs it). The calling
agent loops on `hasMore` until it's `false`.

---

## 8. Naming & Consistency Rules

Package drift (a plugin's `Cargo.toml` `name`, its folder name, and its
own header-comment path disagreeing with each other) has already caused
real confusion in this codebase. When renaming or creating a plugin,
these four locations must all agree, checked in this order:

1. Folder name: `plugins/rune-<name>/`
2. `Cargo.toml` → `[package] name = "rune-<name>"`
3. Root workspace `Cargo.toml` → `members = [..., "plugins/rune-<name>"]`
4. Header comment in every source file: `// plugins/rune-<name>/src/<file>.rs`

If the plugin has a native sidecar binary, its `[[bin]] name` should be
`rune-<name>-native` — consistent, so `rune-kit`'s plugin loader can
derive the expected binary name from the plugin name alone rather than
needing it configured separately per plugin.

---

## 9. Testing Architecture

Both plugin models are tested identically, natively, with no WASM
toolchain involved — this is the entire point of keeping `lib.rs` (or
`bin/native_sidecar.rs`) as a thin boundary over pure `operations.rs`.

### 9.1 Contract tests (`tests/contract_tests.rs`)

```rust
use rune_<name>::{definitions::tool_definitions, operations::execute_tool};
use rune_pdk::test_plugin_contract;

test_plugin_contract!(tool_definitions, execute_tool);
```

The shared `test_plugin_contract!` macro (in `rune-pdk`) automatically
checks: every tool in `definitions()` is routable in `execute_tool`;
every required parameter, when omitted, produces an `Err`; and malformed
argument types don't panic.

### 9.2 Operations tests (`tests/operations_tests.rs`)

```rust
use rune_<name>::operations::execute_tool;
use rune_pdk::ToolCallRequest;
use serde_json::json;

#[test]
fn test_calculate_success() {
    let req = ToolCallRequest { name: "calculate".to_string(), arguments: json!({ "expression": "10 + 5", "precision": 2 }) };
    let res = execute_tool(req).unwrap();
    assert_eq!(res["result"], "42.00");
}

#[test]
fn test_calculate_empty_expression() {
    let req = ToolCallRequest { name: "calculate".to_string(), arguments: json!({ "expression": "" }) };
    let res = execute_tool(req);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Expression cannot be empty"));
}
```

For sidecar plugins, add an additional smoke test that spawns the actual
`rune-<name>-native` binary and round-trips one `list_tools`/`call_tool`
message over real stdio — this is the one thing pure `operations.rs`
tests can't catch (protocol framing bugs).

---

## 10. Compilation & Testing Commands

```bash
# Single plugin, if it needs env vars/tokens from its own .env for tests
dotenvx run -f ./plugins/rune-<name>/.env -- cargo test -p rune-<name> --all-features

# Whole workspace
cargo test --workspace --all-features

# Single plugin, both targets
cargo xtask build rune-<name>

# Just one side, if you only need it
cargo xtask build rune-<name> --wasm-only
cargo xtask build rune-<name> --native-only

# Whole workspace, both targets 
cargo xtask build-all
```

---

## 11. Decision Checklist: WASM Plugin vs Native Sidecar

Walk through in order; stop at the first match.

1. **Does the plugin need a native library/binding that cannot compile to
   `wasm32-wasip1`** (native FFI, OS-specific APIs like CUPS/WinSpool)?
   → **Native sidecar.**
2. **Is the plugin's dominant behavior shelling out to external CLI
   binaries**, with little real compute happening in the plugin itself
   (`ffmpeg`, `yt-dlp`, `git`)? → **Native sidecar** — the WASM layer would
   only be relaying arguments through `host_cmd_exec` anyway, which is
   both unnecessary overhead and the exact generic-exec surface §4.4
   deprecates.
3. **Does most of the value come from in-process pure compute** (parsing,
   encoding, filesystem traversal, graph/state logic) with occasional,
   narrow host calls? → **WASM plugin.**
4. **Mixed** — real in-process compute *and* a native-only dispatch step
   (e.g. `rune-print`: PDF/PWG rasterization is pure Rust compute; final
   job dispatch needs CUPS/WinSpool)? → **Hybrid**: keep the compute-heavy
   part as a WASM plugin, add a narrow typed host_fn (§4.4) or a small
   sidecar for just the native dispatch step. Don't move the whole plugin
   to native just because one operation needs it.

Existing plugins flagged **"review"** in §2 (`rune-git`) haven't been
run through this checklist yet — do that before assuming their current
model is correct, rather than copying their structure by default.