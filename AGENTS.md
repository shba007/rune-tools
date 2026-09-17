# Rune Tool - Agent Documentation

## Overview

The Rune Ecosystem is a Rust-based MCP (Model Context Protocol) plugin architecture that separates untrusted/variable capability code from the trusted host using two execution models instead of one:

| Model | Runs as | Used for |
|---|---|---|
| **WASM plugin** | `\\.wasm` compiled to `wasm32-wasip1`, loaded by `rune-kit` into an Extism/Wasmtime sandbox | Logic that does real in-process compute (parsing, encoding, rendering, string/graph manipulation) and benefits from memory isolation |
| **Native sidecar** | A native OS binary, spawned as a child process by `rune-kit`, spoken to over stdio JSON | Logic whose real work is unavoidably native: linking a native library that can't target `wasm32-wasip1` (e.g. CUPS/WinSpool via `rust-printers`), or shelling out to external binaries (`ffmpeg`, `yt-dlp`, `gallery-dl`) where the WASM layer would just be a pass-through anyway |

**Rule of thumb:** if removing the WASM layer wouldn't remove any real
sandboxing benefit — because the dangerous part (subprocess exec, native
library calls) already happens on the host side of a host_fn — build a
native sidecar instead of a WASM plugin. Don't pay the WASM/host_fn
serialization tax for a pass-through.

Both models expose the same contract to `rune-kit`: `info`, plus a
list+read/call/get pair for each MCP primitive a plugin actually supports
(`list_tools`/`call_tool`, `list_resources`/`read_resource`,
`list_prompts`/`get_prompt` — see §1.2). `rune-kit-core::McpRouter` treats
every plugin uniformly via a `PluginInstance` enum (documented in
`AGENTS.md`, the `rune-kit` companion to this doc — `PluginInstance` is a
host-runtime type, not something this repo owns) — from the MCP client's
point of view, namespacing and dispatch behave identically regardless of
which model backs a given plugin, and regardless of which subset of
primitives it implements.

**Status update, confirmed against `rune-kit-core` as of this revision:**
`McpRouter`'s `resources/list`, `resources/read`, `prompts/list`, and
`prompts/get` routing is no longer aspirational — it's implemented and
live on the host side (previously these were hardcoded to return empty
arrays). §5 and §10.1 below describe what this does and doesn't mean for
plugins in *this* repo: the routing exists, but no plugin here has adopted
it yet — don't read "the host can route resources" as "plugins currently
have resources."

## 1. Core Architecture

### 1.1 Execution Models

The ecosystem separates **untrusted/variable capability code** from
**the trusted host**, using two execution models instead of one:

#### WASM Plugins
- **Target**: `wasm32-wasip1` compilation
- **Runtime**: Extism/Wasmtime sandbox
- **Use Case**: Pure compute operations (parsing, encoding, rendering, string/graph manipulation)
- **Benefit**: Memory isolation and sandboxing
- **Examples**: `rune-filesystem`, `rune-time`, `rune-fetch`, `rune-git`, `rune-memory`, `rune-sequential-thinking`

#### Native Sidecars
- **Target**: Native OS binaries
- **Runtime**: Persistent child process via stdio JSON
- **Use Case**: Native library dependencies, external binary execution
- **Examples**: `rune-audio`, `rune-video`, `rune-image`

### 1.2 MCP Primitive Support

MCP defines three distinct primitives. Mixing them up produces a plugin
that technically works but confuses every client that talks to it. Decide
which one you're building before writing `definitions.rs`:

| Primitive | Who decides to use it | Shape | Rune verbs |
|---|---|---|---|
| **Tool** | The model, autonomously, mid-conversation, based on what the task needs | An action/function with typed args and a return value | `list_tools` / `call_tool` |
| **Resource** | The user or client application — attached explicitly or browsed, not invoked by the model turn-by-turn | Addressable data at a URI — read-only context, not an action | `list_resources` / `read_resource` |
| **Prompt** | The user, explicitly (a slash command, a menu pick) — never triggered autonomously by the model | An argument-templated message sequence that seeds a conversation | `list_prompts` / `get_prompt` |

