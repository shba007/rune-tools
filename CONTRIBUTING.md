# Rune Ecosystem — Architecture & Contributor Guide

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

Both models expose the same contract to `rune-kit`: `info`, plus a
list+read/call/get pair for each MCP primitive a plugin actually supports
(`list_tools`/`call_tool`, `list_resources`/`read_resource`,
`list_prompts`/`get_prompt` — see §2). `rune-kit-core::McpRouter` treats
every plugin uniformly via a `PluginInstance` enum (§5) — from the MCP
client's point of view, namespacing and dispatch behave identically
regardless of which model backs a given plugin, and regardless of which
subset of primitives it implements.

---

## 2. MCP Primitives: Tools, Resources, and Prompts

MCP defines three distinct primitives. Mixing them up produces a plugin
that technically works but confuses every client that talks to it. Decide
which one you're building before writing `definitions.rs`:

| Primitive | Who decides to use it | Shape | Rune verbs |
|---|---|---|---|
| **Tool** | The model, autonomously, mid-conversation, based on what the task needs | An action/function with typed args and a return value | `list_tools` / `call_tool` |
| **Resource** | The user or client application — attached explicitly or browsed, not invoked by the model turn-by-turn | Addressable data at a URI — read-only context, not an action | `list_resources` / `read_resource` |
| **Prompt** | The user, explicitly (a slash command, a menu pick) — never triggered autonomously by the model | An argument-templated message sequence that seeds a conversation | `list_prompts` / `get_prompt` |

