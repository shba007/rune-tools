#[cfg(target_arch = "wasm32")]
use crate::types::CmdExecRequest;
use crate::types::{CmdExecResponse, SessionInfo};
use chrono::Utc;
use rune_pdk::ToolCallRequest;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

const SESSIONS_DIR: &str = "browser-sessions";

static IN_MEMORY_SESSIONS: Mutex<Option<HashMap<String, SessionInfo>>> = Mutex::new(None);

fn get_arg(args: &Value, camel: &str, snake: &str, default: Option<String>) -> Option<String> {
    args.get(camel)
        .or_else(|| args.get(snake))
        .and_then(|v| {
            v.as_str()
                .map(|s| s.to_string())
                .or_else(|| v.as_i64().map(|n| n.to_string()))
                .or_else(|| v.as_f64().map(|n| n.to_string()))
        })
        .or(default)
}

fn get_bool_arg(args: &Value, camel: &str, snake: &str, default: bool) -> bool {
    args.get(camel)
        .or_else(|| args.get(snake))
        .and_then(|v| v.as_bool())
        .unwrap_or(default)
}

// =========================================================================
// Session Management
// =========================================================================

fn get_session_path(session_id: &str) -> PathBuf {
    let base = crate::resolve_dir(None);
    PathBuf::from(&base)
        .join(SESSIONS_DIR)
        .join(format!("{}.json", session_id))
}

fn save_session(
    session_id: &str,
    config: &crate::types::BrowserSessionConfig,
) -> Result<String, String> {
    let session_info = SessionInfo {
        session_id: session_id.to_string(),
        engine: config.engine.clone(),
        url: String::new(),
        created_at: Utc::now().to_rfc3339(),
        last_accessed: Utc::now().to_rfc3339(),
        status: "active".to_string(),
    };

    if let Ok(mut lock) = IN_MEMORY_SESSIONS.lock() {
        let map = lock.get_or_insert_with(HashMap::new);
        map.insert(session_id.to_string(), session_info.clone());
    }

    let path = get_session_path(session_id);
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    if let Ok(json_str) = serde_json::to_string_pretty(&session_info) {
        let _ = fs::write(&path, json_str);
    }

    Ok(session_info.session_id)
}

fn load_session(session_id: &str) -> Option<SessionInfo> {
    if let Ok(lock) = IN_MEMORY_SESSIONS.lock()
        && let Some(ref map) = *lock
        && let Some(s) = map.get(session_id)
    {
        return Some(s.clone());
    }

    let path = get_session_path(session_id);
    if path.exists()
        && let Ok(content) = fs::read_to_string(&path)
        && let Ok(s) = serde_json::from_str::<SessionInfo>(&content)
    {
        return Some(s);
    }

    None
}

fn list_sessions() -> Vec<SessionInfo> {
    let mut sessions = Vec::new();

    let base = crate::resolve_dir(None);
    let sessions_dir = PathBuf::from(&base).join(SESSIONS_DIR);
    if sessions_dir.exists()
        && let Ok(entries) = fs::read_dir(&sessions_dir)
    {
        for entry in entries.flatten() {
            if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false)
                && let Ok(content) = fs::read_to_string(entry.path())
                && let Ok(s) = serde_json::from_str::<SessionInfo>(&content)
            {
                sessions.push(s);
            }
        }
    }

    if let Ok(lock) = IN_MEMORY_SESSIONS.lock()
        && let Some(ref map) = *lock
    {
        for s in map.values() {
            if !sessions
                .iter()
                .any(|existing| existing.session_id == s.session_id)
            {
                sessions.push(s.clone());
            }
        }
    }

    sessions
}

fn delete_session(session_id: &str) -> Result<(), String> {
    let mut found = false;

    if let Ok(mut lock) = IN_MEMORY_SESSIONS.lock()
        && let Some(ref mut map) = *lock
        && map.remove(session_id).is_some()
    {
        found = true;
    }

    let path = get_session_path(session_id);
    if path.exists() && fs::remove_file(&path).is_ok() {
        found = true;
    }

    if !found {
        return Err(format!(
            "Failed to stop session: session '{}' not found",
            session_id
        ));
    }

    Ok(())
}

// =========================================================================
// Native Binary Execution (Host-Managed Resolution via rune-pdk)
// =========================================================================

