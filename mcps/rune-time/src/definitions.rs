use rune_pdk::{ToolDefinition, ToolDefinitionWithOutputSchema, ToolRules};
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
            rules: Some(ToolRules::read_only()),
        },
        ToolDefinition {
            name: "convert_time".to_string(),
            description: "Converts a date and time string from a source IANA timezone to a destination timezone.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "sourceTimezone": {
                        "type": "string",
                        "description": "Source IANA timezone name (e.g. 'UTC', 'America/Los_Angeles')"
                    },
                    "targetTimezone": {
                        "type": "string",
                        "description": "Destination IANA timezone name (e.g. 'Asia/Tokyo', 'Europe/London')"
                    },
                    "time": {
                        "type": "string",
                        "description": "Datetime string to convert (e.g. '2026-09-02T14:30:00' or '2026-09-02 14:30:00')"
                    }
                },
                "required": ["sourceTimezone", "targetTimezone", "time"]
            }),
            rules: Some(ToolRules::read_only()),
        },
    ]
}

/// Wire definitions, including output contracts. Keep the base definitions available
/// to existing input/rules checks and the workspace's input-only Postman generator.
pub fn published_tool_definitions() -> Vec<ToolDefinitionWithOutputSchema> {
    tool_definitions()
        .into_iter()
        .map(|tool| {
            let schema = match tool.name.as_str() {
                "get_current_time" => json!({
                    "type": "object",
                    "properties": {
                        "timezone": {"type": "string", "description": "Canonical IANA timezone of the local datetime."},
                        "datetime": {"type": "string", "description": "Current local datetime in RFC 3339 format."},
                        "utcDatetime": {"type": "string", "description": "The same instant in UTC, in RFC 3339 format."},
                        "utcOffset": {"type": "string", "description": "Local UTC offset formatted as a signed hours-and-minutes offset."},
                        "timestampEpochSeconds": {"type": "integer", "minimum": i64::MIN, "maximum": i64::MAX, "description": "Whole seconds since the Unix epoch at the current instant."}
                    },
                    "required": ["timezone", "datetime", "utcDatetime", "utcOffset", "timestampEpochSeconds"],
                    "additionalProperties": false
                }),
                "convert_time" => json!({
                    "type": "object",
                    "properties": {
                        "source": timezone_details_schema("Source timezone and datetime before conversion."),
                        "target": timezone_details_schema("Destination timezone and converted datetime."),
                        "timeDifferenceHours": {"type": "number", "description": "Destination UTC offset minus source UTC offset, in hours at this instant."}
                    },
                    "required": ["source", "target", "timeDifferenceHours"],
                    "additionalProperties": false
                }),
                _ => unreachable!("every rune-time tool must declare an output schema"),
            };
            tool.with_output_schema(schema)
        })
        .collect()
}

fn timezone_details_schema(description: &str) -> serde_json::Value {
    json!({
        "type": "object",
        "description": description,
        "properties": {
            "timezone": {"type": "string", "description": "Canonical IANA timezone of this datetime."},
            "datetime": {"type": "string", "description": "Datetime in this timezone, in RFC 3339 format."},
            "utcOffset": {"type": "string", "description": "UTC offset formatted as a signed hours-and-minutes offset."}
        },
        "required": ["timezone", "datetime", "utcOffset"],
        "additionalProperties": false
    })
}
