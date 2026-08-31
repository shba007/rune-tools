use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextEdit {
    #[serde(rename = "oldText")]
    pub old_text: String,
    #[serde(rename = "newText")]
    pub new_text: String,
}
