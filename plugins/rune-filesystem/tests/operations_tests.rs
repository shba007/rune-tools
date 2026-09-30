use base64::Engine;
use rune_filesystem::operations::{
    execute_tool, normalize_path, resolve_path, resolve_path_with_root,
};
use rune_pdk::ToolCallRequest;
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

#[test]
fn test_normalize_path() {
    assert_eq!(
        normalize_path(Path::new("a/b/../c/./d")),
        PathBuf::from("a/c/d")
    );
    assert_eq!(
        normalize_path(Path::new("./foo/bar/../../baz")),
        PathBuf::from("baz")
    );
    assert_eq!(
        normalize_path(Path::new("a/b/c/../../../d")),
        PathBuf::from("d")
    );
}

#[test]
fn test_search_files_pattern() {
    let dir = tempdir().unwrap();
    let search_dir = dir.path().join("patterns_search");
    fs::create_dir_all(&search_dir).unwrap();
    let f1 = search_dir.join("test1.txt");
    let f2 = search_dir.join("test2.txt");
    fs::write(&f1, "content 1").unwrap();
    fs::write(&f2, "content 2").unwrap();
    fs::write(search_dir.join("other.txt"), "no match").unwrap();

    let req = ToolCallRequest {
        name: "search_files".to_string(),
        arguments: json!({ "path": search_dir.to_str().unwrap(), "pattern": "*.txt" }),
    };
    let res = execute_tool(req);

    assert!(res.is_ok());
}

#[test]
fn test_get_file_info() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("info_001.txt");
    fs::write(&file, "metadata test").unwrap();

    let req = ToolCallRequest {
        name: "get_file_info".to_string(),
        arguments: json!({ "path": file.to_str().unwrap() }),
    };
    let res = execute_tool(req).unwrap();

    let info: Value = serde_json::from_str(res["content"].as_str().unwrap()).unwrap();
    assert_eq!(info["is_file"], true);
    assert_eq!(info["size_bytes"], 13);
    assert!(info["modified"].as_str().is_some());
}

#[test]
fn test_write_file_and_directory_creation() {
    let dir = tempdir().unwrap();
    let nested_file = dir.path().join("deep/nested/dir/write_test.txt");

    let req = ToolCallRequest {
        name: "write_file".to_string(),
        arguments: json!({ "path": nested_file.to_str().unwrap(), "content": "A".repeat(5000) }),
    };
    let res = execute_tool(req).unwrap();

    assert!(
        res["content"]
            .as_str()
            .unwrap()
            .contains("Successfully wrote 5000 bytes")
    );
    assert_eq!(fs::metadata(&nested_file).unwrap().len(), 5000);
}

#[test]
fn test_write_file_overwrite_truncation() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("overwrite_001.txt");
    fs::write(&file, "initial long content to be truncated").unwrap();

    let req = ToolCallRequest {
        name: "write_file".to_string(),
        arguments: json!({ "path": file.to_str().unwrap(), "content": "short" }),
    };
    let res = execute_tool(req);
    assert!(res.is_ok());

    assert_eq!(fs::metadata(&file).unwrap().len(), 5);
    assert_eq!(fs::read_to_string(&file).unwrap(), "short");
}

#[test]
fn test_directory_tree_with_excludes() {
    let dir = tempdir().unwrap();
    let sub = dir.path().join("dir_tree_test");
    fs::create_dir(&sub).unwrap();
    fs::write(sub.join("main.rs"), "fn main() {}").unwrap();
    fs::write(sub.join("ignore.tmp"), "temp").unwrap();

    let req = ToolCallRequest {
        name: "directory_tree".to_string(),
        arguments: json!({ "path": sub.to_str().unwrap(), "excludePatterns": ["*.tmp"] }),
    };
    let res = execute_tool(req).unwrap();

    let tree: Value = serde_json::from_str(res["content"].as_str().unwrap()).unwrap();
    assert_eq!(tree["type"], "directory");
    let tree_str = res["content"].as_str().unwrap();
    assert!(tree_str.contains("main.rs"));
    assert!(!tree_str.contains("ignore.tmp"));
}

#[test]
fn test_edit_file_dry_run() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("edit_dry_001.rs");
    fs::write(&file, "let a = 1;\nlet b = 2;\nlet c = 3;").unwrap();

    let req = ToolCallRequest {
        name: "edit_file".to_string(),
        arguments: json!({
            "path": file.to_str().unwrap(),
            "edits": [
                { "oldText": "let a = 1;", "newText": "let a = 10;" },
                { "oldText": "let c = 3;", "newText": "let c = 30;" }
            ],
            "dryRun": true
        }),
    };
    let _res = execute_tool(req).unwrap();

    assert_eq!(
        fs::read_to_string(&file).unwrap(),
        "let a = 1;\nlet b = 2;\nlet c = 3;"
    );
}

