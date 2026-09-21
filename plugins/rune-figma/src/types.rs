use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RgbaColor {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    #[serde(default = "default_alpha")]
    pub a: f64,
}

fn default_alpha() -> f64 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnotationProperty {
    pub r#type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnotationItem {
    #[serde(rename = "nodeId")]
    pub node_id: String,
    #[serde(rename = "labelMarkdown")]
    pub label_markdown: String,
    #[serde(rename = "categoryId")]
    pub category_id: Option<String>,
    #[serde(rename = "annotationId")]
    pub annotation_id: Option<String>,
    pub properties: Option<Vec<AnnotationProperty>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextReplacementItem {
    #[serde(rename = "nodeId")]
    pub node_id: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionItem {
    #[serde(rename = "startNodeId")]
    pub start_node_id: String,
    #[serde(rename = "endNodeId")]
    pub end_node_id: String,
    pub text: Option<String>,
}

/// WebSocket Relay Message Envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayEnvelope {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub msg_type: String,
    pub channel: Option<String>,
    pub message: Option<serde_json::Value>,
    pub sender: Option<String>,
}

/// Sidecar IPC request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidecarIpcRequest {
    pub id: String,
    pub command: String,
    pub params: serde_json::Value,
    pub channel: Option<String>,
}
