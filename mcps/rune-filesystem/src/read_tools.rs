use crate::access::{Meta, ReadFs};
use crate::output::Listing;
use crate::pilot_reads::{self, Page, relative_path};
use chrono::{DateTime, Utc};
use glob_match::glob_match;
use rune_pdk::ToolCallRequest;
use serde_json::{Value, json};
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

const MAX_CHUNK_BYTES: usize = 512 * 1024;
const MAX_IMAGE_BYTES: usize = 20 * 1024 * 1024;
pub(crate) const MAX_WALK_NODES: usize = 20_000;

pub(crate) fn is_read_tool(name: &str) -> bool {
    matches!(
        name,
        "read_text_file"
            | "list_directory"
            | "read_media_file"
            | "read_multiple_files"
            | "list_directory_with_sizes"
            | "directory_tree"
            | "search_files"
            | "get_file_info"
    )
}

fn str_arg<'a>(args: &'a Value, name: &str) -> Result<&'a str, String> {
    args.get(name)
        .ok_or_else(|| format!("Missing '{name}' parameter"))?
        .as_str()
        .ok_or_else(|| format!("Parameter '{name}' must be a string"))
}

fn alias<'a>(args: &'a Value, camel: &str, snake: &str) -> Option<&'a Value> {
    args.get(camel).or_else(|| args.get(snake))
}

