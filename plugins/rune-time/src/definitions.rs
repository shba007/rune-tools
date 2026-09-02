use rune_pdk::ToolDefinition;
use serde_json::json;

pub fn tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "get_current_time".to_string(),
            description: "Retrieves the current date and time in UTC and a target IANA timezone with offset details.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "timezone": {
                        "type": "string",
                        "description": "Target IANA timezone name (e.g. 'America/New_York', 'Asia/Kolkata', 'UTC'). Defaults to UTC."
                    }
                }
            }),
        },
        ToolDefinition {
            name: "convert_time".to_string(),
            description: "Converts a date and time string from a source IANA timezone to a destination timezone.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "source_timezone": {
                        "type": "string",
                        "description": "Source IANA timezone name (e.g. 'UTC', 'America/Los_Angeles')"
                    },
                    "target_timezone": {
                        "type": "string",
                        "description": "Destination IANA timezone name (e.g. 'Asia/Tokyo', 'Europe/London')"
                    },
                    "time": {
                        "type": "string",
                        "description": "Datetime string to convert (e.g. '2026-09-02T14:30:00' or '2026-09-02 14:30:00')"
                    }
                },
                "required": ["source_timezone", "target_timezone", "time"]
            }),
        },
    ]
}
