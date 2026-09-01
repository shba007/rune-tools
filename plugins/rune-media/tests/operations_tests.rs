use rune_media::operations::{
    execute_tool, extract_domain_and_stem, find_cookie_file_in_dir, resolve_cookie_arg, resolve_dir,
};
use rune_pdk::ToolCallRequest;
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

const TEST_BASE_DIR: &str = r"D:\Projects\Practice\rune-kit\test-dir";
const TEST_YOUTUBE_URL: &str = "https://www.youtube.com/watch?v=q4BawYzkzC4";

fn get_workspace_dir() -> PathBuf {
    let base = PathBuf::from(TEST_BASE_DIR);
    if !base.exists() {
        let _ = fs::create_dir_all(&base);
    }
    base
}

// =========================================================================
// 1. Real End-to-End YouTube Live Operations (No Stubs)
// =========================================================================

#[test]
fn test_real_inspect_video_metadata_e2e() {
    let workspace = get_workspace_dir();
    let cookies_dir = workspace.join("cookies");
    let _ = fs::create_dir_all(&cookies_dir);

    let req = ToolCallRequest {
        name: "inspect_video_metadata".to_string(),
        arguments: json!({
            "url": TEST_YOUTUBE_URL,
            "cookiesDir": cookies_dir.to_str().unwrap()
        }),
    };

    let res = execute_tool(req).expect("Failed to execute inspect_video_metadata");
    let metadata = &res["metadata"];

    // Assert real metadata from the target YouTube video
    let title = metadata["title"].as_str().expect("Expected video title");
    assert!(
        title.contains("Piyali Deb") || title.contains("FOI Studios"),
        "Unexpected title: {}",
        title
    );

    let channel = metadata["channel"].as_str().expect("Expected channel name");
    assert_eq!(channel, "FOI Studios");

    let duration = metadata["duration_seconds"].as_u64().unwrap_or(0);
    assert!(
        duration > 0 && duration <= 60,
        "Expected ~30s video, got {}",
        duration
    );

    let formats = metadata["available_formats"]
        .as_array()
        .expect("Expected formats array");
    assert!(!formats.is_empty(), "Expected available format list");
}

#[test]
fn test_real_download_video_stream_and_trim_e2e() {
    let workspace = get_workspace_dir();
    let output_dir = workspace.join("downloads");
    let cookies_dir = workspace.join("cookies");
    let _ = fs::create_dir_all(&output_dir);
    let _ = fs::create_dir_all(&cookies_dir);

    // 1. Real download via yt-dlp
    let download_req = ToolCallRequest {
        name: "download_video_stream".to_string(),
        arguments: json!({
            "url": TEST_YOUTUBE_URL,
            "maxResolution": "720p",
            "outputDirectory": output_dir.to_str().unwrap(),
            "cookiesDir": cookies_dir.to_str().unwrap()
        }),
    };

    let res = execute_tool(download_req).expect("Failed to execute real video download");
    assert_eq!(res["status"], "success");

    // Verify downloaded video file exists on disk
    let entries: Vec<PathBuf> = fs::read_dir(&output_dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("");
            matches!(ext, "mp4" | "mkv" | "webm")
        })
        .collect();

    assert!(
        !entries.is_empty(),
        "No downloaded video file found in {}",
        output_dir.display()
    );

    let downloaded_video = &entries[0];
    let file_size = fs::metadata(downloaded_video).unwrap().len();
    assert!(
        file_size > 50_000,
        "Downloaded file is too small ({} bytes)",
        file_size
    );

    // 2. Real lossless trim using ffmpeg on the downloaded file
    let trimmed_output = output_dir.join("test_clip_trimmed.mp4");
    if trimmed_output.exists() {
        let _ = fs::remove_file(&trimmed_output);
    }

    let trim_req = ToolCallRequest {
        name: "trim_media_clip".to_string(),
        arguments: json!({
            "inputFile": downloaded_video.to_str().unwrap(),
            "startTime": "00:00:02",
            "endTime": "00:00:07",
            "lossless": true,
            "outputFile": trimmed_output.to_str().unwrap()
        }),
    };

    let trim_res = execute_tool(trim_req).expect("Failed to execute ffmpeg trim");
    assert_eq!(trim_res["status"], "success");
    assert!(
        trimmed_output.exists(),
        "Trimmed output file was not created"
    );
    assert!(
        fs::metadata(&trimmed_output).unwrap().len() > 10_000,
        "Trimmed output file is empty or corrupted"
    );
}

#[test]
fn test_real_extract_audio_track_e2e() {
    let workspace = get_workspace_dir();
    let audio_output_dir = workspace.join("audio");
    let cookies_dir = workspace.join("cookies");
    let _ = fs::create_dir_all(&audio_output_dir);
    let _ = fs::create_dir_all(&cookies_dir);

    let req = ToolCallRequest {
        name: "extract_audio_track".to_string(),
        arguments: json!({
            "url": TEST_YOUTUBE_URL,
            "audioFormat": "mp3",
            "audioQuality": 0,
            "outputDirectory": audio_output_dir.to_str().unwrap(),
            "cookiesDir": cookies_dir.to_str().unwrap()
        }),
    };

    let res = execute_tool(req).expect("Failed to execute audio extraction");
    assert_eq!(res["status"], "success");

    let entries: Vec<PathBuf> = fs::read_dir(&audio_output_dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("mp3"))
        .collect();

    assert!(
        !entries.is_empty(),
        "No extracted MP3 file found in {}",
        audio_output_dir.display()
    );
    assert!(fs::metadata(&entries[0]).unwrap().len() > 10_000);
}

