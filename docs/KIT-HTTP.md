# Kit HTTP acceptance

Current public transport is Kit HTTP MCP 2026-07-28. Require an HTTP-capable Kit
build (planned 0.6.0); published 0.5.0 does not guarantee HTTP support. Private
native pipes and WASM exports remain unchanged. No screenshot plugin is added.

The Rust smoke commands retain their existing arguments but launch
`rune-kit serve <artifact> --headless --port 0`. They discover the ephemeral
loopback endpoint from stderr, send the required mirrored MCP headers and
per-request metadata, and parse SSE final frames or JSON errors. Server startup,
HTTP/JSON-RPC errors and malformed responses fail explicitly. No terminal is
required and no automatic operation retry is performed.

Use `cargo xtask smoke-kit --rune PATH --git NATIVE --time WASM --report FILE`
and `cargo xtask smoke-filesystem --rune PATH --plugin WASM --fixture DIR
--require-link --report FILE`. Fixtures must be disposable; the escape junction
must actually point outside the allowed root. Historical stdio compatibility
checks belong to Git history, not the active contract.

Only develop pushes start integration/preparation. Generated files are validated
in that original run, rather than through duplicate PR/release-branch events.
Owner merges preparation into develop and promotes with a merge commit preserving
source ancestry. Every main push validates the prepared plugin set; no special
merge title is required. Empty pending sets are successful no-ops. Registry-only
updates cannot recursively create plugin releases. GITHUB_TOKEN handles PR API
operations; GH_TOKEN handles release-branch pushes. See PUBLISHING.md for details.
No plugin package version changes are needed for harness/workflow documentation.

This migration does not change installation paths, storage, overrides or user
data. Kit owns installation migration and the HTTP server/TUI. Its canonical managed
root is ~/.rune/tools; tests use <fixture>/.rune/tools via RUNE_KIT_DATA_DIR. Screen Insight
capture remains Studio-owned. See Kit docs/MCP-CONTRACT.md for the wire contract.
