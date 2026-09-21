pub mod definitions;
pub mod operations;
pub mod types;

pub fn resolve_dir(dir_param: Option<&str>) -> String {
    let explicit = dir_param.map(ToString::to_string).or_else(|| {
        std::env::var("OUTPUT_DIRECTORY")
            .or_else(|_| std::env::var("OUTPUT_DIR"))
            .or_else(|_| std::env::var("ALLOWED_DIR"))
            .ok()
    });
    explicit.unwrap_or_else(|| ".".to_string())
}

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
#[extism_pdk::host_fn("extism:host/user")]
extern "ExtismHost" {
    pub fn host_cmd_exec(input: String) -> String;
}

#[cfg(target_arch = "wasm32")]
#[extism_pdk::plugin_fn]
pub fn mcp_call_tool(input: String) -> extism_pdk::FnResult<String> {
    // 1. Resilient request parsing (supports standard, router-prefixed, and raw JSON-RPC structures)
    let request: ToolCallRequest = match serde_json::from_str::<ToolCallRequest>(&input) {
        Ok(mut req) => {
            if let Some(pos) = req.name.rfind("__") {
                req.name = req.name[pos + 2..].to_string();
            }
            req
        }
        Err(_) => {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&input) {
                let raw_name = val.get("name")
                    .or_else(|| val.get("params").and_then(|p| p.get("name")))
                    .and_then(|n| n.as_str())
                    .unwrap_or("");
                let clean_name = raw_name.rfind("__").map(|p| &raw_name[p + 2..]).unwrap_or(raw_name);
                let args = val.get("arguments")
                    .or_else(|| val.get("params").and_then(|p| p.get("arguments")))
                    .cloned()
                    .unwrap_or_else(|| json!({}));

                ToolCallRequest {
                    name: clean_name.to_string(),
                    arguments: args,
                }
            } else {
                return Ok(serde_json::to_string(&json!({
                    "status": "error",
                    "error": format!("Invalid JSON request: {}", input)
                }))?);
            }
        }
    };

    // 2. Dispatch via operations::execute_tool; catch all errors to prevent Wasmtime traps
    match operations::execute_tool(request) {
        Ok(val) => Ok(serde_json::to_string(&val)?),
        Err(err) => Ok(serde_json::to_string(&json!({
            "status": "error",
            "error": err
        }))?),
    }
}