A fast test that resolves most ambiguity: **would it make sense for the
model to call this five times in a row while reasoning?** If yes, it's a
tool (`search_files`, `git_diff`). If the honest answer is "no, this is
something you'd hand the model once, as background" (the current contents
of a config file, a directory listing meant to be attached rather than
queried) — resource. If it's a full canned interaction the user picks
deliberately ("write me a commit message from this diff," "draft an
incident summary") rather than something the model reaches for
mid-reasoning — prompt.

`rune-git`'s README already documents a prompt-shaped interaction
("formulate a semantic commit message... commit the changes") that's
currently implemented as a sequence of tool calls the *user* has to spell
out in prose every time — a good first candidate for an actual
`prompts/get` entry once §6/§8 land, rather than only reachable by
re-describing the whole request each time.

**Mirrored-type gotcha:** like `ToolDefinition` already does today,
`ResourceDefinition`/`PromptDefinition` need to exist as matching struct
definitions in *two* crates — once in `rune-pdk` (what a plugin's
`definitions.rs` authors against) and once in `rune-kit-core::manifest`
(what the host deserializes into). These aren't the same Rust type shared
across a dependency edge — plugin crates and the host crate are separate
compilation targets that don't depend on each other — they're two
independently-defined structs kept in sync by matching JSON field names.
This isn't new with resources/prompts, it's the existing `ToolDefinition`
pattern; just don't be surprised to find it duplicated when you go
looking for "the" definition.

---

## 3. Repository Layout

```text
rune-tools/                            # plugin workspace
├── .cargo/
│   └── config.toml                    # [alias] xtask — see §11
├── Cargo.toml                         # [workspace] members + shared deps
├── xtask/                             # build/test orchestrator — see §11
└── plugins/
    ├── rune-filesystem/                # WASM-only (pure compute: fs walk, paging)
    ├── rune-time/                      # WASM-only (pure compute)
    ├── rune-fetch/                     # WASM-only (HTML→Markdown, network via host_fn)
    ├── rune-git/                       # candidate for native sidecar — review (see §14)
    ├── rune-audio/                     # NATIVE SIDECAR (yt-dlp/ffmpeg/spotdl)
    ├── rune-video/                     # NATIVE SIDECAR (yt-dlp/ffmpeg/streamlink)
    ├── rune-image/                     # NATIVE SIDECAR (gallery-dl and similar)
    ├── rune-email/                     # execution model unconfirmed — classify via §14 before copying its structure
    ├── rune-browser/                   # new, currently disabled in workspace members — classify via §14
    ├── rune-print/                     # HYBRID — WASM renders, native sidecar dispatches (currently disabled, mid-migration)
    ├── rune-memory/                    # WASM-only, pure compute (currently disabled, mid-migration)
    └── rune-sequential-thinking/       # WASM-only, pure compute (currently disabled, mid-migration)
```

"Currently disabled" plugins are commented out of the root `Cargo.toml`
`[workspace] members` list, not deleted — they exist on disk but the
workspace won't build or test them until they're re-enabled. Check the
actual `members` list before assuming a plugin is active.

Every plugin, regardless of model, follows the same internal module split
(§4) so that domain logic is always unit-testable with plain `cargo test`,
with no WASM toolchain or host process involved.

---

## 4. Standard Plugin Module Layout

```text
plugins/rune-<name>/
├── Cargo.toml                  # see §7/§8 for WASM-only vs sidecar config
├── .env                        # always present, even if empty — see §12
├── src/
│   ├── lib.rs                  # WASM FFI boundary ONLY — gated #[cfg(target_arch = "wasm32")]
│   ├── bin/
│   │   └── native_sidecar.rs   # native entry point — ONLY present for sidecar plugins (§8)
│   ├── definitions.rs          # pure tool/resource/prompt schemas — see note below
│   ├── operations.rs           # pure tool/resource/prompt execution — see note below
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
`run_binary_raw`/`get_config` already do in `rune-audio`. Domain code
above that function should never branch on target arch.

**Resources and prompts follow the same pure-Rust rule as tools.**
`definitions.rs`/`operations.rs` gain `resource_definitions()`/
`read_resource()` and `prompt_definitions()`/`get_prompt()` alongside the
existing `tool_definitions()`/`execute_tool()` — same reasoning, same
file, one implementation reachable from both `lib.rs` and
`native_sidecar.rs`. A plugin with no resources or prompts simply doesn't
define those functions — §5.7 covers how the host detects that
per-plugin, per-execution-model. If a plugin's resource or prompt surface
grows large enough that cramming it into `definitions.rs`/`operations.rs`
hurts readability, split into `resources.rs`/`prompts.rs` with the same
internal shape — a per-plugin readability call, not a repo-wide
requirement.

**Every plugin gets a `.env` file, even an empty one.** This is a repo-wide
convention, not conditional on whether that particular plugin currently
needs secrets — see §12 for why this matters for testing.

---

## 5. Host Runtime (`rune-kit-core`)

### 5.1 `PluginInstance` — uniform dispatch over all three primitives

```rust
// crates/rune-kit-core/src/lib.rs (or protocol.rs)
pub enum PluginInstance {
    Wasm(WasmPluginInstance),
    Native(NativeSidecar),
}

impl PluginInstance {
    pub fn list_tools(&mut self) -> Result<Vec<ToolDefinition>, RuntimeError> { /* ... */ }
    pub fn call_tool(&mut self, name: &str, args: Value) -> Result<Value, RuntimeError> { /* ... */ }

    pub fn list_resources(&mut self) -> Result<Vec<ResourceDefinition>, RuntimeError> { /* ... */ }
    pub fn read_resource(&mut self, uri: &str) -> Result<Value, RuntimeError> { /* ... */ }

    pub fn list_prompts(&mut self) -> Result<Vec<PromptDefinition>, RuntimeError> { /* ... */ }
    pub fn get_prompt(&mut self, name: &str, args: Value) -> Result<Value, RuntimeError> { /* ... */ }

    /// Cached at load time (§5.7) — which primitives this specific
    /// instance actually supports, so aggregation passes in protocol.rs
    /// don't call-and-discard-an-error on every `resources/list`/
    /// `prompts/list` request against plugins that have neither.
    pub fn capabilities(&self) -> PluginCapabilities { /* ... */ }
}
```

`McpRouter.instances: HashMap<String, PluginInstance>` — `tools/list`,
`resources/list`, `prompts/list`, and their `*/call`, `*/read`, `*/get`
counterparts in `protocol.rs` are written once against this enum and
never need to know which model backs a given namespace.

### 5.2 `WasmPluginInstance` (existing)

Loads a `.wasm` file into an Extism `Plugin`, calling `mcp_info` /
`mcp_list_tools` / `mcp_call_tool` — and, when present, `mcp_list_resources`
/ `mcp_read_resource` / `mcp_list_prompts` / `mcp_get_prompt` — as guest
exports. Unchanged from the original tools-only design — see `runtime.rs`.

### 5.3 `NativeSidecar` (existing)

Spawns a native binary as a persistent child process, talking newline-
delimited JSON over stdin/stdout using the shared `rune-sidecar` protocol.

```rust
// crates/rune-kit-core/src/runtime.rs
pub struct NativeSidecar {
    child: std::process::Child,
    stdin: std::process::ChildStdin,
    stdout: std::io::BufReader<std::process::ChildStdout>,
    pub name: String,
}

impl NativeSidecar {
    pub fn spawn(name: &str, binary_path: impl AsRef<std::path::Path>) -> Result<Self, RuntimeError> { /* ... */ }
    fn call(&mut self, method: &str, params: Option<Value>) -> Result<Value, RuntimeError> { /* ... */ }
    pub fn get_info(&mut self) -> Result<PluginInfo, RuntimeError> { /* calls "info" */ }
    pub fn list_tools(&mut self) -> Result<Vec<ToolDefinition>, RuntimeError> { /* calls "list_tools" */ }
    pub fn call_tool(&mut self, name: &str, arguments: Value) -> Result<Value, RuntimeError> { /* calls "call_tool" */ }
    pub fn list_resources(&mut self) -> Result<Vec<ResourceDefinition>, RuntimeError> { /* calls "list_resources" */ }
    pub fn read_resource(&mut self, uri: &str) -> Result<Value, RuntimeError> { /* calls "read_resource" */ }
    pub fn list_prompts(&mut self) -> Result<Vec<PromptDefinition>, RuntimeError> { /* calls "list_prompts" */ }
    pub fn get_prompt(&mut self, name: &str, arguments: Value) -> Result<Value, RuntimeError> { /* calls "get_prompt" */ }
}
```

Spawn `stderr` with `Stdio::inherit()` so sidecar logs surface directly in
`rune-kit`'s own output rather than being swallowed. `Drop` kills the
child process so a crashed or forgotten `PluginInstance` doesn't leak a
process.

`rune-kit` sends the plain `{"method": "..."}` form shown above, but the
sidecar side (`rune-sidecar::run_stdio`, §8.2) understands a superset of
this: it also accepts JSON-RPC-style requests carrying an `"id"` and
`"params"` object, and aliases each method to its MCP-style name
(`"list_tools"` / `"mcp_list_tools"` / `"tools/list"` all route the same
way — same aliasing applies to the resource/prompt methods). That
richness exists so a sidecar binary can be invoked and inspected directly
from a terminal or a different MCP host, not just by `rune-kit` —
`rune-kit` itself only ever needs the simple form.

A panic inside any call — tool, resource, or prompt — must not take down
the whole persistent process. `run_stdio` wraps every handler dispatch in
`std::panic::catch_unwind` and returns an error response instead of
letting the panic unwind through `main()`. Unlike a WASM trap (which
Extism isolates per call automatically), an unguarded panic in a native
sidecar would kill every future call to that plugin until `rune-kit`
notices the process exited and respawns it.

### 5.4 Host capabilities — typed, not generic exec

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

`host_exec`'s allowlist is declared per plugin (see §5.5) — a plugin
manifest that doesn't declare `exec.allowed_binaries` gets no exec
capability at all, full stop.

Native sidecars do **not** need `host_exec` — they call
`std::process::Command` directly, since they're already unsandboxed
native processes you built and ship yourself, not arbitrary loaded WASM.
Moving a plugin from WASM+`host_cmd_exec` to a native sidecar is itself a
security simplification: it removes that plugin's calls from the generic
exec surface entirely.

### 5.5 Capability manifest (default-deny)

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

**This manifest governs resource reads exactly as it governs tool calls.**
`read_resource` is a second verb into the same sandboxed plugin, not a
side channel around its capability scope — see §6.

### 5.6 Installing native binaries

`PackageManager::install` isn't wasm-only. The registry (§13) can list a
`"native"` map of target-triple → archive URL alongside (or instead of)
a plain `"url"` for the wasm build. Installation:

1. Resolves the host's own target triple from `std::env::consts::{OS, ARCH}`.
2. Prefers the wasm build by default; `rune install --native <name>`
   prefers native, falling back to wasm with a warning if no native build
   exists for this host, and erroring only if *neither* exists.
3. For a native install, extracts the downloaded `.tar.gz`/`.zip` (expected
   to contain exactly one file — the sidecar binary), sets the executable
   bit on Unix, and probes it via `NativeSidecar::get_info()` the same way
   a wasm install probes via `WasmPluginInstance::get_info()` — same trust
   model on both paths, not an approximation for one of them.
4. Records `execution: wasm | native` on the lockfile entry
   (`InstalledPlugin`), defaulted via `#[serde(default)]` so lockfiles
   written before this existed keep deserializing as `wasm`.

A plugin that's native-only (no `cdylib` target at all, no `"url"` key in
the registry) can currently only be installed via `--native` or by a
target with no wasm entry to fall back to — there's no ambiguity to
resolve in that case, since there's only one option.

