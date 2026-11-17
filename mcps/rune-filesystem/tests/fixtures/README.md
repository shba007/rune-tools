# Filesystem output contract and captured baseline

`kit-before.json` contains actual client-visible MCP HTTP responses captured
before editing rune-filesystem, from code-tools commit
`8814e65b3d85e6f69de0e9a16fbe7b2818631e49`. The repository was initially clean.
The capture exercised all 13 tools against disposable files, plus traversal and
failed-edit errors. No source repository was used as a mutation fixture.

The unmodified WASM SHA-256 was
`95E0E242C6C1B1760D30DE6083A475977BE3C3396FD1282D7248D38C849D2B7F`.
The existing Kit executable was copied outside all repositories; its SHA-256 was
`B9AB7D36AC1447921BEEC22CE52D85C8E86B42DD75C9B2C305C0B0F46153352F`.

The capture script, executable, before/after WASM files, isolated Kit data and
raw HTTP captures remain outside the repositories at:

```text
C:\Users\shba007\.codex\visualizations\2026\11\16\01a1e663-6c7e-7320-b80a-dd3b23d1cb54\filesystem-output
```

`capture.ps1` runs Kit headlessly on loopback port 47621 with an explicit
forward-slash `allowed_dir` pointing to its disposable `fixture` directory.
It calls `tools/list`, discovers the namespaced tools, then calls each tool over
MCP HTTP 2026-07-28 with matching method/name headers and `_meta`. SSE `data:`
messages are parsed without converting datetime strings. Metadata timestamps
on the fixture file are fixed so before/after metadata text can be compared.

Both errors and all successful tool texts were byte-identical in the live
before/after comparison. The Rust snapshot test uses these exact captured text
strings, not expectations inferred from an envelope. Host tests also exercise
the actual mutation/read boundaries and unchanged media payloads.

## Behavior

- Twelve tools publish output schemas, with required fields and property
  descriptions. Existing inputs, rules, names and operation effects remain.
- `read_media_file` deliberately retains its exact legacy success/error payload,
  without `outputSchema` or `structuredContent`: it permits 20 MiB media, while
  Kit caps structured results at 8 MiB. This exception is commented in code.
- Shared `rune_pdk::tool_error` preserves the legacy JSON error text and returns
  `isError: true`. Filesystem's non-media errors and rune-time use this helper.
- All five listing/search-style tools expose complete `entries`, required
  `truncated`, and required `total`. Totals count all enumerated matching items;
  trees count every node including the root, with entries in depth-first order.
- Complete results below the bounds retain their exact legacy text. Truncated
  results include an explicit omitted-item count in that text.
- Already-capped `list_directory` pages also gain an omission notice, even when
  the full wire result is below 1 MiB. Their existing 200-entry/64 KiB caps still
  apply. Text equality is guaranteed for complete, untruncated results.
- Multi-file reads and edit previews also receive bounded, UTF-8-safe previews
  with an omitted-byte notice. Their `total` is the original UTF-8 byte count.
  Truncating a preview does not shorten, undo or repeat a file operation.

## Byte budget

The bound is **1,048,576 UTF-8 bytes for the entire serialized plugin result**,
including content text, structured content, repeated data, JSON escaping,
metadata, and omission notices. Kit's ceiling is 8,388,608 bytes, leaving
7,340,032 bytes of headroom. No assumed average filename length or row count is
used: each candidate is measured using the same JSON serialization as the WASM
return value. A binary search selects the longest complete item prefix that
fits. Tree pruning happens at node boundaries; text previews stop at UTF-8
boundaries. The existing directory page's 64 KiB content cap also includes its
omission notice.

Tests cover a result within two bytes of the bound, the next oversized item,
worst-case escaping and multibyte text, complete tree nodes, real directory
counts beyond 200 entries, and full file writes despite truncated previews.

An actual 3,000-file Kit HTTP check accepted these bounded results (HTTP result
sizes include Kit's response metadata):

| Tool | Total | Shown | Truncated | HTTP result bytes |
| --- | ---: | ---: | --- | ---: |
| list_directory | 3000 | 200 | true | 65,770 |
| list_directory_with_sizes | 3000 | 2780 | true | 1,048,447 |
| directory_tree | 3001 | 1792 | true | 1,048,359 |
| search_files | 3000 | 1590 | true | 1,048,199 |

Older-Kit compatibility remains a **code-only** check of Kit revision
`4a6d25c`: it ignores unknown output-schema fields and preserves supplied content
arrays and `isError`. No older Kit was run. No dependency on Kit was added.

## Build, isolated installation and undo

Run from PowerShell. The isolated Kit data override avoids replacing an existing
installation. Select a fresh `filesystem-kit-data` directory for a new trial.

```powershell
Set-Location D:\Projects\Public\rune\code-tools
$env:CARGO_HUSKY_DONT_INSTALL_HOOKS = 'true'
cargo build --locked -p rune-filesystem --release --target wasm32-wasip1

New-Item -ItemType Directory -Force target\filesystem-bundle | Out-Null
New-Item -ItemType Directory -Force target\filesystem-demo | Out-Null
Copy-Item target\wasm32-wasip1\release\rune_filesystem.wasm target\filesystem-bundle\
Copy-Item mcps\rune-filesystem\plugin.toml target\filesystem-bundle\

$fsKit = 'D:\Projects\Public\rune\code-kit\target\debug\rune-kit.exe'
$fsPreviousDataDir = $env:RUNE_KIT_DATA_DIR
$env:RUNE_KIT_DATA_DIR = "$PWD\target\filesystem-kit-data"
& $fsKit install "$PWD\target\filesystem-bundle\rune_filesystem.wasm" --version 0.5.0
$fsDemo = (Resolve-Path target\filesystem-demo).Path.Replace('\', '/')
& $fsKit serve rune-filesystem --headless --port 47621 -p "allowed_dir=$fsDemo"
```

After stopping the server with Ctrl+C:

```powershell
& $fsKit uninstall rune-filesystem
$env:RUNE_KIT_DATA_DIR = $fsPreviousDataDir
```

The local install/uninstall sequence was also exercised in the external
capture directory using an isolated `install-check` data store. Kit reported
successful installation and removal of `rune-filesystem` version `0.5.0`, then
an empty installed-plugin list. The ordinary user installation was untouched.

## Validation commands

```powershell
$env:CARGO_HUSKY_DONT_INSTALL_HOOKS = 'true'
cargo test --locked -p rune-filesystem -p rune-pdk -p rune-time
cargo test --locked --workspace --no-fail-fast
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

Logs are under the ignored root `target/` directory:
`filesystem-focused.log`, `filesystem-workspace-tests.log`,
`filesystem-clippy.log`, `filesystem-focused-clippy.log`,
`filesystem-fmt.log` and `filesystem-build.log`. The final workspace run passed
299 tests; the final focused run passed 78, including the test that keeps the
directory omission notice inside the existing 64 KiB cap.
Clippy, formatting and WASM build passed. Existing Windows incremental-cache
hard-link warnings, Cargo filename collisions, and the `imap-proto`
future-compatibility notice remain. Print, scan and ssh remain draft plugins
outside the configured workspace; no manifests were changed to enable them.
