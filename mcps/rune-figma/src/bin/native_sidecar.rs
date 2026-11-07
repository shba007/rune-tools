#[cfg(feature = "native")]
use futures_util::{SinkExt, StreamExt};
#[cfg(feature = "native")]
use rune_figma::{definitions, operations};
#[cfg(feature = "native")]
use serde_json::{Value, json};
#[cfg(feature = "native")]
use std::sync::Arc;
#[cfg(feature = "native")]
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
#[cfg(feature = "native")]
use tokio::sync::Mutex;
#[cfg(feature = "native")]
use tokio::time::{Duration, timeout};
#[cfg(feature = "native")]
use tokio_tungstenite::connect_async;
#[cfg(feature = "native")]
use tokio_tungstenite::tungstenite::Message;

#[cfg(feature = "native")]
struct FigmaBridgeState {
    channel: Option<String>,
    ws_url: String,
}

#[cfg(feature = "native")]
impl FigmaBridgeState {
    fn new() -> Self {
        let ws_url = std::env::var("FIGMA_WS_URL")
            .or_else(|_| std::env::var("FIGMA_PORT").map(|p| format!("ws://127.0.0.1:{}", p)))
            .unwrap_or_else(|_| "ws://127.0.0.1:3055".to_string());
        let channel = std::env::var("FIGMA_CHANNEL")
            .ok()
            .filter(|c| !c.trim().is_empty());

        Self { channel, ws_url }
    }
}

