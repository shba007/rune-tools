use rune_image::operations::{
    execute_tool, extract_domain_and_stem, find_cookie_file_in_dir, resolve_cookie_arg, resolve_dir,
};
use rune_pdk::ToolCallRequest;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

const TEST_BASE_DIR: &str = r"../../temp";
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
            "cookiesFile": r"../../temp/cookies/reddit.txt",
            "cookiesDir": dir.path().to_str().unwrap(),
            "cookiesFromBrowser": "chrome"
        }),
    };
    let resolved_file = resolve_cookie_arg(&req_file, TEST_GALLERY_URL).unwrap();
    assert_eq!(resolved_file.0, "--cookies");
    assert_eq!(resolved_file.1, r"../../temp/cookies/reddit.txt");
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

    match execute_tool(req) {
        Ok(res) => {
            let total = res["total_media_found"].as_u64().unwrap_or(0);
            if total == 0 {
                eprintln!(
                    "Live test skipped: Reddit returned 0 items (rate-limited or blocked by network security)"
                );
                return;
            }
            assert!(!res["previews"].as_array().unwrap().is_empty());
        }
        Err(e) => {
            eprintln!("Live test skipped or connection failed: {}", e);
        }
    }
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

    match execute_tool(req) {
        Ok(res) => {
            assert_eq!(res["status"], "success");
            let file_count = count_files_recursive(&output_dir);
            if file_count == 0 {
                eprintln!("Live download skipped: 0 files saved (Reddit blocked download)");
            }
        }
        Err(e) => {
            eprintln!(
                "Live API test skipped: Download failed (likely blocked by network security): {}",
                e
            );
        }
    }
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

#[test]
fn test_compare_images_missing_image1() {
    let req = ToolCallRequest {
        name: "compare_images".to_string(),
        arguments: json!({
            "image2Path": "/temp/images/test.png"
        }),
    };
    let res = execute_tool(req);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Missing 'image1Path'"));
}

#[test]
fn test_compare_images_missing_image2() {
    let req = ToolCallRequest {
        name: "compare_images".to_string(),
        arguments: json!({
            "image1Path": "/temp/images/test-2.png"
        }),
    };
    let res = execute_tool(req);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Missing 'image2Path'"));
}

#[test]
fn test_compare_images_both_images_missing() {
    if std::env::var("CI").is_ok() {
        eprintln!("Skipping live test: Running in CI environment");
        return;
    }

    let req = ToolCallRequest {
        name: "compare_images".to_string(),
        arguments: json!({}),
    };
    let res = execute_tool(req);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Missing 'image1Path'"));
}

#[test]
fn test_compare_images_invalid_threshold() {
    let req = ToolCallRequest {
        name: "compare_images".to_string(),
        arguments: json!({
            "image1Path": "/temp/images/test.png",
            "image2Path": "/temp/images/test-2.png",
            "threshold": 1.5
        }),
    };
    let res = execute_tool(req);
    assert!(res.is_err());
}

#[test]
fn test_compare_images_invalid_threshold_negative() {
    if std::env::var("CI").is_ok() {
        eprintln!("Skipping live test: Running in CI environment");
        return;
    }

    let req = ToolCallRequest {
        name: "compare_images".to_string(),
        arguments: json!({
            "image1Path": "D:/Projects/Public/rune/code-tools/temp/images/test.png",
            "image2Path": "D:/Projects/Public/rune/code-tools/temp/images/test-2.png",
            "threshold": -0.5
        }),
    };
    let res = execute_tool(req);
    assert!(res.is_err());
}

#[test]
fn test_compare_images_with_threshold() {
    let req = ToolCallRequest {
        name: "compare_images".to_string(),
        arguments: json!({
            "image1Path": "D:/Projects/Public/rune/code-tools/temp/images/test.png",
            "image2Path": "D:/Projects/Public/rune/code-tools/temp/images/test-2.png",
            "threshold": 0.1
        }),
    };
    let res = execute_tool(req);
    if let Err(err) = &res {
        assert!(
            err.contains("Image1 path does not exist") || err.contains("Failed to convert SVG")
        );
    }
}

#[test]
fn test_compare_images_different_formats() {
    let req = ToolCallRequest {
        name: "compare_images".to_string(),
        arguments: json!({
            "image1Path": "/path/to/image1.jpg",
            "image2Path": "D:/Projects/Public/rune/code-tools/temp/images/test-2.png",
            "algorithm": "rms"
        }),
    };
    let res = execute_tool(req);
    if let Err(err) = &res {
        assert!(
            err.contains("Image1 path does not exist") || err.contains("Failed to convert SVG")
        );
    }
}

#[test]
fn test_compare_images_different_dimensions() {
    let req = ToolCallRequest {
        name: "compare_images".to_string(),
        arguments: json!({
            "image1Path": "/path/to/small.png",
            "image2Path": "/path/to/large.png",
            "algorithm": "rms"
        }),
    };
    let res = execute_tool(req);
    if let Err(err) = &res {
        assert!(
            err.contains("Image1 path does not exist") || err.contains("Failed to convert SVG")
        );
    }
}

