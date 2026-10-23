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
fn exactly_two_filesystem_tools_are_eligible() {
    let definitions: Vec<Value> = tool_definitions()
        .into_iter()
        .map(|tool| serde_json::to_value(tool).unwrap())
        .collect();
    let eligible: Vec<_> = definitions
        .iter()
        .filter(|tool| tool["annotations"]["readOnlyHint"] == true)
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();
    assert_eq!(eligible, ["read_text_file", "list_directory"]);
    for tool in definitions {
        assert_eq!(
            tool["annotations"]["readOnlyHint"],
            tool["annotations"]["readOnly"]
        );
        if tool["annotations"]["readOnlyHint"] == true {
            assert_eq!(
                tool["inputSchema"]["x-rune-rules"]["limits"]["root_param"],
                "allowed_dir"
            );
            assert_eq!(
                tool["inputSchema"]["x-rune-read-limits"]["maxContentBytes"],
                MAX_OUTPUT_BYTES
            );
        }
    }
}