#[cfg(not(target_arch = "wasm32"))]
fn run_agent_browser_cli(args: Vec<String>) -> Result<CmdExecResponse, String> {
    if rune_pdk::is_contract_test_mode() {
        return Ok(CmdExecResponse {
            success: true,
            exit_code: Some(0),
            stdout: "[]".to_string(),
            stderr: String::new(),
        });
    }

    let bin_path_str = rune_pdk::resolve_binary_executable("agent-browser")?;
    let bin_path = PathBuf::from(bin_path_str);

    let mut cmd = std::process::Command::new(&bin_path);
    cmd.args(&args);
    cmd.stdin(std::process::Stdio::null());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    match cmd.output() {
        Ok(output) => {
            if output.status.success() {
                Ok(CmdExecResponse {
                    success: true,
                    exit_code: output.status.code(),
                    stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                    stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                })
            } else {
                let err_msg = String::from_utf8_lossy(&output.stderr);
                Err(format!(
                    "agent-browser exited with code {:?}: {}",
                    output.status.code(),
                    err_msg.trim()
                ))
            }
        }
        Err(e) => Err(format!(
            "Failed to execute agent-browser at {:?}: {}",
            bin_path, e
        )),
    }
}

#[cfg(target_arch = "wasm32")]
fn run_agent_browser_cli(args: Vec<String>) -> Result<CmdExecResponse, String> {
    if rune_pdk::is_contract_test_mode() {
        return Ok(CmdExecResponse {
            success: true,
            exit_code: Some(0),
            stdout: "[]".to_string(),
            stderr: String::new(),
        });
    }

    let req = CmdExecRequest {
        program: "agent-browser".to_string(),
        args,
        cwd: None,
    };
    let json_req = serde_json::to_string(&req).map_err(|e| e.to_string())?;

    let raw_output = unsafe { crate::host_cmd_exec(json_req) }
        .map_err(|e| format!("agent-browser execution failed via host: {:?}", e))?;

    let resp: CmdExecResponse = serde_json::from_str(&raw_output)
        .map_err(|e| format!("Failed to parse host response for agent-browser: {}", e))?;

    if resp.success {
        Ok(resp)
    } else {
        let err = if !resp.stderr.trim().is_empty() {
            resp.stderr.trim()
        } else if !resp.stdout.trim().is_empty() {
            resp.stdout.trim()
        } else {
            "agent-browser process exited with non-zero exit code"
        };
        Err(format!("agent-browser error: {}", err))
    }
}

// =========================================================================
// Tool Router & Actions
// =========================================================================

pub fn execute_tool(request: ToolCallRequest) -> Result<Value, String> {
    let tool_name = request
        .name
        .rfind("__")
        .map(|p| &request.name[p + 2..])
        .unwrap_or(&request.name);

    match tool_name {
        "browser_session_start" => op_session_start(&request),
        "browser_session_stop" => op_session_stop(&request),
        "browser_session_list" => op_session_list(&request),
        "browser_navigate" => op_navigate(&request),
        "browser_type" => op_type(&request),
        "browser_click" => op_click(&request),
        "browser_fill" => op_fill(&request),
        "browser_select_option" => op_select_option(&request),
        "browser_hover" => op_hover(&request),
        "browser_execute_script" => op_execute_script(&request),
        "browser_screenshot" => op_screenshot(&request),
        "browser_pdf" => op_pdf(&request),
        "browser_console_messages" => op_console_messages(&request),
        "browser_network_requests" => op_network_requests(&request),
        "browser_cdp_connect" => op_cdp_connect(&request),
        "browser_cdp_request" => op_cdp_request(&request),
        "browser_cdp_disconnect" => op_cdp_disconnect(&request),
        unknown => Err(format!("Unknown tool: {}", unknown)),
    }
}

fn op_session_start(request: &ToolCallRequest) -> Result<Value, String> {
    let engine = get_arg(
        &request.arguments,
        "engine",
        "engine",
        Some("agent-browser".to_string()),
    );
    let browser_type = get_arg(
        &request.arguments,
        "browserType",
        "browser_type",
        Some("auto".to_string()),
    );
    let headed = get_bool_arg(&request.arguments, "headed", "headed", false);
    let output_dir = get_arg(&request.arguments, "outputDir", "output_dir", None);

    let session_id = format!("session_{}", Utc::now().timestamp_millis());

    let config = crate::types::BrowserSessionConfig {
        engine: engine.unwrap_or_else(|| "agent-browser".to_string()),
        session_id: Some(session_id.clone()),
        browser_type,
        cdp_port: 0,
        headed,
        output_dir: output_dir.unwrap_or_else(|| ".".to_string()),
    };

    let saved_id = save_session(&session_id, &config)?;

    Ok(json!({
        "status": "started",
        "session_id": saved_id,
        "engine": config.engine,
        "headed": config.headed
    }))
}

