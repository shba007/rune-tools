#[cfg(not(target_arch = "wasm32"))]
use rune_pdk::{ToolCallRequest, ToolDefinition};
#[cfg(not(target_arch = "wasm32"))]
use rune_sidecar::{run_stdio, SidecarHandler};
#[cfg(not(target_arch = "wasm32"))]
use rune_video::{definitions, operations};
#[cfg(not(target_arch = "wasm32"))]
use serde_json::{json, Value};

#[cfg(not(target_arch = "wasm32"))]
struct VideoSidecarHandler;

#[cfg(not(target_arch = "wasm32"))]
impl SidecarHandler for VideoSidecarHandler {
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
    run_stdio(VideoSidecarHandler)
}

#[cfg(target_arch = "wasm32")]
fn main() {}
