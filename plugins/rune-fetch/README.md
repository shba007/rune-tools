### `rune-fetch`

* **Description:** A web page fetching and HTML-to-Markdown conversion MCP server that retrieves any HTTP/HTTPS URL, converts the response body to clean Markdown (or raw text), and supports character-level pagination for large pages via `start_index` / `next_start_index` cursor semantics.

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

* *(none required)* — all parameters are passed per-invocation.

#### Use Case FCH-01: Fetch a Page as Markdown

* **Category:** Happy Path
* **Prompt:** "Fetch 'https://example.com' and give me the content as Markdown."
* **Expected Tool(s):** `fetch`

#### Use Case FCH-02: Paginate a Large Article

* **Category:** Granular Options / Pagination
* **Prompt:** "Fetch 'https://en.wikipedia.org/wiki/Artificial_intelligence', but only from character 50000 onward."
* **Expected Tool(s):** `fetch`

#### Use Case FCH-03: Raw HTML Retrieval

* **Category:** Granular Options / Raw Mode
* **Prompt:** "Get the raw HTML source of 'https://example.com' without any Markdown conversion."
* **Expected Tool(s):** `fetch`

#### Use Case FCH-04: Length-Capped Fetch

* **Category:** Happy Path / Size Control
* **Prompt:** "Fetch 'https://news.ycombinator.com' but limit the response to 10,000 characters."
* **Expected Tool(s):** `fetch`

#### Use Case FCH-05: Invalid URL Error Handling

* **Category:** Edge Case / Error Handling
* **Prompt:** "Fetch 'https://nonexistent-domain-xyz.invalid/page'."
* **Expected Tool(s):** `fetch`
