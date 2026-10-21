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
pub struct PrinterInfo {
    pub name: String,
    pub system_name: String,
    pub driver_name: String,
    pub uri: String,
    pub location: String,
    pub is_default: bool,
    pub is_shared: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrintDocumentResult {
    pub status: String,
    pub engine: String,
    pub file: String,
    pub copies: usize,
    pub os_printer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScanDocumentResult {
    pub status: String,
    pub saved_path: String,
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScannerCapabilitiesResult {
    pub printer_ip: String,
    pub capabilities_xml: String,
}
