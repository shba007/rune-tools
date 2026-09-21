use rune_pdk::{PromptDefinition, ResourceDefinition, ToolDefinition};
use serde_json::json;

pub fn tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "get_document_info".to_string(),
            description: "Get detailed information about the current Figma document".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "get_selection".to_string(),
            description: "Get information about the current selection in Figma".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "read_my_design".to_string(),
            description: "Get detailed information about the current selection in Figma, including all node details".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "get_node_info".to_string(),
            description: "Get detailed information about a specific node in Figma".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["nodeId"],
                "properties": {
                    "nodeId": { "type": "string", "description": "The ID of the node to inspect" }
                }
            }),
        },
        ToolDefinition {
            name: "get_nodes_info".to_string(),
            description: "Get detailed information about multiple nodes in Figma".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["nodeIds"],
                "properties": {
                    "nodeIds": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Array of node IDs to get information about"
                    }
                }
            }),
        },
        ToolDefinition {
            name: "create_rectangle".to_string(),
            description: "Create a new rectangle in Figma".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["x", "y", "width", "height"],
                "properties": {
                    "x": { "type": "number", "description": "X position on canvas" },
                    "y": { "type": "number", "description": "Y position on canvas" },
                    "width": { "type": "number", "description": "Width of the rectangle" },
                    "height": { "type": "number", "description": "Height of the rectangle" },
                    "name": { "type": "string", "description": "Optional name for the rectangle" },
                    "parentId": { "type": "string", "description": "Optional parent node ID to append the rectangle to" }
                }
            }),
        },
        ToolDefinition {
            name: "create_frame".to_string(),
            description: "Create a new frame in Figma with optional layout and styling configurations".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["x", "y", "width", "height"],
                "properties": {
                    "x": { "type": "number", "description": "X position on canvas" },
                    "y": { "type": "number", "description": "Y position on canvas" },
                    "width": { "type": "number", "description": "Width of the frame" },
                    "height": { "type": "number", "description": "Height of the frame" },
                    "name": { "type": "string", "description": "Optional name for the frame" },
                    "parentId": { "type": "string", "description": "Optional parent node ID to append the frame to" },
                    "fillColor": {
                        "type": "object",
                        "description": "Fill color in RGBA format (values 0-1)",
                        "properties": {
                            "r": { "type": "number", "minimum": 0, "maximum": 1, "description": "Red component (0-1)" },
                            "g": { "type": "number", "minimum": 0, "maximum": 1, "description": "Green component (0-1)" },
                            "b": { "type": "number", "minimum": 0, "maximum": 1, "description": "Blue component (0-1)" },
                            "a": { "type": "number", "minimum": 0, "maximum": 1, "description": "Alpha component (0-1)" }
                        },
                        "required": ["r", "g", "b"]
                    },
                    "strokeColor": {
                        "type": "object",
                        "description": "Stroke color in RGBA format (values 0-1)",
                        "properties": {
                            "r": { "type": "number", "minimum": 0, "maximum": 1, "description": "Red component (0-1)" },
                            "g": { "type": "number", "minimum": 0, "maximum": 1, "description": "Green component (0-1)" },
                            "b": { "type": "number", "minimum": 0, "maximum": 1, "description": "Blue component (0-1)" },
                            "a": { "type": "number", "minimum": 0, "maximum": 1, "description": "Alpha component (0-1)" }
                        },
                        "required": ["r", "g", "b"]
                    },
                    "strokeWeight": { "type": "number", "minimum": 0, "description": "Stroke weight in pixels" },
                    "layoutMode": { "type": "string", "enum": ["NONE", "HORIZONTAL", "VERTICAL"], "description": "Auto-layout mode for the frame (NONE, HORIZONTAL, VERTICAL)" },
                    "layoutWrap": { "type": "string", "enum": ["NO_WRAP", "WRAP"], "description": "Whether the auto-layout frame wraps its children (NO_WRAP, WRAP)" },
                    "paddingTop": { "type": "number", "description": "Top padding for auto-layout frame in pixels" },
                    "paddingRight": { "type": "number", "description": "Right padding for auto-layout frame in pixels" },
                    "paddingBottom": { "type": "number", "description": "Bottom padding for auto-layout frame in pixels" },
                    "paddingLeft": { "type": "number", "description": "Left padding for auto-layout frame in pixels" },
                    "primaryAxisAlignItems": { "type": "string", "enum": ["MIN", "MAX", "CENTER", "SPACE_BETWEEN"], "description": "Primary axis alignment (MIN, MAX, CENTER, SPACE_BETWEEN)" },
                    "counterAxisAlignItems": { "type": "string", "enum": ["MIN", "MAX", "CENTER", "BASELINE"], "description": "Counter axis alignment (MIN, MAX, CENTER, BASELINE)" },
                    "layoutSizingHorizontal": { "type": "string", "enum": ["FIXED", "HUG", "FILL"], "description": "Horizontal layout sizing mode (FIXED, HUG, FILL)" },
                    "layoutSizingVertical": { "type": "string", "enum": ["FIXED", "HUG", "FILL"], "description": "Vertical layout sizing mode (FIXED, HUG, FILL)" },
                    "itemSpacing": { "type": "number", "description": "Distance between children in auto-layout frame in pixels" }
                }
            }),
        },
        ToolDefinition {
            name: "create_text".to_string(),
            description: "Create a new text element in Figma".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["x", "y", "text"],
                "properties": {
                    "x": { "type": "number", "description": "X position on canvas" },
                    "y": { "type": "number", "description": "Y position on canvas" },
                    "text": { "type": "string", "description": "Text content string" },
                    "fontSize": { "type": "number", "description": "Font size in pixels (default: 14)" },
                    "fontWeight": { "type": "number", "description": "Font weight (e.g., 400 for Regular, 700 for Bold)" },
                    "fontColor": {
                        "type": "object",
                        "description": "Font color in RGBA format (values 0-1)",
                        "properties": {
                            "r": { "type": "number", "minimum": 0, "maximum": 1, "description": "Red component (0-1)" },
                            "g": { "type": "number", "minimum": 0, "maximum": 1, "description": "Green component (0-1)" },
                            "b": { "type": "number", "minimum": 0, "maximum": 1, "description": "Blue component (0-1)" },
                            "a": { "type": "number", "minimum": 0, "maximum": 1, "description": "Alpha component (0-1)" }
                        },
                        "required": ["r", "g", "b"]
                    },
                    "name": { "type": "string", "description": "Semantic layer name for text node" },
                    "parentId": { "type": "string", "description": "Optional parent node ID to append text to" }
                }
            }),
        },
        ToolDefinition {
            name: "set_fill_color".to_string(),
            description: "Set the fill color of a node in Figma (TextNode or FrameNode)".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["nodeId", "r", "g", "b"],
                "properties": {
                    "nodeId": { "type": "string", "description": "The ID of the node to modify" },
                    "r": { "type": "number", "minimum": 0, "maximum": 1, "description": "Red color component (0-1)" },
                    "g": { "type": "number", "minimum": 0, "maximum": 1, "description": "Green color component (0-1)" },
                    "b": { "type": "number", "minimum": 0, "maximum": 1, "description": "Blue color component (0-1)" },
                    "a": { "type": "number", "minimum": 0, "maximum": 1, "default": 1.0, "description": "Alpha opacity component (0-1)" }
                }
            }),
        },
        ToolDefinition {
            name: "set_stroke_color".to_string(),
            description: "Set the stroke color and weight of a node in Figma".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["nodeId", "r", "g", "b"],
                "properties": {
                    "nodeId": { "type": "string", "description": "The ID of the node to modify" },
                    "r": { "type": "number", "minimum": 0, "maximum": 1, "description": "Red color component (0-1)" },
                    "g": { "type": "number", "minimum": 0, "maximum": 1, "description": "Green color component (0-1)" },
                    "b": { "type": "number", "minimum": 0, "maximum": 1, "description": "Blue color component (0-1)" },
                    "a": { "type": "number", "minimum": 0, "maximum": 1, "default": 1.0, "description": "Alpha opacity component (0-1)" },
                    "weight": { "type": "number", "minimum": 0.5, "default": 1.0, "description": "Stroke weight in pixels" }
                }
            }),
        },
        ToolDefinition {
            name: "move_node".to_string(),
            description: "Move a node to a new absolute position in Figma".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["nodeId", "x", "y"],
                "properties": {
                    "nodeId": { "type": "string", "description": "The ID of the node to move" },
                    "x": { "type": "number", "description": "New X coordinate on canvas" },
                    "y": { "type": "number", "description": "New Y coordinate on canvas" }
                }
            }),
        },
        ToolDefinition {
            name: "clone_node".to_string(),
            description: "Clone an existing node in Figma with optional new placement".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["nodeId"],
                "properties": {
                    "nodeId": { "type": "string", "description": "The ID of the node to clone" },
                    "x": { "type": "number", "description": "Optional new X position for the clone" },
                    "y": { "type": "number", "description": "Optional new Y position for the clone" }
                }
            }),
        },
        ToolDefinition {
            name: "resize_node".to_string(),
            description: "Resize a node in Figma".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["nodeId", "width", "height"],
                "properties": {
                    "nodeId": { "type": "string", "description": "The ID of the node to resize" },
                    "width": { "type": "number", "minimum": 0.01, "description": "New width in pixels" },
                    "height": { "type": "number", "minimum": 0.01, "description": "New height in pixels" }
                }
            }),
        },
        ToolDefinition {
            name: "delete_node".to_string(),
            description: "Delete a single node from Figma".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["nodeId"],
                "properties": {
                    "nodeId": { "type": "string", "description": "The ID of the node to delete" }
                }
            }),
        },
        ToolDefinition {
            name: "delete_multiple_nodes".to_string(),
            description: "Delete multiple nodes from Figma at once".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["nodeIds"],
                "properties": {
                    "nodeIds": { "type": "array", "items": { "type": "string" }, "description": "Array of node IDs to delete from canvas" }
                }
            }),
        },
        ToolDefinition {
            name: "export_node_as_image".to_string(),
            description: "Export a node as an image from Figma".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["nodeId"],
                "properties": {
                    "nodeId": { "type": "string", "description": "The ID of the node to export" },
                    "format": { "type": "string", "enum": ["PNG", "JPG", "SVG", "PDF"], "default": "PNG", "description": "Export image format (PNG, JPG, SVG, PDF)" },
                    "scale": { "type": "number", "minimum": 0.1, "maximum": 4.0, "default": 1.0, "description": "Export scale factor (e.g., 1.0, 2.0)" }
                }
            }),
        },
        ToolDefinition {
            name: "set_text_content".to_string(),
            description: "Set the text content of an existing text node in Figma".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["nodeId", "text"],
                "properties": {
                    "nodeId": { "type": "string", "description": "The ID of the text node to modify" },
                    "text": { "type": "string", "description": "New text string to set" }
                }
            }),
        },
        ToolDefinition {
            name: "get_styles".to_string(),
            description: "Get all styles from the current Figma document".to_string(),
            input_schema: json!({ "type": "object", "properties": {} }),
        },
        ToolDefinition {
            name: "get_local_components".to_string(),
            description: "Get all local components from the Figma document".to_string(),
            input_schema: json!({ "type": "object", "properties": {} }),
        },
        ToolDefinition {
            name: "get_annotations".to_string(),
            description: "Get all annotations in the current document or for a specific node".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "nodeId": { "type": "string", "description": "Optional node ID to query annotations for" },
                    "includeCategories": { "type": "boolean", "default": true, "description": "Whether to include category details in result" }
                }
            }),
        },
        ToolDefinition {
            name: "set_annotation".to_string(),
            description: "Create or update an annotation on a Figma node".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["nodeId", "labelMarkdown"],
                "properties": {
                    "nodeId": { "type": "string", "description": "The ID of the node to annotate" },
                    "annotationId": { "type": "string", "description": "The ID of the annotation to update if modifying existing" },
                    "labelMarkdown": { "type": "string", "description": "Annotation text content formatted in markdown" },
                    "categoryId": { "type": "string", "description": "The ID of the annotation category" },
                    "properties": {
                        "type": "array",
                        "description": "Additional properties array for the annotation",
                        "items": {
                            "type": "object",
                            "required": ["type"],
                            "properties": { "type": { "type": "string", "description": "Property type identifier" } }
                        }
                    }
                }
            }),
        },
        ToolDefinition {
            name: "set_multiple_annotations".to_string(),
            description: "Set multiple annotations in batches within a node".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["nodeId", "annotations"],
                "properties": {
                    "nodeId": { "type": "string", "description": "The ID of the parent node containing elements to annotate" },
                    "annotations": {
                        "type": "array",
                        "description": "Array of annotation objects to apply",
                        "items": {
                            "type": "object",
                            "required": ["nodeId", "labelMarkdown"],
                            "properties": {
                                "nodeId": { "type": "string", "description": "The ID of the node to annotate" },
                                "labelMarkdown": { "type": "string", "description": "Annotation text content in markdown" },
                                "categoryId": { "type": "string", "description": "Optional category ID" },
                                "annotationId": { "type": "string", "description": "Optional existing annotation ID to update" },
                                "properties": {
                                    "type": "array",
                                    "description": "Optional properties array",
                                    "items": {
                                        "type": "object",
                                        "required": ["type"],
                                        "properties": { "type": { "type": "string", "description": "Property type identifier" } }
                                    }
                                }
                            }
                        }
                    }
                }
            }),
        },
        ToolDefinition {
            name: "create_component_instance".to_string(),
            description: "Create an instance of a local or published Figma component".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["x", "y"],
                "properties": {
                    "componentId": { "type": "string", "description": "ID of a local component to instantiate" },
                    "componentKey": { "type": "string", "description": "Key of a published library component to instantiate" },
                    "x": { "type": "number", "description": "X position on canvas" },
                    "y": { "type": "number", "description": "Y position on canvas" },
                    "parentId": { "type": "string", "description": "Optional parent node ID to place instance inside" }
                }
            }),
        },
        ToolDefinition {
            name: "get_instance_overrides".to_string(),
            description: "Get all override properties from a selected component instance".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "nodeId": { "type": "string", "description": "Optional component instance ID to copy overrides from" }
                }
            }),
        },
        ToolDefinition {
            name: "set_instance_overrides".to_string(),
            description: "Apply previously copied overrides to target component instances".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["sourceInstanceId", "targetNodeIds"],
                "properties": {
                    "sourceInstanceId": { "type": "string", "description": "ID of the source component instance" },
                    "targetNodeIds": { "type": "array", "items": { "type": "string" }, "description": "Array of target instance IDs to apply overrides to" }
                }
            }),
        },
        ToolDefinition {
            name: "set_corner_radius".to_string(),
            description: "Set the corner radius of a node in Figma".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["nodeId", "radius"],
                "properties": {
                    "nodeId": { "type": "string", "description": "The ID of the node to modify" },
                    "radius": { "type": "number", "minimum": 0, "description": "Corner radius value in pixels" },
                    "corners": {
                        "type": "array",
                        "items": { "type": "boolean" },
                        "minItems": 4,
                        "maxItems": 4,
                        "description": "Array of 4 booleans [topLeft, topRight, bottomRight, bottomLeft]"
                    }
                }
            }),
        },
        ToolDefinition {
            name: "scan_text_nodes".to_string(),
            description: "Scan and extract all text nodes within a container node in Figma".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["nodeId"],
                "properties": {
                    "nodeId": { "type": "string", "description": "The ID of the container node to scan for text nodes" }
                }
            }),
        },
        ToolDefinition {
            name: "scan_nodes_by_types".to_string(),
            description: "Scan for child nodes with specific types (e.g. COMPONENT, FRAME, TEXT)".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["nodeId", "types"],
                "properties": {
                    "nodeId": { "type": "string", "description": "The ID of the container node to scan" },
                    "types": { "type": "array", "items": { "type": "string" }, "description": "Array of node types to search for (e.g. COMPONENT, FRAME, TEXT)" }
                }
            }),
        },
        ToolDefinition {
            name: "set_multiple_text_contents".to_string(),
            description: "Set multiple text contents parallelly in a batch within a node".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["nodeId", "text"],
                "properties": {
                    "nodeId": { "type": "string", "description": "The ID of the parent node containing text nodes" },
                    "text": {
                        "type": "array",
                        "description": "Array of text node IDs and their replacement texts",
                        "items": {
                            "type": "object",
                            "required": ["nodeId", "text"],
                            "properties": {
                                "nodeId": { "type": "string", "description": "The ID of the text node" },
                                "text": { "type": "string", "description": "Replacement text string" }
                            }
                        }
                    }
                }
            }),
        },
        ToolDefinition {
            name: "set_layout_mode".to_string(),
            description: "Set the auto-layout mode and wrap behavior of a frame in Figma".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["nodeId", "layoutMode"],
                "properties": {
                    "nodeId": { "type": "string", "description": "The ID of the frame to modify" },
                    "layoutMode": { "type": "string", "enum": ["NONE", "HORIZONTAL", "VERTICAL"], "description": "Auto-layout direction mode (NONE, HORIZONTAL, VERTICAL)" },
                    "layoutWrap": { "type": "string", "enum": ["NO_WRAP", "WRAP"], "default": "NO_WRAP", "description": "Whether the auto-layout frame wraps (NO_WRAP, WRAP)" }
                }
            }),
        },
        ToolDefinition {
            name: "set_padding".to_string(),
            description: "Set padding values for an auto-layout frame in Figma".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["nodeId"],
                "properties": {
                    "nodeId": { "type": "string", "description": "The ID of the frame to modify" },
                    "paddingTop": { "type": "number", "description": "Top padding value in pixels" },
                    "paddingRight": { "type": "number", "description": "Right padding value in pixels" },
                    "paddingBottom": { "type": "number", "description": "Bottom padding value in pixels" },
                    "paddingLeft": { "type": "number", "description": "Left padding value in pixels" }
                }
            }),
        },
        ToolDefinition {
            name: "set_axis_align".to_string(),
            description: "Set primary and counter axis alignment for an auto-layout frame".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["nodeId"],
                "properties": {
                    "nodeId": { "type": "string", "description": "The ID of the frame to modify" },
                    "primaryAxisAlignItems": { "type": "string", "enum": ["MIN", "MAX", "CENTER", "SPACE_BETWEEN"], "description": "Primary axis alignment mode (MIN, MAX, CENTER, SPACE_BETWEEN)" },
                    "counterAxisAlignItems": { "type": "string", "enum": ["MIN", "MAX", "CENTER", "BASELINE"], "description": "Counter axis alignment mode (MIN, MAX, CENTER, BASELINE)" }
                }
            }),
        },
        ToolDefinition {
            name: "set_layout_sizing".to_string(),
            description: "Set horizontal and vertical layout sizing modes (FIXED, HUG, FILL)".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["nodeId"],
                "properties": {
                    "nodeId": { "type": "string", "description": "The ID of the frame to modify" },
                    "layoutSizingHorizontal": { "type": "string", "enum": ["FIXED", "HUG", "FILL"], "description": "Horizontal sizing mode (FIXED, HUG, FILL)" },
                    "layoutSizingVertical": { "type": "string", "enum": ["FIXED", "HUG", "FILL"], "description": "Vertical sizing mode (FIXED, HUG, FILL)" }
                }
            }),
        },
        ToolDefinition {
            name: "set_item_spacing".to_string(),
            description: "Set distance between children in an auto-layout frame".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["nodeId"],
                "properties": {
                    "nodeId": { "type": "string", "description": "The ID of the frame to modify" },
                    "itemSpacing": { "type": "number", "description": "Distance between children in auto-layout frame in pixels" },
                    "counterAxisSpacing": { "type": "number", "description": "Distance between wrapped rows or columns" }
                }
            }),
        },
        ToolDefinition {
            name: "get_reactions".to_string(),
            description: "Get Figma prototyping reactions from multiple nodes".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["nodeIds"],
                "properties": {
                    "nodeIds": { "type": "array", "items": { "type": "string" }, "description": "Array of node IDs to extract prototyping reactions from" }
                }
            }),
        },
        ToolDefinition {
            name: "set_default_connector".to_string(),
            description: "Set a connector node as the default style connector".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "connectorId": { "type": "string", "description": "Optional ID of connector node to set as default style" }
                }
            }),
        },
        ToolDefinition {
            name: "create_connections".to_string(),
            description: "Create connections between nodes using the default connector style".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["connections"],
                "properties": {
                    "connections": {
                        "type": "array",
                        "description": "Array of node connection pairs to create",
                        "items": {
                            "type": "object",
                            "required": ["startNodeId", "endNodeId"],
                            "properties": {
                                "startNodeId": { "type": "string", "description": "Starting node ID" },
                                "endNodeId": { "type": "string", "description": "Ending node ID" },
                                "text": { "type": "string", "description": "Optional label text on the connector" }
                            }
                        }
                    }
                }
            }),
        },
        ToolDefinition {
            name: "set_focus".to_string(),
            description: "Focus on a specific node in Figma and pan/zoom viewport to it".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["nodeId"],
                "properties": {
                    "nodeId": { "type": "string", "description": "The ID of the node to focus and scroll viewport to" }
                }
            }),
        },
        ToolDefinition {
            name: "set_selections".to_string(),
            description: "Select multiple nodes in Figma and adjust viewport to encompass them".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["nodeIds"],
                "properties": {
                    "nodeIds": { "type": "array", "items": { "type": "string" }, "description": "Array of node IDs to select and frame on canvas" }
                }
            }),
        },
        ToolDefinition {
            name: "set_image_fill".to_string(),
            description: "Fill a node in Figma with an image from local path, URL, or base64".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["nodeId"],
                "properties": {
                    "nodeId": { "type": "string", "description": "The ID of the node to fill" },
                    "imagePath": { "type": "string", "description": "Absolute path to local image file" },
                    "imageUrl": { "type": "string", "description": "Image HTTP/HTTPS URL" },
                    "imageBase64": { "type": "string", "description": "Base64 image data string" },
                    "scaleMode": { "type": "string", "enum": ["FILL", "FIT", "CROP", "TILE"], "default": "FILL", "description": "How the image fills the node (FILL, FIT, CROP, TILE)" }
                }
            }),
        },
        ToolDefinition {
            name: "rename_node".to_string(),
            description: "Rename a node in Figma".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["nodeId", "name"],
                "properties": {
                    "nodeId": { "type": "string", "description": "The ID of the node to rename" },
                    "name": { "type": "string", "description": "The new layer name for the node" }
                }
            }),
        },
        ToolDefinition {
            name: "create_section".to_string(),
            description: "Create a section in Figma to group related content on the canvas".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["x", "y", "width", "height"],
                "properties": {
                    "x": { "type": "number", "description": "X position on canvas" },
                    "y": { "type": "number", "description": "Y position on canvas" },
                    "width": { "type": "number", "description": "Width of the section" },
                    "height": { "type": "number", "description": "Height of the section" },
                    "name": { "type": "string", "default": "Section", "description": "Optional name for the section" }
                }
            }),
        },
        ToolDefinition {
            name: "set_parent".to_string(),
            description: "Move a node into a new parent node (section, frame, or group)".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["nodeId", "parentId"],
                "properties": {
                    "nodeId": { "type": "string", "description": "The ID of the node to move" },
                    "parentId": { "type": "string", "description": "The ID of the new parent node (section, frame, or group)" },
                    "x": { "type": "number", "description": "Optional X position relative to new parent" },
                    "y": { "type": "number", "description": "Optional Y position relative to new parent" },
                    "index": { "type": "integer", "description": "Optional child index to insert at" }
                }
            }),
        },
        ToolDefinition {
            name: "join_channel".to_string(),
            description: "Join a specific WebSocket communication channel to communicate with Figma".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["channel"],
                "properties": {
                    "channel": { "type": "string", "description": "The name of the WebSocket channel to join" }
                }
            }),
        },
    ]
}

