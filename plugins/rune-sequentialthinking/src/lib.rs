// plugins/rune-sequentialthinking/src/lib.rs
use extism_pdk::*;
use rune_pdk::{ToolCallRequest, ToolDefinition};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtData {
    pub thought: String,
    #[serde(rename = "thoughtNumber")]
    pub thought_number: i64,
    #[serde(rename = "totalThoughts")]
    pub total_thoughts: i64,
    #[serde(rename = "nextThoughtNeeded")]
    pub next_thought_needed: bool,
    #[serde(rename = "isRevision", default)]
    pub is_revision: Option<bool>,
    #[serde(rename = "revisesThought", default)]
    pub revises_thought: Option<i64>,
    #[serde(rename = "branchFromThought", default)]
    pub branch_from_thought: Option<i64>,
    #[serde(rename = "branchId", default)]
    pub branch_id: Option<String>,
    #[serde(rename = "needsMoreThoughts", default)]
    pub needs_more_thoughts: Option<bool>,
}

#[derive(Default)]
struct ThinkingSession {
    thought_history: Vec<ThoughtData>,
    branches: HashMap<String, Vec<ThoughtData>>,
}

static SESSION: Mutex<Option<ThinkingSession>> = Mutex::new(None);

fn with_session<F, R>(f: F) -> R
where
    F: FnOnce(&mut ThinkingSession) -> R,
{
    let mut guard = SESSION.lock().unwrap();
    if guard.is_none() {
        *guard = Some(ThinkingSession::default());
    }
    f(guard.as_mut().unwrap())
}

fn execute_tool(request: ToolCallRequest) -> Result<Value, String> {
    match request.name.as_str() {
        "sequentialthinking" => {
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

            let result = with_session(|session| {
                // Adjust total thoughts if current thought exceeds initial estimate
                let mut adjusted_total = thought_data.total_thoughts;
                if thought_data.thought_number > adjusted_total {
                    adjusted_total = thought_data.thought_number;
                }

                // Handle branching
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

                json!({
                    "thoughtNumber": thought_data.thought_number,
                    "totalThoughts": adjusted_total,
                    "nextThoughtNeeded": thought_data.next_thought_needed,
                    "branches": session.branches.keys().cloned().collect::<Vec<String>>(),
                    "thoughtHistoryLength": history_count,
                    "activeBranchesCount": branch_count,
                    "isRevision": thought_data.is_revision.unwrap_or(false),
                    "revisesThought": thought_data.revises_thought,
                    "branchId": thought_data.branch_id,
                    "branchFromThought": thought_data.branch_from_thought
                })
            });

            Ok(result)
        }

        unknown => Err(format!("Unknown tool: {}", unknown)),
    }
}

#[plugin_fn]
pub fn mcp_info(_: ()) -> FnResult<String> {
    let info = json!({
        "name": env!("CARGO_PKG_NAME"),
        "version": env!("CARGO_PKG_VERSION"),
        "description": option_env!("CARGO_PKG_DESCRIPTION")
    });
    Ok(serde_json::to_string(&info)?)
}

#[plugin_fn]
pub fn mcp_list_tools(_: ()) -> FnResult<String> {
    let tools = vec![
        ToolDefinition {
            name: "sequentialthinking".to_string(),
            description: "A detailed tool for dynamic and reflective problem-solving through structured, step-by-step thinking processes. Facilitates adaptive reasoning, hypothesis revision, branching exploration, and progress tracking.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "thought": {
                        "type": "string",
                        "description": "The current thinking step containing analysis, hypothesis, or verification"
                    },
                    "nextThoughtNeeded": {
                        "type": "boolean",
                        "description": "Whether another thought step is needed to continue reasoning"
                    },
                    "thoughtNumber": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Current thought number in the sequence"
                    },
                    "totalThoughts": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Estimated total thoughts needed (can be dynamically adjusted)"
                    },
                    "isRevision": {
                        "type": "boolean",
                        "description": "Whether this thought revises, corrects, or replaces previous thinking"
                    },
                    "revisesThought": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Which thought number is being reconsidered or revised"
                    },
                    "branchFromThought": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Branching point thought number if exploring an alternative hypothesis"
                    },
                    "branchId": {
                        "type": "string",
                        "description": "Identifier for the alternative reasoning branch"
                    },
                    "needsMoreThoughts": {
                        "type": "boolean",
                        "description": "Explicit signal that additional thinking steps beyond totalThoughts are required"
                    }
                },
                "required": ["thought", "nextThoughtNeeded", "thoughtNumber", "totalThoughts"]
            }),
        },
    ];

    Ok(serde_json::to_string(&tools)?)
}

#[plugin_fn]
pub fn mcp_call_tool(input: String) -> FnResult<String> {
    let request: ToolCallRequest = serde_json::from_str(&input)?;
    let result = execute_tool(request);

    let output = match result {
        Ok(val) => json!({ "status": "success", "result": val }),
        Err(err) => json!({ "status": "error", "error": err }),
    };

    Ok(serde_json::to_string(&output)?)
}
