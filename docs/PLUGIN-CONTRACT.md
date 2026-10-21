# Plugin contracts and integration limits

## Tool rules and annotations

`rune-pdk::annotations_from_rules` derives annotations during serialization.
Current compatibility output includes `readOnly`/`readOnlyHint`,
`destructive`/`destructiveHint` and `openWorld`/`openWorldHint`. Delete and exec
are destructive; network is open-world. Idempotence is omitted because the rules
do not establish it. Missing rules produce no annotations. Both top-level
`rules` and `inputSchema.x-rune-rules` remain available.

The contract macro rejects read-only declarations with write/delete/send/network/
exec effects or external binary/host limits. The conservative declared read-only
set is `rune-time/get_current_time`, `rune-time/convert_time`, and
`rune-git/validate_commit_message`. Filesystem operations are excluded because
`allowedDir` can be overridden and path confinement was not established.
Subprocess and network reads are excluded under the strict no-process policy.

Use the executable declaration audit to verify the current catalog:

```text
cargo run -p rune-pdk --example audit_rules
```

Older Kit annotation deserializers rejected simultaneous legacy/canonical keys
as duplicate fields. Use current Kit contract tests to verify compatibility;
source changes do not update already installed plugin artifacts. Successful
cross-repository integration must be checked against the exact binaries in use.

## Runtime and persistence limits

`rune-sidecar::run_stdio` handles requests synchronously and observes stdin EOF
between calls. There is no descendant process-tree cancellation dispatcher.
Children such as ffmpeg may survive termination of the direct plugin process,
especially on Windows. Real orphan behavior requires a live acceptance test.

Memory persists JSON to `memory.json` by default; `memoryFile`/configuration
select the backing file and `allowedDir`/configuration select the directory.
Persistence across restart requires the same accessible file. Separate-process
native persistence is tested; WASM mapping across host restarts needs acceptance.

Blueprints under `plugins/*/blueprints/` remain drafts until Studio publishes its
catalog and the resource carrier is agreed. They must not be treated as supported
UI contracts. Source versions are not evidence that registry artifacts were
published; release and registry files remain owned by the normal release process.

## Development checks

```text
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --check
cargo build --workspace --locked
```

Some operation tests call live external services or installed binaries. A full
run depends on that environment; do not remove the tests or claim a passing
run from an excluded subset. `imap-proto` currently emits a dependency
future-compatibility warning, independent of project Clippy diagnostics.
