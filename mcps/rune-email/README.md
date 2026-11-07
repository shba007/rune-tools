### `rune-email`

* **Description:** Universal IMAP/SMTP email client: mailbox listing, search, full message parsing to Markdown, attachment download, send/reply/draft, flag management, and inter-mailbox moves.

* **Tool Definitions:** `verify_email_connection`, `list_mailboxes`, `list_messages`, `search_messages`, `read_message`, `download_attachment`, `send_email`, `reply_email`, `draft_email`, `manage_message_flags`, `move_message`

* **MCP Configuration:**

```json
{
  "mcpServers": {
    "rune-email": {
      "command": "rune",
      "args": [
        "run",
        "rune-email"
      ],
      "env": {
        "ALLOWED_DIR": "./test-dir"
      }
    }
  }
}
```

**Environment Variables:**

* `EMAIL_PRESET`: Vendor preset for auto-resolving IMAP/SMTP hosts and ports. Supported values: `gmail`, `hostinger`, `outlook`. Omit for custom server configuration (default: none).
* `EMAIL_USER`: Email address / username used for both IMAP and SMTP authentication. Also read from `IMAP_USER` or `SMTP_USER` as fallbacks.
* `EMAIL_PASSWORD`: Account password or App Password for authentication. Also read from `IMAP_PASSWORD` or `SMTP_PASSWORD` as fallbacks.
* `EMAIL_DISPLAY_NAME`: Display name appended to the From header on outgoing mail (optional).
* `OUTPUT_DIR`: Default directory where downloaded attachments are saved (default: `.`).
* `ALLOWED_DIR`: Root directory boundary enforced for sandbox isolation. All file operations are restricted to this directory or its subdirectories (default: .).

#### Use Case EML-01: Verify Mail Server Connectivity

* **Prompt:** "Check that my Gmail IMAP and SMTP connections are working."
* **Expected Tool(s):** `verify_email_connection`

#### Use Case EML-02: List All Mailboxes

* **Prompt:** "Show me all the folders and mailboxes available on my mail server."
* **Expected Tool(s):** `list_mailboxes`

#### Use Case EML-03: Fetch Latest Unread Messages

* **Prompt:** "List the 10 most recent unread emails in my inbox."
* **Expected Tool(s):** `list_messages`

#### Use Case EML-04: Search by Sender and Date

* **Prompt:** "Find all emails from 'alice@example.com' sent after 2026-01-01."
* **Expected Tool(s):** `search_messages`

#### Use Case EML-05: Read Full Email with Attachments Metadata

* **Prompt:** "Read the full content of email UID 4521 in my inbox, including all headers and attachment info."
* **Expected Tool(s):** `read_message`

#### Use Case EML-06: Download an Attachment to Disk

* **Prompt:** "Download the first attachment from email UID 4521 and save it to './test-dir/attachments'."
* **Expected Tool(s):** `download_attachment`

#### Use Case EML-07: Send a New Email with Attachment

* **Prompt:** "Send an email to 'bob@example.com' with subject 'Report' and body 'Q3 numbers attached', attaching the file './reports/q3.pdf'."
* **Expected Tool(s):** `send_email`

#### Use Case EML-08: Threaded Reply Preserving Headers

* **Prompt:** "Reply to email UID 4521 with 'Thanks, I will review this and get back to you.' Keep it in the same thread."
* **Expected Tool(s):** `reply_email`

#### Use Case EML-09: Save a Draft Without Sending

* **Prompt:** "Create a draft email to 'team@example.com' with subject 'Meeting Notes' and body 'Will share before Friday.' but don't send it yet."
* **Expected Tool(s):** `draft_email`

#### Use Case EML-10: Mark Message as Starred

* **Prompt:** "Star the email with UID 4521 in my inbox."
* **Expected Tool(s):** `manage_message_flags`

#### Use Case EML-11: Move Email to Archive

* **Prompt:** "Move email UID 300 from INBOX to the Archive folder."
* **Expected Tool(s):** `move_message`

#### Use Case EML-12: Invalid Credentials Error Handling

* **Prompt:** "List my inbox messages" (with an incorrect password configured).
* **Expected Tool(s):** `list_messages`

#### Use Case EML-13: Search by Subject Keyword

* **Prompt:** "Find all emails with 'Project' in the subject from the last month."
* **Expected Tool(s):** `search_messages`

#### Use Case EML-14: Read Multiple Messages

* **Prompt:** "Read the full content of emails with UIDs 4520, 4521, and 4522."
* **Expected Tool(s):** `read_message`

#### Use Case EML-15: Download All Attachments

* **Prompt:** "Download all attachments from the latest 5 messages in INBOX."
* **Expected Tool(s):** `download_attachment`

#### Use Case EML-16: Send Email with Multiple Recipients

* **Prompt:** "Send a notification to 'team@project.com', 'manager@project.com', and 'client@project.com' with subject 'Deadline Extension'."
* **Expected Tool(s):** `send_email`

#### Use Case EML-17: Draft Email with CC and BCC

* **Prompt:** "Create a draft email to 'colleague@project.com' with subject 'Follow-up' and CC 'supervisor@project.com', BCC 'secretary@project.com'."
* **Expected Tool(s):** `draft_email`

#### Use Case EML-18: Search by Date Range

* **Prompt:** "Find emails sent between 2026-01-01 and 2026-01-31 in the Sent folder."
* **Expected Tool(s):** `search_messages`

#### Use Case EML-19: Read with Attachment Filtering

* **Prompt:** "Read email with UID 4521 and only show messages with attachments."
* **Expected Tool(s):** `read_message`

#### Use Case EML-20: Move Multiple Emails

* **Prompt:** "Move all emails from INBOX to Trash that are older than 30 days."
* **Expected Tool(s):** `move_message`
