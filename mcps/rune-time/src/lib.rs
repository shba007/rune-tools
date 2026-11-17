pub mod definitions;
pub mod operations;
pub mod types;

#[cfg(test)]
mod output_contract_tests;

use rune_pdk::ToolCallRequest;
use serde_json::{Value, json};

/// The same result boundary is exercised by host tests and the WASM export.
pub fn call_tool(request: ToolCallRequest) -> Value {
    tool_result(operations::execute_tool(request))
}

fn tool_result(result: Result<Value, String>) -> Value {
    match result {
        Ok(value) => {
            // Preserve the exact compact JSON text observed through Kit before
            // structured output was added, including the legacy success envelope.
            let text = json!({"status": "success", "result": value}).to_string();
            rune_pdk::structured_tool_result(
                text,
                value
                    .as_object()
                    .expect("typed time response is an object")
                    .clone(),
            )
        }
        Err(error) => rune_pdk::tool_error(error),
    }
}

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
    Ok(serde_json::to_string(
        &definitions::published_tool_definitions(),
    )?)
}

#[cfg(target_arch = "wasm32")]
#[extism_pdk::plugin_fn]
pub fn mcp_call_tool(input: String) -> extism_pdk::FnResult<String> {
    let request: ToolCallRequest = serde_json::from_str(&input)?;
    Ok(serde_json::to_string(&call_tool(request))?)
}
