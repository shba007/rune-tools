# Rune Tools - MCP Servers as Plugins

A collection of MCP servers packaged as plugins, runnable via the `rune` binary (`rune run <plugin-name>`).


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
| `rune-memory` | Persistent knowledge-graph memory: typed entities and relations in a JSON file, batch creation, filtered queries, deletion, full inspection. | `create_entities`, `query_memory`, … | [README](plugins/rune-memory/README.md) |
| `rune-print` | Native OS printing and eSCL AirScan scanning: printer discovery, spooler dispatch for TXT/PDF/images, flatbed/ADF scans, capability queries. | `printer_list_printers`, `printer_print_document`, … | [README](plugins/rune-print/README.md) |
| `rune-sequentialthinking` | Dynamic step-by-step reasoning workspace with progress tracking, hypothesis revision, thought branching, automatic total adjustment. | `sequentialthinking` | [README](plugins/rune-sequentialthinking/README.md) |
| `rune-time` | Deterministic timezone queries, ISO-8601 formatting, and DST-aware cross-timezone conversions. | `get_current_time`, `convert_time` | [README](plugins/rune-time/README.md) |
| `rune-video` | Video streaming, playlist extraction, live broadcast recording, and media trimming powered by yt-dlp, streamlink, ffmpeg. | `inspect_video_metadata`, `download_video_stream`, … | [README](plugins/rune-video/README.md) |

## Development

```bash
# Test Single Plugin
cargo xtask test rune-<name>

# Test Whole Workspace
cargo xtask test-all

# Build Single plugin in either wasm or native(if available)
cargo xtask build rune-<name> --wasm-only
cargo xtask build rune-<name> --native-only

# Build Single plugin, both targets
cargo xtask build rune-<name>

# Build Whole workspace, both targets
cargo xtask build-all
```