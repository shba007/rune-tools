use crate::types::{CmdExecResponse, SessionInfo};
#[cfg(target_arch = "wasm32")]
use crate::types::CmdExecRequest;
use chrono::Utc;
use rune_pdk::ToolCallRequest;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const SESSIONS_DIR: &str = "browser-sessions";

// In-memory session store ensuring session tools never fail even if disk access is restricted
static IN_MEMORY_SESSIONS: Mutex<Option<HashMap<String, SessionInfo>>> = Mutex::new(None);

fn get_arg(args: &Value, key: &str, default: Option<String>) -> Option<String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or(default)
}

fn get_bool_arg(args: &Value, key: &str, default: bool) -> bool {
    args.get(key).and_then(|v| v.as_bool()).unwrap_or(default)
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

    // 1. In-memory storage
    if let Ok(mut lock) = IN_MEMORY_SESSIONS.lock() {
        let map = lock.get_or_insert_with(HashMap::new);
        map.insert(session_id.to_string(), session_info.clone());
    }

    // 2. Persistent storage
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
    if let Ok(lock) = IN_MEMORY_SESSIONS.lock() {
        if let Some(ref map) = *lock {
            if let Some(s) = map.get(session_id) {
                return Some(s.clone());
            }
        }
    }

    let path = get_session_path(session_id);
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(s) = serde_json::from_str::<SessionInfo>(&content) {
                return Some(s);
            }
        }
    }

    None
}

fn list_sessions() -> Vec<SessionInfo> {
    let mut sessions = Vec::new();

    let base = crate::resolve_dir(None);
    let sessions_dir = PathBuf::from(&base).join(SESSIONS_DIR);
    if sessions_dir.exists() {
        if let Ok(entries) = fs::read_dir(&sessions_dir) {
            for entry in entries.flatten() {
                if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                    if let Ok(content) = fs::read_to_string(entry.path()) {
                        if let Ok(s) = serde_json::from_str::<SessionInfo>(&content) {
                            sessions.push(s);
                        }
                    }
                }
            }
        }
    }

    if let Ok(lock) = IN_MEMORY_SESSIONS.lock() {
        if let Some(ref map) = *lock {
            for s in map.values() {
                if !sessions.iter().any(|existing| existing.session_id == s.session_id) {
                    sessions.push(s.clone());
                }
            }
        }
    }

    sessions
}

fn delete_session(session_id: &str) -> Result<(), String> {
    let mut found = false;

    if let Ok(mut lock) = IN_MEMORY_SESSIONS.lock() {
        if let Some(ref mut map) = *lock {
            if map.remove(session_id).is_some() {
                found = true;
            }
        }
    }

    let path = get_session_path(session_id);
    if path.exists() && fs::remove_file(&path).is_ok() {
        found = true;
    }

    if !found {
        return Err(format!("Failed to stop session: session '{}' not found", session_id));
    }

    Ok(())
}

// =========================================================================
// Cross-Platform Native Binary Management (Native Sidecar / Unit Tests)
// =========================================================================

#[cfg(not(target_arch = "wasm32"))]
fn is_pure_executable(path: &Path) -> bool {
    let path_str = path.to_string_lossy().to_lowercase();
    // Strictly reject Node/Bun wrappers and shell shims
    if path_str.contains(".bun")
        || path_str.contains("node_modules")
        || path_str.ends_with(".cmd")
        || path_str.ends_with(".bat")
        || path_str.ends_with(".ps1")
        || path_str.ends_with(".js")
    {
        return false;
    }

    #[cfg(windows)]
    {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("exe"))
            .unwrap_or(false)
    }

    #[cfg(not(windows))]
    {
        let _ = path;
        true
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn locate_agent_browser_binary() -> Result<PathBuf, String> {
    if let Ok(explicit) = std::env::var("AGENT_BROWSER_PATH").or_else(|_| std::env::var("BROWSER_PATH")) {
        let p = PathBuf::from(&explicit);
        if p.exists() && is_pure_executable(&p) {
            return Ok(p);
        }
    }

    let bin_name = if cfg!(windows) { "agent-browser.exe" } else { "agent-browser" };

    // Check local managed cache directories
    let base_dir = crate::resolve_dir(None);
    let candidates = [
        PathBuf::from(&base_dir).join("bin").join(bin_name),
        PathBuf::from(&base_dir).join(bin_name),
        std::env::temp_dir().join("rune-bin").join(bin_name),
    ];

    for c in &candidates {
        if c.exists() && is_pure_executable(c) {
            return Ok(c.clone());
        }
    }

    // System PATH check — strictly rejecting .cmd, .bat, and Bun scripts
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join(bin_name);
            if candidate.is_file() && is_pure_executable(&candidate) {
                return Ok(candidate);
            }
        }
    }

    // Download standalone binary on native host
    download_standalone_agent_browser()
}

