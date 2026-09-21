# Rune Tool - Agent Documentation

## Overview

The Rune Ecosystem is a Rust-based MCP (Model Context Protocol) plugin architecture that separates capability code from the host application (`rune-kit`). 

To maximize execution efficiency, eliminate unnecessary serialization hops, and prevent sandbox runtime traps, `rune-kit` supports **two first-class execution models**:

| Model | Architecture | Used for |
|---|---|---|
| **Pure WASM Plugin** | `rune-kit` $\longleftrightarrow$ `plugin.wasm` (`wasm32-wasip1`) | Logic that does real in-process compute (parsing, AST transforms, encoding, graph/state manipulation) inside an Extism/Wasmtime sandbox and benefits from strict memory isolation. |
| **Direct Native Plugin** | `rune-kit` $\longleftrightarrow$ `rune-<name>-native` (Native Executable via Stdio JSON-RPC) | Workloads that are inherently native: persistent duplex WebSockets/raw TCP (e.g., `rune-figma`), OS-specific C/FFI libraries (CUPS/WinSpool in `rune-print`), or controlling external CLI tools (`ffmpeg`, `yt-dlp`). `rune-kit` communicates directly with the native binary over MCP stdio, removing the intermediate WASM translation layer. |

### Architectural Shift: Direct Native Integration

In earlier revisions, every plugin was forced to expose a `plugin.wasm` gateway even if its core operations required a native process. This "WASM-in-the-middle" pattern caused significant runtime friction:
1. **Network & Socket Limitations:** WebAssembly (`wasm32-wasip1`) cannot open raw duplex TCP/WebSockets.
2. **Sandbox Traps:** When a WASM module attempts HTTP/TCP calls to offline or refused ports, Wasmtime aborts with uncatchable runtime traps rather than returning structured error types.
3. **Double Serialization:** Requests were serialized from Host $\to$ WASM $\to$ Native Sidecar $\to$ Host.
4. **Artifact Bloat:** Compiling unused `.wasm` modules alongside native binaries slowed down workspace builds.

**Current Rule:** 
- If a plugin's operations are **pure compute or memory-safe transformations**, compile it to a **Pure WASM plugin** (`wasm32-wasip1`).
- If a plugin requires **OS sockets, native hardware, or native subprocesses**, implement it as a **Direct Native Plugin** (`rune-<name>-native`). Both models expose the exact same MCP protocol primitives (`tools`, `resources`, `prompts`) to `rune-kit`'s `McpRouter`.

**External Binaries:** Third-party CLI tools (such as `ffmpeg`, `yt-dlp`, `gallery-dl`) continue to be **downloaded, verified, cached, and managed by `rune-kit`**, not by individual plugins. Plugins declare external binary dependencies in `plugin.toml` and receive verified executable paths from `rune-kit` at runtime (§13).

---

## 1. Core Architecture

### 1.1 Execution Models

`rune-kit` routes requests to either the in-process WebAssembly engine or the native process supervisor:

```text
                               ┌─────────────────────────────────────────────────────────────┐
                               │                          rune-kit                           │
                               │  (Host process: loads plugins, provisions external tools)   │
                               └──────────────┬──────────────────────────────┬───────────────┘
                                              │                              │
                     MCP Protocol / WASM FFI  │                              │  MCP Protocol / Stdio JSON-RPC
                                              ▼                              ▼
                 ┌────────────────────────────────────────┐     ┌────────────────────────────────────────┐
                 │          plugin.wasm (WASM)            │     │       rune-<name>-native (Native)      │
                 │   • Extism/Wasmtime sandbox            │     │   • Direct OS & hardware access        │
                 │   • Pure in-process compute            │     │   • Duplex WebSockets & TCP sockets    │
                 │   • AST, formatting, graph state       │     │   • Native C/FFI bindings (CUPS, etc.) │
                 │   • Zero host side-effects             │     │   • Subprocess execution (ffmpeg, etc.)│
                 └────────────────────────────────────────┘     └────────────────────────────────────────┘
```

#### Pure WASM Plugins
- **Target**: `wasm32-wasip1` compilation
- **Runtime**: Extism/Wasmtime sandbox inside `rune-kit`
- **Use Case**: In-process pure compute operations (parsing, encoding, rendering, text/graph manipulation)
- **Benefit**: Strict memory isolation, zero host side-effects, portable compilation
- **Examples**: `rune-filesystem`, `rune-time`, `rune-fetch`, `rune-memory`, `rune-sequential-thinking`

