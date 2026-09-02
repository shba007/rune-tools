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
        Ok(val) => {
            if let Some(content_arr) = val.get("content").and_then(|c| c.as_array()) {
                json!({
                    "content": content_arr,
                    "isError": false
                })
            } else if let Some(text_content) = val.get("content").and_then(|c| c.as_str()) {
                json!({
                    "content": [{ "type": "text", "text": text_content }],
                    "isError": false
                })
            } else {
                json!({
                    "content": [{ "type": "text", "text": serde_json::to_string(&val).unwrap_or_default() }],
                    "isError": false
                })
            }
        }
        Err(err) => json!({
            "content": [{ "type": "text", "text": err }],
            "isError": true
        }),
    };

    Ok(serde_json::to_string(&output)?)
}