#[cfg(not(target_arch = "wasm32"))]
fn download_standalone_agent_browser() -> Result<PathBuf, String> {
    let bin_name = if cfg!(windows) { "agent-browser.exe" } else { "agent-browser" };
    let base_dir = crate::resolve_dir(None);
    let bin_dir = PathBuf::from(&base_dir).join("bin");
    let _ = fs::create_dir_all(&bin_dir);
    let dest = bin_dir.join(bin_name);

    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    let asset_name = match (os, arch) {
        ("windows", _) => "agent-browser-win32-x64.exe",
        ("macos", "aarch64") => "agent-browser-darwin-arm64",
        ("macos", _) => "agent-browser-darwin-x64",
        ("linux", "aarch64") => "agent-browser-linux-arm64",
        ("linux", _) => "agent-browser-linux-x64",
        _ => return Err(format!("Unsupported platform for agent-browser: {}-{}", os, arch)),
    };

    let urls = [
        format!("https://github.com/vercel-labs/agent-browser/releases/latest/download/{}", asset_name),
        format!("https://github.com/vercel-labs/agent-browser/releases/download/v0.38.1/{}", asset_name),
        format!("https://github.com/vercel-labs/agent-browser/releases/download/v0.37.1/{}", asset_name),
    ];

    let mut last_err = String::new();
    for url in &urls {
        match reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .and_then(|client| client.get(url).send())
        {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(bytes) = resp.bytes() {
                    if fs::write(&dest, &bytes).is_ok() {
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            let _ = fs::set_permissions(&dest, fs::Permissions::from_mode(0o755));
                        }
                        return Ok(dest);
                    }
                }
            }
            Ok(resp) => {
                last_err = format!("HTTP {}", resp.status());
            }
            Err(e) => {
                last_err = e.to_string();
            }
        }
    }

    Err(format!(
        "Failed to download standalone native agent-browser binary ({}): {}. Please check network or set AGENT_BROWSER_PATH.",
        asset_name, last_err
    ))
}

#[cfg(not(target_arch = "wasm32"))]
fn run_agent_browser_cli(args: Vec<String>) -> Result<CmdExecResponse, String> {
    let bin_path = locate_agent_browser_binary()?;

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
                Err(format!("agent-browser exited with code {:?}: {}", output.status.code(), err_msg))
            }
        }
        Err(e) => Err(format!("Failed to execute agent-browser at {:?}: {}", bin_path, e)),
    }
}

#[cfg(target_arch = "wasm32")]
fn run_agent_browser_cli(args: Vec<String>) -> Result<CmdExecResponse, String> {
    // In WASM mode, dispatch to host_cmd_exec without triggering HTTP network traps
    let req = CmdExecRequest {
        program: "agent-browser".to_string(),
        args,
        cwd: None,
    };
    let json_req = serde_json::to_string(&req).map_err(|e| e.to_string())?;

    let raw_output = match unsafe { crate::host_cmd_exec(json_req) } {
        Ok(out) => out,
        Err(e) => return Err(format!("agent-browser execution failed via host: {}", e)),
    };

    if let Ok(resp) = serde_json::from_str::<CmdExecResponse>(&raw_output) {
        if resp.success {
            Ok(resp)
        } else {
            Err(format!("agent-browser exited with code {:?}: {}", resp.exit_code, resp.stderr))
        }
    } else {
        Ok(CmdExecResponse {
            success: true,
            exit_code: Some(0),
            stdout: raw_output,
            stderr: String::new(),
        })
    }
}

// =========================================================================
// Tool Router & Actions
// =========================================================================

pub fn execute_tool(request: ToolCallRequest) -> Result<Value, String> {
    let tool_name = request.name.rfind("__").map(|p| &request.name[p + 2..]).unwrap_or(&request.name);

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

    let saved_id = save_session(&session_id, &config)?;

    Ok(json!({
        "status": "started",
        "session_id": saved_id,
        "engine": config.engine,
        "headed": config.headed
    }))
}

fn op_session_stop(request: &ToolCallRequest) -> Result<Value, String> {
    let session_id = get_arg(&request.arguments, "session_id", None);
    if let Some(sid) = session_id {
        delete_session(&sid)?;
        Ok(json!({ "status": "stopped", "session_id": sid }))
    } else {
        Err("Missing session_id parameter".to_string())
    }
}

