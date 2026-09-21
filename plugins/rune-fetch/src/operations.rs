use rune_pdk::ToolCallRequest;
use serde_json::{Value, json};

use crate::types::{FetchPayload, FetchResult};

/// Default tool execution without an external fetcher.
/// Complies with `test_plugin_contract!` requirements.
pub fn execute_tool(req: ToolCallRequest) -> Result<Value, String> {
    execute_tool_with_fetcher(req, |url| {
        Err(format!(
            "Network fetch is managed by the host environment. Standalone fetching for '{}' is disabled. Use execute_tool_with_fetcher to inject a custom fetcher.",
            url
        ))
    })
}

/// Executes tools with an injected network fetch closure.
/// Keeps `operations.rs` free of `extism_pdk` or platform socket APIs (§2.1).
pub fn execute_tool_with_fetcher<F>(req: ToolCallRequest, fetcher: F) -> Result<Value, String>
where
    F: FnOnce(&str) -> Result<String, String>,
{
    match req.name.as_str() {
        "fetch" => handle_fetch(&req.arguments, fetcher),
        unknown => Err(format!(
            "Unknown tool: '{}'. Available tools: 'fetch'. Please check the tool name.",
            unknown
        )),
    }
}

fn handle_fetch<F>(args: &Value, fetcher: F) -> Result<Value, String>
where
    F: FnOnce(&str) -> Result<String, String>,
{
    // Validate existence of URL before general deserialization for actionable error messages
    let url_val = args
        .get("url")
        .ok_or_else(|| "Missing required parameter 'url'. Please provide a valid HTTP or HTTPS URL (e.g. 'https://example.com').".to_string())?;

    let url_str = url_val
        .as_str()
        .ok_or_else(|| "Parameter 'url' must be a string.".to_string())?;

    if url_str.trim().is_empty() {
        return Err(
            "Parameter 'url' cannot be empty. Please provide a valid HTTP or HTTPS URL."
                .to_string(),
        );
    }

    let trimmed_url = url_str.trim();
    if !trimmed_url.starts_with("http://") && !trimmed_url.starts_with("https://") {
        return Err(format!(
            "Invalid URL '{}': URL must start with 'http://' or 'https://'.",
            trimmed_url
        ));
    }

    let payload: FetchPayload = serde_json::from_value(args.clone()).map_err(|e| {
        format!(
            "Invalid arguments for 'fetch': {}. Please check your parameter types.",
            e
        )
    })?;

    let raw_body = fetcher(trimmed_url)?;

    process_content(
        &raw_body,
        payload.raw,
        payload.paginate,
        payload.start_index,
        payload.max_length,
    )
}

pub fn process_content(
    raw_content: &str,
    is_raw: bool,
    paginate: bool,
    start_index: usize,
    max_length: usize,
) -> Result<Value, String> {
    let text = if !is_raw && is_html(raw_content) {
        html_to_markdown(raw_content)
    } else {
        raw_content.to_string()
    };

    let char_vec: Vec<char> = text.chars().collect();
    let total_characters = char_vec.len();

    let start = start_index.min(total_characters);
    let end = (start + max_length).min(total_characters);
    let sliced_content: String = char_vec[start..end].iter().collect();
    let length = sliced_content.chars().count();

    let has_more = end < total_characters;
    let next_start_index = if paginate && has_more {
        Some(end)
    } else {
        None
    };

    let result = FetchResult {
        contents: sliced_content,
        total_characters,
        start_index: start,
        length,
        has_more,
        next_start_index,
    };

    serde_json::to_value(result).map_err(|e| format!("Failed to serialize fetch result: {}", e))
}

pub fn read_resource(uri: &str) -> Result<Value, String> {
    match uri {
        "rune://fetch/help" => Ok(json!({
            "contents": [
                {
                    "uri": "rune://fetch/help",
                    "mimeType": "text/markdown",
                    "text": "# Rune Fetch Plugin\n\n\
                             Fetches web pages and extracts their contents as clean Markdown.\n\n\
                             ## Features\n\
                             - Converts HTML into structured Markdown (headers, links, lists, code blocks)\n\
                             - Strips scripts, styles, and unwanted tags\n\
                             - Character-based pagination with `max_length`, `start_index`, and `paginate`\n\
                             - Option for `raw` content retrieval when exact HTML/text is needed."
                }
            ]
        })),
        unknown => Err(format!(
            "Unknown resource URI: '{}'. Available resources: 'rune://fetch/help'.",
            unknown
        )),
    }
}

