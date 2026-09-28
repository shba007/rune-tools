pub mod definitions;
pub mod operations;
pub mod types;

use std::path::PathBuf;

pub fn get_config(key: &str) -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        extism_pdk::config::get(key)
            .ok()
            .flatten()
            .or_else(|| std::env::var(key.to_ascii_uppercase()).ok())
            .filter(|s| !s.is_empty())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::env::var(key.to_ascii_uppercase())
            .or_else(|_| std::env::var(key))
            .ok()
            .filter(|s| !s.is_empty())
    }
}

pub fn resolve_dir(dir_param: Option<&str>) -> String {
    let explicit = dir_param.map(ToString::to_string).or_else(|| {
        std::env::var("OUTPUT_DIRECTORY")
            .or_else(|_| std::env::var("OUTPUT_DIR"))
            .or_else(|_| std::env::var("ALLOWED_DIR"))
            .ok()
    });

    let raw = explicit.unwrap_or_else(|| ".".to_string());
    let target = PathBuf::from(raw);

    if let Some(allowed_root) = get_config("allowed_dir") {
        let root = PathBuf::from(allowed_root);
        if target.is_relative() {
            root.join(target).to_string_lossy().to_string()
        } else {
            target.to_string_lossy().to_string()
        }
    } else {
        target.to_string_lossy().to_string()
    }
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
    let request: ToolCallRequest = match serde_json::from_str::<ToolCallRequest>(&input) {
        Ok(mut req) => {
            if let Some(pos) = req.name.rfind("__") {
                req.name = req.name[pos + 2..].to_string();
            }
            req
        }
        Err(_) => {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&input) {
                let raw_name = val
                    .get("name")
                    .or_else(|| val.get("params").and_then(|p| p.get("name")))
                    .and_then(|n| n.as_str())
                    .unwrap_or("");
                let clean_name = raw_name
                    .rfind("__")
                    .map(|p| &raw_name[p + 2..])
                    .unwrap_or(raw_name);
                let args = val
                    .get("arguments")
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

    let output = match operations::execute_tool(request) {
        Ok(val) => json!({ "status": "success", "result": val }),
        Err(err) => json!({ "status": "error", "error": err }),
    };

    Ok(serde_json::to_string(&output)?)
}
