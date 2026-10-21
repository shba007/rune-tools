// plugins/rune-ssh/src/definitions.rs
use rune_pdk::{RiskLevel, ToolDefinition, ToolEffect, ToolRules};
use serde_json::json;

pub fn tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "ssh_run_command".to_string(),
            description: "Executes a shell command on a remote server via SSH. Uses SSH_HOST, SSH_USER, and SSH_KEY_PATH from environment if not explicitly provided.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The shell command string to execute on the remote machine"
                    },
                    "host": {
                        "type": "string",
                        "description": "Optional hostname or IP (defaults to SSH_HOST from environment)"
                    },
                    "username": {
                        "type": "string",
                        "description": "Optional SSH user (defaults to SSH_USER from environment)"
                    },
                    "privateKeyPath": {
                        "type": "string",
                        "description": "Optional path to private key (defaults to SSH_KEY_PATH from environment)"
                    }
                },
                "required": ["command"]
            }),
            rules: Some(ToolRules::network(
                RiskLevel::High,
                vec![ToolEffect::Write, ToolEffect::Network],
                ["*"],
            )),
        },
        ToolDefinition {
            name: "sftp_upload".to_string(),
            description: "Uploads a local file to a remote server via SFTP. Uses configured environment credentials by default.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "localPath": {
                        "type": "string",
                        "description": "Source path of the file on the local machine to upload"
                    },
                    "remotePath": {
                        "type": "string",
                        "description": "Destination file path on the remote server"
                    },
                    "host": {
                        "type": "string",
                        "description": "Optional hostname or IP (defaults to SSH_HOST)"
                    },
                    "username": {
                        "type": "string",
                        "description": "Optional SSH user (defaults to SSH_USER)"
                    },
                    "privateKeyPath": {
                        "type": "string",
                        "description": "Optional private key path (defaults to SSH_KEY_PATH)"
                    }
                },
                "required": ["localPath", "remotePath"]
            }),
            rules: Some(ToolRules::network(
                RiskLevel::Medium,
                vec![ToolEffect::Write, ToolEffect::Network],
                ["*"],
            )),
        },
        ToolDefinition {
            name: "sftp_download".to_string(),
            description: "Downloads a file from a remote server to the local machine via SFTP. Uses configured environment credentials by default.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "remotePath": {
                        "type": "string",
                        "description": "Source path of the file on the remote server to download"
                    },
                    "localPath": {
                        "type": "string",
                        "description": "Destination file path on the local machine"
                    },
                    "host": {
                        "type": "string",
                        "description": "Optional hostname or IP (defaults to SSH_HOST)"
                    },
                    "username": {
                        "type": "string",
                        "description": "Optional SSH user (defaults to SSH_USER)"
                    },
                    "privateKeyPath": {
                        "type": "string",
                        "description": "Optional private key path (defaults to SSH_KEY_PATH)"
                    }
                },
                "required": ["remotePath", "localPath"]
            }),
            rules: Some(ToolRules::network(
                RiskLevel::Low,
                vec![ToolEffect::Write, ToolEffect::Network],
                ["*"],
            )),
        },
    ]
}
