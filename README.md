### Architecture & Project Debrief

The **Rune Ecosystem** is a modular Model Context Protocol (MCP) toolchain separating the execution host from sandboxed capabilities:

* **`rune-kit` (Host Runtime):** A native Rust binary (`rune`) serving as the MCP server. It manages stdio JSON-RPC protocol negotiation, injects host-level configuration (such as directory whitelists via `allowed_dir`), and executes plugins inside an Extism/Wasmtime WebAssembly sandbox.
* **`rune-tools` (Plugin Workspace):** A modular collection of tool crates compiling to `wasm32-wasip1`. Each plugin operates in memory isolation with mediated access to external resources.
* **Decoupled Architecture Standard:** To allow native `cargo test` execution on any host OS without linker collisions (`LNK2019` on missing WASM host FFI imports), domain logic and schema definitions must remain pure Rust. Extism PDK macros (`#[plugin_fn]`) are strictly isolated to `src/lib.rs` and gated under `#[cfg(target_arch = "wasm32")]`.

---

### Plugin Directory Structure

Every new plugin added under `plugins/` must follow this standardized modular layout:

```text
plugins/rune-<name>/
├── Cargo.toml                  # Dual-target crate config (cdylib + rlib)
├── src/
│   ├── lib.rs                  # WASM FFI boundary & Extism plugin_fn handlers
│   ├── definitions.rs          # Pure ToolDefinition declarations & JSON schemas
│   ├── operations.rs           # Pure execution router & domain logic
│   └── types.rs                # Request/response deserialization data structs
└── tests/
    ├── contract_tests.rs       # Schema, routing, & type rejection macro tests
    └── operations_tests.rs     # Isolated unit tests for domain business logic

```

---

### Step-by-Step Implementation Guide

#### 1. Register Plugin in Workspace `Cargo.toml`

Add the new plugin path to the root `Cargo.toml`:

```toml
[workspace]
members = [
    "crates/rune-pdk",
    "plugins/rune-filesystem",
    "plugins/rune-<name>",
]

```

#### 2. Create `plugins/rune-<name>/Cargo.toml`

Configure dual crate targets (`cdylib` for WebAssembly packaging and `rlib` for native test linking):

```toml
[package]
name = "rune-<name>"
description = "MCP plugin for <functionality>"
version = "0.1.0"
edition.workspace = true
license.workspace = true

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
rune-pdk = { path = "../../crates/rune-pdk" }
serde.workspace = true
serde_json.workspace = true

[target.'cfg(target_arch = "wasm32")'.dependencies]
extism-pdk.workspace = true

[dev-dependencies]
tempfile = "3.14"

```

---

#### 3. Define Input Structs (`src/types.rs`)

Define all structured arguments expected by the tool handlers:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalculationPayload {
    pub expression: String,
    #[serde(default)]
    pub precision: Option<usize>,
}

```

---

#### 4. Define Tool Schemas (`src/definitions.rs`)

Declare tool contracts. **Every parameter must have a `description` and `type**`, and names must be lowercase `snake_case`:

```rust
use rune_pdk::ToolDefinition;
use serde_json::json;

pub fn tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "calculate".to_string(),
            description: "Evaluates a mathematical expression and returns the formatted result.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "expression": {
                        "type": "string",
                        "description": "Mathematical expression string to evaluate (e.g. '2 + 2')"
                    },
                    "precision": {
                        "type": "number",
                        "description": "Optional decimal precision for floating point output"
                    }
                },
                "required": ["expression"]
            }),
        }
    ]
}

```

---

#### 5. Implement Pure Logic & Router (`src/operations.rs`)

Implement the execution logic. Ensure all operations return structured `Result<serde_json::Value, String>` and unhandled tools return `"Unknown tool: <name>"`:

```rust
use crate::types::CalculationPayload;
use rune_pdk::ToolCallRequest;
use serde_json::{Value, json};

