use rune_pdk::ToolCallRequest;
use rune_video::operations::{
    execute_tool, extract_domain_and_stem, find_cookie_file_in_dir, resolve_cookie_arg, resolve_dir,
};
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

const TEST_BASE_DIR: &str = r"D:\Projects\Practice\rune-kit\test-dir";
const TEST_YOUTUBE_URL: &str = "https://www.youtube.com/shorts/EqvgsORpbOU";

fn get_workspace_dir() -> PathBuf {
    let base = PathBuf::from(TEST_BASE_DIR);
    if !base.exists() {
        let _ = fs::create_dir_all(&base);
    }
    base
}

#[test]
fn test_extract_domain_and_stem() {
    assert_eq!(
        extract_domain_and_stem(TEST_YOUTUBE_URL),
        ("youtube.com".to_string(), "youtube".to_string())
    );
    assert_eq!(
        extract_domain_and_stem("https://www.twitch.tv/example"),
        ("twitch.tv".to_string(), "twitch".to_string())
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
}

#[test]
fn test_resolve_dir_custom_path() {
    let resolved = resolve_dir(Some(TEST_BASE_DIR));
    assert_eq!(resolved, TEST_BASE_DIR);
}

#[test]
fn test_real_inspect_video_metadata_e2e() {
    let workspace = get_workspace_dir();
    let cookies_dir = workspace.join("cookies");

    let req = ToolCallRequest {
        name: "inspect_video_metadata".to_string(),
        arguments: json!({
            "url": TEST_YOUTUBE_URL,
            "cookiesDir": cookies_dir.to_str().unwrap()
        }),
    };

    let res = execute_tool(req).expect("Failed to execute inspect_video_metadata");
    let metadata = &res["metadata"];

    let title = metadata["title"].as_str().expect("Expected video title");
    assert!(
        title.contains("Piyali Deb") || title.contains("FOI Studios"),
        "Unexpected title: {}",
        title
    );
}

#[test]
fn test_real_download_video_stream_and_trim_e2e() {
    let workspace = get_workspace_dir();
    let output_dir = workspace.join("video");
    let cookies_dir = workspace.join("cookies");
    let _ = fs::create_dir_all(&output_dir);

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

    let entries: Vec<PathBuf> = fs::read_dir(&output_dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("");
            matches!(ext, "mp4" | "mkv" | "webm")
        })
        .collect();

    assert!(!entries.is_empty(), "No downloaded video file found");

    let downloaded_video = &entries[0];
    let trimmed_output = output_dir.join("test_clip_trimmed.mp4");
    if trimmed_output.exists() {
        let _ = fs::remove_file(&trimmed_output);
    }

    let trim_req = ToolCallRequest {
        name: "trim_media_clip".to_string(),
        arguments: json!({
            "inputFile": downloaded_video.to_str().unwrap(),
            "startTime": "00:00:01",
            "endTime": "00:00:05",
            "lossless": true,
            "outputFile": trimmed_output.to_str().unwrap()
        }),
    };

    let trim_res = execute_tool(trim_req).expect("Failed to execute ffmpeg trim");
    assert_eq!(trim_res["status"], "success");
    assert!(trimmed_output.exists());
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
