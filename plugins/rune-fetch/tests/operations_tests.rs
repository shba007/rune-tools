use rune_fetch::operations::{execute_tool, process_content};
use rune_pdk::ToolCallRequest;
use serde_json::json;

#[test]
fn test_fetch_native_execution() {
    let req = ToolCallRequest {
        name: "fetch".to_string(),
        arguments: json!({ "url": "https://example.com/" }),
    };
    let res = execute_tool(req).unwrap();
    let content = res["contents"].as_str().unwrap();
    assert!(content.contains("This domain is for use in documentation examples"));
}

#[test]
fn test_fetch_missing_url_parameter() {
    let req = ToolCallRequest {
        name: "fetch".to_string(),
        arguments: json!({}),
    };
    let res = execute_tool(req);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Missing 'url' parameter"));
}

#[test]
fn test_fetch_empty_url() {
    let req = ToolCallRequest {
        name: "fetch".to_string(),
        arguments: json!({ "url": "   " }),
    };
    let res = execute_tool(req);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Parameter 'url' cannot be empty"));
}

#[test]
fn test_process_content_html_conversion() {
    let html = "<h1>Heading</h1><p>Text paragraph.</p>";
    // raw = false, paginate = false, start_index = 0, max_length = 1000
    let result = process_content(html, false, false, 0, 1000).unwrap();

    let contents = result["contents"].as_str().unwrap();
    assert!(contents.contains("Heading"));
    assert!(contents.contains("Text paragraph."));
    assert_eq!(result["has_more"], false);
}

#[test]
fn test_process_content_pagination() {
    let sample = "0123456789abcdef";
    // raw = true, paginate = true, start_index = 4, max_length = 6
    let result = process_content(sample, true, true, 4, 6).unwrap();

    assert_eq!(result["contents"], "456789");
    assert_eq!(result["start_index"], 4);
    assert_eq!(result["length"], 6);
    assert_eq!(result["total_characters"], 16);
    assert_eq!(result["has_more"], true);
    assert_eq!(result["next_start_index"], 10);
}

#[test]
fn test_unknown_tool_routing() {
    let req = ToolCallRequest {
        name: "non_existent".to_string(),
        arguments: json!({}),
    };
    let res = execute_tool(req);
    assert!(res.is_err());
    assert_eq!(res.unwrap_err(), "Unknown tool: non_existent");
}
