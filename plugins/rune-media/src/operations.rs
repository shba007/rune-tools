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

#[cfg(target_arch = "wasm32")]
fn get_config(key: &str) -> Option<String> {
    extism_pdk::config::get(key)
        .ok()
        .flatten()
        .filter(|s| !s.is_empty())
}

#[cfg(not(target_arch = "wasm32"))]
fn get_config(key: &str) -> Option<String> {
    std::env::var(key.to_uppercase())
        .or_else(|_| std::env::var(key))
        .ok()
        .filter(|s| !s.is_empty())
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
    let raw = dir_param.unwrap_or(".");
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
    let explicit_file = params
        .arguments
        .get("cookiesFile")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| get_config("cookies_file"));

    if let Some(file) = explicit_file {
        return Some(("--cookies".to_string(), file));
    }

    let cookies_dir = params
        .arguments
        .get("cookiesDir")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| get_config("cookies_dir"));

    if let Some(dir_str) = cookies_dir {
        let dir_path = PathBuf::from(resolve_dir(Some(&dir_str)));
        if let Some(matched_file) = find_cookie_file_in_dir(&dir_path, url) {
            return Some((
                "--cookies".to_string(),
                matched_file.to_string_lossy().to_string(),
            ));
        }
    }

    let browser = params
        .arguments
        .get("cookiesFromBrowser")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| get_config("cookies_from_browser"));

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

    let player_client = params
        .arguments
        .get("playerClient")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| get_config("player_client"));

    if let Some(client) = player_client {
        storage.push("--extractor-args".to_string());
        storage.push(format!("youtube:player_client={}", client));
    }

    let proxy = params
        .arguments
        .get("proxy")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| get_config("proxy"));

    if let Some(p) = proxy {
        storage.push("--proxy".to_string());
        storage.push(p);
    }

    for s in storage.iter() {
        args.push(s.as_str());
    }

    args.push("--geo-bypass");
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

    let proxy = params
        .arguments
        .get("proxy")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| get_config("proxy"));

    if let Some(p) = proxy {
        storage.push("--proxy".to_string());
        storage.push(p);
    }

    for s in storage.iter() {
        args.push(s.as_str());
    }
}

