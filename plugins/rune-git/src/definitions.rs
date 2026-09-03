use rune_pdk::ToolDefinition;
use serde_json::json;

pub fn tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "git_status".to_string(),
            description: "Shows the working tree status using host git.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "repo_path": { "type": "string", "description": "Path to the repository (default: current directory or REPO_PATH)" }
                }
            }),
        },
        ToolDefinition {
            name: "git_diff_unstaged".to_string(),
            description: "Shows changes in the working tree not yet staged.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "repo_path": { "type": "string", "description": "Path to the repository" }
                }
            }),
        },
        ToolDefinition {
            name: "git_diff_staged".to_string(),
            description: "Shows changes that are staged for commit.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "repo_path": { "type": "string", "description": "Path to the repository" }
                }
            }),
        },
        ToolDefinition {
            name: "git_diff".to_string(),
            description: "Shows changes between branches or commits.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "repo_path": { "type": "string", "description": "Path to the repository" },
                    "target": { "type": "string", "description": "Target commit/branch (default: HEAD)" }
                }
            }),
        },
        ToolDefinition {
            name: "git_commit".to_string(),
            description: "Records changes to the repository with a commit message.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "repo_path": { "type": "string", "description": "Path to the repository" },
                    "message": { "type": "string", "description": "The commit message" }
                },
                "required": ["message"]
            }),
        },
        ToolDefinition {
            name: "git_add".to_string(),
            description: "Adds file contents to the staging area.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "repo_path": { "type": "string", "description": "Path to the repository" },
                    "files": { "type": "array", "items": { "type": "string" }, "description": "List of files to add (default: all)" }
                }
            }),
        },
        ToolDefinition {
            name: "git_reset".to_string(),
            description: "Resets current HEAD to the specified state.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "repo_path": { "type": "string", "description": "Path to the repository" }
                }
            }),
        },
        ToolDefinition {
            name: "git_log".to_string(),
            description: "Shows commit logs from the current branch.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "repo_path": { "type": "string", "description": "Path to the repository" },
                    "max_count": { "type": "integer", "description": "Max number of commits to return (default: 10)" }
                }
            }),
        },
        ToolDefinition {
            name: "git_create_branch".to_string(),
            description: "Creates a new branch from current HEAD.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "repo_path": { "type": "string", "description": "Path to the repository" },
                    "branch_name": { "type": "string", "description": "Name of the new branch" }
                },
                "required": ["branch_name"]
            }),
        },
        ToolDefinition {
            name: "git_checkout".to_string(),
            description: "Switches branches in the repository.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "repo_path": { "type": "string", "description": "Path to the repository" },
                    "branch_name": { "type": "string", "description": "Branch to switch to" }
                },
                "required": ["branch_name"]
            }),
        },
        ToolDefinition {
            name: "git_show".to_string(),
            description: "Shows commit metadata and patch details.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "repo_path": { "type": "string", "description": "Path to the repository" },
                    "revision": { "type": "string", "description": "Commit SHA or ref (default: HEAD)" }
                }
            }),
        },
        ToolDefinition {
            name: "git_init".to_string(),
            description: "Initializes a new empty Git repository.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "repo_path": { "type": "string", "description": "Directory path to initialize" }
                }
            }),
        },
        ToolDefinition {
            name: "validate_commit_message".to_string(),
            description: "Validates a commit message against character length constraints and Conventional Commits formatting rules.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "message": {
                        "type": "string",
                        "description": "The full commit message (subject and optional body) to validate"
                    },
                    "max_subject_length": {
                        "type": "integer",
                        "description": "Maximum allowed character count for the subject line (default: 72)"
                    },
                    "conventional": {
                        "type": "boolean",
                        "description": "Whether to enforce Conventional Commits format (e.g. feat:, fix:) (default: false)"
                    }
                },
                "required": ["message"]
            }),
        },
    ]
}
