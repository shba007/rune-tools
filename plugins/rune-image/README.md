### `rune-image`

* **Description:** A MCP server for multi-platform image extraction, visual comparison, and flexible format conversion including SVG and raster files.

* **Tool Definitions:** `inspect_image_gallery`, `download_image_collection`, `compare_images`, `get_image_metadata`, `convert_image_format`

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

#### Use Case IMG-07: Compare Two Images for Visual Differences

* **Category:** Image Comparison / Visual Regression Testing
* **Prompt:** "Compare the original screenshot at '/path/to/original.png' with the edited version at '/path/to/edited.png' and show me where they differ."
* **Expected Tool(s):** `compare_images`
* **Parameters:**
  * `image1_path`: Path to the first image
  * `image2_path`: Path to the second image
  * `output_path`: Path for the difference image (optional, defaults to './diff.png')
  * `algorithm`: Comparison method - 'rms' for pixel-by-pixel, 'mssim' for structural similarity, 'perceptual' for human-perceived differences (optional, defaults to 'rms')
  * `threshold`: Tolerance for minor differences (0.0-1.0, optional, defaults to 0.0)
* **Expected Output:** JSON with match percentage, differences found, and path to the diff image showing mismatches in red/green coloring

#### Use Case IMG-08: Image Comparison with Tolerance

* **Category:** Granular Options / Threshold
* **Prompt:** "Compare two images allowing for minor compression artifacts with threshold 0.1."
* **Expected Tool(s):** `compare_images`
* **Parameters:** `image1_path`, `image2_path`, `threshold: 0.1`
* **Expected Output:** Comparison results showing matching pixels within the tolerance threshold

#### Use Case IMG-09: Compare SVG Vector Graphics

* **Category:** Image Comparison / Vector Support
* **Prompt:** "Compare two SVG vector graphics at '/path/to/icon1.svg' and '/path/to/icon2.svg' to detect visual differences."
* **Expected Tool(s):** `compare_images`
* **Parameters:**
  * `image1_path`: Path to the first SVG file (e.g., `/path/to/icon1.svg`)
  * `image2_path`: Path to the second SVG file (e.g., `/path/to/icon2.svg`)
  * `algorithm`: Comparison method - 'rms' for pixel-by-pixel (optional, defaults to 'rms')
  * `output_path`: Path for the difference image (optional, defaults to './diff.png')
* **Expected Output:** JSON with match percentage, differences found, and path to the diff image showing mismatches in red/green coloring
* **Notes:** SVG files are automatically rasterized to 800x800 pixels for comparison. If width/height attributes are specified in the SVG file or URL, those dimensions are used.

#### Use Case IMG-10: Mixed Raster and SVG Comparison

* **Category:** Image Comparison / Mixed Formats
* **Prompt:** "Compare a raster PNG screenshot at '/path/to/screenshot.png' with an SVG diagram at '/path/to/diagram.svg' to find differences."
* **Expected Tool(s):** `compare_images`
* **Parameters:** `image1_path`, `image2_path`, `algorithm: 'rms'`
* **Expected Output:** Comparison results showing where the raster and vector graphics differ

#### Use Case IMG-11: Vectorize a Photo to SVG, Iteratively, Until It Matches the Original

* **Category:** Format Conversion / Raster-to-Vector / Iterative Refinement
* **Prompt:**

  > Trace `D:\Projects\Public\rune\code-kit\temp\head.png` into an SVG at `D:\Projects\Public\rune\code-kit\temp\head.svg`, using `convert_image_format`. Don't try to guess the perfect tracing settings up front — do this as a loop of small steps:
  >
  > 1. Run `convert_image_format` once with default tracing settings to produce `head.svg`.
  > 2. Run `compare_images` with `image1_path` set to the original `head.png` and `image2_path` set to `head.svg`.
  > 3. Look at `match_percentage` and where the diff image shows red. Based on that, change exactly one or two tracing parameters (see the table below for which one) and re-run `convert_image_format` to regenerate `head.svg`.
  > 4. Run `compare_images` again on the same two paths.
  > 5. Repeat steps 3–4, each time comparing against the *same* original PNG, until `match_percentage` stops improving between iterations or reaches an exact/near-exact match. Report the final `match_percentage` and which parameters you ended on.
  >
  > Don't jump straight to a heavily-tuned config on the first pass — start from defaults and change one or two things at a time so you can tell which change actually helped.