pub fn get_prompt(name: &str, args: &Value) -> Result<Value, String> {
    let url = args
        .get("url")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();

    match name {
        "fetch_and_summarize" => {
            let prompt_text = if url.is_empty() {
                "Please fetch the URL provided by the user using the 'fetch' tool, then summarize the key takeaways and main points in bullet format.".to_string()
            } else {
                format!(
                    "Please fetch '{}' using the 'fetch' tool, then summarize the key takeaways and main points in bullet format.",
                    url
                )
            };

            Ok(json!({
                "description": "Fetch a web page and summarize its core content.",
                "messages": [
                    {
                        "role": "user",
                        "content": {
                            "type": "text",
                            "text": prompt_text
                        }
                    }
                ]
            }))
        }
        "fetch_and_extract_markdown" => {
            let prompt_text = if url.is_empty() {
                "Please fetch the target URL using the 'fetch' tool and output the extracted Markdown content.".to_string()
            } else {
                format!(
                    "Please fetch '{}' using the 'fetch' tool with Markdown extraction enabled, and format the output cleanly.",
                    url
                )
            };

            Ok(json!({
                "description": "Fetch a URL and extract clean Markdown.",
                "messages": [
                    {
                        "role": "user",
                        "content": {
                            "type": "text",
                            "text": prompt_text
                        }
                    }
                ]
            }))
        }
        unknown => Err(format!(
            "Unknown prompt: '{}'. Available prompts: 'fetch_and_summarize', 'fetch_and_extract_markdown'.",
            unknown
        )),
    }
}

fn is_html(content: &str) -> bool {
    let trimmed = content.trim_start();
    trimmed.starts_with("<!DOCTYPE")
        || trimmed.starts_with("<html")
        || trimmed.starts_with("<?xml")
        || (content.contains('<') && content.contains("</"))
        || content.contains("<body")
        || content.contains("<div")
}

/// Converts HTML into clean Markdown without external dependencies.
/// Strips scripts and styles, converts headings, links, lists, code, and decodes HTML entities.
pub fn html_to_markdown(html: &str) -> String {
    let mut output = String::with_capacity(html.len());
    let mut chars = html.chars().peekable();
    let mut skip_tags_depth = 0usize;
    let mut link_href_stack: Vec<Option<String>> = Vec::new();
    let mut list_stack: Vec<char> = Vec::new(); // 'u' for unordered, 'o' for ordered
    let mut ol_counter: Vec<usize> = Vec::new();

    while let Some(c) = chars.next() {
        if c == '<' {
            let mut tag_content = String::new();
            while let Some(&next_c) = chars.peek() {
                chars.next();
                if next_c == '>' {
                    break;
                }
                tag_content.push(next_c);
            }

            let trimmed_tag = tag_content.trim();
            if trimmed_tag.is_empty() {
                continue;
            }

            let is_closing = trimmed_tag.starts_with('/');
            let tag_body = if is_closing {
                &trimmed_tag[1..]
            } else {
                trimmed_tag
            };

            let tag_name = tag_body
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_end_matches('/')
                .to_lowercase();

            // Ignore scripts, styles, SVGs, and head elements
            if matches!(tag_name.as_str(), "script" | "style" | "svg" | "noscript") {
                if is_closing {
                    skip_tags_depth = skip_tags_depth.saturating_sub(1);
                } else if !trimmed_tag.ends_with('/') {
                    skip_tags_depth += 1;
                }
                continue;
            }

            if skip_tags_depth > 0 {
                continue;
            }

            // Headings
            if let Some(level) = match tag_name.as_str() {
                "h1" => Some(1),
                "h2" => Some(2),
                "h3" => Some(3),
                "h4" => Some(4),
                "h5" => Some(5),
                "h6" => Some(6),
                _ => None,
            } {
                if is_closing {
                    ensure_blank_line(&mut output);
                } else {
                    ensure_blank_line(&mut output);
                    output.push_str(&format!("{} ", "#".repeat(level)));
                }
                continue;
            }

            // Links
            if tag_name == "a" {
                if is_closing {
                    if let Some(href_opt) = link_href_stack.pop() {
                        if let Some(href) = href_opt {
                            output.push_str(&format!("]({})", href));
                        } else {
                            output.push(']');
                        }
                    }
                } else {
                    let href = extract_attribute(tag_body, "href");
                    link_href_stack.push(href);
                    output.push('[');
                }
                continue;
            }

            // Lists
            match tag_name.as_str() {
                "ul" => {
                    if is_closing {
                        list_stack.pop();
                        ensure_newline(&mut output);
                    } else {
                        list_stack.push('u');
                        ensure_newline(&mut output);
                    }
                }
                "ol" => {
                    if is_closing {
                        list_stack.pop();
                        ol_counter.pop();
                        ensure_newline(&mut output);
                    } else {
                        list_stack.push('o');
                        ol_counter.push(1);
                        ensure_newline(&mut output);
                    }
                }
                "li" => {
                    if is_closing {
                        ensure_newline(&mut output);
                    } else {
                        ensure_newline(&mut output);
                        if list_stack.last() == Some(&'o') {
                            let count = ol_counter
                                .last_mut()
                                .map(|c| {
                                    let curr = *c;
                                    *c += 1;
                                    curr
                                })
                                .unwrap_or(1);
                            output.push_str(&format!("{}. ", count));
                        } else {
                            output.push_str("* ");
                        }
                    }
                }
                "p" | "div" | "article" | "section" => {
                    if is_closing {
                        ensure_blank_line(&mut output);
                    }
                }
                "br" => {
                    output.push('\n');
                }
                "hr" => {
                    ensure_blank_line(&mut output);
                    output.push_str("---\n\n");
                }
                "strong" | "b" => {
                    output.push_str("**");
                }
                "em" | "i" => {
                    output.push('*');
                }
                "code" => {
                    output.push('`');
                }
                "pre" => {
                    ensure_blank_line(&mut output);
                    output.push_str("```\n");
                }
                "blockquote" => {
                    if is_closing {
                        ensure_blank_line(&mut output);
                    } else {
                        ensure_blank_line(&mut output);
                        output.push_str("> ");
                    }
                }
                _ => {}
            }
        } else if skip_tags_depth == 0 {
            output.push(c);
        }
    }

    let decoded = decode_html_entities(&output);
    clean_markdown_whitespace(&decoded)
}

