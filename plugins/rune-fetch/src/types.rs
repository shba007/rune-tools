use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchPayload {
    pub url: String,
    #[serde(
        default = "default_max_length",
        rename = "maxLength",
        alias = "max_length"
    )]
    pub max_length: usize,
    #[serde(default, rename = "startIndex", alias = "start_index")]
    pub start_index: usize,
    #[serde(default)]
    pub raw: bool,
    #[serde(default)]
    pub paginate: bool,
}

fn default_max_length() -> usize {
    50000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchResult {
    pub contents: String,
    #[serde(rename = "totalCharacters")]
    pub total_characters: usize,
    #[serde(rename = "startIndex")]
    pub start_index: usize,
    pub length: usize,
    #[serde(rename = "hasMore")]
    pub has_more: bool,
    #[serde(rename = "nextStartIndex", skip_serializing_if = "Option::is_none")]
    pub next_start_index: Option<usize>,
}