fn excludes(args: &Value) -> Vec<String> {
    alias(args, "excludePatterns", "exclude_patterns")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn locate(path: &str, root: &str) -> Result<(PathBuf, String), String> {
    let rel = relative_path(path, root)?;
    Ok((rel.clone(), display(root, &rel)))
}

fn display(root: &str, rel: &Path) -> String {
    let root = root.replace('\\', "/");
    let root = root.trim_end_matches('/');
    let rel = rel.to_string_lossy().replace('\\', "/");
    if rel == "." || rel.is_empty() {
        root.to_string()
    } else {
        format!("{root}/{rel}")
    }
}

fn rfc3339(time: Option<SystemTime>) -> Option<String> {
    time.map(|t| DateTime::<Utc>::from(t).to_rfc3339())
}

fn read_error(action: &str, path: &str, error: &io::Error) -> String {
    format!("Failed to {action} '{path}': {error}")
}

pub(crate) fn run(
    request: &ToolCallRequest,
    root: &str,
    fs: &dyn ReadFs,
    listing: &mut Option<Listing>,
) -> Result<Value, String> {
    let args = &request.arguments;
    if args.get("allowedDir").is_some() || args.get("allowed_dir").is_some() {
        return Err(
            "Root overrides are not accepted in tool arguments; configure allowed_dir on the host"
                .into(),
        );
    }
    match request.name.as_str() {
        "read_text_file" => {
            let path = args
                .get("path")
                .and_then(Value::as_str)
                .ok_or("Missing or invalid string 'path'")?;
            let (rel, _) = locate(path, root)?;
            let page = Page::parse(args)?;
            let meta = fs
                .stat(&rel)
                .map_err(|e| pilot_reads::open_error("open file", path, &e))?;
            if !meta.is_file {
                return Err(pilot_reads::not_a_file(path));
            }
            pilot_reads::check_size(meta.len)?;
            let bytes = fs.read(&rel).map_err(|e| format!("Read error: {e}"))?;
            page.render(&bytes)
        }
        "list_directory" => {
            let path = args
                .get("path")
                .and_then(Value::as_str)
                .ok_or("Missing or invalid string 'path'")?;
            let (rel, _) = locate(path, root)?;
            let max = pilot_reads::max_entries(args)?;
            let entries = fs
                .list(&rel)
                .map_err(|e| pilot_reads::open_error("list directory", path, &e))?;
            let rows = entries.into_iter().map(|entry| {
                Ok::<_, io::Error>((
                    entry.name,
                    pilot_reads::kind(entry.meta.is_dir, entry.is_symlink),
                ))
            });
            pilot_reads::list_result(max, Box::new(rows), listing)
        }
        "read_media_file" => {
            let path_str = str_arg(args, "path")?;
            let (rel, shown) = locate(path_str, root)?;
            let mime = mime_guess::from_path(&rel)
                .first_or_octet_stream()
                .to_string();
            let max_allowed = if mime.starts_with("image/") {
                MAX_IMAGE_BYTES
            } else {
                MAX_CHUNK_BYTES
            };
            let offset = args.get("offset").and_then(Value::as_u64).unwrap_or(0);
            let requested = args
                .get("length")
                .and_then(Value::as_u64)
                .map(|v| v as usize);
            let chunk = requested.unwrap_or(max_allowed).min(max_allowed);
            let (buf, total) = fs
                .read_range(&rel, offset, chunk)
                .map_err(|e| read_error("read media file", &shown, &e))?;
            Ok(crate::operations::media_response(
                &shown, &mime, offset, total, &buf,
            ))
        }
        "read_multiple_files" => {
            let paths = args
                .get("paths")
                .and_then(Value::as_array)
                .ok_or_else(|| "Missing 'paths' array".to_string())?;
            let mut out = String::new();
            for p in paths {
                if let Some(path_str) = p.as_str() {
                    let (rel, _) = locate(path_str, root)?;
                    out.push_str(&format!("--- {path_str} ---\n"));
                    match fs
                        .read(&rel)
                        .and_then(|b| String::from_utf8(b).map_err(io::Error::other))
                    {
                        Ok(content) => out.push_str(&content),
                        Err(e) => out.push_str(&format!("[Error reading file: {e}]")),
                    }
                    out.push_str("\n\n");
                }
            }
            Ok(json!({ "content": out.trim_end() }))
        }
        "list_directory_with_sizes" => {
            let path_str = str_arg(args, "path")?;
            let sort_by = alias(args, "sortBy", "sort_by")
                .and_then(Value::as_str)
                .unwrap_or("name");
            let (rel, shown) = locate(path_str, root)?;
            let mut items = fs
                .list(&rel)
                .map_err(|e| read_error("read directory", &shown, &e))?;
            if sort_by == "size" {
                items.sort_by_key(|b| std::cmp::Reverse(b.meta.len));
            } else {
                items.sort_by_key(|a| a.name.to_lowercase());
            }
            let lines: Vec<String> = items
                .into_iter()
                .map(|item| {
                    if item.meta.is_dir {
                        format!("[DIR]  {}", item.name)
                    } else {
                        format!("[FILE] {:<30} ({:>8} bytes)", item.name, item.meta.len)
                    }
                })
                .collect();
            *listing = Some(Listing::lines(&lines));
            Ok(json!({ "content": lines.join("\n") }))
        }
        "directory_tree" => {
            let path_str = str_arg(args, "path")?;
            let (rel, _) = locate(path_str, root)?;
            let patterns = excludes(args);
            let mut nodes = 0;
            let children = build_tree(fs, &rel, &patterns, &mut nodes)?;
            let tree = json!({"name": path_str, "type": "directory", "children": children});
            *listing = Some(Listing::tree(&tree));
            Ok(json!({ "content": serde_json::to_string_pretty(&tree).unwrap_or_default() }))
        }
        "search_files" => {
            let path_str = str_arg(args, "path")?;
            let pattern = str_arg(args, "pattern")?;
            let (rel, shown) = locate(path_str, root)?;
            let patterns = excludes(args);
            let mut matches = Vec::new();
            let mut nodes = 0;
            if let Ok(meta) = fs.stat(&rel) {
                let name = rel
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| {
                        Path::new(root)
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_default()
                    });
                consider(&name, &shown, pattern, &patterns, &mut matches);
                if meta.is_dir {
                    search(fs, root, &rel, pattern, &patterns, &mut matches, &mut nodes)?;
                }
            }
            *listing = Some(Listing::lines(&matches));
            Ok(json!({ "content": matches.join("\n") }))
        }
        "get_file_info" => {
            let path_str = str_arg(args, "path")?;
            let (rel, shown) = locate(path_str, root)?;
            let meta: Meta = fs
                .stat(&rel)
                .map_err(|e| read_error("read metadata for", &shown, &e))?;
            let info = json!({
                "path": path_str,
                "size_bytes": meta.len,
                "is_directory": meta.is_dir,
                "is_file": meta.is_file,
                "is_readonly": meta.readonly,
                "created": rfc3339(meta.created),
                "modified": rfc3339(meta.modified)
            });
            Ok(json!({ "content": serde_json::to_string_pretty(&info).unwrap_or_default() }))
        }
        other => Err(format!("Unknown tool: {other}")),
    }
}

