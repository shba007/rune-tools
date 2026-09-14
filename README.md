# Rune Ecosystem

## Overview

The Rune Ecosystem is a Rust-based MCP (Model Context Protocol) plugin architecture that provides modular, sandboxed execution capabilities for AI models. It separates untrusted/variable capability code from the trusted host using two execution models: **WASM plugins** for pure compute operations and **Native sidecars** for external binary execution.

### Key Benefits

- **Security**: Memory isolation and capability-based access control
- **Flexibility**: Support for both WASM compute and native execution
- **Modularity**: Plugin-based architecture with uniform interface
- **Extensibility**: Easy to add new capabilities without host modifications

## Quick Start

### Prerequisites

- Rust toolchain (stable)
- Docker (optional, for build isolation)

### Installation

```bash
# Clone the repository
cargo clone https://github.com/shba007/rune-tools

# Navigate to the workspace
cd rune-tools

# Test the environment (all plugins)
cargo xtask test-all

# Build all plugins
cargo xtask build-all
```

### Using Plugins

The ecosystem provides plugins for various capabilities:

#### Filesystem Operations
```rust
use rune_filesystem::{create_file, delete_file, list_directory};

// Example usage
let result = create_file("/path/to/file.txt", "content").await;
```

#### Git Management
```rust
use rune_git::{git_status, git_commit, git_log};

// Example usage
let status = git_status("/repo/path").await;
```

#### Audio Processing
```rust
use rune_audio::{extract_audio, convert_format};

// Example usage
let audio_path = extract_audio("https://example.com/audio.mp3").await;
```

## Plugin Architecture

### Execution Models

| Plugin Type | Execution Model | Use Case |
|-------------|----------------|---------|
| **WASM Plugins** | `wasm32-wasip1` sandbox | Pure compute (parsing, encoding, filesystem operations) |
| **Native Sidecars** | Persistent child process | External binary execution, native libraries |

## Available Plugins

| Plugin | Description | Tools | Documentation |
| :--- | :--- | :--- | :--- |
| `rune-audio` | Audio stream extraction and transcoding (MP3, FLAC, WAV, M4A, Opus), plus music track/album/playlist ingestion with ID3 metadata and synced LRC lyrics via spotdl. | `extract_audio_track`, `download_music_track` | [README](plugins/rune-audio/README.md) |
| `rune-browser` | Headless browser automation: navigation, element interaction, forms, JS evaluation, screenshots, PDF export, network/console capture, persistent session profiles. | `browser_navigate`, `browser_screenshot`, … | [README](plugins/rune-browser/README.md) |
| `rune-email` | Universal IMAP/SMTP email client: mailbox listing, search, message parsing to Markdown, attachments, send/reply/draft, flags, and moves. | `list_mailboxes`, `read_message`, … | [README](plugins/rune-email/README.md) |
| `rune-fetch` | Web page fetching with HTML-to-Markdown conversion (or raw text) and character-level pagination via cursor semantics. | `fetch` | [README](plugins/rune-fetch/README.md) |
| `rune-filesystem` | Sandboxed filesystem manipulation: line-based reading, media reads, atomic writes, line edits with dry-run diffs, tree views, glob search, safe moves. | `read_text_file`, `write_file`, … | [README](plugins/rune-filesystem/README.md) |
| `rune-git` | Git repository management: status, diff, staging, committing, branching, switching, merging, log, remotes, fetch/pull/push. | `git_status`, `git_commit`, … | [README](plugins/rune-git/README.md) |
| `rune-image` | Image gallery and social album extraction via gallery-dl (Reddit, Instagram, Imgur, Pixiv, more) with cookies, browser sessions, proxy routing. | `inspect_image_gallery`, `download_image_collection` | [README](plugins/rune-image/README.md) |
| `mhb-mconnect` | A Modest Human Brands (MHB) external interaction API connector MCP server that manages email templates over a REST backend. Lists all available email templates, fetches a specific template definition along with its required variable schema by ID, and renders a fully styled HTML email preview from supplied variables. All requests are proxied through a native sidecar (`mhb-mconnect-native`) targeting the configured MHB base URL. | `mhb_list_templates`, `mhb_get_template`, `mhb_render_template_preview` | [README](plugins/mhb-mconnect/README.md) |
| `rune-memory` | Persistent knowledge-graph memory: typed entities and relations in a JSON file, batch creation, filtered queries, deletion, full inspection. | `create_entities`, `query_memory`, … | [README](plugins/rune-memory/README.md) |
| `rune-print` | Native OS printing and eSCL AirScan scanning: printer discovery, spooler dispatch for TXT/PDF/images, flatbed/ADF scans, capability queries. | `printer_list_printers`, `printer_print_document`, … | [README](plugins/rune-print/README.md) |
| `rune-sequential-thinking` | Dynamic step-by-step reasoning workspace with progress tracking, hypothesis revision, thought branching, automatic total adjustment. | `sequential-thinking` | [README](plugins/rune-sequential-thinking/README.md) |
| `rune-scan` | Advanced file and content scanning with pattern matching, validation, and analysis capabilities. | `scan_directory`, `validate_content`, … | [README](plugins/rune-scan/README.md) |
| `rune-slides` | Presentation creation and management with slide deck generation and editing capabilities. | `create_presentation`, `edit_slide`, … | [README](plugins/rune-slides/README.md) |
| `rune-ssh` | Secure SSH command execution and SFTP file transfers via native sidecar. | `ssh_run_command`, `sftp_upload`, `sftp_download` | [README](plugins/rune-ssh/README.md) |
| `rune-time` | Deterministic timezone queries, ISO-8601 formatting, and DST-aware cross-timezone conversions. | `get_current_time`, `convert_time` | [README](plugins/rune-time/README.md) |
| `rune-video` | Video streaming, playlist extraction, live broadcast recording, and media trimming powered by yt-dlp, streamlink, ffmpeg. | `inspect_video_metadata`, `download_video_stream`, … | [README](plugins/rune-video/README.md) |

