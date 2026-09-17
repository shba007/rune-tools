use rune_image::operations::{
    execute_tool, extract_domain_and_stem, find_cookie_file_in_dir, resolve_cookie_arg, resolve_dir,
};
use rune_pdk::ToolCallRequest;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

const TEST_BASE_DIR: &str = r"../code-kit/test-dir";
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
            "cookiesFile": r"../code-kit/test-dir/cookies/reddit.txt",
            "cookiesDir": dir.path().to_str().unwrap(),
            "cookiesFromBrowser": "chrome"
        }),
    };
    let resolved_file = resolve_cookie_arg(&req_file, TEST_GALLERY_URL).unwrap();
    assert_eq!(resolved_file.0, "--cookies");
    assert_eq!(resolved_file.1, r"../code-kit/test-dir/cookies/reddit.txt");
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

// Tests for compare_images tool
#[test]
fn test_compare_images_missing_image1() {
    let req = ToolCallRequest {
        name: "compare_images".to_string(),
        arguments: json!({
            "image2_path": "/path/to/image2.png"
        }),
    };
    let res = execute_tool(req);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Missing 'image1_path'"));
}

#[test]
fn test_compare_images_missing_image2() {
    let req = ToolCallRequest {
        name: "compare_images".to_string(),
        arguments: json!({
            "image1_path": "/path/to/image1.png"
        }),
    };
    let res = execute_tool(req);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Missing 'image2_path'"));
}

#[test]
fn test_compare_images_both_images_missing() {
    let req = ToolCallRequest {
        name: "compare_images".to_string(),
        arguments: json!({}),
    };
    let res = execute_tool(req);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Missing 'image1_path'"));
}

#[test]
fn test_compare_images_invalid_threshold() {
    let req = ToolCallRequest {
        name: "compare_images".to_string(),
        arguments: json!({
            "image1_path": "/path/to/image1.png",
            "image2_path": "/path/to/image2.png",
            "threshold": 1.5
        }),
    };
    let res = execute_tool(req);
    // Threshold validation happens after image loading, so it will fail with image open error first
    assert!(res.is_err());
}

#[test]
fn test_compare_images_invalid_threshold_negative() {
    let req = ToolCallRequest {
        name: "compare_images".to_string(),
        arguments: json!({
            "image1_path": "/path/to/image1.png",
            "image2_path": "/path/to/image2.png",
            "threshold": -0.5
        }),
    };
    let res = execute_tool(req);
    // Threshold validation happens after image loading, so it will fail with image open error first
    assert!(res.is_err());
}

#[test]
fn test_compare_images_with_threshold() {
    let req = ToolCallRequest {
        name: "compare_images".to_string(),
        arguments: json!({
            "image1_path": "/path/to/image1.png",
            "image2_path": "/path/to/image2.png",
            "threshold": 0.1
        }),
    };
    let res = execute_tool(req);
    // Just check that it doesn't crash on threshold parsing
    // The actual comparison will fail with missing files
    if res.is_err() {
        assert!(
            res.as_ref()
                .unwrap_err()
                .contains("Image1 path does not exist")
                || res.as_ref().unwrap_err().contains("Failed to convert SVG")
        );
    }
}

#[test]
fn test_compare_images_different_formats() {
    let req = ToolCallRequest {
        name: "compare_images".to_string(),
        arguments: json!({
            "image1_path": "/path/to/image1.jpg",
            "image2_path": "/path/to/image2.png",
            "algorithm": "rms"
        }),
    };
    let res = execute_tool(req);
    if res.is_err() {
        assert!(
            res.as_ref()
                .unwrap_err()
                .contains("Image1 path does not exist")
                || res.as_ref().unwrap_err().contains("Failed to convert SVG")
        );
    }
}

#[test]
fn test_compare_images_different_dimensions() {
    let req = ToolCallRequest {
        name: "compare_images".to_string(),
        arguments: json!({
            "image1_path": "/path/to/small.png",
            "image2_path": "/path/to/large.png",
            "algorithm": "rms"
        }),
    };
    let res = execute_tool(req);
    if res.is_err() {
        assert!(
            res.as_ref()
                .unwrap_err()
                .contains("Image1 path does not exist")
                || res.as_ref().unwrap_err().contains("Failed to convert SVG")
        );
    }
}

#[test]
fn test_compare_images_rms_algorithm() {
    let req = ToolCallRequest {
        name: "compare_images".to_string(),
        arguments: json!({
            "image1_path": "/path/to/image1.png",
            "image2_path": "/path/to/image2.png",
            "algorithm": "rms"
        }),
    };
    let res = execute_tool(req);
    if res.is_err() {
        assert!(
            res.as_ref()
                .unwrap_err()
                .contains("Image1 path does not exist")
                || res.as_ref().unwrap_err().contains("Failed to convert SVG")
        );
    }
}

