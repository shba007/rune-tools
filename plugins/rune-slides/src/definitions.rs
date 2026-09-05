use rune_pdk::ToolDefinition;
use serde_json::json;

pub fn tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "slide_init".to_string(),
            description:
                "Initializes a new presentation project file and central theme configuration."
                    .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "projectPath": { "type": "string", "description": "Path to the presentation project JSON bundle" },
                    "title": { "type": "string", "description": "Presentation title" },
                    "themeName": { "type": "string", "default": "modern", "description": "Base theme identifier" }
                },
                "required": ["title"]
            }),
        },
        ToolDefinition {
            name: "slide_update_theme".to_string(),
            description:
                "Defines or updates the central theme styling configuration via markdown/CSS rules."
                    .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "projectPath": { "type": "string", "description": "Path to the presentation project" },
                    "themeDefinition": { "type": "string", "description": "Theme styling definitions and CSS rules" }
                },
                "required": ["themeDefinition"]
            }),
        },
        ToolDefinition {
            name: "slide_add_page".to_string(),
            description: "Adds or inserts a new slide page into the presentation sequence."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "projectPath": { "type": "string", "description": "Path to the presentation project" },
                    "slideTitle": { "type": "string", "description": "Title of the new slide" },
                    "content": { "type": "string", "description": "Markdown content for the slide body" },
                    "layout": { "type": "string", "description": "Optional layout style" },
                    "index": { "type": "integer", "description": "Optional 0-based insertion index" }
                },
                "required": ["slideTitle", "content"]
            }),
        },
        ToolDefinition {
            name: "slide_delete_page".to_string(),
            description:
                "Deletes a specific slide page from the presentation by its 0-based index."
                    .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "projectPath": { "type": "string", "description": "Path to the presentation project" },
                    "index": { "type": "integer", "description": "0-based index of the slide to remove" }
                },
                "required": ["index"]
            }),
        },
ToolDefinition {
            name: "slide_export".to_string(),
            description: "Exports the presentation project into a native PowerPoint (.pptx) or a styled PDF (.pdf).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "projectPath": { "type": "string", "description": "Path to the presentation project" },
                    "outputPath": { "type": "string", "description": "Destination file path (e.g., presentation.pdf or presentation.pptx)" },
                    "format": { "type": "string", "enum": ["pptx", "pdf"], "default": "pptx", "description": "Target export format" }
                }
            }),
        },
    ]
}
