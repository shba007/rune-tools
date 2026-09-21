use rune_fetch::operations::{
    execute_tool, execute_tool_with_fetcher, get_prompt, html_to_markdown, process_content,
    read_resource,
};
use rune_pdk::ToolCallRequest;
use serde_json::json;

const TEST_PAGE_BODY: &str = "<!DOCTYPE html>\n\
<html><head><title>Test Page</title><style>body { color: red; }</style></head>\n\
<body>\n\
<h1>Hello Rune</h1>\n\
<p>Visit <a href=\"https://example.com\">Example</a> today.</p>\n\
<script>console.log('strip me');</script>\n\
</body></html>";

#[test]
fn test_fetch_execution_with_mock_fetcher() {
    let req = ToolCallRequest {
        name: "fetch".to_string(),
        arguments: json!({ "url": "https://example.com/test" }),
    };

    let res = execute_tool_with_fetcher(req, |_url| Ok(TEST_PAGE_BODY.to_string())).unwrap();
    let contents = res["contents"].as_str().unwrap();

    assert!(contents.contains("# Hello Rune"));
    assert!(contents.contains("[Example](https://example.com)"));
    assert!(!contents.contains("console.log"));
    assert!(!contents.contains("color: red"));
    assert_eq!(res["has_more"], false);
}

#[test]
fn test_fetch_missing_url_parameter() {
    let req = ToolCallRequest {
        name: "fetch".to_string(),
        arguments: json!({}),
    };
    let res = execute_tool(req);
    assert!(res.is_err());
    assert!(
        res.unwrap_err()
            .contains("Missing required parameter 'url'")
    );
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
fn test_fetch_invalid_protocol() {
    let req = ToolCallRequest {
        name: "fetch".to_string(),
        arguments: json!({ "url": "ftp://example.com" }),
    };
    let res = execute_tool(req);
    assert!(res.is_err());
    assert!(
        res.unwrap_err()
            .contains("must start with 'http://' or 'https://'")
    );
}

#[test]
fn test_html_to_markdown_elements() {
    let html = "<h2>Title</h2><p>Some <b>bold</b> and <i>italic</i> with <code>inline code</code>.</p>\
                <ul><li>Item 1</li><li>Item 2</li></ul>";
    let md = html_to_markdown(html);

    assert!(md.contains("## Title"));
    assert!(md.contains("**bold**"));
    assert!(md.contains("*italic*"));
    assert!(md.contains("`inline code`"));
    assert!(md.contains("* Item 1"));
    assert!(md.contains("* Item 2"));
}

#[test]
fn test_process_content_raw_mode() {
    let html = "<h1>Heading</h1>";
    let result = process_content(html, true, false, 0, 1000).unwrap();
    assert_eq!(result["contents"], "<h1>Heading</h1>");
}

#[test]
fn test_process_content_pagination() {
    let sample = "0123456789abcdef";

    // Pagination disabled: next_start_index must be None
    let res_no_paginate = process_content(sample, true, false, 0, 5).unwrap();
    assert_eq!(res_no_paginate["contents"], "01234");
    assert_eq!(res_no_paginate["has_more"], true);
    assert!(res_no_paginate.get("next_start_index").is_none());

    // Pagination enabled: next_start_index must be Some(5)
    let res_paginate = process_content(sample, true, true, 0, 5).unwrap();
    assert_eq!(res_paginate["contents"], "01234");
    assert_eq!(res_paginate["has_more"], true);
    assert_eq!(res_paginate["next_start_index"], 5);
}

#[test]
fn test_read_resource() {
    let res = read_resource("rune://fetch/help").unwrap();
    assert!(
        res["contents"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Rune Fetch Plugin")
    );

    let err = read_resource("rune://fetch/unknown");
    assert!(err.is_err());
    assert!(err.unwrap_err().contains("Unknown resource URI"));
}

#[test]
fn test_get_prompt() {
    let args = json!({ "url": "https://example.com" });
    let res = get_prompt("fetch_and_summarize", &args).unwrap();
    let text = res["messages"][0]["content"]["text"].as_str().unwrap();
    assert!(text.contains("https://example.com"));

    let err = get_prompt("invalid_prompt", &args);
    assert!(err.is_err());
    assert!(err.unwrap_err().contains("Unknown prompt"));
}

#[test]
fn test_unknown_tool_routing() {
    let req = ToolCallRequest {
        name: "non_existent".to_string(),
        arguments: json!({}),
    };
    let res = execute_tool(req);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Unknown tool: 'non_existent'"));
}