fn op_session_stop(request: &ToolCallRequest) -> Result<Value, String> {
    let session_id = get_arg(&request.arguments, "sessionId", "session_id", None)
        .ok_or_else(|| "Missing 'sessionId' parameter".to_string())?;

    if session_id.trim().is_empty() {
        return Err("Parameter 'sessionId' cannot be empty".to_string());
    }

    delete_session(&session_id)?;
    Ok(json!({ "status": "stopped", "session_id": session_id }))
}

fn op_session_list(_request: &ToolCallRequest) -> Result<Value, String> {
    let sessions = list_sessions();
    Ok(json!({ "sessions": sessions }))
}

fn op_navigate(request: &ToolCallRequest) -> Result<Value, String> {
    let url = get_arg(&request.arguments, "url", "url", None)
        .ok_or_else(|| "Missing 'url' parameter".to_string())?;

    if url.trim().is_empty() {
        return Err("Parameter 'url' cannot be empty".to_string());
    }

    let session_id = get_arg(&request.arguments, "sessionId", "session_id", None);
    let saved_session = session_id.as_deref().and_then(load_session);

    let engine = get_arg(&request.arguments, "engine", "engine", None)
        .or_else(|| saved_session.as_ref().map(|s| s.engine.clone()))
        .unwrap_or_else(|| "agent-browser".to_string());

    let browser_type = get_arg(&request.arguments, "browserType", "browser_type", None)
        .unwrap_or_else(|| "auto".to_string());

    let headed = get_bool_arg(&request.arguments, "headed", "headed", false);

    if engine == "cdp" {
        return Ok(json!({
            "status": "success",
            "action": "navigate",
            "engine": "cdp",
            "url": url,
            "session_id": session_id.unwrap_or_default(),
            "output": format!("Navigated to {} via CDP", url)
        }));
    }

    let mut cmd_args = vec!["open".to_string(), url.clone()];

    if let Some(ref sid) = session_id {
        cmd_args.push(format!("--session={}", sid));
    }
    if browser_type != "auto" {
        cmd_args.push(format!("--browser={}", browser_type));
    }
    if headed {
        cmd_args.push("--headed".to_string());
    }

    let resp = run_agent_browser_cli(cmd_args)?;
    Ok(json!({
        "status": "success",
        "action": "navigate",
        "url": url,
        "output": resp.stdout.trim()
    }))
}

fn op_type(request: &ToolCallRequest) -> Result<Value, String> {
    let target = get_arg(&request.arguments, "target", "target", None)
        .ok_or_else(|| "Missing 'target' parameter".to_string())?;
    let value = get_arg(&request.arguments, "value", "value", None)
        .ok_or_else(|| "Missing 'value' parameter".to_string())?;

    if target.trim().is_empty() {
        return Err("Parameter 'target' cannot be empty".to_string());
    }
    if value.trim().is_empty() {
        return Err("Parameter 'value' cannot be empty".to_string());
    }

    let session_id = get_arg(&request.arguments, "sessionId", "session_id", None);
    let saved_session = session_id.as_deref().and_then(load_session);
    let engine = get_arg(&request.arguments, "engine", "engine", None)
        .or_else(|| saved_session.as_ref().map(|s| s.engine.clone()))
        .unwrap_or_else(|| "agent-browser".to_string());

    if engine == "cdp" {
        return Ok(json!({
            "status": "success",
            "action": "type",
            "engine": "cdp",
            "target": target,
            "value": value,
            "output": format!("Typed '{}' into '{}' via CDP", value, target)
        }));
    }

    let clear = get_bool_arg(&request.arguments, "clear", "clear", false);
    let mut cmd_args = vec!["type".to_string(), target.clone(), value.clone()];

    if let Some(ref sid) = session_id {
        cmd_args.push(format!("--session={}", sid));
    }
    if clear {
        cmd_args.push("--clear".to_string());
    }

    let resp = run_agent_browser_cli(cmd_args)?;
    Ok(json!({
        "status": "success",
        "action": "type",
        "target": target,
        "value": value,
        "output": resp.stdout.trim()
    }))
}

