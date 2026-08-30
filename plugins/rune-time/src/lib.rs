// plugins/rune-time/src/lib.rs
use chrono::{DateTime, NaiveDateTime, Offset, Utc};
use chrono_tz::Tz;
use extism_pdk::*;
use rune_pdk::{ToolCallRequest, ToolDefinition};
use serde_json::{Value, json};
use std::str::FromStr;

/// Internal execution handler where `?` operates on `Result<Value, String>`
fn execute_tool(request: ToolCallRequest) -> Result<Value, String> {
    match request.name.as_str() {
        "get_current_time" => {
            // Default timezone can be configured via runner parameter `default_timezone`
            let default_tz_str = config::get("default_timezone")
                .unwrap_or(None)
                .unwrap_or_else(|| "UTC".to_string());

            let tz_str = request.arguments["timezone"]
                .as_str()
                .unwrap_or(&default_tz_str);

            let tz: Tz = Tz::from_str(tz_str).map_err(|_| {
                format!(
                    "Invalid IANA timezone '{}'. Example: 'America/New_York' or 'Asia/Kolkata'",
                    tz_str
                )
            })?;

            let now_utc: DateTime<Utc> = Utc::now();
            let now_local = now_utc.with_timezone(&tz);

            Ok(json!({
                "timezone": tz.name(),
                "datetime": now_local.to_rfc3339(),
                "is_dst": false
            }))
        }

        "convert_time" => {
            let source_tz_str = request.arguments["source_timezone"]
                .as_str()
                .ok_or_else(|| "Missing 'source_timezone' argument".to_string())?;

            let target_tz_str = request.arguments["target_timezone"]
                .as_str()
                .ok_or_else(|| "Missing 'target_timezone' argument".to_string())?;

            let time_str = request.arguments["time"]
                .as_str()
                .ok_or_else(|| "Missing 'time' argument (format: 'YYYY-MM-DDTHH:MM:SS' or 'YYYY-MM-DD HH:MM:SS')".to_string())?;

            let source_tz: Tz = Tz::from_str(source_tz_str)
                .map_err(|_| format!("Invalid source timezone: {}", source_tz_str))?;

            let target_tz: Tz = Tz::from_str(target_tz_str)
                .map_err(|_| format!("Invalid target timezone: {}", target_tz_str))?;

            // Normalize timestamp string and parse
            let clean_time = time_str.replace('T', " ");
            let naive_dt = NaiveDateTime::parse_from_str(&clean_time, "%Y-%m-%d %H:%M:%S")
                .map_err(|e| {
                    format!(
                        "Failed to parse time '{}': {}. Expected format: 'YYYY-MM-DDTHH:MM:SS'",
                        time_str, e
                    )
                })?;

            let source_dt = naive_dt
                .and_local_timezone(source_tz)
                .single()
                .ok_or_else(|| "Ambiguous or invalid local time in source timezone".to_string())?;

            let target_dt = source_dt.with_timezone(&target_tz);

            let offset_diff_seconds = target_dt.offset().fix().local_minus_utc()
                - source_dt.offset().fix().local_minus_utc();
            let offset_diff_hours = offset_diff_seconds as f64 / 3600.0;

            Ok(json!({
                "source": {
                    "timezone": source_tz.name(),
                    "datetime": source_dt.to_rfc3339()
                },
                "target": {
                    "timezone": target_tz.name(),
                    "datetime": target_dt.to_rfc3339()
                },
                "time_difference_hours": offset_diff_hours
            }))
        }

        unknown => Err(format!("Unknown tool: {}", unknown)),
    }
}

#[plugin_fn]
pub fn mcp_info(_: ()) -> FnResult<String> {
    let info = json!({
        "name": env!("CARGO_PKG_NAME"),
        "version": env!("CARGO_PKG_VERSION"),
        "description": option_env!("CARGO_PKG_DESCRIPTION")
    });
    Ok(serde_json::to_string(&info)?)
}

/// Exported standard MCP Tool List
#[plugin_fn]
pub fn mcp_list_tools(_: ()) -> FnResult<String> {
    let tools = vec![
        ToolDefinition {
            name: "get_current_time".to_string(),
            description: "Get the current time in a specified IANA timezone (e.g., 'America/New_York', 'Asia/Kolkata', 'Europe/London')".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "timezone": {
                        "type": "string",
                        "description": "Optional IANA timezone name. Defaults to UTC or configured default."
                    }
                }
            }),
        },
        ToolDefinition {
            name: "convert_time".to_string(),
            description: "Convert a datetime string from one timezone to another".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "source_timezone": {
                        "type": "string",
                        "description": "The source IANA timezone name (e.g., 'UTC', 'America/Los_Angeles')"
                    },
                    "target_timezone": {
                        "type": "string",
                        "description": "The target IANA timezone name (e.g., 'Asia/Tokyo', 'Europe/Paris')"
                    },
                    "time": {
                        "type": "string",
                        "description": "The 24-hour datetime to convert in 'YYYY-MM-DDTHH:MM:SS' format"
                    }
                },
                "required": ["source_timezone", "target_timezone", "time"]
            }),
        },
    ];

    Ok(serde_json::to_string(&tools)?)
}

/// Exported standard MCP Tool Invocation
#[plugin_fn]
pub fn mcp_call_tool(input: String) -> FnResult<String> {
    let request: ToolCallRequest = serde_json::from_str(&input)?;
    let result = execute_tool(request);

    let output = match result {
        Ok(val) => json!({ "status": "success", "result": val }),
        Err(err) => json!({ "status": "error", "error": err }),
    };

    Ok(serde_json::to_string(&output)?)
}