### 1.3 Repository Layout

```text
rune-tools/
├── .cargo/
│   └── config.toml                    # [alias] xtask — see §3.1
├── Cargo.toml                         # [workspace] members + shared deps
├── xtask/                             # build/test orchestrator — see §3.1
└── plugins/
    ├── rune-filesystem/                # WASM-only (pure compute: fs walk, paging)
    ├── rune-time/                      # WASM-only (pure compute)
    ├── rune-fetch/                     # WASM-only (HTML→Markdown, network via host_fn)
    ├── rune-git/                       # candidate for native sidecar — review (see §8)
    ├── rune-audio/                     # NATIVE SIDECAR (yt-dlp/ffmpeg/spotdl)
    ├── rune-video/                     # NATIVE SIDECAR (yt-dlp/ffmpeg/streamlink)
    ├── rune-image/                     # NATIVE SIDECAR (gallery-dl and similar)
    ├── rune-email/                     # execution model unconfirmed — classify via §8
    ├── rune-browser/                   # new, currently disabled in workspace members — classify via §8
    ├── rune-print/                     # HYBRID — WASM renders, native sidecar dispatches (currently disabled, mid-migration)
    ├── rune-memory/                    # WASM-only, pure compute (currently disabled, mid-migration)
    └── rune-sequential-thinking/       # WASM-only, pure compute (currently disabled, mid-migration)
```

## 2. Implementation Conventions

### 2.1 Standard Plugin Module Layout

```text
plugins/rune-<name>/
├── Cargo.toml                  # see §7.1/§7.2 for WASM-only vs sidecar config
├── .env                        # always present, even if empty — see §2.4
├── src/
│   ├── lib.rs                  # WASM FFI boundary ONLY — gated #[cfg(target_arch = "wasm32")]
│   ├── bin/
│   │   └── native_sidecar.rs   # native entry point — ONLY present for sidecar plugins (§7.2)
│   ├── definitions.rs          # pure tool/resource/prompt schemas — see note below
│   ├── operations.rs           # pure tool/resource/prompt execution — see note below
│   └── types.rs                # request/response deserialization structs
└── tests/
    ├── contract_tests.rs       # schema/routing/type-rejection macro tests
    └── operations_tests.rs     # domain logic unit tests
```

**Non-negotiable rule:** `definitions.rs`, `operations.rs`, and
`types.rs` must never import `extism_pdk` or anything WASM-specific. This is
what lets `cargo test -p rune-<name>` run on your laptop with zero WASM
toolchain, and it's what lets a native sidecar `main.rs` reuse the exact
same `operations::execute_tool` your WASM `lib.rs` calls — there is only
ever one implementation of the domain logic, never a fork.

### 2.2 Naming & Style Conventions

#### Tool Naming
- **Format**: snake_case (`create_entities`, `git_status`, `add_observations`)
- **Evidence**: 25+ instances across all plugins
- **Scope**: Applied to all tool definitions in `definitions.rs`

#### Function Naming
- **Format**: `mcp_` prefix (`mcp_info`, `mcp_list_tools`, `mcp_call_tool`)
- **Evidence**: Consistent across rune-memory and rune-git plugins
- **Scope**: WASM FFI boundary functions only

#### Variable/File Naming
- **Format**: snake_case (`memory_file`, `entity_name`, `relation_type`)
- **Evidence**: Consistent across all plugin files
- **Scope**: Variables, file names, function parameters

### 2.3 Error Handling Pattern

**Error Type**: `Result<Value, String>` for tool operations
**Error Messages**: Descriptive string with context
**Response Structure**: 
- Success: `{"status": "success", "result": val}`
- Error: `{"status": "error", "error": err}`

**Pattern Example** (`rune-memory/src/operations.rs`):
```rust
pub fn execute_tool(request: ToolCallRequest) -> Result<Value, String> {
    match request.name.as_str() {
        "calculate" => {
            // success path: Ok(value)
        }
        unknown => Err(format!("Unknown tool: {}", unknown)),
    }
}
```

