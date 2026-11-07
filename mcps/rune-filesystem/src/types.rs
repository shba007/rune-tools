use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextEdit {
    #[serde(rename = "oldText", alias = "old_text")]
    pub old_text: String,
    #[serde(rename = "newText", alias = "new_text")]
    pub new_text: String,
}
