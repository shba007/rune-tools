// plugins/rune-memory/tests/operations_tests.rs
use rune_memory::operations::execute_tool;
use rune_pdk::ToolCallRequest;
use serde_json::json;
use tempfile::tempdir;

#[test]
fn test_entity_and_relation_lifecycle() {
    let dir = tempdir().unwrap();
    let mem_file = dir.path().join("test_memory.json");
    let mem_path = mem_file.to_str().unwrap();

    // 1. Create entities
    let create_req = ToolCallRequest {
        name: "create_entities".to_string(),
        arguments: json!({
            "memoryFile": mem_path,
            "entities": [
                {
                    "name": "Gemini",
                    "entityType": "AI",
                    "observations": ["Created by Google", "Multimodal model"]
                },
                {
                    "name": "Rust",
                    "entityType": "Language",
                    "observations": ["Memory safe", "No GC"]
                }
            ]
        }),
    };
    let res = execute_tool(create_req).unwrap();
    assert_eq!(res["created"].as_array().unwrap().len(), 2);

    // 2. Add observation to Gemini
    let obs_req = ToolCallRequest {
        name: "add_observations".to_string(),
        arguments: json!({
            "memoryFile": mem_path,
            "observations": [
                {
                    "entityName": "Gemini",
                    "contents": ["Supports tools"]
                }
            ]
        }),
    };
    let res = execute_tool(obs_req).unwrap();
    assert_eq!(
        res["updated"][0]["observations"].as_array().unwrap().len(),
        3
    );

    // 3. Create relation
    let rel_req = ToolCallRequest {
        name: "create_relations".to_string(),
        arguments: json!({
            "memoryFile": mem_path,
            "relations": [
                {
                    "from": "Gemini",
                    "to": "Rust",
                    "relationType": "implemented_in"
                }
            ]
        }),
    };
    let res = execute_tool(rel_req).unwrap();
    assert_eq!(res["created"].as_array().unwrap().len(), 1);

    // 4. Search nodes
    let search_req = ToolCallRequest {
        name: "search_nodes".to_string(),
        arguments: json!({
            "memoryFile": mem_path,
            "query": "multimodal"
        }),
    };
    let res = execute_tool(search_req).unwrap();
    assert_eq!(res["entities"].as_array().unwrap().len(), 1);
    assert_eq!(res["entities"][0]["name"], "Gemini");

    // 5. Open nodes
    let open_req = ToolCallRequest {
        name: "open_nodes".to_string(),
        arguments: json!({
            "memoryFile": mem_path,
            "names": ["Rust"]
        }),
    };
    let res = execute_tool(open_req).unwrap();
    assert_eq!(res["entities"].as_array().unwrap().len(), 1);
    assert_eq!(res["relations"].as_array().unwrap().len(), 1);

    // 6. Delete entity cascade
    let del_req = ToolCallRequest {
        name: "delete_entities".to_string(),
        arguments: json!({
            "memoryFile": mem_path,
            "entityNames": ["Rust"]
        }),
    };
    execute_tool(del_req).unwrap();

    // 7. Verify relations were purged with entity deletion
    let read_req = ToolCallRequest {
        name: "read_graph".to_string(),
        arguments: json!({ "memoryFile": mem_path }),
    };
    let res = execute_tool(read_req).unwrap();
    assert_eq!(res["entities"].as_array().unwrap().len(), 1);
    assert_eq!(res["relations"].as_array().unwrap().len(), 0);
}

#[test]
fn test_unknown_tool_routing() {
    let req = ToolCallRequest {
        name: "invalid_tool".to_string(),
        arguments: json!({}),
    };
    let res = execute_tool(req);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Unknown memory tool"));
}
