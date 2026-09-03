### `rune-filesystem`

* **Description:** A sandboxed filesystem manipulation and inspection MCP server that exposes line-based text reading, multimodal media/binary reading, atomic file writes, line-based editing with dry-run diffs, directory creation/listing/tree views, safe moves/renames, glob search, and metadata retrieval — all confined within an enforced `ALLOWED_DIR` boundary.

* **Tool Definitions:** `read_text_file`, `read_media_file`, `read_multiple_files`, `write_file`, `edit_file`, `create_directory`, `list_directory`, `list_directory_with_sizes`, `directory_tree`, `move_file`, `search_files`, `get_file_info`, `list_allowed_directories`

* **MCP Configuration:**

```json
{
  "mcpServers": {
    "rune-filesystem": {
      "command": "rune",
      "args": [
        "run",
        "rune-filesystem"
      ],
      "env": {
        "ALLOWED_DIR": "./test-dir"
      }
    }
  }
}
```

**Environment Variables:**

* `ALLOWED_DIR`: Root directory path enforced for filesystem sandbox containment. Relative paths in every tool call are resolved against this root; attempts to read, write, or traverse outside it (e.g., via `..`) are rejected (default: `.`).

#### Use Case FS-01: Read the Head of a Text File

* **Category:** Happy Path
* **Prompt:** "Read the first 50 lines of 'notes.txt' in my workspace."
* **Expected Tool(s):** `read_text_file`

#### Use Case FS-02: Page Through a Large Log File

* **Category:** Granular Options / Pagination
* **Prompt:** "Open 'app.log', skip the first 10,000 lines and read the next 500 lines from there."
* **Expected Tool(s):** `read_text_file`

#### Use Case FS-03: Read an Image for Vision Analysis

* **Category:** Happy Path / Multimodal
* **Prompt:** "Read 'screenshot.png' so I can analyze what's shown in the image."
* **Expected Tool(s):** `read_media_file`

#### Use Case FS-04: Batch Read Multiple Source Files

* **Category:** Happy Path / Multi-File
* **Prompt:** "Read the contents of 'src/main.rs', 'src/lib.rs', and 'Cargo.toml' all at once."
* **Expected Tool(s):** `read_multiple_files`

#### Use Case FS-05: Overwrite a File with New Content

* **Category:** Happy Path / Write
* **Prompt:** "Create or overwrite 'config.json' with the following JSON content: {\"debug\": true}."
* **Expected Tool(s):** `write_file`

#### Use Case FS-06: Line-Based Edit with Dry Run Preview

* **Category:** Granular Options / Safe Edit
* **Prompt:** "In 'src/lib.rs', replace the line 'let x = 1;' with 'let x = 42;', but only show me a git-style diff preview without saving."
* **Expected Tool(s):** `edit_file`

#### Use Case FS-07: Create Nested Directories

* **Category:** Happy Path / Directory Mgmt
* **Prompt:** "Create the directory structure './test-dir/assets/images/icons' including all parent folders."
* **Expected Tool(s):** `create_directory`

#### Use Case FS-08: Recursive Directory Tree with Exclusions

* **Category:** Granular Options / Filtering
* **Prompt:** "Show me a recursive tree of the project root, excluding anything matching '**/*.lock' and 'target/**'."
* **Expected Tool(s):** `directory_tree`

#### Use Case FS-09: Glob Search for Rust Files

* **Category:** Happy Path / Search
* **Prompt:** "Find every file ending in '.rs' under the plugins directory, ignoring test files."
* **Expected Tool(s):** `search_files`

#### Use Case FS-10: Safe Move / Rename

* **Category:** Happy Path / File Mgmt
* **Prompt:** "Rename 'draft.txt' to 'final_report.txt' in the same folder."
* **Expected Tool(s):** `move_file`

#### Use Case FS-11: Sandbox Boundary Traversal Rejection

* **Category:** Edge Case / Security Boundary
* **Prompt:** "Read the file at '../../etc/passwd' (a path that escapes the allowed directory)."
* **Expected Tool(s):** `read_text_file`

#### Use Case FS-12: Read a Non-Existent File Error Handling

* **Category:** Edge Case / Error Handling
* **Prompt:** "Read the contents of 'does_not_exist.txt'."
* **Expected Tool(s):** `read_text_file`