#[test]
fn test_edit_file_sequential_chaining() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("edit_seq_001.rs");
    fs::write(&file, "let a = 1;\nlet b = 2;\nlet c = 3;").unwrap();

    let req = ToolCallRequest {
        name: "edit_file".to_string(),
        arguments: json!({
            "path": file.to_str().unwrap(),
            "edits": [
                { "oldText": "let a = 1;", "newText": "let a = 10;" },
                { "oldText": "let b = 2;", "newText": "let b = 20;" },
                { "oldText": "let c = 3;", "newText": "let c = 30;" }
            ]
        }),
    };
    let res = execute_tool(req);

    assert!(res.is_ok());

    let content = fs::read_to_string(&file).unwrap();
    assert!(content.contains("let a = 10;"));
    assert!(content.contains("let b = 20;"));
    assert!(content.contains("let c = 30;"));
}

#[test]
fn test_move_file_success() {
    let dir = tempdir().unwrap();
    let old_file = dir.path().join("move_old.txt");
    let new_file = dir.path().join("move_new.txt");
    fs::write(&old_file, "content").unwrap();

    let req = ToolCallRequest {
        name: "move_file".to_string(),
        arguments: json!({
            "source": old_file.to_str().unwrap(),
            "destination": new_file.to_str().unwrap()
        }),
    };
    let _res = execute_tool(req).unwrap();

    assert!(!old_file.exists());
    assert!(new_file.exists());
    assert_eq!(fs::read_to_string(&new_file).unwrap(), "content");
}

#[test]
fn test_move_file_collision_rejection() {
    let dir = tempdir().unwrap();
    let old_file = dir.path().join("move_old_002.txt");
    let new_file = dir.path().join("move_new_002.txt");
    fs::write(&old_file, "content").unwrap();
    fs::write(&new_file, "existing content").unwrap();

    let req = ToolCallRequest {
        name: "move_file".to_string(),
        arguments: json!({
            "source": old_file.to_str().unwrap(),
            "destination": new_file.to_str().unwrap()
        }),
    };
    let res = execute_tool(req);

    assert!(res.is_err());
}

#[test]
fn test_unknown_tool_routing() {
    let req = ToolCallRequest {
        name: "non_existent".to_string(),
        arguments: json!({}),
    };
    let res = execute_tool(req);

    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Unknown tool"));
}

#[test]
fn test_create_directory() {
    let dir = tempdir().unwrap();
    let new_dir = dir.path().join("create_test");

    let req = ToolCallRequest {
        name: "create_directory".to_string(),
        arguments: json!({ "path": new_dir.to_str().unwrap() }),
    };
    let _res = execute_tool(req).unwrap();

    assert!(new_dir.exists());
}

#[test]
fn test_list_directory() {
    let dir = tempdir().unwrap();
    let subdir = dir.path().join("list_dir_test");
    let test_file = subdir.join("list_test_file.txt");
    fs::create_dir(&subdir).unwrap();
    fs::write(&test_file, "content").unwrap();

    let req = ToolCallRequest {
        name: "list_directory".to_string(),
        arguments: json!({ "path": subdir.to_str().unwrap() }),
    };
    let res = execute_tool(req);

    assert!(res.is_ok());
}

#[test]
fn test_list_directory_with_sizes() {
    let dir = tempdir().unwrap();
    let f1 = dir.path().join("list_f1.txt");
    let f2 = dir.path().join("list_f2.txt");
    fs::write(&f1, "content one").unwrap();
    fs::write(&f2, "content two").unwrap();

    let req = ToolCallRequest {
        name: "list_directory".to_string(),
        arguments: json!({ "path": dir.path().to_str().unwrap() }),
    };
    let res = execute_tool(req);

    assert!(res.is_ok());
}

#[test]
fn test_list_allowed_directories() {
    let req = ToolCallRequest {
        name: "list_allowed_directories".to_string(),
        arguments: json!({}),
    };
    let res = execute_tool(req).unwrap();

    let allowed: Vec<String> = serde_json::from_str(res["content"].as_str().unwrap()).unwrap();
    assert!(!allowed.is_empty());
}

#[test]
fn test_read_text_file_head() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("read_head.txt");
    fs::write(&file, "Line 1\nLine 2\nLine 3\nLine 4\nLine 5").unwrap();

    let req = ToolCallRequest {
        name: "read_text_file".to_string(),
        arguments: json!({ "path": file.to_str().unwrap(), "head": 2 }),
    };
    let res = execute_tool(req).unwrap();

    assert_eq!(res["content"], "Line 1\nLine 2");
    assert_eq!(res["linesReturned"], 2);
}