fn extract_attribute(tag: &str, attr: &str) -> Option<String> {
    let lower_tag = tag.to_lowercase();
    let pattern = format!("{}=", attr.to_lowercase());
    let idx = lower_tag.find(&pattern)?;
    let after_eq = tag[idx + pattern.len()..].trim_start();
    let quote = after_eq.chars().next()?;

    if quote == '"' || quote == '\'' {
        let val_start = 1;
        let val_end = after_eq[val_start..].find(quote)?;
        Some(after_eq[val_start..val_start + val_end].to_string())
    } else {
        let val_end = after_eq
            .find(|c: char| c.is_whitespace() || c == '>')
            .unwrap_or(after_eq.len());
        Some(after_eq[..val_end].to_string())
    }
}

fn ensure_newline(output: &mut String) {
    if !output.is_empty() && !output.ends_with('\n') {
        output.push('\n');
    }
}

fn ensure_blank_line(output: &mut String) {
    let trimmed = output.trim_end();
    if !trimmed.is_empty() {
        output.truncate(trimmed.len());
        output.push_str("\n\n");
    }
}

fn decode_html_entities(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '&' {
            let mut entity = String::new();
            let mut found_semi = false;

            while let Some(&next_c) = chars.peek() {
                if next_c == ';' {
                    chars.next();
                    found_semi = true;
                    break;
                }
                if next_c.is_alphanumeric() || next_c == '#' {
                    entity.push(chars.next().unwrap());
                    if entity.len() > 10 {
                        break;
                    }
                } else {
                    break;
                }
            }

            if found_semi {
                match entity.as_str() {
                    "amp" => output.push('&'),
                    "lt" => output.push('<'),
                    "gt" => output.push('>'),
                    "quot" => output.push('"'),
                    "apos" => output.push('\''),
                    "nbsp" => output.push(' '),
                    "copy" => output.push('©'),
                    "mdash" => output.push('—'),
                    "ndash" => output.push('–'),
                    s if s.starts_with("#x") || s.starts_with("#X") => {
                        if let Ok(code) = u32::from_str_radix(&s[2..], 16) {
                            if let Some(ch) = char::from_u32(code) {
                                output.push(ch);
                            } else {
                                output.push_str(&format!("&{};", entity));
                            }
                        } else {
                            output.push_str(&format!("&{};", entity));
                        }
                    }
                    s if s.starts_with('#') => {
                        if let Ok(code) = s[1..].parse::<u32>() {
                            if let Some(ch) = char::from_u32(code) {
                                output.push(ch);
                            } else {
                                output.push_str(&format!("&{};", entity));
                            }
                        } else {
                            output.push_str(&format!("&{};", entity));
                        }
                    }
                    _ => {
                        output.push_str(&format!("&{};", entity));
                    }
                }
            } else {
                output.push('&');
                output.push_str(&entity);
            }
        } else {
            output.push(c);
        }
    }
    output
}

fn clean_markdown_whitespace(input: &str) -> String {
    let mut cleaned_lines = Vec::new();
    let mut consecutive_empty = 0usize;

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            consecutive_empty += 1;
            if consecutive_empty <= 1 {
                cleaned_lines.push("");
            }
        } else {
            consecutive_empty = 0;
            cleaned_lines.push(trimmed);
        }
    }

    cleaned_lines.join("\n").trim().to_string()
}