// =========================================================================
// 2. Cookie Resolution & Directory Sanitization
// =========================================================================

#[test]
fn test_extract_domain_and_stem() {
    assert_eq!(
        extract_domain_and_stem(TEST_YOUTUBE_URL),
        ("youtube.com".to_string(), "youtube".to_string())
    );
    assert_eq!(
        extract_domain_and_stem("https://www.youtube.com/shorts/EqvgsORpbOU"),
        ("youtu.be".to_string(), "youtube".to_string())
    );
    assert_eq!(
        extract_domain_and_stem("https://x.com/user/status/123"),
        ("x.com".to_string(), "twitter".to_string())
    );
    assert_eq!(
        extract_domain_and_stem("https://vimeo.com/channels/123"),
        ("vimeo.com".to_string(), "vimeo".to_string())
    );
}

#[test]
fn test_find_cookie_file_in_dir_patterns() {
    let dir = tempdir().unwrap();

    let yt_cookie = dir.path().join("youtube.txt");
    fs::write(&yt_cookie, "domain match").unwrap();
    assert_eq!(
        find_cookie_file_in_dir(dir.path(), TEST_YOUTUBE_URL).unwrap(),
        yt_cookie
    );
    fs::remove_file(&yt_cookie).unwrap();

    let custom_yt = dir.path().join("my_youtube_account.txt");
    fs::write(&custom_yt, "substring match").unwrap();
    assert_eq!(
        find_cookie_file_in_dir(dir.path(), TEST_YOUTUBE_URL).unwrap(),
        custom_yt
    );
    fs::remove_file(&custom_yt).unwrap();

    let default_cookie = dir.path().join("cookies.txt");
    fs::write(&default_cookie, "fallback").unwrap();
    assert_eq!(
        find_cookie_file_in_dir(dir.path(), TEST_YOUTUBE_URL).unwrap(),
        default_cookie
    );
}

#[test]
fn test_resolve_cookie_arg_priority() {
    let dir = tempdir().unwrap();
    let cookie_file = dir.path().join("youtube.txt");
    fs::write(&cookie_file, "cookie").unwrap();

    // 1. Explicit cookiesFile priority
    let req_file = ToolCallRequest {
        name: "download_video_stream".to_string(),
        arguments: json!({
            "url": TEST_YOUTUBE_URL,
            "cookiesFile": r"D:\Projects\Practice\rune-kit\test-dir\cookies\custom.txt",
            "cookiesDir": dir.path().to_str().unwrap(),
            "cookiesFromBrowser": "chrome"
        }),
    };
    let resolved_file = resolve_cookie_arg(&req_file, TEST_YOUTUBE_URL).unwrap();
    assert_eq!(resolved_file.0, "--cookies");
    assert_eq!(
        resolved_file.1,
        r"D:\Projects\Practice\rune-kit\test-dir\cookies\custom.txt"
    );

    // 2. cookiesDir priority over browser
    let req_dir = ToolCallRequest {
        name: "download_video_stream".to_string(),
        arguments: json!({
            "url": TEST_YOUTUBE_URL,
            "cookiesDir": dir.path().to_str().unwrap(),
            "cookiesFromBrowser": "chrome"
        }),
    };
    let resolved_dir = resolve_cookie_arg(&req_dir, TEST_YOUTUBE_URL).unwrap();
    assert_eq!(resolved_dir.0, "--cookies");
    assert_eq!(resolved_dir.1, cookie_file.to_str().unwrap());
}

#[test]
fn test_resolve_dir_custom_path() {
    let resolved = resolve_dir(Some(TEST_BASE_DIR));
    assert_eq!(resolved, TEST_BASE_DIR);
}

// =========================================================================
// 3. Tool Routing & Error Handling
// =========================================================================

#[test]
fn test_verify_downloader_environment() {
    let req = ToolCallRequest {
        name: "verify_downloader_environment".to_string(),
        arguments: json!({}),
    };
    let res = execute_tool(req).unwrap();
    let env = &res["environment"];

    assert_eq!(
        env["yt-dlp"]["installed"], true,
        "yt-dlp should be installed on host"
    );
    assert_eq!(
        env["ffmpeg"]["installed"], true,
        "ffmpeg should be installed on host"
    );
}

#[test]
fn test_empty_required_parameters() {
    let req_empty_url = ToolCallRequest {
        name: "download_video_stream".to_string(),
        arguments: json!({ "url": "   " }),
    };
    let res = execute_tool(req_empty_url);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Parameter 'url' cannot be empty"));
}

#[test]
fn test_unknown_tool_routing() {
    let req = ToolCallRequest {
        name: "non_existent_tool".to_string(),
        arguments: json!({}),
    };
    let res = execute_tool(req);
    assert!(res.is_err());
    assert_eq!(res.unwrap_err(), "Unknown tool: non_existent_tool");
}
