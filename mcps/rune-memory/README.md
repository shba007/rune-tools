### `rune-memory`

* **Description:** A persistent knowledge-graph memory MCP server storing typed entities and relations in a single JSON file, with batch creation, filtered queries, deletion, and full-graph inspection for long-term agent memory.

* **Tool Definitions:** `create_entity`, `create_relation`, `batch_create_entities`, `query_memory`, `delete_entity`, `inspect_memory`, `get_entity`, `list_entities`, `count_entities`

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

* `MEMORY_FILE`: Path of the JSON file backing the knowledge graph (default: `memory.json`).
* `ALLOWED_DIR`: Root boundary directory enforced for sandbox isolation; the memory file must reside inside it (default: `.`).

#### Use Case MEM-01: Create a Single Entity

* **Prompt:** "Remember that 'Apollo-11' is a space mission with property launch_date=1969-07-16."
* **Expected Tool(s):** `create_entity`

#### Use Case MEM-02: Create a Typed Relation

* **Prompt:** "Link entity 'Apollo-11' to 'Neil Armstrong' with the relation 'carried_by'."
* **Expected Tool(s):** `create_relation`

#### Use Case MEM-03: Batch Entity Creation

* **Category:** Batch Ingestion
* **Prompt:** "Create these entities in one batch: 'Project-Rune' (type=project), 'Rust' (type=language), 'MCP' (type=protocol)."
* **Expected Tool(s):** `batch_create_entities`

#### Use Case MEM-04: Filtered Query

* **Category:** Granular Options / Filtering
* **Prompt:** "Query memory for all entities of type 'project' that have property status=active."
* **Expected Tool(s):** `query_memory`

#### Use Case MEM-05: Delete an Entity and Its Relations

* **Prompt:** "Delete entity 'Apollo-11' and all relations pointing to it."
* **Expected Tool(s):** `delete_entity`

#### Use Case MEM-06: Full Graph Inspection

* **Prompt:** "Inspect the entire memory graph: list all entities, their properties, and relations."
* **Expected Tool(s):** `inspect_memory`

#### Use Case MEM-07: Query Non-Existent Entity

* **Category:** Edge Case / Error Handling
* **Prompt:** "Get details for entity 'Entity-That-Does-Not-Exist-12345'."
* **Expected Tool(s):** `get_entity`