#### Direct Native Plugins
- **Target**: Native platform executable (`x86_64-pc-windows-msvc`, `aarch64-apple-darwin`, `x86_64-unknown-linux-gnu`)
- **Runtime**: Dedicated OS child process communicating directly with `rune-kit` via MCP Stdio JSON-RPC
- **Use Case**: Native socket connections (WebSockets, raw TCP, SSH), native OS libraries, or driving host-provisioned binaries
- **Benefit**: Zero WASM network/WASI bottlenecks, direct hardware/socket access, single-step compilation
- **Examples**: `rune-figma`, `rune-audio`, `rune-video`, `rune-image`, `rune-print`, `rune-ssh`, `rune-browser`

### 1.2 MCP Primitive Support

MCP defines three distinct primitives:

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
    ├── rune-filesystem/                # Pure WASM (pure compute: fs walk, paging)
    ├── rune-time/                      # Pure WASM (pure compute)
    ├── rune-fetch/                     # Pure WASM (HTML→Markdown, network via host_fn)
    ├── rune-git/                       # Pure WASM or Direct Native candidate
    ├── rune-figma/                     # Direct Native (WebSocket bridge to Figma canvas)
    ├── rune-audio/                     # Direct Native (ffmpeg, yt-dlp via rune-kit)
    ├── rune-video/                     # Direct Native (ffmpeg via rune-kit)
    ├── rune-image/                     # Direct Native (gallery-dl via rune-kit)
    ├── rune-email/                     # Pure WASM
    ├── rune-browser/                   # Direct Native (agent-browser engine)
    ├── rune-print/                     # Direct Native (native print dispatch: CUPS/WinSpool)
    ├── rune-memory/                    # Pure WASM (knowledge graph storage)
    └── rune-sequential-thinking/       # Pure WASM (workflow evaluation)
```

---

## 2. Implementation Conventions

### 2.1 Standard Plugin Module Layout

Every plugin shares the same internal domain separation (`definitions.rs`, `operations.rs`, `types.rs`) so domain logic can be unit-tested without external processes or WASM runtimes:

#### Pure WASM Plugin Layout
```text
plugins/rune-<name>/
├── Cargo.toml                  # crate config (crate-type = ["cdylib", "rlib"])
├── plugin.toml                 # capabilities & host declarations (§2.5)
├── .env                        # always present, even if empty
├── src/
│   ├── lib.rs                  # WASM FFI boundary to rune-kit (mcp_* exports)
│   ├── definitions.rs          # pure tool/resource/prompt schemas
│   ├── operations.rs           # pure tool/resource/prompt execution
│   └── types.rs                # request/response deserialization structs
└── tests/
    ├── contract_tests.rs       # schema/routing/type-rejection macro tests
    └── operations_tests.rs     # domain logic unit tests
```

#### Direct Native Plugin Layout
```text
plugins/rune-<name>/
├── Cargo.toml                  # crate config (crate-type = ["rlib"], [[bin]])
├── plugin.toml                 # capabilities & native declarations (§2.5)
├── .env                        # always present, even if empty
├── src/
│   ├── lib.rs                  # Re-exports modules (definitions, operations, types)
│   ├── bin/
│   │   └── native_sidecar.rs   # Native entry point (runs MCP Stdio JSON-RPC loop)
│   ├── definitions.rs          # pure tool/resource/prompt schemas
│   ├── operations.rs           # pure domain logic & parameter validation
│   └── types.rs                # request/response deserialization structs
└── tests/
    ├── contract_tests.rs       # schema/routing/type-rejection macro tests
    └── operations_tests.rs     # domain logic unit tests
