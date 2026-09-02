use rune_git::operations::execute_tool;
use rune_pdk::ToolCallRequest;
use serde_json::json;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_git_init_status_commit_flow() {
    if std::env::var("CI").is_ok() {
        eprintln!("Skipping live test: Running in CI environment");
        return;
    }

    let dir = tempdir().unwrap();
    let repo_path = dir.path().to_str().unwrap();

    let res = execute_tool(ToolCallRequest {
        name: "git_init".to_string(),
        arguments: json!({ "repo_path": repo_path }),
    });
    assert!(res.is_ok());

    let dummy_file = dir.path().join("file.txt");
    fs::write(&dummy_file, "hello git").unwrap();

    let res = execute_tool(ToolCallRequest {
        name: "git_add".to_string(),
        arguments: json!({ "repo_path": repo_path, "files": ["file.txt"] }),
    });
    assert!(res.is_ok());

    let res = execute_tool(ToolCallRequest {
        name: "git_commit".to_string(),
        arguments: json!({
            "repo_path": repo_path,
            "message": "initial commit"
        }),
    });
    assert!(res.is_ok());

    let res = execute_tool(ToolCallRequest {
        name: "git_status".to_string(),
        arguments: json!({ "repo_path": repo_path }),
    })
    .unwrap();
    assert!(
        res["content"]
            .as_str()
            .unwrap()
            .contains("nothing to commit")
            || res["content"].as_str().unwrap().contains("clean")
    );
}

#[test]
fn test_git_missing_commit_message_rejection() {
    let res = execute_tool(ToolCallRequest {
        name: "git_commit".to_string(),
        arguments: json!({}),
    });
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Missing 'message' argument"));
}
