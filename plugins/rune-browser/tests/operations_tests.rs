use rune_browser::{operations::execute_tool, resolve_dir};
use rune_pdk::ToolCallRequest;
use serde_json::json;

#[test]
fn test_resolve_dir_custom_path() {
    let resolved = resolve_dir(Some(r"../../temp/browser"));
    assert_eq!(resolved, r"../../temp/browser");
}

#[test]
fn test_empty_required_parameters() {
    let req_empty_url = ToolCallRequest {
        name: "browser_navigate".to_string(),
        arguments: json!({ "url": "" }),
    };
    let res = execute_tool(req_empty_url);
    let err = res.unwrap_err();
    assert!(err.contains("agent-browser") || err.contains("cannot be empty"));

    let req_empty_click = ToolCallRequest {
        name: "browser_click".to_string(),
        arguments: json!({ "target": "" }),
    };
    let res = execute_tool(req_empty_click);
    let err = res.unwrap_err();
    assert!(err.contains("agent-browser") || err.contains("cannot be empty"));

    let req_empty_script = ToolCallRequest {
        name: "browser_execute_script".to_string(),
        arguments: json!({ "script": "" }),
    };
    let res = execute_tool(req_empty_script);
    let err = res.unwrap_err();
    assert!(err.contains("agent-browser") || err.contains("cannot be empty"));
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
            "browserType": "chrome",
            "headed": false,
            "outputDir": "./test-output"
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
    let start_req = ToolCallRequest {
        name: "browser_session_start".to_string(),
        arguments: json!({ "engine": "agent-browser" }),
    };
    let start_res = execute_tool(start_req).unwrap();
    let session_id = start_res["session_id"].as_str().unwrap().to_string();

    let stop_req = ToolCallRequest {
        name: "browser_session_stop".to_string(),
        arguments: json!({ "sessionId": session_id.clone() }),
    };
    let stop_res = execute_tool(stop_req);
    assert!(stop_res.is_ok());
    let result = stop_res.unwrap();
    assert_eq!(result["status"], "stopped");
    assert_eq!(result["session_id"], session_id);
}

#[test]
fn test_session_stop_with_invalid_session() {
    let req = ToolCallRequest {
        name: "browser_session_stop".to_string(),
        arguments: json!({ "sessionId": "nonexistent_session_12345" }),
    };
    let res = execute_tool(req);
    let err = res.unwrap_err();
    assert!(err.contains("Failed to stop session"));
}

#[test]
fn test_session_stop_without_session_id() {
    let req = ToolCallRequest {
        name: "browser_session_stop".to_string(),
        arguments: json!({}),
    };
    let res = execute_tool(req);
    let err = res.unwrap_err();
    assert!(err.contains("sessionId") || err.contains("session_id"));
}

#[test]
fn test_click_with_double_click() {
    let req = ToolCallRequest {
        name: "browser_click".to_string(),
        arguments: json!({
            "target": "#submit-btn",
            "sessionId": "test_session",
            "doubleClick": true,
            "newTab": false
        }),
    };
    let res = execute_tool(req);
    assert!(res.is_ok() || res.unwrap_err().contains("agent-browser"));
}

#[test]
fn test_fill_with_press_enter() {
    let req = ToolCallRequest {
        name: "browser_fill".to_string(),
        arguments: json!({
            "target": "#email",
            "value": "user@example.com",
            "sessionId": "test_session",
            "pressEnter": true
        }),
    };
    let res = execute_tool(req);
    assert!(res.is_ok() || res.unwrap_err().contains("agent-browser"));
}

#[test]
fn test_screenshot_with_full_page() {
    let req = ToolCallRequest {
        name: "browser_screenshot".to_string(),
        arguments: json!({
            "filename": "full-page.png",
            "sessionId": "test_session",
            "fullPage": true,
            "outputDir": "./test-output"
        }),
    };
    let res = execute_tool(req);
    assert!(res.is_ok() || res.unwrap_err().contains("agent-browser"));
}

#[test]
fn test_cdp_request_with_params() {
    let req = ToolCallRequest {
        name: "browser_cdp_request".to_string(),
        arguments: json!({
            "sessionId": "cdp_test",
            "method": "Page.enable",
            "params": {"some": "params"}
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
        arguments: json!({ "sessionId": "cdp_test" }),
    };
    let res = execute_tool(req);
    assert!(res.is_ok());
    let result = res.unwrap();
    assert_eq!(result["status"], "disconnected");
}
