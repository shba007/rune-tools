use rune_pdk::ToolDefinition;
use serde_json::json;

pub fn tool_definitions() -> Vec<ToolDefinition> {
    vec![ToolDefinition {
        name: "sequential_thinking".to_string(),
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
    }]
}
