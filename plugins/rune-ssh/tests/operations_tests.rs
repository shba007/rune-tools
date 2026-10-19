// plugins/rune-ssh/tests/operations_tests.rs
use rune_pdk::ToolCallRequest;
use rune_ssh::operations::execute_tool;
use serde_json::json;
use std::env;
use std::fs;
use tempfile::NamedTempFile;

#[test]
fn test_missing_command_argument_rejection() {
    let req = ToolCallRequest {
        name: "ssh_run_command".to_string(),
        arguments: json!({}),
    };

    let result = execute_tool(req);
    assert!(result.is_err());
}

#[test]
fn test_explicit_override_connection_refusal() {
    let req = ToolCallRequest {
        name: "ssh_run_command".to_string(),
        arguments: json!({
            "host": "127.0.0.1:65535",
            "username": "root",
            "privateKeyPath": "./dummy_key.pem",
            "command": "whoami"
        }),
    };

    let result = execute_tool(req);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("Failed to connect") || err.contains("refused") || err.contains("os error")
    );
}

#[test]
fn test_production_env_lifecycle() {
    if env::var("SSH_HOST").is_err()
        || env::var("SSH_USER").is_err()
        || env::var("SSH_KEY_PATH").is_err()
    {
        eprintln!(
            "Skipping live test: SSH_HOST, SSH_USER, or SSH_KEY_PATH is not configured in .env"
        );
        return;
    }

    // 1. SSH execution using environment variables (no host/user/key in arguments)
    let token = "rune_ssh_env_ok";
    let exec_req = ToolCallRequest {
        name: "ssh_run_command".to_string(),
        arguments: json!({
            "command": format!("echo {}", token)
        }),
    };

    let exec_res = execute_tool(exec_req).expect("Failed running command with env defaults");
    assert_eq!(exec_res["status"], "success");
    assert!(exec_res["output"].as_str().unwrap().contains(token));

    // 2. SFTP Upload using env defaults
    let local_file = NamedTempFile::new().unwrap();
    let content = "Env-configured SFTP lifecycle test";
    fs::write(local_file.path(), content).unwrap();

    let remote_path = format!("/tmp/rune_env_test_{}.txt", std::process::id());
    let upload_req = ToolCallRequest {
        name: "sftp_upload".to_string(),
        arguments: json!({
            "localPath": local_file.path().to_str().unwrap(),
            "remotePath": remote_path
        }),
    };
    let upload_res = execute_tool(upload_req).expect("Failed SFTP upload with env defaults");
    assert_eq!(upload_res["status"], "success");

    // 3. SFTP Download using env defaults
    let download_target = NamedTempFile::new().unwrap();
    let download_req = ToolCallRequest {
        name: "sftp_download".to_string(),
        arguments: json!({
            "remotePath": remote_path,
            "localPath": download_target.path().to_str().unwrap()
        }),
    };
    let download_res = execute_tool(download_req).expect("Failed SFTP download with env defaults");
    assert_eq!(download_res["status"], "success");
    assert_eq!(fs::read_to_string(download_target.path()).unwrap(), content);

    // 4. Remote cleanup
    let _ = execute_tool(ToolCallRequest {
        name: "ssh_run_command".to_string(),
        arguments: json!({
            "command": format!("rm -f {}", remote_path)
        }),
    });
}