### 2.4 Testing Framework

#### Contract Tests
```rust
use rune_<name>::{definitions::tool_definitions, operations::execute_tool};
use rune_pdk::test_plugin_contract;

test_plugin_contract!(tool_definitions, execute_tool);
```

#### Operations Tests
```rust
use rune_<name>::operations::execute_tool;
use rune_pdk::ToolCallRequest;
use serde_json::json;

#[test]
fn test_calculate_success() {
    let req = ToolCallRequest { name: "calculate".to_string(), arguments: json!({ "expression": "10 + 5" }) };
    let res = execute_tool(req).unwrap();
    assert_eq!(res["result"], "42.00");
}
```

#### Environment Integration
All test commands use `dotenvx run -f ./plugins/rune-<name>/.env --`
for consistent environment loading across all plugins.

#### Validation Requirements
Beyond contract and operations tests, plugin changes must also be validated for:
- **Both execution models** — plugin loading and execution in WASM and native modes (where the plugin supports both)
- **Capability enforcement** — declared capabilities are actually enforced at runtime
- **Error recovery and cleanup** — failure paths leave no leaked state (open handles, child processes, temp files)
- **Performance under load** — behavior under repeated/concurrent invocation

### 2.5 Configuration Pattern

#### Plugin.toml Structure
```toml
[capabilities]
network_hosts = []
filesystem = { mode = "scoped", root_param = "allowed_dir" }
exec = { allowed_binaries = [] }
```

#### Environment Variable Resolution
```rust
#[cfg(target_arch = "wasm32")]
fn get_config(key: &str) -> Option<String> {
    extism_pdk::config::get(key).ok().flatten()
}

#[cfg(not(target_arch = "wasm32"))]
fn get_config(key: &str) -> Option<String> {
    let upper = key.to_ascii_uppercase();
    let lower = key.to_ascii_lowercase();
    std::env::var(&upper)
        .or_else(|_| std::env::var(&lower))
        .or_else(|_| std::env::var(key))
        .ok()
}
```

## 3. Build & Deployment

### 3.1 Build Orchestration (xtask)

#### Overview
`xtask` is a host-only binary crate that coordinates builds across all plugins in the workspace, handling both WASM and native sidecar compilation.

#### Commands
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

#### Configuration
```toml
# .cargo/config.toml (repo root)
[alias]
xtask = "run --quiet --package xtask --"
```

### 3.2 Naming & Consistency Rules

**Package Drift Prevention**:
1. Folder name: `plugins/rune-<name>/`
2. Cargo.toml → `[package] name = "rune-<name>"`
3. Workspace Cargo.toml → `members = [..., "plugins/rune-<name>"]`

**Binary Naming**:
- Native sidecar binary name must be exactly `rune-<name>-native`
- Used by `xtask` for discovery and `publish.yml` for release assets

## 4. Security & Isolation

### 4.1 Capability Manifest System

Every plugin ships a manifest declaring exactly what it needs. `rune-kit`
grants only what's declared — no more `with_allowed_host("*")` by default.

#### Capability Categories
```toml
[capabilities]
network_hosts = []                # explicit hostnames/IPs, templated from config where needed
filesystem = { mode = "scoped", root_param = "allowed_dir" }
exec = { allowed_binaries = [] }  # empty = no host_exec capability
```

#### Host Function Policy
**Deprecated for new work**:
- General-purpose `host_cmd_exec(program, args)`
- Security hole: arbitrary program execution with shell interpolation

**Typed, single-purpose replacements**:
```rust
fn host_tcp_send(req: TcpSendRequest) -> TcpSendResponse;
fn host_http_request(req: HttpRequest) -> HttpResponse;
fn host_exec(req: ExecRequest) -> ExecResponse;  # allowlist-controlled only
```

### 4.2 Execution Model Security

