# Rune Tools follow-up: TF-1–TF-6

2026-10-21. Supersedes conflicting claims in `rune-tools-report.md`.
Tools checkout baseline: `ddcb5aac7b881e5fbf0cab3a4ef2067f16e9571a`.
Kit source inspected: local `develop`, ref `53d49d003562c69a185c086acdb9d833dbd2f752`.
Runtime checks used `../code-kit/target/debug/rune.exe`; whether that executable
exactly matches the current kit source is **NOT CERTAIN**.

## TF-1 — Implemented; kit integration still blocked

`rune-pdk::annotations_from_rules` derives annotations during serialization.
It emits both `readOnly`/`readOnlyHint`, `destructive`/`destructiveHint`, and
`openWorld`/`openWorldHint`, as requested. Delete **and exec** count as
destructive; network counts as open-world. Idempotence is omitted because
the rules do not establish it. Missing rules produce no annotations.
Top-level `rules` and `inputSchema.x-rune-rules` remain available.

Added an explicit `Exec` effect; subprocess constructors now declare it.
Contract macro checks serialized annotations against rules and rejects
read-only declarations with write/delete/send/network/exec effects or
external binary/host limits. Serialization tests cover aliases, omitted
idempotence, missing rules, and rejection of read-only deletion.

**Confirmed original gap:** running `rune run --all` against the five existing
installed packages returned 38 tools with zero annotations;
`rune run --all --read-only` returned zero tools. Those installed artifacts
were not replaced and do not contain this change.

**New confirmed blocker:** after building the changed git native plugin with
`cargo build --offline -p rune-git --features native --bin rune-git-native`,
both `rune run <absolute-path-to-rune-git-native.exe>` and the same command
with `--read-only`, receiving a JSON-RPC `tools/list`, returned zero tools:

```text
JSON serialization error: duplicate field `destructiveHint`
```

Kit's `ToolAnnotations` derives deserialization with legacy names as Serde
aliases of canonical fields. An object containing both is rejected. The
dual spelling explicitly requested here is retained; kit must accept both
keys together (or the projects must agree to canonical-only output).
No kit files were changed. Successful listing/filtering of rebuilt plugins
through kit is **NOT CERTAIN** and is not claimed. This remains an integration
blocker even though all declaration checks pass.

## TF-2 — Conservative strict read-only set

The executable declaration audit (`cargo run --offline -p rune-pdk --example
audit_rules`) checked all 140 tools across all 17 plugins, including drafts.
All 140 emitted consistent annotations. Exactly these three declare read-only:

| Plugin / tool | Effects | Risk | Argument review |
|---|---|---|---|
| rune-time / get_current_time | read | low | Parses timezone, reads clock, formats output |
| rune-time / convert_time | read | low | Parses datetime/timezones and calculates conversion |
| rune-git / validate_commit_message | read | low | Checks supplied text, length and conventional format |

Source review confirms these handlers have no file writes, deletion,
subprocess or network calls, including their optional arguments/environment
fallbacks. Time and git operations tests passed. This is the intended
declaration set; the observed kit list remains empty because of TF-1.

Filesystem reads are excluded: root checks in
`plugins/rune-filesystem/src/operations.rs::resolve_path_with_root` are lexical,
and confinement through symlinks/Windows reparse points is **NOT CERTAIN**.
The legacy `read_only_fs` helper now conservatively returns `read_only=false`;
memory and image file reads using it are excluded too. This does not repair
the filesystem boundary; Studio's requested filesystem starter set is deferred.
Git inspection launches `git`, so it is excluded under the requested strict
no-process definition, regardless of whether a particular subcommand writes.
Fetch, mailbox, browser, Figma and other network reads are excluded.

Corrected `compare_images` to declare write (it saves a diff image even with
default arguments), sequential thinking to declare write (session mutation),
and metadata inspection via `yt-dlp`/`gallery-dl` to declare network plus exec.
Draft printer discovery is conservatively network-classified and excluded.
No uncertain filesystem or subprocess tool remains in the declared set.

## TF-3 — Drafts only