#[cfg(feature = "native")]
async fn execute_native_tool(
    name: &str,
    args: Value,
    state: Arc<Mutex<FigmaBridgeState>>,
) -> Result<Value, String> {
    let tools = definitions::tool_definitions();
    let tool = tools
        .iter()
        .find(|t| t.name == name)
        .ok_or_else(|| format!("Unknown tool: '{}'", name))?;

    operations::validate_tool_schema(tool, &args)?;

    if name == "join_channel" {
        let channel_name = args
            .get("channel")
            .and_then(Value::as_str)
            .ok_or_else(|| "Missing 'channel' parameter".to_string())?
            .to_string();

        let ws_url = {
            let mut lock = state.lock().await;
            lock.channel = Some(channel_name.clone());
            lock.ws_url.clone()
        };

        let (mut ws_stream, _) = timeout(Duration::from_secs(5), connect_async(&ws_url))
            .await
            .map_err(|_| format!("Connection timeout to TalkToFigma WebSocket relay at {}", ws_url))?
            .map_err(|e| format!("Cannot connect to TalkToFigma WebSocket at {}: {}. Ensure TalkToFigma desktop/socket is running.", ws_url, e))?;

        let join_msg = json!({
            "type": "join",
            "channel": channel_name
        });

        ws_stream
            .send(Message::Text(join_msg.to_string().into()))
            .await
            .map_err(|e| format!("Failed to send join channel packet: {}", e))?;

        return Ok(json!({
            "status": "success",
            "channel": channel_name,
            "message": format!("Successfully connected to TalkToFigma channel '{}'", channel_name)
        }));
    }

    let (channel, ws_url) = {
        let lock = state.lock().await;
        (lock.channel.clone(), lock.ws_url.clone())
    };

    let active_channel = channel.ok_or_else(|| {
        "No active Figma channel. Please call 'join_channel' with your TalkToFigma plugin channel ID first."
            .to_string()
    })?;

    let (mut ws_stream, _) = timeout(Duration::from_secs(5), connect_async(&ws_url))
        .await
        .map_err(|_| format!("Connection timeout to TalkToFigma WebSocket relay at {}", ws_url))?
        .map_err(|e| format!("Cannot connect to TalkToFigma WebSocket at {}: {}. Ensure TalkToFigma desktop/socket is running.", ws_url, e))?;

    let req_id = format!(
        "req_{}_{}",
        name,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    );
    let payload = json!({
        "id": req_id,
        "type": "message",
        "channel": active_channel,
        "message": {
            "id": req_id,
            "command": name,
            "params": args
        }
    });

    ws_stream
        .send(Message::Text(payload.to_string().into()))
        .await
        .map_err(|e| format!("Failed to dispatch command to Figma WebSocket: {}", e))?;

    let response_timeout = Duration::from_secs(30);
    let response_future = async {
        while let Some(msg_result) = ws_stream.next().await {
            match msg_result {
                Ok(Message::Text(text)) => {
                    if let Ok(parsed) = serde_json::from_str::<Value>(&text) {
                        let msg_body = parsed.get("message").unwrap_or(&parsed);
                        let match_id = msg_body.get("id").and_then(Value::as_str);

                        if match_id == Some(&req_id) {
                            if let Some(err) = msg_body.get("error").filter(|e| !e.is_null()) {
                                return Err(err
                                    .as_str()
                                    .unwrap_or("Figma operation failed")
                                    .to_string());
                            }
                            if let Some(res) = msg_body.get("result") {
                                return Ok(res.clone());
                            }
                            return Ok(msg_body.clone());
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    return Err("Figma WebSocket connection closed unexpectedly".to_string());
                }
                Err(e) => return Err(format!("Figma WebSocket communication error: {}", e)),
                _ => {}
            }
        }
        Err("Figma WebSocket connection closed before response".to_string())
    };

    timeout(response_timeout, response_future)
        .await
        .map_err(|_| "Timed out waiting for response from Figma plugin. Verify the Figma document is open and active.".to_string())?
}

#[cfg(feature = "native")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stdin = BufReader::new(tokio::io::stdin());
    let mut stdout = tokio::io::stdout();
    let mut lines = stdin.lines();

    let state = Arc::new(Mutex::new(FigmaBridgeState::new()));

    while let Ok(Some(line)) = lines.next_line().await {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let req: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                let err_res = json!({
                    "jsonrpc": "2.0",
                    "id": Value::Null,
                    "error": { "code": -32700, "message": format!("Invalid JSON: {}", e) }
                });
                stdout
                    .write_all(format!("{}\n", err_res).as_bytes())
                    .await?;
                stdout.flush().await?;
                continue;
            }
        };

        let id = req.get("id").cloned();
        let method = req.get("method").and_then(Value::as_str).unwrap_or("");

        let response = match method {
            "initialize" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": { "tools": {}, "resources": {}, "prompts": {} },
                    "serverInfo": { "name": env!("CARGO_PKG_NAME"), "version": env!("CARGO_PKG_VERSION") }
                }
            }),
            "notifications/initialized" => continue,
            "tools/list" | "mcp_list_tools" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": { "tools": definitions::tool_definitions() }
            }),
            "resources/list" | "mcp_list_resources" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": { "resources": definitions::resource_definitions() }
            }),
            "resources/read" | "mcp_read_resource" => {
                let uri = req
                    .get("params")
                    .and_then(|p| p.get("uri"))
                    .and_then(Value::as_str)
                    .unwrap_or("");
                match operations::read_resource(uri) {
                    Ok(res) => json!({ "jsonrpc": "2.0", "id": id, "result": res }),
                    Err(err) => {
                        json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32602, "message": err } })
                    }
                }
            }
            "prompts/list" | "mcp_list_prompts" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": { "prompts": definitions::prompt_definitions() }
            }),
            "prompts/get" | "mcp_get_prompt" => {
                let name = req
                    .get("params")
                    .and_then(|p| p.get("name"))
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let args = req
                    .get("params")
                    .and_then(|p| p.get("arguments"))
                    .cloned()
                    .unwrap_or(json!({}));
                match operations::get_prompt(name, &args) {
                    Ok(res) => json!({ "jsonrpc": "2.0", "id": id, "result": res }),
                    Err(err) => {
                        json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32602, "message": err } })
                    }
                }
            }
            "tools/call" | "call_tool" => {
                let params = req.get("params").cloned().unwrap_or(json!({}));
                let tool_name = params.get("name").and_then(Value::as_str).unwrap_or("");
                let tool_args = params.get("arguments").cloned().unwrap_or(json!({}));

                match execute_native_tool(tool_name, tool_args, state.clone()).await {
                    Ok(val) => json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "content": [{ "type": "text", "text": serde_json::to_string(&val).unwrap_or_default() }],
                            "isError": false
                        }
                    }),
                    Err(err) => json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "content": [{ "type": "text", "text": err }],
                            "isError": true
                        }
                    }),
                }
            }
            unknown => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32601, "message": format!("Method not found: {}", unknown) }
            }),
        };

        stdout
            .write_all(format!("{}\n", response).as_bytes())
            .await?;
        stdout.flush().await?;
    }

    Ok(())
}

#[cfg(not(feature = "native"))]
fn main() {
    eprintln!("Compile with --features native to execute rune-figma-native");
}
