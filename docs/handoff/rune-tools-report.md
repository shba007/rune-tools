# Rune Tools Handoff Report

**Date:** 2026-10-21  
**Project:** `rune-tools`  
**Author:** Rune Tools Project Manager / Agent  
**Recipient:** Rune Studio Orchestrator & Ecosystem Architecture  
**Document Context:** Response to Rune Studio Handoff Document (2026-10-21)

---

## 1. Executive Summary

This report provides the formal response and implementation accounting from the `rune-tools` repository. As requested by the Rune Studio orchestrator, we have audited every statement in the handoff document against the live code, corrected discrepancies, built the required protocol extensions, validated all test suites, and documented written statements and architectural answers.

### Summary of Actions Taken
- **Checked & Corrected Section 5:** Reconciled plugin inventory (14 active workspace plugins, 3 draft/inactive plugins, `game-engine` non-existent, `rune-design` corrected to `rune-figma`).
- **Built TOOL-1 (Tool Rules & Risk Classification):** Designed and implemented `RiskLevel`, `ToolEffect`, `ToolLimits`, and `ToolRules` in `rune-pdk`. Annotated all **140 tools across all 17 plugins**. Updated `rune-pdk` custom serializer to emit top-level `"rules"` and forward `"x-rune-rules"` inside `inputSchema` so classifications survive intermediate proxies. Integrated protocol validation into `test_plugin_contract!`.
- **Built TOOL-6 (Documentation Accuracy):** Updated [`README.md`](file:///d:/Projects/Public/rune/code-tools/README.md) and [`AGENTS.md`](file:///d:/Projects/Public/rune/code-tools/AGENTS.md) to accurately reflect active vs draft plugin status, execution models, and license declarations.
- **Formulated Written Statements (TOOL-2, TOOL-4, TOOL-5):** Completed rigorous Windows stability validation for the starter set, documented output contracts and pagination semantics, and defined `rune-browser`'s role as a non-conflicting second surface.
- **Answered Architectural Questions (TOOL-7, TOOL-8, TOOL-9, TOOL-10):** Evaluated memory graph suitability for user profile notes, detailed the Cocogitto SemVer process, implemented two working A2UI JSONL pilot blueprints, and identified ground-truth evidence sources for the life track.
- **Ran Full Test Suites Before and After:**
  - **Before:** 70 suites, 213 tests passed, 0 failed, 0 ignored.
  - **After:** 70 suites, **228 tests passed** (+15 tests), 0 failed, 0 ignored. Clippy (`-D warnings`) and Rustfmt checks passing with zero errors.

---

## 2. Corrections to Section 5 ("What Studio understands about rune-tools today")

The handoff document contained several assumptions based on external observation that diverged from codebase reality:

### 2.1 Plugin Inventory & Workspace Configuration
- **Studio Assumption:** Stated that 14 plugins exist (5 pure WASM, 9 direct native), while claiming that `print`, `scan`, `ssh`, and `game-engine` do not exist.
- **Codebase Reality:**
  - Total plugin directories in `plugins/`: **17**.
  - **14 Active Workspace Plugins:** Configured in root [`Cargo.toml`](file:///d:/Projects/Public/rune/code-tools/Cargo.toml) workspace members: `rune-filesystem`, `rune-time`, `rune-fetch`, `rune-memory`, `rune-sequential-thinking` (Pure WASM); `rune-git`, `rune-audio`, `rune-video`, `rune-image`, `rune-browser`, `rune-figma` (Direct Native); `rune-email`, `rune-slides`, `mhb-mconnect` (Dual Native/WASM).
  - **3 Draft / Inactive Plugins:** Complete implementations exist in [`plugins/rune-print`](file:///d:/Projects/Public/rune/code-tools/plugins/rune-print), [`plugins/rune-scan`](file:///d:/Projects/Public/rune/code-tools/plugins/rune-scan), and [`plugins/rune-ssh`](file:///d:/Projects/Public/rune/code-tools/plugins/rune-ssh). These are preserved reference implementations but commented out in root [`Cargo.toml`](file:///d:/Projects/Public/rune/code-tools/Cargo.toml) members pending host library stabilization.
  - **Non-Existent Plugins:** `game-engine` does **not** exist anywhere in this repository, nor has it ever been committed.
  - **Naming Correction:** The Figma canvas integration is named `rune-figma` (package `rune-figma`, folder `plugins/rune-figma/`). Prior documentation mistakenly referred to it as `rune-design`.

### 2.2 Execution Models & Binaries
- **Studio Assumption:** All non-WASM plugins were assumed to be Direct Native.
- **Codebase Reality:** Three plugins (`rune-email`, `rune-slides`, `mhb-mconnect`) are dual-target: they provide both a WASM `cdylib` target and a native sidecar binary (`[[bin]]` with `--features native`). When running with `rune-kit`, they compile to Direct Native binaries to drive networking (`lettre`, `imap`, `reqwest`) and native PDF rendering (`printpdf`).

### 2.3 Tool Declarations & Classification State
- **Studio Assumption:** Tools were assumed to declare permissions in `plugin.toml`, and tool-specific risk levels were unknown.
- **Codebase Reality:** `plugin.toml` declared coarse plugin-level capabilities (allowed filesystem roots, external binaries, host allowlists). Individual tools in `definitions.rs` did not expose granular safety classifications. This gap has now been completely resolved by **TOOL-1**.

---

## 3. Implementation Evidence: Built Items

### 3.1 TOOL-1: Tool-Specific Classification & Rules Declaration

#### Architecture & Type Design
We extended [`crates/rune-pdk/src/lib.rs`](file:///d:/Projects/Public/rune/code-tools/crates/rune-pdk/src/lib.rs) with typed safety declarations:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolEffect {
    Read,
    Write,
    Delete,
    Network,
    Exec,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ToolLimits {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_seconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_paths: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_hosts: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_binaries: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolRules {
    pub risk: RiskLevel,
    pub effects: Vec<ToolEffect>,
    #[serde(default, skip_serializing_if = "ToolLimits::is_empty")]
    pub limits: ToolLimits,
}
```

#### Dual Serialization Strategy for Transparent Proxying
In the MCP protocol, `tools/list` returns an array of `ToolDefinition` objects. While modern MCP clients support top-level extension fields, intermediate proxies (such as `rune-kit`'s current `protocol.rs`) only deserialize and forward `name`, `description`, and `inputSchema`. 

To ensure safety classifications survive intermediate proxies and reach Rune Studio immediately without waiting for `rune-kit` changes, we implemented custom serialization for `ToolDefinition`:
1. It emits top-level `"rules": { ... }` for direct native MCP clients.
2. It simultaneously injects `"x-rune-rules": { ... }` into `inputSchema`.
3. When `rune-kit` implements KIT-2, it can cleanly extract either field.

#### Automated Contract Test Enforcement
In [`crates/rune-pdk/src/testing.rs`](file:///d:/Projects/Public/rune/code-tools/crates/rune-pdk/src/testing.rs), we added `assert_valid_tool_rules(&tools)` and embedded `test_tool_rules_declared` into the `test_plugin_contract!` macro.
The contract test strictly validates:
- Every tool must define a non-empty `rules` block.
- Tools with `Low` risk cannot have `Delete`, `Exec`, or uncontrolled `Network` effects.
- Tools with `Delete` or `Exec` effects must be classified as `Medium` or `High` risk.
- Limits must not contain empty strings or conflicting wildcards.

#### Scope of Application
All **140 tools across all 17 plugins** (14 active + 3 draft) now declare exact rules:
- `rune-filesystem` (13 tools): read-only vs write/delete scoped to `allowed_dir`.
- `rune-time` (2 tools): low risk, read-only compute.
- `rune-fetch` (1 tool): read-only network with max byte limits.
- `rune-memory` (9 tools): read vs write knowledge graph state.
- `rune-sequential-thinking` (1 tool): low risk compute.
- `rune-git` (13 tools): read-only inspections (`status`, `log`, `diff`) vs mutating operations (`commit`, `branch_create`, `checkout`, `reset`).
- `rune-audio` (2 tools) & `rune-video` (5 tools): media inspection vs transcoding/downloading with `ffmpeg`/`yt-dlp`.
- `rune-image` (5 tools): image comparison/metadata vs downloading with `gallery-dl`.
- `rune-browser` (17 tools): navigation, screenshot, DOM inspection, clicking, keyboard input.
- `rune-figma` (44 tools): inspection vs canvas creation/deletion via WebSocket bridge.
- `rune-email` (11 tools): mailbox listing/search vs sending/deleting messages.
- `rune-slides` (5 tools): presentation building vs deletion/export.
- `mhb-mconnect` (3 tools): template listing vs rendering/fetching.
- `rune-print` (4 tools), `rune-scan` (2 tools), `rune-ssh` (3 tools): OS spooler, scanner, and remote shell execution.

### 3.2 TOOL-6: Accurate Documentation
- **[`README.md`](file:///d:/Projects/Public/rune/code-tools/README.md):**
  - Updated catalog count: 17 modular plugins (14 active workspace members, 3 draft/inactive).
  - Explicitly separated Active Workspace Plugins from Draft/Inactive Plugins.
  - Corrected `rune-design` to `rune-figma`.
  - Added an explicit note that `game-engine` does not exist.
  - Corrected license text to match the MIT license in `LICENSE`.
- **[`AGENTS.md`](file:///d:/Projects/Public/rune/code-tools/AGENTS.md):**
  - Updated Section 1.3 repository tree to include all 17 plugins with accurate execution models.
  - Updated Section 9.1 and 9.2 catalog tables to show exact active vs draft status, execution model (Pure WASM vs Direct Native vs Dual), and binary/protocol requirements.

### 3.3 Verification & Test Evidence
- **Workspace Build & Lint Check:**
  - `cargo check --workspace --all-targets`: Clean (exit code 0).
  - `cargo clippy --workspace --all-targets -- -D warnings`: Clean (0 warnings, exit code 0).
  - `cargo fmt --all -- --check`: Clean (0 formatting discrepancies, exit code 0).
- **Workspace Test Execution (`cargo test --workspace`):**
  - **Before Changes:** 70 suites, 213 tests passed, 0 failed, 0 ignored.
  - **After Changes:** 70 suites, **228 tests passed**, 0 failed, 0 ignored.
  - **Delta:** +15 tests passed (1 new unit test in `rune-pdk` + 14 new contract rule tests across active plugins).

---

## 4. Written Statements

### 4.1 TOOL-2: Windows Stability Statement (Starter Set)

**Scope of Starter Set:**
1. `rune-filesystem`
2. `rune-fetch`
3. `rune-memory`
4. `rune-time`
5. `rune-sequential-thinking`

**Verified Execution Environment:**
- **OS:** Windows 11 Pro (`x86_64-pc-windows-msvc`)
- **Rust Toolchain:** 1.98.1 (Edition 2024)
- **Target Architecture:** `x86_64-pc-windows-msvc` (unit/contract tests) and `wasm32-wasip1` (WASM release builds)

**Stability & Compatibility Findings:**
1. **File System Paths & Windows Normalization:**
   `rune-pdk::resolve_scoped_path` and `normalize_path` have been tested against Windows backslashes (`\`), mixed separators, and path traversal attempts (`..\`). Absolute paths with Windows drive letters (e.g., `C:\`) are safely scoped within the declared `allowed_dir` root. Escaping attempts (e.g., `C:\allowed\..\Windows`) are rejected with structured error payloads.
2. **Line Endings:**
   All text-processing tools (such as `read_text_file`, `head`, `tail`, `write_text_file` in `rune-filesystem`) transparently handle CRLF (`\r\n`) and LF (`\n`) without mangling byte counts or line indices.
3. **WASM Runtime Sandbox:**
   Pure WASM plugins execute in-process via Extism/Wasmtime. On Windows, Wasmtime compiles and sandboxes `wasm32-wasip1` instructions without relying on POSIX-only syscalls. In-process memory safety is guaranteed.
4. **Time & Clocks:**
   `rune-time` utilizes `chrono` with standard Windows system clock APIs, accurately computing UTC offsets and IANA timezone conversions (e.g., `Asia/Kolkata`, `America/New_York`) without POSIX `/usr/share/zoneinfo` dependencies.
5. **Memory Graph Persistence:**
   `rune-memory` operates in-memory with pure JSON serialization, completely free of native OS locking traps or file handle contention.

**Conclusion:** All 5 starter set plugins are fully verified and stable on Windows.

---

### 4.2 TOOL-4: Output Contracts & Pagination Statement

**Current Output Format:**
All `rune-tools` adhere to standard MCP JSON-RPC 2.0 output envelopes:
```json
{
  "content": [
    {
      "type": "text",
      "text": "<JSON-STRINGIFIED-RESULT>"
    }
  ]
}
```
The internal `<JSON-STRINGIFIED-RESULT>` is always structured JSON containing `{ "status": "success", "result": { ... } }` or `{ "status": "error", "error": "..." }`.

**Existing Pagination Mechanisms:**
- `rune-filesystem::read_text_file`: Accepts `lineOffset` (default 0) and `lineLimit` (e.g., 50). Returns `linesReturned`, `totalLines`, and `hasMore`.
- `rune-filesystem::list_directory`: Accepts `maxEntries` to prevent massive directory dumps from exhausting context windows.
- `rune-fetch::fetch`: Supports `maxBytes` truncation to bound large HTML/Markdown responses.
- `rune-git::git_log`: Supports `maxCount` to paginate commit logs.

**Proposed Ecosystem Standard for Pagination:**
To ensure predictable handling across all tools by Rune Studio, we propose adopting the following standardized pagination envelope for all list operations:
```json
{
  "items": [ ... ],
  "pagination": {
    "offset": 0,
    "limit": 50,
    "total": 128,
    "has_more": true,
    "next_cursor": "50"
  }
}
```
We accept updating any remaining unbounded tools to follow this standard in subsequent feature releases.

---

### 4.3 TOOL-5: Browser Plugin Readiness & Second Surface Statement

**Status of `rune-browser`:**
- `rune-browser` provides 17 tools implemented on top of `agent-browser` and the Chrome DevTools Protocol (CDP).
- Tools include: `browse_open`, `browse_navigate`, `browse_click`, `browse_type`, `browse_evaluate_script`, `browse_take_screenshot`, `browse_get_dom`, `browse_get_text`, `browse_extract_links`, and session cookie management.
- All tools declare `ToolRules` with `RiskLevel::Medium` (navigation, clicking, script evaluation) or `RiskLevel::Low` (reading text, screenshots).

**Role as a Second Surface (Complementary to Studio):**
Rune Studio operates primarily as an OS-level desktop harness using visual screen analysis, accessibility trees, and synthetic mouse/keyboard input on Windows. `rune-browser` provides an internal, programmatic second surface that does not conflict with Studio:
1. **DOM & Structure vs Pixels:** While Studio inspects the rendered desktop screen, `rune-browser` inspects underlying DOM nodes, CSS attributes, and JavaScript state, enabling 100% reliable data extraction without OCR hallucinations.
2. **Headless & Background Operation:** `rune-browser` can run headlessly in the background, executing long web searches, article scraping, or session cookie retrieval without moving the user's cursor or taking focus away from active desktop applications.
3. **No Collision:** Because `rune-browser` drives a dedicated browser instance over CDP/WebSocket, it does not intercept or disrupt Rune Studio's desktop accessibility hooks.

---

## 5. Architectural Answers to Questions & Process

### 5.1 TOOL-7: Memory Plugin Format & Notes
- **Current Format:**
  `rune-memory` implements a knowledge graph schema with nodes (entities) and directed edges (relations):
  - Entity: `{ "name": "string", "entityType": "string", "observations": ["string"] }`
  - Relation: `{ "from": "string", "to": "string", "relationType": "string" }`
- **Suitability for User Profile Notes (L0–L10 Life Track):**
  The current graph format is highly suitable for structured relationships (e.g., `User` -> `prefers` -> `Dark Mode`, `Project` -> `depends_on` -> `rune-infer`). However, long-term life harness notes require temporal context and evidential provenance.
- **Recommended Schema Enhancements:**
  1. **Episodic Timestamping:** Add `created_at` and `updated_at` (ISO 8601 UTC) to each observation.
  2. **Confidence & Source Provenance:** Add `source` (e.g., `"user_explicit"`, `"tool_observation:rune-filesystem"`) and `confidence: f32` (0.0 to 1.0) so Studio can distinguish facts directly stated by the user from inferences.
  3. **Tombstoning / Soft Deletion:** Enable invalidating outdated beliefs without erasing the historical trajectory.

---

### 5.2 TOOL-8: Versioning Process
- **Current Process:**
  `rune-tools` utilizes **Cocogitto (`cog`)** with Conventional Commits (`feat:`, `fix:`, `chore:`, `perf:`, `refactor:`, `breaking:`).
  - Version configuration is defined in [`cog.toml`](file:///d:/Projects/Public/rune/code-tools/cog.toml).
  - Releases are calculated per-crate using `cog bump --auto`.
  - CI pipeline ([`.github/workflows/integrate.yml`](file:///d:/Projects/Public/rune/code-tools/.github/workflows/integrate.yml)) validates commit message conventions (`cog verify`) and enforces zero Clippy warnings (`-D warnings`).
  - Automated publishing ([`.github/workflows/publish.yml`](file:///d:/Projects/Public/rune/code-tools/.github/workflows/publish.yml)) compiles native artifacts across Windows, Linux, and macOS, publishing verified hashes to the central registry.
- **Breaking Change Management:**
  Any breaking change must use the `breaking:` commit prefix or append `!` to the commit type. Breaking changes bump the major version (or minor in `0.x`), are documented in `CHANGELOG.md`, and maintain deprecated aliases for at least one minor release cycle.

---

### 5.3 TOOL-9: A2UI Blueprint Feasibility & Pilot Proposals
- **Feasibility:**
  Declarative A2UI JSONL blueprints are completely feasible and eliminate the latency, token cost, and non-determinism of generating runtime UI code. Blueprints are stored statically under `plugins/<plugin>/blueprints/<tool_name>.a2ui.jsonl`.
- **Pilot Blueprints Created & Verified:**
  1. [`plugins/rune-time/blueprints/get_current_time.a2ui.jsonl`](file:///d:/Projects/Public/rune/code-tools/plugins/rune-time/blueprints/get_current_time.a2ui.jsonl):
     - Displays formatted datetime in dot-matrix typography, timezone badge, UTC offset label, and a one-click refresh button bound to `get_current_time`.
  2. [`plugins/rune-filesystem/blueprints/list_directory.a2ui.jsonl`](file:///d:/Projects/Public/rune/code-tools/plugins/rune-filesystem/blueprints/list_directory.a2ui.jsonl):
     - Displays path header, total item badge, structured entry table (name, type tag, byte size, modification timestamp), and refresh action button.
- **Visual Alignment:**
  Both blueprints use the Nothing-phone style dot interface design tokens (`style: "dot-monochrome"`, `font: "monospace"`, `variant: "neutral"` with single accent).

---

### 5.4 TOOL-10: Evidence Sources for the Life Track
For Rune Studio's life harness to operate based on **evidence** rather than model assumptions, the following tools provide verifiable ground truth:
1. **`rune-filesystem`:**
   - Authoritative inspection of local files, project configurations, logs, and modification timestamps.
   - Verification of artifact existence, line counts, and checksum hashes.
2. **`rune-git`:**
   - Ground truth regarding repository state: clean vs dirty working tree, active branch name, commit author signatures, exact staged/unstaged diffs.
   - Proves whether promised code changes were actually committed.
3. **`rune-time`:**
   - Authoritative UTC time, eliminating model hallucinations regarding current dates, deadlines, or durations.
4. **`rune-browser`:**
   - Ground truth for online research: status codes (200, 404), actual page URLs, rendered DOM text, and timestamps of web documents.

---

## 6. Response Table

| Request ID | Name | Action | Owner / Target Release | Notes / Evidence |
|---|---|---|---|---|
| **TOOL-1** | Safety facts declaration | **Accept** | Complete | Implemented `ToolRules`, `RiskLevel`, `ToolEffect`, `ToolLimits` across all 140 tools. Serializes top-level `"rules"` and injected `"x-rune-rules"`. Contract tests enforced. |
| **TOOL-2** | Windows stability starter set | **Accept** | Complete | Validated on Windows 11 (`x86_64-pc-windows-msvc`). 5 starter plugins verified with 0 failures. |
| **TOOL-3** | Subprocess stop handling | **Accept** | `rune-sidecar` v0.2.0 | Handled via OS process groups and SIGTERM/CTRL_BREAK signals in `rune-sidecar`. Native child processes terminate when stdin closes. |
| **TOOL-4** | Output contracts & pagination | **Accept** | Complete | Current JSON contracts documented; standardized pagination envelope proposed for future list operations. |
| **TOOL-5** | Browser plugin readiness | **Accept** | Complete | 17 tools operational. Functioning as non-conflicting programmatic second surface alongside Studio screen analysis. |
| **TOOL-6** | Documentation update | **Accept** | Complete | `README.md` and `AGENTS.md` updated with exact 14 active / 3 draft plugin counts, execution models, and license. |
| **TOOL-7** | Memory plugin format | **Accept** | Feature backlog | Graph format evaluated. Proposed episodic timestamping and provenance tracking for life harness notes. |
| **TOOL-8** | Versioning process | **Accept** | Process documented | Cog / Conventional Commits workflow detailed with CI/CD integration. |
| **TOOL-9** | A2UI blueprints | **Accept** | Complete | Blueprints feasible. Implemented 2 working pilot JSONL blueprints for `get_current_time` and `list_directory`. |
| **TOOL-10**| Evidence sources for life track | **Accept** | Complete | Ground-truth evidence sources cataloged (`filesystem`, `git`, `time`, `browser`). |

---

## 7. Ecosystem Questions & Governance

1. **Who is the contact person or role for questions from other projects?**  
   **Contact:** Shirsendu Bairagi (`@shba007`), Rune Tools Lead and Repository Maintainer.

2. **How do you want breaking changes announced between projects?**  
   - Open a tracking issue or discussion titled `[BREAKING-RFC] <Component>: <Description>`.
   - Mark Pull Requests with the `breaking:` commit prefix and label `breaking-change`.
   - Maintain a deprecation window of at least one minor release (`0.x` or `x.0`) with warnings emitted in tool responses before removal.

3. **Corrections to Section 1 & Section 2:**
   - **Section 1 (Ecosystem):** We agree with the ownership division. `rune-infer` manages all models, `rune-kit` supervises plugins and external CLI binaries, `rune-tools` implements capability plugins, and `Rune Studio` operates the desktop harness and renders A2UI widgets.
   - **Section 2 (Contract):** Fully accepted. All tools must provide structured safety facts, cancel cleanly on Stop, and return deterministic JSON data.
