use rune_filesystem::operations::{execute_tool, normalize_path, resolve_path};
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
fn test_resolve_path_relative_prefix() {
    let p1 = resolve_path("./src/lib.rs").unwrap();
    assert_eq!(p1, PathBuf::from("src/lib.rs"));

    let p2 = resolve_path(".\\src\\lib.rs").unwrap();
    assert_eq!(p2, PathBuf::from("src/lib.rs"));
}

#[test]
fn test_directory_traversal_confinement() {
    let root = Path::new("/var/sandbox");
    let malicious_inputs = [
        "../../etc/passwd",
        "..\\..\\Windows\\System32\\config\\SAM",
        "nested/../../../../secret.txt",
    ];

    for input in malicious_inputs {
        let clean = input.trim_start_matches("./").trim_start_matches(".\\");
        let target = root.join(clean);
        let normalized = normalize_path(&target);
        assert!(
            !normalized.starts_with(root),
            "Failed to catch path escape: {}",
            input
        );
    }
}

#[test]
fn test_read_text_file_head() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("sample.txt");
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
    let file = dir.path().join("sample.txt");
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
    let file = dir.path().join("sample.txt");
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
    let file = dir.path().join("sample.txt");
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
    let file = dir.path().join("empty.txt");
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
    let bin_path = dir.path().join("corrupted.txt");
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
fn test_read_media_file_chunk() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("audio.mp3");
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
fn test_read_media_file_out_of_bounds() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("audio.mp3");
    let payload = vec![0x49, 0x44, 0x33, 0x00, 0x01, 0x02, 0x03, 0x04];
    fs::write(&file, &payload).unwrap();

    let req = ToolCallRequest {
        name: "read_media_file".to_string(),
        arguments: json!({ "path": file.to_str().unwrap(), "offset": 5000 }),
    };
    let res = execute_tool(req).unwrap();

    assert_eq!(res["paging"]["bytesReturned"], 0);
    assert_eq!(res["paging"]["hasMore"], false);
}

#[test]
fn test_read_media_file_zero_byte() {
    let dir = tempdir().unwrap();
    let empty_file = dir.path().join("empty.bin");
    fs::write(&empty_file, "").unwrap();

    let req = ToolCallRequest {
        name: "read_media_file".to_string(),
        arguments: json!({ "path": empty_file.to_str().unwrap() }),
    };
    let res = execute_tool(req).unwrap();

    assert_eq!(res["paging"]["totalBytes"], 0);
    assert_eq!(res["paging"]["bytesReturned"], 0);
    assert_eq!(res["paging"]["hasMore"], false);
}

#[test]
fn test_read_multiple_files() {
    let dir = tempdir().unwrap();
    let f1 = dir.path().join("a.txt");
    let f2 = dir.path().join("b.txt");
    fs::write(&f1, "Content A").unwrap();
    fs::write(&f2, "Content B").unwrap();

    let req = ToolCallRequest {
        name: "read_multiple_files".to_string(),
        arguments: json!({
            "paths": [f1.to_str().unwrap(), f2.to_str().unwrap(), "non_existent.txt"]
        }),
    };
    let res = execute_tool(req).unwrap();

    let text = res["content"].as_str().unwrap();
    assert!(text.contains("--- "));
    assert!(text.contains("Content A"));
    assert!(text.contains("Content B"));
    assert!(text.contains("[Error reading file:"));
}

#[test]
fn test_write_file_and_directory_creation() {
    let dir = tempdir().unwrap();
    let nested_file = dir.path().join("deep/nested/dir/new.txt");

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
    let file = dir.path().join("overwrite.txt");
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
fn test_edit_file_dry_run() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("code.rs");
    fs::write(&file, "let a = 1;\nlet b = 2;\nlet c = 3;").unwrap();

    let req = ToolCallRequest {
        name: "edit_file".to_string(),
        arguments: json!({
            "path": file.to_str().unwrap(),
            "dryRun": true,
            "edits": [{ "oldText": "let a = 1;", "newText": "let a = 100;" }]
        }),
    };
    let res = execute_tool(req).unwrap();

    assert!(res["content"].as_str().unwrap().contains("- let a = 1;"));
    assert_eq!(
        fs::read_to_string(&file).unwrap(),
        "let a = 1;\nlet b = 2;\nlet c = 3;"
    );
}

#[test]
fn test_edit_file_sequential_chaining() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("code.rs");
    fs::write(&file, "let a = 1;\nlet b = 2;\nlet c = 3;").unwrap();

    let req = ToolCallRequest {
        name: "edit_file".to_string(),
        arguments: json!({
            "path": file.to_str().unwrap(),
            "edits": [
                { "oldText": "let a = 1;", "newText": "let a = 10;" },
                { "oldText": "let a = 10;", "newText": "let a = 999;" }
            ]
        }),
    };
    let res = execute_tool(req);

    assert!(res.is_ok());
    assert_eq!(
        fs::read_to_string(&file).unwrap(),
        "let a = 999;\nlet b = 2;\nlet c = 3;"
    );
}

