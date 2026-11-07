use mhb_mconnect::{definitions, operations};
use rune_pdk::{ToolCallRequest, ToolDefinition};
use rune_sidecar::{SidecarHandler, run_stdio};
use serde_json::{Value, json};

struct MhbSidecar;

impl SidecarHandler for MhbSidecar {
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

fn main() -> std::io::Result<()> {
    run_stdio(MhbSidecar)
}
