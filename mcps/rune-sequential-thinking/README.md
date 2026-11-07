### `rune-sequential-thinking`

* **Description:** A structured reasoning workspace MCP server that lets an agent plan, track, and revise multi-step problem solving. Supports total-step estimation, per-step progress tracking, hypothesis revision, thought branching, and automatic total adjustment.

* **Tool Definitions:** `sequential-thinking`

* **MCP Configuration:**

```json
{
  "mcpServers": {
    "rune-sequential-thinking": {
      "command": "rune",
      "args": [
        "run",
        "rune-sequential-thinking"
      ]
    }
  }
}
```

**Environment Variables:**

* None. All state is passed per-call via the tool parameters.

#### Use Case SEQ-01: Initialize a Reasoning Plan

* **Prompt:** "Start a sequential thinking session for 'design a caching layer for our API'. Estimate 6 steps and list the first step."
* **Expected Tool(s):** `sequential-thinking`

#### Use Case SEQ-02: Record a Step and Advance

* **Prompt:** "Mark step 2 of my current reasoning plan as complete with the note 'schema validated', then show the next step."
* **Expected Tool(s):** `sequential-thinking`

#### Use Case SEQ-03: Revise a Hypothesis Mid-Plan

* **Category:** Hypothesis Revision
* **Prompt:** "My assumption in step 3 about the cache backend was wrong — replace it with 'use Redis with TTL eviction' and update the remaining steps accordingly."
* **Expected Tool(s):** `sequential-thinking`

#### Use Case SEQ-04: Branch Into an Alternative Path

* **Category:** Thought Branching
* **Prompt:** "Branch off step 4 and explore an alternative approach using in-process LRU cache instead of a distributed store."
* **Expected Tool(s):** `sequential-thinking`

#### Use Case SEQ-05: Adjust Total Step Count

* **Category:** Automatic Total Adjustment
* **Prompt:** "The plan is growing — add 3 more steps to the total and re-estimate completion."
* **Expected Tool(s):** `sequential-thinking`
