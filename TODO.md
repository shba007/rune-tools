# Rune Tools — Feature Todo

## 1. Plugin SDK & Framework Infrastructure
- [x] Workspace architecture supporting both WebAssembly (`wasm32-wasip1`) and Direct Native execution
- [x] `crates/rune-pdk`: Extism PDK wrapper providing typed definitions (`ToolDefinition`, `ResourceDefinition`, `PromptDefinition`), JSON-RPC macros, and `ToolRules`
- [x] `crates/rune-sidecar`: Supervised OS native process runner with Stdio JSON-RPC event loop, signal handling, and buffered I/O
- [x] Strict domain separation in all plugins (`definitions.rs` schemas, `operations.rs` pure logic, `types.rs` models)
- [x] Automated contract tests verifying parameter descriptions, camelCase naming, and error handling without requiring live runtimes
- [x] Scoped capability manifests (`plugin.toml`) defining filesystem roots, network hosts, and allowed binary commands

## 2. Active Pure WebAssembly Plugins (`wasm32-wasip1`)
- [x] **`rune-filesystem`** (13 tools):
  - [x] Sandboxed file operations: `read_file`, `write_file`, `append_file`, `edit_file`, `delete_file`
  - [x] Directory traversal: `list_dir`, `dir_tree`, `search_files`
  - [x] Management: `move_file`, `copy_file`, `file_stat`, `diff_files`, `watch_dir`
- [x] **`rune-fetch`** (1 tool, 1 resource, 2 prompts):
  - [x] Web fetch with HTML-to-Markdown conversion, pagination, and cursor markers (`fetch`)
  - [x] Help resource (`rune://fetch/help`)
  - [x] Prompt templates (`fetch_markdown`, `fetch_summary`)
- [x] **`rune-memory`** (9 tools):
  - [x] Knowledge graph entity operations: `create_entities`, `delete_entities`
  - [x] Relation management: `create_relations`, `delete_relations`
  - [x] Observation tracking: `add_observations`, `delete_observations`
  - [x] Graph inspection & search: `read_graph`, `search_nodes`, `open_nodes`
- [x] **`rune-sequential-thinking`** (1 tool):
  - [x] Dynamic step-by-step reasoning, hypothesis revision, thought branching, and score tracking (`sequential_thinking`)
- [x] **`rune-time`** (2 tools):
  - [x] Current timestamp inspection (`current_time`)
  - [x] Timezone conversion and difference calculation (`convert_time`)

## 3. Active Direct Native Plugins (Stdio JSON-RPC)
- [x] **`rune-browser`** (17 tools):
  - [x] Browser process lifecycle: `launch`, `close`
  - [x] Navigation & tabs: `navigate`, `tabs`, `new_tab`, `close_tab`, `switch_tab`
  - [x] DOM & interaction: `click`, `type_text`, `press_keys`, `scroll`, `hover`
  - [x] Inspection & capture: `screenshot`, `html`, `evaluate_js`, `cookies`, `storage`
- [x] **`rune-git`** (13 tools):
  - [x] Repository status: `git_status`, `git_diff`, `git_log`, `git_show`
  - [x] Working tree modifications: `git_add`, `git_commit`, `git_stash`, `git_clean`, `git_reset`
  - [x] Branching & tags: `git_branch`, `git_checkout`, `git_tag`
  - [x] Authorship tracking: `git_blame`
- [x] **`rune-figma`** (44 tools, 3 resources, 6 prompts):
  - [x] WebSocket bridge (`ws://localhost:3055`) to live Figma canvas
  - [x] Document and page traversal: `get_document_info`, `list_pages`, `create_page`
  - [x] Shape and frame creation: `create_rectangle`, `create_frame`, `create_text`, `create_component`
  - [x] Node manipulation: `update_node`, `delete_node`, `set_fills`, `set_strokes`, `set_effects`
  - [x] Export capabilities: `export_node_as_image`, `export_page_as_svg`
- [x] **`rune-audio`** (2 tools):
  - [x] Audio format transcoding and slicing via `ffmpeg`
  - [x] Audio stream downloading via `yt-dlp` and `spotdl`
- [x] **`rune-video`** (5 tools):
  - [x] Video metadata extraction, clip trimming, frame extraction, transcoding, and concatenation via `ffmpeg` and `yt-dlp`
- [x] **`rune-image`** (5 tools):
  - [x] Image conversion, resizing, cropping, filtering, and media scraping via `gallery-dl`
- [x] **`rune-email`** (11 tools):
  - [x] IMAP mailbox connection, listing, search, message reading, moving, and deletion
  - [x] SMTP message drafting and delivery
- [x] **`rune-slides`** (5 tools):
  - [x] Presentation creation, slide updating, markdown theme parsing, previewing, and export
- [x] **`mhb-mconnect`** (3 tools):
  - [x] External REST API integration and data synchronization

## 4. Build Orchestration & Verification (`cargo xtask`)
- [x] Multi-target build orchestrator:
  - [x] Single plugin compilation (`cargo xtask build <plugin>`) with `--wasm-only` or `--native-only`
  - [x] Workspace-wide compilation (`cargo xtask build-all`)
  - [x] Automated contract and unit test suite runner (`cargo xtask test <plugin>`, `test-all`)
  - [x] Pinned Cocogitto release preparation (`cargo xtask prepare`, `example`, `notes`)
- [x] Streamable HTTP acceptance test harnesses (`smoke-kit`, `smoke-filesystem`) verifying endpoint discovery, mirrored headers, and SSE streaming responses

## 5. Draft / Inactive Plugins & Roadmap
- [ ] Finish and activate draft plugin **`rune-print`** (CUPS and WinSpool OS print queue management)
- [ ] Finish and activate draft plugin **`rune-scan`** (eSCL AirScan and mDNS scanner discovery)
- [ ] Finish and activate draft plugin **`rune-ssh`** (SSH shell and SFTP remote file transfer)
- [ ] Automated external binary dependency verification runner for macOS and Linux host targets
- [ ] Heartbeat health check and automatic reconnect loop for `rune-figma` WebSocket connections

## 6. Direction (see Goal.md)
- [ ] Review tool descriptions and schemas of the core plugins (filesystem, fetch, browser, git, email, time) for correct calls by small local models
- [ ] Prefer hosting an existing MCP server through Kit over a new first-party plugin, once Kit hosts stdio servers
- [ ] Move `rune-memory` storage to Rune Lake (thin tool surface, or replaced by Lake's tools)
- [ ] Return what changed from tools that modify something, so Studio can show before/after and offer undo
- [ ] `rune-browser`: act on page structure and the accessibility tree before pixels
- [ ] Decide which current plugins are used enough to keep maintaining
- [ ] Declare an output schema, with display hints where needed, on every tool
- [ ] RAW development tool taken from Rune Studio's Portfolio flow