## Development

### Building Plugins

#### Build Commands
```bash
# Test a single plugin
cargo xtask test rune-<name>

# Build a single plugin
cargo xtask build rune-<name>

# Build all plugins
cargo xtask build-all
```

#### Development Workflow

1. **Plugin Development**: Create or modify plugins in `plugins/` directory
2. **Testing**: Use contract tests and operations tests
3. **Build**: Use `xtask` orchestrator for consistent builds
4. **Integration**: Test plugins in the full ecosystem

### Plugin Development

#### Plugin Structure
```rust
plugins/rune-<name>/
├── Cargo.toml          # Plugin configuration
├── .env               # Environment variables
├── src/
│   ├── lib.rs          # WASM FFI boundary
│   ├── definitions.rs  # Tool/resource/prompt schemas
│   ├── operations.rs   # Domain logic
│   └── types.rs        # Request/response types
└── tests/
    ├── contract_tests.rs  # Schema/routing tests
    └── operations_tests.rs # Domain logic tests
```

#### Development Commands
```bash
# Install pre-commit hooks
cargo install cargo-pre-commit
cargo-pre-commit install-hooks

# Format code
cargo fmt --all

# Lint code
cargo clippy --fix

# Run tests
cargo test -p rune-<name>
```

## Configuration

### Environment Variables

Plugins use environment variables for configuration:

#### Common Variables
```bash
# Plugin namespace
RUNE_PLUGIN_NAMESPACE=<plugin-name>

# Filesystem access permissions
RUNE_FILESYSTEM_ALLOWED_DIR=/allowed/path

# Network access
RUNE_NETWORK_HOSTS=host1,host2,host3
```

#### Security Settings
```bash
# Capability manifests
RUNE_CAPABILITIES_NETWORK_HOSTS=[]
RUNE_CAPABILITIES_FILESYSTEM_MODE="scoped"
RUNE_CAPABILITIES_EXEC_ALLOWED_BINARIES=[]
```

### Configuration Files

Plugins use `Cargo.toml` for Rust configuration:

```toml
[package]
name = "rune-<name>"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
rune-pdk = { path = "../../crates/rune-pdk" }
serde = { workspace = true }
serde_json = { workspace = true }
```

## Security

### Capability System

Every plugin declares exact capabilities it needs. The host grants only what's declared:

```toml
[capabilities]
network_hosts = []                    # Explicit hostnames only
filesystem = { mode = "scoped", root_param = "allowed_dir" }
exec = { allowed_binaries = [] }      # Empty by default
```

### Security Features

- **Memory Isolation**: WASM plugins run in Extism/Wasmtime sandbox
- **Process Isolation**: Native sidecars run as child processes
- **Capability Limits**: No unrestricted access to system resources
- **Panic Protection**: `std::panic::catch_unwind` wraps all calls

## Deployment

### Release Process

Plugins are released as individual artifacts:

```bash
# Build a specific plugin
cargo xtask build rune-audio --native-only

# Build all plugins
cargo xtask build-all
```

### Plugin Registry

Built plugins are published to the plugin registry:

```bash
# Update plugin registry
cargo xtask publish-all
```

## API Reference

The ecosystem provides a uniform interface for all plugins:

### Common MCP Primitives

#### Tools
- `list_tools`: List available tools
- `call_tool`: Execute a tool

#### Resources  
- `list_resources`: List available resources
- `read_resource`: Read resource data

#### Prompts
- `list_prompts`: List available prompts
- `get_prompt`: Get prompt template

### Tool Convention

Tools follow a consistent naming pattern:

```rust
// Tool definition
pub fn tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "create_file".to_string(),
            description: "Create a new file".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "content": { "type": "string" }
                }
            })
        }
    ]
}
```

## Troubleshooting

### Common Issues

#### Plugin Not Found
```bash
# Check if plugin exists in workspace
ls plugins/

# Verify in Cargo.toml
 cargo xtask test rune-<name>
```

#### Build Failures
```bash
# Clean and rebuild
cargo clean
cargo xtask build rune-<name>
```

#### Test Failures
```bash
# Run specific plugin tests
cargo test -p rune-<name>

# Run all tests
cargo xtask test-all
```

#### Environment Issues
```bash
# Load environment variables
dotenvx run --f ./plugins/rune-<name>/.env -- cargo test
```

## Contributing

### Code Quality

- **Formatting**: `cargo fmt --all`
- **Linting**: `cargo clippy --fix`
- **Testing**: `cargo test -p rune-<name>`

### Commit Guidelines

Follow conventional commits:

```
m feat: add new plugin functionality
m fix: resolve security vulnerability
m docs: update documentation
m style: format code
m refactor: improve code structure
m test: add test coverage
m chore: maintenance tasks
```

### Pull Requests

1. **Branch naming**: `feature/<name>` or `fix/<name>`
2. **PR description**: Include test results and screenshots if applicable
3. **Review**: Ensure all CI checks pass before merging

## Support

### Community

- **GitHub Issues**: Report bugs and request features
- **Discussions**: General community questions
- **Documentation**: API references and guides

### Resources

- **Architecture Documentation**: AGENTS.md for internal conventions
- **Development Guide**: Plugin development best practices
- **Security Documentation**: Capability and isolation policies

## License

This project is licensed under the MIT License. See `LICENSE` for details.

---

## Getting Help

For more detailed information, refer to:

- **AGENTS.md**: Complete architectural and implementation documentation
- **crates/**: Core library source code
- **plugins/**: Individual plugin implementations
- **xtask/**: Build orchestration tools