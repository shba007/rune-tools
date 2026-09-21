use rune_pdk::ToolDefinition;
use serde_json::json;

pub fn tool_definitions() -> Vec<ToolDefinition> {
    vec![
        // Session Management Tools
        ToolDefinition {
            name: "browser_session_start".to_string(),
            description: "Starts a persistent browser session with the specified configuration."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "engine": { "type": "string", "enum": ["agent-browser", "cdp"], "default": "agent-browser", "description": "Browser engine to use" },
                    "browser_type": { "type": "string", "enum": ["chrome", "brave", "edge", "auto"], "default": "auto", "description": "Browser to launch" },
                    "headed": { "type": "boolean", "default": false, "description": "Launch visible browser window" },
                    "output_dir": { "type": "string", "description": "Directory for artifacts (screenshots, PDFs)" }
                },
                "required": []
            }),
        },
        ToolDefinition {
            name: "browser_session_stop".to_string(),
            description: "Stops a persistent browser session and cleans up resources.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "session_id": { "type": "string", "description": "Session ID to stop (omits default session if omitted)" }
                },
                "required": []
            }),
        },
        ToolDefinition {
            name: "browser_session_list".to_string(),
            description: "Lists all active browser sessions.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
        // Navigation & Interaction Tools
        ToolDefinition {
            name: "browser_navigate".to_string(),
            description: "Opens a URL in a managed browser session or active browser window."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "Target webpage URL to open" },
                    "session_id": { "type": "string", "description": "Session ID to use (creates new if omitted)" },
                    "engine": { "type": "string", "enum": ["agent-browser", "cdp"], "default": "agent-browser", "description": "Browser engine" },
                    "headed": { "type": "boolean", "default": false, "description": "Launch visible browser" }
                },
                "required": ["url"]
            }),
        },
        ToolDefinition {
            name: "browser_type".to_string(),
            description: "Types text into a form input identified by element ref or CSS selector."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "target": { "type": "string", "description": "Element ref (e.g. '@e1') or CSS selector" },
                    "value": { "type": "string", "description": "Text value to type" },
                    "session_id": { "type": "string", "description": "Session ID to use" },
                    "clear": { "type": "boolean", "default": false, "description": "Clear input before typing" }
                },
                "required": ["target", "value"]
            }),
        },
        ToolDefinition {
            name: "browser_click".to_string(),
            description:
                "Clicks an element using an accessibility ref, semantic role, or CSS selector."
                    .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "target": { "type": "string", "description": "Element ref (e.g. '@e2'), selector, or semantic role" },
                    "session_id": { "type": "string", "description": "Session ID to use" },
                    "double_click": { "type": "boolean", "default": false, "description": "Perform double click" },
                    "new_tab": { "type": "boolean", "default": false, "description": "Open in new background tab" }
                },
                "required": ["target"]
            }),
        },
        ToolDefinition {
            name: "browser_fill".to_string(),
            description: "Types text into a form input or textarea.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "target": { "type": "string", "description": "Element ref or CSS selector" },
                    "value": { "type": "string", "description": "Text value to type" },
                    "session_id": { "type": "string", "description": "Session ID to use" },
                    "press_enter": { "type": "boolean", "default": false, "description": "Press Enter after typing" }
                },
                "required": ["target", "value"]
            }),
        },
        ToolDefinition {
            name: "browser_select_option".to_string(),
            description: "Selects an option in a dropdown element.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "target": { "type": "string", "description": "Element ref or CSS selector for the select element" },
                    "value": { "type": "string", "description": "Option value to select" },
                    "label": { "type": "string", "description": "Option label text to select (alternative to value)" },
                    "session_id": { "type": "string", "description": "Session ID to use" }
                },
                "required": ["target"]
            }),
        },
        ToolDefinition {
            name: "browser_hover".to_string(),
            description: "Hovers mouse over an element.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "target": { "type": "string", "description": "Element ref or CSS selector" },
                    "session_id": { "type": "string", "description": "Session ID to use" }
                },
                "required": ["target"]
            }),
        },
        // Content & Execution Tools
        ToolDefinition {
            name: "browser_execute_script".to_string(),
            description: "Evaluates arbitrary JavaScript in the context of the active page."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "script": { "type": "string", "description": "JavaScript expression or function body to execute" },
                    "session_id": { "type": "string", "description": "Session ID to use" }
                },
                "required": ["script"]
            }),
        },
        ToolDefinition {
            name: "browser_screenshot".to_string(),
            description: "Captures a viewport or full-page screenshot to the output directory."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "filename": { "type": "string", "description": "Custom filename (e.g. 'page.png')" },
                    "session_id": { "type": "string", "description": "Session ID to use" },
                    "full_page": { "type": "boolean", "default": false, "description": "Capture full scrollable page" },
                    "output_dir": { "type": "string", "description": "Custom destination folder" }
                }
            }),
        },
        ToolDefinition {
            name: "browser_pdf".to_string(),
            description: "Exports the current page as a PDF file to the output directory."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "filename": { "type": "string", "description": "Custom filename (e.g. 'page.pdf')" },
                    "session_id": { "type": "string", "description": "Session ID to use" },
                    "landscape": { "type": "boolean", "default": false, "description": "Use landscape orientation" },
                    "output_dir": { "type": "string", "description": "Custom destination folder" }
                }
            }),
        },
        // Diagnostics Tools
        ToolDefinition {
            name: "browser_console_messages".to_string(),
            description: "Retrieves console log messages from the browser.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "session_id": { "type": "string", "description": "Session ID to use" },
                    "level": { "type": "string", "enum": ["debug", "info", "warn", "error"], "description": "Filter by log level" },
                    "limit": { "type": "number", "description": "Maximum number of messages to return" }
                }
            }),
        },
        ToolDefinition {
            name: "browser_network_requests".to_string(),
            description: "Retrieves network request information from the browser.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "session_id": { "type": "string", "description": "Session ID to use" },
                    "url": { "type": "string", "description": "Filter by URL pattern" },
                    "method": { "type": "string", "enum": ["GET", "POST", "PUT", "DELETE", "HEAD", "OPTIONS"], "description": "Filter by HTTP method" }
                }
            }),
        },
        // CDP Tools (for connecting to existing browser)
        ToolDefinition {
            name: "browser_cdp_connect".to_string(),
            description: "Connects to an existing Chrome DevTools session.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "target_host": { "type": "string", "description": "Target host (e.g. 'localhost:9222')" },
                    "web_socket_url": { "type": "string", "description": "WebSocket URL for connection" }
                },
                "required": []
            }),
        },
        ToolDefinition {
            name: "browser_cdp_request".to_string(),
            description: "Sends a CDP request to the connected browser.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "session_id": { "type": "string", "description": "Session ID to use" },
                    "method": { "type": "string", "description": "CDP method name (e.g. 'Page.enable')" },
                    "params": { "type": "object", "description": "CDP request parameters" }
                },
                "required": ["session_id", "method"]
            }),
        },
        ToolDefinition {
            name: "browser_cdp_disconnect".to_string(),
            description: "Disconnects from a CDP session.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "session_id": { "type": "string", "description": "Session ID to disconnect" }
                }
            }),
        },
    ]
}
