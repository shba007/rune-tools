### `rune-ssh`

* **Description:** Secure remote server management via SSH execution and SFTP file transfers. Allows remote command execution, file uploads, and downloads to/from remote servers using SSH and SFTP protocols.

* **Tool Definitions:** `ssh_run_command`, `sftp_upload`, `sftp_download`

* **MCP Configuration:**

```json
{
  "mcpServers": {
    "rune-ssh": {
      "command": "rune",
      "args": [
        "run",
        "rune-ssh"
      ],
      "env": {
        "ALLOWED_DIR": "./test-dir"
      }
    }
  }
}
```

**Environment Variables:**

* `SSH_HOST`: Default hostname or IP address for SSH connections (e.g., "example.com:2222"). Used when host parameter is not provided in tool calls.
* `SSH_USER`: Default SSH username for authentication. Required for SSH connections unless overridden in tool parameters.
* `SSH_KEY_PATH`: Path to the private key file for SSH authentication. Must be accessible to the application.
* `ALLOWED_DIR`: Root directory boundary enforced for sandbox isolation. All file operations are restricted to this directory or its subdirectories (default: .).

#### Use Case SSH-01: 

* **Prompt:** "Execute the command 'ls -la /home/user' on server 'example.com' using SSH user 'admin' and key '/home/admin/id_rsa'."
* **Expected Tool(s):** `ssh_run_command`

#### Use Case SSH-02: 

* **Prompt:** "Upload the local file 'document.pdf' to server 'example.com' at path '/uploads/document.pdf'."
* **Expected Tool(s):** `sftp_upload`

#### Use Case SSH-03: 

* **Prompt:** "Download the remote file 'report.txt' from server 'example.com' to the local directory 'Downloads/'."
* **Expected Tool(s):** `sftp_download`

#### Use Case SSH-04: 

* **Prompt:** "Run the command 'ps aux' on server '192.168.1.100' using default SSH credentials."
* **Expected Tool(s):** `ssh_run_command`

#### Use Case SSH-05: 

* **Prompt:** "Upload the configuration file 'config.yaml' to server 'server1.example.com' at path '/etc/config/'."
* **Expected Tool(s):** `sftp_upload`

#### Use Case SSH-06: 

* **Prompt:** "Download the remote backup file 'backup.tar.gz' from server 'server2.example.com' to local machine."
* **Expected Tool(s):** `sftp_download`

#### Use Case SSH-07: 

* **Prompt:** "Execute multiple commands on remote server 'test.example.com': first 'whoami', then 'pwd'."
* **Expected Tool(s):** `ssh_run_command`

#### Use Case SSH-08: 

* **Prompt:** "Attempt to SSH connect to server 'malicious.com' using credentials from environment variables."
* **Expected Tool(s):** `ssh_run_command` (should succeed if credentials are valid)

#### Use Case SSH-09: 

* **Prompt:** "Upload a file to server with path traversal attempt 'server.com../../../etc/passwd'."
* **Expected Tool(s):** `sftp_upload` (should be rejected due to security boundary)

#### Use Case SSH-10: 

* **Prompt:** "Download a remote file from 'server.com' using explicit host, user, and key parameters."
* **Expected Tool(s):** `sftp_download`

#### Use Case SSH-11: 

* **Prompt:** "Execute a command on a server that requires a custom port: 'server.example.com:2222' with user 'user'."
* **Expected Tool(s):** `ssh_run_command`

#### Use Case SSH-12: 

* **Prompt:** "Upload a large file to remote server 'fileserver.example.com' with monitoring."
* **Expected Tool(s):** `sftp_upload`

#### Use Case SSH-13: 

* **Prompt:** "Download multiple files from remote server 'backup.server.com' in parallel."
* **Expected Tool(s):** `sftp_download`

#### Use Case SSH-14: 

* **Prompt:** "Attempt to use invalid SSH key path 'invalid_key.pem' for authentication."
* **Expected Tool(s):** `ssh_run_command` (should fail with authentication error)

#### Use Case SSH-15: 

* **Prompt:** "Execute a command on server 'server.example.com' without specifying host, relying on environment variables."
* **Expected Tool(s):** `ssh_run_command` (should succeed if SSH_HOST is set)

#### Use Case SSH-16: 

* **Prompt:** "Upload a file to remote server using custom SSH port and key."
* **Expected Tool(s):** `sftp_upload`

#### Use Case SSH-17: 

* **Prompt:** "Download a file from remote server and save to a specific local directory."
* **Expected Tool(s):** `sftp_download`

#### Use Case SSH-18: 

* **Prompt:** "Execute a command that outputs JSON data from remote server."
* **Expected Tool(s):** `ssh_run_command`

#### Use Case SSH-19: 

* **Prompt:** "Upload a configuration file with sensitive data to remote server."
* **Expected Tool(s):** `sftp_upload`

#### Use Case SSH-20: 

* **Prompt:** "Download a remote log file from server 'logs.example.com' for analysis."
* **Expected Tool(s):** `sftp_download`
