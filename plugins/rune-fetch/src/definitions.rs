use rune_pdk::{PromptDefinition, ResourceDefinition, ToolDefinition};
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
                    "description": "The full HTTP/HTTPS URL to fetch (e.g. 'https://example.com')"
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
                    "description": "Return raw HTML/content instead of converting to Markdown (default: false)"
                },
                "paginate": {
                    "type": "boolean",
                    "description": "Enable cursor pagination with next_start_index (default: false)"
                }
            },
            "required": ["url"]
        }),
    }]
}

pub fn resource_definitions() -> Vec<ResourceDefinition> {
    vec![ResourceDefinition {
        uri: "rune://fetch/help".to_string(),
        name: "Fetch Documentation".to_string(),
        description:
            "Overview and guidelines for URL fetching, Markdown conversion, and pagination."
                .to_string(),
        mime_type: Some("text/markdown".to_string()),
    }]
}

pub fn prompt_definitions() -> Vec<PromptDefinition> {
    vec![
        PromptDefinition {
            name: "fetch_and_summarize".to_string(),
            description: "Fetches a web page and produces a structured summary of its contents."
                .to_string(),
            arguments: json!([
                {
                    "name": "url",
                    "description": "The HTTP or HTTPS URL to fetch and summarize",
                    "required": true
                }
            ]),
        },
        PromptDefinition {
            name: "fetch_and_extract_markdown".to_string(),
            description:
                "Fetches a URL and formats the page contents as clean, structured Markdown."
                    .to_string(),
            arguments: json!([
                {
                    "name": "url",
                    "description": "The HTTP or HTTPS URL to extract",
                    "required": true
                }
            ]),
        },
    ]
}