pub fn execute_tool(request: ToolCallRequest) -> Result<Value, String> {
    let mut str_storage: Vec<String> = Vec::new();

    match request.name.as_str() {
        "verify_downloader_environment" => {
            let tools: [(&str, &[&str]); 6] = [
                ("yt-dlp", &["--version"]),
                ("gallery-dl", &["--version"]),
                ("ffmpeg", &["-version"]),
                ("aria2c", &["--version"]),
                ("streamlink", &["--version"]),
                ("spotdl", &["--version"]),
            ];
            let mut status_map = json!({});

            for (bin, args) in tools {
                match run_binary(bin, args, None) {
                    Ok(out) => {
                        let ver = out.lines().next().unwrap_or("Installed").trim().to_string();
                        status_map[bin] = json!({
                            "installed": true,
                            "version": ver
                        });
                    }
                    Err(_) => {
                        status_map[bin] = json!({
                            "installed": false,
                            "version": null
                        });
                    }
                }
            }

            Ok(json!({ "environment": status_map }))
        }
        "inspect_video_metadata" => {
            let url = request
                .arguments
                .get("url")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'url' parameter".to_string())?;

            if url.trim().is_empty() {
                return Err("Parameter 'url' cannot be empty".to_string());
            }

            let mut args = vec!["-J", "--no-warnings"];
            apply_ytdlp_access_args(&mut args, &request, url, &mut str_storage);
            args.push(url);

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
            let url = request
                .arguments
                .get("url")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'url' parameter".to_string())?;

            if url.trim().is_empty() {
                return Err("Parameter 'url' cannot be empty".to_string());
            }

            let output_dir = resolve_dir(
                request
                    .arguments
                    .get("outputDirectory")
                    .and_then(|v| v.as_str()),
            );
            let max_res = request
                .arguments
                .get("maxResolution")
                .and_then(|v| v.as_str())
                .unwrap_or("1080p");
            let write_subs = request
                .arguments
                .get("writeSubtitles")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let sub_lang = request
                .arguments
                .get("subtitlesLang")
                .and_then(|v| v.as_str())
                .unwrap_or("en");

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

            apply_ytdlp_access_args(&mut args, &request, url, &mut str_storage);

            if write_subs {
                args.push("--write-subs");
                args.push("--sub-langs");
                args.push(sub_lang);
            }

            args.push(url);

            let out = run_binary("yt-dlp", &args, None)?;
            Ok(json!({ "status": "success", "output": out }))
        }

        "download_video_playlist" => {
            let url = request
                .arguments
                .get("url")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'url' parameter".to_string())?;

            if url.trim().is_empty() {
                return Err("Parameter 'url' cannot be empty".to_string());
            }

            let output_dir = resolve_dir(
                request
                    .arguments
                    .get("outputDirectory")
                    .and_then(|v| v.as_str()),
            );
            let start = request
                .arguments
                .get("startIndex")
                .and_then(|v| v.as_u64())
                .unwrap_or(1)
                .to_string();
            let end = request
                .arguments
                .get("endIndex")
                .and_then(|v| v.as_u64())
                .map(|v| v.to_string());
            let max = request
                .arguments
                .get("maxVideos")
                .and_then(|v| v.as_u64())
                .map(|v| v.to_string());

            let mut args = vec![
                "-P",
                &output_dir,
                "--playlist-start",
                &start,
                "-o",
                "%(playlist_index)s - %(title)s.%(ext)s",
            ];

            apply_ytdlp_access_args(&mut args, &request, url, &mut str_storage);

            if let Some(e) = &end {
                args.push("--playlist-end");
                args.push(e);
            }
            if let Some(m) = &max {
                args.push("--max-downloads");
                args.push(m);
            }

            args.push(url);

            let out = run_binary("yt-dlp", &args, None)?;
            Ok(json!({ "status": "success", "output": out }))
        }

        "record_live_stream" => {
            let url = request
                .arguments
                .get("url")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'url' parameter".to_string())?;

            if url.trim().is_empty() {
                return Err("Parameter 'url' cannot be empty".to_string());
            }

            let output_dir = resolve_dir(
                request
                    .arguments
                    .get("outputDirectory")
                    .and_then(|v| v.as_str()),
            );
            let quality = request
                .arguments
                .get("quality")
                .and_then(|v| v.as_str())
                .unwrap_or("best");
            let duration_min = request
                .arguments
                .get("durationMinutes")
                .and_then(|v| v.as_u64())
                .unwrap_or(5);

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
                url,
                quality,
            ];

            let out = run_binary("streamlink", &args, None)?;
            Ok(json!({ "status": "success", "saved_file": output_filename, "output": out }))
        }

        "extract_audio_track" => {
            let url = request
                .arguments
                .get("url")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'url' parameter".to_string())?;

            if url.trim().is_empty() {
                return Err("Parameter 'url' cannot be empty".to_string());
            }

            let output_dir = resolve_dir(
                request
                    .arguments
                    .get("outputDirectory")
                    .and_then(|v| v.as_str()),
            );
            let format = request
                .arguments
                .get("audioFormat")
                .and_then(|v| v.as_str())
                .unwrap_or("mp3");
            let quality = request
                .arguments
                .get("audioQuality")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
                .to_string();

            let mut args = vec![
                "-x",
                "--audio-format",
                format,
                "--audio-quality",
                &quality,
                "--embed-metadata",
                "--embed-thumbnail",
                "-P",
                &output_dir,
            ];

            apply_ytdlp_access_args(&mut args, &request, url, &mut str_storage);
            args.push(url);

            let out = run_binary("yt-dlp", &args, None)?;
            Ok(json!({ "status": "success", "output": out }))
        }

        "download_music_track" => {
            let url = request
                .arguments
                .get("url")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'url' parameter".to_string())?;

            if url.trim().is_empty() {
                return Err("Parameter 'url' cannot be empty".to_string());
            }

            let output_dir = resolve_dir(
                request
                    .arguments
                    .get("outputDirectory")
                    .and_then(|v| v.as_str()),
            );
            let include_lyrics = request
                .arguments
                .get("includeLyrics")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let mut args = vec!["download", url, "--output", &output_dir];
            if include_lyrics {
                args.push("--generate-lrc");
            }

            let out = run_binary("spotdl", &args, None)?;
            Ok(json!({ "status": "success", "output": out }))
        }

        "inspect_image_gallery" => {
            let url = request
                .arguments
                .get("url")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'url' parameter".to_string())?;

            if url.trim().is_empty() {
                return Err("Parameter 'url' cannot be empty".to_string());
            }

            let mut args = vec!["--dump-json"];
            apply_gallerydl_access_args(&mut args, &request, url, &mut str_storage);
            args.push(url);

            let out = run_binary("gallery-dl", &args, None)?;
            let mut count = 0;
            let mut previews = Vec::new();

            for line in out.lines() {
                if let Ok(item) = serde_json::from_str::<Value>(line) {
                    if item.is_array() && item.as_array().map(|a| a.len()).unwrap_or(0) >= 2 {
                        let url_str = item[1].as_str().unwrap_or("");
                        previews.push(url_str.to_string());
                        count += 1;
                    }
                }
            }

            Ok(json!({
                "gallery_url": url,
                "total_media_found": count,
                "previews": previews.iter().take(5).collect::<Vec<_>>()
            }))
        }

        "download_image_collection" => {
            let url = request
                .arguments
                .get("url")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'url' parameter".to_string())?;

            if url.trim().is_empty() {
                return Err("Parameter 'url' cannot be empty".to_string());
            }

            let output_dir = resolve_dir(
                request
                    .arguments
                    .get("outputDirectory")
                    .and_then(|v| v.as_str()),
            );
            let range = request
                .arguments
                .get("filterRange")
                .and_then(|v| v.as_str());

            let mut args = vec!["--directory", &output_dir];
            apply_gallerydl_access_args(&mut args, &request, url, &mut str_storage);

            if let Some(r) = range {
                args.push("--range");
                args.push(r);
            }
            args.push(url);

            let out = run_binary("gallery-dl", &args, None)?;
            Ok(json!({ "status": "success", "output": out }))
        }

        "download_direct_file" => {
            let url = request
                .arguments
                .get("url")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'url' parameter".to_string())?;

            if url.trim().is_empty() {
                return Err("Parameter 'url' cannot be empty".to_string());
            }

            let output_dir = resolve_dir(
                request
                    .arguments
                    .get("outputDirectory")
                    .and_then(|v| v.as_str()),
            );
            let connections = request
                .arguments
                .get("connectionsPerServer")
                .and_then(|v| v.as_u64())
                .unwrap_or(8)
                .to_string();
            let filename = request
                .arguments
                .get("outputFilename")
                .and_then(|v| v.as_str());

            let mut args = vec![
                "-x",
                &connections,
                "-s",
                &connections,
                "-d",
                &output_dir,
                "--auto-file-renaming=false",
                "--allow-overwrite=true",
            ];

            if let Some(f) = filename {
                args.push("-o");
                args.push(f);
            }
            args.push(url);

            let out = run_binary("aria2c", &args, None)?;
            Ok(json!({ "status": "success", "output": out }))
        }

        "download_torrent_magnet" => {
            let uri = request
                .arguments
                .get("uri")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'uri' parameter".to_string())?;

            if uri.trim().is_empty() {
                return Err("Parameter 'uri' cannot be empty".to_string());
            }

            let output_dir = resolve_dir(
                request
                    .arguments
                    .get("outputDirectory")
                    .and_then(|v| v.as_str()),
            );
            let speed_limit = request
                .arguments
                .get("maxDownloadSpeed")
                .and_then(|v| v.as_str());

            let mut args = vec!["-d", &output_dir, "--seed-time=0", "--bt-stop-timeout=30"];

            let limit_flag;
            if let Some(limit) = speed_limit {
                limit_flag = format!("--max-download-limit={}", limit);
                args.push(&limit_flag);
            }
            args.push(uri);

            let out = run_binary("aria2c", &args, None)?;
            Ok(json!({ "status": "success", "output": out }))
        }

        "trim_media_clip" => {
            let input_file = request
                .arguments
                .get("inputFile")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'inputFile' parameter".to_string())?;
            let start = request
                .arguments
                .get("startTime")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'startTime' parameter".to_string())?;
            let end = request
                .arguments
                .get("endTime")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'endTime' parameter".to_string())?;

            let lossless = request
                .arguments
                .get("lossless")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            let output_file = request
                .arguments
                .get("outputFile")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    let p = Path::new(input_file);
                    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
                    let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("mp4");
                    format!("{}_trimmed.{}", stem, ext)
                });

            let mut args = vec!["-y", "-ss", start, "-to", end, "-i", input_file];

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
