# Plugin contracts and integration limits

## Public protocol and private API inventory

Current public target: [MCP 2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28).
Minimum gateway version for this contract: Kit 0.5.0. The Kit repository owns
the public server implementation; this repository supplies its plugins and
public-client acceptance scripts. Local 0.5.0 verification is distinct from
the older released Kit compatibility checks below.

| Boundary | Implementation | Contract |
| --- | --- | --- |
| External client → `rune-kit run` stdio | Kit gateway, outside this repository | Public MCP 2026-07-28 |
| Kit → `rune-<plugin>-native` / legacy standalone plugin CLI | `rune-sidecar::run_stdio`, handlers in plugin main/native_sidecar files | Private newline JSON request/response API, including old method aliases |
| Kit → WASM `mcp_info`, `mcp_list_tools`, `mcp_call_tool` and optional resource/prompt exports | Plugin lib.rs files and Extism | Private FFI ABI, serialized definitions/calls and success/error envelopes |
| Native plugin → Figma WebSocket / external service / CLI | Plugin operations | Service-specific APIs, not public MCP endpoints |
| In-process Rust operations and PDK definitions | Plugin operations.rs and rune-pdk | Internal library API, not a transport |

No Tools binary currently implements a standalone public MCP transport. The
private API's `tools/list` aliases do not make it a public MCP server: it lacks
public discovery/negotiation and returns private shapes. Those formats are
preserved without new protocol fields or packaged-runtime changes. The mcp.json
templates launch Kit, rather than exposing a sidecar directly. Do not advertise
unimplemented optional capabilities or change a private API just to resemble MCP.

The public stdio binding uses single UTF-8 newline-delimited JSON-RPC messages,
no batches and no initialize handshake. Every request carries its own metadata:

```json
{"jsonrpc":"2.0","id":1,"method":"server/discover","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{},"io.modelcontextprotocol/clientInfo":{"name":"rune-tools-acceptance","version":"1"}}}}
```

Send the version and clientCapabilities on **every** tools/list, tools/call,
resource or prompt request; capabilities are not remembered from discovery.
Use exact discovered names and schemas. Successful results include
`resultType: "complete"` and `_meta.io.modelcontextprotocol/serverInfo`.
Cacheable results include `ttlMs: 0` and `cacheScope: "private"`. An empty
capabilities object opts into no optional client features. Missing metadata is
`-32602`; unsupported versions are `-32022`. Initialize is not a current method.
Stdio cancellation is a notification and has no notification reply; do not
infer a public server capability from private plugin operations. Tools implements
no public HTTP binding. Kit's full schema/transport validation remains Kit-owned.

Authoritative references: [normative schema](https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/schema/2026-07-28/schema.ts),
[transport binding](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/stdio),
and the Kit repository's docs/MCP-CONTRACT.md. Acceptance clients enforce current
metadata, discovery and complete-result envelopes, and probe missing metadata,
unsupported versions and obsolete initialize. Only `--legacy-protocol` enables
the explicit older-released-Kit compatibility fixture.

## Tool rules and annotations

`rune-pdk::annotations_from_rules` derives annotations during serialization.
Current compatibility output includes `readOnly`/`readOnlyHint`,
`destructive`/`destructiveHint` and `openWorld`/`openWorldHint`. Delete and exec
are destructive; network is open-world. Idempotence is omitted because the rules
do not establish it. Missing rules produce no annotations. Both top-level
`rules` and `inputSchema.x-rune-rules` remain available.

The contract macro rejects read-only declarations with write/delete/send/network/
exec effects or external binary/host limits. The conservative declared read-only
set is `rune-time/get_current_time`, `rune-time/convert_time`,
`rune-git/validate_commit_message`, and (filesystem 0.4.1+) the two bounded reads
`rune-filesystem/read_text_file` and `rune-filesystem/list_directory`.
The shared `read_only_fs` helper remains conservative; all other filesystem,
media and memory reads retain their existing classification.
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

### Filesystem 0.4.1 integration contract

The published filesystem plugin is WASM, with no native subprocess. Configure
one trusted root at gateway startup, never from model arguments:

```powershell
rune-kit run rune-filesystem --read-only -p allowed_dir=D:/Disposable/Pilot
```

The Extism/WASI manifest maps the
absolute host root to the same guest path and passes `allowed_dir` unchanged.
Use forward slashes for the Windows root and prefer relative tool paths.
Kit 0.5.0 retains the mapping used by released 0.4.0, validates roots and
provides no implicit working-directory preopen. Tools 0.4.1 refuses missing,
relative, blank or inaccessible roots even on 0.3.1. Changing roots requires
a new gateway. Neither `allowedDir` nor `allowed_dir` is accepted in a call.

Paths are checked for traversal, outside absolute names, sibling prefixes,
Windows device names and alternate streams. Those checks provide diagnostics;
the security boundary is the actual WASI directory capability at open time,
including link resolution. No canonicalize-then-open confinement claim is made.
Native domain tests use cap-std directory capabilities; they are not evidence
of WASI behavior. The host must not grant additional preopens to this plugin.
The selected root and its ancestors are trusted host configuration; owners
must select a folder they intend to expose. Files may change between calls.

Accepted schemas (all properties have descriptions in discovery):

