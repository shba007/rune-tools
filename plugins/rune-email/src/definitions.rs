use rune_pdk::ToolDefinition;
use serde_json::json;

pub fn tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "verify_email_connection".to_string(),
            description: "Probes and verifies IMAP and SMTP authentication and TLS connectivity with the configured mail server.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "preset": { "type": "string", "enum": ["gmail", "hostinger", "outlook", "custom"], "description": "Vendor configuration preset" },
                    "email": { "type": "string", "description": "Email address" },
                    "password": { "type": "string", "description": "Account password or App Password" }
                }
            }),
        },
        ToolDefinition {
            name: "list_mailboxes".to_string(),
            description: "Lists all available mailboxes/folders (e.g. INBOX, Sent, Drafts, Trash, Spam) on the IMAP server.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
        ToolDefinition {
            name: "list_messages".to_string(),
            description: "Fetches paginated email summaries (UID, subject, sender, date, read status, attachment indicators) from a mailbox.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "mailbox": { "type": "string", "default": "INBOX", "description": "Target mailbox folder name" },
                    "limit": { "type": "integer", "default": 20, "description": "Maximum number of messages to fetch" },
                    "page": { "type": "integer", "default": 1, "description": "Page number (1-based)" },
                    "unreadOnly": { "type": "boolean", "default": false, "description": "Filter strictly for unseen messages" }
                }
            }),
        },
        ToolDefinition {
            name: "search_messages".to_string(),
            description: "Searches messages in a mailbox matching criteria such as sender, recipient, subject, body keyword, or date.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "mailbox": { "type": "string", "default": "INBOX", "description": "Target mailbox folder" },
                    "query": { "type": "string", "description": "Keyword to search across subject and body" },
                    "from": { "type": "string", "description": "Filter by sender address/name" },
                    "to": { "type": "string", "description": "Filter by recipient address" },
                    "subject": { "type": "string", "description": "Filter by subject keyword" },
                    "sinceDate": { "type": "string", "description": "Date filter in YYYY-MM-DD or DD-MMM-YYYY format" },
                    "limit": { "type": "integer", "default": 20, "description": "Maximum results to return" }
                }
            }),
        },
        ToolDefinition {
            name: "read_message".to_string(),
            description: "Fetches and parses a full email by UID into Markdown/plain text, headers, recipient lists, and attachment metadata.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "uid": { "type": "integer", "description": "Unique identifier (UID) of the message" },
                    "mailbox": { "type": "string", "default": "INBOX", "description": "Mailbox folder containing the message" },
                    "markAsRead": { "type": "boolean", "default": true, "description": "Whether to mark the message as read" }
                },
                "required": ["uid"]
            }),
        },
        ToolDefinition {
            name: "download_attachment".to_string(),
            description: "Extracts an attachment from a specific email UID and saves it to the output directory.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "uid": { "type": "integer", "description": "UID of the message" },
                    "attachmentIndex": { "type": "integer", "description": "Index of the attachment in the message" },
                    "filename": { "type": "string", "description": "Filename of the attachment to extract" },
                    "mailbox": { "type": "string", "default": "INBOX", "description": "Mailbox folder containing the message" },
                    "outputDirectory": { "type": "string", "description": "Target folder on host disk to save the file" }
                },
                "required": ["uid"]
            }),
        },
        ToolDefinition {
            name: "send_email".to_string(),
            description: "Sends an email via SMTP with visual parity (automatically appends a copy to Sent mailbox if required by provider).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "to": { "type": "string", "description": "Recipient email address(es), comma-separated" },
                    "subject": { "type": "string", "description": "Subject line of the email" },
                    "bodyText": { "type": "string", "description": "Plain text content of the email" },
                    "bodyHtml": { "type": "string", "description": "Optional HTML content of the email" },
                    "cc": { "type": "string", "description": "Optional CC recipient(s), comma-separated" },
                    "bcc": { "type": "string", "description": "Optional BCC recipient(s), comma-separated" },
                    "attachmentPaths": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Optional array of host file paths to attach"
                    }
                },
                "required": ["to", "subject", "bodyText"]
            }),
        },
        ToolDefinition {
            name: "reply_email".to_string(),
            description: "Sends a threaded reply to an existing email, preserving Message-ID, In-Reply-To, and References headers for UI grouping.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "originalUid": { "type": "integer", "description": "UID of the original message to reply to" },
                    "mailbox": { "type": "string", "default": "INBOX", "description": "Mailbox containing the original message" },
                    "replyBody": { "type": "string", "description": "Body text of the reply" },
                    "replyAll": { "type": "boolean", "default": false, "description": "Whether to reply to all original recipients" }
                },
                "required": ["originalUid", "replyBody"]
            }),
        },
        ToolDefinition {
            name: "draft_email".to_string(),
            description: "Creates and saves an email draft into the Drafts mailbox via IMAP APPEND without sending.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "to": { "type": "string", "description": "Intended recipient email address" },
                    "subject": { "type": "string", "description": "Subject line" },
                    "bodyText": { "type": "string", "description": "Draft body text" }
                },
                "required": ["to", "subject", "bodyText"]
            }),
        },
        ToolDefinition {
            name: "manage_message_flags".to_string(),
            description: "Modifies message flags (mark read/unread, star/flag, unstar) on target message UIDs.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "uid": { "type": "integer", "description": "Target message UID" },
                    "mailbox": { "type": "string", "default": "INBOX", "description": "Mailbox folder" },
                    "action": { "type": "string", "enum": ["mark_read", "mark_unread", "star", "unstar"], "description": "Flag action to perform" }
                },
                "required": ["uid", "action"]
            }),
        },
        ToolDefinition {
            name: "move_message".to_string(),
            description: "Moves a message from one mailbox to another (e.g. archiving or moving to Trash).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "uid": { "type": "integer", "description": "Target message UID" },
                    "sourceMailbox": { "type": "string", "default": "INBOX", "description": "Source mailbox" },
                    "destinationMailbox": { "type": "string", "description": "Destination mailbox (e.g. Trash, Archive)" }
                },
                "required": ["uid", "destinationMailbox"]
            }),
        },
    ]
}
