### `rune-sequentialthinking`

* **Description:** A dynamic step-by-step reasoning and thought branching MCP server that provides a structured thinking workspace for AI agents. Supports sequential thought tracking with progress indicators, hypothesis revision (correcting previous thoughts), alternative branch exploration from any prior thought node, and automatic total-thoughts adjustment when reasoning exceeds initial estimates.

* **Tool Definitions:** `sequentialthinking`

* **MCP Configuration:**

```json
{
  "mcpServers": {
    "rune-sequentialthinking": {
      "command": "rune",
      "args": [
        "run",
        "rune-sequentialthinking"
      ]
    }
  }
}
```

**Environment Variables:**

* *(none required)* — all parameters are passed per-invocation.

#### Use Case STT-01: Linear Step-by-Step Problem Solving

* **Category:** Happy Path / Sequential
* **Prompt:** "I need to solve a complex math problem. Start thinking through it step by step, beginning with understanding the problem statement."
* **Expected Tool(s):** `sequentialthinking`

#### Use Case STT-02: Revising a Previous Hypothesis

* **Category:** Granular Options / Revision
* **Prompt:** "Wait — my second assumption was wrong. Let me revise thought #2: the boundary condition should be at x=0, not x=1."
* **Expected Tool(s):** `sequentialthinking`

#### Use Case STT-03: Branch to Explore an Alternative Path

* **Category:** Granular Options / Branching
* **Prompt:** "Let me branch off from thought #4 and explore the alternative approach using dynamic programming instead of recursion. Call this branch 'dp-approach'."
* **Expected Tool(s):** `sequentialthinking`

#### Use Case STT-04: Dynamic Total Adjustment

* **Category:** Happy Path / Adaptive
* **Prompt:** "I estimated 5 steps but I'm on step 7 and still need more analysis. Continue with the next thought, updating the total."
* **Expected Tool(s):** `sequentialthinking`

#### Use Case STT-05: Empty Thought Rejection

* **Category:** Edge Case / Validation
* **Prompt:** "Add a thought with empty text content."
* **Expected Tool(s):** `sequentialthinking`
