use rune_pdk::ToolCallRequest;
use rune_sequential_thinking::operations::{execute_tool, reset_session};
use serde_json::json;
use std::sync::Mutex;

static TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn test_sequential_thinking_progression() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset_session();

    // Step 1: Initial hypothesis
    let req1 = ToolCallRequest {
        name: "sequential_thinking".to_string(),
        arguments: json!({
            "thought": "Initial problem analysis: checking base assumptions",
            "thoughtNumber": 1,
            "totalThoughts": 3,
            "nextThoughtNeeded": true
        }),
    };
    let res1 = execute_tool(req1).unwrap();
    assert_eq!(res1["thoughtNumber"], 1);
    assert_eq!(res1["totalThoughts"], 3);
    assert_eq!(res1["nextThoughtNeeded"], true);
    assert_eq!(res1["thoughtHistoryLength"], 1);

    // Step 2: Dynamic expansion where thought number exceeds initial total
    let req2 = ToolCallRequest {
        name: "sequential_thinking".to_string(),
        arguments: json!({
            "thought": "Deeper analysis: problem requires 4 steps instead of 3",
            "thoughtNumber": 4,
            "totalThoughts": 3,
            "nextThoughtNeeded": true
        }),
    };
    let res2 = execute_tool(req2).unwrap();
    assert_eq!(res2["thoughtNumber"], 4);
    assert_eq!(res2["totalThoughts"], 4);
    assert_eq!(res2["thoughtHistoryLength"], 2);
}

#[test]
fn test_branching_and_revision() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset_session();

    // Step 1: Base thought
    let req1 = ToolCallRequest {
        name: "sequential_thinking".to_string(),
        arguments: json!({
            "thought": "Standard approach",
            "thoughtNumber": 1,
            "totalThoughts": 2,
            "nextThoughtNeeded": true
        }),
    };
    execute_tool(req1).unwrap();

    // Step 2: Branch exploration
    let req_branch = ToolCallRequest {
        name: "sequential_thinking".to_string(),
        arguments: json!({
            "thought": "Alternative branch hypothesis",
            "thoughtNumber": 2,
            "totalThoughts": 3,
            "nextThoughtNeeded": false,
            "branchId": "alt-route",
            "branchFromThought": 1,
            "isRevision": true,
            "revisesThought": 1
        }),
    };
    let res_branch = execute_tool(req_branch).unwrap();
    assert_eq!(res_branch["activeBranchesCount"], 1);
    assert_eq!(res_branch["branchId"], "alt-route");
    assert_eq!(res_branch["branchFromThought"], 1);
    assert_eq!(res_branch["isRevision"], true);
    assert_eq!(res_branch["revisesThought"], 1);
}

#[test]
fn test_empty_thought_validation() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset_session();

    let req = ToolCallRequest {
        name: "sequential_thinking".to_string(),
        arguments: json!({
            "thought": "   ",
            "thoughtNumber": 1,
            "totalThoughts": 1,
            "nextThoughtNeeded": false
        }),
    };
    let res = execute_tool(req);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Thought text cannot be empty"));
}

#[test]
fn test_invalid_tool_routing() {
    let _guard = TEST_LOCK.lock().unwrap();
    let req = ToolCallRequest {
        name: "unknown_thinking_tool".to_string(),
        arguments: json!({}),
    };
    let res = execute_tool(req);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Unknown tool"));
}
