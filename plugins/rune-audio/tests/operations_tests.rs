use rune_audio::operations::{
    execute_tool, extract_domain_and_stem, find_cookie_file_in_dir, resolve_cookie_arg, resolve_dir,
};
use rune_pdk::ToolCallRequest;
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

const TEST_BASE_DIR: &str = r"D:\Projects\Practice\rune-kit\test-dir";
const TEST_AUDIO_URL: &str = "https://www.youtube.com/shorts/EqvgsORpbOU";

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
        extract_domain_and_stem(TEST_AUDIO_URL),
        ("youtube.com".to_string(), "youtube".to_string())
    );
    assert_eq!(
        extract_domain_and_stem("https://spotify.com/track/0HE9a9ndSFMCELuobaW5yK"),
        ("spotify.com".to_string(), "spotify".to_string())
    );
}

#[test]
fn test_find_cookie_file_in_dir_patterns() {
    let dir = tempdir().unwrap();

    let yt_cookie = dir.path().join("youtube.txt");
    fs::write(&yt_cookie, "domain match").unwrap();
    assert_eq!(
        find_cookie_file_in_dir(dir.path(), TEST_AUDIO_URL).unwrap(),
        yt_cookie
    );
    fs::remove_file(&yt_cookie).unwrap();

    let default_cookie = dir.path().join("cookies.txt");
    fs::write(&default_cookie, "fallback").unwrap();
    assert_eq!(
        find_cookie_file_in_dir(dir.path(), TEST_AUDIO_URL).unwrap(),
        default_cookie
    );
}

#[test]
fn test_resolve_cookie_arg_priority() {
    let dir = tempdir().unwrap();
    let cookie_file = dir.path().join("youtube.txt");
    fs::write(&cookie_file, "cookie").unwrap();

    let req_file = ToolCallRequest {
        name: "extract_audio_track".to_string(),
        arguments: json!({
            "url": TEST_AUDIO_URL,
            "cookiesFile": r"D:\Projects\Practice\rune-kit\test-dir\cookies\custom.txt",
            "cookiesDir": dir.path().to_str().unwrap(),
            "cookiesFromBrowser": "chrome"
        }),
    };
    let resolved_file = resolve_cookie_arg(&req_file, TEST_AUDIO_URL).unwrap();
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
fn test_live_extract_audio_track_e2e() {
    if std::env::var("CI").is_ok() {
        eprintln!("Skipping live test: Running in CI environment");
        return;
    }

    let workspace = get_workspace_dir();
    let audio_output_dir = workspace.join("audio");
    let cookies_dir = workspace.join("cookies");
    let _ = fs::create_dir_all(&audio_output_dir);
    let _ = fs::create_dir_all(&cookies_dir);

    let req = ToolCallRequest {
        name: "extract_audio_track".to_string(),
        arguments: json!({
            "url": TEST_AUDIO_URL,
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
}

#[test]
fn test_empty_required_parameters() {
    let req_empty_url = ToolCallRequest {
        name: "extract_audio_track".to_string(),
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
