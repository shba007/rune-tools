use rune_pdk::{PromptDefinition, ResourceDefinition, ToolCallRequest, ToolDefinition};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::{self, BufRead, Write};

pub trait SidecarHandler {
    fn info(&self) -> Value;
    fn list_tools(&self) -> Vec<ToolDefinition>;
    fn call_tool(&self, req: ToolCallRequest) -> Result<Value, String>;

    fn list_resources(&self) -> Vec<ResourceDefinition> {
        vec![]
    }

    fn read_resource(&self, _uri: &str) -> Result<Value, String> {
        Err("Resources not supported by this sidecar".to_string())
    }

    fn list_prompts(&self) -> Vec<PromptDefinition> {
        vec![]
    }

    fn get_prompt(&self, _name: &str, _args: Value) -> Result<Value, String> {
        Err("Prompts not supported by this sidecar".to_string())
    }
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
    #[serde(default)]
    uri: Option<String>,
}

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
                    "list_resources" | "mcp_list_resources" | "resources/list" => {
                        let resources = handler.list_resources();
                        format_response(id, Ok(json!(resources)))
                    }
                    "read_resource" | "mcp_read_resource" | "resources/read" => {
                        let uri = req
                            .uri
                            .or_else(|| {
                                req.params
                                    .as_ref()
                                    .and_then(|p| p.get("uri"))
                                    .and_then(Value::as_str)
                                    .map(ToString::to_string)
                            })
                            .unwrap_or_default();
                        let result = handler.read_resource(&uri);
                        format_response(id, result)
                    }
                    "list_prompts" | "mcp_list_prompts" | "prompts/list" => {
                        let prompts = handler.list_prompts();
                        format_response(id, Ok(json!(prompts)))
                    }
                    "get_prompt" | "mcp_get_prompt" | "prompts/get" => {
                        let prompt_name = req
                            .name
                            .or_else(|| {
                                req.params
                                    .as_ref()
                                    .and_then(|p| p.get("name"))
                                    .and_then(Value::as_str)
                                    .map(ToString::to_string)
                            })
                            .unwrap_or_default();
                        let args = req
                            .arguments
                            .or_else(|| {
                                req.params
                                    .as_ref()
                                    .and_then(|p| p.get("arguments"))
                                    .cloned()
                            })
                            .unwrap_or(json!({}));
                        let result = handler.get_prompt(&prompt_name, args);
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
                    unknown => format_response(id, Err(format!("Unknown method: {}", unknown))),
                }
            }
            Err(e) => json!({
                "jsonrpc": "2.0",
                "id": Value::Null,
                "error": { "code": -32700, "message": format!("Invalid JSON payload: {}", e) }
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
