pub mod definitions;
pub mod operations;
mod output;
pub mod pilot_reads;
pub mod types;

#[cfg(test)]
mod output_contract_tests;

#[cfg(target_arch = "wasm32")]
pub(crate) fn host_allowed_root() -> Option<String> {
    // Kit must configure both this value and the matching WASI preopen.
    // An environment fallback could disagree with that capability.
    extism_pdk::config::get("allowed_dir").ok().flatten()
}

use rune_pdk::ToolCallRequest;
use serde_json::json;

/// The host-testable boundary used by the WASM export.
pub fn call_tool(request: ToolCallRequest) -> serde_json::Value {
    if request.name == "read_media_file" {
        // Deliberately legacy: Kit caps structured results at 8 MiB, while this
        // tool permits 20 MiB media. Preserve its payload and error behavior.
        return match operations::execute_tool(request) {
            Ok(value) => json!({"status": "success", "result": value}),
            Err(error) => json!({"status": "error", "error": error}),
        };
    }
    let bounded_text = matches!(request.name.as_str(), "read_multiple_files" | "edit_file");
    tool_result(operations::execute_tool_with_items(request), bounded_text)
}

fn tool_result(
    result: Result<(serde_json::Value, Option<output::Listing>), String>,
    bounded_text: bool,
) -> serde_json::Value {
    match result {
        Ok((legacy, Some(listing))) => output::listing_result(legacy, &listing),
        Ok((legacy, None)) if bounded_text => output::text_result(legacy),
        Ok((legacy, None)) => output::success(legacy.clone(), legacy),
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
