use rune_filesystem::{definitions::tool_definitions, pilot_reads::*};
use rune_pdk::ToolCallRequest;
use serde_json::{Value, json};
use std::fs;

fn call(root: Option<&str>, name: &str, arguments: Value) -> Result<Value, String> {
    execute(
        ToolCallRequest {
            name: name.into(),
            arguments,
        },
        root,
    )
}

#[test]
fn root_is_required_and_cannot_be_overridden() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_str().unwrap();
    for absent in [None, Some(""), Some("."), Some("Z:/does-not-exist-rune")] {
        assert!(call(absent, "list_directory", json!({"path":"."})).is_err());
    }
    for key in ["allowedDir", "allowed_dir"] {
        assert!(
            call(Some(root), "list_directory", json!({"path":".",key:root}))
                .unwrap_err()
                .contains("overrides")
        );
    }
}

#[test]
fn paths_reject_traversal_prefix_and_windows_special_names() {
    let root = "D:/fixture/root";
    for path in [
        "../secret",
        "child/../../secret",
        "D:/fixture/root-other/secret",
        "/etc/passwd",
        "C:\\secret",
        "a:stream",
        "CON.txt",
        "dir/../file",
        "trailing.",
        "NUL",
        "\\\\?\\C:\\secret",
    ] {
        assert!(relative_path(path, root).is_err(), "accepted {path}");
    }
    assert_eq!(
        relative_path("D:/fixture/root/child/file", root).unwrap(),
        std::path::Path::new("child/file")
    );
    assert_eq!(
        relative_path("child\\file", root).unwrap(),
        std::path::Path::new("child/file")
    );
}

#[test]
fn text_paging_tail_and_budgets() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_str().unwrap();
    fs::write(dir.path().join("note"), "one\ntwo\nthree\n").unwrap();
    let page = call(
        Some(root),
        "read_text_file",
        json!({"path":"note","lineOffset":1,"lineLimit":1}),
    )
    .unwrap();
    assert_eq!(page["content"], "two");
    assert_eq!(page["nextLineOffset"], 2);
    assert_eq!(page["hasMore"], true);
    let tail = call(
        Some(root),
        "read_text_file",
        json!({"path":"note","tail":1}),
    )
    .unwrap();
    assert_eq!(tail["content"], "three");
    for args in [
        json!({"path":"note","head":2001}),
        json!({"path":"note","tail":-1}),
        json!({"path":"note","lineLimit":1.5}),
        json!({"path":"note","head":1,"tail":1}),
    ] {
        assert!(call(Some(root), "read_text_file", args).is_err());
    }
    fs::write(
        dir.path().join("pages"),
        ("x".repeat(100) + "\n").repeat(3000),
    )
    .unwrap();
    let page = call(Some(root), "read_text_file", json!({"path":"pages"})).unwrap();
    assert!(page["content"].as_str().unwrap().len() <= MAX_OUTPUT_BYTES);
    assert!(page["linesReturned"].as_u64().unwrap() <= MAX_LINES as u64);
    assert_eq!(page["hasMore"], true);
    fs::write(dir.path().join("long"), vec![b'x'; MAX_OUTPUT_BYTES + 1]).unwrap();
    assert!(
        call(Some(root), "read_text_file", json!({"path":"long"}))
            .unwrap_err()
            .contains("line")
    );
    fs::write(dir.path().join("large"), vec![b'x'; MAX_FILE_BYTES + 1]).unwrap();
    assert!(
        call(Some(root), "read_text_file", json!({"path":"large"}))
            .unwrap_err()
            .contains("input limit")
    );
}

#[test]
fn directory_is_bounded_with_explicit_truncation() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..MAX_ENTRIES + 1 {
        fs::write(dir.path().join(format!("file{i}")), "").unwrap();
    }
    let root = dir.path().to_str().unwrap();
    let page = call(Some(root), "list_directory", json!({"path":"."})).unwrap();
    assert_eq!(page["entriesReturned"], MAX_ENTRIES);
    assert_eq!(page["truncated"], true);
    assert!(page["content"].as_str().unwrap().len() <= MAX_OUTPUT_BYTES);
    assert!(
        call(
            Some(root),
            "list_directory",
            json!({"path":".","maxEntries":201})
        )
        .is_err()
    );
}

