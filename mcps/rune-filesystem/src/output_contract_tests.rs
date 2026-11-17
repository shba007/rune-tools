use super::*;
use output::{Listing, MAX_RESULT_BYTES};
use serde_json::Value;

fn captures() -> Vec<Value> {
    serde_json::from_str(include_str!("../tests/fixtures/kit-before.json")).unwrap()
}

fn validator(name: &str) -> jsonschema::Validator {
    let tool = definitions::published_tool_definitions()
        .into_iter()
        .find(|t| t.definition.name == name)
        .unwrap();
    jsonschema::options()
        .offline()
        .build(tool.output_schema.as_ref().unwrap())
        .unwrap()
}

fn assert_properties(schema: &Value) {
    if let Some(properties) = schema["properties"].as_object() {
        let required = schema["required"].as_array().unwrap();
        assert_eq!(required.len(), properties.len());
        for (name, property) in properties {
            assert!(!property["description"].as_str().unwrap().is_empty());
            assert!(required.contains(&json!(name)));
            assert_properties(property);
        }
    }
    if schema.get("items").is_some() {
        assert_properties(&schema["items"]);
    }
}

#[test]
fn schemas_preserve_inputs_rules_and_the_media_exception() {
    let tools = definitions::published_tool_definitions();
    assert_eq!(tools.len(), 13);
    assert_eq!(
        tools.iter().filter(|t| t.output_schema.is_some()).count(),
        12
    );
    for (tool, base) in tools.iter().zip(definitions::tool_definitions()) {
        let mut published = serde_json::to_value(tool).unwrap();
        let schema = published.as_object_mut().unwrap().remove("outputSchema");
        assert_eq!(published, serde_json::to_value(base).unwrap());
        if tool.definition.name == "read_media_file" {
            assert!(schema.is_none());
        } else {
            assert_properties(&schema.unwrap());
        }
    }
}