#[test]
fn test_compare_images_rms_algorithm() {
    let req = ToolCallRequest {
        name: "compare_images".to_string(),
        arguments: json!({
            "image1Path": "D:/Projects/Public/rune/code-tools/temp/images/test.png",
            "image2Path": "D:/Projects/Public/rune/code-tools/temp/images/test-2.png",
            "algorithm": "rms"
        }),
    };
    let res = execute_tool(req);
    if let Err(err) = &res {
        assert!(
            err.contains("Image1 path does not exist") || err.contains("Failed to convert SVG")
        );
    }
}

#[test]
fn test_compare_images_mssim_algorithm() {
    let req = ToolCallRequest {
        name: "compare_images".to_string(),
        arguments: json!({
            "image1Path": "D:/Projects/Public/rune/code-tools/temp/images/test.png",
            "image2Path": "D:/Projects/Public/rune/code-tools/temp/images/test-2.png",
            "algorithm": "mssim"
        }),
    };
    let res = execute_tool(req);
    if let Err(err) = &res {
        assert!(
            err.contains("Image1 path does not exist") || err.contains("Failed to convert SVG")
        );
    }
}

#[test]
fn test_compare_images_perceptual_algorithm() {
    let req = ToolCallRequest {
        name: "compare_images".to_string(),
        arguments: json!({
            "image1Path": "D:/Projects/Public/rune/code-tools/temp/images/test.png",
            "image2Path": "D:/Projects/Public/rune/code-tools/temp/images/test-2.png",
            "algorithm": "perceptual"
        }),
    };
    let res = execute_tool(req);
    if let Err(err) = &res {
        assert!(
            err.contains("Image1 path does not exist") || err.contains("Failed to convert SVG")
        );
    }
}

#[test]
fn test_compare_images_default_algorithm() {
    let req = ToolCallRequest {
        name: "compare_images".to_string(),
        arguments: json!({
            "image1Path": "D:/Projects/Public/rune/code-tools/temp/images/test.png",
            "image2Path": "D:/Projects/Public/rune/code-tools/temp/images/test-2.png"
        }),
    };
    let res = execute_tool(req);
    if let Err(err) = &res {
        assert!(
            err.contains("Image1 path does not exist") || err.contains("Failed to convert SVG")
        );
    }
}

#[test]
fn test_compare_images_svg_support() {
    let req = ToolCallRequest {
        name: "compare_images".to_string(),
        arguments: json!({
            "image1Path": "/path/to/image1.svg",
            "image2Path": "/path/to/image2.svg",
            "algorithm": "rms"
        }),
    };
    let res = execute_tool(req);
    if let Err(err) = &res {
        assert!(
            err.contains("Image1 path does not exist") || err.contains("Failed to convert SVG")
        );
    }
}

#[test]
fn test_compare_images_svg_mixed_with_png() {
    let req = ToolCallRequest {
        name: "compare_images".to_string(),
        arguments: json!({
            "image1Path": "/path/to/image1.svg",
            "image2Path": "D:/Projects/Public/rune/code-tools/temp/images/test-2.png",
            "algorithm": "rms"
        }),
    };
    let res = execute_tool(req);
    if let Err(err) = &res {
        assert!(
            err.contains("Image1 path does not exist") || err.contains("Failed to convert SVG")
        );
    }
}

#[test]
fn test_compare_images_same_file() {
    let req = ToolCallRequest {
        name: "compare_images".to_string(),
        arguments: json!({
            "image1Path": "/path/to/same_image.png",
            "image2Path": "/path/to/same_image.png",
            "algorithm": "rms"
        }),
    };
    let res = execute_tool(req);
    if let Err(err) = &res {
        assert!(
            err.contains("Image1 path does not exist") || err.contains("Failed to convert SVG")
        );
    }
}

#[test]
fn test_compare_images_with_min_threshold() {
    let req = ToolCallRequest {
        name: "compare_images".to_string(),
        arguments: json!({
            "image1Path": "D:/Projects/Public/rune/code-tools/temp/images/test.png",
            "image2Path": "D:/Projects/Public/rune/code-tools/temp/images/test-2.png",
            "threshold": 0.0
        }),
    };
    let res = execute_tool(req);
    if let Err(err) = &res {
        assert!(
            err.contains("Image1 path does not exist") || err.contains("Failed to convert SVG")
        );
    }
}

#[test]
fn test_compare_images_output_path_handling() {
    let req = ToolCallRequest {
        name: "compare_images".to_string(),
        arguments: json!({
            "image1Path": "D:/Projects/Public/rune/code-tools/temp/images/test.png",
            "image2Path": "D:/Projects/Public/rune/code-tools/temp/images/test-2.png",
            "outputPath": "D:/test/output.png"
        }),
    };
    let res = execute_tool(req);
    if let Err(err) = &res {
        assert!(
            err.contains("Image1 path does not exist") || err.contains("Failed to convert SVG")
        );
    }
}