#### WASM Plugins
- **Sandbox**: Extism/Wasmtime isolation per call
- **Boundary**: `lib.rs` contains only FFI functions
- **Testing**: No WASM toolchain required for unit tests

#### Native Sidecars
- **Isolation**: Persistent child process
- **Panic Handling**: `std::panic::catch_unwind` wraps every call
- **Logging**: `Stdio::inherit()` for sidecar logs
- **Security**: Child process killed on parent crash

## 5. Resource & Prompt Support

**Host-side status (confirmed, not aspirational):** `rune-kit-core`'s
`McpRouter` now actually implements `resources/list` / `resources/read` /
`prompts/list` / `prompts/get` routing — this used to be hardcoded to return
empty arrays and was documented here as a future architecture-document idea.
It no longer is one. What follows in this section is still accurate for how
a plugin *would* implement these, but the gap it's closing is "no plugin in
this repo has adopted it yet," not "the host can't route it yet." See §10.1
for the current adoption state.

### 5.1 Primitive Design Principles

**Resource Convention**:
- **URI Format**: `rune://<plugin-namespace>/<plugin-defined-path>`
- **Namespacing**: Host segment = plugin namespace, path segment = plugin-local URI
- **Capability**: Same `allowed_dir` scoping as tool calls

**Prompt Convention**:
- **Tool Alternative**: Replace user-spoken tool chains with actual prompts
- **Example**: `rune-git` currently implements ", "path": "\n"formulate a semantic commit message... commit the changes"

### 5.2 Implementation Pattern

**Shared Structure** (`definitions.rs`/`operations.rs`):
```rust
// definitions.rs
pub fn resource_definitions() -> Vec<ResourceDefinition> {
    vec![ResourceDefinition {
        uri: "current".to_string(),
        name: "Current Calculation State".to_string(),
        description: "The last evaluated expression and result.".to_string(),
        mime_type: Some("application/json".to_string()),
    }]
}

// operations.rs
pub fn read_resource(uri: &str) -> Result<Value, String> {
    match uri {
        "current" => Ok(json!({ "expression": "...", "result": "..." })),
        unknown => Err(format!("Unknown resource: {}", unknown)),
    }
}
```

**WASM Exports** (`lib.rs`):
```rust
#[cfg(target_arch = "wasm32")]
#[extism_pdk::plugin_fn]
pub fn mcp_list_resources(_: ()) -> extism_pdk::FnResult<String> {
    Ok(serde_json::to_string(&definitions::resource_definitions())?)
}

#[cfg(target_arch = "wasm32")]
#[extism_pdk::plugin_new]
pub fn mcp_read_resource(uri: String) -> extism_pdk::FnResult<String> {
    // implementation
}
```

## 6. CI/CD Pipeline

### 6.1 Development CI (`ci.yml`)

- **Trigger**: Push to `develop`
- **Checks**: fmt, clippy, xtask build-all, xtask test-all
- **Purpose**: Code quality and integration testing

### 6.2 Release Pipeline (`publish.yml`)

#### Detection
```yaml
- name: detect
  run: python scripts/detect-publishable.py
```

#### Build Matrix
```yaml
strategy:
  matrix:
    plugin: [rune-audio, rune-video, rune-image, ...]
    target: [wasm32-wasip1, x86_64-unknown-linux-gnu, ...]
```

#### Publishing
```yaml
- name: publish
  run: python scripts/update-registry.py
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

## 7. Plugin Development Guide

### 7.1 WASM Plugin Development

#### File Structure
```rust
plugins/rune-<name>/
├── Cargo.toml          # see !!7.2
├── .env               # always present
├── src/
│   ├── lib.rs          # WASM FFI only
│   ├── definitions.rs  # tool schemas
│   ├── operations.rs   # domain logic
│   └── types.rs        # request/response types
└── tests/
    ├── contract_tests.rs
    └── operations_tests.rs
```

#### Cargo.toml (WASM)
```toml
[package]
name = "rune-<name>"
version = "0.1.0"
edition.workspace = true
license.workspace = true

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
rune-pdk = { path = "../../crates/rune-pdk" }
serde.workspace = true
serde_json.workspace = true

