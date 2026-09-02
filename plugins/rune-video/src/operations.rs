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

fn get_u64_arg(args: &Value, camel: &str, snake: &str) -> Option<u64> {
    if let Some(val) = args
        .get(camel)
        .or_else(|| args.get(snake))
        .and_then(Value::as_u64)
    {
        return Some(val);
    }
    let env_snake = snake.to_ascii_uppercase();
    let env_camel = camel.to_ascii_uppercase();
    std::env::var(&env_snake)
        .or_else(|_| std::env::var(&env_camel))
        .ok()
        .and_then(|v| v.parse().ok())
}

fn get_bool_arg(args: &Value, camel: &str, snake: &str) -> bool {
    if let Some(val) = args
        .get(camel)
        .or_else(|| args.get(snake))
        .and_then(Value::as_bool)
    {
        return val;
    }
    let env_snake = snake.to_ascii_uppercase();
    let env_camel = camel.to_ascii_uppercase();
    std::env::var(&env_snake)
        .or_else(|_| std::env::var(&env_camel))
        .ok()
        .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
        .unwrap_or(false)
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

    let stem = if clean_domain.contains("youtu.be") || clean_domain.contains("youtube") {
        "youtube".to_string()
    } else if clean_domain.contains("twitter") || clean_domain == "x.com" {
        "twitter".to_string()
    } else if clean_domain.contains("twitch") {
        "twitch".to_string()
    } else {
        clean_domain
            .split('.')
            .next()
            .unwrap_or(&clean_domain)
            .to_string()
    };

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

fn apply_ytdlp_access_args<'a>(
    args: &mut Vec<&'a str>,
    params: &'a ToolCallRequest,
    url: &str,
    storage: &'a mut Vec<String>,
) {
    if let Some((flag, val)) = resolve_cookie_arg(params, url) {
        storage.push(flag);
        storage.push(val);
    }

    let player_client = get_str_arg(&params.arguments, "playerClient", "player_client")
        .or_else(|| get_config("player_client"));

    if let Some(client) = player_client {
        storage.push("--extractor-args".to_string());
        storage.push(format!("youtube:player_client={}", client));
    }

    let proxy = get_str_arg(&params.arguments, "proxy", "proxy").or_else(|| get_config("proxy"));

    if let Some(p) = proxy {
        storage.push("--proxy".to_string());
        storage.push(p);
    }

    for s in storage.iter() {
        args.push(s.as_str());
    }

    args.push("--geo-bypass");
}

