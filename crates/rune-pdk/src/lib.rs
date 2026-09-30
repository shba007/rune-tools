pub mod testing;

use serde::{Deserialize, Serialize};
use serde_json::Value;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

#[cfg(target_arch = "wasm32")]
pub use extism_pdk::*;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema", alias = "input_schema")]
    pub input_schema: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequest {
    pub name: String,
    pub arguments: Value,
}

#[derive(Serialize, Deserialize)]
pub struct Page<T> {
    pub items: T,
    pub cursor: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceDefinition {
    pub uri: String,
    pub name: String,
    pub description: String,
    #[serde(
        rename = "mimeType",
        alias = "mime_type",
        skip_serializing_if = "Option::is_none"
    )]
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceReadRequest {
    pub uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptArgument {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptDefinition {
    pub name: String,
    pub description: String,
    #[serde(rename = "arguments", alias = "args", default)]
    pub arguments: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptGetRequest {
    pub name: String,
    #[serde(default)]
    pub arguments: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BinaryProvisionStatus {
    Installed,
    Provisioning,
    Missing,
    UnsupportedPlatform,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryStatus {
    pub name: String,
    pub status: BinaryProvisionStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

pub fn get_config(key: &str) -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        extism_pdk::config::get(key).ok().flatten()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let upper = key.to_ascii_uppercase();
        let lower = key.to_ascii_lowercase();
        std::env::var(&upper)
            .or_else(|_| std::env::var(&lower))
            .or_else(|_| std::env::var(key))
            .ok()
    }
}

pub fn is_contract_test_mode() -> bool {
    std::env::var("RUNE_CONTRACT_TEST").as_deref() == Ok("1")
}

pub fn resolve_binary_executable(program: &str) -> Result<String, String> {
    #[cfg(target_arch = "wasm32")]
    {
        Ok(program.to_string())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        if is_contract_test_mode() {
            return Ok(program.to_string());
        }

        let upper_key = format!("{}_PATH", program.replace('-', "_").to_ascii_uppercase());
        if let Ok(explicit) = std::env::var(&upper_key) {
            let p = PathBuf::from(&explicit);
            if p.exists() {
                return Ok(p.to_string_lossy().to_string());
            }
        }

        let lower_key = format!("{}_path", program.replace('-', "_").to_ascii_lowercase());
        if let Ok(explicit) = std::env::var(&lower_key) {
            let p = PathBuf::from(&explicit);
            if p.exists() {
                return Ok(p.to_string_lossy().to_string());
            }
        }

        let exe_name = if cfg!(windows) && !program.ends_with(".exe") {
            format!("{}.exe", program)
        } else {
            program.to_string()
        };

        let local_dirs = [
            std::env::var("ALLOWED_DIR").ok(),
            std::env::var("OUTPUT_DIR").ok(),
            Some(".".to_string()),
        ];

        for base in local_dirs.into_iter().flatten() {
            let candidate_bin = PathBuf::from(&base).join("bin").join(&exe_name);
            if candidate_bin.is_file() {
                return Ok(candidate_bin.to_string_lossy().to_string());
            }
            let candidate_direct = PathBuf::from(&base).join(&exe_name);
            if candidate_direct.is_file() {
                return Ok(candidate_direct.to_string_lossy().to_string());
            }
        }

        if let Ok(path_var) = std::env::var("PATH") {
            for dir in std::env::split_paths(&path_var) {
                let candidate = dir.join(&exe_name);
                if candidate.is_file() {
                    return Ok(candidate.to_string_lossy().to_string());
                }
            }
        }

        if std::process::Command::new(program)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Ok(program.to_string());
        }

        Err(format!(
            "Required external binary '{}' not found. It was neither provisioned by rune-kit (checked environment variable '{}') nor found on the system PATH.",
            program, upper_key
        ))
    }
}
