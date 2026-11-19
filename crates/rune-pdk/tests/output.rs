use rune_pdk::{ToolDefinition, ToolDefinitionWithOutputSchema, ToolRules, structured_tool_result};
use serde_json::{Value, json};

#[test]
fn shared_error_keeps_legacy_text_and_sets_error_flag() {
    let error = "Failed: \"quoted\" path\ntry again";
    let result = rune_pdk::tool_error(error);
    assert_eq!(result["isError"], true);
    assert_eq!(
        result["content"],
        json!([{"type":"text", "text":json!({"error":error,"status":"error"}).to_string()}])
    );
    assert!(result.get("structuredContent").is_none());
}

#[test]
fn absent_output_schema_preserves_exact_serialization() {
    let tool = ToolDefinition {
        name: "example".into(),
        description: "Example tool".into(),
        input_schema: json!({"type": "object"}),
        rules: None,
    };
    let before =
        r#"{"name":"example","description":"Example tool","inputSchema":{"type":"object"}}"#;
    assert_eq!(serde_json::to_string(&tool).unwrap(), before);
    let wrapped = ToolDefinitionWithOutputSchema {
        definition: tool,
        output_schema: None,
    };
    assert_eq!(serde_json::to_string(&wrapped).unwrap(), before);
}

#[test]
fn output_schema_is_published_alongside_unchanged_rules_and_inputs() {
    let tool = ToolDefinition::new("example", "Example tool", json!({"type": "object"}))
        .with_rules(ToolRules::read_only());
    let before = serde_json::to_value(&tool).unwrap();
    let schema = json!({"type": "object", "properties": {"count": {"type": "integer"}}});
    let tool = tool.with_output_schema(schema.clone());
    let mut published = serde_json::to_value(&tool).unwrap();
    assert_eq!(
        published.as_object_mut().unwrap().remove("outputSchema"),
        Some(schema)
    );
    assert_eq!(published, before);
    let restored: ToolDefinitionWithOutputSchema =
        serde_json::from_value(serde_json::to_value(&tool).unwrap()).unwrap();
    assert_eq!(restored, tool);
}

#[test]
fn structured_success_has_text_object_and_false_error_flag() {
    let result = structured_tool_result(
        "unchanged text",
        json!({"count": 3}).as_object().unwrap().clone(),
    );
    assert_eq!(
        result,
        json!({
            "content": [{"type": "text", "text": "unchanged text"}],
            "structuredContent": {"count": 3},
            "isError": false
        })
    );
    let _: Value = serde_json::from_str(&result.to_string()).unwrap();
}
