#[cfg(not(target_arch = "wasm32"))]
use mhb_mconnect::{definitions, operations};
#[cfg(not(target_arch = "wasm32"))]
use rune_pdk::{ToolCallRequest, ToolDefinition};
#[cfg(not(target_arch = "wasm32"))]
use rune_sidecar::{SidecarHandler, run_stdio};
#[cfg(not(target_arch = "wasm32"))]
use serde_json::{Value, json};

#[cfg(not(target_arch = "wasm32"))]
struct MhbHandler;

#[cfg(not(target_arch = "wasm32"))]
impl SidecarHandler for MhbHandler {
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

    if args.len() >= 3 && args[1] == "--exec" {
        let raw_payload = &args[2];
        let request: ToolCallRequest = match serde_json::from_str(raw_payload) {
            Ok(req) => req,
            Err(e) => {
                let err_resp =
                    json!({ "status": "error", "error": format!("Invalid JSON request: {}", e) });
                println!("{}", err_resp);
                return Ok(());
            }
        };

        match operations::execute_tool(request) {
            Ok(val) => println!("{}", serde_json::to_string(&val).unwrap()),
            Err(err) => {
                let err_resp = json!({ "status": "error", "error": err });
                println!("{}", err_resp);
            }
        }
        return Ok(());
    }

    run_stdio(MhbHandler)
}

#[cfg(target_arch = "wasm32")]
fn main() {}
