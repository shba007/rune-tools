// plugins/rune-fs/src/lib.rs
use base64::Engine;
use chrono::{DateTime, Utc};
use extism_pdk::*;
use glob_match::glob_match;
use rune_pdk::{ToolCallRequest, ToolDefinition};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

fn resolve_path(relative_or_abs: &str) -> Result<PathBuf, String> {
    let target = PathBuf::from(relative_or_abs);

    if let Ok(Some(allowed_root)) = config::get("allowed_dir") {
        let root = PathBuf::from(allowed_root);
        if target.is_relative() {
            Ok(root.join(target))
        } else {
            Ok(target)
        }
    } else {
        Ok(target)
    }
}

fn get_allowed_root() -> String {
    config::get("allowed_dir")
        .unwrap_or(None)
        .unwrap_or_else(|| ".".to_string())
}

#[derive(serde::Deserialize)]
struct TextEdit {
    #[serde(rename = "oldText")]
    old_text: String,
    #[serde(rename = "newText")]
    new_text: String,
}

fn execute_tool(request: ToolCallRequest) -> Result<Value, String> {
    match request.name.as_str() {
        "read_text_file" => {
            let path_str = request.arguments["path"]
                .as_str()
                .ok_or_else(|| "Missing 'path' parameter".to_string())?;
            let path = resolve_path(path_str)?;
            let raw = fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read text file '{}': {}", path.display(), e))?;

            let lines: Vec<&str> = raw.lines().collect();
            let total_lines = lines.len();

            let head = request
                .arguments
                .get("head")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize);
            let tail = request
                .arguments
                .get("tail")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize);

            let result_content = if let Some(h) = head {
                lines.into_iter().take(h).collect::<Vec<_>>().join("\n")
            } else if let Some(t) = tail {
                let skip = total_lines.saturating_sub(t);
                lines.into_iter().skip(skip).collect::<Vec<_>>().join("\n")
            } else {
                raw
            };

            Ok(json!({ "content": result_content }))
        }

        "read_media_file" => {
            let path_str = request.arguments["path"]
                .as_str()
                .ok_or_else(|| "Missing 'path' parameter".to_string())?;
            let path = resolve_path(path_str)?;
            let bytes = fs::read(&path)
                .map_err(|e| format!("Failed to read media file '{}': {}", path.display(), e))?;

            let mime = mime_guess::from_path(&path)
                .first_or_octet_stream()
                .to_string();

            let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);

            let media_type = if mime.starts_with("image/") {
                "image"
            } else if mime.starts_with("audio/") {
                "audio"
            } else {
                "resource"
            };

            if media_type == "resource" {
                Ok(json!({
                    "content": [{
                        "type": "resource",
                        "resource": {
                            "uri": format!("file:///{}", path.display()),
                            "mimeType": mime,
                            "blob": b64
                        }
                    }]
                }))
            } else {
                Ok(json!({
                    "content": [{
                        "type": media_type,
                        "data": b64,
                        "mimeType": mime
                    }]
                }))
            }
        }

        "read_multiple_files" => {
            let paths = request.arguments["paths"]
                .as_array()
                .ok_or_else(|| "Missing 'paths' array".to_string())?;

            let mut out = String::new();
            for p in paths {
                if let Some(path_str) = p.as_str() {
                    let path = resolve_path(path_str)?;
                    out.push_str(&format!("--- {} ---\n", path_str));
                    match fs::read_to_string(&path) {
                        Ok(content) => out.push_str(&content),
                        Err(e) => out.push_str(&format!("[Error reading file: {}]", e)),
                    }
                    out.push_str("\n\n");
                }
            }
            Ok(json!({ "content": out.trim_end() }))
        }

        "write_file" => {
            let path_str = request.arguments["path"]
                .as_str()
                .ok_or_else(|| "Missing 'path' parameter".to_string())?;
            let content = request.arguments["content"]
                .as_str()
                .ok_or_else(|| "Missing 'content' parameter".to_string())?;
            let path = resolve_path(path_str)?;

            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }

            fs::write(&path, content)
                .map_err(|e| format!("Failed to write file '{}': {}", path.display(), e))?;

            Ok(
                json!({ "content": format!("Successfully wrote {} bytes to {}", content.len(), path_str) }),
            )
        }

        "edit_file" => {
            let path_str = request.arguments["path"]
                .as_str()
                .ok_or_else(|| "Missing 'path' parameter".to_string())?;
            let edits_val = request
                .arguments
                .get("edits")
                .ok_or_else(|| "Missing 'edits' parameter".to_string())?;
            let edits: Vec<TextEdit> = serde_json::from_value(edits_val.clone())
                .map_err(|e| format!("Invalid edits array: {}", e))?;
            let dry_run = request
                .arguments
                .get("dryRun")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let path = resolve_path(path_str)?;
            let mut text = fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read file '{}': {}", path.display(), e))?;

            let mut diff_log = String::new();
            for (idx, edit) in edits.iter().enumerate() {
                if !text.contains(&edit.old_text) {
                    return Err(format!(
                        "Edit #{} failed: text to replace was not found in '{}'",
                        idx + 1,
                        path_str
                    ));
                }
                diff_log.push_str(&format!(
                    "--- chunk {}\n- {}\n+ {}\n",
                    idx + 1,
                    edit.old_text,
                    edit.new_text
                ));
                text = text.replacen(&edit.old_text, &edit.new_text, 1);
            }

            if !dry_run {
                fs::write(&path, &text)
                    .map_err(|e| format!("Failed to apply edits to '{}': {}", path.display(), e))?;
            }

            Ok(json!({ "content": diff_log.trim_end() }))
        }

        "create_directory" => {
            let path_str = request.arguments["path"]
                .as_str()
                .ok_or_else(|| "Missing 'path' parameter".to_string())?;
            let path = resolve_path(path_str)?;
            fs::create_dir_all(&path)
                .map_err(|e| format!("Failed to create directory '{}': {}", path.display(), e))?;

            Ok(json!({ "content": format!("Directory '{}' ready", path_str) }))
        }

        "list_directory" => {
            let path_str = request.arguments["path"].as_str().unwrap_or(".");
            let path = resolve_path(path_str)?;
            let entries = fs::read_dir(&path)
                .map_err(|e| format!("Failed to read directory '{}': {}", path.display(), e))?;

            let mut lines = Vec::new();
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                if is_dir {
                    lines.push(format!("[DIR]  {}", name));
                } else {
                    lines.push(format!("[FILE] {}", name));
                }
            }
            lines.sort();
            Ok(json!({ "content": lines.join("\n") }))
        }

        "list_directory_with_sizes" => {
            let path_str = request.arguments["path"].as_str().unwrap_or(".");
            let sort_by = request
                .arguments
                .get("sortBy")
                .and_then(|v| v.as_str())
                .unwrap_or("name");
            let path = resolve_path(path_str)?;

            let entries = fs::read_dir(&path)
                .map_err(|e| format!("Failed to read directory '{}': {}", path.display(), e))?;

            struct Item {
                name: String,
                is_dir: bool,
                size: u64,
            }

            let mut items = Vec::new();
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                let meta = entry.metadata().ok();
                let is_dir = meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
                let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
                items.push(Item { name, is_dir, size });
            }

            if sort_by == "size" {
                items.sort_by(|a, b| b.size.cmp(&a.size));
            } else {
                items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            }

            let lines: Vec<String> = items
                .into_iter()
                .map(|item| {
                    if item.is_dir {
                        format!("[DIR]  {}", item.name)
                    } else {
                        format!("[FILE] {:<30} ({:>8} bytes)", item.name, item.size)
                    }
                })
                .collect();

            Ok(json!({ "content": lines.join("\n") }))
        }

        "directory_tree" => {
            let path_str = request.arguments["path"].as_str().unwrap_or(".");
            let path = resolve_path(path_str)?;
            let excludes: Vec<String> = request
                .arguments
                .get("excludePatterns")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            fn build_tree(dir: &Path, excludes: &[String]) -> Value {
                let mut children = Vec::new();
                if let Ok(entries) = fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if excludes.iter().any(|pattern| glob_match(pattern, &name)) {
                            continue;
                        }
                        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                        if is_dir {
                            children.push(json!({
                                "name": name,
                                "type": "directory",
                                "children": build_tree(&entry.path(), excludes)
                            }));
                        } else {
                            children.push(json!({
                                "name": name,
                                "type": "file"
                            }));
                        }
                    }
                }
                json!(children)
            }

            let tree = json!({
                "name": path_str,
                "type": "directory",
                "children": build_tree(&path, &excludes)
            });

            Ok(json!({ "content": serde_json::to_string_pretty(&tree).unwrap_or_default() }))
        }

        "move_file" => {
            let src_str = request.arguments["source"]
                .as_str()
                .ok_or_else(|| "Missing 'source' parameter".to_string())?;
            let dest_str = request.arguments["destination"]
                .as_str()
                .ok_or_else(|| "Missing 'destination' parameter".to_string())?;

            let src = resolve_path(src_str)?;
            let dest = resolve_path(dest_str)?;

            if dest.exists() {
                return Err(format!("Destination path '{}' already exists", dest_str));
            }

            fs::rename(&src, &dest)
                .map_err(|e| format!("Failed to move '{}' to '{}': {}", src_str, dest_str, e))?;

            Ok(json!({ "content": format!("Successfully moved '{}' to '{}'", src_str, dest_str) }))
        }

        "search_files" => {
            let path_str = request.arguments["path"].as_str().unwrap_or(".");
            let pattern = request.arguments["pattern"]
                .as_str()
                .ok_or_else(|| "Missing 'pattern' parameter".to_string())?;
            let path = resolve_path(path_str)?;

            let excludes: Vec<String> = request
                .arguments
                .get("excludePatterns")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            let mut matches = Vec::new();
            for entry in WalkDir::new(&path).into_iter().flatten() {
                let file_name = entry.file_name().to_string_lossy();
                if excludes.iter().any(|ex| glob_match(ex, &file_name)) {
                    continue;
                }
                if glob_match(pattern, &file_name) {
                    matches.push(entry.path().display().to_string());
                }
            }

            Ok(json!({ "content": matches.join("\n") }))
        }

        "get_file_info" => {
            let path_str = request.arguments["path"]
                .as_str()
                .ok_or_else(|| "Missing 'path' parameter".to_string())?;
            let path = resolve_path(path_str)?;
            let meta = fs::metadata(&path)
                .map_err(|e| format!("Failed to read metadata for '{}': {}", path.display(), e))?;

            let modified: Option<String> = meta.modified().ok().map(|t| {
                let dt: DateTime<Utc> = t.into();
                dt.to_rfc3339()
            });
            let created: Option<String> = meta.created().ok().map(|t| {
                let dt: DateTime<Utc> = t.into();
                dt.to_rfc3339()
            });

            let info = json!({
                "path": path_str,
                "size_bytes": meta.len(),
                "is_directory": meta.is_dir(),
                "is_file": meta.is_file(),
                "is_readonly": meta.permissions().readonly(),
                "created": created,
                "modified": modified
            });

            Ok(json!({ "content": serde_json::to_string_pretty(&info).unwrap_or_default() }))
        }

        "list_allowed_directories" => {
            let allowed = get_allowed_root();
            Ok(json!({ "content": serde_json::to_string(&vec![allowed]).unwrap() }))
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
    let tools = vec![
        ToolDefinition {
            name: "read_text_file".to_string(),
            description: "Read the complete contents of a file from the file system as text with optional line range head/tail clipping. Only works within allowed directories.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "The file path to read" },
                    "head": { "type": "number", "description": "If provided, returns only the first N lines of the file" },
                    "tail": { "type": "number", "description": "If provided, returns only the last N lines of the file" }
                },
                "required": ["path"]
            }),
        },
        ToolDefinition {
            name: "read_media_file".to_string(),
            description: "Read a file and return it as a base64-encoded content block with its MIME type. Image and audio files are returned as image/audio content.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Path to the media/binary file" }
                },
                "required": ["path"]
            }),
        },
        ToolDefinition {
            name: "read_multiple_files".to_string(),
            description: "Read the contents of multiple files simultaneously.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "paths": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Array of file paths to read"
                    }
                },
                "required": ["paths"]
            }),
        },
        ToolDefinition {
            name: "write_file".to_string(),
            description: "Create a new file or completely overwrite an existing file with new content. Handles text content with proper UTF-8 encoding.".to_string(),
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
            name: "edit_file".to_string(),
            description: "Make line-based edits to a text file. Each edit replaces exact line sequences with new content. Returns a git-style diff.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "The file path to edit" },
                    "edits": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "oldText": { "type": "string", "description": "Text to search for - must match exactly" },
                                "newText": { "type": "string", "description": "Text to replace with" }
                            },
                            "required": ["oldText", "newText"]
                        }
                    },
                    "dryRun": { "type": "boolean", "default": false, "description": "Preview changes using git-style diff format without saving" }
                },
                "required": ["path", "edits"]
            }),
        },
        ToolDefinition {
            name: "create_directory".to_string(),
            description: "Create a new directory or ensure a directory exists, including parent directories.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "The directory path to create" }
                },
                "required": ["path"]
            }),
        },
        ToolDefinition {
            name: "list_directory".to_string(),
            description: "Get a detailed listing of all files and directories in a specified path, with [FILE] and [DIR] prefixes.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "The directory path to inspect (defaults to '.')" }
                },
                "required": ["path"]
            }),
        },
        ToolDefinition {
            name: "list_directory_with_sizes".to_string(),
            description: "Get a detailed listing of all files and directories in a specified path with sizes and sort options.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "The directory path to inspect" },
                    "sortBy": { "type": "string", "enum": ["name", "size"], "default": "name", "description": "Sort entries by name or size" }
                },
                "required": ["path"]
            }),
        },
        ToolDefinition {
            name: "directory_tree".to_string(),
            description: "Get a recursive tree view of files and directories as a JSON structure.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "The directory root path" },
                    "excludePatterns": { "type": "array", "items": { "type": "string" }, "default": [], "description": "Glob patterns to exclude" }
                },
                "required": ["path"]
            }),
        },
        ToolDefinition {
            name: "move_file".to_string(),
            description: "Move or rename files and directories safely. If destination exists, the operation will fail.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "source": { "type": "string", "description": "Source path" },
                    "destination": { "type": "string", "description": "Destination path" }
                },
                "required": ["source", "destination"]
            }),
        },
        ToolDefinition {
            name: "search_files".to_string(),
            description: "Recursively search for files and directories matching a glob pattern.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Root path to search within" },
                    "pattern": { "type": "string", "description": "Glob pattern (e.g. '*.rs', '**/*.json')" },
                    "excludePatterns": { "type": "array", "items": { "type": "string" }, "default": [], "description": "Patterns to ignore" }
                },
                "required": ["path", "pattern"]
            }),
        },
        ToolDefinition {
            name: "get_file_info".to_string(),
            description: "Retrieve comprehensive metadata about a file or directory including size, created/modified timestamps, and permissions.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Path to inspect" }
                },
                "required": ["path"]
            }),
        },
        ToolDefinition {
            name: "list_allowed_directories".to_string(),
            description: "Returns the list of directories that this server is allowed to access.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
    ];

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
