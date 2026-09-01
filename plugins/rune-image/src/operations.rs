// plugins/rune-image/src/operations.rs
use crate::types::{CmdExecRequest, CmdExecResponse};
use rune_pdk::ToolCallRequest;
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(target_arch = "wasm32")]
#[extism_pdk::host_fn("extism:host/user")]
extern "ExtismHost" {
    fn host_cmd_exec(input: String) -> String;
}

fn get_str_arg(args: &Value, camel: &str, snake: &str) -> Option<String> {
    if let Some(val) = args
        .get(camel)
        .or_else(|| args.get(snake))
        .and_then(Value::as_str)
    {
        return Some(val.to_string());
    }
    let env_snake = snake.to_ascii_uppercase();
    let env_camel = camel.to_ascii_uppercase();
    std::env::var(&env_snake)
        .or_else(|_| std::env::var(&env_camel))
        .or_else(|_| {
            if snake == "output_directory" {
                std::env::var("OUTPUT_DIR").or_else(|_| std::env::var("ALLOWED_DIR"))
            } else {
                Err(std::env::VarError::NotPresent)
            }
        })
        .ok()
}

pub fn get_config(key: &str) -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        extism_pdk::config::get(key)
            .ok()
            .flatten()
            .or_else(|| std::env::var(key.to_ascii_uppercase()).ok())
            .filter(|s| !s.is_empty())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::env::var(key.to_ascii_uppercase())
            .or_else(|_| std::env::var(key))
            .ok()
            .filter(|s| !s.is_empty())
    }
}

#[cfg(target_arch = "wasm32")]
fn run_binary_raw(req: &CmdExecRequest) -> Result<CmdExecResponse, String> {
    let raw_req = serde_json::to_string(req).map_err(|e| e.to_string())?;
    let raw_resp =
        unsafe { host_cmd_exec(raw_req) }.map_err(|e| format!("Host execution failed: {:?}", e))?;
    serde_json::from_str(&raw_resp).map_err(|e| format!("Failed to parse host response: {}", e))
}

#[cfg(not(target_arch = "wasm32"))]
fn run_binary_raw(req: &CmdExecRequest) -> Result<CmdExecResponse, String> {
    let mut cmd = std::process::Command::new(&req.program);
    cmd.args(&req.args);
    if let Some(ref cwd) = req.cwd {
        cmd.current_dir(cwd);
    }
    match cmd.output() {
        Ok(output) => Ok(CmdExecResponse {
            success: output.status.success(),
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        }),
        Err(e) => Ok(CmdExecResponse {
            success: false,
            exit_code: None,
            stdout: String::new(),
            stderr: format!("Failed to spawn {}: {}", req.program, e),
        }),
    }
}

pub fn run_binary(program: &str, args: &[&str], cwd: Option<&str>) -> Result<String, String> {
    let req = CmdExecRequest {
        program: program.to_string(),
        args: args.iter().map(|s| s.to_string()).collect(),
        cwd: cwd.map(|c| c.to_string()),
    };

    let resp = run_binary_raw(&req)?;

    if resp.success {
        Ok(resp.stdout.trim().to_string())
    } else {
        let err = if !resp.stderr.trim().is_empty() {
            resp.stderr.trim()
        } else if !resp.stdout.trim().is_empty() {
            resp.stdout.trim()
        } else {
            "Process exited with non-zero exit code"
        };
        Err(format!("Error executing '{}': {}", program, err))
    }
}

pub fn resolve_dir(dir_param: Option<&str>) -> String {
    let explicit = dir_param.map(ToString::to_string).or_else(|| {
        std::env::var("OUTPUT_DIRECTORY")
            .or_else(|_| std::env::var("OUTPUT_DIR"))
            .or_else(|_| std::env::var("ALLOWED_DIR"))
            .ok()
    });

    let raw = explicit.unwrap_or_else(|| ".".to_string());
    let target = PathBuf::from(raw);

    if let Some(allowed_root) = get_config("allowed_dir") {
        let root = PathBuf::from(allowed_root);
        if target.is_relative() {
            root.join(target).to_string_lossy().to_string()
        } else {
            target.to_string_lossy().to_string()
        }
    } else {
        target.to_string_lossy().to_string()
    }
}

pub fn extract_domain_and_stem(url: &str) -> (String, String) {
    let without_proto = url
        .trim()
        .strip_prefix("https://")
        .or_else(|| url.trim().strip_prefix("http://"))
        .unwrap_or(url.trim());

    let host = without_proto
        .split(['/', '?', '#', ':'])
        .next()
        .unwrap_or("")
        .to_lowercase();

    let clean_domain = host.trim_start_matches("www.").to_string();
    let stem = clean_domain
        .split('.')
        .next()
        .unwrap_or(&clean_domain)
        .to_string();

    (clean_domain, stem)
}

pub fn find_cookie_file_in_dir(dir_path: &Path, url: &str) -> Option<PathBuf> {
    if !dir_path.exists() || !dir_path.is_dir() {
        return None;
    }

    let (domain, stem) = extract_domain_and_stem(url);
    let candidates = [
        format!("{}.txt", domain),
        format!("{}.txt", stem),
        format!("{}_cookies.txt", domain),
        format!("{}_cookies.txt", stem),
        format!("{}-cookies.txt", stem),
        format!("cookies-{}.txt", stem),
    ];

    for candidate in &candidates {
        let path = dir_path.join(candidate);
        if path.is_file() {
            return Some(path);
        }
    }

    if let Ok(entries) = fs::read_dir(dir_path) {
        for entry in entries.flatten() {
            if let Ok(ft) = entry.file_type() {
                if ft.is_file() {
                    let name = entry.file_name().to_string_lossy().to_lowercase();
                    if name.ends_with(".txt") && (name.contains(&stem) || name.contains(&domain)) {
                        return Some(entry.path());
                    }
                }
            }
        }
    }

    let default_cookie = dir_path.join("cookies.txt");
    if default_cookie.is_file() {
        return Some(default_cookie);
    }

    None
}

