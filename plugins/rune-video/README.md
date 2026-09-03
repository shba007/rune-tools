### `rune-video`

* **Description:** A video streaming, playlist extraction, broadcast recording, and media trimming MCP server powered by yt-dlp, streamlink, and ffmpeg (all required on the system PATH). Supports metadata inspection with available-format enumeration, resolution-capped downloads with optional subtitle embedding, batched playlist ingestion with index ranges, timed live-broadcast capture (Twitch, Kick, YouTube Live), and lossless or re-encoded clip trimming — all with automatic domain-matched cookie handling, browser session cookie loading, YouTube player-client spoofing, and proxy routing.

* **Tool Definitions:** `inspect_video_metadata`, `download_video_stream`, `download_video_playlist`, `record_live_stream`, `trim_media_clip`

* **MCP Configuration:**

```json
{
  "mcpServers": {
    "rune-video": {
      "command": "rune",
      "args": [
        "run",
        "rune-video"
      ],
      "env": {
        "COOKIES_DIR": "./test-dir/cookies",
        "OUTPUT_DIR": "./test-dir/video",
        "ALLOWED_DIR": "./test-dir"
      }
    }
  }
}
```

**Environment Variables:**

* `COOKIES_DIR`: Directory containing Netscape-formatted cookie files (e.g., `youtube.txt`, `twitch.txt`, `cookies.txt`). Cookie files are automatically matched against target domains to bypass authentication gates and bot-detection challenges.
* `OUTPUT_DIR`: Default destination directory where downloaded videos, playlist batches, and live recordings are saved (default: falls back to `ALLOWED_DIR`, then `.`).
* `ALLOWED_DIR`: Root boundary directory used to resolve relative output paths and enforced for sandbox isolation (default: `.`).

#### Use Case VID-01: Inspect Video Metadata Before Downloading

* **Category:** Happy Path / Inspection
* **Prompt:** "Inspect 'https://www.youtube.com/watch?v=dQw4w9WgXcQ' and tell me the title, duration, upload date, and available formats before I download it."
* **Expected Tool(s):** `inspect_video_metadata`

#### Use Case VID-02: Resolution-Capped Video Download

* **Category:** Happy Path / Download
* **Prompt:** "Download 'https://www.youtube.com/watch?v=abc123' capped at 720p into './test-dir/video/youtube'."
* **Expected Tool(s):** `download_video_stream`

#### Use Case VID-03: Download with Embedded Subtitles

* **Category:** Granular Options / Subtitles
* **Prompt:** "Download the talk at 'https://www.youtube.com/watch?v=lecture_01' in 1080p with English subtitles downloaded and embedded."
* **Expected Tool(s):** `download_video_stream`

#### Use Case VID-04: Members-Only Video with Cookie File and Client Spoofing

* **Category:** Authentication / Session Handling
* **Prompt:** "Download the members-only video 'https://www.youtube.com/watch?v=members_only' using cookies from './test-dir/cookies/youtube.txt' and spoofing the android player client."
* **Expected Tool(s):** `download_video_stream`

#### Use Case VID-05: Playlist Range Download in Batches

* **Category:** Happy Path / Playlist
* **Prompt:** "Download items 3 through 10 of the playlist 'https://www.youtube.com/playlist?list=PLabc123' into './test-dir/video/courses'."
* **Expected Tool(s):** `download_video_playlist`

#### Use Case VID-06: Capped Playlist Batch with Browser Cookies

* **Category:** Granular Options / Batch Limit + Auth
* **Prompt:** "Download at most 5 videos from 'https://www.youtube.com/playlist?list=PLdef456' using session cookies loaded directly from Chrome."
* **Expected Tool(s):** `download_video_playlist`

#### Use Case VID-07: Timed Twitch Stream Recording

* **Category:** Happy Path / Live Recording
* **Prompt:** "Record the live Twitch stream at 'https://www.twitch.tv/somechannel' for 10 minutes at best quality."
* **Expected Tool(s):** `record_live_stream`

#### Use Case VID-08: Audio-Only Broadcast Capture

* **Category:** Granular Options / Quality Preset
* **Prompt:** "Capture just the audio of the Kick stream 'https://kick.com/somestreamer' for 30 minutes into './test-dir/video/live'."
* **Expected Tool(s):** `record_live_stream`

#### Use Case VID-09: Lossless Clip Trim with Stream Copy

* **Category:** Happy Path / Trimming
* **Prompt:** "Trim './test-dir/video/talk.mp4' from 00:01:30 to 00:05:45 using lossless stream copying."
* **Expected Tool(s):** `trim_media_clip`

#### Use Case VID-10: Re-Encoded Trim with Custom Output Path

* **Category:** Granular Options / Custom Output
* **Prompt:** "Cut the segment between 90 and 120 seconds from './test-dir/video/podcast.mp4' into './test-dir/video/clips/clip_01.mp4' with lossless disabled."
* **Expected Tool(s):** `trim_media_clip`

#### Use Case VID-11: Unsupported URL Error Handling

* **Category:** Edge Case / Error Handling
* **Prompt:** "Download the video at 'https://invalid-streaming-site.fake/stream.mp4' into my video directory."
* **Expected Tool(s):** `download_video_stream`
