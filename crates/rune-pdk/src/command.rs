use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmdExecRequest {
    pub program: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub cwd: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmdExecResponse {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[cfg(target_arch = "wasm32")]
#[extism_pdk::host_fn("extism:host/user")]
extern "ExtismHost" {
    fn host_cmd_exec(input: String) -> String;
}

#[cfg(target_arch = "wasm32")]
fn run_binary_raw(req: &CmdExecRequest) -> Result<CmdExecResponse, String> {
    let raw_req = serde_json::to_string(req).map_err(|e| e.to_string())?;
    let raw_resp =
        unsafe { host_cmd_exec(raw_req) }.map_err(|e| format!("Host execution failed: {:?}", e))?;
    serde_json::from_str(&raw_resp).map_err(|e| format!("Failed to parse host response: {}", e))
}

#[cfg(not(target_arch = "wasm32"))]
fn run_binary_raw(req: &CmdExecRequest) -> Result<CmdExecResponse, String> {
    let mut cmd = std::process::Command::new(&req.program);
    cmd.args(&req.args);
    if let Some(ref cwd) = req.cwd {
        cmd.current_dir(cwd);
    }
    match cmd.output() {
        Ok(output) => Ok(CmdExecResponse {
            success: output.status.success(),
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        }),
        Err(e) => Ok(CmdExecResponse {
            success: false,
            exit_code: None,
            stdout: String::new(),
            stderr: format!("Failed to spawn {}: {}", req.program, e),
        }),
    }
}

pub fn run_binary(program: &str, args: &[&str], cwd: Option<&str>) -> Result<String, String> {
    let executable = crate::resolve_binary_executable(program)?;

    let req = CmdExecRequest {
        program: executable,
        args: args.iter().map(|s| s.to_string()).collect(),
        cwd: cwd.map(|c| c.to_string()),
    };

    let resp = run_binary_raw(&req)?;

    if resp.success {
        Ok(resp.stdout.trim().to_string())
    } else {
        let err = if !resp.stderr.trim().is_empty() {
            resp.stderr.trim()
        } else if !resp.stdout.trim().is_empty() {
            resp.stdout.trim()
        } else {
            "Process exited with non-zero exit code"
        };
        Err(format!("Error executing '{}': {}", program, err))
    }
}
