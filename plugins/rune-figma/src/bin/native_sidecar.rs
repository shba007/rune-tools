use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::{Mutex, mpsc, oneshot};
use tokio_tungstenite::connect_async;

type PendingMap = Arc<Mutex<HashMap<String, oneshot::Sender<Result<Value, String>>>>>;

fn get_env_var(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn parse_cli_arg(flag: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    for i in 1..args.len() {
        if args[i] == flag && i + 1 < args.len() {
            let val = args[i + 1].trim();
            if !val.is_empty() {
                return Some(val.to_string());
            }
        }
    }
    None
}

fn load_dotenv_fallback() {
    if let Ok(content) = std::fs::read_to_string(".env") {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = trimmed.split_once('=') {
                let key = k.trim();
                let val = v.trim().trim_matches('"').trim_matches('\'');
                // Only populate if not already set by rune-kit
                if std::env::var(key).is_err() && !val.is_empty() {
                    unsafe {
                        std::env::set_var(key, val);
                    }
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Load local fallback only for variables not already passed by rune-kit
    load_dotenv_fallback();

    let ws_url = parse_cli_arg("--ws-url")
        .or_else(|| get_env_var("FIGMA_WS_URL"))
        .or_else(|| get_env_var("FIGMA_WS_PORT").map(|p| format!("ws://localhost:{}", p)))
        .unwrap_or_else(|| "ws://localhost:3055".to_string());

    let default_channel = parse_cli_arg("--channel")
        .or_else(|| get_env_var("FIGMA_CHANNEL"))
        .unwrap_or_default();

    let current_ws_url = Arc::new(Mutex::new(ws_url));
    let current_channel = Arc::new(Mutex::new(default_channel));
    let pending: PendingMap = Arc::new(Mutex::new(HashMap::new()));
    let (ws_out_tx, mut ws_out_rx) = mpsc::unbounded_channel::<String>();

    // 3. Resilient background WebSocket connection manager
    let bg_ws_url = current_ws_url.clone();
    let bg_channel = current_channel.clone();
    let bg_pending = pending.clone();

    tokio::spawn(async move {
        loop {
            let target_url = bg_ws_url.lock().await.clone();
            let active_channel = bg_channel.lock().await.clone();

            eprintln!("[INFO] Connecting to Figma WebSocket at {}...", target_url);
            match connect_async(&target_url).await {
                Ok((ws_stream, _)) => {
                    eprintln!("[INFO] Connected to Figma WebSocket on {}", target_url);
                    let (mut ws_sink, mut ws_stream) = ws_stream.split();

                    // Join current channel on connect
                    let join_payload = json!({
                        "type": "join",
                        "channel": active_channel,
                        "id": "init_join"
                    });

                    if let Err(e) = ws_sink
                        .send(tokio_tungstenite::tungstenite::Message::Text(
                            join_payload.to_string(),
                        ))
                        .await
                    {
                        eprintln!("[WARN] Failed to send join payload: {}", e);
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        continue;
                    }
                    eprintln!("[INFO] Joined channel: '{}'", active_channel);

                    // Process bi-directional communication
                    loop {
                        tokio::select! {
                            Some(outgoing) = ws_out_rx.recv() => {
                                if let Err(e) = ws_sink.send(tokio_tungstenite::tungstenite::Message::Text(outgoing)).await {
                                    eprintln!("[WARN] Failed to send to WebSocket: {}", e);
                                    break;
                                }
                            }
                            incoming = ws_stream.next() => {
                                match incoming {
                                    Some(Ok(tokio_tungstenite::tungstenite::Message::Text(txt))) => {
                                        if let Ok(val) = serde_json::from_str::<Value>(&txt) {
                                            let inner = val.get("message").unwrap_or(&val);
                                            let req_id = inner.get("id")
                                                .or_else(|| val.get("id"))
                                                .and_then(|v| v.as_str());

                                            if let Some(id) = req_id {
                                                let mut p = bg_pending.lock().await;
                                                if let Some(sender) = p.remove(id) {
                                                    if let Some(err) = inner.get("error").and_then(|e| e.as_str()) {
                                                        let _ = sender.send(Err(err.to_string()));
                                                    } else if let Some(res) = inner.get("result") {
                                                        let _ = sender.send(Ok(res.clone()));
                                                    } else {
                                                        let _ = sender.send(Ok(inner.clone()));
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    Some(Ok(tokio_tungstenite::tungstenite::Message::Close(_))) | None => {
                                        eprintln!("[WARN] WebSocket disconnected. Reconnecting...");
                                        break;
                                    }
                                    Some(Err(e)) => {
                                        eprintln!("[WARN] WebSocket error: {}. Reconnecting...", e);
                                        break;
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!(
                        "[WARN] Could not connect to {}: {}. Retrying in 2 seconds...",
                        target_url, e
                    );
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
        }
    });

    // 4. Stdio MCP protocol loop
    let stdin = BufReader::new(tokio::io::stdin());
    let mut lines = stdin.lines();

    while let Ok(Some(line)) = lines.next_line().await {
        if line.trim().is_empty() {
            continue;
        }

        let req: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let method = req.get("method").and_then(|v| v.as_str()).unwrap_or("");
        let id = req.get("id");

        match method {
            "initialize" => {
                // Support configuration passed dynamically in initialize params
                if let Some(params) = req.get("params")
                    && let Some(opts) = params
                        .get("initializationOptions")
                        .or_else(|| params.get("env"))
                {
                    if let Some(url) = opts
                        .get("FIGMA_WS_URL")
                        .or_else(|| opts.get("ws_url"))
                        .or_else(|| opts.get("wsUrl"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                    {
                        let mut u = current_ws_url.lock().await;
                        *u = url.to_string();
                    }

                    if let Some(ch) = opts
                        .get("FIGMA_CHANNEL")
                        .or_else(|| opts.get("channel"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                    {
                        let mut c = current_channel.lock().await;
                        *c = ch.to_string();
                    }
                }

                let res = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "protocolVersion": "2024-11-05",
                        "capabilities": { "tools": {} },
                        "serverInfo": {
                            "name": env!("CARGO_PKG_NAME"),
                            "version": env!("CARGO_PKG_VERSION")
                        }
                    }
                });
                println!("{}", res);
            }
            "notifications/initialized" => {}

            "tools/list" => {
                let tools = rune_figma::definitions::tool_definitions();
                let res = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": { "tools": tools }
                });
                println!("{}", res);
            }

            "tools/call" => {
                let params = req.get("params").cloned().unwrap_or(json!({}));
                let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

                let result = dispatch_command_to_figma(
                    tool_name,
                    arguments,
                    &ws_out_tx,
                    &pending,
                    &current_channel,
                    &current_ws_url,
                )
                .await;

                let mcp_res = match result {
                    Ok(val) => json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "content": [{
                                "type": "text",
                                "text": serde_json::to_string(&val).unwrap_or_default()
                            }]
                        }
                    }),
                    Err(err) => json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "isError": true,
                        "result": {
                            "content": [{ "type": "text", "text": err }]
                        }
                    }),
                };
                println!("{}", mcp_res);
            }

            _ => {
                if let Some(id) = id {
                    let err_res = json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": { "code": -32601, "message": "Method not found" }
                    });
                    println!("{}", err_res);
                }
            }
        }
    }

    Ok(())
}

async fn dispatch_command_to_figma(
    command: &str,
    params: Value,
    ws_tx: &mpsc::UnboundedSender<String>,
    pending: &PendingMap,
    current_channel: &Arc<Mutex<String>>,
    current_ws_url: &Arc<Mutex<String>>,
) -> Result<Value, String> {
    if command == "join_channel"
        && let Some(new_ch) = params.get("channel").and_then(|v| v.as_str())
    {
        let mut ch = current_channel.lock().await;
        *ch = new_ch.to_string();
        let join_msg = json!({
            "type": "join",
            "channel": new_ch,
            "id": "switch_channel"
        });
        let _ = ws_tx.send(join_msg.to_string());
        return Ok(json!({ "message": format!("Joined channel '{}'", new_ch) }));
    }

    let req_id = uuid_or_random();
    let (resp_tx, resp_rx) = oneshot::channel();
    let channel = current_channel.lock().await.clone();

    pending.lock().await.insert(req_id.clone(), resp_tx);

    let figma_envelope = json!({
        "type": "message",
        "channel": channel,
        "message": {
            "id": req_id,
            "command": command,
            "params": params
        }
    });

    if ws_tx.send(figma_envelope.to_string()).is_err() {
        return Err("Internal error: WebSocket dispatcher queue is closed.".to_string());
    }

    match tokio::time::timeout(Duration::from_secs(30), resp_rx).await {
        Ok(Ok(Ok(val))) => {
            let filtered = if command == "get_node_info" || command == "read_my_design" {
                rune_figma::operations::filter_figma_node(&val).unwrap_or(val)
            } else {
                val
            };
            Ok(filtered)
        }
        Ok(Ok(Err(err))) => Err(err),
        _ => {
            pending.lock().await.remove(&req_id);
            let active_url = current_ws_url.lock().await.clone();
            Err(format!(
                "Command '{}' timed out after 30s. Ensure cursor-talk-to-figma-socket is running at '{}' and TalkToFigma in Figma is open in channel '{}'.",
                command, active_url, channel
            ))
        }
    }
}

fn uuid_or_random() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("req_{:x}", nanos)
}
