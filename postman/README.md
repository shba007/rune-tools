Current transport and release trigger amendment: [Kit HTTP](../docs/KIT-HTTP.md). This supersedes historical stdio/PR-trigger examples below.

# Rune Tools MCP collection

Open this repository in Postman v12 Local View and select **Rune Kit** under
`postman/collections`. It replaces the Rune Tools Distribution HTTP collection.
All calls use Kit public MCP **2026-07-28 over stdio**; there is no HTTP adapter.

## Layout and coverage

- One folder for each of the 17 source plugins, containing **every declared
  tool call** (140 total), with its input schema, plugin rules and required
  argument template in the request description/body.
- Gateway contains the 12 moved Kit discovery, resource, read-only pilot and
  protocol-refusal examples. These retain their earlier generic tool variables.
- `tool-coverage.json` records the exact source tool-name inventory.
- Includes rune-print, rune-scan and rune-ssh even though they are excluded from
  the current Cargo workspace. Source coverage does not mean a plugin is
  installed, released, configured or available on your platform.

## Configure before sending

Install Kit 0.5.0+ and the relevant plugin separately. Set `kit_executable` to
`rune-kit` on PATH or an absolute executable path. Paths with spaces are quoted.
For each plugin, collection variables `<plugin>_selector` and
`<plugin>_namespace` use underscores in the variable key (e.g. rune_git_selector).
Selector defaults to the installed package name; it can be an absolute artifact
path. Set namespace to the exact prefix returned by tools/list, particularly
when selecting a local binary rather than an installed package.

`<plugin>_parameters` supplies explicit Kit `-p key=value` arguments. Read that
plugin's README for dependencies, credentials, host services and supported OS.
Filesystem parameters require a real existing absolute `allowed_root`; no folder
is implicitly exposed. Keep credentials in local Postman values/Vault, never
in these files or shared values. The generator does not discover credentials.

Every tool body is an **argument template**. Replace `<set this value>`, review
all required fields, and add optional properties using the included exact input
schema. Defaults and enum choices are used where available; generated examples
are not a claim of semantically valid application data. Tool-less objects use
empty arguments. Consult schema descriptions for paths, IDs, units and formats.

`read_only_flag` defaults to `--read-only`. It intentionally blocks mutating
tools even though their requests are present. To test a write-capable tool,
deliberately clear this variable only for the chosen request/local session,
review all arguments and external effects, and use disposable resources.
Do not bulk-run this collection with read-only mode disabled: tools include
sending mail, changing Git state, filesystem writes and device operations.
No tool call is executed by collection generation or validation.

## Maintain and validate

```powershell
$env:CARGO_HUSKY_DONT_INSTALL_HOOKS='true'
cargo xtask postman
cargo xtask postman --check
cargo test --locked -p xtask postman::tests
postman collection lint 'postman/collections/Rune Kit' --fail-severity warning
```

The Rust generator compiles each plugin's actual definitions.rs against rune-pdk;
it does not scrape Rust text or modify runtime plugins. --check verifies generated
content and per-plugin request counts without writing. New plugins must be added
to xtask/src/postman.rs. Removed/renamed tools fail on stale files rather than
silently deleting hand edits. Generation updates only generated plugin requests,
folder metadata, root variables and coverage; Gateway examples are maintained
separately. Postman YAML and its client settings remain Postman's native format.

Validation: 140 source tool calls in 17 plugin folders; all **152** requests pass
Postman CLI 1.75.0 lint with zero errors/warnings. That CLI skips MCP requests
as unsupported, so a successful CLI run exit code is not MCP execution evidence.
Desktop Postman execution and plugin effects are **NOT CERTAIN** (not run here).
Previously, four Gateway wire messages passed direct Windows stdio checks with
local Kit 0.5.0 and Time WASM 0.3.0; that does not verify these 140 new calls.
Protocol timing/cancellation checks remain in Kit's Rust harness.

Format references:
- https://learning.postman.com/docs/use/use-collections/collections-schemas/
- https://learning.postman.com/docs/use/send-requests/protocols/mcp-requests/create
