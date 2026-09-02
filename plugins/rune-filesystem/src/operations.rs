use crate::types::TextEdit;
use base64::Engine;
use chrono::{DateTime, Utc};
use glob_match::glob_match;
use rune_pdk::ToolCallRequest;
use serde_json::{Value, json};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Component, Path, PathBuf};
use walkdir::WalkDir;

const MAX_CHUNK_BYTES: usize = 512 * 1024;
const MAX_IMAGE_BYTES: usize = 20 * 1024 * 1024;

fn get_param<'a>(args: &'a Value, camel: &str, snake: &str) -> Option<&'a Value> {
    args.get(camel).or_else(|| args.get(snake))
}

pub fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => match components.last() {
                Some(Component::Normal(_)) => {
                    components.pop();
                }
                Some(Component::RootDir) | Some(Component::Prefix(_)) => {}
                _ => components.push(Component::ParentDir),
            },
            c => components.push(c),
        }
    }
    components.into_iter().collect()
}

pub fn get_allowed_root() -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        extism_pdk::config::get("allowed_dir")
            .ok()
            .flatten()
            .or_else(|| std::env::var("ALLOWED_DIR").ok())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::env::var("ALLOWED_DIR").ok()
    }
}

fn normalize_separators(p: &str) -> String {
    p.replace('\\', "/")
}

pub fn resolve_path_with_root(
    relative_or_abs: &str,
    allowed_root: Option<&str>,
) -> Result<PathBuf, String> {
    let clean_input = normalize_separators(relative_or_abs.trim());

    if let Some(allowed_root) = allowed_root {
        let clean_root = normalize_separators(allowed_root);
        let root_trimmed = clean_root.trim_end_matches('/');
        let root_path = PathBuf::from(root_trimmed);

        // 1. If path matches root or starts with root prefix, strip it
        let relative_part = if clean_input.eq_ignore_ascii_case(root_trimmed) {
            ""
        } else if clean_input
            .to_ascii_lowercase()
            .starts_with(&format!("{}/", root_trimmed.to_ascii_lowercase()))
        {
            &clean_input[root_trimmed.len() + 1..]
        } else if clean_input.contains(":/") || clean_input.starts_with("//") {
            // Absolute path pointing to another location or drive
            return Err(format!(
                "Access denied: path '{}' is outside allowed directory '{}'",
                relative_or_abs, allowed_root
            ));
        } else {
            // Relative path (clean up leading ./ or /)
            clean_input.trim_start_matches("./").trim_start_matches('/')
        };

        let full_path = if relative_part.is_empty() {
            root_path.clone()
        } else {
            root_path.join(relative_part)
        };

        let normalized = normalize_path(&full_path);
        let normalized_root = normalize_path(&root_path);

        if !normalized.starts_with(&normalized_root) {
            return Err(format!(
                "Access denied: path '{}' escapes allowed directory",
                relative_or_abs
            ));
        }

        Ok(normalized)
    } else {
        let clean = clean_input.strip_prefix("./").unwrap_or(&clean_input);
        Ok(normalize_path(&PathBuf::from(clean)))
    }
}

pub fn resolve_path(relative_or_abs: &str) -> Result<PathBuf, String> {
    let allowed = get_allowed_root();
    resolve_path_with_root(relative_or_abs, allowed.as_deref())
}

