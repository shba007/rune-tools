use rune_pdk::{ToolCallRequest, ToolDefinition};
use serde_json::{Map, Value, json};
use std::collections::HashMap;

/// Parses raw CLI string flags into a key-value dictionary.
/// Supports `--key value`, `--key=value`, and boolean flags `--key`.
pub fn parse_cli_args(args: &[String]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let mut idx = 0;

    while idx < args.len() {
        let arg = &args[idx];
        if let Some(stripped) = arg.strip_prefix("--") {
            if let Some((k, v)) = stripped.split_once('=') {
                map.insert(k.replace('-', "_"), v.to_string());
            } else if idx + 1 < args.len() && !args[idx + 1].starts_with('-') {
                map.insert(stripped.replace('-', "_"), args[idx + 1].clone());
                idx += 1;
            } else {
                map.insert(stripped.replace('-', "_"), "true".to_string());
            }
        }
        idx += 1;
    }

    map
}

/// Resolves tool arguments prioritizing CLI values over uppercase environment variables.
///
/// Precedence:
/// 1. CLI flag (e.g., `--max_length 500` or `--max-length=500`)
/// 2. Uppercase Environment variable (e.g., `MAX_LENGTH=500`)
pub fn resolve_arguments(cli_args: &[String], tool_def: &ToolDefinition) -> Result<Value, String> {
    let cli_map = parse_cli_args(cli_args);
    let mut resolved_map = Map::new();

    let properties = tool_def
        .input_schema
        .get("properties")
        .and_then(Value::as_object);

    if let Some(props) = properties {
        for (prop_name, prop_spec) in props {
            let target_type = prop_spec
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("string");

            let env_key = prop_name.to_ascii_uppercase();

            let raw_value_opt = if let Some(cli_val) = cli_map.get(prop_name) {
                Some(cli_val.clone())
            } else if let Ok(env_val) = std::env::var(&env_key) {
                Some(env_val)
            } else {
                None
            };

            if let Some(raw_val) = raw_value_opt {
                let parsed_val = parse_typed_value(&raw_val, target_type)?;
                resolved_map.insert(prop_name.clone(), parsed_val);
            }
        }
    }

    // Preserve any ad-hoc CLI flags not declared in explicit properties
    for (k, v) in cli_map {
        if !resolved_map.contains_key(&k) {
            resolved_map.insert(k, Value::String(v));
        }
    }

    Ok(Value::Object(resolved_map))
}

fn parse_typed_value(raw: &str, target_type: &str) -> Result<Value, String> {
    match target_type {
        "integer" | "number" => raw
            .parse::<i64>()
            .map(|n| json!(n))
            .or_else(|_| raw.parse::<f64>().map(|n| json!(n)))
            .map_err(|_| format!("Failed to parse '{}' as number", raw)),
        "boolean" => match raw.to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" => Ok(json!(true)),
            "false" | "0" | "no" => Ok(json!(false)),
            _ => Err(format!("Failed to parse '{}' as boolean", raw)),
        },
        "object" | "array" => serde_json::from_str(raw)
            .map_err(|e| format!("Failed to parse '{}' as JSON: {}", raw, e)),
        _ => Ok(json!(raw)),
    }
}

/// Executes a tool dispatch through the resolved arguments pipeline.
pub fn execute_sidecar<F>(
    args: &[String],
    tool_def: ToolDefinition,
    executor: F,
) -> Result<Value, String>
where
    F: FnOnce(ToolCallRequest) -> Result<Value, String>,
{
    let resolved_args = resolve_arguments(args, &tool_def)?;
    let request = ToolCallRequest {
        name: tool_def.name,
        arguments: resolved_args,
    };
    executor(request)
}
