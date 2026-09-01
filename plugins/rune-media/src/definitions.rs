use rune_pdk::ToolDefinition;
use serde_json::json;

pub fn tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "verify_downloader_environment".to_string(),
            description: "Probes host system for yt-dlp, gallery-dl, ffmpeg, aria2c, streamlink, and spotdl.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
        ToolDefinition {
            name: "inspect_video_metadata".to_string(),
            description: "Extracts video/audio title, formats, duration, upload date, age limit, and thumbnail with cookie dir auto-matching.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "URL of the video stream"
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
                    "playerClient": {
                        "type": "string",
                        "description": "YouTube player client spoofing (e.g. 'android', 'web', 'ios', 'tv')"
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
            name: "download_video_stream".to_string(),
            description: "Fetches video streams with resolution capping, subtitle embedding, and cookie dir auto-matching.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "URL of the video stream"
                    },
                    "maxResolution": {
                        "type": "string",
                        "enum": ["480p", "720p", "1080p", "1440p", "2160p"],
                        "default": "1080p",
                        "description": "Maximum video resolution to download"
                    },
                    "writeSubtitles": {
                        "type": "boolean",
                        "default": false,
                        "description": "Whether to download and embed subtitles"
                    },
                    "subtitlesLang": {
                        "type": "string",
                        "default": "en",
                        "description": "Subtitle language code (e.g. 'en', 'es')"
                    },
                    "outputDirectory": {
                        "type": "string",
                        "description": "Target download directory"
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
                    "playerClient": {
                        "type": "string",
                        "description": "YouTube player client spoofing (e.g. 'android', 'web', 'ios', 'tv')"
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
            name: "download_video_playlist".to_string(),
            description: "Downloads video playlists or channels in batches with range and cookie dir auto-matching.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "URL of the playlist"
                    },
                    "startIndex": {
                        "type": "integer",
                        "default": 1,
                        "description": "Index of the first playlist item to download"
                    },
                    "endIndex": {
                        "type": "integer",
                        "description": "Index of the last playlist item to download"
                    },
                    "maxVideos": {
                        "type": "integer",
                        "description": "Maximum number of playlist videos to download"
                    },
                    "outputDirectory": {
                        "type": "string",
                        "description": "Target download directory"
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
                    "playerClient": {
                        "type": "string",
                        "description": "YouTube player client spoofing (e.g. 'android', 'web', 'ios', 'tv')"
                    }
                },
                "required": ["url"]
            }),
        },
        ToolDefinition {
            name: "record_live_stream".to_string(),
            description: "Captures live broadcasts (Twitch, Kick, YouTube Live) for a set duration via streamlink.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "Live stream URL"
                    },
                    "durationMinutes": {
                        "type": "integer",
                        "default": 5,
                        "description": "Duration in minutes to record the stream"
                    },
                    "quality": {
                        "type": "string",
                        "enum": ["best", "1080p", "720p", "480p", "audio_only"],
                        "default": "best",
                        "description": "Stream quality preset to record"
                    },
                    "outputDirectory": {
                        "type": "string",
                        "description": "Target output directory for recorded files"
                    }
                },
                "required": ["url"]
            }),
        },
        ToolDefinition {
            name: "extract_audio_track".to_string(),
            description: "Extracts and converts audio from video links into MP3, FLAC, M4A, or Opus with cookie dir auto-matching.".to_string(),
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
            name: "download_direct_file".to_string(),
            description: "Accelerated multi-connection segmented download for direct HTTP/HTTPS/FTP URLs via aria2c.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "Direct file URL"
                    },
                    "connectionsPerServer": {
                        "type": "integer",
                        "default": 8,
                        "description": "Number of parallel connections per server"
                    },
                    "outputFilename": {
                        "type": "string",
                        "description": "Custom destination filename"
                    },
                    "outputDirectory": {
                        "type": "string",
                        "description": "Target download directory"
                    }
                },
                "required": ["url"]
            }),
        },
        ToolDefinition {
            name: "download_torrent_magnet".to_string(),
            description: "Fetches files from .torrent files or magnet: URIs with rate limiting via aria2c.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "uri": {
                        "type": "string",
                        "description": "Magnet URI or path to .torrent file"
                    },
                    "maxDownloadSpeed": {
                        "type": "string",
                        "description": "Max speed limit (e.g. '5M', '500K')"
                    },
                    "outputDirectory": {
                        "type": "string",
                        "description": "Target download directory"
                    }
                },
                "required": ["uri"]
            }),
        },
        ToolDefinition {
            name: "trim_media_clip".to_string(),
            description: "Crops or trims video/audio files between timestamps with lossless stream-copying via ffmpeg.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "inputFile": {
                        "type": "string",
                        "description": "Source audio/video file path"
                    },
                    "startTime": {
                        "type": "string",
                        "description": "Start timestamp (HH:MM:SS or SS)"
                    },
                    "endTime": {
                        "type": "string",
                        "description": "End timestamp (HH:MM:SS or SS)"
                    },
                    "lossless": {
                        "type": "boolean",
                        "default": true,
                        "description": "Whether to perform lossless stream copying without re-encoding"
                    },
                    "outputFile": {
                        "type": "string",
                        "description": "Destination file path"
                    }
                },
                "required": ["inputFile", "startTime", "endTime"]
            }),
        },
    ]
}
