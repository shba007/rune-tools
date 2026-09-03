use crate::types::{ThoughtData, ThoughtResponse};
use rune_pdk::ToolCallRequest;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Default)]
pub struct ThinkingSession {
    pub thought_history: Vec<ThoughtData>,
    pub branches: HashMap<String, Vec<ThoughtData>>,
}

static SESSION: Mutex<Option<ThinkingSession>> = Mutex::new(None);

pub fn with_session<F, R>(f: F) -> R
where
    F: FnOnce(&mut ThinkingSession) -> R,
{
    let mut guard = SESSION.lock().unwrap();
    if guard.is_none() {
        *guard = Some(ThinkingSession::default());
    }
    f(guard.as_mut().unwrap())
}

pub fn reset_session() {
    let mut guard = SESSION.lock().unwrap();
    *guard = Some(ThinkingSession::default());
}

pub fn execute_tool(request: ToolCallRequest) -> Result<Value, String> {
    match request.name.as_str() {
        "sequential-thinking" | "sequential_thinking" => {
            let thought_data: ThoughtData = serde_json::from_value(request.arguments)
                .map_err(|e| format!("Invalid thought parameters: {}", e))?;

            if thought_data.thought.trim().is_empty() {
                return Err("Thought text cannot be empty".to_string());
            }

            if thought_data.thought_number < 1 {
                return Err("thoughtNumber must be >= 1".to_string());
            }

            if thought_data.total_thoughts < 1 {
                return Err("totalThoughts must be >= 1".to_string());
            }

            let response = with_session(|session| {
                let mut adjusted_total = thought_data.total_thoughts;
                if thought_data.thought_number > adjusted_total {
                    adjusted_total = thought_data.thought_number;
                }

                if let Some(branch_id) = &thought_data.branch_id {
                    session
                        .branches
                        .entry(branch_id.clone())
                        .or_default()
                        .push(thought_data.clone());
                } else {
                    session.thought_history.push(thought_data.clone());
                }

                let history_count = session.thought_history.len();
                let branch_count = session.branches.len();

                ThoughtResponse {
                    thought_number: thought_data.thought_number,
                    total_thoughts: adjusted_total,
                    next_thought_needed: thought_data.next_thought_needed,
                    branches: session.branches.keys().cloned().collect(),
                    thought_history_length: history_count,
                    active_branches_count: branch_count,
                    is_revision: thought_data.is_revision.unwrap_or(false),
                    revises_thought: thought_data.revises_thought,
                    branch_id: thought_data.branch_id,
                    branch_from_thought: thought_data.branch_from_thought,
                }
            });

            Ok(json!(response))
        }

        unknown => Err(format!("Unknown tool: {}", unknown)),
    }
}
