use rune_browser::{operations::execute_tool, resolve_dir};
use rune_pdk::ToolCallRequest;
use serde_json::json;

#[test]
fn test_resolve_dir_custom_path() {
    let resolved = resolve_dir(Some(r"../code-kit/test-dir/browser"));
    assert_eq!(resolved, r"../code-kit/test-dir/browser");
}

#[test]
fn test_empty_required_parameters() {
    let req_empty_url = ToolCallRequest {
        name: "browser_navigate".to_string(),
        arguments: json!({ "url": "   " }),
    };
    let res = execute_tool(req_empty_url);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Parameter 'url' cannot be empty"));

    let req_empty_click = ToolCallRequest {
        name: "browser_click".to_string(),
        arguments: json!({ "target": "" }),
    };
    let res = execute_tool(req_empty_click);
    assert!(res.is_err());
    assert!(res
        .unwrap_err()
        .contains("Parameter 'target' cannot be empty"));

    let req_empty_script = ToolCallRequest {
        name: "browser_execute_script".to_string(),
        arguments: json!({ "script": "  " }),
    };
    let res = execute_tool(req_empty_script);
    assert!(res.is_err());
    assert!(res
        .unwrap_err()
        .contains("Parameter 'script' cannot be empty"));
}

#[test]
fn test_unknown_tool_routing() {
    let req = ToolCallRequest {
        name: "non_existent_tool".to_string(),
        arguments: json!({}),
    };
    let res = execute_tool(req);
    assert!(res.is_err());
    assert_eq!(res.unwrap_err(), "Unknown tool: non_existent_tool");
}

#[test]
fn test_session_start() {
    let req = ToolCallRequest {
        name: "browser_session_start".to_string(),
        arguments: json!({
            "engine": "agent-browser",
            "browser_type": "chrome",
            "headed": false,
            "output_dir": "./test-output"
        }),
    };
    let res = execute_tool(req);
    assert!(res.is_ok());
    let result = res.unwrap();
    assert_eq!(result["status"], "started");
    assert!(result["session_id"].is_string());
}

#[test]
fn test_session_list() {
    let req = ToolCallRequest {
        name: "browser_session_list".to_string(),
        arguments: json!({}),
    };
    let res = execute_tool(req);
    assert!(res.is_ok());
    let result = res.unwrap();
    assert!(result["sessions"].is_array());
}

#[test]
fn test_session_stop() {
    // First start a session
    let start_req = ToolCallRequest {
        name: "browser_session_start".to_string(),
        arguments: json!({ "engine": "agent-browser" }),
    };
    let start_res = execute_tool(start_req).unwrap();
    let session_id = start_res["session_id"].as_str().unwrap().to_string();
    
    // Then stop it
    let stop_req = ToolCallRequest {
        name: "browser_session_stop".to_string(),
        arguments: json!({ "session_id": session_id.clone() }),
    };
    let stop_res = execute_tool(stop_req);
    assert!(stop_res.is_ok());
    let result = stop_res.unwrap();
    assert_eq!(result["status"], "stopped");
    assert_eq!(result["session_id"], session_id);
}

#[test]
fn test_navigate_empty_url() {
    let req = ToolCallRequest {
        name: "browser_navigate".to_string(),
        arguments: json!({ "url": "" }),
    };
    let res = execute_tool(req);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("cannot be empty"));
}

#[test]
fn test_navigate_valid() {
    let req = ToolCallRequest {
        name: "browser_navigate".to_string(),
        arguments: json!({
            "url": "https://example.com",
            "session_id": "test_session",
            "engine": "agent-browser",
            "headed": false
        }),
    };
    let res = execute_tool(req);
    // Will fail if agent-browser is not installed, but should not fail due to invalid params
    assert!(res.is_ok() || res.unwrap_err().contains("agent-browser"));
}

#[test]
fn test_type_valid() {
    let req = ToolCallRequest {
        name: "browser_type".to_string(),
        arguments: json!({
            "target": "#search-input",
            "value": "test text",
            "session_id": "test_session"
        }),
    };
    let res = execute_tool(req);
    assert!(res.is_ok() || res.unwrap_err().contains("agent-browser"));
}

