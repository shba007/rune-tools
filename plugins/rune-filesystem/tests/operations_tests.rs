use rune_filesystem::operations::{execute_tool, normalize_path, resolve_path};
use rune_pdk::ToolCallRequest;
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

fn call(name: &str, args: Value) -> Result<Value, String> {
    execute_tool(ToolCallRequest {
        name: name.to_string(),
        arguments: args,
    })
}

// =========================================================================
// 1. Path Sanitization & Security Confinement
// =========================================================================
mod security {
    use super::*;

    #[test]
    fn test_path_normalization() {
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
    fn test_relative_prefix_resolution() {
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
}

// =========================================================================
// 2. Reading & Streaming (Happy Paths + Edge Cases)
// =========================================================================
mod reading_and_paging {
    use super::*;

    #[test]
    fn test_read_text_file_lifecycle() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("sample.txt");
        fs::write(&file, "Line 1\nLine 2\nLine 3\nLine 4\nLine 5").unwrap();
        let path = file.to_str().unwrap();

        // 1. Head mode
        let res_head = call("read_text_file", json!({ "path": path, "head": 2 })).unwrap();
        assert_eq!(res_head["content"], "Line 1\nLine 2");
        assert_eq!(res_head["linesReturned"], 2);

        // 2. Tail mode
        let res_tail = call("read_text_file", json!({ "path": path, "tail": 2 })).unwrap();
        assert_eq!(res_tail["content"], "Line 4\nLine 5");
        assert_eq!(res_tail["totalLines"], 5);

        // 3. Paging mode
        let res_page = call(
            "read_text_file",
            json!({ "path": path, "lineOffset": 1, "lineLimit": 2 }),
        )
        .unwrap();
        assert_eq!(res_page["content"], "Line 2\nLine 3");
        assert_eq!(res_page["hasMore"], true);
        assert_eq!(res_page["nextLineOffset"], 3);

        // 4. Out-of-bounds lineOffset
        let res_oob = call(
            "read_text_file",
            json!({ "path": path, "lineOffset": 100, "lineLimit": 10 }),
        )
        .unwrap();
        assert_eq!(res_oob["content"], "");
        assert_eq!(res_oob["linesReturned"], 0);
        assert_eq!(res_oob["hasMore"], false);
        assert_eq!(res_oob["nextLineOffset"], json!(null));
    }

    #[test]
    fn test_read_media_file_lifecycle() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("audio.mp3");
        let payload = vec![0x49, 0x44, 0x33, 0x00, 0x01, 0x02, 0x03, 0x04];
        fs::write(&file, &payload).unwrap();
        let path = file.to_str().unwrap();

        // Chunk read
        let res = call(
            "read_media_file",
            json!({ "path": path, "offset": 0, "length": 4 }),
        )
        .unwrap();
        assert_eq!(res["content"][0]["type"], "audio");
        assert_eq!(res["paging"]["totalBytes"], 8);
        assert_eq!(res["paging"]["bytesReturned"], 4);
        assert_eq!(res["paging"]["hasMore"], true);
        assert_eq!(res["paging"]["nextOffset"], 4);

        // Out-of-bounds byte offset
        let res_oob = call("read_media_file", json!({ "path": path, "offset": 5000 })).unwrap();
        assert_eq!(res_oob["paging"]["bytesReturned"], 0);
        assert_eq!(res_oob["paging"]["hasMore"], false);
    }

    #[test]
    fn test_read_multiple_files() {
        let dir = tempdir().unwrap();
        let f1 = dir.path().join("a.txt");
        let f2 = dir.path().join("b.txt");
        fs::write(&f1, "Content A").unwrap();
        fs::write(&f2, "Content B").unwrap();

        let res = call(
            "read_multiple_files",
            json!({
                "paths": [f1.to_str().unwrap(), f2.to_str().unwrap(), "non_existent.txt"]
            }),
        )
        .unwrap();

        let text = res["content"].as_str().unwrap();
        assert!(text.contains("--- "));
        assert!(text.contains("Content A"));
        assert!(text.contains("Content B"));
        assert!(text.contains("[Error reading file:"));
    }