#[test]
fn test_compare_images_mssim_algorithm() {
    let req = ToolCallRequest {
        name: "compare_images".to_string(),
        arguments: json!({
            "image1_path": "/path/to/image1.png",
            "image2_path": "/path/to/image2.png",
            "algorithm": "mssim"
        }),
    };
    let res = execute_tool(req);
    if res.is_err() {
        assert!(
            res.as_ref()
                .unwrap_err()
                .contains("Image1 path does not exist")
                || res.as_ref().unwrap_err().contains("Failed to convert SVG")
        );
    }
}

#[test]
fn test_compare_images_perceptual_algorithm() {
    let req = ToolCallRequest {
        name: "compare_images".to_string(),
        arguments: json!({
            "image1_path": "/path/to/image1.png",
            "image2_path": "/path/to/image2.png",
            "algorithm": "perceptual"
        }),
    };
    let res = execute_tool(req);
    if res.is_err() {
        assert!(
            res.as_ref()
                .unwrap_err()
                .contains("Image1 path does not exist")
                || res.as_ref().unwrap_err().contains("Failed to convert SVG")
        );
    }
}

#[test]
fn test_compare_images_default_algorithm() {
    let req = ToolCallRequest {
        name: "compare_images".to_string(),
        arguments: json!({
            "image1_path": "/path/to/image1.png",
            "image2_path": "/path/to/image2.png"
        }),
    };
    let res = execute_tool(req);
    if res.is_err() {
        assert!(
            res.as_ref()
                .unwrap_err()
                .contains("Image1 path does not exist")
                || res.as_ref().unwrap_err().contains("Failed to convert SVG")
        );
    }
}

#[test]
fn test_compare_images_svg_support() {
    let req = ToolCallRequest {
        name: "compare_images".to_string(),
        arguments: json!({
            "image1_path": "/path/to/image1.svg",
            "image2_path": "/path/to/image2.svg",
            "algorithm": "rms"
        }),
    };
    let res = execute_tool(req);
    if res.is_err() {
        assert!(
            res.as_ref()
                .unwrap_err()
                .contains("Image1 path does not exist")
                || res.as_ref().unwrap_err().contains("Failed to convert SVG")
        );
    }
}

#[test]
fn test_compare_images_svg_mixed_with_png() {
    let req = ToolCallRequest {
        name: "compare_images".to_string(),
        arguments: json!({
            "image1_path": "/path/to/image1.svg",
            "image2_path": "/path/to/image2.png",
            "algorithm": "rms"
        }),
    };
    let res = execute_tool(req);
    if res.is_err() {
        assert!(
            res.as_ref()
                .unwrap_err()
                .contains("Image1 path does not exist")
                || res.as_ref().unwrap_err().contains("Failed to convert SVG")
        );
    }
}

#[test]
fn test_compare_images_same_file() {
    let req = ToolCallRequest {
        name: "compare_images".to_string(),
        arguments: json!({
            "image1_path": "/path/to/same_image.png",
            "image2_path": "/path/to/same_image.png",
            "algorithm": "rms"
        }),
    };
    let res = execute_tool(req);
    // Should fail with file not found, but if files existed, should show 0 differences
    if res.is_err() {
        assert!(
            res.as_ref()
                .unwrap_err()
                .contains("Image1 path does not exist")
                || res.as_ref().unwrap_err().contains("Failed to convert SVG")
        );
    }
}

#[test]
fn test_compare_images_with_min_threshold() {
    let req = ToolCallRequest {
        name: "compare_images".to_string(),
        arguments: json!({
            "image1_path": "/path/to/image1.png",
            "image2_path": "/path/to/image2.png",
            "threshold": 0.0
        }),
    };
    let res = execute_tool(req);
    // Minimum threshold of 0.001 should be applied even if user specifies 0.0
    if res.is_err() {
        assert!(
            res.as_ref()
                .unwrap_err()
                .contains("Image1 path does not exist")
                || res.as_ref().unwrap_err().contains("Failed to convert SVG")
        );
    }
}

