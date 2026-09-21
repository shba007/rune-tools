use rune_pdk::ToolCallRequest;
use serde_json::{Value, json};

pub fn rgba_to_hex(val: &Value) -> String {
    if let Some(s) = val.as_str()
        && s.starts_with('#')
    {
        return s.to_string();
    }

    let r = val.get("r").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let g = val.get("g").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let b = val.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let a = val.get("a").and_then(|v| v.as_f64()).unwrap_or(1.0);

    let r_u8 = (r * 255.0).round().clamp(0.0, 255.0) as u8;
    let g_u8 = (g * 255.0).round().clamp(0.0, 255.0) as u8;
    let b_u8 = (b * 255.0).round().clamp(0.0, 255.0) as u8;
    let a_u8 = (a * 255.0).round().clamp(0.0, 255.0) as u8;

    if a_u8 == 255 {
        format!("#{:02x}{:02x}{:02x}", r_u8, g_u8, b_u8)
    } else {
        format!("#{:02x}{:02x}{:02x}{:02x}", r_u8, g_u8, b_u8, a_u8)
    }
}

pub fn filter_figma_node(node: &Value) -> Option<Value> {
    let obj = node.as_object()?;

    if obj.get("type").and_then(|t| t.as_str()) == Some("VECTOR") {
        return None;
    }

    let mut filtered = serde_json::Map::new();
    if let Some(id) = obj.get("id") {
        filtered.insert("id".to_string(), id.clone());
    }
    if let Some(name) = obj.get("name") {
        filtered.insert("name".to_string(), name.clone());
    }
    if let Some(node_type) = obj.get("type") {
        filtered.insert("type".to_string(), node_type.clone());
    }

    if let Some(fills) = obj.get("fills").and_then(|f| f.as_array()) {
        let processed: Vec<Value> = fills
            .iter()
            .map(|fill| {
                let mut f_obj = fill.as_object().cloned().unwrap_or_default();
                f_obj.remove("boundVariables");
                f_obj.remove("imageRef");

                if let Some(color) = f_obj.get("color") {
                    f_obj.insert("color".to_string(), Value::String(rgba_to_hex(color)));
                }

                if let Some(stops) = f_obj.get("gradientStops").and_then(|s| s.as_array()) {
                    let new_stops: Vec<Value> = stops
                        .iter()
                        .map(|stop| {
                            let mut s_obj = stop.as_object().cloned().unwrap_or_default();
                            s_obj.remove("boundVariables");
                            if let Some(c) = s_obj.get("color") {
                                s_obj.insert("color".to_string(), Value::String(rgba_to_hex(c)));
                            }
                            Value::Object(s_obj)
                        })
                        .collect();
                    f_obj.insert("gradientStops".to_string(), Value::Array(new_stops));
                }

                Value::Object(f_obj)
            })
            .collect();
        filtered.insert("fills".to_string(), Value::Array(processed));
    }

    if let Some(strokes) = obj.get("strokes").and_then(|s| s.as_array()) {
        let processed: Vec<Value> = strokes
            .iter()
            .map(|stroke| {
                let mut s_obj = stroke.as_object().cloned().unwrap_or_default();
                s_obj.remove("boundVariables");
                if let Some(color) = s_obj.get("color") {
                    s_obj.insert("color".to_string(), Value::String(rgba_to_hex(color)));
                }
                Value::Object(s_obj)
            })
            .collect();
        filtered.insert("strokes".to_string(), Value::Array(processed));
    }

    if let Some(radius) = obj.get("cornerRadius") {
        filtered.insert("cornerRadius".to_string(), radius.clone());
    }
    if let Some(bbox) = obj.get("absoluteBoundingBox") {
        filtered.insert("absoluteBoundingBox".to_string(), bbox.clone());
    }
    if let Some(chars) = obj.get("characters") {
        filtered.insert("characters".to_string(), chars.clone());
    }

    if let Some(style) = obj.get("style").and_then(|s| s.as_object()) {
        let mut clean_style = serde_json::Map::new();
        for key in &[
            "fontFamily",
            "fontStyle",
            "fontWeight",
            "fontSize",
            "textAlignHorizontal",
            "letterSpacing",
            "lineHeightPx",
        ] {
            if let Some(v) = style.get(*key) {
                clean_style.insert((*key).to_string(), v.clone());
            }
        }
        filtered.insert("style".to_string(), Value::Object(clean_style));
    }

    if let Some(children) = obj.get("children").and_then(|c| c.as_array()) {
        let clean_children: Vec<Value> = children.iter().filter_map(filter_figma_node).collect();
        filtered.insert("children".to_string(), Value::Array(clean_children));
    }

    Some(Value::Object(filtered))
}

pub fn execute_tool(request: ToolCallRequest) -> Result<Value, String> {
    let tools = crate::definitions::tool_definitions();
    let tool = tools
        .iter()
        .find(|t| t.name == request.name)
        .ok_or_else(|| {
            format!(
                "Unknown tool: '{}'. Use list_tools to inspect supported Figma commands.",
                request.name
            )
        })?;

    // Generic schema validation enforcing required arguments
    validate_tool_schema(tool, &request.arguments)?;

    Ok(json!({
        "status": "success",
        "result": {
            "command": request.name,
            "params": request.arguments,
            "dispatched": true
        }
    }))
}