```

**Non-negotiable rule:** `definitions.rs`, `operations.rs`, and `types.rs` must **never import `extism_pdk` or target-specific networking APIs**. This allows `cargo test -p rune-<name>` to execute on any host without a WASM toolchain or live network connections.

### 2.2 Naming & Style Conventions

#### Tool Naming
- **Format**: `snake_case` (`create_rectangle`, `git_status`, `export_node_as_image`)
- **Scope**: Applied to all tool definitions in `definitions.rs`

#### Function Naming
- **WASM FFI Functions**: `mcp_` prefix (`mcp_info`, `mcp_list_tools`, `mcp_call_tool`) in `lib.rs` (Pure WASM plugins only).
- **Native Dispatch Functions**: `dispatch_` or `execute_` prefix in `operations.rs` and `native_sidecar.rs`.

#### Variable/File Naming
- **Format**: `snake_case` (`node_id`, `output_format`, `allowed_dir`)

### 2.3 Error Handling Pattern

- **Error Type**: `Result<Value, String>` for tool operations
- **Actionable Messages**: Error strings must clearly describe *what went wrong and what to try next* (§12.2 #6)
- **Response Structure**:
  - Success: `{"status": "success", "result": val}`
  - Error: `{"status": "error", "error": err}`

### 2.4 Testing Framework

#### Contract Tests
Contract tests ensure schemas strictly comply with MCP protocol rules (every parameter must define a `description`, required arguments must fail if omitted, and tool names must route properly):

```rust
use rune_<name>::{definitions::tool_definitions, operations::execute_tool};
use rune_pdk::test_plugin_contract;

test_plugin_contract!(tool_definitions, execute_tool);
```

#### Operations Tests
Domain unit tests run directly against `operations.rs`:

```rust
use rune_<name>::operations::execute_tool;
use rune_pdk::ToolCallRequest;
use serde_json::json;

#[test]
fn test_create_shape_validation() {
    let req = ToolCallRequest {
        name: "create_rectangle".to_string(),
        arguments: json!({ "x": 100, "y": 200, "width": 300, "height": 400 }),
    };
    let res = execute_tool(req).unwrap();
    assert_eq!(res["status"], "success");
}
```

---

## 3. Build & Deployment

### 3.1 Build Orchestration (xtask)

`xtask` is the host-only orchestrator coordinating builds across both Pure WASM and Direct Native plugins:

```bash
# Test Single Plugin (Runs native unit & contract tests)
cargo xtask test rune-<name>

# Test Whole Workspace  
cargo xtask test-all

# Build Single Plugin:
# - Pure WASM: compiles target/wasm32-wasip1/release/rune_<name>.wasm
# - Direct Native: compiles target/release/rune-<name>-native
cargo xtask build rune-<name>

# Build Whole Workspace
cargo xtask build-all
```

### 3.2 Naming & Artifact Rules

1. Folder name: `plugins/rune-<name>/`
2. `Cargo.toml` package name: `rune-<name>`
3. **Artifact Output**:
   - Pure WASM plugins: `target/wasm32-wasip1/release/rune_<name>.wasm`
   - Direct Native plugins: `target/release/rune-<name>-native` (or `.exe` on Windows)

---

## 4. Security & Isolation

### 4.1 Capability Manifest System (`plugin.toml`)

Every plugin declares its capabilities in `plugin.toml`. `rune-kit` verifies and enforces these at runtime:

```toml
[plugin]
name = "rune-figma"
version = "0.1.0"
execution_model = "native" # "wasm" or "native"
description = "TalkToFigma MCP integration"

[capabilities]
network_hosts = ["localhost", "127.0.0.1"]
filesystem = { mode = "scoped", root_param = "image_dir" }
exec = { allowed_binaries = [] }

[dependencies.binaries]
```

### 4.2 Execution Model Security

- **Pure WASM Plugins**: Run in-process within an Extism/Wasmtime memory sandbox. Memory cannot leak into or compromise host address space.
- **Direct Native Plugins**: Run as separate, unprivileged OS child processes supervised by `rune-kit`. They communicate exclusively through standard I/O streams (`stdin`/`stdout`). If a native plugin panics or disconnects, the host process remains healthy and surfaces a structured error to the client.

---

## 5. Resource & Prompt Support

`rune-kit-core` implements full MCP routing for `resources/list`, `resources/read`, `prompts/list`, and `prompts/get`.

### 5.1 Pattern Implementation

Every plugin exposes prompt and resource definitions via `definitions.rs` and execution handlers in `operations.rs`:

```rust
// definitions.rs
pub fn resource_definitions() -> Vec<ResourceDefinition> {
    vec![ResourceDefinition {
        uri: "rune://figma/selection".to_string(),
        name: "Active Selection".to_string(),
        description: "Currently selected layers on the active canvas.".to_string(),
        mime_type: Some("application/json".to_string()),
    }]
}

