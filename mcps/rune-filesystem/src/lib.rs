pub mod access;
pub mod definitions;
pub mod operations;
mod output;
pub mod pilot_reads;
mod read_tools;
mod staged;
pub mod types;

#[cfg(test)]
mod output_contract_tests;

#[cfg(target_arch = "wasm32")]
pub(crate) fn host_allowed_root() -> Option<String> {
    extism_pdk::config::get("allowed_dir").ok().flatten()
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn host_staged_root() -> Result<Option<String>, String> {
    let staged = extism_pdk::config::get("rune_staged").ok().flatten();
    if staged.as_deref() != Some("true") {
        return Ok(None);
    }
    match extism_pdk::config::get("rune_staged_root").ok().flatten() {
        Some(root) if !root.trim().is_empty() => Ok(Some(root)),
        _ => Err("Staged mode is on but rune_staged_root is missing; update rune-kit".into()),
    }
}

#[cfg(target_arch = "wasm32")]
#[extism_pdk::host_fn("extism:host/user")]
extern "ExtismHost" {
    fn host_file_access(input: String) -> String;
}

#[cfg(target_arch = "wasm32")]
struct HostBroker;

#[cfg(target_arch = "wasm32")]
impl access::Broker for HostBroker {
    fn call(&self, request: serde_json::Value) -> Result<serde_json::Value, String> {
        // SAFETY: the host function is provided by rune-kit for every plugin load.
        let output = unsafe { host_file_access(request.to_string()) }
            .map_err(|e| format!("host_file_access failed: {e}"))?;
        serde_json::from_str(&output).map_err(|e| format!("Invalid host_file_access reply: {e}"))
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn host_broker() -> Box<dyn access::Broker> {
    Box::new(HostBroker)
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn host_broker() -> Box<dyn access::Broker> {
    struct Unavailable;
    impl access::Broker for Unavailable {
        fn call(&self, _: serde_json::Value) -> Result<serde_json::Value, String> {
            Err("host_file_access is only available inside rune-kit".into())
        }
    }
    Box::new(Unavailable)
}

use rune_pdk::ToolCallRequest;
use serde_json::json;

pub fn call_tool(request: ToolCallRequest) -> serde_json::Value {
    call_with(request, |request| {
        operations::execute_tool_with_items(request)
    })
}

pub fn call_tool_in(request: ToolCallRequest, mode: &operations::Mode) -> serde_json::Value {
    call_with(request, |request| {
        operations::execute_in_mode(request, mode)
    })
}

fn call_with(
    request: ToolCallRequest,
    run: impl FnOnce(ToolCallRequest) -> Result<(serde_json::Value, Option<output::Listing>), String>,
) -> serde_json::Value {
    if request.name == "read_media_file" {
        return match run(request) {
            Ok((value, _)) => json!({"status": "success", "result": value}),
            Err(error) => json!({"status": "error", "error": error}),
        };
    }
    let bounded_text = matches!(request.name.as_str(), "read_multiple_files" | "edit_file");
    tool_result(run(request), bounded_text)
}

fn tool_result(
    result: Result<(serde_json::Value, Option<output::Listing>), String>,
    bounded_text: bool,
) -> serde_json::Value {
    let staged = matches!(&result, Ok((legacy, _)) if legacy["staged"] == true);
    let mut out = match result {
        Ok((legacy, Some(listing))) => output::listing_result(legacy, &listing),
        Ok((legacy, None)) if bounded_text => output::text_result(legacy),
        Ok((legacy, None)) => output::success(legacy.clone(), legacy),
        Err(error) => return rune_pdk::tool_error(error),
    };
    if staged && let Some(structured) = out["structuredContent"].as_object_mut() {
        structured.insert("staged".into(), json!(true));
    }
    out
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