* **Expected Tool(s):** `convert_image_format`, `compare_images` — called alternately, several times, in that order, not once each.
* **Parameters (`convert_image_format`, per iteration):**
  * `input_path`: `D:\Projects\Public\rune\code-kit\temp\head.png`
  * `output_format`: `'svg'`
  * `trace_color_mode`, `trace_hierarchical`, `trace_curve_mode`, `color_precision`, `filter_speckle`, `layer_difference`, `corner_threshold`: adjusted between iterations per the diagnosis table below
* **Parameters (`compare_images`, per iteration):**
  * `image1_path`: `D:\Projects\Public\rune\code-kit\temp\head.png`
  * `image2_path`: `D:\Projects\Public\rune\code-kit\temp\head.svg`
  * `algorithm`: `'rms'`
* **Expected Output:** A `match_percentage` from `compare_images` that increases (or holds) across iterations, and a final SVG whose traced output is visually indistinguishable from `head.png` at the diff-image level.
* **Diagnosis table — what to change based on the diff, one step at a time:**

  | Symptom in the diff image | Likely cause | Adjust |
  |---|---|---|
  | Broad color banding, flat gradients look stepped | `color_precision` too low | Raise `color_precision` (default 6) |
  | Lots of tiny scattered red speckles, noisy background | Small shapes surviving that shouldn't | Raise `filter_speckle` (default 4) |
  | Fine detail / small features missing entirely | `filter_speckle` too aggressive | Lower `filter_speckle` |
  | Smooth curves look jagged or faceted | `trace_curve_mode` set to `polygon`, or `corner_threshold` too low | Set `trace_curve_mode` to `'spline'` (default) or raise `corner_threshold` |
  | Soft gradients rendered as hard color bands | `layer_difference` too high | Lower `layer_difference` (default 16) |
  | Shapes look flattened / missing depth in overlaps | Wrong `trace_hierarchical` mode | Try the other of `'stacked'` / `'cutout'` |
  | Color image traced in one flat tone | `trace_color_mode` accidentally set to `'binary'` | Set `trace_color_mode` to `'color'` (default) |

* **Notes:**
  * You don't need to force the SVG to rasterize at the PNG's exact pixel dimensions before comparing: `compare_images` already resizes both inputs to matching dimensions (via Lanczos3) before diffing, so a `head.png` at 891x647 and a `head.svg` rasterized at the default 800x800 are still compared correctly. (There *is* a way to make `load_and_convert_image` rasterize an SVG at a specific size instead of the 800x800 default — the filename itself has to literally contain `width=` and `height=` before the `.svg` extension, e.g. `head_width=891_height=647.svg` — but that's an existing quirk of this codebase, not something you need for this workflow, since the resize-to-match-dimensions step already handles it.)
  * This is the workflow the earlier one-shot version of this use case was missing — a single `convert_image_format` call with hand-picked settings is a guess, not a fit. Comparing after *every* change and moving one knob at a time is what makes the loop converge instead of thrashing between unrelated parameter changes.
  * This loop depends on `compare_images`'s threshold handling being correct — a prior bug in this plugin could report a 0% match when comparing an image against an identical copy of itself, which would have made "did that last change actually help?" impossible to answer honestly. That's fixed, so `match_percentage` here can be trusted step to step.

#### Use Case IMG-12: Rasterize an SVG Icon

* **Category:** Format Conversion / Vector-to-Raster
* **Prompt:** "Convert the icon at '/path/to/icon.svg' to a 512x512 PNG."
* **Expected Tool(s):** `convert_image_format`
* **Parameters:** `input_path: '/path/to/icon.svg'`, `output_format: 'png'`
* **Expected Output:** JSON with `output_path` pointing to the generated raster file