#[test]
fn test_click_valid() {
    let req = ToolCallRequest {
        name: "browser_click".to_string(),
        arguments: json!({
            "target": "#submit-btn",
            "session_id": "test_session",
            "double_click": false,
            "new_tab": false
        }),
    };
    let res = execute_tool(req);
    assert!(res.is_ok() || res.unwrap_err().contains("agent-browser"));
}

#[test]
fn test_fill_valid() {
    let req = ToolCallRequest {
        name: "browser_fill".to_string(),
        arguments: json!({
            "target": "#email",
            "value": "user@example.com",
            "session_id": "test_session",
            "press_enter": false
        }),
    };
    let res = execute_tool(req);
    assert!(res.is_ok() || res.unwrap_err().contains("agent-browser"));
}

#[test]
fn test_select_option_valid() {
    let req = ToolCallRequest {
        name: "browser_select_option".to_string(),
        arguments: json!({
            "target": "#country",
            "value": "us",
            "session_id": "test_session"
        }),
    };
    let res = execute_tool(req);
    assert!(res.is_ok() || res.unwrap_err().contains("agent-browser"));
}

#[test]
fn test_hover_valid() {
    let req = ToolCallRequest {
        name: "browser_hover".to_string(),
        arguments: json!({
            "target": "#hover-target",
            "session_id": "test_session"
        }),
    };
    let res = execute_tool(req);
    assert!(res.is_ok() || res.unwrap_err().contains("agent-browser"));
}

#[test]
fn test_execute_script_valid() {
    let req = ToolCallRequest {
        name: "browser_execute_script".to_string(),
        arguments: json!({
            "script": "document.title",
            "session_id": "test_session"
        }),
    };
    let res = execute_tool(req);
    assert!(res.is_ok() || res.unwrap_err().contains("agent-browser"));
}

#[test]
fn test_screenshot_valid() {
    let req = ToolCallRequest {
        name: "browser_screenshot".to_string(),
        arguments: json!({
            "filename": "test.png",
            "session_id": "test_session",
            "full_page": false,
            "output_dir": "./test-output"
        }),
    };
    let res = execute_tool(req);
    assert!(res.is_ok() || res.unwrap_err().contains("agent-browser"));
}

#[test]
fn test_pdf_valid() {
    let req = ToolCallRequest {
        name: "browser_pdf".to_string(),
        arguments: json!({
            "filename": "test.pdf",
            "session_id": "test_session",
            "landscape": false,
            "output_dir": "./test-output"
        }),
    };
    let res = execute_tool(req);
    assert!(res.is_ok() || res.unwrap_err().contains("agent-browser"));
}

#[test]
fn test_console_messages_valid() {
    let req = ToolCallRequest {
        name: "browser_console_messages".to_string(),
        arguments: json!({
            "session_id": "test_session",
            "level": "info",
            "limit": 10
        }),
    };
    let res = execute_tool(req);
    assert!(res.is_ok() || res.unwrap_err().contains("agent-browser"));
}

#[test]
fn test_network_requests_valid() {
    let req = ToolCallRequest {
        name: "browser_network_requests".to_string(),
        arguments: json!({
            "session_id": "test_session",
            "url": "example.com",
            "method": "GET"
        }),
    };
    let res = execute_tool(req);
    assert!(res.is_ok() || res.unwrap_err().contains("agent-browser"));
}

#[test]
fn test_cdp_connect_valid() {
    let req = ToolCallRequest {
        name: "browser_cdp_connect".to_string(),
        arguments: json!({
            "target_host": "localhost:9222",
            "web_socket_url": "ws://localhost:9222/devtools"
        }),
    };
    let res = execute_tool(req);
    assert!(res.is_ok());
    let result = res.unwrap();
    assert_eq!(result["status"], "connected");
}

#[test]
fn test_cdp_request_valid() {
    let req = ToolCallRequest {
        name: "browser_cdp_request".to_string(),
        arguments: json!({
            "session_id": "cdp_test",
            "method": "Page.enable",
            "params": {}
        }),
    };
    let res = execute_tool(req);
    assert!(res.is_ok());
    let result = res.unwrap();
    assert_eq!(result["status"], "success");
}

#[test]
fn test_cdp_disconnect_valid() {
    let req = ToolCallRequest {
        name: "browser_cdp_disconnect".to_string(),
        arguments: json!({ "session_id": "cdp_test" }),
    };
    let res = execute_tool(req);
    assert!(res.is_ok());
    let result = res.unwrap();
    assert_eq!(result["status"], "disconnected");
}
