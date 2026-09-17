use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

use rune_fetch::operations::{execute_tool, process_content};
use rune_pdk::ToolCallRequest;
use serde_json::json;

const TEST_PAGE_BODY: &str = "<!DOCTYPE html>\n\
<html><head><title>Local Test</title></head>\n\
<body><h1>Hello Rune</h1><p>Served by the local test server.</p></body></html>";

/// Serves `TEST_PAGE_BODY` once on a random 127.0.0.1 port and returns the base URL,
/// so the fetch test runs fully offline with no external network dependency.
fn start_local_test_server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("failed to bind local test port");
    let port = listener.local_addr().expect("local address").port();

    thread::spawn(move || {
        let (mut stream, _) = match listener.accept() {
            Ok(conn) => conn,
            Err(_) => return,
        };

        // Read until the request headers are complete (GET has no body).
        let mut received = Vec::new();
        let mut buf = [0u8; 1024];
        while !received.windows(4).any(|w| w == b"\r\n\r\n") {
            match stream.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => received.extend_from_slice(&buf[..n]),
            }
        }

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            TEST_PAGE_BODY.len(),
            TEST_PAGE_BODY
        );
        let _ = stream.write_all(response.as_bytes());
        let _ = stream.flush();
    });

    format!("http://127.0.0.1:{port}/")
}

#[test]
fn test_fetch_native_execution() {
    let url = start_local_test_server();
    let req = ToolCallRequest {
        name: "fetch".to_string(),
        arguments: json!({ "url": url }),
    };
    let res = execute_tool(req).unwrap();
    let contents = res["contents"].as_str().unwrap();
    assert!(contents.contains("Hello Rune"));
    assert!(contents.contains("Served by the local test server."));
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