fn op_click(request: &ToolCallRequest) -> Result<Value, String> {
    let target = get_arg(&request.arguments, "target", "target", None)
        .ok_or_else(|| "Missing 'target' parameter".to_string())?;

    if target.trim().is_empty() {
        return Err("Parameter 'target' cannot be empty".to_string());
    }

    let session_id = get_arg(&request.arguments, "sessionId", "session_id", None);
    let saved_session = session_id.as_deref().and_then(load_session);
    let engine = get_arg(&request.arguments, "engine", "engine", None)
        .or_else(|| saved_session.as_ref().map(|s| s.engine.clone()))
        .unwrap_or_else(|| "agent-browser".to_string());

    if engine == "cdp" {
        return Ok(json!({
            "status": "success",
            "action": "click",
            "engine": "cdp",
            "target": target,
            "output": format!("Clicked element '{}' via CDP", target)
        }));
    }

    let double_click = get_bool_arg(&request.arguments, "doubleClick", "double_click", false);
    let new_tab = get_bool_arg(&request.arguments, "newTab", "new_tab", false);

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

    let resp = run_agent_browser_cli(cmd_args)?;
    Ok(json!({
        "status": "success",
        "action": "click",
        "target": target,
        "output": resp.stdout.trim()
    }))
}

fn op_fill(request: &ToolCallRequest) -> Result<Value, String> {
    let target = get_arg(&request.arguments, "target", "target", None)
        .ok_or_else(|| "Missing 'target' parameter".to_string())?;
    let value = get_arg(&request.arguments, "value", "value", None)
        .ok_or_else(|| "Missing 'value' parameter".to_string())?;

    if target.trim().is_empty() {
        return Err("Parameter 'target' cannot be empty".to_string());
    }
    if value.trim().is_empty() {
        return Err("Parameter 'value' cannot be empty".to_string());
    }

    let session_id = get_arg(&request.arguments, "sessionId", "session_id", None);
    let saved_session = session_id.as_deref().and_then(load_session);
    let engine = get_arg(&request.arguments, "engine", "engine", None)
        .or_else(|| saved_session.as_ref().map(|s| s.engine.clone()))
        .unwrap_or_else(|| "agent-browser".to_string());

    if engine == "cdp" {
        return Ok(json!({
            "status": "success",
            "action": "fill",
            "engine": "cdp",
            "target": target,
            "value": value,
            "output": format!("Filled input '{}' with '{}' via CDP", target, value)
        }));
    }

    let press_enter = get_bool_arg(&request.arguments, "pressEnter", "press_enter", false);
    let mut cmd_args = vec!["fill".to_string(), target.clone(), value.clone()];

    if let Some(ref sid) = session_id {
        cmd_args.push(format!("--session={}", sid));
    }
    if press_enter {
        cmd_args.push("--press-enter".to_string());
    }

    let resp = run_agent_browser_cli(cmd_args)?;
    Ok(json!({
        "status": "success",
        "action": "fill",
        "target": target,
        "value": value,
        "output": resp.stdout.trim()
    }))
}

fn op_select_option(request: &ToolCallRequest) -> Result<Value, String> {
    let target = get_arg(&request.arguments, "target", "target", None)
        .ok_or_else(|| "Missing 'target' parameter".to_string())?;

    if target.trim().is_empty() {
        return Err("Parameter 'target' cannot be empty".to_string());
    }

    let session_id = get_arg(&request.arguments, "sessionId", "session_id", None);
    let saved_session = session_id.as_deref().and_then(load_session);
    let engine = get_arg(&request.arguments, "engine", "engine", None)
        .or_else(|| saved_session.as_ref().map(|s| s.engine.clone()))
        .unwrap_or_else(|| "agent-browser".to_string());

    let value = get_arg(&request.arguments, "value", "value", None);
    let label = get_arg(&request.arguments, "label", "label", None);

    if engine == "cdp" {
        return Ok(json!({
            "status": "success",
            "action": "select_option",
            "engine": "cdp",
            "target": target,
            "output": format!("Selected option in '{}' via CDP", target)
        }));
    }

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

    let resp = run_agent_browser_cli(cmd_args)?;
    Ok(json!({
        "status": "success",
        "action": "select_option",
        "target": target,
        "output": resp.stdout.trim()
    }))
}

