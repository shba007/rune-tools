use rune_email::operations::{execute_tool, resolve_account_config};
use rune_pdk::ToolCallRequest;
use serde_json::json;

#[test]
fn test_resolve_account_config_preset_hostinger() {
    let args = json!({
        "preset": "hostinger",
        "email": "user@mybusiness.com",
        "password": "secret_password"
    });
    let config = resolve_account_config(&args).unwrap();
    assert_eq!(config.imap_host, "imap.hostinger.com");
    assert_eq!(config.imap_port, 993);
    assert_eq!(config.smtp_host, "smtp.hostinger.com");
    assert_eq!(config.smtp_port, 465);
}

#[test]
fn test_resolve_account_config_preset_gmail() {
    let args = json!({
        "preset": "gmail",
        "email": "user@gmail.com",
        "password": "app_password_16_chars"
    });
    let config = resolve_account_config(&args).unwrap();
    assert_eq!(config.imap_host, "imap.gmail.com");
    assert_eq!(config.smtp_host, "smtp.gmail.com");
}

#[test]
fn test_resolve_account_config_custom() {
    let args = json!({
        "imapHost": "mail.customserver.net",
        "smtpHost": "mail.customserver.net",
        "email": "agent@customserver.net",
        "password": "pass"
    });
    let config = resolve_account_config(&args).unwrap();
    assert_eq!(config.imap_host, "mail.customserver.net");
    assert_eq!(config.smtp_host, "mail.customserver.net");
}

#[test]
fn test_empty_required_parameters() {
    let req_empty = ToolCallRequest {
        name: "send_email".to_string(),
        arguments: json!({}),
    };
    let res = execute_tool(req_empty);
    assert!(res.is_err());
}

#[test]
fn test_unknown_tool_routing() {
    let req = ToolCallRequest {
        name: "non_existent_tool".to_string(),
        arguments: json!({
            "email": "test@test.com",
            "password": "pass",
            "preset": "gmail"
        }),
    };
    let res = execute_tool(req);
    assert!(res.is_err());
    assert_eq!(res.unwrap_err(), "Unknown tool: non_existent_tool");
}

// =========================================================================
// Live E2E Operations (Driven by dotenvx CLI injection)
// =========================================================================

#[test]
fn test_live_verify_email_connection_e2e() {
    if std::env::var("EMAIL_USER").is_err() || std::env::var("EMAIL_PASSWORD").is_err() {
        eprintln!("Skipping live test: EMAIL_USER or EMAIL_PASSWORD not set in environment");
        return;
    }

    let req = ToolCallRequest {
        name: "verify_email_connection".to_string(),
        arguments: json!({}),
    };

    let res = execute_tool(req).expect("Failed to verify live email connection");
    assert_eq!(res["status"], "connected");
    assert_eq!(res["imap"]["authenticated"], true);
}

#[test]
fn test_live_list_mailboxes_e2e() {
    if std::env::var("EMAIL_USER").is_err() || std::env::var("EMAIL_PASSWORD").is_err() {
        return;
    }

    let req = ToolCallRequest {
        name: "list_mailboxes".to_string(),
        arguments: json!({}),
    };

    let res = execute_tool(req).expect("Failed to list mailboxes from live server");
    let mailboxes = res["mailboxes"].as_array().expect("Expected mailbox list");

    assert!(
        !mailboxes.is_empty(),
        "Server should return at least one mailbox"
    );
}

#[test]
fn test_live_print_last_emails_e2e() {
    if std::env::var("EMAIL_USER").is_err() || std::env::var("EMAIL_PASSWORD").is_err() {
        eprintln!("Skipping live test: EMAIL_USER or EMAIL_PASSWORD not set in environment");
        return;
    }

    let req = ToolCallRequest {
        name: "list_messages".to_string(),
        arguments: json!({
            "mailbox": "INBOX",
            "limit": 3,
            "page": 1,
            "unreadOnly": false
        }),
    };

    let res = execute_tool(req).expect("Failed to fetch messages from live server");
    let messages = res["messages"].as_array().expect("Expected messages array");

    println!("\n=== Last {} Emails in INBOX ===", messages.len());
    for (i, msg) in messages.iter().enumerate() {
        println!(
            "[{}] UID: {} | Date: {} | From: {} | Subject: {}",
            i + 1,
            msg["uid"],
            msg["date"].as_str().unwrap_or("N/A"),
            msg["from"].as_str().unwrap_or("Unknown"),
            msg["subject"].as_str().unwrap_or("(No Subject)")
        );
    }
    println!("======================================\n");

    assert!(
        !messages.is_empty(),
        "Expected at least 1 message in INBOX if mailbox is not empty"
    );
}
