// plugins/rune-fetch/src/lib.rs
use extism_pdk::*;
use rune_pdk::{ToolCallRequest, ToolDefinition};
use serde_json::{Value, json};

fn execute_tool(request: ToolCallRequest) -> Result<Value, String> {
    match request.name.as_str() {
        "fetch" => {
            let url = request.arguments["url"]
                .as_str()
                .ok_or_else(|| "Missing 'url' parameter".to_string())?;

            let max_length = request
                .arguments
                .get("max_length")
                .and_then(|v| v.as_u64())
                .unwrap_or(5000) as usize;

            let start_index = request
                .arguments
                .get("start_index")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;

            let raw = request
                .arguments
                .get("raw")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            // Create outbound HTTP request via Extism host capabilities
            let user_agent = config::get("user_agent")
                .unwrap_or(None)
                .unwrap_or_else(|| "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Rune-MCP/0.1.0".to_string());

            let http_req = HttpRequest::new(url)
                .with_method("GET")
                .with_header("User-Agent", user_agent)
                .with_header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,text/plain;q=0.8,*/*;q=0.7");

            let response = http::request::<()>(&http_req, None)
                .map_err(|e| format!("Failed to fetch URL '{}': {}", url, e))?;

            let status = response.status_code();
            if status >= 400 {
                return Err(format!("HTTP request failed with status code: {}", status));
            }

            let body_bytes = response.body();
            let body_str = String::from_utf8_lossy(&body_bytes).to_string();

            // Format as Markdown or return raw text
            let processed_text = if raw {
                body_str
            } else {
                html2text::from_read(body_str.as_bytes(), 80)
                    .map_err(|e| format!("Failed to convert HTML to Markdown: {}", e))?
            };

            // Apply pagination and character limits
            let total_chars = processed_text.chars().count();
            let truncated_text: String = processed_text
                .chars()
                .skip(start_index)
                .take(max_length)
                .collect();

            let has_more = (start_index + truncated_text.chars().count()) < total_chars;

            let mut result = json!({
                "contents": truncated_text,
                "total_characters": total_chars,
                "start_index": start_index,
                "length": truncated_text.chars().count(),
                "has_more": has_more
            });

            if has_more {
                result["next_start_index"] = json!(start_index + truncated_text.chars().count());
            }

            Ok(json!({ "content": serde_json::to_string_pretty(&result).unwrap() }))
        }

        unknown => Err(format!("Unknown tool: {}", unknown)),
    }
}

#[plugin_fn]
pub fn mcp_info(_: ()) -> FnResult<String> {
    let info = json!({
        "name": env!("CARGO_PKG_NAME"),
        "version": env!("CARGO_PKG_VERSION"),
        "description": option_env!("CARGO_PKG_DESCRIPTION")
    });
    Ok(serde_json::to_string(&info)?)
}

#[plugin_fn]
pub fn mcp_list_tools(_: ()) -> FnResult<String> {
    let tools = vec![ToolDefinition {
        name: "fetch".to_string(),
        description:
            "Fetches a URL from the internet and extracts its contents as markdown (or raw text)."
                .to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The full HTTP/HTTPS URL to fetch"
                },
                "max_length": {
                    "type": "integer",
                    "default": 5000,
                    "description": "Maximum number of characters to return (default: 5000)"
                },
                "start_index": {
                    "type": "integer",
                    "default": 0,
                    "description": "Start character index for pagination (default: 0)"
                },
                "raw": {
                    "type": "boolean",
                    "default": false,
                    "description": "Return raw HTML/content instead of converting to Markdown"
                }
            },
            "required": ["url"]
        }),
    }];

    Ok(serde_json::to_string(&tools)?)
}

#[plugin_fn]
pub fn mcp_call_tool(input: String) -> FnResult<String> {
    let request: ToolCallRequest = serde_json::from_str(&input)?;
    let result = execute_tool(request);

    let output = match result {
        Ok(val) => json!({ "status": "success", "result": val }),
        Err(err) => json!({ "status": "error", "error": err }),
    };

    Ok(serde_json::to_string(&output)?)
}