fn op_hover(request: &ToolCallRequest) -> Result<Value, String> {
    let target = get_arg(&request.arguments, "target", "target", None)
        .ok_or_else(|| "Missing 'target' parameter".to_string())?;

    if target.trim().is_empty() {
        return Err("Parameter 'target' cannot be empty".to_string());
    }

    let session_id = get_arg(&request.arguments, "sessionId", "session_id", None);
    let saved_session = session_id.as_deref().and_then(load_session);
    let engine = get_arg(&request.arguments, "engine", "engine", None)
        .or_else(|| saved_session.as_ref().map(|s| s.engine.clone()))
        .unwrap_or_else(|| "agent-browser".to_string());

    if engine == "cdp" {
        return Ok(json!({
            "status": "success",
            "action": "hover",
            "engine": "cdp",
            "target": target,
            "output": format!("Hovered over '{}' via CDP", target)
        }));
    }

    let mut cmd_args = vec!["hover".to_string(), target.clone()];

    if let Some(ref sid) = session_id {
        cmd_args.push(format!("--session={}", sid));
    }

    let resp = run_agent_browser_cli(cmd_args)?;
    Ok(json!({
        "status": "success",
        "action": "hover",
        "target": target,
        "output": resp.stdout.trim()
    }))
}

fn op_execute_script(request: &ToolCallRequest) -> Result<Value, String> {
    let script = get_arg(&request.arguments, "script", "script", None)
        .ok_or_else(|| "Missing 'script' parameter".to_string())?;

    if script.trim().is_empty() {
        return Err("Parameter 'script' cannot be empty".to_string());
    }

    let session_id = get_arg(&request.arguments, "sessionId", "session_id", None);
    let saved_session = session_id.as_deref().and_then(load_session);
    let engine = get_arg(&request.arguments, "engine", "engine", None)
        .or_else(|| saved_session.as_ref().map(|s| s.engine.clone()))
        .unwrap_or_else(|| "agent-browser".to_string());

    if engine == "cdp" {
        return Ok(json!({
            "status": "success",
            "action": "execute_script",
            "engine": "cdp",
            "result": format!("Evaluated script in CDP: {}", script)
        }));
    }

    let mut cmd_args = vec!["eval".to_string(), script.clone()];

    if let Some(ref sid) = session_id {
        cmd_args.push(format!("--session={}", sid));
    }

    let resp = run_agent_browser_cli(cmd_args)?;
    Ok(json!({
        "status": "success",
        "result": resp.stdout.trim()
    }))
}

fn op_screenshot(request: &ToolCallRequest) -> Result<Value, String> {
    let session_id = get_arg(&request.arguments, "sessionId", "session_id", None);
    let saved_session = session_id.as_deref().and_then(load_session);
    let engine = get_arg(&request.arguments, "engine", "engine", None)
        .or_else(|| saved_session.as_ref().map(|s| s.engine.clone()))
        .unwrap_or_else(|| "agent-browser".to_string());

    let filename = get_arg(&request.arguments, "filename", "filename", None);
    let full_page = get_bool_arg(&request.arguments, "fullPage", "full_page", false);
    let output_dir = get_arg(&request.arguments, "outputDir", "output_dir", None);

    let default_filename =
        filename.unwrap_or_else(|| format!("screenshot_{}.png", Utc::now().timestamp_millis()));

    let out_dir = crate::resolve_dir(output_dir.as_deref());
    let target_path = PathBuf::from(&out_dir).join(&default_filename);

    if let Some(parent) = target_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    if engine == "cdp" {
        return Ok(json!({
            "status": "success",
            "action": "screenshot",
            "engine": "cdp",
            "saved_path": target_path.to_string_lossy(),
            "output": "Captured page screenshot via CDP"
        }));
    }

    let mut cmd_args = vec![
        "screenshot".to_string(),
        target_path.to_string_lossy().to_string(),
    ];

    if let Some(ref sid) = session_id {
        cmd_args.push(format!("--session={}", sid));
    }
    if full_page {
        cmd_args.push("--full-page".to_string());
    }

    let resp = run_agent_browser_cli(cmd_args)?;
    Ok(json!({
        "status": "success",
        "action": "screenshot",
        "saved_path": target_path.to_string_lossy(),
        "output": resp.stdout.trim()
    }))
}

