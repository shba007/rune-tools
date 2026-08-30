// plugins/rune-media/src/lib.rs
use extism_pdk::*;
use rune_pdk::{ToolCallRequest, ToolDefinition};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};

#[host_fn("extism:host/user")]
extern "ExtismHost" {
    fn host_cmd_exec(input: String) -> String;
}

#[derive(Serialize, Deserialize)]
struct CmdExecRequest {
    program: String,
    args: Vec<String>,
    cwd: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct CmdExecResponse {
    success: bool,
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
}

fn resolve_dir(dir_param: Option<&str>) -> String {
    let raw = dir_param.unwrap_or(".");
    let target = PathBuf::from(raw);

    if let Ok(Some(allowed_root)) = config::get("allowed_dir") {
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

fn run_binary(program: &str, args: &[&str], cwd: Option<&str>) -> Result<String, String> {
    let req = CmdExecRequest {
        program: program.to_string(),
        args: args.iter().map(|s| s.to_string()).collect(),
        cwd: cwd.map(|c| c.to_string()),
    };

    let raw_req = serde_json::to_string(&req).map_err(|e| e.to_string())?;
    let raw_resp =
        unsafe { host_cmd_exec(raw_req) }.map_err(|e| format!("Host execution failed: {:?}", e))?;

    let resp: CmdExecResponse = serde_json::from_str(&raw_resp)
        .map_err(|e| format!("Failed to parse host response: {}", e))?;

    if resp.success {
        Ok(resp.stdout.trim().to_string())
    } else {
        let err = if !resp.stderr.trim().is_empty() {
            resp.stderr.trim()
        } else {
            resp.stdout.trim()
        };
        Err(format!("Error executing '{}': {}", program, err))
    }
}

/// Extracts domain & stem from URL (e.g. "https://www.youtube.com/watch" -> "youtube.com", "youtube")
fn extract_domain_and_stem(url: &str) -> (String, String) {
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

/// Scans a directory for the best matching cookie file for the target site
fn find_cookie_file_in_dir(dir_path: &Path, url: &str) -> Option<PathBuf> {
    if !dir_path.exists() || !dir_path.is_dir() {
        return None;
    }

    let (domain, stem) = extract_domain_and_stem(url);

    // 1. Direct candidate matches
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

    // 2. Substring scan (e.g. "my_youtube_account.txt")
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

    // 3. Fallback to generic cookies.txt in the folder
    let default_cookie = dir_path.join("cookies.txt");
    if default_cookie.is_file() {
        return Some(default_cookie);
    }

    None
}

/// Resolves cookie configuration from explicit file, cookies directory, or active browser
fn resolve_cookie_arg(params: &ToolCallRequest, url: &str) -> Option<(String, String)> {
    // 1. Explicit file parameter or config
    let explicit_file = params
        .arguments
        .get("cookiesFile")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            config::get("cookies_file")
                .ok()
                .flatten()
                .filter(|s| !s.is_empty())
        });

    if let Some(file) = explicit_file {
        return Some(("--cookies".to_string(), file));
    }

    // 2. Cookies directory auto-matching
    let cookies_dir = params
        .arguments
        .get("cookiesDir")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            config::get("cookies_dir")
                .ok()
                .flatten()
                .filter(|s| !s.is_empty())
        });

    if let Some(dir_str) = cookies_dir {
        let dir_path = PathBuf::from(resolve_dir(Some(&dir_str)));
        if let Some(matched_file) = find_cookie_file_in_dir(&dir_path, url) {
            return Some((
                "--cookies".to_string(),
                matched_file.to_string_lossy().to_string(),
            ));
        }
    }

    // 3. Browser session extraction
    let browser = params
        .arguments
        .get("cookiesFromBrowser")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            config::get("cookies_from_browser")
                .ok()
                .flatten()
                .filter(|s| !s.is_empty())
        });

    if let Some(b) = browser {
        return Some(("--cookies-from-browser".to_string(), b));
    }

    None
}

