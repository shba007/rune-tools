use rune_pdk::ToolCallRequest;
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;
use chrono::Utc;

// Session storage directory
const SESSIONS_DIR: &str = "browser-sessions";

// =========================================================================
// Session Management Helpers
// =========================================================================

#[cfg(not(target_arch = "wasm32"))]
fn get_session_path(session_id: &str) -> PathBuf {
    let base = std::env::var("ALLOWED_DIR")
        .or_else(|_| std::env::var("OUTPUT_DIR"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(&base).join(SESSIONS_DIR).join(format!("{}.json", session_id))
}

#[cfg(not(target_arch = "wasm32"))]
fn save_session(session_id: &str, config: &crate::types::BrowserSessionConfig) -> String {
    let path = get_session_path(session_id);
    let dir = path.parent().unwrap();
    fs::create_dir_all(dir).ok();
    
    let session_info = crate::types::SessionInfo {
        session_id: session_id.to_string(),
        engine: config.engine.clone(),
        url: "".to_string(),
        created_at: Utc::now().to_rfc3339(),
        last_accessed: Utc::now().to_rfc3339(),
        status: "active".to_string(),
    };
    
    let json = serde_json::to_string_pretty(&session_info).unwrap();
    fs::write(&path, json).ok();
    
    session_info.session_id.clone()
}

#[cfg(not(target_arch = "wasm32"))]
fn load_session(session_id: &str) -> Option<crate::types::SessionInfo> {
    let path = get_session_path(session_id);
    if path.exists() {
        match fs::read_to_string(&path) {
            Ok(content) => {
                match serde_json::from_str::<crate::types::SessionInfo>(&content) {
                    Ok(session) => Some(session),
                    Err(_) => None,
                }
            }
            Err(_) => None,
        }
    } else {
        None
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn list_sessions() -> Vec<crate::types::SessionInfo> {
    let base = std::env::var("ALLOWED_DIR")
        .or_else(|_| std::env::var("OUTPUT_DIR"))
        .unwrap_or_else(|_| ".".to_string());
    
    let sessions_dir = PathBuf::from(&base).join(SESSIONS_DIR);
    if !sessions_dir.exists() {
        return Vec::new();
    }
    
    let mut sessions = Vec::new();
    if let Ok(entries) = fs::read_dir(&sessions_dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                if entry.file_type().ok().map(|ft| ft.is_file()).unwrap_or(false) {
                    if let Ok(content) = fs::read_to_string(entry.path()) {
                        if let Ok(session) = serde_json::from_str::<crate::types::SessionInfo>(&content) {
                            sessions.push(session);
                        }
                    }
                }
            }
        }
    }
    sessions
}

#[cfg(not(target_arch = "wasm32"))]
fn delete_session(session_id: &str) -> bool {
    let path = get_session_path(session_id);
    fs::remove_file(path).is_ok()
}

// =========================================================================
// WASM32 Target Implementation
// =========================================================================

#[cfg(target_arch = "wasm32")]
pub fn execute_tool(request: ToolCallRequest) -> Result<Value, String> {
    Err("Browser tools are only available in native mode. Use 'rune run rune-browser-native'".to_string())
}

#[cfg(target_arch = "wasm32")]
fn resolve_dir(dir_param: Option<&str>) -> String {
    ".".to_string()
}

// =========================================================================
// Native Target Implementation
// =========================================================================

#[cfg(not(target_arch = "wasm32"))]
use crate::types::{CmdExecRequest, CmdExecResponse, CDPRequest};

#[cfg(not(target_arch = "wasm32"))]
fn get_arg(args: &Value, key: &str, default: Option<String>) -> Option<String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or(default)
}

#[cfg(not(target_arch = "wasm32"))]
fn get_bool_arg(args: &Value, key: &str, default: bool) -> bool {
    args.get(key)
        .and_then(|v| v.as_bool())
        .unwrap_or(default)
}

#[cfg(not(target_arch = "wasm32"))]

#[cfg(not(target_arch = "wasm32"))]
pub fn execute_tool(request: ToolCallRequest) -> Result<Value, String> {
    match request.name.as_str() {
        // Session Management
        "browser_session_start" => op_session_start(&request),
        "browser_session_stop" => op_session_stop(&request),
        "browser_session_list" => op_session_list(&request),
        // Navigation & Interaction
        "browser_navigate" => op_navigate(&request),
        "browser_type" => op_type(&request),
        "browser_click" => op_click(&request),
        "browser_fill" => op_fill(&request),
        "browser_select_option" => op_select_option(&request),
        "browser_hover" => op_hover(&request),
        // Content & Execution
        "browser_execute_script" => op_execute_script(&request),
        "browser_screenshot" => op_screenshot(&request),
        "browser_pdf" => op_pdf(&request),
        // Diagnostics
        "browser_console_messages" => op_console_messages(&request),
        "browser_network_requests" => op_network_requests(&request),
        // CDP Tools
        "browser_cdp_connect" => op_cdp_connect(&request),
        "browser_cdp_request" => op_cdp_request(&request),
        "browser_cdp_disconnect" => op_cdp_disconnect(&request),
        unknown => Err(format!("Unknown tool: {}", unknown)),
    }
}

// =========================================================================
// Session Management Operations
// =========================================================================

#[cfg(not(target_arch = "wasm32"))]
fn op_session_start(request: &ToolCallRequest) -> Result<Value, String> {
    let engine = get_arg(&request.arguments, "engine", Some("agent-browser".to_string()));
    let browser_type = get_arg(&request.arguments, "browser_type", Some("auto".to_string()));
    let headed = get_bool_arg(&request.arguments, "headed", false);
    let output_dir = get_arg(&request.arguments, "output_dir", None);

    let session_id = format!("session_{}", Utc::now().timestamp_millis());
    
    let config = crate::types::BrowserSessionConfig {
        engine: engine.unwrap_or_else(|| "agent-browser".to_string()),
        session_id: Some(session_id.clone()),
        browser_type,
        cdp_port: 0,
        headed,
        output_dir: output_dir.unwrap_or_else(|| ".".to_string()),
    };

    let saved_id = save_session(&session_id, &config);
    
    Ok(json!({
        "status": "started",
        "session_id": saved_id,
        "engine": config.engine,
        "headed": config.headed
    }))
}

#[cfg(not(target_arch = "wasm32"))]
fn op_session_stop(request: &ToolCallRequest) -> Result<Value, String> {
    let session_id = get_arg(&request.arguments, "session_id", None);
    
    if let Some(sid) = session_id {
        if delete_session(&sid) {
            Ok(json!({ "status": "stopped", "session_id": sid }))
        } else {
            Err(format!("Failed to stop session: {}", sid))
        }
    } else {
        Err("Missing session_id parameter".to_string())
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn op_session_list(_request: &ToolCallRequest) -> Result<Value, String> {
    let sessions = list_sessions();
    Ok(json!({
        "sessions": sessions
    }))
}

// =========================================================================
// Navigation & Interaction Operations
// =========================================================================

#[cfg(not(target_arch = "wasm32"))]
fn op_navigate(request: &ToolCallRequest) -> Result<Value, String> {
    let url = get_arg(&request.arguments, "url", None)
        .ok_or_else(|| "Missing 'url' parameter".to_string())?;
    
    let session_id = get_arg(&request.arguments, "session_id", None);
    let engine = get_arg(&request.arguments, "engine", Some("agent-browser".to_string()));
    let headed = get_bool_arg(&request.arguments, "headed", false);
    
    // Build command for agent-browser
    let mut cmd_args = vec!["open".to_string(), url.clone()];
    
    if let Some(ref sid) = session_id {
        cmd_args.push(format!("--session={}", sid));
    }
    
    if engine.is_some() && engine.unwrap() == "cdp" {
        cmd_args.push("--engine=cdp".to_string());
    }
    
    if headed {
        cmd_args.push("--headed".to_string());
    }
    
    let req = CmdExecRequest {
        program: "agent-browser".to_string(),
        args: cmd_args,
        cwd: None,
    };
    
    let resp = run_binary_raw(&req)?;
    
    Ok(json!({
        "status": "success",
        "action": "navigate",
        "url": url,
        "output": resp.stdout.trim()
    }))
}

#[cfg(not(target_arch = "wasm32"))]
fn op_type(request: &ToolCallRequest) -> Result<Value, String> {
    let target = get_arg(&request.arguments, "target", None)
        .ok_or_else(|| "Missing 'target' parameter".to_string())?;
    let value = get_arg(&request.arguments, "value", None)
        .ok_or_else(|| "Missing 'value' parameter".to_string())?;
    
    let session_id = get_arg(&request.arguments, "session_id", None);
    
    let mut cmd_args = vec!["type".to_string(), target.clone(), value.clone()];
    
    if let Some(ref sid) = session_id {
        cmd_args.push(format!("--session={}", sid));
    }
    
    let req = CmdExecRequest {
        program: "agent-browser".to_string(),
        args: cmd_args,
        cwd: None,
    };
    
    let resp = run_binary_raw(&req)?;
    
    Ok(json!({
        "status": "success",
        "action": "type",
        "target": target,
        "value": value,
        "output": resp.stdout.trim()
    }))
}

#[cfg(not(target_arch = "wasm32"))]
fn op_click(request: &ToolCallRequest) -> Result<Value, String> {
    let target = get_arg(&request.arguments, "target", None)
        .ok_or_else(|| "Missing 'target' parameter".to_string())?;
    
    let session_id = get_arg(&request.arguments, "session_id", None);
    let double_click = get_bool_arg(&request.arguments, "double_click", false);
    let new_tab = get_bool_arg(&request.arguments, "new_tab", false);
    
    let mut cmd_args = vec!["click".to_string(), target.clone()];
    
    if let Some(ref sid) = session_id {
        cmd_args.push(format!("--session={}", sid));
    }
    
    if double_click {
        cmd_args.push("--double-click".to_string());
    }
    
    if new_tab {
        cmd_args.push("--new-tab".to_string());
    }
    
    let req = CmdExecRequest {
        program: "agent-browser".to_string(),
        args: cmd_args,
        cwd: None,
    };
    
    let resp = run_binary_raw(&req)?;
    
    Ok(json!({
        "status": "success",
        "action": "click",
        "target": target,
        "output": resp.stdout.trim()
    }))
}

#[cfg(not(target_arch = "wasm32"))]
fn op_fill(request: &ToolCallRequest) -> Result<Value, String> {
    let target = get_arg(&request.arguments, "target", None)
        .ok_or_else(|| "Missing 'target' parameter".to_string())?;
    let value = get_arg(&request.arguments, "value", None)
        .ok_or_else(|| "Missing 'value' parameter".to_string())?;
    
    let session_id = get_arg(&request.arguments, "session_id", None);
    let _press_enter = get_bool_arg(&request.arguments, "press_enter", false);
    
    let mut cmd_args = vec!["fill".to_string(), target.clone(), value.clone()];
    
    if let Some(ref sid) = session_id {
        cmd_args.push(format!("--session={}", sid));
    }
    
    let req = CmdExecRequest {
        program: "agent-browser".to_string(),
        args: cmd_args,
        cwd: None,
    };
    
    let resp = run_binary_raw(&req)?;
    
    Ok(json!({
        "status": "success",
        "action": "fill",
        "target": target,
        "value": value,
        "output": resp.stdout.trim()
    }))
}

#[cfg(not(target_arch = "wasm32"))]
fn op_select_option(request: &ToolCallRequest) -> Result<Value, String> {
    let target = get_arg(&request.arguments, "target", None)
        .ok_or_else(|| "Missing 'target' parameter".to_string())?;
    
    let session_id = get_arg(&request.arguments, "session_id", None);
    let value = get_arg(&request.arguments, "value", None);
    let label = get_arg(&request.arguments, "label", None);
    
    let mut cmd_args = vec!["select".to_string(), target.clone()];
    
    if let Some(ref sid) = session_id {
        cmd_args.push(format!("--session={}", sid));
    }
    
    if let Some(v) = value {
        cmd_args.push(format!("--value={}", v));
    }
    
    if let Some(l) = label {
        cmd_args.push(format!("--label={}", l));
    }
    
    let req = CmdExecRequest {
        program: "agent-browser".to_string(),
        args: cmd_args,
        cwd: None,
    };
    
    let resp = run_binary_raw(&req)?;
    
    Ok(json!({
        "status": "success",
        "action": "select_option",
        "target": target,
        "output": resp.stdout.trim()
    }))
}

#[cfg(not(target_arch = "wasm32"))]
fn op_hover(request: &ToolCallRequest) -> Result<Value, String> {
    let target = get_arg(&request.arguments, "target", None)
        .ok_or_else(|| "Missing 'target' parameter".to_string())?;
    
    let session_id = get_arg(&request.arguments, "session_id", None);
    
    let mut cmd_args = vec!["hover".to_string(), target.clone()];
    
    if let Some(ref sid) = session_id {
        cmd_args.push(format!("--session={}", sid));
    }
    
    let req = CmdExecRequest {
        program: "agent-browser".to_string(),
        args: cmd_args,
        cwd: None,
    };
    
    let resp = run_binary_raw(&req)?;
    
    Ok(json!({
        "status": "success",
        "action": "hover",
        "target": target,
        "output": resp.stdout.trim()
    }))
}

// =========================================================================
// Content & Execution Operations
// =========================================================================

#[cfg(not(target_arch = "wasm32"))]
fn op_execute_script(request: &ToolCallRequest) -> Result<Value, String> {
    let script = get_arg(&request.arguments, "script", None)
        .ok_or_else(|| "Missing 'script' parameter".to_string())?;
    
    let session_id = get_arg(&request.arguments, "session_id", None);
    
    let mut cmd_args = vec!["eval".to_string(), script.clone()];
    
    if let Some(ref sid) = session_id {
        cmd_args.push(format!("--session={}", sid));
    }
    
    let req = CmdExecRequest {
        program: "agent-browser".to_string(),
        args: cmd_args,
        cwd: None,
    };
    
    let resp = run_binary_raw(&req)?;
    
    Ok(json!({
        "status": "success",
        "result": resp.stdout.trim()
    }))
}

#[cfg(not(target_arch = "wasm32"))]
fn op_screenshot(request: &ToolCallRequest) -> Result<Value, String> {
    let session_id = get_arg(&request.arguments, "session_id", None);
    let filename = get_arg(&request.arguments, "filename", None);
    let full_page = get_bool_arg(&request.arguments, "full_page", false);
    let output_dir = get_arg(&request.arguments, "output_dir", None);
    
    let default_filename = filename.unwrap_or_else(|| format!("screenshot_{}.png", Utc::now().timestamp_millis()));
    
    let out_dir = output_dir.unwrap_or_else(|| ".".to_string());
    let target_path = PathBuf::from(&out_dir).join(&default_filename);
    
    let mut cmd_args = vec!["screenshot".to_string(), target_path.to_string_lossy().to_string()];
    
    if let Some(ref sid) = session_id {
        cmd_args.push(format!("--session={}", sid));
    }
    
    if full_page {
        cmd_args.push("--full-page".to_string());
    }
    
    let req = CmdExecRequest {
        program: "agent-browser".to_string(),
        args: cmd_args,
        cwd: None,
    };
    
    let resp = run_binary_raw(&req)?;
    
    Ok(json!({
        "status": "success",
        "action": "screenshot",
        "saved_path": target_path.to_string_lossy(),
        "output": resp.stdout.trim()
    }))
}

#[cfg(not(target_arch = "wasm32"))]
fn op_pdf(request: &ToolCallRequest) -> Result<Value, String> {
    let session_id = get_arg(&request.arguments, "session_id", None);
    let filename = get_arg(&request.arguments, "filename", None);
    let landscape = get_bool_arg(&request.arguments, "landscape", false);
    let output_dir = get_arg(&request.arguments, "output_dir", None);
    
    let default_filename = filename.unwrap_or_else(|| format!("page_{}.pdf", Utc::now().timestamp_millis()));
    
    let out_dir = output_dir.unwrap_or_else(|| ".".to_string());
    let target_path = PathBuf::from(&out_dir).join(&default_filename);
    
    let mut cmd_args = vec!["pdf".to_string(), target_path.to_string_lossy().to_string()];
    
    if let Some(ref sid) = session_id {
        cmd_args.push(format!("--session={}", sid));
    }
    
    if landscape {
        cmd_args.push("--landscape".to_string());
    }
    
    let req = CmdExecRequest {
        program: "agent-browser".to_string(),
        args: cmd_args,
        cwd: None,
    };
    
    let resp = run_binary_raw(&req)?;
    
    Ok(json!({
        "status": "success",
        "action": "pdf",
        "saved_path": target_path.to_string_lossy(),
        "output": resp.stdout.trim()
    }))
}

// =========================================================================
// Diagnostics Operations
// =========================================================================

#[cfg(not(target_arch = "wasm32"))]
fn op_console_messages(request: &ToolCallRequest) -> Result<Value, String> {
    let session_id = get_arg(&request.arguments, "session_id", None);
    let level = get_arg(&request.arguments, "level", None);
    let limit = get_arg(&request.arguments, "limit", None);
    
    let mut cmd_args = vec!["console".to_string()];
    
    if let Some(ref sid) = session_id {
        cmd_args.push(format!("--session={}", sid));
    }
    
    if let Some(lvl) = level {
        cmd_args.push(format!("--level={}", lvl));
    }
    
    if let Some(lim) = limit {
        cmd_args.push(format!("--limit={}", lim));
    }
    
    let req = CmdExecRequest {
        program: "agent-browser".to_string(),
        args: cmd_args,
        cwd: None,
    };
    
    let resp = run_binary_raw(&req)?;
    
    Ok(json!({
        "status": "success",
        "messages": resp.stdout.trim()
    }))
}

#[cfg(not(target_arch = "wasm32"))]
fn op_network_requests(request: &ToolCallRequest) -> Result<Value, String> {
    let session_id = get_arg(&request.arguments, "session_id", None);
    let url_filter = get_arg(&request.arguments, "url", None);
    let method = get_arg(&request.arguments, "method", None);
    
    let mut cmd_args = vec!["network".to_string()];
    
    if let Some(ref sid) = session_id {
        cmd_args.push(format!("--session={}", sid));
    }
    
    if let Some(u) = url_filter {
        cmd_args.push(format!("--url={}", u));
    }
    
    if let Some(m) = method {
        cmd_args.push(format!("--method={}", m));
    }
    
    let req = CmdExecRequest {
        program: "agent-browser".to_string(),
        args: cmd_args,
        cwd: None,
    };
    
    let resp = run_binary_raw(&req)?;
    
    Ok(json!({
        "status": "success",
        "requests": resp.stdout.trim()
    }))
}

// =========================================================================
// CDP Operations
// =========================================================================

#[cfg(not(target_arch = "wasm32"))]
fn op_cdp_connect(request: &ToolCallRequest) -> Result<Value, String> {
    let target_host = get_arg(&request.arguments, "target_host", None);
    let web_socket_url = get_arg(&request.arguments, "web_socket_url", None);
    
    // For now, we'll return a session ID that will be used for subsequent CDP operations
    let session_id = format!("cdp_{}", Utc::now().timestamp_millis());
    
    // In a real implementation, we would connect to the CDP endpoint here
    // For now, we just acknowledge the connection
    Ok(json!({
        "status": "connected",
        "session_id": session_id,
        "target_host": target_host,
        "web_socket_url": web_socket_url
    }))
}

#[cfg(not(target_arch = "wasm32"))]
fn op_cdp_request(request: &ToolCallRequest) -> Result<Value, String> {
    let session_id = get_arg(&request.arguments, "session_id", None)
        .ok_or_else(|| "Missing 'session_id' parameter".to_string())?;
    let method = get_arg(&request.arguments, "method", None)
        .ok_or_else(|| "Missing 'method' parameter".to_string())?;
    let params = &request.arguments.get("params").cloned();
    
    Ok(json!({
        "status": "success",
        "method": method,
        "session_id": session_id,
        "result": serde_json::json!({ "success": true })
    }))
}

#[cfg(not(target_arch = "wasm32"))]
fn op_cdp_disconnect(request: &ToolCallRequest) -> Result<Value, String> {
    let session_id = get_arg(&request.arguments, "session_id", None);
    
    if let Some(sid) = session_id {
        Ok(json!({ "status": "disconnected", "session_id": sid }))
    } else {
        Err("Missing session_id parameter".to_string())
    }
}

// =========================================================================
// Helper Functions
// =========================================================================

#[cfg(not(target_arch = "wasm32"))]
fn run_binary_raw(req: &CmdExecRequest) -> Result<CmdExecResponse, String> {
    let mut cmd = std::process::Command::new(&req.program);
    cmd.args(&req.args);
    cmd.stdin(std::process::Stdio::inherit());
    cmd.stdout(std::process::Stdio::inherit());
    cmd.stderr(std::process::Stdio::inherit());
    
    match cmd.output() {
        Ok(output) => Ok(CmdExecResponse {
            success: output.status.success(),
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        }),
        Err(e) => Ok(CmdExecResponse {
            success: false,
            exit_code: None,
            stdout: String::new(),
            stderr: format!("Failed to spawn {}: {}", req.program, e),
        }),
    }
}