    #[test]
    fn test_zero_byte_file_handling() {
        let dir = tempdir().unwrap();
        let empty_file = dir.path().join("empty.bin");
        fs::write(&empty_file, "").unwrap();
        let path = empty_file.to_str().unwrap();

        let res_txt = call("read_text_file", json!({ "path": path })).unwrap();
        assert_eq!(res_txt["content"], "");
        assert_eq!(res_txt["linesReturned"], 0);
        assert_eq!(res_txt["hasMore"], false);

        let res_media = call("read_media_file", json!({ "path": path })).unwrap();
        assert_eq!(res_media["paging"]["totalBytes"], 0);
        assert_eq!(res_media["paging"]["bytesReturned"], 0);
        assert_eq!(res_media["paging"]["hasMore"], false);
    }

    #[test]
    fn test_non_utf8_binary_rejection_in_text_reader() {
        let dir = tempdir().unwrap();
        let bin_path = dir.path().join("corrupted.txt");
        fs::write(&bin_path, vec![0xFF, 0xFE, 0xFD]).unwrap();

        let res = call(
            "read_text_file",
            json!({ "path": bin_path.to_str().unwrap() }),
        );
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Read error"));
    }
}

// =========================================================================
// 3. Mutations & File Modifications
// =========================================================================
mod mutations_and_edits {
    use super::*;

    #[test]
    fn test_write_file_and_truncation() {
        let dir = tempdir().unwrap();
        let nested_file = dir.path().join("deep/nested/dir/new.txt");
        let path = nested_file.to_str().unwrap();

        // 1. Initial write with auto-directory creation
        let res = call(
            "write_file",
            json!({ "path": path, "content": "A".repeat(5000) }),
        )
        .unwrap();
        assert!(
            res["content"]
                .as_str()
                .unwrap()
                .contains("Successfully wrote 5000 bytes")
        );
        assert_eq!(fs::metadata(&nested_file).unwrap().len(), 5000);

        // 2. Overwrite truncation check
        call("write_file", json!({ "path": path, "content": "short" })).unwrap();
        assert_eq!(fs::metadata(&nested_file).unwrap().len(), 5);
        assert_eq!(fs::read_to_string(&nested_file).unwrap(), "short");
    }

    #[test]
    fn test_edit_file_dry_run_and_sequential_chaining() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("code.rs");
        fs::write(&file, "let a = 1;\nlet b = 2;\nlet c = 3;").unwrap();
        let path = file.to_str().unwrap();

        // 1. Dry run should not modify disk
        let dry_res = call(
            "edit_file",
            json!({
                "path": path,
                "dryRun": true,
                "edits": [{ "oldText": "let a = 1;", "newText": "let a = 100;" }]
            }),
        )
        .unwrap();
        assert!(
            dry_res["content"]
                .as_str()
                .unwrap()
                .contains("- let a = 1;")
        );
        assert_eq!(
            fs::read_to_string(&file).unwrap(),
            "let a = 1;\nlet b = 2;\nlet c = 3;"
        );

