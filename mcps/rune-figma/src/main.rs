use rune_figma::{definitions, operations};
use rune_pdk::{PromptDefinition, ResourceDefinition, ToolCallRequest, ToolDefinition};
use rune_sidecar::{SidecarHandler, run_stdio};
use serde_json::{Value, json};

struct FigmaSidecar;

impl SidecarHandler for FigmaSidecar {
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

    fn list_resources(&self) -> Vec<ResourceDefinition> {
        definitions::resource_definitions()
    }

    fn read_resource(&self, uri: &str) -> Result<Value, String> {
        operations::read_resource(uri)
    }

    fn list_prompts(&self) -> Vec<PromptDefinition> {
        definitions::prompt_definitions()
    }

    fn get_prompt(&self, name: &str, args: Value) -> Result<Value, String> {
        operations::get_prompt(name, &args)
    }
}

fn main() -> std::io::Result<()> {
    run_stdio(FigmaSidecar)
}