### 5.7 Resource & prompt capability probing

Resources and prompts are optional per plugin, and wasm/native use
genuinely different mechanisms to signal "I don't have any" — this isn't
one abstraction wearing two hats, be aware of the asymmetry:

- **WASM**: a guest either exports `mcp_list_resources`/`mcp_list_prompts`
  or it doesn't — there's no such thing as a default wasm export. The
  install-time probe (§5.6, step 3) attempts each call once; a
  function-not-found failure is recorded as "unsupported"
  (`PluginCapabilities { resources: false, ... }`), while any *other*
  error is a real bug in the plugin and gets surfaced, not silenced —
  the same distinction `mcp_info` probing already makes (`[warn] Failed
  to call mcp_info on '<name>': <err>`).
- **Native sidecar**: `SidecarHandler` gives `list_resources`/
  `read_resource`/`list_prompts`/`get_prompt` default trait
  implementations — empty list / a clear "not supported by this plugin"
  error. A plugin author simply never overrides them unless the plugin
  has resources or prompts to expose:

  ```rust
  pub trait SidecarHandler {
      fn info(&self) -> Value;
      fn list_tools(&self) -> Vec<ToolDefinition>;
      fn call_tool(&self, req: ToolCallRequest) -> Result<Value, String>;

      // Default: unsupported. Override only if the plugin has them.
      fn list_resources(&self) -> Vec<ResourceDefinition> { Vec::new() }
      fn read_resource(&self, _uri: &str) -> Result<Value, String> {
          Err("resources not supported by this plugin".to_string())
      }
      fn list_prompts(&self) -> Vec<PromptDefinition> { Vec::new() }
      fn get_prompt(&self, _name: &str, _arguments: Value) -> Result<Value, String> {
          Err("prompts not supported by this plugin".to_string())
      }
  }
  ```

