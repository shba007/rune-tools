use rune_figma::operations::{
    execute_tool, filter_figma_node, get_prompt, read_resource, rgba_to_hex,
};
use rune_pdk::ToolCallRequest;
use serde_json::json;

#[test]
fn test_rgba_to_hex() {
    let white = json!({ "r": 1.0, "g": 1.0, "b": 1.0, "a": 1.0 });
    assert_eq!(rgba_to_hex(&white), "#ffffff");

    let translucent_red = json!({ "r": 1.0, "g": 0.0, "b": 0.0, "a": 0.5 });
    assert_eq!(rgba_to_hex(&translucent_red), "#ff000080");

    let bare_hex = json!("123456");
    assert_eq!(rgba_to_hex(&bare_hex), "#123456");

    let hex_passthrough = json!("#123456");
    assert_eq!(rgba_to_hex(&hex_passthrough), "#123456");
}

#[test]
fn test_filter_figma_node_pruning() {
    let node = json!({
        "id": "1:2",
        "name": "Card Container",
        "type": "FRAME",
        "fills": [{
            "type": "SOLID",
            "color": { "r": 0.0, "g": 0.5, "b": 1.0, "a": 1.0 },
            "boundVariables": { "color": "var:123" },
            "imageRef": "img-999"
        }],
        "children": [
            {
                "id": "1:3",
                "name": "Vector Icon",
                "type": "VECTOR"
            },
            {
                "id": "1:4",
                "name": "Card Title",
                "type": "TEXT",
                "characters": "Hello World"
            }
        ]
    });

    let filtered = filter_figma_node(&node).expect("Should retain FRAME");
    assert_eq!(filtered["id"], "1:2");

    let fills = filtered["fills"].as_array().unwrap();
    assert_eq!(fills[0]["color"], "#0080ff");
    assert!(fills[0].get("boundVariables").is_none());
    assert!(fills[0].get("imageRef").is_none());

    let children = filtered["children"].as_array().unwrap();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0]["id"], "1:4");
    assert_eq!(children[0]["characters"], "Hello World");
}

#[test]
fn test_execute_tool_success() {
    let req = ToolCallRequest {
        name: "create_rectangle".to_string(),
        arguments: json!({ "x": 100, "y": 200, "width": 300, "height": 400 }),
    };

    let res = execute_tool(req).unwrap();
    assert_eq!(res["status"], "success");
    assert_eq!(res["result"]["command"], "create_rectangle");
}

#[test]
fn test_execute_tool_validation_failure() {
    let req = ToolCallRequest {
        name: "create_rectangle".to_string(),
        arguments: json!({ "x": 100 }),
    };

    let res = execute_tool(req);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("requires parameter"));
}

#[test]
fn test_unknown_tool_rejection() {
    let req = ToolCallRequest {
        name: "unknown_figma_tool".to_string(),
        arguments: json!({}),
    };

    let res = execute_tool(req);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Unknown tool"));
}

#[test]
fn test_empty_parameter_rejection() {
    let req = ToolCallRequest {
        name: "get_node_info".to_string(),
        arguments: json!({ "nodeId": "   " }),
    };

    let res = execute_tool(req);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("to be non-empty"));
}

#[test]
fn test_set_image_fill_mutually_exclusive_sources() {
    let req_none = ToolCallRequest {
        name: "set_image_fill".to_string(),
        arguments: json!({ "nodeId": "1:2" }),
    };
    assert!(execute_tool(req_none).is_err());

    let req_both = ToolCallRequest {
        name: "set_image_fill".to_string(),
        arguments: json!({
            "nodeId": "1:2",
            "imagePath": "/tmp/a.png",
            "imageUrl": "https://example.com/a.png"
        }),
    };
    assert!(execute_tool(req_both).is_err());

    let req_valid = ToolCallRequest {
        name: "set_image_fill".to_string(),
        arguments: json!({
            "nodeId": "1:2",
            "imagePath": "/tmp/a.png"
        }),
    };
    assert!(execute_tool(req_valid).is_ok());
}

#[test]
fn test_resources_and_prompts() {
    assert!(read_resource("rune://figma/document").is_ok());
    assert!(read_resource("rune://figma/unknown").is_err());
    assert!(get_prompt("design_strategy", &json!({})).is_ok());
    assert!(get_prompt("unknown_strategy", &json!({})).is_err());
}
