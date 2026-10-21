<div align="center">

# Rune Tools

**Modular, Sandboxed Model Context Protocol (MCP) Plugins for AI Runtimes**

[![CI](https://github.com/shba007/rune-tools/actions/workflows/integrate.yml/badge.svg)](https://github.com/shba007/rune-tools/actions/workflows/integrate.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust: 2024 Edition](https://img.shields.io/badge/Rust-2024%20Edition-DEA584.svg?logo=rust)](Cargo.toml)
[![MCP Protocol: 2024-11-05](https://img.shields.io/badge/MCP%20Protocol-2024--11--05-8A2BE2.svg)](https://modelcontextprotocol.io)
[![WASM Target: wasm32-wasip1](https://img.shields.io/badge/WASM%20Target-wasm32--wasip1-654FF0.svg?logo=webassembly)](Cargo.toml)

</div>

---

## Overview

The **Rune Tools** ecosystem is a Rust-based suite of Model Context Protocol (MCP) plugins engineered for autonomous AI agents and supervisor runtimes (such as `rune-kit`). It cleanly isolates capability code from the trusted host using two first-class execution models: **Pure WASM plugins** (`wasm32-wasip1`) executing inside memory-isolated Extism/Wasmtime sandboxes for pure in-process compute, and **Direct Native plugins** communicating over Stdio JSON-RPC 2.0 to drive external binaries and OS-level operations without intermediate serialization hops.

Every plugin enforces a strict internal separation between declarative MCP schemas (`definitions.rs`), pure domain operations (`operations.rs`), and data types (`types.rs`), backed by automated contract tests that validate parameter descriptions, camelCase properties, and required-field constraints.

### Features

- 🛡️ **Dual Execution Architecture**: Pure WASM sandboxing via Extism/Wasmtime for zero-host-side-effect compute, paired with Direct Native processes for persistent sockets and hardware workloads.
- 🔌 **Strict MCP Protocol Compliance**: Implements the Model Context Protocol (2024-11-05 specification) across `tools`, `resources`, and `prompts` with automated contract and schema validation.
- 🗂️ **Comprehensive Domain Catalog**: 17 modular plugins (14 active workspace members, 3 draft/inactive) covering sandboxed filesystem manipulation, media transcoding (`ffmpeg`, `yt-dlp`), browser automation, Git inspection, IMAP/SMTP email, knowledge graphs, time conversions, and Figma canvas design. Note: `game-engine` does not exist.
- ⚙️ **Unified Build & Test Orchestrator**: Integrated `cargo xtask` runner coordinating dual-target compilation (`wasm32-wasip1` cdylibs and host-native binaries) alongside isolated environment testing via `dotenvx`.
- 🔐 **Capability-Based Security**: Scoped filesystem access, explicit hostname allowlists, and host-managed external binary provisioning (`ffmpeg`, `gallery-dl`, `yt-dlp`, `git`) declared in `plugin.toml`.
- 📦 **Automated Attested Releases**: GitHub Actions CI/CD with build provenance attestation, multi-platform binary packaging (Linux, macOS Apple Silicon/Intel, Windows), and central registry indexing.

---

## Architecture & Execution Models

`rune-kit` and other MCP-compliant hosts route incoming tool, resource, and prompt requests to either an in-process WebAssembly instance or a supervised native child process:

```text
                               ┌─────────────────────────────────────────────────────────────┐
                               │                    Host Runtime (rune-kit)                  │
                               │  (Dispatches MCP requests, provisions external CLI binaries)│
                               └──────────────┬──────────────────────────────┬───────────────┘
                                              │                              │
                     MCP Protocol / WASM FFI  │                              │  MCP Protocol / Stdio JSON-RPC
                                              ▼                              ▼
                 ┌────────────────────────────────────────┐     ┌────────────────────────────────────────┐
                 │          plugin.wasm (WASM)            │     │       rune-<name>-native (Native)      │
                 │   • Target: wasm32-wasip1              │     │   • Target: Host Platform Binary       │
                 │   • Extism/Wasmtime memory sandbox     │     │   • Direct OS & socket access          │
                 │   • Pure compute, graph state, parsing │     │   • Duplex WebSockets, TCP, SSH        │
                 │   • Strict filesystem root containment │     │   • Child processes (ffmpeg, yt-dlp)   │
                 └────────────────────────────────────────┘     └────────────────────────────────────────┘
```

| Dimension | Pure WASM Plugin | Direct Native Plugin |
|---|---|---|
| **Compilation Target** | `wasm32-wasip1` (`cdylib`) | Host platform executable (`rlib` + `bin`) |
| **Runtime Isolation** | Memory-isolated Extism/Wasmtime sandbox | Unprivileged OS child process over standard I/O |
| **Communication** | WASM FFI (`mcp_call_tool`, `mcp_list_tools`) | Stdio newline-delimited JSON-RPC 2.0 |
| **Optimal For** | In-process compute: filesystem walk/edit, knowledge graphs, time conversions, text parsing | Hardware access: WebSockets, TCP sockets, browser automation, CLI subprocesses (`ffmpeg`, `git`) |
| **Artifact Output** | `target/wasm32-wasip1/release/<name>.wasm` | `target/release/<name>-native` (or `.exe` on Windows) |

---

## Plugin Catalog

### Active Workspace Plugins (14)

| Plugin | Version | Execution Model | External Binaries / Protocols | Tools | Resources & Prompts | Docs |
|:---|:---:|:---:|:---|:---:|:---|:---:|
| [`rune-filesystem`](plugins/rune-filesystem/) | `0.3.5` | Pure WASM | None | 13 | — | [Directory](plugins/rune-filesystem/) |
| [`rune-fetch`](plugins/rune-fetch/) | `0.3.2` | Pure WASM | None (host HTTP FFI) | 1 | 1 Resource (`rune://fetch/help`), 2 Prompts | [Directory](plugins/rune-fetch/) |
| [`rune-memory`](plugins/rune-memory/) | `0.2.2` | Pure WASM | None | 9 | — | [README](plugins/rune-memory/README.md) |
| [`rune-sequential-thinking`](plugins/rune-sequential-thinking/) | `0.2.0` | Pure WASM | None | 1 | — | [README](plugins/rune-sequential-thinking/README.md) |
| [`rune-time`](plugins/rune-time/) | `0.2.2` | Pure WASM | None | 2 | — | [README](plugins/rune-time/README.md) |
| [`rune-audio`](plugins/rune-audio/) | `0.2.3` | Direct Native | `ffmpeg`, `yt-dlp`, `spotdl` | 2 | — | [README](plugins/rune-audio/README.md) |
| [`rune-browser`](plugins/rune-browser/) | `0.2.4` | Direct Native | `agent-browser`, Chrome DevTools Protocol | 17 | — | [README](plugins/rune-browser/README.md) |
| [`rune-figma`](plugins/rune-figma/) | `0.2.1` | Direct Native | WebSocket (`ws://localhost:3055`) | 44 | 3 Resources, 6 Prompts | [Directory](plugins/rune-figma/) |
| [`rune-email`](plugins/rune-email/) | `0.1.5` | Direct Native (Dual) | IMAP / SMTP | 11 | — | [README](plugins/rune-email/README.md) |
| [`rune-git`](plugins/rune-git/) | `0.2.4` | Direct Native | `git` (host-managed) | 13 | — | [README](plugins/rune-git/README.md) |
| [`rune-image`](plugins/rune-image/) | `0.2.2` | Direct Native | `gallery-dl` | 5 | — | [README](plugins/rune-image/README.md) |
| [`rune-slides`](plugins/rune-slides/) | `0.2.0` | Direct Native (Dual) | None | 5 | — | [README](plugins/rune-slides/README.md) |
| [`rune-video`](plugins/rune-video/) | `0.2.0` | Direct Native | `ffmpeg`, `yt-dlp`, `streamlink` | 5 | — | [README](plugins/rune-video/README.md) |
| [`mhb-mconnect`](plugins/mhb-mconnect/) | `0.1.3` | Direct Native (Dual) | REST API | 3 | — | [README](plugins/mhb-mconnect/README.md) |

### Draft / Inactive Plugins (3)

| Plugin | Version | Execution Model | External Binaries / Protocols | Tools | Status | Docs |
|:---|:---:|:---:|:---|:---:|:---|:---:|
| [`rune-print`](plugins/rune-print/) | `0.2.0` | Direct Native | OS Print Spooler (CUPS / WinSpool) | 4 | Draft (commented out in `Cargo.toml`) | [README](plugins/rune-print/README.md) |
| [`rune-scan`](plugins/rune-scan/) | `0.1.0` | Direct Native | eSCL AirScan / mDNS | 2 | Draft (commented out in `Cargo.toml`) | [README](plugins/rune-scan/README.md) |
| [`rune-ssh`](plugins/rune-ssh/) | `0.1.0` | Direct Native | SSH / SFTP | 3 | Draft (commented out in `Cargo.toml`) | [README](plugins/rune-ssh/README.md) |

> [!NOTE]
> - **Active vs Draft Plugins**: The 14 active plugins are fully tested and built as part of the Cargo workspace. `rune-print`, `rune-scan`, and `rune-ssh` are preserved in the repository in `plugins/` as reference implementations but are commented out in `Cargo.toml` workspace members.
> - **Non-existent Plugins**: `game-engine` does **not** exist in this repository.
> - **Naming**: The Figma canvas integration is `rune-figma` (previously referenced in some draft documentation as `rune-design`).

---

## Build Orchestration (`cargo xtask`)

The repository includes a dedicated build orchestrator in `xtask/` accessible through the Cargo alias `cargo xtask` (configured in `.cargo/config.toml`). Because WASM cdylib compilation (`wasm32-wasip1`) and host-native compilation require separate Cargo target passes, `xtask` coordinates dual-target builds and tests across the entire workspace with typed error propagation.

### Invocation Commands

```bash
# Build a single plugin (compiles WASM and/or Native based on package targets)
cargo xtask build <plugin-name>

# Build only the WASM artifact for a plugin
cargo xtask build <plugin-name> --wasm-only

# Build only the native sidecar binary for a plugin
cargo xtask build <plugin-name> --native-only

# Build all discovered plugins in the workspace
cargo xtask build-all

# Run contract and unit tests for a single plugin (loads plugins/<name>/.env via dotenvx)
cargo xtask test <plugin-name>

# Run contract and unit tests across all workspace plugins
cargo xtask test-all
```

### Options & Flags Reference

| Flag | Type | Default | Description |
|---|---|---|---|
| `<plugin-name>` | `string` | *(Required for `build` & `test`)* | Name of the target plugin directory under `plugins/` (e.g. `rune-filesystem`, `rune-audio`). |
| `--wasm-only` | `boolean` | `false` | Restricts compilation exclusively to the `wasm32-wasip1` target (`target/wasm32-wasip1/release/<artifact>.wasm`). |
| `--native-only` | `boolean` | `false` | Restricts compilation exclusively to the host-native sidecar binary (`target/release/<name>-native`) with `--features native`. |

---

## Programmatic Usage & Integration

### 1. Running Native Sidecars via MCP Stdio (Client Configuration)

Direct Native plugins can be registered directly with any standard MCP host (such as `rune-kit`, Claude Desktop, or Cursor) as stdio servers:

```json
{
  "mcpServers": {
    "rune-git": {
      "command": "/absolute/path/to/target/release/rune-git-native",
      "args": [],
      "env": {
        "REPO_PATH": "/path/to/git/repository"
      }
    },
    "rune-audio": {
      "command": "/absolute/path/to/target/release/rune-audio-native",
      "args": [],
      "env": {
        "FFMPEG_PATH": "/usr/local/bin/ffmpeg",
        "YT_DLP_PATH": "/usr/local/bin/yt-dlp",
        "ALLOWED_DIR": "/path/to/output/media"
      }
    }
  }
}
```

#### Stdio JSON-RPC 2.0 Protocol Flow

Native sidecars listen on `stdin` for newline-delimited JSON-RPC 2.0 requests and write responses to `stdout`:

```json
// 1. Client sends initialize
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05"}}

// 2. Client lists available tools
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}

// 3. Client calls an MCP tool
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"git_status","arguments":{"repoPath":"."}}}

// 4. Plugin responds with structured output
{"jsonrpc":"2.0","id":3,"result":{"content":[{"type":"text","text":"{\"clean\":true,\"branch\":\"main\",\"staged\":[],\"unstaged\":[]}"}]}}
```

### 2. Loading Pure WASM Plugins via Extism (Rust Host)

Pure WASM plugins run within an Extism/Wasmtime sandbox and expose standard FFI exports:

```rust
use extism::{Manifest, Plugin, Wasm};
use serde_json::{Value, json};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let wasm_path = PathBuf::from("target/wasm32-wasip1/release/rune_filesystem.wasm");
    
    // Configure sandbox memory and directory access permissions
    let manifest = Manifest::new([Wasm::file(wasm_path)])
        .with_config([("allowed_dir".to_string(), ".".to_string())])
        .with_allowed_path(".", "/");

    let mut plugin = Plugin::new(&manifest, [], true)?;

    // List exported tools
    let tools_raw = plugin.call::<(), &str>("mcp_list_tools", ())?;
    let tools: Value = serde_json::from_str(tools_raw)?;
    println!("Exported tools: {}", tools.as_array().map_or(0, |a| a.len()));

    // Call an MCP tool (e.g. read_text_file with line-based paging)
    let call_payload = json!({
        "name": "read_text_file",
        "arguments": {
            "path": "Cargo.toml",
            "head": 10
        }
    }).to_string();

    let output_raw = plugin.call::<&str, &str>("mcp_call_tool", &call_payload)?;
    let response: Value = serde_json::from_str(output_raw)?;

    if response["status"] == "success" {
        println!("Content:\n{}", response["result"]["content"].as_str().unwrap());
    } else {
        eprintln!("Error: {}", response["error"]);
    }

    Ok(())
}
```

### 3. In-Process Domain Invocation (Rust PDK API)

All plugins isolate domain operations in `operations.rs`, allowing direct consumption in Rust applications:

```rust
use rune_pdk::ToolCallRequest;
use serde_json::json;

fn main() -> Result<(), String> {
    // 1. Filesystem: Paged text file read
    let read_req = ToolCallRequest {
        name: "read_text_file".to_string(),
        arguments: json!({
            "path": "Cargo.toml",
            "lineOffset": 0,
            "lineLimit": 25
        }),
    };
    let read_res = rune_filesystem::operations::execute_tool(read_req)?;
    println!("Lines read: {}", read_res["linesReturned"]);

    // 2. Time: Timezone query and conversion
    let time_req = ToolCallRequest {
        name: "convert_time".to_string(),
        arguments: json!({
            "sourceTimezone": "UTC",
            "targetTimezone": "Asia/Tokyo",
            "time": "2026-10-16 12:00:00"
        }),
    };
    let time_res = rune_time::operations::execute_tool(time_req)?;
    println!("Converted: {}", time_res["target_timezone"]["datetime"]);

    // 3. Sequential Thinking: Step-by-step reasoning
    let think_req = ToolCallRequest {
        name: "sequential_thinking".to_string(),
        arguments: json!({
            "thought": "Evaluate memory requirements for batch processing",
            "nextThoughtNeeded": true,
            "thoughtNumber": 1,
            "totalThoughts": 3
        }),
    };
    let think_res = rune_sequential_thinking::operations::execute_tool(think_req)?;
    println!("Status: {}", think_res["status"]);

    Ok(())
}
```

---

## Security & Capability Manifests

Every plugin declares its resource requirements in a top-level `plugin.toml` manifest. The host runtime enforces these permissions before executing requests:

```toml
[plugin]
name = "rune-video"
version = "0.2.0"
execution_model = "native"
description = "Video streaming, playlist extraction, and media trimming powered by yt-dlp and ffmpeg."

[capabilities]
network_hosts = [
  "youtube.com",
  "www.youtube.com",
  "twitch.tv",
  "kick.com"
]
filesystem = { mode = "scoped", root_param = "allowed_dir" }
exec = { allowed_binaries = ["ffmpeg", "yt-dlp", "streamlink"] }

[dependencies.binaries.ffmpeg]
version = ">=6.0"
optional = false
description = "Video slicing, lossy/lossless stream clipping, and thumbnail generation"

[dependencies.binaries.yt-dlp]
version = ">=2024.01.01"
optional = false
description = "Video streaming and playlist metadata extraction"
```

### Security Enforcement Principles

1. **Scoped Filesystem Roots**: Pure WASM plugins verify canonical path containment against `ALLOWED_DIR` (or the Extism `allowed_dir` configuration). Attempts to traverse directory boundaries with `../` or absolute symlinks are rejected with structured errors.
2. **Explicit Network Host Allowlist**: Network operations are restricted strictly to declared domains; untrusted outbound requests fail at the sandbox boundary.
3. **Host-Managed Binaries**: Third-party CLI tools (`ffmpeg`, `yt-dlp`, `gallery-dl`, `git`) are not bundled or downloaded by plugins. `rune-kit` verifies, caches, and provides validated executable paths to plugins via `<PROGRAM>_PATH` environment variables.

---

See [plugin contracts and integration limits](docs/PLUGIN-CONTRACT.md) for tool rules, read-only classification, persistence and cancellation limits.

## Development & Contribution

<details>
<summary><strong>Local Development Guide</strong></summary>

### Prerequisites

- **Rust Toolchain**: Stable release (`edition = "2024"`).
- **WebAssembly Target**: Install the WASI target:
  ```bash
  rustup target add wasm32-wasip1
  ```
- **Node.js**: LTS release (required for `@dotenvx/dotenvx` during tests).
- **Dotenvx**: Install globally for environment variable injection:
  ```bash
  npm install -g @dotenvx/dotenvx
  ```
- **Cocogitto & Gitleaks**: Conventional Commits enforcement and secret scanning in the git hooks (`.cargo-husky/hooks`):
  ```bash
  cargo install cocogitto
  cargo xtask hooks   # install the pre-commit and commit-msg hooks
  ```

### Workspace Commands

```bash
# Check code formatting across all crates
cargo fmt --all -- --check

# Apply formatting fixes
cargo fmt --all

# Run Clippy with warnings treated as errors
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Run all plugin unit and contract tests
cargo xtask test-all

# Run tests for a specific plugin
cargo xtask test rune-filesystem

# Build all plugin artifacts (WASM modules and native binaries)
cargo xtask build-all
```

### Creating a New Plugin

1. Create a directory `plugins/rune-<name>/` with standard module separation:
   ```text
   plugins/rune-<name>/
   ├── Cargo.toml          # Package definition with rune-pdk dependency
   ├── plugin.toml         # Capability manifest and dependencies
   ├── .env                # Local test environment variables
   ├── src/
   │   ├── lib.rs          # WASM FFI exports or module declarations
   │   ├── definitions.rs  # Pure ToolDefinition, ResourceDefinition, PromptDefinition
   │   ├── operations.rs   # Domain execution logic: execute_tool(ToolCallRequest)
   │   └── types.rs        # Serde request/response types
   └── tests/
       ├── contract_tests.rs   # test_plugin_contract!(tool_definitions, execute_tool)
       └── operations_tests.rs # Unit tests for domain logic
   ```
2. For Direct Native plugins requiring child processes or sockets, add a `[[bin]]` entry in `Cargo.toml` pointing to `src/bin/native_sidecar.rs` and implement `rune_sidecar::SidecarHandler`.
3. Add the crate path to the root `Cargo.toml` workspace `members` list.
4. Run `cargo xtask test rune-<name>` to verify contract compliance.

</details>

---

### License & Contributors

Published under the [MIT](LICENSE) license.

<br>

<a href="https://github.com/shba007/rune-tools/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=shba007/rune-tools" alt="Contributors">
</a>