| Operation | Required | Optional properties |
| --- | --- | --- |
| `read_text_file` | `path: string` | `head`, `tail`, `lineLimit`: integers 0..2000; `lineOffset`: nonnegative integer |
| `list_directory` | `path: string` | `maxEntries`: integer 0..200 |

Use `path: "."` for the root listing. Absolute paths are accepted only inside
the exact configured root prefix. Backslashes in call paths are normalized.
`head` and `tail` cannot be combined. `head` overrides `lineLimit`; `tail`
selects from the end and ignores `lineOffset`. Compatibility aliases
`line_offset`, `line_limit`, `max_entries` are also accepted by implementation;
clients should use the discovered camelCase schema. If both aliases occur,
camelCase wins. An offset beyond the file returns an empty page.

Text must be UTF-8 and the opened file must be regular, no larger than 8 MiB.
The read uses at most 8 MiB + one sentinel byte even if the file grows. Each
page contains at most 2000 lines and 64 KiB of UTF-8 content. A single line
larger than the output budget is refused with an actionable error. Response:
`{content,lineOffset,linesReturned,totalLines,hasMore,nextLineOffset,maxInputBytes,maxOutputBytes}`.
Continue at `nextLineOffset` while `hasMore` is true. A zero line limit makes
no progress; clients must request a positive limit to advance. Empty files
have zero lines. Newlines are normalized by Rust `str::lines`.

Listings return at most 200 entries and 64 KiB of content, with
`{content,entriesReturned,hasMore,truncated,maxEntries,maxOutputBytes}`.
The retained subset is sorted; directory enumeration is not a stable global
order and there is no directory continuation cursor. A truncated listing is
incomplete. Entry types are inspected without following links, while the
requested directory itself is resolved through WASI. A host runtime may
report a Windows junction as FILE rather than LINK. Content budgets exclude
JSON escaping/envelope overhead. Numeric limits are enforced by the plugin,
also advertised in schema maxima and `x-rune-read-limits`; Kit does not enforce
that extension. Rules have `read_only=true`, `effects=["read"]`, `risk="low"`,
and `limits.root_param="allowed_dir"`; both readOnly spellings are true, both
destructive/openWorld spellings false, and idempotence is omitted.

Example call arguments: `{"path":"note.txt","lineLimit":1}` returns
`{"content":"inside marker","lineOffset":0,"linesReturned":1,"hasMore":true,"nextLineOffset":1,...}`.
`{"path":"../root-other/secret.txt"}` is refused. On released legacy Kit and
the tested local 0.5.0 gateway, the MCP result can have `isError:false` while
its text contains the plugin's
`{"status":"error","error":"Access denied: ..."}` envelope. Inspect both
the MCP error and this envelope. Success text is
`{"status":"success","result":{...}}`. Discovered names include the host
namespace; use those exact names for calls. Write tools are hidden and forced
write calls are rejected in `--read-only` mode.
Public normalization of private operation errors into `isError:true` remains a
Kit-owned follow-up; it does not require rewriting the plugin ABI or weakening
filesystem refusal checks.

Acceptance on Windows uses `scripts/smoke-filesystem.py` with a disposable
root, sibling folder and a real junction, including concurrent junction/name
replacement while reading. Run with `--require-link` to prevent silent fixture
omission. The script retains fixtures and changes no installation. Non-Windows
runtime behavior has not been accepted by this test. Normal-mode write tools,
other reads and arbitrary runtimes are outside this approval.

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

## Release packaging and pilot smoke check

`plugin.toml` declares each active plugin's execution model. Build and registry
planning use that declaration: WASM plugins publish their cdylib; native plugins
publish only `<plugin>-native`, never the standalone CLI wrapper. Draft plugins
remain outside the workspace. Registry versions already recorded are skipped;
a shared PDK change therefore requires new consumer versions when old artifacts
have already shipped. Release uploads must not replace existing assets.

For the git/time pilots, first build the actual release targets:

```text
cargo build --locked -p rune-git --release --target x86_64-pc-windows-msvc --bin rune-git-native --features native
cargo build --locked -p rune-time --release --target wasm32-wasip1 --lib
python scripts/smoke-kit-plugins.py --rune <rune.exe> --git <rune-git-native.exe> --time <rune_time.wasm>
```

The smoke script checks stateless discovery, schemas, annotations, normal/restricted
discovery and harmless calls. After installation, substitute the installed
plugin names `rune-git` and `rune-time` to verify the installed artifacts.
The script does not install plugins or change owner configuration. Record the
exact Kit executable version/hash and Tools build commit with acceptance results.

Released Kit 0.3.1 (`rune`) and 0.4.0 (`rune-kit`) are historical compatibility
targets, not the current protocol recommendation. Use `--legacy-protocol` only
when testing those releases. Current-protocol checks require the 0.5.0 gateway;
its publication status is owned by Kit's release process. Filesystem 0.4.1's
payload and private API need no version bump to work through that gateway.

Release notes, checksum generation, registry handoff, Git ownership, shared
installation paths and exact owner actions are documented in
[PUBLISHING.md](PUBLISHING.md). The former formatting/version-PR automation and
registry source commit/push automation have been removed from the prepared
workflows. The owner must publish the reviewed registry patch after artifacts
exist; until then, existing package discovery remains available.

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
