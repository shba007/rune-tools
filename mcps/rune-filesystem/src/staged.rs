//! Staged mode: Kit gives this plugin no preopen of the real folder, so every operation
//! (reads included, so they see staged content) goes through `host_file_access` with
//! absolute paths under `rune_staged_root`. Writes become reviewable changes in Kit; the
//! real folder is untouched until the host applies them.
//!
//! Composition from the broker's primitives (each step is its own reviewable item):
//! * `mkdir -p`: `create_directory` per level, shallowest first.
//! * move a file: `read_file` + `write_file` + `delete_file`.
//! * move a folder: `create_directory`, recurse, `remove_directory` of the emptied source.
use crate::access::{Broker, Brokered, ReadFs};
use crate::output::Listing;
use crate::pilot_reads::relative_path;
use crate::types::TextEdit;
use rune_pdk::ToolCallRequest;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

fn str_arg<'a>(args: &'a Value, name: &str) -> Result<&'a str, String> {
    args.get(name)
        .ok_or_else(|| format!("Missing '{name}' parameter"))?
        .as_str()
        .ok_or_else(|| format!("Parameter '{name}' must be a string"))
}

fn rel(path: &str, root: &str) -> Result<PathBuf, String> {
    relative_path(path, root)
}

fn staged(content: String) -> Value {
    json!({ "content": content, "staged": true })
}

pub(crate) fn execute(
    request: &ToolCallRequest,
    root: &str,
    broker: &dyn Broker,
    listing: &mut Option<Listing>,
) -> Result<Value, String> {
    if root.trim().is_empty() {
        return Err("Staged mode needs the host's rune_staged_root; update rune-kit".into());
    }
    let fs = Brokered { root, broker };
    let args = &request.arguments;
    if crate::read_tools::is_read_tool(&request.name) {
        return crate::read_tools::run(request, root, &fs, listing);
    }
    match request.name.as_str() {
        "list_allowed_directories" => {
            *listing = Some(Listing::directories(root));
            Ok(json!({ "content": serde_json::to_string(&vec![root]).unwrap() }))
        }
        "write_file" => {
            let path_str = str_arg(args, "path")?;
            let content = str_arg(args, "content")?;
            let path = rel(path_str, root)?;
            if let Some(parent) = path.parent() {
                fs.create_dir_all(parent)
                    .map_err(|e| format!("Failed to create parent folders of '{path_str}': {e}"))?;
            }
            fs.write(&path, content.as_bytes())
                .map_err(|e| format!("Failed to write file '{}': {e}", fs.abs(&path)))?;
            Ok(staged(format!(
                "Successfully wrote {} bytes to {}",
                content.len(),
                path_str
            )))
        }
        "edit_file" => {
            let path_str = str_arg(args, "path")?;
            let edits: Vec<TextEdit> = serde_json::from_value(
                args.get("edits")
                    .cloned()
                    .ok_or_else(|| "Missing 'edits' parameter".to_string())?,
            )
            .map_err(|e| format!("Invalid edits array: {e}"))?;
            let dry_run = args
                .get("dryRun")
                .or_else(|| args.get("dry_run"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let path = rel(path_str, root)?;
            let text = fs
                .read(&path)
                .map_err(|e| e.to_string())
                .and_then(|b| String::from_utf8(b).map_err(|e| e.to_string()))
                .map_err(|e| format!("Failed to read file '{}': {e}", fs.abs(&path)))?;
            let (text, diff_log) = crate::operations::apply_edits(text, &edits, path_str)?;
            if dry_run {
                return Ok(json!({ "content": diff_log.trim_end() }));
            }
            fs.write(&path, text.as_bytes())
                .map_err(|e| format!("Failed to apply edits to '{}': {e}", fs.abs(&path)))?;
            Ok(staged(diff_log.trim_end().to_string()))
        }
        "create_directory" => {
            let path_str = str_arg(args, "path")?;
            let path = rel(path_str, root)?;
            fs.create_dir_all(&path)
                .map_err(|e| format!("Failed to create directory '{}': {e}", fs.abs(&path)))?;
            Ok(staged(format!("Directory '{path_str}' ready")))
        }
        "move_file" => {
            let src_str = str_arg(args, "source")?;
            let dest_str = str_arg(args, "destination")?;
            let src = rel(src_str, root)?;
            let dest = rel(dest_str, root)?;
            let fail = |e: String| format!("Failed to move '{src_str}' to '{dest_str}': {e}");
            if src == Path::new(".") {
                return Err(fail("the allowed directory itself cannot be moved".into()));
            }
            if fs.stat(&dest).is_ok() {
                return Err(format!("Destination path '{dest_str}' already exists"));
            }
            if dest.starts_with(&src) {
                return Err(fail("a folder cannot be moved into itself".into()));
            }
            move_tree(&fs, &src, &dest).map_err(|e| fail(e.to_string()))?;
            Ok(staged(format!(
                "Successfully moved '{src_str}' to '{dest_str}'"
            )))
        }
        unknown => Err(format!("Unknown tool: {unknown}")),
    }
}

fn move_tree(fs: &Brokered, src: &Path, dest: &Path) -> std::io::Result<()> {
    if fs.stat(src)?.is_dir {
        fs.create_dir(dest)?;
        for entry in fs.list(src)? {
            move_tree(fs, &src.join(&entry.name), &dest.join(&entry.name))?;
        }
        fs.remove_dir(src)
    } else {
        let data = fs.read(src)?;
        fs.write(dest, &data)?;
        fs.delete(src)
    }
}