pub fn execute_tool(request: ToolCallRequest) -> Result<Value, String> {
    match request.name.as_str() {
        "calculate" => {
            let expr = request.arguments["expression"]
                .as_str()
                .ok_or_else(|| "Missing 'expression' parameter".to_string())?;

            let precision = request
                .arguments
                .get("precision")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize)
                .unwrap_or(2);

            // Domain logic
            let result_val = evaluate_expression(expr)?;

            Ok(json!({
                "expression": expr,
                "result": format!("{:.precision$}", result_val, precision = precision)
            }))
        }
        unknown => Err(format!("Unknown tool: {}", unknown)),
    }
}

fn evaluate_expression(expr: &str) -> Result<f64, String> {
    if expr.trim().is_empty() {
        return Err("Expression cannot be empty".to_string());
    }
    // Example logic
    Ok(42.0)
}

```

---

#### 6. Implement WASM Boundary Handlers (`src/lib.rs`)

Expose standard MCP endpoints across the WebAssembly FFI boundary:

```rust
pub mod definitions;
pub mod operations;
pub mod types;

#[cfg(target_arch = "wasm32")]
use rune_pdk::ToolCallRequest;
#[cfg(target_arch = "wasm32")]
use serde_json::json;

#[cfg(target_arch = "wasm32")]
#[extism_pdk::plugin_fn]
pub fn mcp_info(_: ()) -> extism_pdk::FnResult<String> {
    let info = json!({
        "name": env!("CARGO_PKG_NAME"),
        "version": env!("CARGO_PKG_VERSION"),
        "description": option_env!("CARGO_PKG_DESCRIPTION")
    });
    Ok(serde_json::to_string(&info)?)
}

#[cfg(target_arch = "wasm32")]
#[extism_pdk::plugin_fn]
pub fn mcp_list_tools(_: ()) -> extism_pdk::FnResult<String> {
    Ok(serde_json::to_string(&definitions::tool_definitions())?)
}

#[cfg(target_arch = "wasm32")]
#[extism_pdk::plugin_fn]
pub fn mcp_call_tool(input: String) -> extism_pdk::FnResult<String> {
    let request: ToolCallRequest = serde_json::from_str(&input)?;
    let result = operations::execute_tool(request);

    let output = match result {
        Ok(val) => json!({ "status": "success", "result": val }),
        Err(err) => json!({ "status": "error", "error": err }),
    };

    Ok(serde_json::to_string(&output)?)
}

```

---

### Testing Architecture

#### Contract Tests (`tests/contract_tests.rs`)

Uses the shared `test_plugin_contract!` macro from `rune-pdk` to automatically test schema validity, routing presence, required-parameter dropping, and invalid type fuzzing:

```rust
use rune_<name>::{definitions::tool_definitions, operations::execute_tool};
use rune_pdk::test_plugin_contract;

test_plugin_contract!(tool_definitions, execute_tool);

```

#### Operations Tests (`tests/operations_tests.rs`)

Implements specific domain unit tests for positive outcomes and failure states:

```rust
use rune_<name>::operations::execute_tool;
use rune_pdk::ToolCallRequest;
use serde_json::json;

#[test]
fn test_calculate_success() {
    let req = ToolCallRequest {
        name: "calculate".to_string(),
        arguments: json!({ "expression": "10 + 5", "precision": 2 }),
    };
    let res = execute_tool(req).unwrap();
    assert_eq!(res["result"], "42.00");
}

#[test]
fn test_calculate_empty_expression() {
    let req = ToolCallRequest {
        name: "calculate".to_string(),
        arguments: json!({ "expression": "" }),
    };
    let res = execute_tool(req);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Expression cannot be empty"));
}

```

---

### Compilation & Testing Commands

```bash
# 1. Run unit, contract, and logic tests for the new plugin natively
cargo test -p rune-<name>

# 2. Run all tests across all workspace crates
cargo test --workspace

# 3. Build the release WebAssembly binary (WASI target)
cargo build -p rune-<name> --target wasm32-wasip1 --release

# 4. Build the release of all WebAssembly binary (WASI target)
cargo build --workspace --target wasm32-wasip1 --release

# 5. Verify the output binary exists
# target/wasm32-wasip1/release/rune_<name>.wasm

```