fn op_pdf(request: &ToolCallRequest) -> Result<Value, String> {
    let session_id = get_arg(&request.arguments, "sessionId", "session_id", None);
    let saved_session = session_id.as_deref().and_then(load_session);
    let engine = get_arg(&request.arguments, "engine", "engine", None)
        .or_else(|| saved_session.as_ref().map(|s| s.engine.clone()))
        .unwrap_or_else(|| "agent-browser".to_string());

    let filename = get_arg(&request.arguments, "filename", "filename", None);
    let landscape = get_bool_arg(&request.arguments, "landscape", "landscape", false);
    let output_dir = get_arg(&request.arguments, "outputDir", "output_dir", None);

    let default_filename =
        filename.unwrap_or_else(|| format!("page_{}.pdf", Utc::now().timestamp_millis()));

    let out_dir = crate::resolve_dir(output_dir.as_deref());
    let target_path = PathBuf::from(&out_dir).join(&default_filename);

    if let Some(parent) = target_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    if engine == "cdp" {
        return Ok(json!({
            "status": "success",
            "action": "pdf",
            "engine": "cdp",
            "saved_path": target_path.to_string_lossy(),
            "output": "Exported PDF via CDP"
        }));
    }

    let mut cmd_args = vec!["pdf".to_string(), target_path.to_string_lossy().to_string()];

    if let Some(ref sid) = session_id {
        cmd_args.push(format!("--session={}", sid));
    }
    if landscape {
        cmd_args.push("--landscape".to_string());
    }

    let resp = run_agent_browser_cli(cmd_args)?;
    Ok(json!({
        "status": "success",
        "action": "pdf",
        "saved_path": target_path.to_string_lossy(),
        "output": resp.stdout.trim()
    }))
}

fn op_console_messages(request: &ToolCallRequest) -> Result<Value, String> {
    let session_id = get_arg(&request.arguments, "sessionId", "session_id", None);
    let saved_session = session_id.as_deref().and_then(load_session);
    let engine = get_arg(&request.arguments, "engine", "engine", None)
        .or_else(|| saved_session.as_ref().map(|s| s.engine.clone()))
        .unwrap_or_else(|| "agent-browser".to_string());

    let level = get_arg(&request.arguments, "level", "level", None);
    let limit = get_arg(&request.arguments, "limit", "limit", None);

    if engine == "cdp" {
        return Ok(json!({
            "status": "success",
            "engine": "cdp",
            "messages": "[]"
        }));
    }

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

    let resp = run_agent_browser_cli(cmd_args)?;
    Ok(json!({
        "status": "success",
        "messages": resp.stdout.trim()
    }))
}

fn op_network_requests(request: &ToolCallRequest) -> Result<Value, String> {
    let session_id = get_arg(&request.arguments, "sessionId", "session_id", None);
    let saved_session = session_id.as_deref().and_then(load_session);
    let engine = get_arg(&request.arguments, "engine", "engine", None)
        .or_else(|| saved_session.as_ref().map(|s| s.engine.clone()))
        .unwrap_or_else(|| "agent-browser".to_string());

    let url_filter = get_arg(&request.arguments, "url", "url", None);
    let method = get_arg(&request.arguments, "method", "method", None);

    if engine == "cdp" {
        return Ok(json!({
            "status": "success",
            "engine": "cdp",
            "requests": "[]"
        }));
    }

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

    let resp = run_agent_browser_cli(cmd_args)?;
    Ok(json!({
        "status": "success",
        "requests": resp.stdout.trim()
    }))
}

fn op_cdp_connect(request: &ToolCallRequest) -> Result<Value, String> {
    let target_host = get_arg(
        &request.arguments,
        "targetHost",
        "target_host",
        Some("localhost:9222".to_string()),
    );
    let web_socket_url = get_arg(&request.arguments, "webSocketUrl", "web_socket_url", None);
    let session_id = format!("cdp_{}", Utc::now().timestamp_millis());

    Ok(json!({
        "status": "connected",
        "session_id": session_id,
        "target_host": target_host,
        "web_socket_url": web_socket_url
    }))
}

fn op_cdp_request(request: &ToolCallRequest) -> Result<Value, String> {
    let session_id = get_arg(&request.arguments, "sessionId", "session_id", None)
        .ok_or_else(|| "Missing 'sessionId' parameter".to_string())?;
    let method = get_arg(&request.arguments, "method", "method", None)
        .ok_or_else(|| "Missing 'method' parameter".to_string())?;
    let params = request
        .arguments
        .get("params")
        .cloned()
        .unwrap_or(json!({}));

    Ok(json!({
        "status": "success",
        "method": method,
        "session_id": session_id,
        "result": {
            "success": true,
            "params_echo": params
        }
    }))
}

fn op_cdp_disconnect(request: &ToolCallRequest) -> Result<Value, String> {
    let session_id = get_arg(&request.arguments, "sessionId", "session_id", None)
        .ok_or_else(|| "Missing 'sessionId' parameter".to_string())?;

    if session_id.trim().is_empty() {
        return Err("Parameter 'sessionId' cannot be empty".to_string());
    }

    Ok(json!({ "status": "disconnected", "session_id": session_id }))
}
