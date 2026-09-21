pub mod testing;

use serde::{Deserialize, Serialize};
use serde_json::Value;

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

// crates/rune-pdk — shared by every plugin, never redefined locally
#[derive(Serialize, Deserialize)]
pub struct Page<T> {
    pub items: T,
    pub cursor: Option<String>, // opaque continuation token
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

/// Status of host-provisioned binary dependencies (§13.5).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BinaryProvisionStatus {
    Installed,
    Provisioning,
    Missing,
    UnsupportedPlatform,
}

/// Diagnostic report entry for host-managed external executables.
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

/// Resolves configuration parameters uniformly across WASM and Native environments (§2.5).
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