/// Injects cookies, extractor args, proxies, and geo-bypass into yt-dlp
///
/// `storage` owns every String backing an arg pushed here. This is written in
/// two phases: first every dynamic flag/value is pushed into `storage` as an
/// owned String with no reads in between, and only once `storage` is fully
/// built (and won't be mutated again in this call) do we borrow it to hand
/// out `&'a str`s into `args`. Interleaving pushes with reads doesn't work:
/// a `&'a str` borrowed from `storage` has to stay valid for the whole
/// function (since it ends up living in the caller's `args`), so any later
/// `storage.push(...)` — which could reallocate — would conflict with that
/// still-live borrow.
fn apply_ytdlp_access_args<'a>(
    args: &mut Vec<&'a str>,
    params: &'a ToolCallRequest,
    url: &str,
    storage: &'a mut Vec<String>,
) {
    // Phase 1: build every dynamic flag/value pair as owned Strings.
    if let Some((flag, val)) = resolve_cookie_arg(params, url) {
        storage.push(flag);
        storage.push(val);
    }

    let player_client = params
        .arguments
        .get("playerClient")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            config::get("player_client")
                .ok()
                .flatten()
                .filter(|s| !s.is_empty())
        });

    if let Some(client) = player_client {
        storage.push("--extractor-args".to_string());
        storage.push(format!("youtube:player_client={}", client));
    }

    let proxy = params
        .arguments
        .get("proxy")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            config::get("proxy")
                .ok()
                .flatten()
                .filter(|s| !s.is_empty())
        });

    if let Some(p) = proxy {
        storage.push("--proxy".to_string());
        storage.push(p);
    }

    // Phase 2: storage is fully populated and won't be touched again in this
    // call, so a single borrow to hand out &str slices is safe.
    for s in storage.iter() {
        args.push(s.as_str());
    }

    args.push("--geo-bypass");
}

/// Injects cookies and proxies into gallery-dl
///
/// See `apply_ytdlp_access_args` for why the build-then-borrow phasing matters.
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
        .or_else(|| {
            config::get("proxy")
                .ok()
                .flatten()
                .filter(|s| !s.is_empty())
        });

    if let Some(p) = proxy {
        storage.push("--proxy".to_string());
        storage.push(p);
    }

    for s in storage.iter() {
        args.push(s.as_str());
    }
}

