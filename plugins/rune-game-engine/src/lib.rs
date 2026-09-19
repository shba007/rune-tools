pub mod definitions;
pub mod operations;
pub mod types;

#[cfg(target_arch = "wasm32")]
#[extism_pdk::plugin_fn]
pub fn mcp_info(_: ()) -> extism_pdk::FnResult<String> {
    let info = serde_json::json!({
        "name": "rune-game-engine",
        "version": "0.1.0",
        "description": "3D Web Game & Simulation Engine MCP plugin providing spline math, procedural biomes, kinematics, and single-file WebGL bundling"
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
pub fn mcp_call_tool(request_json: String) -> extism_pdk::FnResult<String> {
    let req: rune_pdk::ToolCallRequest = serde_json::from_str(&request_json)?;
    match operations::execute_tool(req) {
        Ok(res) => Ok(serde_json::to_string(&serde_json::json!({
            "status": "success",
            "result": res
        }))?),
        Err(err) => Ok(serde_json::to_string(&serde_json::json!({
            "status": "error",
            "error": err
        }))?),
    }
}
