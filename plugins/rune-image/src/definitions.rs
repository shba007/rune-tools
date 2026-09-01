use rune_pdk::ToolDefinition;
use serde_json::json;

pub fn tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "inspect_image_gallery".to_string(),
            description: "Scans albums, artist profiles, or social posts (Reddit, Instagram, Imgur, Pixiv) with cookie dir auto-matching.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "Gallery or post URL"
                    },
                    "cookiesDir": {
                        "type": "string",
                        "description": "Directory containing site cookie files (e.g. reddit.txt, cookies.txt)"
                    },
                    "cookiesFromBrowser": {
                        "type": "string",
                        "description": "Browser to load session cookies from"
                    },
                    "cookiesFile": {
                        "type": "string",
                        "description": "Explicit path to a cookies.txt file"
                    },
                    "proxy": {
                        "type": "string",
                        "description": "HTTP/HTTPS/SOCKS proxy URL"
                    }
                },
                "required": ["url"]
            }),
        },
        ToolDefinition {
            name: "download_image_collection".to_string(),
            description: "Downloads image galleries, multi-image posts, or artist boards with cookie dir auto-matching.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "Gallery URL"
                    },
                    "filterRange": {
                        "type": "string",
                        "description": "Range of items to download (e.g. '1-10')"
                    },
                    "outputDirectory": {
                        "type": "string",
                        "description": "Target directory for saved images"
                    },
                    "cookiesDir": {
                        "type": "string",
                        "description": "Directory containing site cookie files (e.g. reddit.txt, cookies.txt)"
                    },
                    "cookiesFromBrowser": {
                        "type": "string",
                        "description": "Browser to load session cookies from"
                    },
                    "cookiesFile": {
                        "type": "string",
                        "description": "Explicit path to a cookies.txt file"
                    },
                    "proxy": {
                        "type": "string",
                        "description": "HTTP/HTTPS/SOCKS proxy URL"
                    }
                },
                "required": ["url"]
            }),
        },
    ]
}
