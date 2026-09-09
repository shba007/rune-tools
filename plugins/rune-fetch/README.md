### `rune-fetch`

* **Description:** A web page fetching MCP server that converts HTML to Markdown (or returns raw text) with character-level pagination via cursor semantics, enabling large pages to be read in sequential chunks.

* **Tool Definitions:** `fetch`

* **MCP Configuration:**

```json
{
  "mcpServers": {
    "rune-fetch": {
      "command": "rune",
      "args": [
        "run",
        "rune-fetch"
      ]
    }
  }
}
```

**Environment Variables:**

* None. All behavior is controlled per-call via tool parameters.

#### Use Case FET-01: Fetch a Page as Markdown

* **Prompt:** "Fetch 'https://example.com/article' and give me the content as Markdown."
* **Expected Tool(s):** `fetch`

#### Use Case FET-02: Paginated Read of a Long Page

* **Category:** Pagination / Cursor Semantics
* **Prompt:** "Fetch 'https://example.com/long-doc'. Read the first 20,000 characters, then continue from the returned cursor for the next 20,000."
* **Expected Tool(s):** `fetch`

#### Use Case FET-03: Raw Text Mode

* **Category:** Granular Options
* **Prompt:** "Fetch 'https://example.com/page' as raw text without any HTML-to-Markdown conversion."
* **Expected Tool(s):** `fetch`

#### Use Case FET-04: Custom Chunk Size

* **Category:** Granular Options
* **Prompt:** "Fetch 'https://example.com/small-page' with a max length of 5,000 characters starting from index 0."
* **Expected Tool(s):** `fetch`

#### Use Case FET-05: Invalid URL Error Handling

* **Category:** Edge Case / Error Handling
* **Prompt:** "Fetch 'not-a-valid-url' and show me the content."
* **Expected Tool(s):** `fetch`
