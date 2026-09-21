#[cfg(not(target_arch = "wasm32"))]
use rune_browser::{definitions, operations};
#[cfg(not(target_arch = "wasm32"))]
use rune_pdk::{ToolCallRequest, ToolDefinition};
#[cfg(not(target_arch = "wasm32"))]
use rune_sidecar::{SidecarHandler, run_stdio};
#[cfg(not(target_arch = "wasm32"))]
use serde_json::{Value, json};

#[cfg(not(target_arch = "wasm32"))]
struct BrowserSidecarHandler;

#[cfg(not(target_arch = "wasm32"))]
impl SidecarHandler for BrowserSidecarHandler {
    fn info(&self) -> Value {
        json!({
            "name": env!("CARGO_PKG_NAME"),
            "version": env!("CARGO_PKG_VERSION"),
            "description": option_env!("CARGO_PKG_DESCRIPTION")
        })
    }

    fn list_tools(&self) -> Vec<ToolDefinition> {
        definitions::tool_definitions()
    }

    fn call_tool(&self, req: ToolCallRequest) -> Result<Value, String> {
        operations::execute_tool(req)
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // One-shot execution invoked by WASM host_cmd_exec
    if args.len() >= 3 && args[1] == "--exec" {
        let raw_payload = &args[2];
        let request: ToolCallRequest = match serde_json::from_str(raw_payload) {
            Ok(req) => req,
            Err(e) => {
                println!("{}", json!({ "status": "error", "error": format!("Invalid JSON request: {}", e) }));
                return Ok(());
            }
        };

        match operations::execute_tool(request) {
            Ok(val) => println!("{}", serde_json::to_string(&val).unwrap()),
            Err(err) => println!("{}", json!({ "status": "error", "error": err })),
        }
        return Ok(());
    }

    // Default persistent stdio MCP mode
    run_stdio(BrowserSidecarHandler)
}

#[cfg(target_arch = "wasm32")]
fn main() {}