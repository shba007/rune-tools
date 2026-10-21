### `rune-scan`

* **Description:** A flatbed/ADF document scanning MCP server speaking the eSCL protocol over HTTP(S), supporting scan initiation, progress monitoring, and scanner discovery against a network eSCL device.

* **Tool Definitions:** `scan_document`, `list_scanners`

* **MCP Configuration:**

```json
{
  "mcpServers": {
    "rune-scan": {
      "command": "rune",
      "args": [
        "run",
        "rune-scan"
      ],
      "env": {
        "SCANNER_IP": "192.168.1.50",
        "SCANNER_PORT": "443",
        "SCANNER_USE_HTTPS": "true",
        "TEST_BASE_DIR": "./test-dir",
        "ALLOWED_DIR": "./test-dir"
      }
    }
  }
}
```

**Environment Variables:**

* `SCANNER_IP`: Default eSCL scanner host address (no default; required unless passed per call).
* `SCANNER_PORT`: eSCL endpoint port (default: `443`).
* `SCANNER_USE_HTTPS`: Whether to use HTTPS for the eSCL endpoint (default: `true`).
* `TEST_BASE_DIR`: Base directory used for test fixtures and scanned output staging (no default).
* `ALLOWED_DIR`: Root boundary directory enforced for sandbox containment of scanned output files (default: `.`).

#### Use Case SCN-01: Flatbed Scan at 300 DPI

* **Prompt:** "Scan the document on the flatbed at 300 DPI as a PDF and save it to the test directory."
* **Expected Tool(s):** `scan_document`

#### Use Case SCN-02: ADF Multi-Page Scan

* **Category:** Granular Options
* **Prompt:** "Run an ADF scan of all pages at 200 DPI in grayscale, output as JPEG, into './test-dir/scans/.'"
* **Expected Tool(s):** `scan_document`

#### Use Case SCN-03: Discover Available Scanners

* **Prompt:** "List the eSCL scanners reachable on the network and their capabilities."
* **Expected Tool(s):** `list_scanners`

#### Use Case SCN-04: Explicit Scanner Endpoint Override

* **Category:** Authentication / Session Handling
* **Prompt:** "Scan using the scanner at '10.0.0.5:8080' over plain HTTP instead of the configured default."
* **Expected Tool(s):** `scan_document`

#### Use Case SCN-05: Unreachable Scanner Error Handling

* **Category:** Edge Case / Error Handling
* **Prompt:** "Scan the flatbed at 300 DPI from scanner '192.168.255.255'."
* **Expected Tool(s):** `scan_document`
