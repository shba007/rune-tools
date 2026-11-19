use rune_pdk::ToolCallRequest;
use serde_json::{Value, json};
use std::io::Read;
use std::path::{Path, PathBuf};

pub use crate::definitions::{MAX_ENTRIES, MAX_FILE_BYTES, MAX_LINES, MAX_OUTPUT_BYTES};

pub fn relative_path(path: &str, root: &str) -> Result<PathBuf, String> {
    let input = path.replace('\\', "/");
    let root = root.replace('\\', "/");
    let root = root.trim_end_matches('/');
    if input.is_empty() || input.contains('\0') {
        return Err("Path must be a non-empty file or directory name".into());
    }
    let absolute = input.starts_with('/') || input.contains(':');
    let relative = if input == root {
        "."
    } else if absolute {
        input
            .strip_prefix(root)
            .and_then(|p| p.strip_prefix('/'))
            .ok_or("Access denied: absolute path is outside configured allowed_dir")?
    } else {
        &input
    };
    let mut result = PathBuf::new();
    for part in relative.split('/') {
        if part == "." || part.is_empty() {
            continue;
        }
        let device = part.split('.').next().unwrap_or("").to_ascii_uppercase();
        if part == ".."
            || part.contains(':')
            || part.ends_with([' ', '.'])
            || matches!(device.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || (device.len() == 4
                && (device.starts_with("COM") || device.starts_with("LPT"))
                && matches!(device.as_bytes()[3], b'1'..=b'9'))
        {
            return Err(
                "Access denied: traversal, device paths and alternate streams are not accepted"
                    .into(),
            );
        }
        result.push(part);
    }
    if result.as_os_str().is_empty() {
        result.push(".");
    }
    Ok(result)
}

pub(crate) fn number(
    args: &Value,
    name: &str,
    alias: &str,
    default: usize,
    max: usize,
) -> Result<usize, String> {
    let Some(value) = args.get(name).or_else(|| args.get(alias)) else {
        return Ok(default);
    };
    let value = value
        .as_u64()
        .ok_or_else(|| format!("'{name}' must be a non-negative integer"))?;
    let value = usize::try_from(value).map_err(|_| format!("'{name}' is too large"))?;
    if value > max {
        return Err(format!("'{name}' exceeds the supported maximum {max}"));
    }
    Ok(value)
}

pub fn execute(request: ToolCallRequest, root: Option<&str>) -> Result<Value, String> {
    execute_with_items(request, root, &mut None)
}

pub(crate) fn execute_with_items(
    request: ToolCallRequest,
    root: Option<&str>,
    listing: &mut Option<crate::output::Listing>,
) -> Result<Value, String> {
    let root = root.filter(|r| !r.trim().is_empty()).ok_or(
        "Configure an existing absolute root using -p allowed_dir=<folder> before reading",
    )?;
    let clean_root = root.replace('\\', "/");
    if !(clean_root.starts_with('/')
        || (clean_root.len() > 2 && clean_root.as_bytes()[1..3] == *b":/"))
        || !Path::new(root).is_dir()
    {
        return Err("Configured allowed_dir must be an existing absolute directory visible through the host preopen".into());
    }
    let args = &request.arguments;
    if args.get("allowedDir").is_some() || args.get("allowed_dir").is_some() {
        return Err(
            "Root overrides are not accepted in tool arguments; configure allowed_dir on the host"
                .into(),
        );
    }
    let path = args
        .get("path")
        .and_then(Value::as_str)
        .ok_or("Missing or invalid string 'path'")?;
    let relative = relative_path(path, root)?;
    match request.name.as_str() {
        "read_text_file" => {
            let page = Page::parse(args)?;
            let file = match open_file(root, &relative) {
                Ok(file) => file,
                Err(e)
                    if e.kind() != std::io::ErrorKind::NotFound
                        && directory_entries(root, &relative).is_ok() =>
                {
                    return Err(not_a_file(path));
                }
                Err(e) => return Err(open_error("open file", path, &e)),
            };
            let metadata = file
                .metadata()
                .map_err(|e| format!("Cannot inspect opened file: {e}"))?;
            if !metadata.is_file() {
                return Err(not_a_file(path));
            }
            check_size(metadata.len())?;
            let mut bytes = Vec::new();
            file.take((MAX_FILE_BYTES + 1) as u64)
                .read_to_end(&mut bytes)
                .map_err(|e| format!("Read error: {e}"))?;
            page.render(&bytes)
        }
        "list_directory" => {
            let max = number(args, "maxEntries", "max_entries", MAX_ENTRIES, MAX_ENTRIES)?;
            let entries = directory_entries(root, &relative)
                .map_err(|e| open_error("list directory", path, &e))?;
            list_result(max, entries, listing)
        }
        _ => Err("Only read_text_file and list_directory use the approved read boundary".into()),
    }
}

pub(crate) fn check_size(len: u64) -> Result<(), String> {
    if len > MAX_FILE_BYTES as u64 {
        return Err(format!(
            "Text file exceeds the {MAX_FILE_BYTES}-byte input limit; select a smaller text file"
        ));
    }
    Ok(())
}

pub(crate) struct Page {
    offset: usize,
    head: usize,
    tail: usize,
    from_end: bool,
}

impl Page {
    pub(crate) fn parse(args: &Value) -> Result<Self, String> {
        let offset = number(args, "lineOffset", "line_offset", 0, usize::MAX)?;
        let limit = number(args, "lineLimit", "line_limit", MAX_LINES, MAX_LINES)?;
        let head = number(args, "head", "head", limit, MAX_LINES)?;
        let tail = number(args, "tail", "tail", 0, MAX_LINES)?;
        if args.get("head").is_some() && args.get("tail").is_some() {
            return Err("Use either head or tail, not both".into());
        }
        Ok(Self {
            offset,
            head,
            tail,
            from_end: args.get("tail").is_some(),
        })
    }

    pub(crate) fn render(&self, bytes: &[u8]) -> Result<Value, String> {
        if bytes.len() > MAX_FILE_BYTES {
            return Err("File grew beyond the input byte limit; retry with a smaller file".into());
        }
        let text = std::str::from_utf8(bytes)
            .map_err(|e| format!("Read error: expected UTF-8 text: {e}"))?;
        let total_lines = text.lines().count();
        let (offset, limit) = if self.from_end {
            (total_lines.saturating_sub(self.tail), self.tail)
        } else {
            (self.offset, self.head)
        };
        let mut output = String::new();
        let mut returned = 0;
        for line in text.lines().skip(offset).take(limit) {
            let extra = line.len() + usize::from(returned > 0);
            if output.len() + extra > MAX_OUTPUT_BYTES {
                if returned == 0 {
                    return Err("A text line exceeds the 65536-byte output limit; choose a file with shorter lines".into());
                }
                break;
            }
            if returned > 0 {
                output.push('\n');
            }
            output.push_str(line);
            returned += 1;
        }
        let next = offset.saturating_add(returned);
        let more = next < total_lines;
        Ok(
            json!({"content":output,"lineOffset":offset,"linesReturned":returned,
            "totalLines":total_lines,"hasMore":more,"nextLineOffset":more.then_some(next),
            "maxInputBytes":MAX_FILE_BYTES,"maxOutputBytes":MAX_OUTPUT_BYTES}),
        )
    }
}

pub(crate) fn max_entries(args: &Value) -> Result<usize, String> {
    number(args, "maxEntries", "max_entries", MAX_ENTRIES, MAX_ENTRIES)
}

pub(crate) fn list_result(
    max: usize,
    entries: Entries,
    listing: &mut Option<crate::output::Listing>,
) -> Result<Value, String> {
    let mut lines = Vec::new();
    let mut bytes = 0;
    let mut more = false;
    let mut total = 0;
    for entry in entries {
        let (name, kind) = entry.map_err(|e| format!("Cannot inspect directory entry: {e}"))?;
        total += 1;
        let line = format!("[{kind}] {name}");
        let extra = line.len() + usize::from(!lines.is_empty());
        if more || lines.len() == max || bytes + extra > MAX_OUTPUT_BYTES {
            more = true;
            continue;
        }
        bytes += extra;
        lines.push(line);
    }
    lines.sort();
    let mut captured = crate::output::Listing::lines(&lines);
    captured.total = total;
    *listing = Some(captured);
    Ok(
        json!({"content":lines.join("\n"),"entriesReturned":lines.len(),
        "hasMore":more,"truncated":more,"maxEntries":max,"maxOutputBytes":MAX_OUTPUT_BYTES}),
    )
}

pub(crate) fn not_a_file(given: &str) -> String {
    format!("\"{given}\" is not a regular text file; give the name of a file, not a directory.")
}

pub(crate) fn open_error(action: &str, given: &str, error: &std::io::Error) -> String {
    let reason = if error.kind() == std::io::ErrorKind::NotFound {
        "no such file or directory".to_string()
    } else {
        error.to_string()
    };
    let hint = if ["[FILE]", "[DIR]", "[LINK]"]
        .iter()
        .any(|marker| given.trim_start().starts_with(marker))
    {
        " Use the name from list_directory without the [FILE], [DIR] or [LINK] marker."
    } else {
        ""
    };
    format!("Cannot {action} \"{given}\" within allowed_dir: {reason}.{hint}")
}

#[cfg(not(target_arch = "wasm32"))]
fn open_file(root: &str, path: &Path) -> std::io::Result<std::fs::File> {
    let dir = cap_std::fs::Dir::open_ambient_dir(root, cap_std::ambient_authority())?;
    Ok(dir.open(path)?.into_std())
}

#[cfg(target_arch = "wasm32")]
fn open_file(root: &str, path: &Path) -> std::io::Result<std::fs::File> {
    std::fs::File::open(Path::new(root).join(path))
}

pub(crate) type Entries = Box<dyn Iterator<Item = std::io::Result<(String, &'static str)>>>;

pub(crate) fn kind(directory: bool, link: bool) -> &'static str {
    if link {
        "LINK"
    } else if directory {
        "DIR"
    } else {
        "FILE"
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn directory_entries(root: &str, path: &Path) -> std::io::Result<Entries> {
    let dir =
        cap_std::fs::Dir::open_ambient_dir(root, cap_std::ambient_authority())?.open_dir(path)?;
    Ok(Box::new(dir.entries()?.map(|entry| {
        let entry = entry?;
        let ty = entry.file_type()?;
        Ok((
            entry.file_name().to_string_lossy().into_owned(),
            kind(ty.is_dir(), ty.is_symlink()),
        ))
    })))
}

#[cfg(target_arch = "wasm32")]
fn directory_entries(root: &str, path: &Path) -> std::io::Result<Entries> {
    Ok(Box::new(
        std::fs::read_dir(Path::new(root).join(path))?.map(|entry| {
            let entry = entry?;
            let ty = entry.file_type()?;
            Ok((
                entry.file_name().to_string_lossy().into_owned(),
                kind(ty.is_dir(), ty.is_symlink()),
            ))
        }),
    ))
}