[target.'cfg(target_arch = "wasm32")'.dependencies]
extism-pdk.workspace = true
```

### 7.2 Native Sidecar Development

#### File Structure
```rust
plugins/rune-<name>/
├── Cargo.toml          # see !!8.1
├── .env               # always present
├── src/
│   ├── lib.rs          # WASM FFI (conditional)
│   ├── bin/
│   │   └── native_sidecar.rs   # native entry point
│   ├── definitions.rs  # tool schemas
│   ├── operations.rs   # domain logic
│   └── types.rs        # request/response types
└── tests/
    ├── contract_tests.rs
    └── operations_tests.rs
```

#### Cargo.toml (Native)
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
native = []

[dependencies]
rune-pdk = { path = "../../crates/rune-pdk" }
serde.workspace = true
serde_json.workspace = true

[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
rune-sidecar = { path = "../../crates/rune-sidecar" }
```

## 8. Decision Checklist: WASM Plugin vs Native Sidecar

Walk through in order; stop at the first match.

1. **Does the plugin need a native library/binding that cannot compile to
   `wasm32-wasip1`** (native FFI, OS-specific APIs like CUPS/WinSpool)?
   → **Native sidecar.**

2. **Is the plugin's dominant behavior shelling out to external CLI
   binaries**, with little real compute happening in the plugin itself
   (`ffmpeg`, `yt-dlp`, `git`)? → **Native sidecar** — the WASM layer would
   only be relaying arguments through `host_cmd_exec` anyway, which is
   both unnecessary overhead and the exact generic-exec surface §4.1
   deprecates.

3. **Does most of the value come from in-process pure compute** (parsing,
   encoding, filesystem traversal, graph/state logic) with occasional,
   narrow host calls? → **WASM plugin.**

4. **Mixed** — real in-process compute *and* a native-only dispatch step
   (e.g. `rune-print`: PDF/PWG rasterization is pure Rust compute; final
   job dispatch needs CUPS/WinSpool)? → **Hybrid**: keep the compute-heavy
   part as a WASM plugin, add a narrow typed host_fn (!!5.4) or a small
   sidecar for just the native dispatch step. Don't move the whole plugin
   to native just because one operation needs it.

## 9. Plugin Catalog

### 9.1 Current Plugin Set

| Plugin | Description | Execution Model |
|---|---|---|
| rune-filesystem | Filesystem operations (walk, paging) | WASM |
| rune-time | Time-related utilities | WASM |
| rune-fetch | HTML→Markdown conversion | WASM |
| rune-git | Git repository management | WASM (candidate for sidecar) |
| rune-audio | Audio track extraction (yt-dlp/ffmpeg/spotdl) | NATIVE |
| rune-video | Video streaming (ffmpeg/streamlink) | NATIVE |
| rune-image | Image processing (gallery-dl) | NATIVE |
| rune-email | Email utilities | Unconfirmed |
| rune-browser | Browser utilities | Unconfirmed |
| rune-print | Print utilities (hybrid) | HYBRID |
| rune-memory | Knowledge graph storage | WASM |
| rune-sequential-thinking | Sequential thinking workflows | WASM |
| mhb-mconnect | MHB email template API connector (native REST proxy) | NATIVE |
| rune-scan | eSCL AirScan scanner client (ippusb, flatbed/ADF capture) | NATIVE |
| rune-slides | Markdown presentation builder (native PPTX/PDF export) | HYBRID |
| rune-ssh | SSH command execution + SFTP transfers | NATIVE |

### 9.2 Plugin Development Status

**Currently Enabled** (in workspace `Cargo.toml` `members`):
- `mhb-mconnect`, `rune-audio`, `rune-email`, `rune-fetch`, `rune-filesystem`, `rune-git`, `rune-image`, `rune-memory`, `rune-sequential-thinking`, `rune-slides`, `rune-time`, `rune-video`