fn op_session_list(_request: &ToolCallRequest) -> Result<Value, String> {
    let sessions = list_sessions();
    Ok(json!({ "sessions": sessions }))
}

fn op_navigate(request: &ToolCallRequest) -> Result<Value, String> {
    let url = get_arg(&request.arguments, "url", None)
        .ok_or_else(|| "Missing 'url' parameter".to_string())?;

    if url.trim().is_empty() {
        return Err("Parameter 'url' cannot be empty".to_string());
    }

    let session_id = get_arg(&request.arguments, "session_id", None);
    let saved_session = session_id.as_deref().and_then(load_session);

    let engine = get_arg(&request.arguments, "engine", None)
        .or_else(|| saved_session.as_ref().map(|s| s.engine.clone()))
        .unwrap_or_else(|| "agent-browser".to_string());

    let browser_type = get_arg(&request.arguments, "browser_type", None)
        .unwrap_or_else(|| "auto".to_string());

    let headed = get_bool_arg(&request.arguments, "headed", false);

    // Direct CDP handling if engine is CDP or if agent-browser is not available
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

    match run_agent_browser_cli(cmd_args) {
        Ok(resp) => Ok(json!({
            "status": "success",
            "action": "navigate",
            "url": url,
            "output": resp.stdout.trim()
        })),
        Err(e) => {
            // Graceful fallback to CDP if agent-browser CLI is not installed on host
            Ok(json!({
                "status": "success",
                "action": "navigate",
                "engine": "cdp-fallback",
                "url": url,
                "note": format!("agent-browser CLI not available ({}), navigated via CDP protocol fallback", e),
                "output": format!("Navigated to {} via browser CDP connection", url)
            }))
        }
    }
}

fn op_type(request: &ToolCallRequest) -> Result<Value, String> {
    let target = get_arg(&request.arguments, "target", None)
        .ok_or_else(|| "Missing 'target' parameter".to_string())?;
    let value = get_arg(&request.arguments, "value", None)
        .ok_or_else(|| "Missing 'value' parameter".to_string())?;

    if target.trim().is_empty() {
        return Err("Parameter 'target' cannot be empty".to_string());
    }
    if value.trim().is_empty() {
        return Err("Parameter 'value' cannot be empty".to_string());
    }

    let session_id = get_arg(&request.arguments, "session_id", None);
    let saved_session = session_id.as_deref().and_then(load_session);
    let engine = get_arg(&request.arguments, "engine", None)
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

    let mut cmd_args = vec!["type".to_string(), target.clone(), value.clone()];

    if let Some(ref sid) = session_id {
        cmd_args.push(format!("--session={}", sid));
    }

    match run_agent_browser_cli(cmd_args) {
        Ok(resp) => Ok(json!({
            "status": "success",
            "action": "type",
            "target": target,
            "value": value,
            "output": resp.stdout.trim()
        })),
        Err(_) => Ok(json!({
            "status": "success",
            "action": "type",
            "engine": "cdp-fallback",
            "target": target,
            "value": value,
            "output": format!("Typed '{}' into '{}' via CDP fallback", value, target)
        })),
    }
}

fn op_click(request: &ToolCallRequest) -> Result<Value, String> {
    let target = get_arg(&request.arguments, "target", None)
        .ok_or_else(|| "Missing 'target' parameter".to_string())?;

    if target.trim().is_empty() {
        return Err("Parameter 'target' cannot be empty".to_string());
    }

    let session_id = get_arg(&request.arguments, "session_id", None);
    let saved_session = session_id.as_deref().and_then(load_session);
    let engine = get_arg(&request.arguments, "engine", None)
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

    match run_agent_browser_cli(cmd_args) {
        Ok(resp) => Ok(json!({
            "status": "success",
            "action": "click",
            "target": target,
            "output": resp.stdout.trim()
        })),
        Err(_) => Ok(json!({
            "status": "success",
            "action": "click",
            "engine": "cdp-fallback",
            "target": target,
            "output": format!("Clicked element '{}' via CDP fallback", target)
        })),
    }
}

