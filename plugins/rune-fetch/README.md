### `rune-fetch`

* **Description:** A web page fetching MCP server that converts HTML to Markdown (or returns raw text) with character-level offset-based pagination, enabling large pages to be read in sequential chunks via numeric start indices and lengths.

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

#### Use Case FET-02: Offset-Based Pagination

* **Category:** Pagination / Offset-Based Reading
* **Prompt:** "Fetch 'https://example.com/long-doc'. Read the first 20,000 characters, then continue from index 20,000 for the next 20,000."
* **Expected Tool(s):** `fetch`

#### Use Case FET-03: Raw Text Mode

* **Category:** Granular Options
* **Prompt:** "Fetch 'https://example.com/page' as raw text without any HTML-to-Markdown conversion."
* **Expected Tool(s):** `fetch`

#### Use Case FET-04: Custom Chunk Size

* **Category:** Granular Options
* **Prompt:** "Fetch 'https://example.com/small-page' with a max length of 5,000 characters starting from index 0."
* **Expected Tool(s):** `fetch`

#### Use Case FET-05: Non-200 Error Handling

* **Category:** Edge Case / HTTP Error Handling
* **Prompt:** "Fetch 'https://example.com/invalid-url' and show me the error response."
* **Expected Tool(s):** `fetch`

#### Use Case FET-06: Full Pagination Loop

* **Category:** Pagination / Offset-Based Reading
* **Prompt:** "Fetch 'https://example.com/huge-doc'. Read the first 10,000 characters, then continue from index 10,000 for the next 10,000, and finally from index 20,000 for the remainder."
* **Expected Tool(s):** `fetch`

#### Use Case FET-07: Offset Beyond Content

* **Category:** Edge Case / Out-of-Range Handling
* **Prompt:** "Fetch 'https://example.com/page' starting from index 100,000 (beyond content length)."
* **Expected Tool(s):** `fetch`

#### Use Case FET-08: Empty URL Error Handling

* **Category:** Edge Case / Error Handling
* **Prompt:** "Fetch a page with an empty URL and show me the error."
* **Expected Tool(s):** `fetch`

#### Use Case FET-09: Non-Html Content as Raw Text

* **Category:** Granular Options
* **Prompt:** "Fetch 'https://example.com/data.json' as raw text without HTML conversion."
* **Expected Tool(s):** `fetch`

#### Use Case FET-10: Large Offset with Max Length

* **Category:** Granular Options
* **Prompt:** "Fetch 'https://example.com/large-file' with start_index=50000 and max_length=1000 (reading in the middle of a large document)."
* **Expected Tool(s):** `fetch`
