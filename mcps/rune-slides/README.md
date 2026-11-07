### `rune-slides`

* **Description:** A slide deck generation MCP server that manages a JSON project bundle of slides, supports themed layouts, and exports decks to PPTX and PDF formats.

* **Tool Definitions:** `slides_create_project`, `slides_add_slide`, `slides_update_slide`, `slides_export`, `slides_list_projects`

* **MCP Configuration:**

```json
{
  "mcpServers": {
    "rune-slides": {
      "command": "rune",
      "args": [
        "run",
        "rune-slides"
      ],
      "env": {
        "DEFAULT_THEME": "modern",
        "TEST_BASE_DIR": "./test-dir",
        "ALLOWED_DIR": "./test-dir"
      }
    }
  }
}
```

**Environment Variables:**

* `DEFAULT_THEME`: Slide theme applied when a project or slide does not specify one (default: `modern`).
* `TEST_BASE_DIR`: Base directory used for test fixtures and export output staging (no default).
* `ALLOWED_DIR`: Root boundary directory enforced for sandbox containment of project bundles and exported files (default: `.`).

#### Use Case SLIDE-01: Create a New Deck

* **Prompt:** "Create a slide project called 'Q3 Review' with the modern theme."
* **Expected Tool(s):** `slides_create_project`

#### Use Case SLIDE-02: Add a Title Slide

* **Prompt:** "Add a title slide to 'Q3 Review' with title 'Q3 2026 Results' and subtitle 'Engineering Division'."
* **Expected Tool(s):** `slides_add_slide`

#### Use Case SLIDE-03: Update an Existing Slide

* **Prompt:** "Change slide 2 of 'Q3 Review' to a bullet layout with items: Revenue up 12%, Churn down 3%, Headcount +5."
* **Expected Tool(s):** `slides_update_slide`

#### Use Case SLIDE-04: Export to PPTX

* **Category:** Granular Options / Export
* **Prompt:** "Export 'Q3 Review' as a PPTX file into './test-dir/decks/.'"
* **Expected Tool(s):** `slides_export`

#### Use Case SLIDE-05: Export to PDF

* **Category:** Granular Options / Export
* **Prompt:** "Export 'Q3 Review' as a PDF into './test-dir/decks/q3.pdf'."
* **Expected Tool(s):** `slides_export`

#### Use Case SLIDE-06: List Existing Projects

* **Prompt:** "List all slide projects currently on disk."
* **Expected Tool(s):** `slides_list_projects`

#### Use Case SLIDE-07: Export Non-Existent Project

* **Category:** Edge Case / Error Handling
* **Prompt:** "Export the slide project 'NoSuchDeck-12345' to PPTX."
* **Expected Tool(s):** `slides_export`
