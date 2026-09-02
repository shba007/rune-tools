// plugins/rune-filesystem/src/definitions.rs
use rune_pdk::ToolDefinition;
use serde_json::json;

pub fn tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "read_text_file".to_string(),
            description: "Read a text file as line-based pages. Use 'head'/'tail' for quick previews, or 'lineOffset'/'lineLimit' to page through large files.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The file path to read. Can be relative to the allowed directory (e.g. 'notes.txt') or an absolute path within it."
                    },
                    "head": { "type": "number", "description": "If provided, returns only the first N lines of the file" },
                    "tail": { "type": "number", "description": "If provided, returns only the last N lines of the file" },
                    "lineOffset": { "type": "number", "description": "0-based line number to start reading from" },
                    "lineLimit": { "type": "number", "description": "Max lines to return in this page (default 2000)" }
                },
                "required": ["path"]
            }),
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
        },
        ToolDefinition {
            name: "list_directory".to_string(),
            description: "Get a detailed listing of all files and directories in a specified path.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The directory path to inspect (defaults to '.' for the root allowed directory, or a relative/absolute path within it)"
                    }
                },
                "required": ["path"]
            }),
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
        },
        ToolDefinition {
            name: "list_allowed_directories".to_string(),
            description: "Returns the list of directories that this server is allowed to access. Paths in other tool calls can be relative to these roots or passed directly.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
    ]
}
