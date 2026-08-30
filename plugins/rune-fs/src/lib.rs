// plugins/rune-fs/src/lib.rs
use extism_pdk::*;
use rune_pdk::{ToolCallRequest, ToolDefinition};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};

/// Check if the path is within the allowed directory configuration
fn resolve_path(relative_or_abs: &str) -> Result<PathBuf, String> {
    let target = PathBuf::from(relative_or_abs);

    if let Ok(Some(allowed_root)) = config::get("allowed_dir") {
        let root = Path::new(&allowed_root);
        let canonical_root = root.canonicalize().map_err(|e| e.to_string())?;

        let candidate = if target.is_relative() {
            canonical_root.join(target)
        } else {
            target
        };

        let canonical_candidate = candidate
            .canonicalize()
            .unwrap_or_else(|_| candidate.clone());

        if !canonical_candidate.starts_with(&canonical_root) {
            return Err(format!(
                "Access denied: '{}' is outside the allowed directory '{}'",
                relative_or_abs, allowed_root
            ));
        }
        Ok(canonical_candidate)
    } else {
        Ok(target)
    }
}

/// Internal execution handler where `?` returns `Result<Value, String>`
fn execute_tool(request: ToolCallRequest) -> Result<Value, String> {
    match request.name.as_str() {
        "read_file" => {
            let path_str = request.arguments["path"]
                .as_str()
                .ok_or_else(|| "Missing 'path' argument".to_string())?;
            let path = resolve_path(path_str)?;
            fs::read_to_string(&path)
                .map(|content| json!({ "content": content }))
                .map_err(|e| format!("Failed to read file '{}': {}", path.display(), e))
        }
        "write_file" => {
            let path_str = request.arguments["path"]
                .as_str()
                .ok_or_else(|| "Missing 'path' argument".to_string())?;
            let content = request.arguments["content"]
                .as_str()
                .ok_or_else(|| "Missing 'content' argument".to_string())?;
            let path = resolve_path(path_str)?;
            fs::write(&path, content)
                .map(|_| json!({ "status": "success", "bytes_written": content.len() }))
                .map_err(|e| format!("Failed to write file '{}': {}", path.display(), e))
        }
        "list_directory" => {
            let path_str = request.arguments["path"].as_str().unwrap_or(".");
            let path = resolve_path(path_str)?;
            let entries = fs::read_dir(&path)
                .map_err(|e| format!("Failed to read directory '{}': {}", path.display(), e))?;

            let mut files = Vec::new();
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                files.push(json!({ "name": name, "is_directory": is_dir }));
            }
            Ok(json!({ "entries": files }))
        }
        unknown => Err(format!("Unknown tool: {}", unknown)),
    }
}

/// Exported standard MCP Tool List
#[plugin_fn]
pub fn mcp_list_tools(_: ()) -> FnResult<String> {
    let tools = vec![
        ToolDefinition {
            name: "read_file".to_string(),
            description: "Read the entire contents of a file as UTF-8 text".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "The file path to read" }
                },
                "required": ["path"]
            }),
        },
        ToolDefinition {
            name: "write_file".to_string(),
            description: "Write text contents to a file".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "The file path to write to" },
                    "content": { "type": "string", "description": "The text content to write" }
                },
                "required": ["path", "content"]
            }),
        },
        ToolDefinition {
            name: "list_directory".to_string(),
            description: "List all files and folders inside a given directory".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "The directory path to inspect (defaults to '.')" }
                }
            }),
        },
    ];

    Ok(serde_json::to_string(&tools)?)
}

/// Exported standard MCP Tool Invocation
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