pub fn prompt_definitions() -> Vec<PromptDefinition> {
    vec![PromptDefinition {
        name: "design_strategy".to_string(),
        description: "Best practices for working with Figma designs.".to_string(),
        arguments: serde_json::json!([]),
    }]
}
```

---

## 6. CI/CD Pipeline

### 6.1 Development CI (`ci.yml`)
- Checks: `cargo fmt --all`, `cargo clippy`, `cargo xtask build-all`, `cargo xtask test-all`.
- Validates clean compilation across WASM and native targets.

### 6.2 Release Pipeline (`publish.yml`)
- Compiles `wasm32-wasip1` modules for Pure WASM plugins.
- Cross-compiles native companion binaries (`rune-<name>-native`) across host architectures (`x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`).
- Release packages **never bundle third-party tools** (`ffmpeg`, `yt-dlp`); `rune-kit` provisions these dynamically at runtime (§13).

---

## 7. Plugin Development Guide

### 7.1 Pure WASM Plugin Development

Use this when all logic is pure in-process compute (data parsing, encoding, graph traversal).

#### `Cargo.toml`
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

#### `src/lib.rs` (WASM Boundary)
```rust
#[cfg(target_arch = "wasm32")]
use extism_pdk::*;
use rune_pdk::ToolCallRequest;

pub mod definitions;
pub mod operations;
pub mod types;

#[cfg(target_arch = "wasm32")]
#[plugin_fn]
pub fn mcp_call_tool(input: String) -> FnResult<String> {
    let req: ToolCallRequest = serde_json::from_str(&input)?;
    let res = operations::execute_tool(req);
    match res {
        Ok(val) => Ok(serde_json::to_string(&serde_json::json!({ "status": "success", "result": val }))?),
        Err(err) => Ok(serde_json::to_string(&serde_json::json!({ "status": "error", "error": err }))?),
    }
}
```

---

### 7.2 Direct Native Plugin Development

Use this when operations require persistent WebSockets, raw TCP sockets, OS-specific libraries (CUPS/WinSpool), or child processes.

#### `Cargo.toml`
```toml
[package]
name = "rune-<name>"
version = "0.1.0"
edition.workspace = true
license.workspace = true

[lib]
crate-type = ["rlib"]

[[bin]]
name = "rune-<name>-native"
path = "src/bin/native_sidecar.rs"
required-features = ["native"]

[features]
default = []
native = [
    "dep:tokio",
    "dep:tokio-tungstenite",
    "dep:futures-util",
]

[dependencies]
rune-pdk = { path = "../../crates/rune-pdk" }
serde = { workspace = true, features = ["derive"] }
serde_json.workspace = true

# Native dependencies activated during native binary builds
tokio = { version = "1", features = ["full"], optional = true }
tokio-tungstenite = { version = "0.21", features = ["connect"], optional = true }
futures-util = { version = "0.3", optional = true }

[dev-dependencies]
serde_json.workspace = true
```

#### `src/bin/native_sidecar.rs` (Native Stdio MCP Entry Point)
```rust
#[cfg(feature = "native")]
use futures_util::StreamExt;
#[cfg(feature = "native")]
use serde_json::{json, Value};
#[cfg(feature = "native")]
use tokio::io::{AsyncBufReadExt, BufReader};

