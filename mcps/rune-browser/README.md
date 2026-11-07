# rune-browser

**Description:** Browser automation with agent-browser (headless) and CDP (existing browser) support. Features: navigation, element interaction, form filling, screenshots, PDF export, console/network diagnostics, and persistent sessions.

**Tool Definitions:** `browser_session_start`, `browser_session_stop`, `browser_session_list`, `browser_navigate`, `browser_type`, `browser_click`, `browser_fill`, `browser_select_option`, `browser_hover`, `browser_execute_script`, `browser_screenshot`, `browser_pdf`, `browser_console_messages`, `browser_network_requests`, `browser_cdp_connect`, `browser_cdp_request`, `browser_cdp_disconnect`

**MCP Configuration:**

```json
{
  "mcpServers": {
    "rune-browser": {
      "command": "rune",
      "args": ["run", "rune-browser"],
      "env": {
        "OUTPUT_DIR": "./test-dir/browser-out",
        "ALLOWED_DIR": "./test-dir"
      }
    }
  }
}
```

**Environment Variables:**

* `OUTPUT_DIR`: Default destination for screenshots, PDF exports, and other captured artifacts (default: `./browser-out`).
* `ALLOWED_DIR`: Root boundary directory enforced for sandbox isolation; artifact output paths are confined within it (default: `.`).
* `AGENT_BROWSER_PATH`: Override the path to the agent-browser binary (default: `agent-browser`).

### Core Features

#### Headless Mode (agent-browser)

Launches a headless browser with full lifecycle management:

* **Session Management**
  * `browser_session_start` - Start persistent session
  * `browser_session_stop` - Stop session
  * `browser_session_list` - List active sessions

* **Navigation & Interaction**
  * `browser_navigate` - Navigate to URL
  * `browser_type` - Type text into inputs
  * `browser_click` - Click elements
  * `browser_fill` - Fill form inputs
  * `browser_select_option` - Select dropdown options
  * `browser_hover` - Hover over elements

* **Content & Export**
  * `browser_execute_script` - Execute JavaScript
  * `browser_screenshot` - Capture screenshots
  * `browser_pdf` - Export as PDF

#### Diagnostics

* **Console & Network**
  * `browser_console_messages` - Console log capture
  * `browser_network_requests` - Network request inspection

#### CDP Mode (existing browser)

Connect to existing Chrome DevTools sessions:

* **Connection Management**
  * `browser_cdp_connect` - Connect to CDP endpoint
  * `browser_cdp_request` - Send CDP requests
  * `browser_cdp_disconnect` - Disconnect from session

### Use Cases

#### BCW-01: Start Headless Browser Session

* **Category:** Session Management
* **Prompt:** "Start a headless browser session for testing."
* **Expected Tool(s):** `browser_session_start`

#### BCW-02: Navigate and Interact

* **Category:** Happy Path
* **Prompt:** "Navigate to 'https://example.com', click the first link, and take a screenshot."
* **Expected Tool(s):** `browser_navigate`, `browser_click`, `browser_screenshot`

#### BCW-03: Fill Form and Submit

* **Category:** Multi-Tool Chain
* **Prompt:** "Fill the login form with email 'user@example.com' and password 'secret', then submit."
* **Expected Tool(s):** `browser_navigate`, `browser_fill`, `browser_click`

#### BCW-04: Export Page as PDF

* **Category:** Artifacts
* **Prompt:** "Export the current page as a PDF."
* **Expected Tool(s):** `browser_pdf`

#### BCW-05: Inspect Console Messages

* **Category:** Diagnostics
* **Prompt:** "Show all console messages from the current session."
* **Expected Tool(s):** `browser_console_messages`

#### BCW-06: Connect to Existing Browser

* **Category:** CDP Integration
* **Prompt:** "Connect to Chrome at localhost:9222 for debugging."
* **Expected Tool(s):** `browser_cdp_connect`
