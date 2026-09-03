### `rune-memory`

* **Description:** A persistent knowledge-graph memory MCP server that stores typed entities (each carrying observations) and relations in a single JSON file. Supports batch entity/relation creation, adding or removing observations, deleting entities/relations/observations, full graph inspection, substring search across the graph, and targeted node lookup with connected relations.

* **Tool Definitions:** `create_entities`, `create_relations`, `add_observations`, `delete_entities`, `delete_observations`, `delete_relations`, `read_graph`, `search_nodes`, `open_nodes`

* **MCP Configuration:**

```json
{
  "mcpServers": {
    "rune-memory": {
      "command": "rune",
      "args": [
        "run",
        "rune-memory"
      ],
      "env": {
        "MEMORY_FILE": "graph_memory.json",
        "ALLOWED_DIR": "./test-dir"
      }
    }
  }
}
```

**Environment Variables:**

* `MEMORY_FILE`: Path to the JSON file used for persisting the knowledge graph. When it is a bare filename it is resolved relative to `ALLOWED_DIR` (default: `memory.json`).
* `ALLOWED_DIR`: Root directory against which the memory file path is resolved and enforced for filesystem sandbox containment (default: `.`).

#### Use Case MEM-01: Create a Single Entity

* **Category:** Happy Path / Creation
* **Prompt:** "Remember that 'Ada Lovelace' is a Person with the observation 'first programmer'."
* **Expected Tool(s):** `create_entities`

#### Use Case MEM-02: Batch Create Entities

* **Category:** Happy Path / Batch
* **Prompt:** "Create entities for three projects: 'Alpha', 'Beta', and 'Gamma', each of type Project."
* **Expected Tool(s):** `create_entities`

#### Use Case MEM-03: Create a Relation Between Entities

* **Category:** Happy Path / Creation
* **Prompt:** "Record that 'Ada Lovelace' authored the 'Analytical Engine'."
* **Expected Tool(s):** `create_relations`

#### Use Case MEM-04: Add Observations to an Existing Entity

* **Category:** Happy Path / Mutation
* **Prompt:** "Add the observation 'born 1815' to the entity 'Ada Lovelace'."
* **Expected Tool(s):** `add_observations`

#### Use Case MEM-05: Delete an Entity and Its Relations

* **Category:** Happy Path / Mutation
* **Prompt:** "Delete the entity 'Gamma' from memory."
* **Expected Tool(s):** `delete_entities`

#### Use Case MEM-06: Delete Specific Observations

* **Category:** Granular Options / Mutation
* **Prompt:** "Remove the observation 'first programmer' from 'Ada Lovelace'."
* **Expected Tool(s):** `delete_observations`

#### Use Case MEM-07: Delete a Relation

* **Category:** Happy Path / Mutation
* **Prompt:** "Remove the relation where 'Ada Lovelace' authored the 'Analytical Engine'."
* **Expected Tool(s):** `delete_relations`

#### Use Case MEM-08: Inspect the Full Graph

* **Category:** Happy Path / Inspection
* **Prompt:** "Dump all entities and relations currently stored in memory."
* **Expected Tool(s):** `read_graph`

#### Use Case MEM-09: Search Nodes by Substring

* **Category:** Happy Path / Query
* **Prompt:** "Find everything in memory that mentions 'engine'."
* **Expected Tool(s):** `search_nodes`

#### Use Case MEM-10: Open Specific Nodes

* **Category:** Granular Options / Inspection
* **Prompt:** "Show me the details and connected relations for 'Ada Lovelace' and 'Analytical Engine'."
* **Expected Tool(s):** `open_nodes`