// plugins/rune-image/src/operations.rs

pub fn resolve_cookie_arg(params: &ToolCallRequest, url: &str) -> Option<(String, String)> {
    let explicit_file = get_str_arg(&params.arguments, "cookiesFile", "cookies_file");
    if let Some(file) = explicit_file {
        return Some(("--cookies".to_string(), file));
    }

    let cookies_dir = get_str_arg(&params.arguments, "cookiesDir", "cookies_dir");
    if let Some(dir_str) = cookies_dir {
        let dir_path = PathBuf::from(resolve_dir(Some(&dir_str)));
        if let Some(matched_file) = find_cookie_file_in_dir(&dir_path, url) {
            return Some((
                "--cookies".to_string(),
                matched_file.to_string_lossy().to_string(),
            ));
        }
    }

    let browser = get_str_arg(
        &params.arguments,
        "cookiesFromBrowser",
        "cookies_from_browser",
    );
    if let Some(b) = browser {
        return Some(("--cookies-from-browser".to_string(), b));
    }

    None
}

fn apply_gallerydl_access_args<'a>(
    args: &mut Vec<&'a str>,
    params: &'a ToolCallRequest,
    url: &str,
    storage: &'a mut Vec<String>,
) {
    if let Some((flag, val)) = resolve_cookie_arg(params, url) {
        storage.push(flag);
        storage.push(val);
    }

    if let Some(p) = get_str_arg(&params.arguments, "proxy", "proxy") {
        storage.push("--proxy".to_string());
        storage.push(p);
    }

    for s in storage.iter() {
        args.push(s.as_str());
    }
}

fn parse_gallerydl_output(raw_output: &str) -> Vec<String> {
    let mut urls = Vec::new();

    if let Ok(root) = serde_json::from_str::<Value>(raw_output) {
        collect_urls_from_value(&root, &mut urls);
    } else {
        let stream = serde_json::Deserializer::from_str(raw_output).into_iter::<Value>();
        for val in stream.flatten() {
            collect_urls_from_value(&val, &mut urls);
        }
    }

    let mut deduped = Vec::new();
    for u in urls {
        if !deduped.contains(&u) {
            deduped.push(u);
        }
    }
    deduped
}

fn collect_urls_from_value(val: &Value, urls: &mut Vec<String>) {
    if let Some(arr) = val.as_array() {
        if !arr.is_empty() && arr[0].is_number() {
            if let Some(s) = arr.get(1).and_then(Value::as_str) {
                if s.starts_with("http://") || s.starts_with("https://") {
                    urls.push(s.to_string());
                }
            }
        } else {
            for elem in arr {
                collect_urls_from_value(elem, urls);
            }
        }
    } else if let Some(obj) = val.as_object() {
        for key in ["url", "file_url", "image", "preview_url", "src"] {
            if let Some(s) = obj.get(key).and_then(Value::as_str) {
                if s.starts_with("http://") || s.starts_with("https://") {
                    urls.push(s.to_string());
                    break;
                }
            }
        }
    }
}

pub fn execute_tool(request: ToolCallRequest) -> Result<Value, String> {
    let mut str_storage: Vec<String> = Vec::new();

    match request.name.as_str() {
        "inspect_image_gallery" => {
            let url = get_str_arg(&request.arguments, "url", "url")
                .ok_or_else(|| "Missing 'url' parameter".to_string())?;

            let mut args = vec!["--dump-json"];
            apply_gallerydl_access_args(&mut args, &request, &url, &mut str_storage);
            args.push(&url);

            let out = run_binary("gallery-dl", &args, None)?;
            let previews = parse_gallerydl_output(&out);

            let count = previews.len();
            Ok(json!({
                "gallery_url": url,
                "total_media_found": count,
                "previews": previews.into_iter().take(5).collect::<Vec<_>>()
            }))
        }

        "download_image_collection" => {
            let url = get_str_arg(&request.arguments, "url", "url")
                .ok_or_else(|| "Missing 'url' parameter".to_string())?;

            if url.trim().is_empty() {
                return Err("Parameter 'url' cannot be empty".to_string());
            }

            let output_dir_param =
                get_str_arg(&request.arguments, "outputDirectory", "output_directory");
            let output_dir = resolve_dir(output_dir_param.as_deref());

            let _ = fs::create_dir_all(&output_dir);

            let range = get_str_arg(&request.arguments, "filterRange", "filter_range");

            let mut args = vec!["--directory", &output_dir];
            apply_gallerydl_access_args(&mut args, &request, &url, &mut str_storage);

            if let Some(ref r) = range {
                args.push("--range");
                args.push(r);
            }
            args.push(&url);

            let out = run_binary("gallery-dl", &args, None)?;
            Ok(json!({ "status": "success", "output": out }))
        }

        unknown => Err(format!("Unknown tool: {}", unknown)),
    }
}
