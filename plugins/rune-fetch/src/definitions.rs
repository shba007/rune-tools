use rune_pdk::ToolDefinition;
use serde_json::json;

pub fn tool_definitions() -> Vec<ToolDefinition> {
    vec![ToolDefinition {
        name: "fetch".to_string(),
        description:
            "Fetches a URL from the internet and extracts its contents as markdown (or raw text)."
                .to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The full HTTP/HTTPS URL to fetch"
                },
                "max_length": {
                    "type": "integer",
                    "description": "Maximum number of characters to return (default: 50000)"
                },
                "start_index": {
                    "type": "integer",
                    "description": "Start character index for pagination (default: 0)"
                },
                "raw": {
                    "type": "boolean",
                    "description": "Return raw HTML/content instead of converting to Markdown"
                },
                "paginate": {
                    "type": "boolean",
                    "description": "Enable cursor pagination with next_start_index"
                }
            },
            "required": ["url"]
        }),
    }]
}