pub fn resource_definitions() -> Vec<ResourceDefinition> {
    vec![
        ResourceDefinition {
            uri: "rune://figma/document".to_string(),
            name: "Figma Document Info".to_string(),
            description: "Root metadata and page layout of the current Figma document".to_string(),
            mime_type: Some("application/json".to_string()),
        },
        ResourceDefinition {
            uri: "rune://figma/selection".to_string(),
            name: "Figma Current Selection".to_string(),
            description: "Details of currently selected nodes on the active page".to_string(),
            mime_type: Some("application/json".to_string()),
        },
        ResourceDefinition {
            uri: "rune://figma/styles".to_string(),
            name: "Figma Document Styles".to_string(),
            description: "Color, text, effect, and grid styles declared in this document"
                .to_string(),
            mime_type: Some("application/json".to_string()),
        },
    ]
}

pub fn prompt_definitions() -> Vec<PromptDefinition> {
    vec![
        PromptDefinition {
            name: "design_strategy".to_string(),
            description: "Best practices and hierarchical strategy for authoring Figma designs".to_string(),
            arguments: json!([]),
        },
        PromptDefinition {
            name: "read_design_strategy".to_string(),
            description: "Best practices for inspecting, parsing, and reading Figma designs".to_string(),
            arguments: json!([]),
        },
        PromptDefinition {
            name: "text_replacement_strategy".to_string(),
            description: "Systematic chunked approach for replacing text and preserving visual layout".to_string(),
            arguments: json!([]),
        },
        PromptDefinition {
            name: "annotation_conversion_strategy".to_string(),
            description: "Strategy for converting manual visual callouts to native Figma annotations".to_string(),
            arguments: json!([]),
        },
        PromptDefinition {
            name: "swap_overrides_instances".to_string(),
            description: "Guide to swap overrides and transfer customized content between component instances".to_string(),
            arguments: json!([]),
        },
        PromptDefinition {
            name: "reaction_to_connector_strategy".to_string(),
            description: "Convert Figma prototype reactions to visual canvas connector lines".to_string(),
            arguments: json!([]),
        },
    ]
}