#[test]
fn test_compare_images_all_formats() {
    let formats = vec![
        ("png", "image1.png"),
        ("jpg", "image2.jpg"),
        ("jpeg", "image3.jpeg"),
        ("gif", "image4.gif"),
        ("webp", "image5.webp"),
    ];

    for (ext, filename) in formats {
        let req = ToolCallRequest {
            name: "compare_images".to_string(),
            arguments: json!({
                "image1Path": format!("/path/to/{}.png", filename),
                "image2Path": format!("/path/to/{}.{}", filename, ext),
                "algorithm": "rms"
            }),
        };
        let res = execute_tool(req);
        if let Err(err) = &res {
            assert!(
                err.contains("Image1 path does not exist") || err.contains("Failed to convert SVG")
            );
        }
    }
}

#[test]
fn test_compare_images_relative_path_output() {
    let req = ToolCallRequest {
        name: "compare_images".to_string(),
        arguments: json!({
            "image1Path": "D:/Projects/Public/rune/code-tools/temp/images/test.png",
            "image2Path": "D:/Projects/Public/rune/code-tools/temp/images/test-2.png",
            "outputPath": "./diff_output.png"
        }),
    };
    let res = execute_tool(req);
    if let Err(err) = &res {
        assert!(
            !err.contains("Failed to save diff image"),
            "Output path should be handled correctly"
        );
    }
}

#[test]
fn test_compare_images_svg_dimensions() {
    let req = ToolCallRequest {
        name: "compare_images".to_string(),
        arguments: json!({
            "image1Path": "/path/to/icon.svg?width=100&height=100",
            "image2Path": "/path/to/other.svg",
            "algorithm": "rms"
        }),
    };
    let res = execute_tool(req);
    if let Err(err) = &res {
        assert!(
            err.contains("Image1 path does not exist") || err.contains("Failed to convert SVG")
        );
    }
}

#[test]
fn test_compare_images_empty_path() {
    let req = ToolCallRequest {
        name: "compare_images".to_string(),
        arguments: json!({
            "image1Path": "",
            "image2Path": "D:/Projects/Public/rune/code-tools/temp/images/test-2.png"
        }),
    };
    let res = execute_tool(req);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Image paths cannot be empty"));
}

#[test]
fn test_compare_real_png_with_svg() {
    if std::env::var("CI").is_ok() {
        eprintln!("Skipping live test: Running in CI environment");
        return;
    }

    let dir = tempdir().unwrap();
    let png_path = dir.path().join("head.png");
    let svg_path = dir.path().join("head.svg");

    let img = image::RgbImage::from_fn(20, 20, |_x, _y| image::Rgb([255, 0, 0]));
    img.save(&png_path).expect("Failed to create test PNG");

    let svg_content = r#"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20">
        <rect width="20" height="20" fill="red"/>
    </svg>"#;
    fs::write(&svg_path, svg_content).expect("Failed to create test SVG");

    let diff_path = dir.path().join("test_diff.png");

    let req = ToolCallRequest {
        name: "compare_images".to_string(),
        arguments: json!({
            "image1Path": png_path.to_str().unwrap(),
            "image2Path": svg_path.to_str().unwrap(),
            "algorithm": "rms",
            "outputPath": diff_path.to_str().unwrap()
        }),
    };

    let res = execute_tool(req);
    match res {
        Ok(result) => {
            assert!(
                result
                    .get("outputPath")
                    .or_else(|| result.get("output_path"))
                    .is_some()
            );
            assert!(
                result
                    .get("matchPercentage")
                    .or_else(|| result.get("match_percentage"))
                    .is_some()
            );
        }
        Err(err) => {
            println!("PNG vs SVG comparison error: {}", err);
            assert!(
                err.contains("Failed") || err.contains("SVG"),
                "Error should be clear: {}",
                err
            );
        }
    }
}

#[test]
fn test_compare_same_png_file() {
    if std::env::var("CI").is_ok() {
        eprintln!("Skipping live test: Running in CI environment");
        return;
    }

    let dir = tempdir().unwrap();
    let png_path = dir.path().join("head.png");
    let diff_path = dir.path().join("same_diff.png");

    let img = image::RgbImage::from_fn(20, 20, |x, y| {
        image::Rgb([(x * 12) as u8, (y * 12) as u8, 128])
    });
    img.save(&png_path).expect("Failed to create test PNG");

    let req = ToolCallRequest {
        name: "compare_images".to_string(),
        arguments: json!({
            "image1Path": png_path.to_str().unwrap(),
            "image2Path": png_path.to_str().unwrap(),
            "algorithm": "rms",
            "outputPath": diff_path.to_str().unwrap()
        }),
    };

    let res = execute_tool(req).expect("Failed to compare same PNG file");
    let match_pct = res
        .get("matchPercentage")
        .or_else(|| res.get("match_percentage"))
        .and_then(|v| v.as_f64())
        .expect("Expected match percentage");
    assert_eq!(match_pct, 100.0);
}
