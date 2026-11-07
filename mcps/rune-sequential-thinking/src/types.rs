use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ThoughtData {
    pub thought: String,
    #[serde(rename = "thoughtNumber", alias = "thought_number")]
    pub thought_number: i64,
    #[serde(rename = "totalThoughts", alias = "total_thoughts")]
    pub total_thoughts: i64,
    #[serde(rename = "nextThoughtNeeded", alias = "next_thought_needed")]
    pub next_thought_needed: bool,
    #[serde(rename = "isRevision", alias = "is_revision", default)]
    pub is_revision: Option<bool>,
    #[serde(rename = "revisesThought", alias = "revises_thought", default)]
    pub revises_thought: Option<i64>,
    #[serde(rename = "branchFromThought", alias = "branch_from_thought", default)]
    pub branch_from_thought: Option<i64>,
    #[serde(rename = "branchId", alias = "branch_id", default)]
    pub branch_id: Option<String>,
    #[serde(rename = "needsMoreThoughts", alias = "needs_more_thoughts", default)]
    pub needs_more_thoughts: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ThoughtResponse {
    #[serde(rename = "thoughtNumber")]
    pub thought_number: i64,
    #[serde(rename = "totalThoughts")]
    pub total_thoughts: i64,
    #[serde(rename = "nextThoughtNeeded")]
    pub next_thought_needed: bool,
    pub branches: Vec<String>,
    #[serde(rename = "thoughtHistoryLength")]
    pub thought_history_length: usize,
    #[serde(rename = "activeBranchesCount")]
    pub active_branches_count: usize,
    #[serde(rename = "isRevision")]
    pub is_revision: bool,
    #[serde(rename = "revisesThought")]
    pub revises_thought: Option<i64>,
    #[serde(rename = "branchId")]
    pub branch_id: Option<String>,
    #[serde(rename = "branchFromThought")]
    pub branch_from_thought: Option<i64>,
}