fn validate_tool_schema(tool: &rune_pdk::ToolDefinition, args: &Value) -> Result<(), String> {
    // 1. Enforce required arguments from input_schema
    if let Some(required) = tool.input_schema.get("required").and_then(|r| r.as_array()) {
        for req_field in required {
            if let Some(field_name) = req_field.as_str() {
                let is_missing = match args {
                    Value::Object(map) => match map.get(field_name) {
                        None | Some(Value::Null) => true,
                        _ => false,
                    },
                    _ => true,
                };

                if is_missing {
                    return Err(format!(
                        "Tool '{}' requires parameter '{}'. Ensure it is provided.",
                        tool.name, field_name
                    ));
                }
            }
        }
    }

    // 2. Domain-specific validation (mutually exclusive sources)
    if tool.name == "set_image_fill" {
        let has_path = args.get("imagePath").is_some();
        let has_url = args.get("imageUrl").is_some();
        let has_b64 = args.get("imageBase64").is_some();
        let count = [has_path, has_url, has_b64].iter().filter(|&&b| b).count();
        if count != 1 {
            return Err("Tool 'set_image_fill' requires exactly one of 'imagePath', 'imageUrl', or 'imageBase64'.".to_string());
        }
    }

    Ok(())
}

pub fn read_resource(uri: &str) -> Result<Value, String> {
    match uri {
        "rune://figma/document" => Ok(json!({
            "status": "success",
            "result": { "type": "DOCUMENT", "name": "Active Figma Document", "children": [] }
        })),
        "rune://figma/selection" => Ok(json!({
            "status": "success",
            "result": { "selectedNodes": [], "count": 0 }
        })),
        "rune://figma/styles" => Ok(json!({
            "status": "success",
            "result": { "styles": [] }
        })),
        unknown => Err(format!(
            "Unknown resource URI: '{}'. Valid Figma URIs: rune://figma/document, rune://figma/selection, rune://figma/styles",
            unknown
        )),
    }
}

pub fn get_prompt(name: &str, _arguments: &Value) -> Result<Value, String> {
    match name {
        "design_strategy" => Ok(json!({
            "description": "Best practices for working with Figma designs",
            "messages": [{
                "role": "assistant",
                "content": {
                    "type": "text",
                    "text": "1. Start with Document Structure: Inspect with get_document_info().\n2. Naming Conventions: Use semantic layer names.\n3. Layout Hierarchy: Build parent containers first, then child frames.\n4. Input Fields: Group label and input frames.\n5. Autolayout: Prefer autolayout frames over manual coordinates."
                }
            }]
        })),
        "read_design_strategy" => Ok(json!({
            "description": "Best practices for reading Figma designs",
            "messages": [{
                "role": "assistant",
                "content": {
                    "type": "text",
                    "text": "1. Inspect selection: call read_my_design().\n2. Filter Primitives: Vector paths are stripped for clean layout analysis.\n3. Recurse Frames: Walk hierarchy from parent to child."
                }
            }]
        })),
        "text_replacement_strategy" => Ok(json!({
            "description": "Systematic approach for replacing text in Figma designs",
            "messages": [{
                "role": "assistant",
                "content": {
                    "type": "text",
                    "text": "1. Scan: scan_text_nodes(nodeId).\n2. Chunk: Group replacements into batches of 5-10 elements.\n3. Mutate: set_multiple_text_contents(nodeId, text).\n4. Verify: export_node_as_image(nodeId, scale: 0.5) to confirm text bounds."
                }
            }]
        })),
        "annotation_conversion_strategy" => Ok(json!({
            "description": "Strategy for converting manual annotations to native Figma annotations",
            "messages": [{
                "role": "assistant",
                "content": {
                    "type": "text",
                    "text": "1. Get Selection: get_selection().\n2. Scan Markers & Descriptions: scan_text_nodes().\n3. Scan Targets: scan_nodes_by_types(nodeId, ['COMPONENT', 'INSTANCE', 'FRAME']).\n4. Batch Apply: set_multiple_annotations()."
                }
            }]
        })),
        "swap_overrides_instances" => Ok(json!({
            "description": "Guide to swap instance overrides between instances",
            "messages": [{
                "role": "assistant",
                "content": {
                    "type": "text",
                    "text": "1. Extract: get_instance_overrides(nodeId: source).\n2. Target: identify destination instances via scan_nodes_by_types.\n3. Apply: set_instance_overrides(sourceInstanceId, targetNodeIds)."
                }
            }]
        })),
        "reaction_to_connector_strategy" => Ok(json!({
            "description": "Strategy for converting Figma prototype reactions to connector lines",
            "messages": [{
                "role": "assistant",
                "content": {
                    "type": "text",
                    "text": "1. Query reactions: get_reactions(nodeIds).\n2. Ensure Default Connector: set_default_connector().\n3. Transform: Map NAVIGATE/OPEN_OVERLAY reactions into connection objects.\n4. Create: create_connections(connections)."
                }
            }]
        })),
        unknown => Err(format!(
            "Unknown prompt: '{}'. Check prompt_definitions for valid templates.",
            unknown
        )),
    }
}
