pub mod definitions;
pub mod operations;
pub mod types;

#[cfg(target_arch = "wasm32")]
use rune_pdk::ToolCallRequest;
#[cfg(target_arch = "wasm32")]
use serde_json::{Value, json};

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
    let result = operations::execute_tool_with_fetcher(request, wasm_fetch);

    let output = match result {
        Ok(val) => json!({ "status": "success", "result": val }),
        Err(err) => json!({ "status": "error", "error": err }),
    };

    Ok(serde_json::to_string(&output)?)
}

#[cfg(target_arch = "wasm32")]
#[extism_pdk::plugin_fn]
pub fn mcp_list_resources(_: ()) -> extism_pdk::FnResult<String> {
    Ok(serde_json::to_string(&definitions::resource_definitions())?)
}

#[cfg(target_arch = "wasm32")]
#[extism_pdk::plugin_fn]
pub fn mcp_read_resource(input: String) -> extism_pdk::FnResult<String> {
    let req: Value = serde_json::from_str(&input)?;
    let uri = req.get("uri").and_then(|v| v.as_str()).unwrap_or("");
    let result = operations::read_resource(uri);

    let output = match result {
        Ok(val) => json!({ "status": "success", "result": val }),
        Err(err) => json!({ "status": "error", "error": err }),
    };

    Ok(serde_json::to_string(&output)?)
}

#[cfg(target_arch = "wasm32")]
#[extism_pdk::plugin_fn]
pub fn mcp_list_prompts(_: ()) -> extism_pdk::FnResult<String> {
    Ok(serde_json::to_string(&definitions::prompt_definitions())?)
}

#[cfg(target_arch = "wasm32")]
#[extism_pdk::plugin_fn]
pub fn mcp_get_prompt(input: String) -> extism_pdk::FnResult<String> {
    let req: Value = serde_json::from_str(&input)?;
    let name = req.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let args = req.get("arguments").unwrap_or(&Value::Null);
    let result = operations::get_prompt(name, args);

    let output = match result {
        Ok(val) => json!({ "status": "success", "result": val }),
        Err(err) => json!({ "status": "error", "error": err }),
    };

    Ok(serde_json::to_string(&output)?)
}

#[cfg(target_arch = "wasm32")]
fn wasm_fetch(url: &str) -> Result<String, String> {
    let req = extism_pdk::HttpRequest::new(url)
        .with_method("GET")
        .with_header(
            "User-Agent",
            concat!("rune-fetch/", env!("CARGO_PKG_VERSION")),
        );

    let res = extism_pdk::http::request::<()>(&req, None).map_err(|e| {
        format!(
            "HTTP request failed: {}. Verify network connectivity and capability permissions.",
            e
        )
    })?;

    let status = res.status_code();
    if !(200..300).contains(&status) {
        return Err(format!(
            "HTTP request returned error status: {}. Check that the URL is accessible.",
            status
        ));
    }

    String::from_utf8(res.body())
        .map_err(|e| format!("Failed to parse response body as UTF-8: {}", e))
}
