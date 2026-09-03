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

#[test]
fn test_validate_commit_message_exact_length_check() {
    let msg = "feat(rune-memory): upgrade to v0.2.0 with observation support, search, and graph inspection";
    let res = execute_tool(ToolCallRequest {
        name: "validate_commit_message".to_string(),
        arguments: json!({
            "message": msg,
            "max_subject_length": 100,
            "conventional": true
        }),
    })
    .unwrap();

    assert_eq!(res["valid"], true);
    assert_eq!(res["subject_length"], 91);
    assert_eq!(res["issues"].as_array().unwrap().len(), 0);

    // Default threshold is 72, which 91 exceeds
    let res_default = execute_tool(ToolCallRequest {
        name: "validate_commit_message".to_string(),
        arguments: json!({
            "message": msg,
            "conventional": true
        }),
    })
    .unwrap();

    assert_eq!(res_default["valid"], false);
    assert_eq!(res_default["subject_length"], 91);
    assert!(
        res_default["issues"][0]
            .as_str()
            .unwrap()
            .contains("Subject line exceeds 72 characters (actual: 91)")
    );
}

#[test]
fn test_validate_commit_message_conventional_rejection() {
    let res = execute_tool(ToolCallRequest {
        name: "validate_commit_message".to_string(),
        arguments: json!({
            "message": "bad commit message without type prefix",
            "conventional": true
        }),
    })
    .unwrap();

    assert_eq!(res["valid"], false);
    assert!(!res["issues"].as_array().unwrap().is_empty());
}
