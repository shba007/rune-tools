# Captured legacy tool output

`kit-before.json` contains actual MCP HTTP responses captured **before editing
rune-time**, not reconstructed expectations. Tests compare the literal `text`
strings; the fixture includes both successful tools and both invalid-timezone
errors.

Source: code-tools commit `0a89308a80c094d98338076cb2b92160c2b9f59f`.
The pre-existing staged documentation changes were left intact.

Unmodified plugin build:

```powershell
$env:CARGO_HUSKY_DONT_INSTALL_HOOKS = 'true'
cargo build --locked -p rune-time --release --target wasm32-wasip1
```

SHA-256 of that WASM:
`D01B6EF4084F7B7753FBC4CE28B1DB34869D3852E98128EB1D17106C591B1161`.

Host: the existing Kit 0.9.0 executable copied from
`D:\Projects\Public\rune\code-kit\target\debug\rune-kit.exe` into an isolated
directory outside all repositories. Its SHA-256 was
`B2C44F46EC086F2F0905740BF4170C2364D33A0B6151480A9910991A199B3B4F`.
This executable supports output schemas; it is **not** an older-Kit test.

The external directory used for the capture was:

```text
C:\Users\shba007\.codex\visualizations\2026\11\16\01a1e663-6c7e-7320-b80a-dd3b23d1cb54\rune-t1
```

From there, with `RUNE_KIT_DATA_DIR` pointing to its `data` subdirectory:

```powershell
.\rune-kit.exe serve .\rune_time_before.wasm --headless --port 47620
```

The capture client called `tools/list`, selected the namespaced tools, then sent
`tools/call` for the four fixed argument objects recorded in the fixture. Every
request carried `MCP-Protocol-Version: 2026-07-28`, `MCP-Method`, and, for calls,
`MCP-Name`, plus the matching `_meta` protocol version and empty client
capabilities. SSE `data:` messages were parsed and their JSON responses saved.
The exact capture script and raw before/after captures remain in that external
directory as `capture.ps1`, `before-tools.json`, `before-calls.json`, and
`after-existing-kit-*.json`.

The current-time call used the real clock. Its recorded UTC instant is replayed
only by unit tests through a `#[cfg(test)]` function. No shipped input, config
field, environment variable, or fixed clock was introduced. Conversion uses the
fixed supplied datetime. The success text includes the original compact
`result`/`status` envelope. Errors retain their exact captured text; with owner
approval, the new error result has `isError: true` so Kit bypasses output-schema
validation.

Older-Kit compatibility was checked **from code only**, at Kit commit
`4a6d25c`: its WASM runtime parses arbitrary JSON; its tool definition ignores
unknown output-schema fields; its router preserves a supplied content array and
reads `isError`. It does not forward structured content. No older Kit was run,
and code-tools has no dependency on Kit.
