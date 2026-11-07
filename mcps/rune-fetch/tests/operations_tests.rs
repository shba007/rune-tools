use rune_fetch::operations::{
    execute_tool, execute_tool_with_fetcher, get_prompt, html_to_markdown, is_html,
    process_content, read_resource,
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
    assert_eq!(res["hasMore"], false);
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
fn test_fetch_bare_protocol_rejection() {
    let req = ToolCallRequest {
        name: "fetch".to_string(),
        arguments: json!({ "url": "https://" }),
    };
    let res = execute_tool(req);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("must include a host domain"));
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
fn test_html_comments_and_img_extraction() {
    let html = "<!-- outer comment --><p>Check <img src=\"https://example.com/logo.png\" alt=\"Logo\" /> here.</p>";
    let md = html_to_markdown(html);

    assert!(!md.contains("outer comment"));
    assert!(md.contains("![Logo](https://example.com/logo.png)"));
}

#[test]
fn test_pre_and_code_preserves_indentation() {
    let html = "<pre><code>  def foo():\n    return 42</code></pre>";
    let md = html_to_markdown(html);

    assert!(md.contains("```\n  def foo():\n    return 42\n```"));
    assert!(!md.contains("`  def foo()"));
}

#[test]
fn test_extended_html_entities() {
    let html = "<p>Prices &euro;50 &mdash; &copy; 2026 &ldquo;Rune&rdquo;</p>";
    let md = html_to_markdown(html);

    assert!(md.contains("Prices €50 — © 2026 “Rune”"));
}

#[test]
fn test_is_html_accurate_boundary() {
    assert!(is_html("<!doctype html><html><body>ok</body></html>"));
    assert!(is_html("<p>Paragraph</p>"));
    assert!(!is_html("let val = x < y && z </ 2;"));
    assert!(!is_html(
        "This is a simple markdown note without html tags."
    ));
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

    let res_no_paginate = process_content(sample, true, false, 0, 5).unwrap();
    assert_eq!(res_no_paginate["contents"], "01234");
    assert_eq!(res_no_paginate["hasMore"], true);
    assert!(res_no_paginate.get("nextStartIndex").is_none());

    let res_paginate = process_content(sample, true, true, 0, 5).unwrap();
    assert_eq!(res_paginate["contents"], "01234");
    assert_eq!(res_paginate["hasMore"], true);
    assert_eq!(res_paginate["nextStartIndex"], 5);
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
    assert!(read_resource("help").is_ok());
    assert!(read_resource("rune://rune-fetch/help").is_ok());

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
fn test_router_prefix_and_camel_case_alias() {
    let req = ToolCallRequest {
        name: "rune-fetch__fetch".to_string(),
        arguments: json!({
            "url": "https://example.com",
            "maxLength": 10,
            "startIndex": 0
        }),
    };

    let res = execute_tool_with_fetcher(req, |_url| Ok("1234567890abcdef".to_string())).unwrap();
    assert_eq!(res["contents"], "1234567890");
    assert_eq!(res["length"], 10);
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