pub fn execute_tool(request: ToolCallRequest) -> Result<Value, String> {
    let mut str_storage: Vec<String> = Vec::new();

    match request.name.as_str() {
        "inspect_video_metadata" => {
            let url = get_str_arg(&request.arguments, "url", "url")
                .ok_or_else(|| "Missing 'url' parameter".to_string())?;

            if url.trim().is_empty() {
                return Err("Parameter 'url' cannot be empty".to_string());
            }

            let mut args = vec!["-J", "--no-warnings"];
            apply_ytdlp_access_args(&mut args, &request, &url, &mut str_storage);
            args.push(&url);

            let out = run_binary("yt-dlp", &args, None)?;
            let parsed: Value = serde_json::from_str(&out)
                .map_err(|e| format!("Failed to parse yt-dlp JSON: {}", e))?;

            let summary = json!({
                "title": parsed["title"],
                "channel": parsed["uploader"],
                "duration_seconds": parsed["duration"],
                "upload_date": parsed["upload_date"],
                "view_count": parsed["view_count"],
                "thumbnail": parsed["thumbnail"],
                "age_limit": parsed["age_limit"],
                "available_formats": parsed["formats"].as_array().map(|arr| {
                    arr.iter().map(|f| json!({
                        "format_id": f["format_id"],
                        "resolution": f["resolution"],
                        "ext": f["ext"],
                        "vcodec": f["vcodec"],
                        "acodec": f["acodec"]
                    })).collect::<Vec<_>>()
                })
            });

            Ok(json!({ "metadata": summary }))
        }

        "download_video_stream" => {
            let url = get_str_arg(&request.arguments, "url", "url")
                .ok_or_else(|| "Missing 'url' parameter".to_string())?;

            if url.trim().is_empty() {
                return Err("Parameter 'url' cannot be empty".to_string());
            }

            let output_dir_param =
                get_str_arg(&request.arguments, "outputDirectory", "output_directory");
            let output_dir = resolve_dir(output_dir_param.as_deref());
            let _ = fs::create_dir_all(&output_dir);

            let max_res = get_str_arg(&request.arguments, "maxResolution", "max_resolution")
                .unwrap_or_else(|| "1080p".to_string());
            let write_subs = get_bool_arg(&request.arguments, "writeSubtitles", "write_subtitles");
            let sub_lang = get_str_arg(&request.arguments, "subtitlesLang", "subtitles_lang")
                .unwrap_or_else(|| "en".to_string());

            let height = max_res.replace('p', "");
            let format_str = format!(
                "bestvideo[height<={}]+bestaudio/best[height<={}]/best",
                height, height
            );

            let mut args = vec![
                "-f",
                &format_str,
                "--merge-output-format",
                "mp4",
                "-P",
                &output_dir,
                "--no-mtime",
            ];

            apply_ytdlp_access_args(&mut args, &request, &url, &mut str_storage);

            if write_subs {
                args.push("--write-subs");
                args.push("--sub-langs");
                args.push(&sub_lang);
            }

            args.push(&url);

            let out = run_binary("yt-dlp", &args, None)?;
            Ok(json!({ "status": "success", "output": out }))
        }

        "download_video_playlist" => {
            let url = get_str_arg(&request.arguments, "url", "url")
                .ok_or_else(|| "Missing 'url' parameter".to_string())?;

            if url.trim().is_empty() {
                return Err("Parameter 'url' cannot be empty".to_string());
            }

            let output_dir_param =
                get_str_arg(&request.arguments, "outputDirectory", "output_directory");
            let output_dir = resolve_dir(output_dir_param.as_deref());
            let _ = fs::create_dir_all(&output_dir);

            let start = get_u64_arg(&request.arguments, "startIndex", "start_index")
                .unwrap_or(1)
                .to_string();
            let end =
                get_u64_arg(&request.arguments, "endIndex", "end_index").map(|v| v.to_string());
            let max =
                get_u64_arg(&request.arguments, "maxVideos", "max_videos").map(|v| v.to_string());

            let mut args = vec![
                "-P",
                &output_dir,
                "--playlist-start",
                &start,
                "-o",
                "%(playlist_index)s - %(title)s.%(ext)s",
            ];

            apply_ytdlp_access_args(&mut args, &request, &url, &mut str_storage);

            if let Some(ref e) = end {
                args.push("--playlist-end");
                args.push(e);
            }
            if let Some(ref m) = max {
                args.push("--max-downloads");
                args.push(m);
            }

            args.push(&url);

            let out = run_binary("yt-dlp", &args, None)?;
            Ok(json!({ "status": "success", "output": out }))
        }

        "record_live_stream" => {
            let url = get_str_arg(&request.arguments, "url", "url")
                .ok_or_else(|| "Missing 'url' parameter".to_string())?;

            if url.trim().is_empty() {
                return Err("Parameter 'url' cannot be empty".to_string());
            }

            let output_dir_param =
                get_str_arg(&request.arguments, "outputDirectory", "output_directory");
            let output_dir = resolve_dir(output_dir_param.as_deref());
            let _ = fs::create_dir_all(&output_dir);

            let quality = get_str_arg(&request.arguments, "quality", "quality")
                .unwrap_or_else(|| "best".to_string());
            let duration_min =
                get_u64_arg(&request.arguments, "durationMinutes", "duration_minutes").unwrap_or(5);

            let output_filename = format!(
                "{}/live_stream_{}.mp4",
                output_dir,
                chrono::Utc::now().timestamp()
            );
            let duration_sec = (duration_min * 60).to_string();

            let args = [
                "--hls-duration",
                &duration_sec,
                "-o",
                &output_filename,
                &url,
                &quality,
            ];

            let out = run_binary("streamlink", &args, None)?;
            Ok(json!({ "status": "success", "saved_file": output_filename, "output": out }))
        }

        "trim_media_clip" => {
            let input_file = get_str_arg(&request.arguments, "inputFile", "input_file")
                .ok_or_else(|| "Missing 'inputFile' parameter".to_string())?;
            let start = get_str_arg(&request.arguments, "startTime", "start_time")
                .ok_or_else(|| "Missing 'startTime' parameter".to_string())?;
            let end = get_str_arg(&request.arguments, "endTime", "end_time")
                .ok_or_else(|| "Missing 'endTime' parameter".to_string())?;

            let lossless = get_bool_arg(&request.arguments, "lossless", "lossless");

            let output_file = get_str_arg(&request.arguments, "outputFile", "output_file")
                .unwrap_or_else(|| {
                    let p = Path::new(&input_file);
                    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
                    let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("mp4");
                    format!("{}_trimmed.{}", stem, ext)
                });

            if let Some(parent) = Path::new(&output_file).parent() {
                if !parent.as_os_str().is_empty() {
                    let _ = fs::create_dir_all(parent);
                }
            }

            let mut args = vec!["-y", "-ss", &start, "-to", &end, "-i", &input_file];

            if lossless {
                args.extend_from_slice(&["-c", "copy"]);
            }

            args.push(&output_file);

            let out = run_binary("ffmpeg", &args, None)?;
            Ok(json!({ "status": "success", "trimmed_file": output_file, "ffmpeg_log": out }))
        }

        unknown => Err(format!("Unknown tool: {}", unknown)),
    }
}
