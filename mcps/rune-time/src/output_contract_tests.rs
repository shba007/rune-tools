use super::*;
use crate::types::{ConvertTimeResponse, CurrentTimeResponse};
use chrono::{DateTime, Utc};

fn captures() -> Vec<Value> {
    serde_json::from_str(include_str!("../tests/fixtures/kit-before.json")).unwrap()
}

fn assert_schema_properties(schema: &Value) {
    let properties = schema["properties"].as_object().unwrap();
    let required = schema["required"].as_array().unwrap();
    assert_eq!(properties.len(), required.len());
    assert_eq!(schema["additionalProperties"], false);
    for (name, spec) in properties {
        assert!(!spec["description"].as_str().unwrap().trim().is_empty());
        assert!(required.contains(&json!(name)));
        if spec["type"] == "object" {
            assert_schema_properties(spec);
        }
    }
}

#[test]
fn published_schemas_match_typed_results_and_captured_client_text() {
    let tools = definitions::published_tool_definitions();
    assert_eq!(tools.len(), 2);
    for (published, base) in tools.iter().zip(definitions::tool_definitions()) {
        let mut wire = serde_json::to_value(published).unwrap();
        assert!(
            wire.as_object_mut()
                .unwrap()
                .remove("outputSchema")
                .is_some()
        );
        assert_eq!(wire, serde_json::to_value(base).unwrap());
    }
    for capture in captures().into_iter().take(2) {
        let request: ToolCallRequest = serde_json::from_value(capture["input"].clone()).unwrap();
        let tool = tools
            .iter()
            .find(|t| t.definition.name == request.name)
            .unwrap();
        let published = serde_json::to_value(tool).unwrap();
        let schema = &published["outputSchema"];
        assert_schema_properties(schema);
        let validator = jsonschema::options().offline().build(schema).unwrap();
        let old_text = capture["response"]["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        let old: Value = serde_json::from_str(old_text).unwrap();
        let output = if request.name == "get_current_time" {
            let now: DateTime<Utc> = old["result"]["utcDatetime"]
                .as_str()
                .unwrap()
                .parse()
                .unwrap();
            tool_result(operations::execute_tool_at(request, now))
        } else {
            call_tool(request)
        };
        assert_eq!(output["isError"], false);
        assert_eq!(output["content"][0]["type"], "text");
        assert_eq!(output["content"][0]["text"], old_text);
        assert_eq!(output["structuredContent"], old["result"]);
        let structured = output["structuredContent"].clone();
        validator.validate(&structured).unwrap();
        let typed = match tool.definition.name.as_str() {
            "get_current_time" => serde_json::to_value(
                serde_json::from_value::<CurrentTimeResponse>(structured.clone()).unwrap(),
            )
            .unwrap(),
            "convert_time" => serde_json::to_value(
                serde_json::from_value::<ConvertTimeResponse>(structured.clone()).unwrap(),
            )
            .unwrap(),
            _ => unreachable!(),
        };
        assert_eq!(typed, structured);
        assert_eq!(
            schema["properties"].as_object().unwrap().len(),
            structured.as_object().unwrap().len()
        );
        for key in structured.as_object().unwrap().keys() {
            let mut missing = structured.clone();
            missing.as_object_mut().unwrap().remove(key);
            assert!(!validator.is_valid(&missing), "missing {key}");
            let mut wrong_type = structured.clone();
            wrong_type[key] = json!(null);
            assert!(!validator.is_valid(&wrong_type), "null {key}");
        }
        if tool.definition.name == "convert_time" {
            for parent in ["source", "target"] {
                for key in ["timezone", "datetime", "utcOffset"] {
                    let mut missing = structured.clone();
                    missing[parent].as_object_mut().unwrap().remove(key);
                    assert!(!validator.is_valid(&missing));
                }
            }
        }
    }
}

#[test]
fn invalid_timezones_keep_captured_error_text_and_domain_errors() {
    for capture in captures().into_iter().skip(2) {
        let request: ToolCallRequest = serde_json::from_value(capture["input"].clone()).unwrap();
        let old_text = capture["response"]["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        let old: Value = serde_json::from_str(old_text).unwrap();
        assert_eq!(
            operations::execute_tool(request.clone()).unwrap_err(),
            old["error"]
        );
        let result = call_tool(request);
        assert_eq!(result["content"][0]["text"], old_text);
        assert_eq!(result["content"][0]["type"], "text");
        assert_eq!(result["isError"], true);
        assert!(result.get("structuredContent").is_none());
    }
}

#[test]
fn shipped_entry_reads_real_time() {
    let before = Utc::now();
    let result = call_tool(ToolCallRequest {
        name: "get_current_time".into(),
        arguments: json!({"timezone": "UTC"}),
    });
    let after = Utc::now();
    let actual: DateTime<Utc> = result["structuredContent"]["utcDatetime"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    assert!((before..=after).contains(&actual));
}
