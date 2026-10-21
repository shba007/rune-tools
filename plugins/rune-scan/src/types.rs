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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScanDocumentResult {
    pub status: String,
    pub saved_path: String,
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScannerCapabilitiesResult {
    pub scanner_ip: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub make_and_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manufacturer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serial_number: Option<String>,
    pub supported_sources: Vec<String>,
    pub supported_resolutions: Vec<u32>,
    pub supported_color_modes: Vec<String>,
    pub supported_formats: Vec<String>,
    pub supported_intents: Vec<String>,
    pub platen_max_width: u32,
    pub platen_max_height: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adf_max_width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adf_max_height: Option<u32>,
    pub capabilities_xml: String,
}
