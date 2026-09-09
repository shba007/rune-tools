### `rune-time`

* **Description:** A deterministic time and timezone MCP server built on the IANA `tzdb` database, providing current-time queries, ISO-8601 formatting, and DST-aware cross-timezone conversions. No external binaries required.

* **Tool Definitions:** `get_current_time`, `convert_time`

* **MCP Configuration:**

```json
{
  "mcpServers": {
    "rune-time": {
      "command": "rune",
      "args": [
        "run",
        "rune-time"
      ],
      "env": {
        "DEFAULT_TIMEZONE": "UTC"
      }
    }
  }
}
```

**Environment Variables:**

* `DEFAULT_TIMEZONE`: IANA timezone name (e.g., `America/New_York`, `Asia/Tokyo`) used when a tool call omits the `timezone` parameter (default: `UTC`).

#### Use Case TIME-01: Current Time in UTC

* **Prompt:** "What is the current time in UTC?"
* **Expected Tool(s):** `get_current_time`

#### Use Case TIME-02: Local Time with Custom Date Format

* **Prompt:** "Show me the current time in Europe/Berlin formatted as 'YYYY-MM-DD HH:mm:ss'."
* **Expected Tool(s):** `get_current_time`

#### Use Case TIME-03: Cross-Timezone Conversion

* **Prompt:** "What time is it in Tokyo when it's 3:00 PM in New York?"
* **Expected Tool(s):** `convert_time`

#### Use Case TIME-04: DST-Aware Conversion Check

* **Prompt:** "Convert 2026-03-08 01:30 from America/Chicago to Asia/Kolkata and tell me whether daylight saving applies on that date."
* **Expected Tool(s):** `convert_time`

#### Use Case TIME-05: Invalid Timezone Error Handling

* **Category:** Edge Case / Error Handling
* **Prompt:** "Get the current time in timezone 'Mars/Olympus_Mons'."
* **Expected Tool(s):** `get_current_time`