fn execute_tool(request: ToolCallRequest) -> Result<Value, String> {
    let mut str_storage: Vec<String> = Vec::new();

    match request.name.as_str() {
        "verify_downloader_environment" => {
            let binaries = [
                "yt-dlp",
                "gallery-dl",
                "ffmpeg",
                "aria2c",
                "streamlink",
                "spotdl",
            ];
            let mut status_map = json!({});

            for bin in binaries {
                match run_binary(bin, &["--version"], None) {
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
            let url = request.arguments["url"]
                .as_str()
                .ok_or_else(|| "Missing 'url' parameter".to_string())?;

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
            let url = request.arguments["url"]
                .as_str()
                .ok_or_else(|| "Missing 'url' parameter".to_string())?;
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
            let url = request.arguments["url"]
                .as_str()
                .ok_or_else(|| "Missing 'url' parameter".to_string())?;
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
            let url = request.arguments["url"]
                .as_str()
                .ok_or_else(|| "Missing 'url' parameter".to_string())?;
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
            let url = request.arguments["url"]
                .as_str()
                .ok_or_else(|| "Missing 'url' parameter".to_string())?;
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
            let url = request.arguments["url"]
                .as_str()
                .ok_or_else(|| "Missing 'url' parameter".to_string())?;
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
            let url = request.arguments["url"]
                .as_str()
                .ok_or_else(|| "Missing 'url' parameter".to_string())?;

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
            let url = request.arguments["url"]
                .as_str()
                .ok_or_else(|| "Missing 'url' parameter".to_string())?;
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
            let url = request.arguments["url"]
                .as_str()
                .ok_or_else(|| "Missing 'url' parameter".to_string())?;
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
            let uri = request.arguments["uri"]
                .as_str()
                .ok_or_else(|| "Missing 'uri' parameter".to_string())?;
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
            let input_file = request.arguments["inputFile"]
                .as_str()
                .ok_or_else(|| "Missing 'inputFile' parameter".to_string())?;
            let start = request.arguments["startTime"]
                .as_str()
                .ok_or_else(|| "Missing 'startTime' parameter".to_string())?;
            let end = request.arguments["endTime"]
                .as_str()
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
                    let p = std::path::Path::new(input_file);
                    let stem = p.file_stem().unwrap().to_str().unwrap();
                    let ext = p.extension().unwrap_or_default().to_str().unwrap_or("mp4");
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

        unknown => Err(format!("Unknown media tool: {}", unknown)),
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
            name: "verify_downloader_environment".to_string(),
            description: "Probes host system for yt-dlp, gallery-dl, ffmpeg, aria2c, streamlink, and spotdl.".to_string(),
            input_schema: json!({ "type": "object", "properties": {} }),
        },
        ToolDefinition {
            name: "inspect_video_metadata".to_string(),
            description: "Extracts video/audio title, formats, duration, upload date, age limit, and thumbnail with cookie dir auto-matching.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "URL of the video stream" },
                    "cookiesDir": { "type": "string", "description": "Directory containing site cookie files (e.g. youtube.txt, reddit.txt, cookies.txt)" },
                    "cookiesFromBrowser": { "type": "string", "description": "Browser to load session cookies from (e.g. 'chrome', 'firefox', 'edge', 'brave')" },
                    "cookiesFile": { "type": "string", "description": "Explicit path to a cookies.txt file" },
                    "playerClient": { "type": "string", "description": "YouTube player client spoofing (e.g. 'android', 'web', 'ios', 'tv')" },
                    "proxy": { "type": "string", "description": "HTTP/HTTPS/SOCKS proxy URL" }
                },
                "required": ["url"]
            }),
        },
        ToolDefinition {
            name: "download_video_stream".to_string(),
            description: "Fetches video streams with resolution capping, subtitle embedding, and cookie dir auto-matching.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "URL of the video stream" },
                    "maxResolution": { "type": "string", "enum": ["480p", "720p", "1080p", "1440p", "2160p"], "default": "1080p" },
                    "writeSubtitles": { "type": "boolean", "default": false },
                    "subtitlesLang": { "type": "string", "default": "en" },
                    "outputDirectory": { "type": "string", "description": "Target download directory" },
                    "cookiesDir": { "type": "string", "description": "Directory containing site cookie files (e.g. youtube.txt, reddit.txt, cookies.txt)" },
                    "cookiesFromBrowser": { "type": "string", "description": "Browser to load session cookies from (e.g. 'chrome', 'firefox', 'edge', 'brave')" },
                    "cookiesFile": { "type": "string", "description": "Explicit path to a cookies.txt file" },
                    "playerClient": { "type": "string", "description": "YouTube player client spoofing (e.g. 'android', 'web', 'ios', 'tv')" },
                    "proxy": { "type": "string", "description": "HTTP/HTTPS/SOCKS proxy URL" }
                },
                "required": ["url"]
            }),
        },
        ToolDefinition {
            name: "download_video_playlist".to_string(),
            description: "Downloads video playlists or channels in batches with range and cookie dir auto-matching.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "URL of the playlist" },
                    "startIndex": { "type": "integer", "default": 1 },
                    "endIndex": { "type": "integer" },
                    "maxVideos": { "type": "integer" },
                    "outputDirectory": { "type": "string" },
                    "cookiesDir": { "type": "string" },
                    "cookiesFromBrowser": { "type": "string" },
                    "cookiesFile": { "type": "string" },
                    "playerClient": { "type": "string" }
                },
                "required": ["url"]
            }),
        },
        ToolDefinition {
            name: "record_live_stream".to_string(),
            description: "Captures live broadcasts (Twitch, Kick, YouTube Live) for a set duration via streamlink.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "Live stream URL" },
                    "durationMinutes": { "type": "integer", "default": 5 },
                    "quality": { "type": "string", "enum": ["best", "1080p", "720p", "480p", "audio_only"], "default": "best" },
                    "outputDirectory": { "type": "string" }
                },
                "required": ["url"]
            }),
        },
        ToolDefinition {
            name: "extract_audio_track".to_string(),
            description: "Extracts and converts audio from video links into MP3, FLAC, M4A, or Opus with cookie dir auto-matching.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "Video/Audio stream URL" },
                    "audioFormat": { "type": "string", "enum": ["mp3", "flac", "wav", "m4a", "opus"], "default": "mp3" },
                    "audioQuality": { "type": "integer", "enum": [0, 1, 2, 3, 4, 5, 6, 7, 8, 9], "default": 0 },
                    "outputDirectory": { "type": "string" },
                    "cookiesDir": { "type": "string" },
                    "cookiesFromBrowser": { "type": "string" },
                    "cookiesFile": { "type": "string" }
                },
                "required": ["url"]
            }),
        },
        ToolDefinition {
            name: "download_music_track".to_string(),
            description: "Fetches tracks, albums, or playlists from Spotify/Apple Music with ID3 metadata via spotdl.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "Spotify or Apple Music URL" },
                    "includeLyrics": { "type": "boolean", "default": false },
                    "outputDirectory": { "type": "string" }
                },
                "required": ["url"]
            }),
        },
        ToolDefinition {
            name: "inspect_image_gallery".to_string(),
            description: "Scans albums, artist profiles, or social posts (Reddit, Instagram, Imgur, Pixiv) with cookie dir auto-matching.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "Gallery or post URL" },
                    "cookiesDir": { "type": "string" },
                    "cookiesFromBrowser": { "type": "string" },
                    "cookiesFile": { "type": "string" },
                    "proxy": { "type": "string" }
                },
                "required": ["url"]
            }),
        },
        ToolDefinition {
            name: "download_image_collection".to_string(),
            description: "Downloads image galleries, multi-image posts, or artist boards with cookie dir auto-matching.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "Gallery URL" },
                    "filterRange": { "type": "string", "description": "Range of items (e.g. '1-10')" },
                    "outputDirectory": { "type": "string" },
                    "cookiesDir": { "type": "string" },
                    "cookiesFromBrowser": { "type": "string" },
                    "cookiesFile": { "type": "string" },
                    "proxy": { "type": "string" }
                },
                "required": ["url"]
            }),
        },
        ToolDefinition {
            name: "download_direct_file".to_string(),
            description: "Accelerated multi-connection segmented download for direct HTTP/HTTPS/FTP URLs via aria2c.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "Direct file URL" },
                    "connectionsPerServer": { "type": "integer", "default": 8 },
                    "outputFilename": { "type": "string" },
                    "outputDirectory": { "type": "string" }
                },
                "required": ["url"]
            }),
        },
        ToolDefinition {
            name: "download_torrent_magnet".to_string(),
            description: "Fetches files from .torrent files or magnet: URIs with rate limiting via aria2c.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "uri": { "type": "string", "description": "Magnet URI or path to .torrent file" },
                    "maxDownloadSpeed": { "type": "string", "description": "Max speed limit (e.g. '5M')" },
                    "outputDirectory": { "type": "string" }
                },
                "required": ["uri"]
            }),
        },
        ToolDefinition {
            name: "trim_media_clip".to_string(),
            description: "Crops or trims video/audio files between timestamps with lossless stream-copying via ffmpeg.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "inputFile": { "type": "string", "description": "Source audio/video path" },
                    "startTime": { "type": "string", "description": "Start timestamp (HH:MM:SS or SS)" },
                    "endTime": { "type": "string", "description": "End timestamp (HH:MM:SS or SS)" },
                    "lossless": { "type": "boolean", "default": true },
                    "outputFile": { "type": "string", "description": "Destination file path" }
                },
                "required": ["inputFile", "startTime", "endTime"]
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
