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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserSessionConfig {
    pub engine: String, // "agent-browser" | "cdp"
    pub session_id: Option<String>,
    pub browser_type: Option<String>, // "brave" | "chrome" | "edge" | "auto"
    pub cdp_port: u16,
    pub headed: bool,
    pub output_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub engine: String,
    pub url: String,
    pub created_at: String,
    pub last_accessed: String,
    pub status: String, // "active" | "stopped"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleMessage {
    pub level: String,
    pub text: String,
    pub timestamp: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRequest {
    pub url: String,
    pub method: String,
    pub status: u16,
    pub status_text: String,
    pub timing: f64,
    pub from_cache: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CDPRequest {
    pub method: String,
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CDPResponse {
    pub result: serde_json::Value,
    pub session_id: String,
}

#[derive(Debug, Clone)]
pub struct BrowserBinaryConfig {
    pub binary_name: String,
    pub download_url: String,
    pub platforms: Vec<PlatformConfig>,
}

#[derive(Debug, Clone)]
pub struct PlatformConfig {
    pub os: String,
    pub binary_name: String,
    pub download_url: String,
}

impl Default for BrowserBinaryConfig {
    fn default() -> Self {
        Self {
            binary_name: "agent-browser".to_string(),
            download_url: "https://github.com/vercel-labs/agent-browser/releases/latest/download/agent-browser".to_string(),
            platforms: vec![
                PlatformConfig {
                    os: "windows".to_string(),
                    binary_name: "agent-browser.exe".to_string(),
                    download_url: "https://github.com/vercel-labs/agent-browser/releases/latest/download/agent-browser.exe".to_string(),
                },
                PlatformConfig {
                    os: "linux".to_string(),
                    binary_name: "agent-browser".to_string(),
                    download_url: "https://github.com/vercel-labs/agent-browser/releases/latest/download/agent-browser-linux".to_string(),
                },
                PlatformConfig {
                    os: "macos".to_string(),
                    binary_name: "agent-browser".to_string(),
                    download_url: "https://github.com/vercel-labs/agent-browser/releases/latest/download/agent-browser-macos".to_string(),
                },
            ],
        }
    }
}
