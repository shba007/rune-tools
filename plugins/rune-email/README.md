### `rune-email`

* **Description:** A universal email client MCP server supporting full IMAP and SMTP operations across Gmail, Outlook, Hostinger, and custom mail servers. Provides mailbox listing, paginated message retrieval, multi-criteria search, full message parsing to Markdown, attachment extraction, sending with attachments/CC/BCC, threaded replies preserving Message-ID headers, draft creation via IMAP APPEND, flag management, and inter-mailbox moves.

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
        "EMAIL_PRESET": "gmail",
        "EMAIL_USER": "user@gmail.com",
        "EMAIL_PASSWORD": "your-app-password",
        "OUTPUT_DIR": "./test-dir/email-out"
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

#### Use Case EML-01: Verify Mail Server Connectivity

* **Category:** Happy Path / Diagnostics
* **Prompt:** "Check that my Gmail IMAP and SMTP connections are working."
* **Expected Tool(s):** `verify_email_connection`

#### Use Case EML-02: List All Mailboxes

* **Category:** Happy Path
* **Prompt:** "Show me all the folders and mailboxes available on my mail server."
* **Expected Tool(s):** `list_mailboxes`

#### Use Case EML-03: Fetch Latest Unread Messages

* **Category:** Happy Path / Inbox
* **Prompt:** "List the 10 most recent unread emails in my inbox."
* **Expected Tool(s):** `list_messages`

#### Use Case EML-04: Search by Sender and Date

* **Category:** Granular Options / Search
* **Prompt:** "Find all emails from 'alice@example.com' sent after 2026-01-01."
* **Expected Tool(s):** `search_messages`

#### Use Case EML-05: Read Full Email with Attachments Metadata

* **Category:** Happy Path / Reading
* **Prompt:** "Read the full content of email UID 4521 in my inbox, including all headers and attachment info."
* **Expected Tool(s):** `read_message`

#### Use Case EML-06: Download an Attachment to Disk

* **Category:** Happy Path / Attachments
* **Prompt:** "Download the first attachment from email UID 4521 and save it to './test-dir/attachments'."
* **Expected Tool(s):** `download_attachment`

#### Use Case EML-07: Send a New Email with Attachment

* **Category:** Happy Path / Sending
* **Prompt:** "Send an email to 'bob@example.com' with subject 'Report' and body 'Q3 numbers attached', attaching the file './reports/q3.pdf'."
* **Expected Tool(s):** `send_email`

#### Use Case EML-08: Threaded Reply Preserving Headers

* **Category:** Happy Path / Replying
* **Prompt:** "Reply to email UID 4521 with 'Thanks, I will review this and get back to you.' Keep it in the same thread."
* **Expected Tool(s):** `reply_email`

#### Use Case EML-09: Save a Draft Without Sending

* **Category:** Happy Path / Drafting
* **Prompt:** "Create a draft email to 'team@example.com' with subject 'Meeting Notes' and body 'Will share before Friday.' but don't send it yet."
* **Expected Tool(s):** `draft_email`

#### Use Case EML-10: Mark Message as Starred

* **Category:** Happy Path / Flags
* **Prompt:** "Star the email with UID 4521 in my inbox."
* **Expected Tool(s):** `manage_message_flags`

#### Use Case EML-11: Move Email to Archive

* **Category:** Happy Path / Organization
* **Prompt:** "Move email UID 300 from INBOX to the Archive folder."
* **Expected Tool(s):** `move_message`

#### Use Case EML-12: Invalid Credentials Error Handling

* **Category:** Edge Case / Error Handling
* **Prompt:** "List my inbox messages" (with an incorrect password configured).
* **Expected Tool(s):** `list_messages`