        // 2. Sequential chained edits
        let edit_res = call(
            "edit_file",
            json!({
                "path": path,
                "edits": [
                    { "oldText": "let a = 1;", "newText": "let a = 10;" },
                    { "oldText": "let a = 10;", "newText": "let a = 999;" }
                ]
            }),
        );
        assert!(edit_res.is_ok());
        assert_eq!(
            fs::read_to_string(&file).unwrap(),
            "let a = 999;\nlet b = 2;\nlet c = 3;"
        );
    }

    #[test]
    fn test_move_file_and_collision_rejection() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src.txt");
        let dest = dir.path().join("dest.txt");
        fs::write(&src, "payload").unwrap();
        fs::write(&dest, "already exists").unwrap();

        // Target collision fails
        let fail_res = call(
            "move_file",
            json!({
                "source": src.to_str().unwrap(),
                "destination": dest.to_str().unwrap()
            }),
        );
        assert!(fail_res.is_err());
        assert!(fail_res.unwrap_err().contains("already exists"));

        // Clean move succeeds
        let clean_dest = dir.path().join("renamed.txt");
        let success_res = call(
            "move_file",
            json!({
                "source": src.to_str().unwrap(),
                "destination": clean_dest.to_str().unwrap()
            }),
        )
        .unwrap();
        assert!(
            success_res["content"]
                .as_str()
                .unwrap()
                .contains("Successfully moved")
        );
        assert!(!src.exists());
        assert!(clean_dest.exists());
    }

    #[test]
    fn test_create_directory() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("new_folder/sub_folder");
        let path = target.to_str().unwrap();

        let res = call("create_directory", json!({ "path": path })).unwrap();
        assert!(res["content"].as_str().unwrap().contains("ready"));
        assert!(target.is_dir());
    }
}

// =========================================================================
// 4. Discovery, Inspection & Metadata
// =========================================================================
mod discovery_and_metadata {
    use super::*;

    #[test]
    fn test_directory_listings_and_sizes() {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join("folder_a")).unwrap();
        fs::write(dir.path().join("small.txt"), "123").unwrap();
        fs::write(dir.path().join("big.txt"), "1234567890").unwrap();
        let path = dir.path().to_str().unwrap();

        // Basic list
        let res_list = call("list_directory", json!({ "path": path })).unwrap();
        let lines = res_list["content"].as_str().unwrap();
        assert!(lines.contains("[DIR]  folder_a"));
        assert!(lines.contains("[FILE] small.txt"));

        // Sorted by size
        let res_sizes = call(
            "list_directory_with_sizes",
            json!({ "path": path, "sortBy": "size" }),
        )
        .unwrap();
        let size_lines: Vec<&str> = res_sizes["content"].as_str().unwrap().lines().collect();
        assert!(size_lines[0].contains("big.txt"));
        assert!(size_lines[1].contains("small.txt"));
    }

    #[test]
    fn test_directory_tree_and_pattern_search() {
        let dir = tempdir().unwrap();
        let sub = dir.path().join("src");
        fs::create_dir(&sub).unwrap();
        fs::write(sub.join("main.rs"), "fn main() {}").unwrap();
        fs::write(sub.join("ignore.tmp"), "temp").unwrap();
        let path = dir.path().to_str().unwrap();

        // Recursive tree with excludes
        let res_tree = call(
            "directory_tree",
            json!({ "path": path, "excludePatterns": ["*.tmp"] }),
        )
        .unwrap();
        let tree: Value = serde_json::from_str(res_tree["content"].as_str().unwrap()).unwrap();
        assert_eq!(tree["type"], "directory");
        let tree_str = res_tree["content"].as_str().unwrap();
        assert!(tree_str.contains("main.rs"));
        assert!(!tree_str.contains("ignore.tmp"));

        // Pattern search
        let res_search = call("search_files", json!({ "path": path, "pattern": "*.rs" })).unwrap();
        let matches = res_search["content"].as_str().unwrap();
        assert!(matches.contains("main.rs"));
    }

    #[test]
    fn test_file_info_and_allowed_directories() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("info.txt");
        fs::write(&file, "metadata test").unwrap();
        let path = file.to_str().unwrap();

        // get_file_info
        let res_info = call("get_file_info", json!({ "path": path })).unwrap();
        let info: Value = serde_json::from_str(res_info["content"].as_str().unwrap()).unwrap();
        assert_eq!(info["is_file"], true);
        assert_eq!(info["size_bytes"], 13);
        assert!(info["modified"].as_str().is_some());

        // list_allowed_directories
        let res_allowed = call("list_allowed_directories", json!({})).unwrap();
        let allowed: Vec<String> =
            serde_json::from_str(res_allowed["content"].as_str().unwrap()).unwrap();
        assert!(!allowed.is_empty());
    }
}
