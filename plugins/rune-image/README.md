### `rune-image`

* **Description:** An image gallery and social album extraction MCP server powered by gallery-dl. Supports inspecting and downloading multi-image collections from Reddit, Instagram, Imgur, Pixiv, and other supported platforms with automatic domain-matched cookie handling, browser session cookie loading, and proxy routing.

* **Tool Definitions:** `inspect_image_gallery`, `download_image_collection`

* **MCP Configuration:**

```json
{
  "mcpServers": {
    "rune-image": {
      "command": "rune",
      "args": [
        "run",
        "rune-image"
      ],
      "env": {
        "COOKIES_DIR": "./test-dir/cookies",
        "OUTPUT_DIR": "./test-dir/images",
        "ALLOWED_DIR": "./test-dir"
      }
    }
  }
}
```

**Environment Variables:**

* `COOKIES_DIR`: Directory containing Netscape-formatted cookie files (e.g., `reddit.txt`, `instagram.txt`, `cookies.txt`). Cookie files are automatically matched against target domains to bypass authentication gates.
* `OUTPUT_DIR`: Default destination directory where downloaded images are saved (default: `./images`).
* `ALLOWED_DIR`: Root boundary directory enforced for sandbox isolation; output paths attempting to write outside this boundary are rejected (default: `.`).

#### Use Case IMG-01: Inspect a Reddit Image Post

* **Category:** Happy Path / Inspection
* **Prompt:** "Inspect the image gallery at 'https://www.reddit.com/r/pics/comments/abc123/' and tell me what images are available."
* **Expected Tool(s):** `inspect_image_gallery`

#### Use Case IMG-02: Download a Full Instagram Album

* **Category:** Happy Path / Download
* **Prompt:** "Download all images from the Instagram post 'https://www.instagram.com/p/abc123/' into './test-dir/images/instagram'."
* **Expected Tool(s):** `download_image_collection`

#### Use Case IMG-03: Partial Range Download with Cookie File

* **Category:** Granular Options / Range + Auth
* **Prompt:** "Download only images 1 through 10 from the Pixiv gallery 'https://www.pixiv.net/en/artworks/12345678' using cookies from './test-dir/cookies/pixiv.txt'."
* **Expected Tool(s):** `download_image_collection`

#### Use Case IMG-04: Browser Session Cookie Authentication

* **Category:** Authentication / Session Handling
* **Prompt:** "Inspect the private Instagram profile at 'https://www.instagram.com/someuser/' using cookies loaded directly from Chrome."
* **Expected Tool(s):** `inspect_image_gallery`

#### Use Case IMG-05: Proxy-Routed Gallery Download

* **Category:** Granular Options / Proxy
* **Prompt:** "Download the Imgur album at 'https://imgur.com/a/xyz123' routing traffic through proxy 'http://127.0.0.1:8080'."
* **Expected Tool(s):** `download_image_collection`

#### Use Case IMG-06: Unsupported URL Error Handling

* **Category:** Edge Case / Error Handling
* **Prompt:** "Inspect the image gallery at 'https://example.com/not-a-gallery'."
* **Expected Tool(s):** `inspect_image_gallery`