#[test]
fn test_move_file_collision_rejection() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src.txt");
    let dest = dir.path().join("dest.txt");
    fs::write(&src, "payload").unwrap();
    fs::write(&dest, "already exists").unwrap();

    let req = ToolCallRequest {
        name: "move_file".to_string(),
        arguments: json!({
            "source": src.to_str().unwrap(),
            "destination": dest.to_str().unwrap()
        }),
    };
    let res = execute_tool(req);

    assert!(res.is_err());
    assert!(res.unwrap_err().contains("already exists"));
}

#[test]
fn test_move_file_success() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src.txt");
    let dest = dir.path().join("renamed.txt");
    fs::write(&src, "payload").unwrap();

    let req = ToolCallRequest {
        name: "move_file".to_string(),
        arguments: json!({
            "source": src.to_str().unwrap(),
            "destination": dest.to_str().unwrap()
        }),
    };
    let res = execute_tool(req).unwrap();

    assert!(
        res["content"]
            .as_str()
            .unwrap()
            .contains("Successfully moved")
    );
    assert!(!src.exists());
    assert!(dest.exists());
}

#[test]
fn test_create_directory() {
    let dir = tempdir().unwrap();
    let target = dir.path().join("new_folder/sub_folder");

    let req = ToolCallRequest {
        name: "create_directory".to_string(),
        arguments: json!({ "path": target.to_str().unwrap() }),
    };
    let res = execute_tool(req).unwrap();

    assert!(res["content"].as_str().unwrap().contains("ready"));
    assert!(target.is_dir());
}

#[test]
fn test_list_directory() {
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("folder_a")).unwrap();
    fs::write(dir.path().join("small.txt"), "123").unwrap();

    let req = ToolCallRequest {
        name: "list_directory".to_string(),
        arguments: json!({ "path": dir.path().to_str().unwrap() }),
    };
    let res = execute_tool(req).unwrap();

    let lines = res["content"].as_str().unwrap();
    assert!(lines.contains("[DIR]  folder_a"));
    assert!(lines.contains("[FILE] small.txt"));
}

#[test]
fn test_list_directory_with_sizes() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("small.txt"), "123").unwrap();
    fs::write(dir.path().join("big.txt"), "1234567890").unwrap();

    let req = ToolCallRequest {
        name: "list_directory_with_sizes".to_string(),
        arguments: json!({ "path": dir.path().to_str().unwrap(), "sortBy": "size" }),
    };
    let res = execute_tool(req).unwrap();

    let size_lines: Vec<&str> = res["content"].as_str().unwrap().lines().collect();
    assert!(size_lines[0].contains("big.txt"));
    assert!(size_lines[1].contains("small.txt"));
}

#[test]
fn test_directory_tree_with_excludes() {
    let dir = tempdir().unwrap();
    let sub = dir.path().join("src");
    fs::create_dir(&sub).unwrap();
    fs::write(sub.join("main.rs"), "fn main() {}").unwrap();
    fs::write(sub.join("ignore.tmp"), "temp").unwrap();

    let req = ToolCallRequest {
        name: "directory_tree".to_string(),
        arguments: json!({ "path": dir.path().to_str().unwrap(), "excludePatterns": ["*.tmp"] }),
    };
    let res = execute_tool(req).unwrap();

    let tree: Value = serde_json::from_str(res["content"].as_str().unwrap()).unwrap();
    assert_eq!(tree["type"], "directory");
    let tree_str = res["content"].as_str().unwrap();
    assert!(tree_str.contains("main.rs"));
    assert!(!tree_str.contains("ignore.tmp"));
}

#[test]
fn test_search_files_pattern() {
    let dir = tempdir().unwrap();
    let sub = dir.path().join("src");
    fs::create_dir(&sub).unwrap();
    fs::write(sub.join("main.rs"), "fn main() {}").unwrap();

    let req = ToolCallRequest {
        name: "search_files".to_string(),
        arguments: json!({ "path": dir.path().to_str().unwrap(), "pattern": "*.rs" }),
    };
    let res = execute_tool(req).unwrap();

    let matches = res["content"].as_str().unwrap();
    assert!(matches.contains("main.rs"));
}

#[test]
fn test_get_file_info() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("info.txt");
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
fn test_unknown_tool_routing() {
    let req = ToolCallRequest {
        name: "non_existent".to_string(),
        arguments: json!({}),
    };
    let res = execute_tool(req);
    assert!(res.is_err());
    assert_eq!(res.unwrap_err(), "Unknown tool: non_existent");
}