Either way, the result lands in the same `PluginCapabilities` struct on
`PluginInstance`, computed once at load time rather than re-derived on
every `resources/list`/`prompts/list` aggregation pass.

**`initialize`'s declared capabilities are unconditional, not derived from
the currently-loaded plugin set.** `protocol.rs` always declares
`"resources": { "listChanged": false }` and `"prompts": {
"listChanged": false }` once these verbs exist in the router, regardless
of whether any given moment's registered plugins actually have any — the
same way `"tools": { "listChanged": false }` is already declared
unconditionally today. A client calling `resources/list` against a
plugin set with zero resources just gets `{ "resources": [] }`, which is
valid MCP, not a capability lie. (`listChanged: false` is honest, not a
placeholder to fix later — dynamic list-changed notifications aren't
implemented for any primitive yet. `subscribe` is omitted entirely for
the same reason: don't advertise a capability nothing implements.)

---

## 6. Resource URI Convention

MCP resource identifiers are URIs, not bare names, so they need their own
namespacing convention — the URI-shaped equivalent of the `plugin__tool`
separator tools already use:

```
rune://<plugin-namespace>/<plugin-defined-path>
```

`resources/list` aggregation in `protocol.rs` prefixes whatever
plugin-local URI a plugin returns from `list_resources()` with
`rune://<namespace>/`. `resources/read` does the reverse: parses the
`rune://` scheme, takes the namespace as the host segment, looks up that
`PluginInstance`, and passes everything after it to `read_resource()`
**unqualified**. Plugins never construct or parse the `rune://` prefix
themselves — same separation of concerns as tool namespacing (§10),
where a plugin only ever knows its own bare tool name, never the
`plugin__tool` form the client sees.

**Worked example:** `rune-filesystem` returns a bare resource identifier
like `D:/workspace/notes.md` from `list_resources()`. The client sees
`rune://filesystem/D:/workspace/notes.md`. Calling `resources/read` on
that URI strips the prefix back down to `D:/workspace/notes.md` before it
ever reaches `rune-filesystem`'s `read_resource()` — which still goes
through the exact same `allowed_dir` capability scoping (§5.5) that
`read_file` does. **Resources are not a way to bypass capability
scoping** — they're a second verb into the same sandboxed plugin, subject
to the same manifest, full stop.

**Resource templates** (`resources/templates/list`) work identically for
plugins whose resource space is naturally parameterized rather than
enumerable — e.g. `rune-git` exposing `rune://git/log/{ref}` rather than
listing every possible ref up front. A plugin returns both concrete
resources and templates from the same listing call; `protocol.rs` routes
them to the appropriate MCP list endpoint on the client side.

---

## 7. Building a WASM-only Plugin

Use this when the plugin's logic is real in-process compute with no
native library dependency and no external binary to shell out to
(`rune-filesystem`, `rune-time`, `rune-memory`, `rune-sequential-thinking`
are the reference examples).

### 7.1 Register in workspace `Cargo.toml`

```toml
[workspace]
members = [
    "crates/rune-pdk",
    "crates/rune-sidecar",
    "plugins/rune-filesystem",
    "plugins/rune-<name>",
    "xtask",
]
```

### 7.2 `plugins/rune-<name>/Cargo.toml`

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

### 7.3 `src/types.rs` — request/response structs

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalculationPayload {
    pub expression: String,
    #[serde(default)]
    pub precision: Option<usize>,
}
```

### 7.4 `src/definitions.rs` — tool schemas

Every parameter needs a `description` and a `type`; tool and parameter
names are `snake_case`. If a tool can return a large or unbounded result
(file contents, byte streams, long lists), it **must** use the paging
envelope from §9 rather than returning everything in one call.

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

### 7.5 `src/operations.rs` — router + domain logic

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
raster buffer) — see §9 for why and what to do instead.

### 7.6 Adding resources or prompts (optional)

Alongside `tool_definitions()`/`execute_tool()`:

```rust
// definitions.rs
pub fn resource_definitions() -> Vec<ResourceDefinition> {
    vec![ResourceDefinition {
        uri: "current".to_string(),   // plugin-local; rune-kit prefixes it — see §6
        name: "Current Calculation State".to_string(),
        description: "The last evaluated expression and result.".to_string(),
        mime_type: Some("application/json".to_string()),
    }]
}

pub fn prompt_definitions() -> Vec<PromptDefinition> {
    vec![PromptDefinition {
        name: "explain_result".to_string(),
        description: "Ask the model to explain the last calculation in plain language.".to_string(),
        arguments: vec![],
    }]
}
```

```rust
// operations.rs
pub fn read_resource(uri: &str) -> Result<Value, String> {
    match uri {
        "current" => Ok(json!({ "expression": "...", "result": "..." })),
        unknown => Err(format!("Unknown resource: {}", unknown)),
    }
}

pub fn get_prompt(name: &str, _arguments: Value) -> Result<Value, String> {
    match name {
        "explain_result" => Ok(json!({
            "messages": [{
                "role": "user",
                "content": { "type": "text", "text": "Explain the last calculation result in plain language." }
            }]
        })),
        unknown => Err(format!("Unknown prompt: {}", unknown)),
    }
}
```

```rust
// lib.rs — new wasm exports, added ONLY if the plugin has resources/prompts.
// Omitting either export entirely is how rune-kit's install-time probe
// (§5.7) detects that this plugin doesn't support that primitive — don't
// add empty stub exports "just in case"; their absence is the signal.
#[cfg(target_arch = "wasm32")]
#[extism_pdk::plugin_fn]
pub fn mcp_list_resources(_: ()) -> extism_pdk::FnResult<String> {
    Ok(serde_json::to_string(&definitions::resource_definitions())?)
}

#[cfg(target_arch = "wasm32")]
#[extism_pdk::plugin_fn]
pub fn mcp_read_resource(uri: String) -> extism_pdk::FnResult<String> {
    let result = operations::read_resource(&uri);
    let output = match result {
        Ok(val) => json!({ "status": "success", "result": val }),
        Err(err) => json!({ "status": "error", "error": err }),
    };
    Ok(serde_json::to_string(&output)?)
}

// mcp_list_prompts / mcp_get_prompt follow the exact same shape.
```

### 7.7 `src/lib.rs` — WASM FFI boundary only

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

## 8. Building a Native Sidecar Plugin

Use this when the plugin's core job is either (a) linking a native
library that cannot target `wasm32-wasip1` (e.g. `rust-printers` needing
CUPS/WinSpool), or (b) shelling out to external binaries as its dominant
behavior (`rune-audio`/`rune-video` → `ffmpeg`/`yt-dlp`/`spotdl`/`streamlink`).

Everything in §4 and §7.3–§7.6 (`types.rs`, `definitions.rs`,
`operations.rs`) is written **identically** to a WASM plugin — a sidecar
is a different entry point over the same domain logic, not a different
architecture underneath.

### 8.1 `Cargo.toml` — add the native `[[bin]]` target

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
required-features = ["native"]

[features]
native = []   # bare flag is correct even with no optional deps behind it — see note below

[dependencies]
rune-pdk = { path = "../../crates/rune-pdk" }
serde.workspace = true
serde_json.workspace = true

[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
rune-sidecar = { path = "../../crates/rune-sidecar" }

[target.'cfg(target_arch = "wasm32")'.dependencies]
extism-pdk.workspace = true   # omit entirely if the plugin has NO wasm build at all
```

**`required-features = ["native"]` and `[features] native = []` are both
mandatory, and they must be added together.** `required-features` on its
own references a feature that doesn't exist, which is a hard `cargo`
error the moment anyone explicitly builds that bin target (e.g.
`cargo build -p rune-<name> --bin rune-<name>-native` fails outright with
`Package 'rune-<name>' does not have feature 'native'`). The feature can
be a bare `native = []` even when `rune-sidecar` is already pulled in
unconditionally via the `cfg(not(target_arch = "wasm32"))` dependency
block above — its only job here is giving `required-features` something
real to point at, gating whether the native binary target builds at all
without `--features native`.

This gate exists so a bare `cargo build -p rune-<name>` (no explicit
`--bin`/`--features`, e.g. from `cargo check --workspace` or `cargo test
-p rune-<name>`) doesn't try to compile the native sidecar and drag in
its native-only dependencies unconditionally. `cargo metadata` still
reports the bin target's existence and its `required_features` regardless
of which features are currently active — that's what lets `xtask` (§11)
discover and correctly build it without needing per-plugin configuration.

If the plugin has no reason to ever run as WASM (true for `rune-audio`),
you can drop the `cdylib` target and the wasm dependency block entirely —
keep `rlib` only, and skip `lib.rs`'s `#[cfg(target_arch = "wasm32")]`
handlers.

### 8.2 `src/bin/native_sidecar.rs` — the entire native entry point

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

    // Optional — omit entirely to inherit SidecarHandler's "not supported"
    // defaults (§5.7). Only override if this plugin actually has resources
    // or prompts.
    fn list_resources(&self) -> Vec<rune_pdk::ResourceDefinition> {
        definitions::resource_definitions()
    }
    fn read_resource(&self, uri: &str) -> Result<Value, String> {
        operations::read_resource(uri)
    }
}