#[test]
fn only_confined_read_tools_are_strictly_read_only() {
    // Every tool here reads through a root-confined capability (or, staged, the host
    // broker); confinement is tested in `read_tools` and `staged_tests`.
    let strict = [
        "read_text_file",
        "read_media_file",
        "read_multiple_files",
        "list_directory",
        "list_directory_with_sizes",
        "directory_tree",
        "search_files",
        "get_file_info",
        "list_allowed_directories",
    ];
    let definitions: Vec<Value> = tool_definitions()
        .into_iter()
        .map(|tool| serde_json::to_value(tool).unwrap())
        .collect();
    let eligible: Vec<_> = definitions
        .iter()
        .filter(|tool| tool["annotations"]["readOnlyHint"] == true)
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();
    assert_eq!(eligible, strict);
    for tool in definitions {
        assert_eq!(
            tool["annotations"]["readOnlyHint"],
            tool["annotations"]["readOnly"]
        );
        let rules = &tool["inputSchema"]["x-rune-rules"];
        if tool["annotations"]["readOnlyHint"] == true {
            assert_eq!(rules["limits"]["root_param"], "allowed_dir");
            assert_eq!(rules["effects"], json!(["read"]));
            assert_eq!(rules["risk"], "low");
            if matches!(
                tool["name"].as_str().unwrap(),
                "read_text_file" | "list_directory"
            ) {
                assert_eq!(
                    tool["inputSchema"]["x-rune-read-limits"]["maxContentBytes"],
                    MAX_OUTPUT_BYTES
                );
            }
        } else {
            // Writers stay non-strict and keep their root and effects.
            assert_eq!(rules["limits"]["root_param"], "allowed_dir");
            assert_ne!(rules["effects"], json!(["read"]));
        }
    }
}

#[test]
fn a_missing_path_is_named_as_given_without_an_os_error_code() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_str().unwrap();
    fs::write(dir.path().join("Expenses.txt"), "15 Nov\n49 internet\n").unwrap();
    let error = call(Some(root), "read_text_file", json!({"path":"missing.txt"})).unwrap_err();
    assert!(
        error.contains("\"missing.txt\"") && error.contains("no such file or directory"),
        "{error}"
    );
    assert!(!error.contains("os error"), "{error}");
    assert!(
        !error.contains("marker"),
        "a plain missing name gets no marker hint: {error}"
    );
    let missing_dir = call(Some(root), "list_directory", json!({"path":"nowhere"})).unwrap_err();
    assert!(
        missing_dir.contains("Cannot list directory \"nowhere\""),
        "{missing_dir}"
    );
}

#[test]
fn a_whole_listing_row_used_as_a_path_is_refused_with_a_hint() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_str().unwrap();
    fs::write(dir.path().join("Expenses.txt"), "15 Nov\n49 internet\n").unwrap();
    fs::create_dir(dir.path().join("sub")).unwrap();
    for (tool, path) in [
        ("read_text_file", "[FILE] Expenses.txt"),
        ("read_text_file", "  [FILE] Expenses.txt"),
        ("list_directory", "[DIR] sub"),
        ("read_text_file", "[LINK] Expenses.txt"),
    ] {
        let error = call(Some(root), tool, json!({"path":path})).unwrap_err();
        assert!(error.contains(&format!("\"{path}\"")), "{error}");
        assert!(
            error.contains("without the [FILE], [DIR] or [LINK] marker"),
            "{error}"
        );
    }
    let ok = call(Some(root), "read_text_file", json!({"path":"Expenses.txt"})).unwrap();
    assert!(ok["content"].as_str().unwrap().contains("49 internet"));
}

#[test]
fn a_directory_given_to_read_text_file_names_the_path() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_str().unwrap();
    fs::create_dir(dir.path().join("sub")).unwrap();
    let error = call(Some(root), "read_text_file", json!({"path":"sub"})).unwrap_err();
    assert!(
        error.contains("\"sub\"") && error.contains("not a regular text file"),
        "{error}"
    );
}
