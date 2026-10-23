use rune_pdk::{RiskLevel, ToolDefinition, ToolEffect, ToolRules};
use serde_json::json;

pub const MAX_FILE_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_OUTPUT_BYTES: usize = 64 * 1024;
pub const MAX_LINES: usize = 2000;
pub const MAX_ENTRIES: usize = 200;

fn approved_read() -> ToolRules {
    ToolRules::read_only().with_limits(rune_pdk::ToolLimits {
        root_param: Some("allowed_dir".into()),
        ..Default::default()
    })
}

pub fn tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "read_text_file".to_string(),
            description: "Read a UTF-8 text file inside the host-configured allowed_dir. Requires a root. Files are limited to 8 MiB; each page to 2000 lines and 64 KiB of content. Use head/tail or lineOffset/lineLimit.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The file path to read. Can be relative to the allowed directory (e.g. 'notes.txt') or an absolute path within it."
                    },
                    "head": { "type": "integer", "minimum":0,"maximum":MAX_LINES,"description": "First N lines; cannot be combined with tail" },
                    "tail": { "type": "integer", "minimum":0,"maximum":MAX_LINES,"description": "Last N lines; cannot be combined with head" },
                    "lineOffset": { "type": "integer", "minimum":0,"description": "0-based line number to start reading from" },
                    "lineLimit": { "type": "integer", "minimum":0,"maximum":MAX_LINES,"description": "Max lines to return in this page (default 2000)" }
                },
                "x-rune-read-limits":{"maxInputBytes":MAX_FILE_BYTES,"maxContentBytes":MAX_OUTPUT_BYTES,"maxLines":MAX_LINES},
                "required": ["path"]
            }),
            rules: Some(approved_read()),
        },
        ToolDefinition {
            name: "read_media_file".to_string(),
            description: "Read an image or media file. Images are returned directly as visual image blocks for multimodal vision models up to 20MB. Audio and binary return as paged resources.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the media/binary file. Can be relative to the allowed directory or an absolute path within it."
                    },
                    "offset": { "type": "number", "description": "Byte offset to start reading from (default 0)" },
                    "length": { "type": "number", "description": "Bytes to read (defaults to full file for images up to 20MB, capped at 512KB for non-images)" }
                },
                "required": ["path"]
            }),
            rules: Some(ToolRules::read_only_fs("allowed_dir")),
        },
        ToolDefinition {
            name: "read_multiple_files".to_string(),
            description: "Read the contents of multiple files simultaneously.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "paths": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Array of file paths to read (relative to the allowed directory or absolute paths within it)"
                    }
                },
                "required": ["paths"]
            }),
            rules: Some(ToolRules::read_only_fs("allowed_dir")),
        },
        ToolDefinition {
            name: "write_file".to_string(),
            description: "Create a new file or completely overwrite an existing file with new content.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The file path to write to. Can be relative to the allowed directory or an absolute path within it."
                    },
                    "content": { "type": "string", "description": "The text content to write" }
                },
                "required": ["path", "content"]
            }),
            rules: Some(ToolRules::write_fs("allowed_dir", RiskLevel::Medium)),
        },
        ToolDefinition {
            name: "edit_file".to_string(),
            description: "Make line-based edits to a text file. Each edit replaces exact line sequences with new content.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The file path to edit. Can be relative to the allowed directory or an absolute path within it."
                    },
                    "edits": {
                        "type": "array",
                        "description": "List of text replacement edits to apply",
                        "items": {
                            "type": "object",
                            "properties": {
                                "oldText": { "type": "string", "description": "Text to search for - must match exactly" },
                                "newText": { "type": "string", "description": "Text to replace with" }
                            },
                            "required": ["oldText", "newText"]
                        }
                    },
                    "dryRun": { "type": "boolean", "default": false, "description": "Preview changes using git-style diff format without saving" }
                },
                "required": ["path", "edits"]
            }),
            rules: Some(ToolRules::write_fs("allowed_dir", RiskLevel::Medium)),
        },
        ToolDefinition {
            name: "create_directory".to_string(),
            description: "Create a new directory or ensure a directory exists, including parent directories.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The directory path to create. Can be relative to the allowed directory or an absolute path within it."
                    }
                },
                "required": ["path"]
            }),
            rules: Some(ToolRules::write_fs("allowed_dir", RiskLevel::Low)),
        },
        ToolDefinition {
            name: "list_directory".to_string(),
            description: "List a directory inside the host-configured allowed_dir without following entry links. Requires a root. Returns up to 200 entries and 64 KiB of content with truncation metadata.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Required directory path: use '.' for the root, or a relative/absolute path within it"
                    },
                    "maxEntries":{"type":"integer","minimum":0,"maximum":MAX_ENTRIES,"description":"Maximum entries to return (default and maximum 200)"}
                },
                "x-rune-read-limits":{"maxContentBytes":MAX_OUTPUT_BYTES,"maxEntries":MAX_ENTRIES},
                "required": ["path"]
            }),
            rules: Some(approved_read()),
        },
        ToolDefinition {
            name: "list_directory_with_sizes".to_string(),
            description: "Get a detailed listing of all files and directories in a specified path with sizes and sort options.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The directory path to inspect (defaults to '.' for the root allowed directory, or a relative/absolute path within it)"
                    },
                    "sortBy": { "type": "string", "enum": ["name", "size"], "default": "name", "description": "Sort entries by name or size" }
                },
                "required": ["path"]
            }),
            rules: Some(ToolRules::read_only_fs("allowed_dir")),
        },
        ToolDefinition {
            name: "directory_tree".to_string(),
            description: "Get a recursive tree view of files and directories as a JSON structure.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The directory root path (defaults to '.' for the root allowed directory, or a relative/absolute path within it)"
                    },
                    "excludePatterns": { "type": "array", "items": { "type": "string" }, "default": [], "description": "Glob patterns to exclude" }
                },
                "required": ["path"]
            }),
            rules: Some(ToolRules::read_only_fs("allowed_dir")),
        },
        ToolDefinition {
            name: "move_file".to_string(),
            description: "Move or rename files and directories safely.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "source": {
                        "type": "string",
                        "description": "Source path. Can be relative to the allowed directory or an absolute path within it."
                    },
                    "destination": {
                        "type": "string",
                        "description": "Destination path. Can be relative to the allowed directory or an absolute path within it."
                    }
                },
                "required": ["source", "destination"]
            }),
            rules: Some(ToolRules::mutate_fs(
                "allowed_dir",
                RiskLevel::Medium,
                vec![ToolEffect::Write, ToolEffect::Delete],
            )),
        },
        ToolDefinition {
            name: "search_files".to_string(),
            description: "Recursively search for files and directories matching a glob pattern.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Root path to search within (defaults to '.' for the root allowed directory, or a relative/absolute path within it)"
                    },
                    "pattern": { "type": "string", "description": "Glob pattern (e.g. '*.rs', '**/*.json')" },
                    "excludePatterns": { "type": "array", "items": { "type": "string" }, "default": [], "description": "Patterns to ignore" }
                },
                "required": ["path", "pattern"]
            }),
            rules: Some(ToolRules::read_only_fs("allowed_dir")),
        },
        ToolDefinition {
            name: "get_file_info".to_string(),
            description: "Retrieve comprehensive metadata about a file or directory.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to inspect. Can be relative to the allowed directory or an absolute path within it."
                    }
                },
                "required": ["path"]
            }),
            rules: Some(ToolRules::read_only_fs("allowed_dir")),
        },
        ToolDefinition {
            name: "list_allowed_directories".to_string(),
            description: "Returns the list of directories that this server is allowed to access. Paths in other tool calls can be relative to these roots or passed directly.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
            rules: Some(ToolRules::read_only_fs("allowed_dir")),
        },
    ]
}
