use rune_memory::operations::execute_tool;
use rune_pdk::ToolCallRequest;
use serde_json::json;

#[test]
fn memory_process_worker() {
    let Ok(path) = std::env::var("RUNE_MEMORY_RESTART_TEST_FILE") else {
        return;
    };
    let writing = std::env::var("RUNE_MEMORY_RESTART_TEST_WRITE").is_ok();
    let request = if writing {
        ToolCallRequest {
            name: "create_entities".into(),
            arguments: json!({
                "memoryFile": path,
                "entities": [{"name":"restart-evidence","entityType":"test","observations":[]}]
            }),
        }
    } else {
        ToolCallRequest {
            name: "read_graph".into(),
            arguments: json!({"memoryFile":path}),
        }
    };
    let result = execute_tool(request).unwrap();
    if !writing {
        assert_eq!(result["entities"][0]["name"], "restart-evidence");
    }
}

#[test]
fn graph_survives_process_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("memory.json");
    for writing in [true, false] {
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .args(["--exact", "memory_process_worker", "--nocapture"])
            .env("RUNE_MEMORY_RESTART_TEST_FILE", &path)
            .env_remove("RUNE_MEMORY_RESTART_TEST_WRITE");
        if writing {
            command.env("RUNE_MEMORY_RESTART_TEST_WRITE", "1");
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