fn main() -> std::io::Result<()> {
    run_stdio(Handler)
}
```

`rune-sidecar::run_stdio` (in `crates/rune-sidecar`) implements the
newline-delimited JSON loop (`info` / `list_tools` / `call_tool` /
`list_resources` / `read_resource` / `list_prompts` / `get_prompt`
methods, plus their MCP-style aliases and JSON-RPC `id` handling — see
§5.3) that `NativeSidecar` on the host side speaks to — it exists once,
shared by every sidecar plugin, so no plugin hand-rolls its own stdio
protocol.

### 8.3 Registering the sidecar with `rune-kit`

`PackageManager` records the plugin's execution model (§5.6) on install;
`McpRouter::register` constructs `PluginInstance::Native(NativeSidecar::
spawn(name, path)?)` instead of `PluginInstance::Wasm(...)` for that
namespace, based on the lockfile entry. Namespacing, tool listing, and
dispatch in `protocol.rs` are otherwise unchanged (§5.1).

### 8.4 Native subprocess calls stay in `operations.rs`, not the sidecar entry point

Keep `run_binary`/`Command::new` calls inside `operations.rs`'s
`#[cfg(not(target_arch = "wasm32"))]` branch, exactly as `rune-audio`
already does — `src/bin/native_sidecar.rs` should never call
`std::process::Command` directly. This keeps the domain logic testable
via `cargo test -p rune-<name>` without spawning real subprocesses in
tests that don't need to.

