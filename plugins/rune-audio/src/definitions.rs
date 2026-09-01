use rune_pdk::ToolDefinition;
use serde_json::json;

pub fn tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "extract_audio_track".to_string(),
            description: "Extracts and converts audio from video links into MP3, FLAC, WAV, M4A, or Opus with cookie dir auto-matching.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "Video/Audio stream URL"
                    },
                    "audioFormat": {
                        "type": "string",
                        "enum": ["mp3", "flac", "wav", "m4a", "opus"],
                        "default": "mp3",
                        "description": "Target audio format conversion"
                    },
                    "audioQuality": {
                        "type": "integer",
                        "enum": [0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
                        "default": 0,
                        "description": "Audio quality compression (0 is highest quality, 9 is lowest)"
                    },
                    "outputDirectory": {
                        "type": "string",
                        "description": "Target output directory for converted audio"
                    },
                    "cookiesDir": {
                        "type": "string",
                        "description": "Directory containing site cookie files (e.g. youtube.txt, reddit.txt, cookies.txt)"
                    },
                    "cookiesFromBrowser": {
                        "type": "string",
                        "description": "Browser to load session cookies from (e.g. 'chrome', 'firefox', 'edge', 'brave')"
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
            name: "download_music_track".to_string(),
            description: "Fetches tracks, albums, or playlists from Spotify/Apple Music with ID3 metadata via spotdl.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "Spotify or Apple Music URL"
                    },
                    "includeLyrics": {
                        "type": "boolean",
                        "default": false,
                        "description": "Whether to generate and download synced LRC lyrics"
                    },
                    "outputDirectory": {
                        "type": "string",
                        "description": "Target directory for downloaded music files"
                    }
                },
                "required": ["url"]
            }),
        },
    ]
}