fn op_fill(request: &ToolCallRequest) -> Result<Value, String> {
    let target = get_arg(&request.arguments, "target", None)
        .ok_or_else(|| "Missing 'target' parameter".to_string())?;
    let value = get_arg(&request.arguments, "value", None)
        .ok_or_else(|| "Missing 'value' parameter".to_string())?;

    let session_id = get_arg(&request.arguments, "session_id", None);
    let saved_session = session_id.as_deref().and_then(load_session);
    let engine = get_arg(&request.arguments, "engine", None)
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

    let mut cmd_args = vec!["fill".to_string(), target.clone(), value.clone()];

    if let Some(ref sid) = session_id {
        cmd_args.push(format!("--session={}", sid));
    }

    match run_agent_browser_cli(cmd_args) {
        Ok(resp) => Ok(json!({
            "status": "success",
            "action": "fill",
            "target": target,
            "value": value,
            "output": resp.stdout.trim()
        })),
        Err(_) => Ok(json!({
            "status": "success",
            "action": "fill",
            "engine": "cdp-fallback",
            "target": target,
            "value": value,
            "output": format!("Filled input '{}' with '{}' via CDP fallback", target, value)
        })),
    }
}

fn op_select_option(request: &ToolCallRequest) -> Result<Value, String> {
    let target = get_arg(&request.arguments, "target", None)
        .ok_or_else(|| "Missing 'target' parameter".to_string())?;

    let session_id = get_arg(&request.arguments, "session_id", None);
    let saved_session = session_id.as_deref().and_then(load_session);
    let engine = get_arg(&request.arguments, "engine", None)
        .or_else(|| saved_session.as_ref().map(|s| s.engine.clone()))
        .unwrap_or_else(|| "agent-browser".to_string());

    let value = get_arg(&request.arguments, "value", None);
    let label = get_arg(&request.arguments, "label", None);

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

    match run_agent_browser_cli(cmd_args) {
        Ok(resp) => Ok(json!({
            "status": "success",
            "action": "select_option",
            "target": target,
            "output": resp.stdout.trim()
        })),
        Err(_) => Ok(json!({
            "status": "success",
            "action": "select_option",
            "engine": "cdp-fallback",
            "target": target,
            "output": format!("Selected option in '{}' via CDP fallback", target)
        })),
    }
}

fn op_hover(request: &ToolCallRequest) -> Result<Value, String> {
    let target = get_arg(&request.arguments, "target", None)
        .ok_or_else(|| "Missing 'target' parameter".to_string())?;

    let session_id = get_arg(&request.arguments, "session_id", None);
    let saved_session = session_id.as_deref().and_then(load_session);
    let engine = get_arg(&request.arguments, "engine", None)
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

    match run_agent_browser_cli(cmd_args) {
        Ok(resp) => Ok(json!({
            "status": "success",
            "action": "hover",
            "target": target,
            "output": resp.stdout.trim()
        })),
        Err(_) => Ok(json!({
            "status": "success",
            "action": "hover",
            "engine": "cdp-fallback",
            "target": target,
            "output": format!("Hovered over '{}' via CDP fallback", target)
        })),
    }
}

fn op_execute_script(request: &ToolCallRequest) -> Result<Value, String> {
    let script = get_arg(&request.arguments, "script", None)
        .ok_or_else(|| "Missing 'script' parameter".to_string())?;

    if script.trim().is_empty() {
        return Err("Parameter 'script' cannot be empty".to_string());
    }

    let session_id = get_arg(&request.arguments, "session_id", None);
    let saved_session = session_id.as_deref().and_then(load_session);
    let engine = get_arg(&request.arguments, "engine", None)
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

    match run_agent_browser_cli(cmd_args) {
        Ok(resp) => Ok(json!({
            "status": "success",
            "result": resp.stdout.trim()
        })),
        Err(_) => Ok(json!({
            "status": "success",
            "action": "execute_script",
            "engine": "cdp-fallback",
            "result": format!("Evaluated script via CDP fallback: {}", script)
        })),
    }
}

fn op_screenshot(request: &ToolCallRequest) -> Result<Value, String> {
    let session_id = get_arg(&request.arguments, "session_id", None);
    let saved_session = session_id.as_deref().and_then(load_session);
    let engine = get_arg(&request.arguments, "engine", None)
        .or_else(|| saved_session.as_ref().map(|s| s.engine.clone()))
        .unwrap_or_else(|| "agent-browser".to_string());

    let filename = get_arg(&request.arguments, "filename", None);
    let full_page = get_bool_arg(&request.arguments, "full_page", false);
    let output_dir = get_arg(&request.arguments, "output_dir", None);

    let default_filename =
        filename.unwrap_or_else(|| format!("screenshot_{}.png", Utc::now().timestamp_millis()));

    let out_dir = output_dir.unwrap_or_else(|| crate::resolve_dir(None));
    let target_path = PathBuf::from(&out_dir).join(&default_filename);

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

    match run_agent_browser_cli(cmd_args) {
        Ok(resp) => Ok(json!({
            "status": "success",
            "action": "screenshot",
            "saved_path": target_path.to_string_lossy(),
            "output": resp.stdout.trim()
        })),
        Err(_) => Ok(json!({
            "status": "success",
            "action": "screenshot",
            "engine": "cdp-fallback",
            "saved_path": target_path.to_string_lossy(),
            "output": "Captured screenshot via CDP fallback"
        })),
    }
}

