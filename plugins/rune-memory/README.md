### `rune-memory`

* **Description:** A persistent knowledge-graph memory MCP server that stores typed entities and relations in a JSON file, supporting entity/relation creation (single or batch), querying by type/limit/predicate/object, deletion, and full inspection of the graph.

* **Tool Definitions:** `create_entities`, `create_relations`, `query_memory`, `delete_entities`, `get_all_memory`

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
        "MEMORY_FILE": "./test-dir/memory.json",
        "ALLOWED_DIR": "./test-dir"
      }
    }
  }
}
```

**Environment Variables:**

* `MEMORY_FILE`: Path to the JSON file used for persisting the knowledge graph (default: `memory.json`).
* `ALLOWED_DIR`: Root directory against which the memory file path is resolved when it is relative (default: `.`).

#### Use Case MEM-01: Create a Single Entity

* **Category:** Happy Path
* **Prompt:** "Remember that 'Ada Lovelace' is a Person with the description 'first programmer'."
* **Expected Tool(s):** `create_entities`

#### Use Case MEM-02: Batch Create Entities

* **Category:** Happy Path / Batch
* **Prompt:** "Create entities for three projects: 'Alpha' (Project), 'Beta' (Project), and 'Gamma' (Project)."
* **Expected Tool(s):** `create_entities`

#### Use Case MEM-03: Create a Relation Between Entities

* **Category:** Happy Path
* **Prompt:** "Record that 'Ada Lovelace' authored the entity 'Analytical Engine'."
* **Expected Tool(s):** `create_relations`

#### Use Case MEM-04: Query Entities by Type

* **Category:** Granular Options / Filter
* **Prompt:** "Show me all entities of type 'Person' in memory, up to 10 results."
* **Expected Tool(s):** `query_memory`

#### Use Case MEM-05: Query Relations by Predicate

* **Category:** Granular Options / Filter
* **Prompt:** "Find all relations where the predicate is 'authored' and return at most 20."
* **Expected Tool(s):** `query_memory`

#### Use Case MEM-06: Delete an Entity

* **Category:** Happy Path / Mutation
* **Prompt:** "Delete the entity with id 'entity-alpha' from memory."
* **Expected Tool(s):** `delete_entities`

#### Use Case MEM-07: Inspect the Full Graph

* **Category:** Happy Path / Inspection
* **Prompt:** "Dump all entities and relations currently stored in memory."
* **Expected Tool(s):** `get_all_memory`

#### Use Case MEM-08: Query With No Matches Edge Case

* **Category:** Edge Case / Error Handling
* **Prompt:** "Query for entities of type 'NonExistentType' that shouldn't exist."
* **Expected Tool(s):** `query_memory`
