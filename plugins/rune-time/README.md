### `rune-time`

* **Description:** Deterministic timezone queries, ISO-8601 formatting, and DST-aware cross-timezone conversions.

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
        "ALLOWED_DIR": "./test-dir"
      }
    }
  }
}
```

**Environment Variables:**

* `default_timezone`: Default IANA timezone name (e.g., "America/New_York", "Asia/Kolkata", "UTC"). Used when timezone parameter is not provided in tool calls.
* `ALLOWED_DIR`: Root directory boundary enforced for sandbox isolation. All file operations are restricted to this directory or its subdirectories (default: .).

#### Use Case TIME-01: 

* **Prompt:** "Get the current time in UTC timezone."
* **Expected Tool(s):** `get_current_time`

#### Use Case TIME-02: 

* **Prompt:** "Get the current time in the 'America/New_York' timezone."
* **Expected Tool(s):** `get_current_time`

#### Use Case TIME-03: 

* **Prompt:** "Convert the time '2026-09-02T14:30:00' from 'UTC' to 'Asia/Tokyo' timezone."
* **Expected Tool(s):** `convert_time`

#### Use Case TIME-04: 

* **Prompt:** "Convert the time '2026-09-02 14:30:00' from 'America/Los_Angeles' to 'Europe/London' timezone."
* **Expected Tool(s):** `convert_time`

#### Use Case TIME-05: 

* **Prompt:** "Get the current time in multiple timezones: UTC, 'Asia/Kolkata', and 'Europe/London'."
* **Expected Tool(s):** `get_current_time`

#### Use Case TIME-06: 

* **Prompt:** "Convert the time '2026-09-02T14:30:00Z' from 'UTC' to 'America/New_York' (DST aware)."
* **Expected Tool(s):** `convert_time`

#### Use Case TIME-07: 

* **Prompt:** "Convert the time '2026-09-02 14:30:00' from 'Europe/London' to 'Asia/Tokyo' with automatic DST handling."
* **Expected Tool(s):** `convert_time`

#### Use Case TIME-08: 

* **Prompt:** "Get the current time in a custom timezone like 'Asia/Kolkata'."
* **Expected Tool(s):** `get_current_time`

#### Use Case TIME-09: 

* **Prompt:** "Convert the time '2026-09-02T14:30:00' from 'UTC' to 'America/Chicago' and show the offset."
* **Expected Tool(s):** `convert_time`

#### Use Case TIME-10: 

* **Prompt:** "Get the current time in multiple timezones for cross-region coordination."
* **Expected Tool(s):** `get_current_time`

#### Use Case TIME-11: 

* **Prompt:** "Convert the time '2026-09-02 14:30:00' from 'Australia/Sydney' to 'Asia/Dubai'."
* **Expected Tool(s):** `convert_time`

#### Use Case TIME-12: 

* **Prompt:** "Get the current time in 'Europe/Moscow' timezone."
* **Expected Tool(s):** `get_current_time`

#### Use Case TIME-13: 

* **Prompt:** "Convert the time '2026-09-02T14:30:00' from 'America/Phoenix' to 'UTC' (Arizona doesn't observe DST)."
* **Expected Tool(s):** `convert_time`

#### Use Case TIME-14: 

* **Prompt:** "Get the current time in 'Antarctica/McMurdo' timezone."
* **Expected Tool(s):** `get_current_time`

#### Use Case TIME-15: 

* **Prompt:** "Convert the time '2026-09-02 14:30:00' from 'Asia/Shanghai' to 'Asia/Tokyo'."
* **Expected Tool(s):** `convert_time`

#### Use Case TIME-16: 

* **Prompt:** "Get the current time in 'America/Juneau' timezone."
* **Expected Tool(s):** `get_current_time`

#### Use Case TIME-17: 

* **Prompt:** "Convert the time '2026-09-02T14:30:00' from 'Europe/Paris' to 'Europe/Berlin' (same timezone)."
* **Expected Tool(s):** `convert_time`

#### Use Case TIME-18: 

* **Prompt:** "Get the current time in 'Pacific/Honolulu' timezone."
* **Expected Tool(s):** `get_current_time`

#### Use Case TIME-19: 

* **Prompt:** "Convert the time '2026-09-02 14:30:00' from 'Asia/Kolkata' to 'Asia/Kolkata' (same timezone)."
* **Expected Tool(s):** `convert_time`

#### Use Case TIME-20: 

* **Prompt:** "Get the current time in 'America/Los_Angeles' timezone for cross-timezone planning."
* **Expected Tool(s):** `get_current_time`