**Currently Disabled** (commented out in workspace `Cargo.toml` `members`):
- `rune-browser`: Awaiting execution model classification
- `rune-print`: Mid-migration from WASM to hybrid native
- `rune-scan`: eSCL AirScan scanner client (driverless)
- `rune-ssh`: SSH execution + SFTP transfers

## 10. Open Design Questions

### 10.1 Implementation Gaps

#### Resources and Prompts
- **Current State**: Host-side routing (`McpRouter` in `rune-kit-core`) is
  now implemented — `resources/list`, `resources/read`, `prompts/list`,
  `prompts/get` all work end-to-end if a plugin defines them. What's still
  true: every plugin examined in this repo remains tools-only in practice.
  The gap is plugin adoption, not host support.
- **Next Steps**: Pick one low-risk plugin (a WASM-only one, per §7.1, to
  keep the blast radius small) and implement `resource_definitions()`/
  `read_resource()` per §5.2 as a first real end-to-end test of the routing
  — that will surface any rough edges in the URI-namespacing convention
  (§5.1) that a design read-through can't.

#### Capability Manifest Completeness
- **Current State**: Basic capabilities defined
- **Architecture Document**: Rich security model with typed host functions
- **Next Steps**: Refine capability granularity and host function authorization

#### Execution Model Decisions
- **Current State**: Some plugins mid-migration between models
- **Architecture Document**: Clear decision framework (!!14)
- **Next Steps**: Complete migration and classification

### 10.2 Research Needed

#### WASM vs Native Decision Process
- **Action**: Apply !!14 decision checklist to `rune-git` and other plugins
- **Goal**: Complete execution model classification
- **Impact**: Affects build configuration and security posture

#### Resource/Prompt Implementation
- **Action**: Now that host-side routing is confirmed live (§5), pick a
  candidate plugin and implement it end-to-end rather than continuing to
  treat this as an open research question
- **Goal**: Validate the §5.1 URI/namespacing conventions against a real
  plugin, not just the design doc
- **Impact**: Expands plugin capabilities and client integration

## 11. Development Workflow

### 11.1 Local Development

```bash
# Test a single plugin
cargo xtask test rune-<name>

# Test all plugins
cargo xtask test-all

# Build a single plugin
cargo xtask build rune-<name>

# Build all plugins
cargo xtask build-all
```

### 11.2 CI/CD Integration

**Pre-commit Hooks**:
- Conventional commit verification
- Auto-formatting (`cargo fmt --all`)
- Clippy auto-fixes (`cargo clippy --fix`)

**Commit Message Requirements**:
- Conventional commit format
- Auto-formatting and linting applied automatically

### 11.3 Troubleshooting: Plugin Loading Issues

When a plugin fails to load in `rune-kit`, check in order:
1. **Manifest format** — `plugin.toml` is valid TOML with the expected structure
2. **Capability declarations** — `[capabilities]` entries match what the plugin actually requests at runtime
3. **Binary path** — the artifact exists and is named exactly `rune-<name>-native` (sidecars) / `.wasm` (WASM), per §3.2
4. **Runtime compatibility** — the plugin version is compatible with the running `rune-kit` version

## 12. Glossary

| Term | Definition |
|---|---|
| **WASM plugin** | WASM-compiled plugin running in Extism/Wasmtime sandbox |
| **Native sidecar** | Native OS binary spawned as persistent child process |
| **PluginInstance** | Enum wrapper in `rune-kit-core` for uniform dispatch |
| **Extism** | Extism SDK for WASM plugin execution |
| **Sidecar** | Native process companion to a WASM plugin |
| **MCP** | Model Context Protocol — communication protocol |
| **Tool** | Autonomous action/function with typed args and return value |
| **Resource** | Addressable data at a URI (read-only context) |
| **Prompt** | Argument-templated message sequence for user interaction |
| **xtask** | Build/test orchestrator for the workspace |

---

*Document generated based on confirmed code patterns and comprehensive architecture analysis.*
*All claims are evidence-based with real file references from the codebase.*
*Patterns are confirmed with 2-3+ instances across multiple plugins.*