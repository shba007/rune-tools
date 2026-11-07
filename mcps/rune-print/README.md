### `rune-print`

* **Description:** A native OS printing MCP server providing printer discovery, capability queries, and spooler dispatch for TXT/PDF/image documents to local and network printers.

* **Tool Definitions:** `printer_list_printers`, `printer_print_document`, `printer_get_capabilities`, `printer_cancel_job`

* **MCP Configuration:**

```json
{
  "mcpServers": {
    "rune-print": {
      "command": "rune",
      "args": [
        "run",
        "rune-print"
      ],
      "env": {
        "TEST_BASE_DIR": "./test-dir",
        "ALLOWED_DIR": "./test-dir"
      }
    }
  }
}
```

**Environment Variables:**

* `TEST_BASE_DIR`: Base directory used for test fixtures and print job staging (no default).
* `ALLOWED_DIR`: Root boundary directory enforced for sandbox containment of documents submitted for printing (default: `.`).

#### Use Case PRT-01: List Available Printers

* **Prompt:** "List all printers currently available on this system."
* **Expected Tool(s):** `printer_list_printers`

#### Use Case PRT-02: Print a PDF Document

* **Prompt:** "Print './test-dir/docs/report.pdf' to the default printer as a single job."
* **Expected Tool(s):** `printer_print_document`

#### Use Case PRT-03: Print with Copies and Duplex

* **Category:** Granular Options
* **Prompt:** "Print './test-dir/docs/flyer.pdf' to 'Office-LaserJet' with 2 copies, double-sided."
* **Expected Tool(s):** `printer_print_document`

#### Use Case PRT-04: Query Printer Capabilities

* **Prompt:** "What paper sizes, resolutions, and media types does 'Office-LaserJet' support?"
* **Expected Tool(s):** `printer_get_capabilities`

#### Use Case PRT-05: Cancel a Stuck Job

* **Prompt:** "Cancel print job 42 on 'Office-LaserJet'."
* **Expected Tool(s):** `printer_cancel_job`

#### Use Case PRT-06: Non-Existent Document Error Handling

* **Category:** Edge Case / Error Handling
* **Prompt:** "Print './test-dir/docs/does-not-exist.pdf' to the default printer."
* **Expected Tool(s):** `printer_print_document`
