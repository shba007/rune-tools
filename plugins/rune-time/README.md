### `rune-time`

* **Description:** A deterministic timezone query and conversion MCP server built on chrono/chrono-tz. Retrieves the current date and time in UTC alongside any target IANA timezone (with offset details and epoch seconds), and converts flexible ISO-8601 datetime strings between timezones with DST-aware offset computation. Supports RFC 3339, `YYYY-MM-DD HH:MM[:SS]`, slash-separated variants, and date-only inputs (treated as midnight).

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
        "DEFAULT_TIMEZONE": "America/New_York"
      }
    }
  }
}
```

**Environment Variables:**

* `DEFAULT_TIMEZONE`: Default IANA timezone used by `get_current_time` when no `timezone` parameter is supplied (default: `UTC`).

#### Use Case TIME-01: Current Time in a Target Timezone

* **Category:** Happy Path / Query
* **Prompt:** "What's the current date and time in Tokyo? Include the UTC offset."
* **Expected Tool(s):** `get_current_time`

#### Use Case TIME-02: Current Time Using the Configured Default

* **Category:** Granular Options / Default Timezone
* **Prompt:** "What time is it right now?" (no timezone specified — falls back to the configured default)
* **Expected Tool(s):** `get_current_time`

#### Use Case TIME-03: Cross-Timezone Meeting Conversion

* **Category:** Happy Path / Conversion
* **Prompt:** "Convert '2026-09-02T14:30:00' from America/Los_Angeles to Asia/Tokyo so I can schedule the call."
* **Expected Tool(s):** `convert_time`

#### Use Case TIME-04: Flexible Datetime Formats

* **Category:** Granular Options / Parsing
* **Prompt:** "What UTC time does midnight on 2026/12/25 in Europe/London correspond to?"
* **Expected Tool(s):** `convert_time`

#### Use Case TIME-05: Invalid IANA Timezone Rejection

* **Category:** Edge Case / Validation
* **Prompt:** "Convert '2026-09-02 14:30:00' from 'New York City' to Asia/Tokyo."
* **Expected Tool(s):** `convert_time`

#### Use Case TIME-06: Ambiguous DST Fallback Handling

* **Category:** Edge Case / DST Overlap
* **Prompt:** "Convert '2026-11-01 01:30:00' in America/New_York to UTC." (falls inside the fall-back overlap — ambiguous local time)
* **Expected Tool(s):** `convert_time`