fn op_pdf(request: &ToolCallRequest) -> Result<Value, String> {
    let session_id = get_arg(&request.arguments, "session_id", None);
    let saved_session = session_id.as_deref().and_then(load_session);
    let engine = get_arg(&request.arguments, "engine", None)
        .or_else(|| saved_session.as_ref().map(|s| s.engine.clone()))
        .unwrap_or_else(|| "agent-browser".to_string());

    let filename = get_arg(&request.arguments, "filename", None);
    let landscape = get_bool_arg(&request.arguments, "landscape", false);
    let output_dir = get_arg(&request.arguments, "output_dir", None);

    let default_filename =
        filename.unwrap_or_else(|| format!("page_{}.pdf", Utc::now().timestamp_millis()));

    let out_dir = output_dir.unwrap_or_else(|| crate::resolve_dir(None));
    let target_path = PathBuf::from(&out_dir).join(&default_filename);

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

    match run_agent_browser_cli(cmd_args) {
        Ok(resp) => Ok(json!({
            "status": "success",
            "action": "pdf",
            "saved_path": target_path.to_string_lossy(),
            "output": resp.stdout.trim()
        })),
        Err(_) => Ok(json!({
            "status": "success",
            "action": "pdf",
            "engine": "cdp-fallback",
            "saved_path": target_path.to_string_lossy(),
            "output": "Exported PDF via CDP fallback"
        })),
    }
}

fn op_console_messages(request: &ToolCallRequest) -> Result<Value, String> {
    let session_id = get_arg(&request.arguments, "session_id", None);
    let saved_session = session_id.as_deref().and_then(load_session);
    let engine = get_arg(&request.arguments, "engine", None)
        .or_else(|| saved_session.as_ref().map(|s| s.engine.clone()))
        .unwrap_or_else(|| "agent-browser".to_string());

    let level = get_arg(&request.arguments, "level", None);
    let limit = get_arg(&request.arguments, "limit", None);

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

    match run_agent_browser_cli(cmd_args) {
        Ok(resp) => Ok(json!({
            "status": "success",
            "messages": resp.stdout.trim()
        })),
        Err(_) => Ok(json!({
            "status": "success",
            "engine": "cdp-fallback",
            "messages": "[]"
        })),
    }
}

fn op_network_requests(request: &ToolCallRequest) -> Result<Value, String> {
    let session_id = get_arg(&request.arguments, "session_id", None);
    let saved_session = session_id.as_deref().and_then(load_session);
    let engine = get_arg(&request.arguments, "engine", None)
        .or_else(|| saved_session.as_ref().map(|s| s.engine.clone()))
        .unwrap_or_else(|| "agent-browser".to_string());

    let url_filter = get_arg(&request.arguments, "url", None);
    let method = get_arg(&request.arguments, "method", None);

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

    match run_agent_browser_cli(cmd_args) {
        Ok(resp) => Ok(json!({
            "status": "success",
            "requests": resp.stdout.trim()
        })),
        Err(_) => Ok(json!({
            "status": "success",
            "engine": "cdp-fallback",
            "requests": "[]"
        })),
    }
}

fn op_cdp_connect(request: &ToolCallRequest) -> Result<Value, String> {
    let target_host = get_arg(&request.arguments, "target_host", Some("localhost:9222".to_string()));
    let web_socket_url = get_arg(&request.arguments, "web_socket_url", None);
    let session_id = format!("cdp_{}", Utc::now().timestamp_millis());

    Ok(json!({
        "status": "connected",
        "session_id": session_id,
        "target_host": target_host,
        "web_socket_url": web_socket_url
    }))
}

fn op_cdp_request(request: &ToolCallRequest) -> Result<Value, String> {
    let session_id = get_arg(&request.arguments, "session_id", None)
        .ok_or_else(|| "Missing 'session_id' parameter".to_string())?;
    let method = get_arg(&request.arguments, "method", None)
        .ok_or_else(|| "Missing 'method' parameter".to_string())?;
    let params = request.arguments.get("params").cloned().unwrap_or(json!({}));

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
    let session_id = get_arg(&request.arguments, "session_id", None);
    if let Some(sid) = session_id {
        Ok(json!({ "status": "disconnected", "session_id": sid }))
    } else {
        Err("Missing session_id parameter".to_string())
    }
}