use rune_pdk::ToolCallRequest;
use rune_time::operations::execute_tool;
use serde_json::json;

#[test]
fn test_get_current_time_utc() {
    let req = ToolCallRequest {
        name: "get_current_time".to_string(),
        arguments: json!({ "timezone": "UTC" }),
    };

    let res = execute_tool(req).expect("Failed to get UTC time");
    assert_eq!(res["timezone"], "UTC");
    assert!(res["datetime"].as_str().is_some());
    assert_eq!(res["utc_offset"], "+00:00");
}

#[test]
fn test_get_current_time_kolkata() {
    let req = ToolCallRequest {
        name: "get_current_time".to_string(),
        arguments: json!({ "timezone": "Asia/Kolkata" }),
    };

    let res = execute_tool(req).expect("Failed to get Kolkata time");
    assert_eq!(res["timezone"], "Asia/Kolkata");
    assert_eq!(res["utc_offset"], "+05:30");
}

#[test]
fn test_convert_time_fixed_difference() {
    let req = ToolCallRequest {
        name: "convert_time".to_string(),
        arguments: json!({
            "source_timezone": "UTC",
            "target_timezone": "Asia/Kolkata",
            "time": "2026-09-02T12:00:00"
        }),
    };

    let res = execute_tool(req).expect("Failed to convert time");
    assert_eq!(res["source"]["timezone"], "UTC");
    assert_eq!(res["target"]["timezone"], "Asia/Kolkata");
    assert_eq!(res["time_difference_hours"], 5.5);
    assert_eq!(res["target"]["datetime"], "2026-09-02T17:30:00+05:30");
}

#[test]
fn test_convert_time_invalid_timezone() {
    let req = ToolCallRequest {
        name: "convert_time".to_string(),
        arguments: json!({
            "source_timezone": "Invalid/Zone",
            "target_timezone": "UTC",
            "time": "2026-09-02T12:00:00"
        }),
    };

    let res = execute_tool(req);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Invalid source IANA timezone"));
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