#[test]
fn captured_text_is_byte_identical_and_structured_results_validate() {
    for capture in captures() {
        let name = capture["input"]["name"].as_str().unwrap();
        let text = capture["response"]["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        let old: Value = serde_json::from_str(text).unwrap();
        if name == "read_media_file" {
            continue;
        } // Dedicated live exception test below.
        let result = if old["status"] == "error" {
            let error = old["error"].as_str().unwrap().to_owned();
            tool_result(Err(error), false)
        } else {
            let legacy = old["result"].clone();
            let listing = match name {
                "list_directory" | "list_directory_with_sizes" | "search_files" => {
                    Some(Listing::lines(
                        &legacy["content"]
                            .as_str()
                            .unwrap()
                            .lines()
                            .map(str::to_owned)
                            .collect::<Vec<_>>(),
                    ))
                }
                "directory_tree" => Some(Listing::tree(
                    &serde_json::from_str(legacy["content"].as_str().unwrap()).unwrap(),
                )),
                "list_allowed_directories" => {
                    let roots: Vec<String> =
                        serde_json::from_str(legacy["content"].as_str().unwrap()).unwrap();
                    Some(Listing::directories(&roots[0]))
                }
                _ => None,
            };
            tool_result(
                Ok((legacy, listing)),
                matches!(name, "read_multiple_files" | "edit_file"),
            )
        };
        assert_eq!(
            result["content"][0]["text"].as_str().unwrap().as_bytes(),
            text.as_bytes(),
            "{name}"
        );
        assert_eq!(result["isError"], old["status"] == "error");
        if old["status"] == "success" {
            validator(name)
                .validate(&result["structuredContent"])
                .unwrap();
            assert!(serde_json::to_vec(&result).unwrap().len() < 8 * 1024 * 1024);
        } else {
            assert!(result.get("structuredContent").is_none());
        }
    }
}

fn listing_with_rows(rows: &[String]) -> Value {
    output::listing_result(json!({"content": rows.join("\n")}), &Listing::lines(rows))
}

#[test]
fn listing_just_below_and_above_the_exact_byte_bound() {
    // A plain character occurs three times: legacy text, structured content,
    // and the explicit entry. Derive the largest complete single-item result.
    let overhead = serde_json::to_vec(&listing_with_rows(&[String::new()]))
        .unwrap()
        .len();
    let length = (MAX_RESULT_BYTES - overhead) / 3;
    let below = listing_with_rows(&["x".repeat(length)]);
    let size = serde_json::to_vec(&below).unwrap().len();
    assert!(size <= MAX_RESULT_BYTES && MAX_RESULT_BYTES - size < 3);
    assert_eq!(below["structuredContent"]["truncated"], false);
    assert_eq!(below["structuredContent"]["total"], 1);
    assert_eq!(
        below["structuredContent"]["entries"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let above = listing_with_rows(&["x".repeat(length + 1)]);
    assert_eq!(above["structuredContent"]["truncated"], true);
    assert_eq!(above["structuredContent"]["total"], 1);
    assert!(
        above["structuredContent"]["entries"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        above["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("omitted 1 of 1 items")
    );
    assert!(serde_json::to_vec(&above).unwrap().len() <= MAX_RESULT_BYTES);
    for result in [below, above] {
        validator("search_files")
            .validate(&result["structuredContent"])
            .unwrap();
    }
}

#[test]
fn escaped_worst_case_retains_complete_items_under_kits_ceiling() {
    // Quotes, backslashes, control characters and multibyte UTF-8 expand
    // differently in the nested legacy JSON text and the structured payload.
    let rows: Vec<_> = (0..20)
        .map(|i| format!("{i}:{}", "\"\\\n\t🦀".repeat(12_000)))
        .collect();
    let result = listing_with_rows(&rows);
    let structured = &result["structuredContent"];
    let entries = structured["entries"].as_array().unwrap();
    assert!(!entries.is_empty() && entries.len() < rows.len());
    assert_eq!(structured["total"], rows.len());
    assert_eq!(structured["truncated"], true);
    for (entry, original) in entries.iter().zip(&rows) {
        assert_eq!(entry, original);
    }
    let size = serde_json::to_vec(&result).unwrap().len();
    assert!(size <= MAX_RESULT_BYTES);
    assert!(size < 8 * 1024 * 1024);
    validator("list_directory_with_sizes")
        .validate(structured)
        .unwrap();
    assert!(
        result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains(&format!("omitted {} of 20 items", 20 - entries.len()))
    );
}

#[test]
fn tree_truncation_keeps_complete_nodes_and_real_recursive_count() {
    let tree = json!({"name":"root", "type":"directory", "children":[
        {"name":"small", "type":"file"},
        {"name":"huge".repeat(MAX_RESULT_BYTES), "type":"file"}
    ]});
    let legacy = json!({"content":serde_json::to_string_pretty(&tree).unwrap()});
    let result = output::listing_result(legacy, &Listing::tree(&tree));
    let structured = &result["structuredContent"];
    assert_eq!(structured["total"], 3);
    assert_eq!(structured["entries"].as_array().unwrap().len(), 2);
    assert_eq!(
        structured["entries"][1],
        json!({"name":"small", "type":"file", "path":"root/small"})
    );
    assert_eq!(structured["truncated"], true);
    validator("directory_tree").validate(structured).unwrap();
    assert!(serde_json::to_vec(&result).unwrap().len() <= MAX_RESULT_BYTES);
}

#[test]
fn text_previews_are_utf8_safe_and_report_omitted_bytes() {
    for name in ["read_multiple_files", "edit_file"] {
        let content = "\"\\\n🦀".repeat(MAX_RESULT_BYTES / 4);
        let result = output::text_result(json!({"content":content}));
        let structured = &result["structuredContent"];
        assert_eq!(structured["total"], content.len());
        assert_eq!(structured["truncated"], true);
        assert!(
            result["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("UTF-8 bytes")
        );
        validator(name).validate(structured).unwrap();
        assert!(serde_json::to_vec(&result).unwrap().len() <= MAX_RESULT_BYTES);
    }
}

#[test]
fn real_directory_count_includes_entries_beyond_the_existing_page_cap() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..205 {
        std::fs::write(dir.path().join(format!("{i:03}.txt")), "").unwrap();
    }
    let mut listing = None;
    let legacy = pilot_reads::execute_with_items(
        ToolCallRequest {
            name: "list_directory".into(),
            arguments: json!({"path":"."}),
        },
        dir.path().to_str(),
        &mut listing,
    )
    .unwrap();
    let result = tool_result(Ok((legacy, listing)), false);
    assert_eq!(result["structuredContent"]["total"], 205);
    assert_eq!(result["structuredContent"]["entriesReturned"], 200);
    assert_eq!(result["structuredContent"]["truncated"], true);
    assert!(
        result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("omitted 5 of 205 items")
    );
    validator("list_directory")
        .validate(&result["structuredContent"])
        .unwrap();
}

#[test]
fn directory_notice_respects_the_existing_content_byte_limit() {
    let rows = vec!["a".repeat(32767), "b".repeat(32767)];
    let mut listing = Listing::lines(&rows);
    listing.total = 3;
    let legacy = json!({"content":rows.join("\n"),"maxOutputBytes":65536,
        "entriesReturned":2,"maxEntries":200,"hasMore":true,"truncated":true});
    let result = output::listing_result(legacy, &listing);
    let structured = &result["structuredContent"];
    assert!(structured["content"].as_str().unwrap().len() <= 65536);
    assert_eq!(structured["entriesReturned"], 1);
    assert_eq!(structured["entries"][0], rows[0]);
    assert_eq!(structured["total"], 3);
    assert!(
        structured["content"]
            .as_str()
            .unwrap()
            .contains("omitted 2 of 3 items")
    );
    validator("list_directory").validate(structured).unwrap();
}

#[test]
fn media_success_and_failure_keep_the_exact_legacy_payload() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("image.png");
    std::fs::write(&file, [1, 2, 3]).unwrap();
    for path in [file, dir.path().join("missing.png")] {
        let request = ToolCallRequest {
            name: "read_media_file".into(),
            arguments: json!({"path":path}),
        };
        let expected = match operations::execute_tool(request.clone()) {
            Ok(value) => json!({"status":"success", "result":value}),
            Err(error) => json!({"status":"error", "error":error}),
        };
        let result = call_tool(request);
        assert_eq!(result, expected);
        assert!(result.get("structuredContent").is_none());
        assert!(result.get("isError").is_none());
    }
}

#[test]
fn public_boundary_marks_non_media_failures_as_errors() {
    let result = call_tool(ToolCallRequest {
        name: "write_file".into(),
        arguments: json!({}),
    });
    assert_eq!(result, rune_pdk::tool_error("Missing 'path' parameter"));
    assert_eq!(result["isError"], true);
}

#[test]
fn oversized_text_previews_do_not_shorten_the_file_operation() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("large.txt");
    let original = "🦀".repeat(MAX_RESULT_BYTES / 2);
    std::fs::write(&file, &original).unwrap();
    let read = call_tool(ToolCallRequest {
        name: "read_multiple_files".into(),
        arguments: json!({"paths":[file]}),
    });
    assert_eq!(read["structuredContent"]["truncated"], true);
    assert_eq!(std::fs::read_to_string(&file).unwrap(), original);
    let replacement = "updated🦀".repeat(MAX_RESULT_BYTES / 4);
    let edit = call_tool(ToolCallRequest {
        name: "edit_file".into(),
        arguments: json!({"path":file, "edits":[{"oldText":original,"newText":replacement}]}),
    });
    assert_eq!(edit["structuredContent"]["truncated"], true);
    assert_eq!(std::fs::read_to_string(&file).unwrap(), replacement);
    for (name, result) in [("read_multiple_files", read), ("edit_file", edit)] {
        assert_eq!(result["isError"], false);
        validator(name)
            .validate(&result["structuredContent"])
            .unwrap();
        assert!(serde_json::to_vec(&result).unwrap().len() <= MAX_RESULT_BYTES);
    }
}