#[cfg(feature = "native")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stdin = BufReader::new(tokio::io::stdin());
    let mut lines = stdin.lines();

    while let Ok(Some(line)) = lines.next_line().await {
        if line.trim().is_empty() { continue; }

        let req: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let method = req.get("method").and_then(|v| v.as_str()).unwrap_or("");
        let id = req.get("id");

        match method {
            "initialize" => {
                println!("{}", json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "protocolVersion": "2024-11-05",
                        "capabilities": { "tools": {} },
                        "serverInfo": { "name": env!("CARGO_PKG_NAME"), "version": env!("CARGO_PKG_VERSION") }
                    }
                }));
            }
            "notifications/initialized" => {}
            "tools/list" => {
                let tools = rune_<name>::definitions::tool_definitions();
                println!("{}", json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": { "tools": tools }
                }));
            }
            "tools/call" => {
                let params = req.get("params").cloned().unwrap_or(json!({}));
                let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or(json!({}));

                let result = rune_<name>::operations::execute_native_tool(name, args).await;
                match result {
                    Ok(val) => println!("{}", json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": { "content": [{ "type": "text", "text": serde_json::to_string(&val)? }] }
                    })),
                    Err(err) => println!("{}", json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "isError": true,
                        "result": { "content": [{ "type": "text", "text": err }] }
                    })),
                }
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(not(feature = "native"))]
fn main() {
    eprintln!("Compile with --features native to build native binary");
}
```

---

## 8. Decision Checklist: Pure WASM vs Direct Native

Use this checklist to choose the correct model when creating or refactoring a plugin:

1. **Does the plugin require persistent duplex sockets (WebSockets, TCP listeners, SSH connections)?**  
   $\rightarrow$ **Direct Native Plugin.** WASI p1 cannot maintain persistent asynchronous duplex sockets. Compiling to a direct native binary avoids connection traps.

2. **Does the plugin link against OS-specific system APIs (CUPS/WinSpool, Windows Registry, CoreAudio)?**  
   $\rightarrow$ **Direct Native Plugin.** Link system C/FFI libraries directly in native Rust.

3. **Does the plugin drive external CLI binaries (`ffmpeg`, `yt-dlp`, `gallery-dl`, `git`)?**  
   $\rightarrow$ **Direct Native Plugin.** The native binary executes verified binaries provisioned by `rune-kit` (§13).

4. **Is the plugin performing in-process pure compute (markdown parsing, JSON schema validation, math, state tracking)?**  
   $\rightarrow$ **Pure WASM Plugin.** Compiles to `wasm32-wasip1` with zero host dependencies and maximum sandboxed safety.

---

## 9. Plugin Catalog

### 9.1 Current Plugin Set

| Plugin | Description | Execution Model | External Binaries / Protocols |
|---|---|---|---|
| rune-filesystem | Filesystem operations (walk, paging) | Pure WASM | None |
| rune-time | Time-related utilities | Pure WASM | None |
| rune-fetch | HTML $\to$ Markdown conversion | Pure WASM | None |
| rune-git | Git repository inspection | Direct Native | `git` (host-provisioned) |
| rune-figma | Canvas inspection, shape creation, annotation | Direct Native | WebSockets (`ws://localhost:3055`) |
| rune-audio | Audio track extraction & conversion | Direct Native | `ffmpeg`, `yt-dlp` |
| rune-video | Video slicing, scene detection, inspection | Direct Native | `ffmpeg` |
| rune-image | Image processing & gallery scraping | Direct Native | `gallery-dl` |
| rune-email | Email templates & MIME parsing | Pure WASM | None |
| rune-browser | Browser automation engine | Direct Native | Browser engine |
| rune-print | Print rasterization & OS spooling | Direct Native | CUPS / WinSpool |
| rune-memory | Knowledge graph storage | Pure WASM | None |
| rune-sequential-thinking | Sequential thinking workflow engine | Pure WASM | None |
| rune-scan | eSCL AirScan scanner client | Direct Native | Raw TCP / mDNS |
| rune-slides | Markdown presentation builder | Pure WASM | None |
| rune-ssh | Remote command execution & SFTP | Direct Native | Native SSH/TCP |

### 9.2 Plugin Development Status

- **Enabled plugins**: `rune-filesystem`, `rune-time`, `rune-fetch`, `rune-memory`, `rune-sequential-thinking`, `rune-slides`, `rune-figma`, `rune-audio`, `rune-video`, `rune-image`, `rune-browser`, `rune-email`
- **Active migration**: `rune-print`, `rune-scan`, `rune-ssh`

---

## 10. Design Considerations

### 10.1 Uniform Routing in `rune-kit`
`rune-kit-core::McpRouter` exposes uniform routing regardless of whether the target plugin is running in-process via Wasmtime or as an unprivileged child process over stdio. Clients connect to the host or native binary using standard MCP JSON-RPC 2.0 without target-specific adapters.

---

## 11. Development Workflow

### 11.1 Local Development

```bash
# Test a single plugin (unit + contract tests)
cargo xtask test rune-<name>

# Test all plugins in workspace
cargo xtask test-all

# Build a single plugin (produces .wasm or -native binary based on model)
cargo xtask build rune-<name>

# Build all plugins in workspace
cargo xtask build-all
```

---

## 12. Tool Design Philosophy

This section governs *what* a plugin's tools should be. The target consumer is a mid-size model (~27B parameters) with limited context and perception.

### 12.1 Core Idea: Diagnose the Bottleneck First