pub fn execute_tool(request: ToolCallRequest) -> Result<Value, String> {
    match request.name.as_str() {
        "read_text_file" => {
            let path_val = request
                .arguments
                .get("path")
                .ok_or_else(|| "Missing 'path' parameter".to_string())?;
            let path_str = path_val
                .as_str()
                .ok_or_else(|| "Parameter 'path' must be a string".to_string())?;
            let path = resolve_path(path_str)?;

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
            let line_offset = get_param(&request.arguments, "lineOffset", "line_offset")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;
            let line_limit = get_param(&request.arguments, "lineLimit", "line_limit")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize)
                .unwrap_or(2000);

            if let Some(t) = tail {
                let raw = fs::read_to_string(&path)
                    .map_err(|e| format!("Failed to read text file '{}': {}", path.display(), e))?;
                let lines: Vec<&str> = raw.lines().collect();
                let skip = lines.len().saturating_sub(t);
                return Ok(json!({
                    "content": lines[skip..].join("\n"),
                    "totalLines": lines.len()
                }));
            }

            let file = File::open(&path)
                .map_err(|e| format!("Failed to open text file '{}': {}", path.display(), e))?;
            let reader = BufReader::new(file);

            let limit = head.unwrap_or(line_limit);
            let mut out_lines: Vec<String> = Vec::with_capacity(limit.min(4096));
            let mut seen = 0usize;
            let mut yielded = 0usize;
            let mut has_more = false;

            for line in reader.lines() {
                let line = line.map_err(|e| format!("Read error: {}", e))?;
                if seen < line_offset {
                    seen += 1;
                    continue;
                }
                if yielded >= limit {
                    has_more = true;
                    break;
                }
                out_lines.push(line);
                seen += 1;
                yielded += 1;
            }

            Ok(json!({
                "content": out_lines.join("\n"),
                "lineOffset": line_offset,
                "linesReturned": yielded,
                "hasMore": has_more,
                "nextLineOffset": if has_more { Some(line_offset + yielded) } else { None }
            }))
        }

        "read_media_file" => {
            let path_val = request
                .arguments
                .get("path")
                .ok_or_else(|| "Missing 'path' parameter".to_string())?;
            let path_str = path_val
                .as_str()
                .ok_or_else(|| "Parameter 'path' must be a string".to_string())?;
            let path = resolve_path(path_str)?;
            let mime = mime_guess::from_path(&path)
                .first_or_octet_stream()
                .to_string();
            let is_image = mime.starts_with("image/");
            let max_allowed = if is_image {
                MAX_IMAGE_BYTES
            } else {
                MAX_CHUNK_BYTES
            };

            let offset = request
                .arguments
                .get("offset")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let requested_len = request
                .arguments
                .get("length")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize);

            let mut file = File::open(&path)
                .map_err(|e| format!("Failed to open media file '{}': {}", path.display(), e))?;
            let total_size = file
                .metadata()
                .map_err(|e| format!("Failed to stat '{}': {}", path.display(), e))?
                .len();

            file.seek(SeekFrom::Start(offset))
                .map_err(|e| format!("Seek failed on '{}': {}", path.display(), e))?;

            let chunk_len = requested_len.unwrap_or(max_allowed).min(max_allowed);
            let mut buf = vec![0u8; chunk_len];
            let n = file
                .read(&mut buf)
                .map_err(|e| format!("Failed to read media file '{}': {}", path.display(), e))?;
            buf.truncate(n);

            let end = offset + n as u64;
            let has_more = end < total_size;

            let b64 = base64::engine::general_purpose::STANDARD.encode(&buf);

            let media_type = if mime.starts_with("image/") {
                "image"
            } else if mime.starts_with("audio/") {
                "audio"
            } else {
                "resource"
            };

            let paging = json!({
                "totalBytes": total_size,
                "offset": offset,
                "bytesReturned": n,
                "hasMore": has_more,
                "nextOffset": if has_more { Some(end) } else { None }
            });

            if media_type == "resource" {
                Ok(json!({
                    "content": [{
                        "type": "resource",
                        "resource": {
                            "uri": format!("file:///{}", path.display()),
                            "mimeType": mime,
                            "blob": b64
                        }
                    }],
                    "paging": paging
                }))
            } else {
                Ok(json!({
                    "content": [{
                        "type": media_type,
                        "data": b64,
                        "mimeType": mime
                    }],
                    "paging": paging
                }))
            }
        }

        "read_multiple_files" => {
            let paths = request
                .arguments
                .get("paths")
                .and_then(|v| v.as_array())
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
            let path_val = request
                .arguments
                .get("path")
                .ok_or_else(|| "Missing 'path' parameter".to_string())?;
            let path_str = path_val
                .as_str()
                .ok_or_else(|| "Parameter 'path' must be a string".to_string())?;

            let content_val = request
                .arguments
                .get("content")
                .ok_or_else(|| "Missing 'content' parameter".to_string())?;
            let content = content_val
                .as_str()
                .ok_or_else(|| "Parameter 'content' must be a string".to_string())?;

            let path = resolve_path(path_str)?;

            if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
                let _ = fs::create_dir_all(parent);
            }

            fs::write(&path, content)
                .map_err(|e| format!("Failed to write file '{}': {}", path.display(), e))?;

            Ok(
                json!({ "content": format!("Successfully wrote {} bytes to {}", content.len(), path_str) }),
            )
        }

        "edit_file" => {
            let path_val = request
                .arguments
                .get("path")
                .ok_or_else(|| "Missing 'path' parameter".to_string())?;
            let path_str = path_val
                .as_str()
                .ok_or_else(|| "Parameter 'path' must be a string".to_string())?;

            let edits_val = request
                .arguments
                .get("edits")
                .ok_or_else(|| "Missing 'edits' parameter".to_string())?;
            let edits: Vec<TextEdit> = serde_json::from_value(edits_val.clone())
                .map_err(|e| format!("Invalid edits array: {}", e))?;

            let dry_run = get_param(&request.arguments, "dryRun", "dry_run")
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
            let path_val = request
                .arguments
                .get("path")
                .ok_or_else(|| "Missing 'path' parameter".to_string())?;
            let path_str = path_val
                .as_str()
                .ok_or_else(|| "Parameter 'path' must be a string".to_string())?;
            let path = resolve_path(path_str)?;

            fs::create_dir_all(&path)
                .map_err(|e| format!("Failed to create directory '{}': {}", path.display(), e))?;

            Ok(json!({ "content": format!("Directory '{}' ready", path_str) }))
        }

        "list_directory" => {
            let path_val = request
                .arguments
                .get("path")
                .ok_or_else(|| "Missing 'path' parameter".to_string())?;
            let path_str = path_val
                .as_str()
                .ok_or_else(|| "Parameter 'path' must be a string".to_string())?;
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
            let path_val = request
                .arguments
                .get("path")
                .ok_or_else(|| "Missing 'path' parameter".to_string())?;
            let path_str = path_val
                .as_str()
                .ok_or_else(|| "Parameter 'path' must be a string".to_string())?;

            let sort_by = get_param(&request.arguments, "sortBy", "sort_by")
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
                items.sort_by_key(|b| std::cmp::Reverse(b.size));
            } else {
                items.sort_by_key(|a| a.name.to_lowercase());
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
            let path_val = request
                .arguments
                .get("path")
                .ok_or_else(|| "Missing 'path' parameter".to_string())?;
            let path_str = path_val
                .as_str()
                .ok_or_else(|| "Parameter 'path' must be a string".to_string())?;
            let path = resolve_path(path_str)?;

            let excludes: Vec<String> =
                get_param(&request.arguments, "excludePatterns", "exclude_patterns")
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
            let src_val = request
                .arguments
                .get("source")
                .ok_or_else(|| "Missing 'source' parameter".to_string())?;
            let src_str = src_val
                .as_str()
                .ok_or_else(|| "Parameter 'source' must be a string".to_string())?;

            let dest_val = request
                .arguments
                .get("destination")
                .ok_or_else(|| "Missing 'destination' parameter".to_string())?;
            let dest_str = dest_val
                .as_str()
                .ok_or_else(|| "Parameter 'destination' must be a string".to_string())?;

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
            let path_val = request
                .arguments
                .get("path")
                .ok_or_else(|| "Missing 'path' parameter".to_string())?;
            let path_str = path_val
                .as_str()
                .ok_or_else(|| "Parameter 'path' must be a string".to_string())?;

            let pattern_val = request
                .arguments
                .get("pattern")
                .ok_or_else(|| "Missing 'pattern' parameter".to_string())?;
            let pattern = pattern_val
                .as_str()
                .ok_or_else(|| "Parameter 'pattern' must be a string".to_string())?;

            let path = resolve_path(path_str)?;

            let excludes: Vec<String> =
                get_param(&request.arguments, "excludePatterns", "exclude_patterns")
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
            let path_val = request
                .arguments
                .get("path")
                .ok_or_else(|| "Missing 'path' parameter".to_string())?;
            let path_str = path_val
                .as_str()
                .ok_or_else(|| "Parameter 'path' must be a string".to_string())?;
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
            let allowed = get_allowed_root().unwrap_or_else(|| ".".to_string());
            Ok(json!({ "content": serde_json::to_string(&vec![allowed]).unwrap() }))
        }

        unknown => Err(format!("Unknown tool: {}", unknown)),
    }
}
