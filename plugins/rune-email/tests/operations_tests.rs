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
fn test_empty_send_email_parameters() {
    let req_empty = ToolCallRequest {
        name: "send_email".to_string(),
        arguments: json!({}),
    };
    let res = execute_tool(req_empty);
    assert!(res.is_err());
}

#[test]
fn test_send_email_empty_strings_rejected() {
    let req = ToolCallRequest {
        name: "send_email".to_string(),
        arguments: json!({
            "email": "user@gmail.com",
            "password": "secret_password",
            "preset": "gmail",
            "to": "   ",
            "subject": "Test",
            "bodyText": "Hello"
        }),
    };
    let res = execute_tool(req);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Parameter 'to' cannot be empty"));

    let req_subj = ToolCallRequest {
        name: "send_email".to_string(),
        arguments: json!({
            "email": "user@gmail.com",
            "password": "secret_password",
            "preset": "gmail",
            "to": "recipient@example.com",
            "subject": "   ",
            "bodyText": "Hello"
        }),
    };
    let res_subj = execute_tool(req_subj);
    assert!(res_subj.is_err());
    assert!(
        res_subj
            .unwrap_err()
            .contains("Parameter 'subject' cannot be empty")
    );
}

#[test]
fn test_invalid_uid_rejected() {
    let req = ToolCallRequest {
        name: "read_message".to_string(),
        arguments: json!({
            "email": "user@gmail.com",
            "password": "secret_password",
            "preset": "gmail",
            "uid": 0
        }),
    };
    let res = execute_tool(req);
    assert!(res.is_err());
    assert!(
        res.unwrap_err()
            .contains("Parameter 'uid' must be a positive integer")
    );
}

#[test]
fn test_unknown_tool_routing() {
    let req = ToolCallRequest {
        name: "non_existent_tool".to_string(),
        arguments: json!({}),
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
    if std::env::var("CI").is_ok() {
        eprintln!("Skipping live test: Running in CI environment");
        return;
    }

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
    if std::env::var("CI").is_ok() {
        eprintln!("Skipping live test: Running in CI environment");
        return;
    }

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
    if std::env::var("CI").is_ok() {
        eprintln!("Skipping live test: Running in CI environment");
        return;
    }

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

    assert!(
        !messages.is_empty(),
        "Expected at least 1 message in INBOX if mailbox is not empty"
    );
}
