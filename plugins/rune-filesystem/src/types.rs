use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextEdit {
    #[serde(alias = "old_text", alias = "oldText")]
    pub old_text: String,
    #[serde(alias = "new_text", alias = "newText")]
    pub new_text: String,
}
