### `rune-audio`

* **Description:** An audio processing and media ingestion MCP server providing stream extraction, format transcoding (MP3, FLAC, WAV, M4A, Opus) with automatic domain-matched cookie handling, and music track/album/playlist retrieval with ID3 metadata embedding and synced LRC lyrics via spotdl.

* **Tool Definitions:** `extract_audio_track`, `download_music_track`

* **MCP Configuration:**

```json
{
  "mcpServers": {
    "rune-audio": {
      "command": "rune",
      "args": [
        "run",
        "rune-audio"
      ],
      "env": {
        "COOKIES_DIR": "./test-dir/cookies",
        "OUTPUT_DIR": "./test-dir/audio",
        "ALLOWED_DIR": "./test-dir"
      }
    }
  }
}
```

**Environment Variables:**

* `COOKIES_DIR`: Directory containing Netscape-formatted cookie files (e.g., youtube.txt, spotify.txt, cookies.txt). Cookie files are automatically matched against target domains to bypass authentication gates and bot-detection challenges.
* `OUTPUT_DIR`: Default destination path where converted audio files, music tracks, and synced lyric files (.lrc) are saved (default: ./audio).
* `ALLOWED_DIR`: Root boundary directory enforced for sandbox isolation. Output paths attempting to write outside this boundary are prohibited (default: .).

#### Use Case AUD-01: Default Audio Stream Extraction to MP3

* **Category:** Happy Path
* **Prompt:** "Extract the audio from 'https://www.youtube.com/watch?v=dQw4w9WgXcQ' as a high-quality MP3 and save it to the configured audio directory."
* **Expected Tool(s):** `extract_audio_track`

#### Use Case AUD-02: Lossless FLAC Extraction at Maximum Quality

* **Category:** Granular Options / Quality
* **Prompt:** "Extract the audio from the live performance stream at 'https://www.youtube.com/watch?v=live_stream_id' in lossless FLAC format with quality set to 0."
* **Expected Tool(s):** `extract_audio_track`

#### Use Case AUD-03: Opus Format Conversion for Shorts with Explicit Cookie File

* **Category:** Granular Options / Auth
* **Prompt:** "Convert the audio from 'https://www.youtube.com/shorts/EqvgsORpbOU' into Opus format and explicitly load cookies from './test-dir/cookies/youtube.txt'."
* **Expected Tool(s):** `extract_audio_track`

#### Use Case AUD-04: Audio Extraction with Active Browser Session Cookies

* **Category:** Authentication / Session Handling
* **Prompt:** "Extract the audio track from the member-exclusive video 'https://www.youtube.com/watch?v=members_only' into M4A format using session cookies extracted directly from Chrome."
* **Expected Tool(s):** `extract_audio_track`

#### Use Case AUD-05: Proxy-Routed Audio Extraction

* **Category:** Network Routing / Proxy
* **Prompt:** "Extract the audio from 'https://www.youtube.com/watch?v=geo_restricted' to MP3 routing network traffic through proxy 'http://127.0.0.1:8080'."
* **Expected Tool(s):** `extract_audio_track`

#### Use Case AUD-06: Custom Output Path with Automatic Directory Creation

* **Category:** Storage Management / Path Routing
* **Prompt:** "Extract the WAV audio stream from 'https://www.youtube.com/watch?v=sound_effects' and save it into './test-dir/audio/sfx/wav'."
* **Expected Tool(s):** `extract_audio_track`

#### Use Case AUD-07: Compressed Low-Bitrate Voice Extraction

* **Category:** Bandwidth Optimization / Transcoding
* **Prompt:** "Extract the audio from the 2-hour interview at 'https://www.youtube.com/watch?v=podcast_ep12' as an MP3 with compression quality set to 7 to minimize file size."
* **Expected Tool(s):** `extract_audio_track`

#### Use Case AUD-08: Spotify Single Track Ingestion with Synced LRC Lyrics

* **Category:** Happy Path / Music & Lyrics
* **Prompt:** "Download the track 'https://open.spotify.com/track/4cOdK2wGLETKBW3PvgPWqT' with synced LRC lyrics and save it to the audio downloads directory."
* **Expected Tool(s):** `download_music_track`

#### Use Case AUD-09: Apple Music Track Download without Lyrics

* **Category:** Happy Path / Platform Ingestion
* **Prompt:** "Download the song from Apple Music at 'https://music.apple.com/us/album/song-name/123456789?i=987654321' into './test-dir/audio/apple' without downloading lyrics."
* **Expected Tool(s):** `download_music_track`

#### Use Case AUD-10: Spotify Album / Playlist Batch Ingestion

* **Category:** Batch Ingestion / Collections
* **Prompt:** "Download all tracks from the Spotify album 'https://open.spotify.com/album/4LH4d3cOWNNXdsqFd42wQn' including synced lyrics for every track into './test-dir/audio/albums'."
* **Expected Tool(s):** `download_music_track`

#### Use Case AUD-11: Invalid Streaming URL Error Handling

* **Category:** Edge Case / Error Handling
* **Prompt:** "Extract audio from 'https://invalid-streaming-site.fake/stream.mp4' into MP3."
* **Expected Tool(s):** `extract_audio_track`

#### Use Case AUD-12: Target Output Directory Traversal Rejection

* **Category:** Edge Case / Security Boundary
* **Prompt:** "Download the Spotify song 'https://open.spotify.com/track/4cOdK2wGLETKBW3PvgPWqT' and force the output directory to '../../../../etc/music'."
* **Expected Tool(s):** `download_music_track`
