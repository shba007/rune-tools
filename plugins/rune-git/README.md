### `rune-git`

* **Description:** A Git repository management MCP server exposing 30 core operations covering status, diff, staging, committing, branching, switching, merging, log inspection, remote management, fetching/pulling, and pushing — operating on a target repository path resolved from the environment.

* **Tool Definitions:** `git_status`, `git_diff`, `git_add`, `git_commit`, `git_branch_list`, `git_branch_create`, `git_checkout`, `git_merge`, `git_log`, `git_remote_add`, `git_remote_remove`, `git_fetch`, `git_pull`, `git_push`

* **MCP Configuration:**

```json
{
  "mcpServers": {
    "rune-git": {
      "command": "rune",
      "args": [
        "run",
        "rune-git"
      ],
      "env": {
        "REPO_PATH": "./test-dir/my-repo"
      }
    }
  }
}
```

**Environment Variables:**

* `REPO_PATH`: Path to the Git repository that operations target when no explicit path is supplied (default: current working directory).

#### Use Case GIT-01: Check Working Tree Status

* **Category:** Happy Path
* **Prompt:** "Show me the git status of the repository."
* **Expected Tool(s):** `git_status`

#### Use Case GIT-02: View Unstaged and Staged Changes

* **Category:** Happy Path / Diff
* **Prompt:** "Show the diff between my working tree and the index, including staged changes."
* **Expected Tool(s):** `git_diff`

#### Use Case GIT-03: Stage All Changes

* **Category:** Happy Path
* **Prompt:** "Stage all modified and new files in the repository."
* **Expected Tool(s):** `git_add`

#### Use Case GIT-04: Commit with Message

* **Category:** Happy Path
* **Prompt:** "Commit the staged changes with the message 'feat: add user authentication'."
* **Expected Tool(s):** `git_commit`

#### Use Case GIT-05: Create and Switch to a New Branch

* **Category:** Multi-Tool Chain
* **Prompt:** "Create a new branch called 'feature/login' and switch to it."
* **Expected Tool(s):** `git_branch_create`, `git_checkout`

#### Use Case GIT-06: Inspect Commit History

* **Category:** Happy Path / Log
* **Prompt:** "Show me the last 15 commits on the current branch with full message details."
* **Expected Tool(s):** `git_log`

#### Use Case GIT-07: Add a Remote and Fetch

* **Category:** Multi-Tool Chain / Remote
* **Prompt:** "Add an upstream remote pointing to 'https://github.com/acme/repo.git' and then fetch from it."
* **Expected Tool(s):** `git_remote_add`, `git_fetch`

#### Use Case GIT-08: Pull Latest Changes

* **Category:** Happy Path / Remote
* **Prompt:** "Pull the latest changes from the origin remote into my current branch."
* **Expected Tool(s):** `git_pull`

#### Use Case GIT-09: Push to Remote

* **Category:** Happy Path / Remote
* **Prompt:** "Push my current branch to the origin remote."
* **Expected Tool(s):** `git_push`

#### Use Case GIT-10: Merge a Feature Branch

* **Category:** Multi-Tool Chain
* **Prompt:** "Merge the 'feature/login' branch into 'main'."
* **Expected Tool(s):** `git_merge`

#### Use Case GIT-11: Commit with No Changes Error Handling

* **Category:** Edge Case / Error Handling
* **Prompt:** "Commit the staged changes with message 'update' even though nothing has been staged."
* **Expected Tool(s):** `git_commit`