fn child(dir: &Path, name: &str) -> PathBuf {
    if dir == Path::new(".") {
        PathBuf::from(name)
    } else {
        dir.join(name)
    }
}

fn too_big() -> String {
    format!(
        "Directory walk exceeds {MAX_WALK_NODES} entries; use a narrower path or excludePatterns"
    )
}

fn build_tree(
    fs: &dyn ReadFs,
    dir: &Path,
    patterns: &[String],
    nodes: &mut usize,
) -> Result<Value, String> {
    let mut children = Vec::new();
    let Ok(entries) = fs.list(dir) else {
        return Ok(json!(children));
    };
    for entry in entries {
        if patterns.iter().any(|p| glob_match(p, &entry.name)) {
            continue;
        }
        *nodes += 1;
        if *nodes > MAX_WALK_NODES {
            return Err(too_big());
        }
        if entry.meta.is_dir && !entry.is_symlink {
            children.push(json!({
                "name": entry.name,
                "type": "directory",
                "children": build_tree(fs, &child(dir, &entry.name), patterns, nodes)?
            }));
        } else {
            children.push(json!({"name": entry.name, "type": "file"}));
        }
    }
    Ok(json!(children))
}

fn consider(name: &str, shown: &str, pattern: &str, excludes: &[String], out: &mut Vec<String>) {
    if excludes.iter().any(|ex| glob_match(ex, name)) {
        return;
    }
    if glob_match(pattern, name) {
        out.push(shown.to_string());
    }
}

