// plugins/rune-ssh/src/operations.rs
#[cfg(target_arch = "wasm32")]
use crate::types::{CmdExecRequest, CmdExecResponse};
#[cfg(target_arch = "wasm32")]
use rune_pdk::ToolCallRequest;
#[cfg(target_arch = "wasm32")]
use serde_json::{Value, json};

#[cfg(target_arch = "wasm32")]
#[extism_pdk::host_fn("extism:host/user")]
extern "ExtismHost" {
    fn host_cmd_exec(input: String) -> String;
}

#[cfg(target_arch = "wasm32")]
pub fn execute_tool(request: ToolCallRequest) -> Result<Value, String> {
    let payload_str =
        serde_json::to_string(&request).map_err(|e| format!("Serialization error: {}", e))?;

    let cmd_req = CmdExecRequest {
        program: "rune-ssh-native".to_string(),
        args: vec!["--exec".to_string(), payload_str],
        cwd: None,
    };

    let raw_req = serde_json::to_string(&cmd_req).map_err(|e| e.to_string())?;
    let raw_resp =
        unsafe { host_cmd_exec(raw_req) }.map_err(|e| format!("Host execution failed: {:?}", e))?;

    let resp: CmdExecResponse = serde_json::from_str(&raw_resp)
        .map_err(|e| format!("Failed to parse host response: {}", e))?;

    if !resp.success && resp.stdout.trim().is_empty() {
        return Err(if !resp.stderr.is_empty() {
            resp.stderr
        } else {
            "rune-ssh-native exited with failure".to_string()
        });
    }

    let parsed_val: Value = serde_json::from_str(&resp.stdout).map_err(|e| {
        format!(
            "Failed to parse output JSON: {} (stdout: {})",
            e, resp.stdout
        )
    })?;

    if let Some(err) = parsed_val.get("error").and_then(Value::as_str) {
        return Err(err.to_string());
    }

    Ok(parsed_val)
}

#[cfg(not(target_arch = "wasm32"))]
pub mod native {
    use rune_pdk::ToolCallRequest;
    use serde_json::{Value, json};
    use ssh2::Session;
    use std::env;
    use std::fs::File;
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::path::Path;

    fn resolve_param(args: &Value, arg_key: &str, env_key: &str) -> Result<String, String> {
        if let Some(val) = args
            .get(arg_key)
            .and_then(Value::as_str)
            .filter(|s| !s.trim().is_empty())
        {
            return Ok(val.to_string());
        }

        env::var(env_key)
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                format!(
                    "Missing '{}' argument and environment variable '{}' is not set.",
                    arg_key, env_key
                )
            })
    }

    fn connect_ssh(host: &str, user: &str, key_path: &str) -> Result<Session, String> {
        let host_port = if host.contains(':') {
            host.to_string()
        } else {
            format!("{}:22", host)
        };
        let tcp = TcpStream::connect(&host_port)
            .map_err(|e| format!("Failed to connect to {}: {}", host_port, e))?;
        let mut sess = Session::new().map_err(|e| e.to_string())?;
        sess.set_tcp_stream(tcp);
        sess.handshake()
            .map_err(|e| format!("SSH Handshake failed: {}", e))?;

        sess.userauth_pubkey_file(user, None, Path::new(key_path), None)
            .map_err(|e| format!("Authentication failed: {}", e))?;

        Ok(sess)
    }

    pub fn execute_tool(request: ToolCallRequest) -> Result<Value, String> {
        let args = &request.arguments;

        let host = resolve_param(args, "host", "SSH_HOST")?;
        let user = resolve_param(args, "username", "SSH_USER")?;
        let key = resolve_param(args, "privateKeyPath", "SSH_KEY_PATH")?;

        match request.name.as_str() {
            "ssh_run_command" => {
                let cmd = args
                    .get("command")
                    .and_then(Value::as_str)
                    .ok_or("Missing 'command'")?;
                let sess = connect_ssh(&host, &user, &key)?;
                let mut channel = sess.channel_session().map_err(|e| e.to_string())?;

                channel
                    .exec(cmd)
                    .map_err(|e| format!("Exec failed: {}", e))?;
                let mut output = String::new();
                channel
                    .read_to_string(&mut output)
                    .map_err(|e| e.to_string())?;
                channel.wait_close().map_err(|e| e.to_string())?;

                Ok(json!({
                    "status": "success",
                    "host": host,
                    "exit_status": channel.exit_status().unwrap_or(1),
                    "output": output
                }))
            }
            "sftp_upload" => {
                let local = args
                    .get("localPath")
                    .and_then(Value::as_str)
                    .ok_or("Missing 'localPath'")?;
                let remote = args
                    .get("remotePath")
                    .and_then(Value::as_str)
                    .ok_or("Missing 'remotePath'")?;

                let sess = connect_ssh(&host, &user, &key)?;
                let sftp = sess.sftp().map_err(|e| e.to_string())?;

                let mut local_file = File::open(local).map_err(|e| e.to_string())?;
                let mut remote_file = sftp.create(Path::new(remote)).map_err(|e| e.to_string())?;

                let mut buffer = Vec::new();
                local_file
                    .read_to_end(&mut buffer)
                    .map_err(|e| e.to_string())?;
                remote_file.write_all(&buffer).map_err(|e| e.to_string())?;

                Ok(json!({
                    "status": "success",
                    "host": host,
                    "message": format!("Uploaded to {}", remote)
                }))
            }
            "sftp_download" => {
                let remote = args
                    .get("remotePath")
                    .and_then(Value::as_str)
                    .ok_or("Missing 'remotePath'")?;
                let local = args
                    .get("localPath")
                    .and_then(Value::as_str)
                    .ok_or("Missing 'localPath'")?;

                let sess = connect_ssh(&host, &user, &key)?;
                let sftp = sess.sftp().map_err(|e| e.to_string())?;

                let mut remote_file = sftp.open(Path::new(remote)).map_err(|e| e.to_string())?;
                let mut local_file = File::create(local).map_err(|e| e.to_string())?;

                let mut buffer = Vec::new();
                remote_file
                    .read_to_end(&mut buffer)
                    .map_err(|e| e.to_string())?;
                local_file.write_all(&buffer).map_err(|e| e.to_string())?;

                Ok(json!({
                    "status": "success",
                    "host": host,
                    "message": format!("Downloaded to {}", local)
                }))
            }
            unknown => Err(format!("Unknown tool: {}", unknown)),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use native::execute_tool;