### 8.5 Optional: standalone CLI invocation

`rune-sidecar` also exports `parse_cli_args` and `resolve_arguments` —
helpers that let a sidecar binary be invoked directly from a terminal
(`rune-audio-native --url https://... --format mp3`) with CLI flags
resolved against a tool's schema, falling back to `SCREAMING_SNAKE_CASE`
environment variables when a flag isn't passed (`CLI > ENV` priority).
This is optional plumbing for debugging or non-`rune-kit` use of a
sidecar binary — `rune-kit` itself only ever talks to a sidecar over the
stdio protocol in §5.3/§8.2, never through these CLI helpers.

---

## 9. Data & Memory Handling — the Paging Rule

**No single tool call, resource read, or prompt response may require
materializing more than a bounded amount of data in memory before
returning**, regardless of execution model or primitive. This is not a
style preference — it's what previously caused large files to appear
truncated: a WASM guest has a finite linear memory ceiling, and a
response has to exist there in full (often twice — once as raw bytes,
again as serialized JSON) before it can cross back to the host. Native
sidecars don't have the WASM ceiling, but the same rule still keeps stdio
messages small and responsive, so apply it universally — including
`resources/list`/`prompts/list` aggregation across many plugins, if the
plugin or resource count ever grows large enough for that to matter.

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
agent loops on `hasMore` until it's `false`. A resource whose contents are
large (e.g. a big file exposed as a resource rather than read via a tool)
follows the same pattern — page it, don't return it whole.

---

## 10. Naming & Consistency Rules

Package drift (a plugin's `Cargo.toml` `name`, its folder name, and its
own header-comment path disagreeing with each other) has already caused
real confusion in this codebase. When renaming or creating a plugin,
these locations must all agree, checked in this order:

1. Folder name: `plugins/rune-<name>/`
2. `Cargo.toml` → `[package] name = "rune-<name>"`
3. Root workspace `Cargo.toml` → `members = [..., "plugins/rune-<name>"]`

If the plugin has a native sidecar binary, its `[[bin]] name` **must** be
exactly `rune-<name>-native`. This isn't just a style preference anymore
— `xtask` (§11) constructs that exact string
(`format!("{}-native", plugin.name)`) to build and test it, and the
publish pipeline (§13) constructs the same string for release-asset
filenames. A binary named anything else silently won't be found by
either.

**Resource and prompt naming:**

- A plugin's `resource_definitions()` returns **bare, unqualified** URIs
  or identifiers — never including the `rune://` scheme or its own
  namespace segment. That prefix is applied centrally in `protocol.rs`
  (§6), exactly the way `plugin__tool` is applied centrally rather than
  by each plugin.
- Prompt names follow the same `snake_case` convention as tool names, and
  the same `plugin__name` namespacing is applied centrally when
  aggregating `prompts/list` — a plugin's `prompt_definitions()` returns
  bare names, just like `tool_definitions()` does.

---

## 11. Build Orchestration (`xtask`)

Cargo builds for exactly one `--target` per invocation — there is no flag
that produces a `wasm32-wasip1` cdylib and a host-native binary from a
single `cargo build`. A "dual-target build" is therefore always at least
two real `cargo build` processes; the only question is whether that
chaining happens visibly in a shell (`&&`, which silently stops relaying
useful state on the first failure only in the way you'd expect from a
single command) or inside a real program that Cargo invokes as a
subcommand. `xtask` is that program.

### 11.1 Repo setup

```toml
# .cargo/config.toml (repo root, NOT inside xtask/)
[alias]
xtask = "run --quiet --package xtask --"
```

```toml
# root Cargo.toml
[workspace]
members = [
    # ...
    "xtask",
]
```

`xtask` is a plain host-only binary crate (`xtask/Cargo.toml`,
`xtask/src/main.rs`) — a normal workspace member, never built for
`wasm32-wasip1`, alongside `rune-sidecar`.