fn search(
    fs: &dyn ReadFs,
    root: &str,
    dir: &Path,
    pattern: &str,
    excludes: &[String],
    matches: &mut Vec<String>,
    nodes: &mut usize,
) -> Result<(), String> {
    let Ok(entries) = fs.list(dir) else {
        return Ok(());
    };
    for entry in entries {
        *nodes += 1;
        if *nodes > MAX_WALK_NODES {
            return Err(too_big());
        }
        let rel = child(dir, &entry.name);
        consider(
            &entry.name,
            &display(root, &rel),
            pattern,
            excludes,
            matches,
        );
        if entry.meta.is_dir && !entry.is_symlink {
            search(fs, root, &rel, pattern, excludes, matches, nodes)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::access::Scoped;
    use std::fs;

    fn run_scoped(root: &Path, name: &str, args: Value) -> Result<Value, String> {
        let root = root.to_str().unwrap().replace('\\', "/");
        let request = ToolCallRequest {
            name: name.into(),
            arguments: args,
        };
        run(&request, &root, &Scoped { root: &root }, &mut None)
    }

    fn link(target: &Path, at: &Path) -> bool {
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(target, at).is_ok()
        }
        #[cfg(windows)]
        {
            let made = if target.is_dir() {
                std::os::windows::fs::symlink_dir(target, at)
            } else {
                std::os::windows::fs::symlink_file(target, at)
            };
            made.is_ok()
        }
    }

    const READS: [(&str, &str); 8] = [
        ("read_text_file", "path"),
        ("read_media_file", "path"),
        ("get_file_info", "path"),
        ("list_directory", "path"),
        ("list_directory_with_sizes", "path"),
        ("directory_tree", "path"),
        ("search_files", "path"),
        ("read_multiple_files", "paths"),
    ];

    fn args_for(key: &str, path: &str) -> Value {
        match key {
            "paths" => json!({"paths": [path]}),
            _ => json!({"path": path, "pattern": "*"}),
        }
    }

    #[test]
    fn rooted_reads_work_and_match_the_legacy_formats() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir(root.join("sub")).unwrap();
        fs::write(root.join("sub/a.txt"), "hello").unwrap();
        fs::write(root.join("big.bin"), vec![0u8; 40]).unwrap();

        let info = run_scoped(root, "get_file_info", json!({"path": "sub/a.txt"})).unwrap();
        let info: Value = serde_json::from_str(info["content"].as_str().unwrap()).unwrap();
        assert_eq!(info["size_bytes"], 5);
        assert!(info["modified"].as_str().is_some());

        let sizes = run_scoped(root, "list_directory_with_sizes", json!({"path": "."})).unwrap();
        let text = sizes["content"].as_str().unwrap();
        assert!(text.contains("[DIR]  sub") && text.contains("big.bin"));
        assert!(text.contains("(      40 bytes)"));

        let tree = run_scoped(root, "directory_tree", json!({"path": "."})).unwrap();
        let tree: Value = serde_json::from_str(tree["content"].as_str().unwrap()).unwrap();
        assert_eq!(tree["name"], ".");
        assert_eq!(tree["type"], "directory");

        let found =
            run_scoped(root, "search_files", json!({"path": ".", "pattern": "a.*"})).unwrap();
        assert!(found["content"].as_str().unwrap().ends_with("/sub/a.txt"));

        let many = run_scoped(
            root,
            "read_multiple_files",
            json!({"paths": ["sub/a.txt", "none"]}),
        )
        .unwrap();
        let text = many["content"].as_str().unwrap();
        assert!(text.starts_with("--- sub/a.txt ---\nhello\n\n--- none ---\n[Error reading file:"));

        let media = run_scoped(root, "read_media_file", json!({"path": "big.bin"})).unwrap();
        assert_eq!(media["paging"]["totalBytes"], 40);
        let past = run_scoped(
            root,
            "read_media_file",
            json!({"path": "big.bin", "offset": 100}),
        )
        .unwrap();
        assert_eq!(past["paging"]["bytesReturned"], 0);
    }

    #[test]
    fn rooted_reads_refuse_traversal_and_outside_paths() {
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("secret.txt"), "secret").unwrap();
        let secret = outside.path().join("secret.txt");
        let secret = secret.to_str().unwrap();
        for (name, key) in READS {
            for path in [
                "../secret.txt",
                "sub/../../secret.txt",
                secret,
                "/etc/passwd",
            ] {
                let error = run_scoped(dir.path(), name, args_for(key, path))
                    .expect_err(&format!("{name} read {path}"));
                assert!(error.contains("Access denied"), "{name} {path}: {error}");
            }
        }
        let error = run_scoped(
            dir.path(),
            "get_file_info",
            json!({"path": ".", "allowedDir": "/"}),
        )
        .unwrap_err();
        assert!(error.contains("Root overrides"));
    }

    #[test]
    fn rooted_reads_do_not_follow_links_out_of_the_root() {
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("secret.txt"), "secret").unwrap();
        fs::write(dir.path().join("ok.txt"), "ok").unwrap();
        let file_link = link(
            &outside.path().join("secret.txt"),
            &dir.path().join("leak.txt"),
        );
        let dir_link = link(outside.path(), &dir.path().join("leakdir"));
        if !(file_link && dir_link) {
            eprintln!("symlinks unavailable here (needs privilege on Windows); skipping");
            return;
        }
        for (name, key) in READS {
            for path in ["leak.txt", "leakdir", "leakdir/secret.txt"] {
                if let Ok(value) = run_scoped(dir.path(), name, args_for(key, path)) {
                    let shown = value.to_string();
                    assert!(
                        !shown.contains("secret\""),
                        "{name} leaked via {path}: {shown}"
                    );
                    assert!(!shown.contains("\"secret\""), "{name} leaked via {path}");
                    assert!(
                        !shown.contains(r"\nsecret") && !shown.ends_with("secret"),
                        "{name} leaked via {path}"
                    );
                }
            }
        }
        let tree = run_scoped(dir.path(), "directory_tree", json!({"path": "."})).unwrap();
        let tree = tree["content"].as_str().unwrap();
        assert!(tree.contains("leakdir") && !tree.contains("secret.txt"));
        let found = run_scoped(
            dir.path(),
            "search_files",
            json!({"path": ".", "pattern": "secret*"}),
        )
        .unwrap();
        assert_eq!(found["content"], "");
        let read = run_scoped(dir.path(), "read_text_file", json!({"path": "leak.txt"}));
        assert!(read.is_err());
    }
}