| Bottleneck | Question | Tool Type |
|---|---|---|
| **Perception** | Can it see the result? | Renderers, screenshots, frame samplers, DOM/a11y tree |
| **Expression** | Can it state the edit precisely? | Structured DSLs and APIs instead of raw pixels or freeform code |
| **Verification** | Can it tell good from bad? | Constraint checkers, linters, critics |
| **State and memory** | Can it track a big, evolving artifact? | Scene graph, timeline JSON, checkpoints, diffs |
| **Search** | Can it try many options cheaply? | Fast preview, branching, best-of-n with a ranker |

### 12.2 Ten Tool-Design Guidelines

1. **Close the perceive $\to$ act loop first.** For visual and temporal domains, this is the highest leverage change. Without it, the model operates blind.
2. **Give it an editable intermediate representation, not raw binary/pixel data.**
   - Figma/UI: prune heavy vector coordinates (`filter_figma_node`), expose clean component/node trees.
   - Video: timeline JSON (clips, in/out points).
3. **Make verification layered.**
   - *Hard verifiers (sound):* Schema checks, bounds overlap, syntax checkers, collision tests (**gates**).
   - *Soft critics (learned):* VLM aesthetic evaluation (**advisors**).
4. **Prefer priors over generation from scratch.** Start from the closest template or component rather than generating raw primitives.
5. **Few, semantic, composable tools.** Aim for **10–25 well-named tools per domain** at the intent level (`create_rectangle`, `set_auto_layout`, `scan_text_nodes`).
6. **Design outputs and errors for the model.** Return compact, structured, actionable text. State *what went wrong and what to try next*.
7. **Make operations cheap, deterministic, and reversible.** Destructive operations should support `dry_run` and undo.
8. **Externalize memory.** Store project state, checkpoints, and task history in queryable structures, not conversation text.
9. **Planner/executor split with fixed workflows.** Encode known pipelines into MCP prompts (`annotation_conversion_strategy`, `text_replacement_strategy`).
10. **Build the eval harness before the tools.** Tools that do not measurably improve metrics should be ablated.

---

## 13. External Binary Dependencies (Host-Managed Provisioning)

### 13.1 Division of Responsibilities
1. **The user's machine has no developer tools.** No Node, Bun, Python, pip, npm, cargo, Homebrew, or Chocolatey can be assumed to exist.
2. **`rune-kit` owns downloading, verification, and caching.** The host application provisions binaries at runtime, verifies cryptographic checksums, and manages executable permissions.
3. **`rune-tools` owns dependency specification.** Plugins declare external binary constraints in `plugin.toml`.

### 13.2 Dependency Declaration in `plugin.toml`
```toml
[dependencies.binaries]
ffmpeg = { version = ">=6.1", optional = false, description = "Video slicing and audio extraction" }
yt-dlp = { version = ">=2024.01.01", optional = true, description = "Remote media extraction" }
```

### 13.3 Runtime Path Resolution
For Direct Native Plugins executing provisioned binaries:
- `rune-kit` resolves the absolute cached path and provides it via command-line arguments or environment variables.
- Plugins only spawn binaries using validated absolute paths (never shell invocations).

---

## 14. Glossary

| Term | Definition |
|---|---|
| **Pure WASM plugin** | WebAssembly module (`plugin.wasm`) compiled to `wasm32-wasip1` running inside an Extism/Wasmtime sandbox for pure in-process compute. |
| **Direct Native plugin** | Native binary (`rune-<name>-native`) executing as an unprivileged child process communicating directly with `rune-kit` over MCP Stdio JSON-RPC. |
| **Host-Managed Binary** | Third-party executable (`ffmpeg`, `yt-dlp`) downloaded and cryptographically verified by `rune-kit`. |
| **McpRouter** | Host dispatch component in `rune-kit-core` routing tools, resources, and prompts across WASM and native plugins. |
| **Tool** | Autonomous action/function with typed arguments and return value invoked by the model. |
| **Resource** | Addressable data at a URI (`rune://<namespace>/<path>`) representing read-only context. |
| **Prompt** | Argument-templated message sequence used to seed conversations and workflow pipelines. |
| **xtask** | Host-only build and test orchestrator for the workspace. |

***

*Document updated to reflect the removal of the intermediate WASM layer for native plugins in favor of first-class Direct Native MCP binaries.*