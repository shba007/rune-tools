use crate::types::{ConvertTimeResponse, CurrentTimeResponse, TimezoneDetails};
use chrono::{DateTime, NaiveDate, NaiveDateTime, Offset, Utc};
use chrono_tz::Tz;
use rune_pdk::ToolCallRequest;
use serde_json::{Value, json};
use std::str::FromStr;

fn get_str_arg(args: &Value, camel: &str, snake: &str) -> Option<String> {
    if let Some(val) = args
        .get(camel)
        .or_else(|| args.get(snake))
        .and_then(Value::as_str)
    {
        return Some(val.to_string());
    }
    let env_snake = snake.to_ascii_uppercase();
    let env_camel = camel.to_ascii_uppercase();
    std::env::var(&env_snake)
        .or_else(|_| std::env::var(&env_camel))
        .ok()
}

pub fn get_config(key: &str) -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        extism_pdk::config::get(key)
            .ok()
            .flatten()
            .or_else(|| std::env::var(key.to_ascii_uppercase()).ok())
            .filter(|s| !s.is_empty())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::env::var(key.to_ascii_uppercase())
            .or_else(|_| std::env::var(key))
            .ok()
            .filter(|s| !s.is_empty())
    }
}

fn parse_flexible_datetime(input: &str) -> Result<NaiveDateTime, String> {
    let clean = input.trim();
    if clean.is_empty() {
        return Err("Datetime string cannot be empty".to_string());
    }

    if let Ok(dt) = DateTime::parse_from_rfc3339(clean) {
        return Ok(dt.naive_utc());
    }

    let normalized = clean.replace('T', " ");
    let formats = [
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%d %H:%M",
        "%Y/%m/%d %H:%M:%S",
        "%Y/%m/%d %H:%M",
    ];

    for fmt in formats {
        if let Ok(ndt) = NaiveDateTime::parse_from_str(&normalized, fmt) {
            return Ok(ndt);
        }
    }

    if let Ok(nd) = NaiveDate::parse_from_str(clean, "%Y-%m-%d")
        && let Some(ndt) = nd.and_hms_opt(0, 0, 0)
    {
        return Ok(ndt);
    }

    Err(format!(
        "Failed to parse datetime '{}'. Expected ISO-8601 format like 'YYYY-MM-DDTHH:MM:SS'",
        clean
    ))
}

pub fn execute_tool(request: ToolCallRequest) -> Result<Value, String> {
    match request.name.as_str() {
        "get_current_time" => {
            let default_tz_str =
                get_config("default_timezone").unwrap_or_else(|| "UTC".to_string());
            let tz_param =
                get_str_arg(&request.arguments, "timezone", "timezone").unwrap_or(default_tz_str);

            let tz: Tz = Tz::from_str(tz_param.trim()).map_err(|_| {
                format!(
                    "Invalid IANA timezone '{}'. Example: 'America/New_York' or 'Asia/Kolkata'",
                    tz_param
                )
            })?;

            let now_utc: DateTime<Utc> = Utc::now();
            let now_local = now_utc.with_timezone(&tz);
            let offset = now_local.offset().fix();

            Ok(json!(CurrentTimeResponse {
                timezone: tz.name().to_string(),
                datetime: now_local.to_rfc3339(),
                utc_datetime: now_utc.to_rfc3339(),
                utc_offset: offset.to_string(),
                timestamp_epoch_seconds: now_utc.timestamp(),
            }))
        }

        "convert_time" => {
            let source_tz_str =
                get_str_arg(&request.arguments, "sourceTimezone", "source_timezone")
                    .ok_or_else(|| "Missing 'source_timezone' parameter".to_string())?;
            if source_tz_str.trim().is_empty() {
                return Err("Parameter 'source_timezone' cannot be empty".to_string());
            }

            let target_tz_str =
                get_str_arg(&request.arguments, "targetTimezone", "target_timezone")
                    .ok_or_else(|| "Missing 'target_timezone' parameter".to_string())?;
            if target_tz_str.trim().is_empty() {
                return Err("Parameter 'target_timezone' cannot be empty".to_string());
            }

            let time_str = get_str_arg(&request.arguments, "time", "time")
                .ok_or_else(|| "Missing 'time' parameter".to_string())?;
            if time_str.trim().is_empty() {
                return Err("Parameter 'time' cannot be empty".to_string());
            }

            let source_tz: Tz = Tz::from_str(source_tz_str.trim())
                .map_err(|_| format!("Invalid source IANA timezone: '{}'", source_tz_str))?;

            let target_tz: Tz = Tz::from_str(target_tz_str.trim())
                .map_err(|_| format!("Invalid target IANA timezone: '{}'", target_tz_str))?;

            let naive_dt = parse_flexible_datetime(&time_str)?;

            let source_dt = naive_dt
                .and_local_timezone(source_tz)
                .single()
                .ok_or_else(|| "Ambiguous or invalid local datetime for the source timezone (e.g. DST gap/overlap)".to_string())?;

            let target_dt = source_dt.with_timezone(&target_tz);

            let offset_diff_seconds = target_dt.offset().fix().local_minus_utc()
                - source_dt.offset().fix().local_minus_utc();
            let offset_diff_hours = offset_diff_seconds as f64 / 3600.0;

            Ok(json!(ConvertTimeResponse {
                source: TimezoneDetails {
                    timezone: source_tz.name().to_string(),
                    datetime: source_dt.to_rfc3339(),
                    utc_offset: source_dt.offset().fix().to_string(),
                },
                target: TimezoneDetails {
                    timezone: target_tz.name().to_string(),
                    datetime: target_dt.to_rfc3339(),
                    utc_offset: target_dt.offset().fix().to_string(),
                },
                time_difference_hours: offset_diff_hours,
            }))
        }

        unknown => Err(format!("Unknown tool: {}", unknown)),
    }
}
