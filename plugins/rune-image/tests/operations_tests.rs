use rune_image::operations::{
    execute_tool, extract_domain_and_stem, find_cookie_file_in_dir, resolve_cookie_arg, resolve_dir,
};
use rune_pdk::ToolCallRequest;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

const TEST_BASE_DIR: &str = r"D:\Projects\Practice\rune-kit\test-dir";
const TEST_GALLERY_URL: &str = "https://www.reddit.com/r/LocalLLaMA/comments/1ve4uoe/daniel_han_of_unsloth_validates_qwen3827b_will/";

fn get_workspace_dir() -> PathBuf {
    let base = PathBuf::from(TEST_BASE_DIR);
    if !base.exists() {
        let _ = fs::create_dir_all(&base);
    }
    base
}

fn count_files_recursive(dir: &Path) -> usize {
    let mut count = 0;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                count += count_files_recursive(&path);
            } else if path.is_file() {
                count += 1;
            }
        }
    }
    count
}

#[test]
fn test_extract_domain_and_stem() {
    assert_eq!(
        extract_domain_and_stem(TEST_GALLERY_URL),
        ("reddit.com".to_string(), "reddit".to_string())
    );
    // assert_eq!(
    //     extract_domain_and_stem("https://i.imgur.com/example.jpg"),
    //     ("imgur.com".to_string(), "imgur".to_string())
    // );
    // assert_eq!(
    //     extract_domain_and_stem("https://x.com/user/status/123"),
    //     ("x.com".to_string(), "twitter".to_string())
    // );
}

#[test]
fn test_find_cookie_file_in_dir_patterns() {
    let dir = tempdir().unwrap();

    let reddit_cookie = dir.path().join("reddit.txt");
    fs::write(&reddit_cookie, "domain match").unwrap();
    assert_eq!(
        find_cookie_file_in_dir(dir.path(), TEST_GALLERY_URL).unwrap(),
        reddit_cookie
    );
    fs::remove_file(&reddit_cookie).unwrap();

    let default_cookie = dir.path().join("cookies.txt");
    fs::write(&default_cookie, "fallback").unwrap();
    assert_eq!(
        find_cookie_file_in_dir(dir.path(), TEST_GALLERY_URL).unwrap(),
        default_cookie
    );
}

#[test]
fn test_resolve_cookie_arg_priority() {
    let dir = tempdir().unwrap();
    let cookie_file = dir.path().join("reddit.txt");
    fs::write(&cookie_file, "cookie").unwrap();

    let req_file = ToolCallRequest {
        name: "download_image_collection".to_string(),
        arguments: json!({
            "url": TEST_GALLERY_URL,
            "cookiesFile": r"D:\Projects\Practice\rune-kit\test-dir\cookies\reddit.txt",
            "cookiesDir": dir.path().to_str().unwrap(),
            "cookiesFromBrowser": "chrome"
        }),
    };
    let resolved_file = resolve_cookie_arg(&req_file, TEST_GALLERY_URL).unwrap();
    assert_eq!(resolved_file.0, "--cookies");
    assert_eq!(
        resolved_file.1,
        r"D:\Projects\Practice\rune-kit\test-dir\cookies\reddit.txt"
    );
}

#[test]
fn test_resolve_dir_custom_path() {
    let resolved = resolve_dir(Some(TEST_BASE_DIR));
    assert_eq!(resolved, TEST_BASE_DIR);
}

#[test]
fn test_live_inspect_image_gallery_e2e() {
    if std::env::var("CI").is_ok() {
        eprintln!("Skipping live test: Running in CI environment");
        return;
    }

    let workspace = get_workspace_dir();
    let cookies_dir = workspace.join("cookies");

    let req = ToolCallRequest {
        name: "inspect_image_gallery".to_string(),
        arguments: json!({
            "url": TEST_GALLERY_URL,
            "cookiesDir": cookies_dir.to_str().unwrap()
        }),
    };

    let res = execute_tool(req).expect("Failed to execute inspect_image_gallery");
    print!("{}", res);
    assert!(res["total_media_found"].as_u64().unwrap_or(0) > 0);
    assert!(!res["previews"].as_array().unwrap().is_empty());
}

#[test]
fn test_live_download_image_collection_e2e() {
    if std::env::var("CI").is_ok() {
        eprintln!("Skipping live test: Running in CI environment");
        return;
    }

    let workspace = get_workspace_dir();
    let output_dir = workspace.join("images");
    let cookies_dir = workspace.join("cookies");
    let _ = fs::create_dir_all(&output_dir);
    let _ = fs::create_dir_all(&cookies_dir);

    let req = ToolCallRequest {
        name: "download_image_collection".to_string(),
        arguments: json!({
            "url": TEST_GALLERY_URL,
            "outputDirectory": output_dir.to_str().unwrap(),
            "cookiesDir": cookies_dir.to_str().unwrap(),
            "filterRange": "1"
        }),
    };

    let res = execute_tool(req).expect("Failed to execute download_image_collection");
    assert_eq!(res["status"], "success");

    let file_count = count_files_recursive(&output_dir);
    assert!(
        file_count > 0,
        "No downloaded images found in {}",
        output_dir.display()
    );
}

#[test]
fn test_empty_required_parameters() {
    let req_empty_url = ToolCallRequest {
        name: "inspect_image_gallery".to_string(),
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