Both existing blueprint folders now have README notices: **draft, not consumed
by anyone yet**. Removed invented style/font/variant/color/accent/gap tokens.
No new blueprints were added. Components, bindings and carrier remain unverified;
adapt after Studio K2 and agreement on kit's proposed plugin resource carrier.

## TF-4 — Cancellation correction

Built today: `rune-sidecar::run_stdio` synchronously handles each request and
observes stdin EOF only between calls. Video runs `Command::output()` and waits
for the subprocess. There is no sidecar cancellation dispatcher, process-group
cleanup, Windows Job Object, or CTRL_BREAK implementation in this code.
Kit's current source polls cancellation/timeouts and kills/waits for the direct
native plugin process; it does not show descendant-tree cleanup in that path.

Therefore child processes such as ffmpeg **may survive** a native plugin kill,
especially on Windows. Actual ffmpeg orphan behavior was not exercised:
**NOT CERTAIN**. The prior report's claims that process groups were implemented
and stdin closure stops active descendants were incorrect. Sidecar v0.2.0 was
a proposed target, not a shipped implementation or committed delivery date.
TOOL-3 in the original handoff was the OCR question; subprocess handling was
an additional proposal.

## TF-5 — Memory does persist

The prior in-memory statement was incorrect. Memory saves JSON and reloads it
on later calls, defaulting to `memory.json`; `memoryFile`/configuration select
the file and `allowedDir`/configuration can select its directory. Persistence
requires the same accessible backing file after restart. A new test creates
an entity in one process and reads it in a second process; it passed.
WASM host mapping across host restarts was not exercised: **NOT CERTAIN**.
No memory schema or provenance changes were made.

## TF-6 — Prepared versions; not published

These source versions contain TF-1 after rebuilding against rune-pdk **0.1.1**:

| Plugin | Version |
|---|---|
| mhb-mconnect | 0.1.4 |
| rune-audio | 0.2.4 |
| rune-browser | 0.2.5 |
| rune-email | 0.1.6 |
| rune-fetch | 0.3.3 |
| rune-figma | 0.2.2 |
| rune-filesystem | 0.3.6 |
| rune-git | 0.2.5 |
| rune-image | 0.2.3 |
| rune-memory | 0.2.3 |
| rune-sequential-thinking | 0.2.1 |
| rune-slides | 0.2.1 |
| rune-time | 0.2.3 |
| rune-video | 0.2.1 |
| rune-print (inactive draft) | 0.2.1 |
| rune-scan (inactive draft) | 0.1.1 |
| rune-ssh (inactive draft) | 0.1.1 |

Cargo manifests, active plugin manifests and Cargo.lock were synchronized.
Shared PDK changes affect every rebuilt plugin, so all plugin source versions
were advanced. Drafts remain outside the publishing workspace; their manifest
files have no version field. PDK is an internal shared crate, not a registry
plugin. No tags, pushes or registry publication were performed. Publication
date is **NOT CERTAIN**; the normal Conventional Commit/Cocogitto release flow
and `main` publishing workflow still apply. Studio must not treat these as
available registry minimums until publication and the TF-1 kit blocker resolve.

## Verification performed

- `cargo fmt --all -- --check`: passed.
- `cargo clippy --workspace --all-targets --offline -- -D warnings`: passed;
  dependency future-compatibility warning for `imap-proto` remains.
- Declaration audit: 17 plugins, 140 annotated tools, exactly three read-only.
- Focused PDK/time/git/memory tests: **38 passed, zero failed**, with TEMP/TMP
  pointing inside the workspace; includes separate-process memory persistence.
- Full workspace test attempt failed on the existing live YouTube extraction
  test because DNS was unavailable. Live operations are **NOT CERTAIN**.
- Workspace run excluding `test_live` completed with 220 passed and two failed:
  an old PDK expectation of read-only filesystem reads (corrected and rerun
  successfully above), and existing `test_compare_images_output_path_handling`
  using hardcoded `D:/test/output.png`. The latter assertion remains failing;
  its full underlying error is **NOT CERTAIN** because the test omits it.
  An earlier default-temp run also hit a filesystem move access denial;
  the workspace-temp run passed that test. No full green workspace run claimed.

Local detailed logs/catalog are under `temp/tf-*` (ignored working artifacts).
Pagination, memory schemas and additional plugins were left out of scope.
