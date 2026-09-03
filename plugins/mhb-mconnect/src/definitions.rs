use rune_pdk::ToolDefinition;
use serde_json::json;

pub fn tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "mhb_list_templates".to_string(),
            description: "Retrieves all available email templates from the MHB interaction service.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
        ToolDefinition {
            name: "mhb_get_template".to_string(),
            description: "Fetches a specific email template definition and its required variable schema by ID.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "templateId": {
                        "type": "string",
                        "description": "Unique identifier of the email template (e.g., 'internship-completion-certificate')"
                    }
                },
                "required": ["templateId"]
            }),
        },
        ToolDefinition {
            name: "mhb_render_template_preview".to_string(),
            description: "Submits variables to render a fully styled HTML email template preview.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "templateId": {
                        "type": "string",
                        "description": "Identifier of the template to render"
                    },
                    "variables": {
                        "type": "object",
                        "description": "JSON object containing data values mapped to the template's expected variables"
                    }
                },
                "required": ["templateId", "variables"]
            }),
        },
    ]
}