#[test]
fn test_read_text_file_tail() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("read_tail.txt");
    fs::write(&file, "Line 1\nLine 2\nLine 3\nLine 4\nLine 5").unwrap();

    let req = ToolCallRequest {
        name: "read_text_file".to_string(),
        arguments: json!({ "path": file.to_str().unwrap(), "tail": 2 }),
    };
    let res = execute_tool(req).unwrap();

    assert_eq!(res["content"], "Line 4\nLine 5");
    assert_eq!(res["totalLines"], 5);
}

#[test]
fn test_read_text_file_paging() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("read_page.txt");
    fs::write(&file, "Line 1\nLine 2\nLine 3\nLine 4\nLine 5").unwrap();

    let req = ToolCallRequest {
        name: "read_text_file".to_string(),
        arguments: json!({ "path": file.to_str().unwrap(), "lineOffset": 1, "lineLimit": 2 }),
    };
    let res = execute_tool(req).unwrap();

    assert_eq!(res["content"], "Line 2\nLine 3");
    assert_eq!(res["hasMore"], true);
    assert_eq!(res["nextLineOffset"], 3);
}

#[test]
fn test_read_text_file_out_of_bounds() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("read_ob.txt");
    fs::write(&file, "Line 1\nLine 2\nLine 3\nLine 4\nLine 5").unwrap();

    let req = ToolCallRequest {
        name: "read_text_file".to_string(),
        arguments: json!({ "path": file.to_str().unwrap(), "lineOffset": 100, "lineLimit": 10 }),
    };
    let res = execute_tool(req).unwrap();

    assert_eq!(res["content"], "");
    assert_eq!(res["linesReturned"], 0);
    assert_eq!(res["hasMore"], false);
    assert_eq!(res["nextLineOffset"], json!(null));
}

#[test]
fn test_read_text_file_zero_byte() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("read_zero.txt");
    fs::write(&file, "").unwrap();

    let req = ToolCallRequest {
        name: "read_text_file".to_string(),
        arguments: json!({ "path": file.to_str().unwrap() }),
    };
    let res = execute_tool(req).unwrap();

    assert_eq!(res["content"], "");
    assert_eq!(res["linesReturned"], 0);
    assert_eq!(res["hasMore"], false);
}

#[test]
fn test_read_text_file_non_utf8_binary_rejection() {
    let dir = tempdir().unwrap();
    let bin_path = dir.path().join("corrupted.bin");
    fs::write(&bin_path, vec![0xFF, 0xFE, 0xFD]).unwrap();

    let req = ToolCallRequest {
        name: "read_text_file".to_string(),
        arguments: json!({ "path": bin_path.to_str().unwrap() }),
    };
    let res = execute_tool(req);

    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Read error"));
}

#[test]
fn test_read_media_file_image() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("media_img.png");
    let payload = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x01];
    fs::write(&file, &payload).unwrap();

    let req = ToolCallRequest {
        name: "read_media_file".to_string(),
        arguments: json!({ "path": file.to_str().unwrap() }),
    };
    let res = execute_tool(req).unwrap();

    let expected_b64 = base64::engine::general_purpose::STANDARD.encode(&payload);

    assert_eq!(res["content"][0]["type"], "image");
    assert_eq!(res["content"][0]["mimeType"], "image/png");
    assert_eq!(res["content"][0]["data"], expected_b64);
    assert_eq!(res["paging"]["totalBytes"], 10);
    assert_eq!(res["paging"]["bytesReturned"], 10);
    assert_eq!(res["paging"]["hasMore"], false);
}

#[test]
fn test_read_media_file_chunk() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("media_chunk.mp3");
    let payload = vec![0x49, 0x44, 0x33, 0x00, 0x01, 0x02, 0x03, 0x04];
    fs::write(&file, &payload).unwrap();

    let req = ToolCallRequest {
        name: "read_media_file".to_string(),
        arguments: json!({ "path": file.to_str().unwrap(), "offset": 0, "length": 4 }),
    };
    let res = execute_tool(req).unwrap();

    assert_eq!(res["content"][0]["type"], "audio");
    assert_eq!(res["paging"]["totalBytes"], 8);
    assert_eq!(res["paging"]["bytesReturned"], 4);
    assert_eq!(res["paging"]["hasMore"], true);
    assert_eq!(res["paging"]["nextOffset"], 4);
}

#[test]
fn test_read_media_file_resource() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("media_res.zip");
    let payload = vec![0x50, 0x4B, 0x03, 0x04, 0x05];
    fs::write(&file, &payload).unwrap();

    let req = ToolCallRequest {
        name: "read_media_file".to_string(),
        arguments: json!({ "path": file.to_str().unwrap() }),
    };
    let res = execute_tool(req).unwrap();

    assert_ne!(res["content"][0]["mimeType"].as_str(), Some("image/png"));
    assert_eq!(res["paging"]["totalBytes"], 5);
    assert_eq!(res["paging"]["bytesReturned"], 5);
    assert_eq!(res["paging"]["hasMore"], false);
}

