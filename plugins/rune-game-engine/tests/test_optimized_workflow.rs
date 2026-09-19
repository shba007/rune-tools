use rune_game_engine::operations::execute_tool;
use rune_pdk::ToolCallRequest;
use serde_json::json;

#[test]
fn test_compile_declarative_bundle() {
    let req = ToolCallRequest {
        name: "compile_declarative_bundle".to_string(),
        arguments: json!({
            "engine": "threejs",
            "theme": "ghibli_coastal",
            "pipeline": [
                { "type": "spline", "params": { "closed": true } },
                { "type": "biome", "params": { "preset": "floating_archipelago" } }
            ]
        }),
    };

    let res = execute_tool(req).expect("Failed to compile declarative bundle");
    assert_eq!(res["status"], "success");
    assert!(res["html_document"].as_str().is_some());
    assert_eq!(res["validation_report"]["validation_status"], "passed");
}

#[test]
fn test_batch_pipeline_executor() {
    let req = ToolCallRequest {
        name: "batch_pipeline_executor".to_string(),
        arguments: json!({
            "steps": [
                { "type": "spline", "params": {} },
                { "type": "biome", "params": {} }
            ],
            "input_state": { "initialized": true }
        }),
    };

    let res = execute_tool(req).expect("Failed to execute batch pipeline");
    assert_eq!(res["status"], "success");
    assert_eq!(res["execution_order"].as_array().unwrap().len(), 2);
}

#[test]
fn test_patch_ast_node() {
    let req = ToolCallRequest {
        name: "patch_ast_node".to_string(),
        arguments: json!({
            "file_path": "/tmp/test.js",
            "node_type": "ObjectProperty",
            "node_name": "config",
            "operation": "update",
            "new_value": { "fps": 60 }
        }),
    };

    let res = execute_tool(req).expect("Failed to patch AST node");
    assert_eq!(res["status"], "success");
}

#[test]
fn test_validate_headless_runtime() {
    let req = ToolCallRequest {
        name: "validate_headless_runtime".to_string(),
        arguments: json!({
            "html_bundle": "<html><body>Test</body></html>",
            "test_cases": ["init", "render"]
        }),
    };

    let res = execute_tool(req).expect("Failed to validate runtime");
    assert_eq!(res["status"], "success");
}

#[test]
fn test_cache_template_registry() {
    let req = ToolCallRequest {
        name: "cache_template_registry".to_string(),
        arguments: json!({
            "template_name": "threejs_boilerplate",
            "template_type": "html_boilerplate"
        }),
    };

    let res = execute_tool(req).expect("Failed to cache template");
    assert_eq!(res["status"], "success");
    assert!(res["template"].as_str().is_some());
}