### 11.2 What it does

`xtask` shells out to `cargo metadata --no-deps --format-version 1` to
discover every package under `plugins/`, checks each package's `targets`
for a `cdylib` (wasm-buildable) and/or a `bin` (native-buildable) kind —
the same detection approach the publish pipeline uses (§13) — and drives
the appropriate `cargo build`/`cargo test` invocations per plugin.

```bash
# Test Single Plugin
cargo xtask test rune-<name>

# Test Whole Workspace
cargo xtask test-all

# Build Single plugin in either wasm or native (if available)
cargo xtask build rune-<name> --wasm-only
cargo xtask build rune-<name> --native-only

# Build Single plugin, both targets
cargo xtask build rune-<name>

# Build Whole workspace, both targets
cargo xtask build-all
```

`xtask test`/`test-all` always wraps `cargo test -p <name> --all-features`
with `dotenvx run -f plugins/<name>/.env --` — unconditionally, not
gated on whether that particular plugin's `.env` happens to be non-empty
(§12 explains why the unconditional form is the right default here).

`build`/`build-all` run the wasm and native builds **sequentially**, not
in parallel — a deliberate simplicity-over-speed choice, since concurrent
`cargo build` invocations against the same `target/` directory can hit
lock contention depending on cargo version. If build time on `build-all`
becomes a real bottleneck, spawning both `Command`s before waiting on
either (rather than `.status()` sequentially) is the fix — not a redesign.

### 11.3 CI uses the exact same commands

`ci.yml` calls `cargo xtask build-all` and `cargo xtask test-all`
directly rather than re-deriving its own version of plugin discovery.
This isn't just less duplicated code — it means "it builds/tests on my
machine" and "it builds/tests in CI" are provably the same claim, because
they're the same command. Previously, CI's hand-rolled wasm-only build
loop meant native sidecar compilation had **zero** CI coverage; that gap
closed automatically once CI started calling `xtask` instead of
reimplementing a subset of it. `ci.yml` only proves the native side
compiles on the Linux runner it executes on — the publish pipeline's
matrix (§13) is what actually cross-builds and releases the macOS/Windows
binaries; the two are complementary, not redundant.

---

## 12. Testing Architecture

Both plugin models are tested identically, natively, with no WASM
toolchain involved — this is the entire point of keeping `lib.rs` (or
`bin/native_sidecar.rs`) as a thin boundary over pure `operations.rs`.

### 12.1 Every plugin has a `.env`, so tests always run through `dotenvx`

Every plugin gets a `.env` file at `plugins/rune-<name>/.env` as a repo
convention — even a plugin with no secrets today gets an empty one. That
means the test command doesn't need to branch on "does this plugin
happen to need env vars" — it's always:

```bash
dotenvx run -f ./plugins/rune-<name>/.env -- cargo test -p rune-<name> --all-features
```

or, equivalently and preferably, `cargo xtask test rune-<name>` (§11),
which does exactly this. If a plugin is ever found missing its `.env`,
that's a repo-consistency bug to fix by adding the file — not a case for
`xtask` to special-case around by falling back to a bare `cargo test`.

```bash
# Whole workspace, if you're not going through xtask
cargo test --workspace --all-features
```

### 12.2 Contract tests (`tests/contract_tests.rs`)

```rust
use rune_<name>::{definitions::tool_definitions, operations::execute_tool};
use rune_pdk::test_plugin_contract;

test_plugin_contract!(tool_definitions, execute_tool);
```

The shared `test_plugin_contract!` macro (in `rune-pdk`) automatically
checks: every tool in `definitions()` is routable in `execute_tool`;
every required parameter, when omitted, produces an `Err`; and malformed
argument types don't panic.

A plugin that also implements resources and/or prompts extends the same
call rather than adding separate macro invocations:

```rust
test_plugin_contract!(
    tools: (tool_definitions, execute_tool),
    resources: (resource_definitions, read_resource),
    prompts: (prompt_definitions, get_prompt),
);
```

Omitted primitives are skipped, not treated as failures — the macro
validates whichever `definitions()`/handler pairs you actually pass it,
using the same "description required," "no panics on malformed input"
rules already applied to tools. Existing single-primitive invocations
(`test_plugin_contract!(tool_definitions, execute_tool)`) keep compiling
unchanged; extending the macro to accept resources/prompts is additive.

### 12.3 Operations tests (`tests/operations_tests.rs`)

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
(and `list_resources`/`read_resource`, `list_prompts`/`get_prompt`, if
implemented) message over real stdio — this is the one thing pure
`operations.rs` tests can't catch (protocol framing bugs).

---

## 13. CI/CD Pipeline