#[test]
fn test_read_media_file_zero_byte() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("media_zero.mp3");
    fs::write(&file, vec![]).unwrap();

    let req = ToolCallRequest {
        name: "read_media_file".to_string(),
        arguments: json!({ "path": file.to_str().unwrap() }),
    };
    let res = execute_tool(req).unwrap();

    assert_eq!(res["content"][0]["type"], "audio");
    assert_eq!(res["content"][0]["mimeType"], "audio/mpeg");
    assert_eq!(res["paging"]["totalBytes"], 0);
    assert_eq!(res["paging"]["bytesReturned"], 0);
    assert_eq!(res["paging"]["hasMore"], false);
}

#[test]
fn test_read_media_file_out_of_bounds() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("media_ob.mp3");
    let content = vec![0xFF; 1000];
    fs::write(&file, &content).unwrap();

    let req = ToolCallRequest {
        name: "read_media_file".to_string(),
        arguments: json!({ "path": file.to_str().unwrap(), "limit": 500 }),
    };
    let res = execute_tool(req).unwrap();

    assert_eq!(res["content"][0]["type"], "audio");
    assert_eq!(res["content"][0]["mimeType"], "audio/mpeg");
    assert_eq!(res["paging"]["totalBytes"], 1000);
    assert_eq!(res["paging"]["bytesReturned"], 1000);
    assert_eq!(res["paging"]["hasMore"], false);
}

#[test]
fn test_read_media_file_large() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("media_large.mp3");
    fs::write(&file, vec![0xFF; 50000]).unwrap();

    let req = ToolCallRequest {
        name: "read_media_file".to_string(),
        arguments: json!({ "path": file.to_str().unwrap() }),
    };
    let res = execute_tool(req).unwrap();

    assert_eq!(res["content"][0]["type"], "audio");
    assert_eq!(res["paging"]["totalBytes"], 50000);
    assert_eq!(res["paging"]["bytesReturned"], 50000);
    assert_eq!(res["paging"]["hasMore"], false);
}

#[test]
fn test_read_multiple_files() {
    let dir = tempdir().unwrap();
    let f1 = dir.path().join("read_multi_1.txt");
    let f2 = dir.path().join("read_multi_2.txt");
    fs::write(&f1, "content one").unwrap();
    fs::write(&f2, "content two").unwrap();

    let req = ToolCallRequest {
        name: "read_multiple_files".to_string(),
        arguments: json!({ "paths": [f1.to_str().unwrap(), f2.to_str().unwrap()] }),
    };
    let res = execute_tool(req);

    assert!(res.is_ok());
}

#[test]
fn test_resolve_path_relative_prefix() {
    let p1 = resolve_path("./src/lib.rs").unwrap();
    assert_eq!(p1, PathBuf::from("src/lib.rs"));

    let p2 = resolve_path(".\\src\\lib.rs").unwrap();
    assert_eq!(p2, PathBuf::from("src/lib.rs"));
}

#[test]
fn test_resolve_path_strips_redundant_allowed_root() {
    // Windows path resolution
    let root = Some("D:/Projects/Public/rune/code-tools/temp/cookies");

    let p1 = resolve_path_with_root("index.html", root).unwrap();
    assert_eq!(
        p1,
        PathBuf::from("D:/Projects/Public/rune/code-tools/temp/cookies/index.html")
    );

    let p2 = resolve_path_with_root("images/screenshots", root).unwrap();
    assert_eq!(
        p2,
        PathBuf::from("D:/Projects/Public/rune/code-tools/temp/cookies/images/screenshots")
    );

    let p3 = resolve_path_with_root("./images/screenshots", root).unwrap();
    assert_eq!(
        p3,
        PathBuf::from("D:/Projects/Public/rune/code-tools/temp/cookies/images/screenshots")
    );

    let p4 = resolve_path_with_root(
        "D:/Projects/Public/rune/code-tools/temp/cookies/photo.png",
        root,
    )
    .unwrap();
    assert_eq!(
        p4,
        PathBuf::from("D:/Projects/Public/rune/code-tools/temp/cookies/photo.png")
    );

    let p5 = resolve_path_with_root("C:/Windows/System32", root);
    assert!(p5.is_err());

    // Unix path resolution
    let unix_root = Some("/tmp/cookies");
    let u1 = resolve_path_with_root("index.html", unix_root).unwrap();
    assert_eq!(u1, PathBuf::from("/tmp/cookies/index.html"));

    let u2 = resolve_path_with_root("/etc/passwd", unix_root);
    assert!(u2.is_err());

    let u3 = resolve_path_with_root("/tmp/cookies/../../etc/passwd", unix_root);
    assert!(u3.is_err());
    assert!(u3.unwrap_err().contains("outside allowed directory"));
}
