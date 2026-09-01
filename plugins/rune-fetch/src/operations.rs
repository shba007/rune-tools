use rune_pdk::ToolCallRequest;
use serde_json::{Value, json};
use std::time::Duration;

/// Executes tool operations for `rune-fetch`.
pub fn execute_tool(req: ToolCallRequest) -> Result<Value, String> {
    match req.name.as_str() {
        "fetch" => handle_fetch(&req.arguments),
        unknown => Err(format!("Unknown tool: {}", unknown)),
    }
}

/// Validates arguments and executes a real HTTP GET request.
fn handle_fetch(args: &Value) -> Result<Value, String> {
    let url_val = args
        .get("url")
        .ok_or_else(|| "Missing 'url' parameter".to_string())?;

    let url = url_val
        .as_str()
        .ok_or_else(|| "Parameter 'url' must be a string".to_string())?;

    if url.trim().is_empty() {
        return Err("Parameter 'url' cannot be empty".to_string());
    }

    let raw = args.get("raw").and_then(|v| v.as_bool()).unwrap_or(false);
    let paginate = args
        .get("paginate")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let start_index = args
        .get("start_index")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    let max_length = args
        .get("max_length")
        .and_then(|v| v.as_u64())
        .unwrap_or(50_000) as usize;

    // Real HTTP request using blocking client
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("rune-fetch/0.1.1")
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let response = client
        .get(url)
        .send()
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        return Err(format!("HTTP request returned error status: {}", status));
    }

    let body = response
        .text()
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    process_content(&body, raw, paginate, start_index, max_length)
}

/// Converts HTML if necessary, computes character bounds, and handles pagination.
pub fn process_content(
    raw_content: &str,
    is_raw: bool,
    paginate: bool,
    start_index: usize,
    max_length: usize,
) -> Result<Value, String> {
    let text = if !is_raw && is_html(raw_content) {
        strip_html(raw_content)
    } else {
        raw_content.to_string()
    };

    let char_vec: Vec<char> = text.chars().collect();
    let total_characters = char_vec.len();

    let (sliced_content, has_more, next_start_index) = if paginate {
        let start = start_index.min(total_characters);
        let end = (start + max_length).min(total_characters);
        let sliced: String = char_vec[start..end].iter().collect();
        let more = end < total_characters;
        let next_idx = if more { Value::from(end) } else { Value::Null };

        (sliced, more, next_idx)
    } else {
        let start = start_index.min(total_characters);
        let end = (start + max_length).min(total_characters);
        let sliced: String = char_vec[start..end].iter().collect();

        (sliced, false, Value::Null)
    };

    let length = sliced_content.chars().count();

    let mut response = json!({
        "contents": sliced_content,
        "start_index": start_index,
        "length": length,
        "total_characters": total_characters,
        "has_more": has_more,
    });

    if paginate && has_more {
        response["next_start_index"] = next_start_index;
    }

    Ok(response)
}

fn is_html(content: &str) -> bool {
    let trimmed = content.trim_start();
    trimmed.starts_with("<!DOCTYPE")
        || trimmed.starts_with("<html")
        || (content.contains('<') && content.contains("</"))
}

/// Strips HTML tags and normalizes whitespace while preserving line-break structure.
fn strip_html(html: &str) -> String {
    let mut in_tag = false;
    let mut result = String::with_capacity(html.len());
    let mut chars = html.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                result.push(' ');
            }
            _ if !in_tag => result.push(c),
            _ => {}
        }
    }

    result
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}