Two workflows, deliberately shaped differently, because they're solving
different problems in a monorepo of **independently-versioned** plugins
(each plugin's own `Cargo.toml` version, not a single repo-wide version).

### 13.1 `ci.yml` — push to `develop`

fmt check → clippy (`--all-features`, so native-gated code gets linted
too) → `cargo xtask build-all` → `cargo xtask test-all` → autofix commit.
No version bumping here — each plugin's maintainer bumps its version in
the same PR that changes it, crates.io-style, not an automated repo-wide
bump.

### 13.2 `publish.yml` — push to `main`

A "release" isn't one repo-wide event here — it's however many plugin
versions changed since the last publish, each getting its own tag and
release:

1. **`detect`** — `scripts/detect-publishable.py` diffs every plugin's
   `Cargo.toml` version against `registry/index.json` via `cargo
   metadata`, emitting two build matrices (`wasm`, `native`) containing
   only versions that aren't published yet. If nothing changed, both are
   empty and the build jobs below are skipped entirely.
2. **`build-wasm`** — one matrix job per new plugin version with a
   `cdylib` target: `cargo build -p <name> --release --target
   wasm32-wasip1 --lib`.
3. **`build-native`** — one matrix job per (new plugin version × target
   triple) for plugins with a `bin` target, across
   `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`,
   `x86_64-apple-darwin`, `x86_64-pc-windows-msvc`:
   `cargo build -p <name> --release --target <triple> --bin
   <name>-native --features native`. The `--features native` is not
   optional — omitting it is a hard `cargo` error given §8.1's
   `required-features` gate, not a silent skip.
4. Both build jobs tag+release as `<plugin-name>-v<version>` (idempotent
   — `gh release create ... || true` handles the race where wasm and
   native jobs for the same version both try to create the tag) and
   upload their artifact with `gh release upload ... --clobber`.
5. **`update-registry`** — after both build jobs finish (or are
   skipped), `scripts/update-registry.py` re-derives the same "what's
   new" list and merges each into `registry/index.json`, then commits
   directly to `main` with `[skip ci]` (relying on GitHub's native
   skip-ci detection to avoid retriggering itself).

### 13.3 Registry schema

```json
{
  "rune-audio": {
    "latest": "0.1.0",
    "description": "Audio track extraction and music downloader powered by yt-dlp, ffmpeg, and spotdl",
    "versions": {
      "0.1.0": {
        "url": "https://github.com/<owner>/rune-tools/releases/download/rune-audio-v0.1.0/rune-audio-0.1.0.wasm",
        "native": {
          "x86_64-unknown-linux-gnu": "https://.../rune-audio-native-0.1.0-x86_64-unknown-linux-gnu.tar.gz",
          "x86_64-pc-windows-msvc": "https://.../rune-audio-native-0.1.0-x86_64-pc-windows-msvc.zip"
        }
      }
    }
  }
}
```

`"url"` is backward-compatible with `rune-kit`'s existing
`fetch_from_registry` — every wasm-capable plugin still has it in the
same place. `"native"` is additive. A plugin with no `cdylib` target at
all has no `"url"` key, which `rune-kit`'s installer (§5.6) now handles
via `--native`/fallback logic — but a registry consumer written before
§5.6 existed would still error trying to read `.url` unconditionally, so
don't assume every registry entry has one.

Neither `scripts/detect-publishable.py` nor `scripts/update-registry.py`
needs to change when `required-features`/`[features] native` is added to
a plugin, nor when resources/prompts are added — all three only read
target *kinds* (`cdylib`/`bin`) and package metadata from `cargo
metadata`, none of which depend on which MCP primitives a plugin
implements or which features are currently enabled.

---

## 14. Decision Checklist: WASM Plugin vs Native Sidecar

Walk through in order; stop at the first match. (This checklist is about
execution *model* — wasm vs native. For which MCP *primitive* — tool,
resource, or prompt — a given piece of functionality should be, see §2
instead; the two decisions are independent of each other.)

1. **Does the plugin need a native library/binding that cannot compile to
   `wasm32-wasip1`** (native FFI, OS-specific APIs like CUPS/WinSpool)?
   → **Native sidecar.**
2. **Is the plugin's dominant behavior shelling out to external CLI
   binaries**, with little real compute happening in the plugin itself
   (`ffmpeg`, `yt-dlp`, `git`)? → **Native sidecar** — the WASM layer would
   only be relaying arguments through `host_cmd_exec` anyway, which is
   both unnecessary overhead and the exact generic-exec surface §5.4
   deprecates.
3. **Does most of the value come from in-process pure compute** (parsing,
   encoding, filesystem traversal, graph/state logic) with occasional,
   narrow host calls? → **WASM plugin.**
4. **Mixed** — real in-process compute *and* a native-only dispatch step
   (e.g. `rune-print`: PDF/PWG rasterization is pure Rust compute; final
   job dispatch needs CUPS/WinSpool)? → **Hybrid**: keep the compute-heavy
   part as a WASM plugin, add a narrow typed host_fn (§5.4) or a small
   sidecar for just the native dispatch step. Don't move the whole plugin
   to native just because one operation needs it.

Plugins not yet run through this checklist, don't copy their structure by
default without doing so first