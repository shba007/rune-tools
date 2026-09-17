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
        ToolDefinition {
            name: "compare_images".to_string(),
            description: "Compares two images pixel-by-pixel and generates a visual difference image showing mismatches. Supports multiple algorithms (RMS for exact match, MSSIM for structural similarity, perceptual for human-perceived differences).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "image1_path": {
                        "type": "string",
                        "description": "Path to the first image file (PNG, JPEG, GIF, SVG or WebP)"
                    },
                    "image2_path": {
                        "type": "string",
                        "description": "Path to the second image file (PNG, JPEG, GIF, SVG or WebP)"
                    },
                    "output_path": {
                        "type": "string",
                        "description": "Path for the difference image output. Defaults to './diff.png'."
                    },
                    "algorithm": {
                        "type": "string",
                        "description": "Comparison algorithm: 'rms' (pixel-by-pixel exact match), 'mssim' (structural similarity), 'perceptual' (human-perceived differences). Defaults to 'rms'."
                    },
                    "threshold": {
                        "type": "number",
                        "description": "Threshold for considering pixels different (0.0-1.0). Lower = more strict. Defaults to 0.0 (exact match)."
                    }
                },
                "required": ["image1_path", "image2_path"]
            }),
        },
    ]
}
