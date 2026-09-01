use rune_pdk::{ToolCallRequest, ToolDefinition};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use std::collections::HashMap;
use std::io::{self, BufRead, Write};

/// Trait implemented by native sidecar entry points to expose tool metadata and invocation.
pub trait SidecarHandler {
    fn info(&self) -> Value;
    fn list_tools(&self) -> Vec<ToolDefinition>;
    fn call_tool(&self, req: ToolCallRequest) -> Result<Value, String>;
}

#[derive(Deserialize)]
struct SidecarRpcRequest {
    #[serde(default)]
    id: Option<Value>,
    #[serde(default)]
    method: Option<String>,
    #[serde(default)]
    params: Option<Value>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    arguments: Option<Value>,
}

/// Runs a persistent newline-delimited stdio server for native sidecar binaries.
pub fn run_stdio<H: SidecarHandler>(handler: H) -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let response = match serde_json::from_str::<SidecarRpcRequest>(trimmed) {
            Ok(req) => {
                let id = req.id.clone();
                let method = req.method.as_deref().unwrap_or("");

                match method {
                    "info" | "mcp_info" => {
                        let info_val = handler.info();
                        format_response(id, Ok(info_val))
                    }
                    "list_tools" | "mcp_list_tools" | "tools/list" => {
                        let tools = handler.list_tools();
                        format_response(id, Ok(json!(tools)))
                    }
                    "call_tool" | "mcp_call_tool" | "tools/call" => {
                        let tool_req = if let Some(params) = req.params {
                            if let Ok(r) = serde_json::from_value::<ToolCallRequest>(params.clone())
                            {
                                r
                            } else {
                                ToolCallRequest {
                                    name: params
                                        .get("name")
                                        .and_then(Value::as_str)
                                        .unwrap_or_default()
                                        .to_string(),
                                    arguments: params
                                        .get("arguments")
                                        .cloned()
                                        .unwrap_or(json!({})),
                                }
                            }
                        } else if let Some(name) = req.name {
                            ToolCallRequest {
                                name,
                                arguments: req.arguments.unwrap_or(json!({})),
                            }
                        } else {
                            ToolCallRequest {
                                name: String::new(),
                                arguments: json!({}),
                            }
                        };
                        let result = handler.call_tool(tool_req);
                        format_response(id, result)
                    }
                    _ if req.name.is_some() => {
                        let tool_req = ToolCallRequest {
                            name: req.name.unwrap(),
                            arguments: req.arguments.unwrap_or(json!({})),
                        };
                        let result = handler.call_tool(tool_req);
                        format_response(id, result)
                    }
                    unknown => json!({
                        "status": "error",
                        "error": format!("Unknown method: {}", unknown)
                    }),
                }
            }
            Err(e) => json!({
                "status": "error",
                "error": format!("Invalid JSON payload: {}", e)
            }),
        };

        writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
        stdout.flush()?;
    }

    Ok(())
}

fn format_response(id: Option<Value>, result: Result<Value, String>) -> Value {
    if let Some(id_val) = id {
        match result {
            Ok(val) => json!({ "jsonrpc": "2.0", "id": id_val, "result": val }),
            Err(err) => json!({
                "jsonrpc": "2.0",
                "id": id_val,
                "error": { "code": -32000, "message": err }
            }),
        }
    } else {
        match result {
            Ok(val) => json!({ "status": "success", "result": val }),
            Err(err) => json!({ "status": "error", "error": err }),
        }
    }
}

/// Parses raw CLI string flags into a key-value dictionary.
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

/// Resolves tool arguments prioritizing CLI values over uppercase environment variables (`CLI > ENV`).
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