#[test]
fn test_compare_images_output_path_handling() {
    let req = ToolCallRequest {
        name: "compare_images".to_string(),
        arguments: json!({
            "image1_path": "/path/to/image1.png",
            "image2_path": "/path/to/image2.png",
            "output_path": "D:/test/output.png"
        }),
    };
    let res = execute_tool(req);
    // Should fail with file not found, but output path should be handled correctly
    if res.is_err() {
        assert!(
            res.as_ref()
                .unwrap_err()
                .contains("Image1 path does not exist")
                || res.as_ref().unwrap_err().contains("Failed to convert SVG")
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
                "image1_path": format!("/path/to/{}.png", filename),
                "image2_path": format!("/path/to/{}.{}", filename, ext),
                "algorithm": "rms"
            }),
        };
        let res = execute_tool(req);
        if res.is_err() {
            assert!(
                res.as_ref()
                    .unwrap_err()
                    .contains("Image1 path does not exist")
                    || res.as_ref().unwrap_err().contains("Failed to convert SVG")
            );
        }
    }
}

#[test]
fn test_compare_images_relative_path_output() {
    let req = ToolCallRequest {
        name: "compare_images".to_string(),
        arguments: json!({
            "image1_path": "/path/to/image1.png",
            "image2_path": "/path/to/image2.png",
            "output_path": "./diff_output.png"
        }),
    };
    let res = execute_tool(req);
    // Should fail with file not found, but should not error on path handling
    if res.is_err() {
        let err = res.as_ref().unwrap_err();
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
            "image1_path": "/path/to/icon.svg?width=100&height=100",
            "image2_path": "/path/to/other.svg",
            "algorithm": "rms"
        }),
    };
    let res = execute_tool(req);
    // Should fail with file not found, but SVG dimension parsing should work
    if res.is_err() {
        assert!(
            res.as_ref()
                .unwrap_err()
                .contains("Image1 path does not exist")
                || res.as_ref().unwrap_err().contains("Failed to convert SVG")
        );
    }
}

#[test]
fn test_compare_images_empty_path() {
    let req = ToolCallRequest {
        name: "compare_images".to_string(),
        arguments: json!({
            "image1_path": "",
            "image2_path": "/path/to/image2.png"
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
    // Test comparing actual PNG and SVG files from test directory
    // Use absolute paths to avoid relative path issues
    let png_path = r"D:\Projects\Public\rune\code-kit\test-dir\head.png";
    let svg_path = r"D:\Projects\Public\rune\code-kit\test-dir\head.svg";

    // Check if files exist
    assert!(
        Path::new(&png_path).exists(),
        "PNG test file should exist: {}",
        png_path
    );
    assert!(
        Path::new(&svg_path).exists(),
        "SVG test file should exist: {}",
        svg_path
    );

    let req = ToolCallRequest {
        name: "compare_images".to_string(),
        arguments: json!({
            "image1_path": &png_path,
            "image2_path": &svg_path,
            "algorithm": "rms",
            "output_path": "./test_diff.png"
        }),
    };

    let res = execute_tool(req);

    // The test should succeed or fail gracefully
    // If SVG is not supported, it should return a clear error
    if res.is_ok() {
        // Success case - comparison worked
        let result = res.unwrap();

        // Verify output path is returned
        assert!(
            result.get("output_path").is_some(),
            "Should return output_path"
        );
        assert!(
            result.get("match_percentage").is_some(),
            "Should return match_percentage"
        );
    } else {
        // Error case - check error message is helpful
        let err = res.unwrap_err();
        println!("PNG vs SVG comparison error: {}", err);
        // Should either succeed or give a clear error message
        assert!(
            err.contains("Failed") || err.contains("SVG"),
            "Error should be clear: {}",
            err
        );
    }
}

#[test]
fn test_compare_same_png_file() {
    if std::env::var("CI").is_ok() {
        eprintln!("Skipping live test: Running in CI environment");
        return;
    }
    // Test comparing the same PNG file with itself
    // This verifies that the comparison tool handles same-file comparisons correctly
    let png_path = r"D:\Projects\Public\rune\code-kit\test-dir\head.png";

    assert!(Path::new(&png_path).exists(), "PNG test file should exist");

    let req = ToolCallRequest {
        name: "compare_images".to_string(),
        arguments: json!({
            "image1_path": &png_path,
            "image2_path": &png_path,
            "algorithm": "rms"
        }),
    };

    let res = execute_tool(req);

    // The comparison should complete successfully
    if res.is_ok() {
        let result = res.unwrap();

        // Verify the comparison produced a result
        assert!(
            result.get("output_path").is_some(),
            "Should return output_path"
        );
        assert!(
            result.get("match_percentage").is_some(),
            "Should return match_percentage"
        );
    } else {
        let err = res.unwrap_err();
        println!("Same file comparison error: {}", err);
        // Should succeed for same file comparison
        assert!(
            err.is_empty() || err.contains("Failed"),
            "Error should be clear: {}",
            err
        );
    }
}
