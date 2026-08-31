use crate::{ToolCallRequest, ToolDefinition};
use serde_json::{Value, json};
use std::collections::HashMap;

pub fn assert_valid_tool_definitions(tools: &[ToolDefinition]) {
    assert!(!tools.is_empty(), "Plugin must export at least one tool");

    for tool in tools {
        assert!(!tool.name.trim().is_empty(), "Tool name cannot be empty");
        assert!(
            tool.name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c == '_'),
            "Tool '{}' must be lowercase snake_case",
            tool.name
        );
        assert!(
            tool.description.len() >= 10,
            "Tool '{}' description is too short or missing",
            tool.name
        );

        let schema = &tool.input_schema;
        assert_eq!(
            schema.get("type").and_then(Value::as_str),
            Some("object"),
            "Tool '{}' input_schema must be of type 'object'",
            tool.name
        );

        if let Some(props) = schema.get("properties").and_then(Value::as_object) {
            for (prop_name, prop_val) in props {
                assert!(
                    prop_val.get("description").is_some(),
                    "Tool '{}' property '{}' must specify a description",
                    tool.name,
                    prop_name
                );
                assert!(
                    prop_val.get("type").is_some(),
                    "Tool '{}' property '{}' must specify a type",
                    tool.name,
                    prop_name
                );
            }
        }
    }
}

pub fn assert_required_fields_enforced<F>(tools: &[ToolDefinition], mut execute: F)
where
    F: FnMut(ToolCallRequest) -> Result<Value, String>,
{
    for tool in tools {
        let schema = &tool.input_schema;
        let props = match schema.get("properties").and_then(Value::as_object) {
            Some(p) => p,
            None => continue,
        };
        let required = schema
            .get("required")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        let mut base_payload = HashMap::new();
        for (name, spec) in props {
            if required.iter().any(|r| r.as_str() == Some(name)) {
                base_payload.insert(name.clone(), mock_valid_value(name, spec));
            }
        }

        for req_field in &required {
            let field_name = req_field.as_str().unwrap();
            let mut test_payload = base_payload.clone();
            test_payload.remove(field_name);

            let req = ToolCallRequest {
                name: tool.name.clone(),
                arguments: json!(test_payload),
            };

            let res = execute(req);
            assert!(
                res.is_err(),
                "Tool '{}' must fail when required field '{}' is omitted",
                tool.name,
                field_name
            );
        }
    }
}

pub fn assert_invalid_types_rejected<F>(tools: &[ToolDefinition], mut execute: F)
where
    F: FnMut(ToolCallRequest) -> Result<Value, String>,
{
    for tool in tools {
        let schema = &tool.input_schema;
        let props = match schema.get("properties").and_then(Value::as_object) {
            Some(p) => p,
            None => continue,
        };

        for (prop_name, prop_spec) in props {
            let invalid_val = mock_invalid_value(prop_spec);
            let mut payload = HashMap::new();
            payload.insert(prop_name.clone(), invalid_val);

            let req = ToolCallRequest {
                name: tool.name.clone(),
                arguments: json!(payload),
            };

            // Assert execution handles adversarial types without panicking
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| execute(req)));
            assert!(
                result.is_ok(),
                "Tool '{}' panicked on malformed input for property '{}'",
                tool.name,
                prop_name
            );
        }
    }
}

fn mock_valid_value(name: &str, spec: &Value) -> Value {
    if let Some(enums) = spec.get("enum").and_then(Value::as_array) {
        return enums.first().cloned().unwrap_or(json!(""));
    }
    match spec.get("type").and_then(Value::as_str).unwrap_or("string") {
        "string" => json!(format!("mock_{}", name)),
        "number" | "integer" => json!(1),
        "boolean" => json!(true),
        "array" => json!([]),
        "object" => json!({}),
        _ => json!(null),
    }
}

fn mock_invalid_value(spec: &Value) -> Value {
    match spec.get("type").and_then(Value::as_str).unwrap_or("string") {
        "string" => json!(123456),
        "number" | "integer" => json!("invalid_number_string"),
        "boolean" => json!("invalid_bool"),
        "array" => json!("invalid_array"),
        "object" => json!("invalid_object"),
        _ => json!(true),
    }
}

#[macro_export]
macro_rules! test_plugin_contract {
    ($tool_fn:path, $exec_fn:path) => {
        #[test]
        fn test_schema_validity() {
            let tools = $tool_fn();
            $crate::testing::assert_valid_tool_definitions(&tools);
        }

        #[test]
        fn test_all_tools_routable() {
            let tools = $tool_fn();
            for tool in tools {
                let req = $crate::ToolCallRequest {
                    name: tool.name.clone(),
                    arguments: ::serde_json::json!({}),
                };
                let res = $exec_fn(req);
                if let Err(err) = res {
                    assert!(
                        !err.starts_with("Unknown tool"),
                        "Tool '{}' is listed in tool_definitions but not handled in execute_tool",
                        tool.name
                    );
                }
            }
        }

        #[test]
        fn test_required_arguments_enforced() {
            let tools = $tool_fn();
            $crate::testing::assert_required_fields_enforced(&tools, $exec_fn);
        }

        #[test]
        fn test_invalid_types_rejection() {
            let tools = $tool_fn();
            $crate::testing::assert_invalid_types_rejected(&tools, $exec_fn);
        }
    };
}
