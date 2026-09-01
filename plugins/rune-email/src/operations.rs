use rune_pdk::ToolCallRequest;
use serde_json::Value;

// =========================================================================
// WASM32 Target Implementation (Host Subprocess Bridge)
// =========================================================================

#[cfg(target_arch = "wasm32")]
use crate::types::{CmdExecRequest, CmdExecResponse};

#[cfg(target_arch = "wasm32")]
#[extism_pdk::host_fn("extism:host/user")]
extern "ExtismHost" {
    fn host_cmd_exec(input: String) -> String;
}

#[cfg(target_arch = "wasm32")]
pub fn execute_tool(request: ToolCallRequest) -> Result<Value, String> {
    let payload_str =
        serde_json::to_string(&request).map_err(|e| format!("Serialization error: {}", e))?;

    let cmd_req = CmdExecRequest {
        program: "rune-email-native".to_string(),
        args: vec!["--exec".to_string(), payload_str],
        cwd: None,
    };

    let raw_req = serde_json::to_string(&cmd_req).map_err(|e| e.to_string())?;

    let raw_resp =
        unsafe { host_cmd_exec(raw_req) }.map_err(|e| format!("Host execution failed: {:?}", e))?;

    let resp: CmdExecResponse = serde_json::from_str(&raw_resp)
        .map_err(|e| format!("Failed to parse host response: {}", e))?;

    if !resp.success && resp.stdout.trim().is_empty() {
        return Err(if !resp.stderr.is_empty() {
            resp.stderr
        } else {
            "rune-email-native exited with failure".to_string()
        });
    }

    let parsed_val: Value = serde_json::from_str(&resp.stdout).map_err(|e| {
        format!(
            "Failed to parse output JSON: {} (stdout: {})",
            e, resp.stdout
        )
    })?;

    if let Some(err) = parsed_val.get("error").and_then(Value::as_str) {
        return Err(err.to_string());
    }

    Ok(parsed_val)
}

// =========================================================================
// Native Target Implementation (IMAP/SMTP Direct Networking)
// =========================================================================

#[cfg(not(target_arch = "wasm32"))]
use crate::types::{AttachmentInfo, EmailAccountConfig, MessageHeaderSummary};
#[cfg(not(target_arch = "wasm32"))]
use imap::Session;
#[cfg(not(target_arch = "wasm32"))]
use lettre::message::{MultiPart, SinglePart, header::ContentType};
#[cfg(not(target_arch = "wasm32"))]
use lettre::transport::smtp::authentication::Credentials;
#[cfg(not(target_arch = "wasm32"))]
use lettre::{Message, SmtpTransport, Transport};
#[cfg(not(target_arch = "wasm32"))]
use mail_parser::MimeHeaders;
#[cfg(not(target_arch = "wasm32"))]
use native_tls::TlsConnector;
#[cfg(not(target_arch = "wasm32"))]
use serde_json::json;
#[cfg(not(target_arch = "wasm32"))]
use std::fs;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

#[cfg(not(target_arch = "wasm32"))]
fn get_str_arg(args: &Value, camel: &str, snake: &str) -> Option<String> {
    if let Some(val) = args
        .get(camel)
        .or_else(|| args.get(snake))
        .and_then(Value::as_str)
    {
        return Some(val.to_string());
    }
    let env_snake = snake.to_ascii_uppercase();
    let env_camel = camel.to_ascii_uppercase();
    std::env::var(&env_snake)
        .or_else(|_| std::env::var(&env_camel))
        .ok()
}

#[cfg(not(target_arch = "wasm32"))]
fn get_u64_arg(args: &Value, camel: &str, snake: &str) -> Option<u64> {
    if let Some(val) = args
        .get(camel)
        .or_else(|| args.get(snake))
        .and_then(Value::as_u64)
    {
        return Some(val);
    }
    let env_snake = snake.to_ascii_uppercase();
    let env_camel = camel.to_ascii_uppercase();
    std::env::var(&env_snake)
        .or_else(|_| std::env::var(&env_camel))
        .ok()
        .and_then(|v| v.parse().ok())
}

#[cfg(not(target_arch = "wasm32"))]
fn get_bool_arg(args: &Value, camel: &str, snake: &str, default: bool) -> bool {
    if let Some(val) = args
        .get(camel)
        .or_else(|| args.get(snake))
        .and_then(Value::as_bool)
    {
        return val;
    }
    let env_snake = snake.to_ascii_uppercase();
    let env_camel = camel.to_ascii_uppercase();
    std::env::var(&env_snake)
        .or_else(|_| std::env::var(&env_camel))
        .ok()
        .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
        .unwrap_or(default)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn resolve_dir(dir_param: Option<&str>) -> String {
    let explicit = dir_param.map(ToString::to_string).or_else(|| {
        std::env::var("OUTPUT_DIRECTORY")
            .or_else(|_| std::env::var("OUTPUT_DIR"))
            .or_else(|_| std::env::var("ALLOWED_DIR"))
            .ok()
    });
    explicit.unwrap_or_else(|| ".".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn resolve_account_config(args: &Value) -> Result<EmailAccountConfig, String> {
    let preset = get_str_arg(args, "preset", "preset")
        .or_else(|| std::env::var("EMAIL_PRESET").ok())
        .map(|s| s.to_ascii_lowercase());

    let email = get_str_arg(args, "email", "email")
        .or_else(|| std::env::var("EMAIL_USER").ok())
        .or_else(|| std::env::var("IMAP_USER").ok())
        .or_else(|| std::env::var("SMTP_USER").ok())
        .ok_or_else(|| "Missing required email address/username".to_string())?;

    let pass = get_str_arg(args, "password", "password")
        .or_else(|| std::env::var("EMAIL_PASSWORD").ok())
        .or_else(|| std::env::var("IMAP_PASSWORD").ok())
        .or_else(|| std::env::var("SMTP_PASSWORD").ok())
        .ok_or_else(|| "Missing required email password".to_string())?;

    let display_name = get_str_arg(args, "displayName", "display_name")
        .or_else(|| std::env::var("EMAIL_DISPLAY_NAME").ok());

    let (def_imap_host, def_imap_port, def_smtp_host, def_smtp_port) = match preset.as_deref() {
        Some("gmail") => ("imap.gmail.com", 993, "smtp.gmail.com", 465),
        Some("hostinger") => ("imap.hostinger.com", 993, "smtp.hostinger.com", 465),
        Some("outlook") => ("outlook.office365.com", 993, "smtp.office365.com", 587),
        _ => ("", 993, "", 465),
    };

    let imap_host =
        get_str_arg(args, "imapHost", "imap_host").unwrap_or_else(|| def_imap_host.to_string());
    let smtp_host =
        get_str_arg(args, "smtpHost", "smtp_host").unwrap_or_else(|| def_smtp_host.to_string());

    if imap_host.is_empty() {
        return Err("IMAP host is not configured (specify preset or imapHost)".to_string());
    }
    if smtp_host.is_empty() {
        return Err("SMTP host is not configured (specify preset or smtpHost)".to_string());
    }

    let imap_port =
        get_u64_arg(args, "imapPort", "imap_port").unwrap_or(def_imap_port as u64) as u16;
    let smtp_port =
        get_u64_arg(args, "smtpPort", "smtp_port").unwrap_or(def_smtp_port as u64) as u16;

    let imap_tls = get_bool_arg(args, "imapTls", "imap_tls", true);
    let smtp_tls = get_bool_arg(args, "smtpTls", "smtp_tls", true);

    Ok(EmailAccountConfig {
        preset,
        imap_host,
        imap_port,
        imap_user: email.clone(),
        imap_pass: pass.clone(),
        imap_tls,
        smtp_host,
        smtp_port,
        smtp_user: email.clone(),
        smtp_pass: pass,
        smtp_tls,
        display_name,
        from_email: email,
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn connect_imap(
    config: &EmailAccountConfig,
) -> Result<Session<native_tls::TlsStream<std::net::TcpStream>>, String> {
    let socket = std::net::TcpStream::connect((config.imap_host.as_str(), config.imap_port))
        .map_err(|e| {
            format!(
                "TCP connection to {}:{} failed: {}",
                config.imap_host, config.imap_port, e
            )
        })?;

    let tls = TlsConnector::builder()
        .build()
        .map_err(|e| format!("TLS init error: {}", e))?;

    let tls_stream = tls
        .connect(&config.imap_host, socket)
        .map_err(|e| format!("TLS handshake error with {}: {}", config.imap_host, e))?;

    let client = imap::Client::new(tls_stream);

    let session = client
        .login(&config.imap_user, &config.imap_pass)
        .map_err(|(e, _)| {
            format!(
                "IMAP authentication failed for {}: {:?}",
                config.imap_user, e
            )
        })?;
    Ok(session)
}

#[cfg(not(target_arch = "wasm32"))]
fn detect_sent_mailbox(
    session: &mut Session<native_tls::TlsStream<std::net::TcpStream>>,
    preset: Option<&str>,
) -> String {
    if let Some("gmail") = preset {
        return "[Gmail]/Sent Mail".to_string();
    }
    if let Ok(mailboxes) = session.list(None, Some("*")) {
        for mb in mailboxes.iter() {
            let name = mb.name();
            if name.eq_ignore_ascii_case("Sent")
                || name.eq_ignore_ascii_case("INBOX.Sent")
                || name.eq_ignore_ascii_case("Sent Items")
                || name.eq_ignore_ascii_case("Sent Messages")
            {
                return name.to_string();
            }
        }
    }
    "Sent".to_string()
}

#[cfg(not(target_arch = "wasm32"))]
fn detect_drafts_mailbox(
    session: &mut Session<native_tls::TlsStream<std::net::TcpStream>>,
    preset: Option<&str>,
) -> String {
    if let Some("gmail") = preset {
        return "[Gmail]/Drafts".to_string();
    }
    if let Ok(mailboxes) = session.list(None, Some("*")) {
        for mb in mailboxes.iter() {
            let name = mb.name();
            if name.eq_ignore_ascii_case("Drafts")
                || name.eq_ignore_ascii_case("INBOX.Drafts")
                || name.eq_ignore_ascii_case("Draft")
            {
                return name.to_string();
            }
        }
    }
    "Drafts".to_string()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn execute_tool(request: ToolCallRequest) -> Result<Value, String> {
    let config = resolve_account_config(&request.arguments)?;

    match request.name.as_str() {
        "verify_email_connection" => {
            let mut imap_session = connect_imap(&config)?;
            let mailboxes = imap_session
                .list(None, Some("*"))
                .map_err(|e| format!("Failed to list mailboxes: {}", e))?;
            let mb_count = mailboxes.len();
            let _ = imap_session.logout();

            let creds = Credentials::new(config.smtp_user.clone(), config.smtp_pass.clone());
            let transport = SmtpTransport::relay(&config.smtp_host)
                .map_err(|e| format!("SMTP relay configuration error: {}", e))?
                .port(config.smtp_port)
                .credentials(creds)
                .build();
            let smtp_tested = transport.test_connection().unwrap_or(true);

            Ok(json!({
                "status": "connected",
                "account": config.from_email,
                "imap": { "host": config.imap_host, "port": config.imap_port, "authenticated": true, "mailbox_count": mb_count },
                "smtp": { "host": config.smtp_host, "port": config.smtp_port, "connected": smtp_tested }
            }))
        }

        "list_mailboxes" => {
            let mut imap_session = connect_imap(&config)?;
            let list = imap_session
                .list(None, Some("*"))
                .map_err(|e| format!("IMAP list error: {}", e))?;

            let result: Vec<Value> = list.iter().map(|mb| {
                json!({
                    "name": mb.name(),
                    "delimiter": mb.delimiter(),
                    "attributes": mb.attributes().iter().map(|a| format!("{:?}", a)).collect::<Vec<_>>()
                })
            }).collect();

            let _ = imap_session.logout();
            Ok(json!({ "mailboxes": result }))
        }

        "list_messages" => {
            let mailbox = get_str_arg(&request.arguments, "mailbox", "mailbox")
                .unwrap_or_else(|| "INBOX".to_string());
            let limit = get_u64_arg(&request.arguments, "limit", "limit").unwrap_or(20) as usize;
            let page = get_u64_arg(&request.arguments, "page", "page").unwrap_or(1) as usize;
            let unread_only = get_bool_arg(&request.arguments, "unreadOnly", "unread_only", false);

            let mut imap_session = connect_imap(&config)?;
            imap_session
                .select(&mailbox)
                .map_err(|e| format!("Failed to select mailbox '{}': {}", mailbox, e))?;

            let search_query = if unread_only { "UNSEEN" } else { "ALL" };
            let uids = imap_session
                .uid_search(search_query)
                .map_err(|e| format!("Search error: {}", e))?;
            let mut uid_list: Vec<u32> = uids.into_iter().collect();
            uid_list.sort_unstable_by(|a, b| b.cmp(a));

            let total_found = uid_list.len();
            let start_idx = (page.saturating_sub(1)) * limit;
            let paged_uids: Vec<u32> = uid_list.into_iter().skip(start_idx).take(limit).collect();

            let mut summaries: Vec<MessageHeaderSummary> = Vec::new();

            if !paged_uids.is_empty() {
                let seq_set = paged_uids
                    .iter()
                    .map(|u| u.to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                let messages = imap_session
                    .uid_fetch(&seq_set, "(UID FLAGS RFC822.SIZE RFC822.HEADER)")
                    .map_err(|e| format!("UID fetch error: {}", e))?;

                for msg in messages.iter() {
                    let uid = msg.uid.unwrap_or(0);
                    let size = msg.size.unwrap_or(0);
                    let is_read = msg
                        .flags()
                        .iter()
                        .any(|f| matches!(f, imap::types::Flag::Seen));
                    let is_flagged = msg
                        .flags()
                        .iter()
                        .any(|f| matches!(f, imap::types::Flag::Flagged));

                    let mut subject = String::new();
                    let mut from = String::new();
                    let mut to = Vec::new();
                    let mut date_str = None;
                    let mut has_attachments = false;

                    if let Some(header_bytes) = msg.header() {
                        if let Some(parsed) =
                            mail_parser::MessageParser::default().parse(header_bytes)
                        {
                            subject = parsed.subject().unwrap_or("(No Subject)").to_string();
                            from = parsed
                                .from()
                                .and_then(|f| f.first())
                                .map(|a| {
                                    if let Some(name) = a.name() {
                                        format!("{} <{}>", name, a.address().unwrap_or(""))
                                    } else {
                                        a.address().unwrap_or("").to_string()
                                    }
                                })
                                .unwrap_or_default();

                            if let Some(to_addrs) = parsed.to() {
                                for a in to_addrs.iter() {
                                    to.push(a.address().unwrap_or("").to_string());
                                }
                            }
                            date_str = parsed.date().map(|d| d.to_rfc3339());
                            has_attachments = parsed.attachment_count() > 0;
                        }
                    }

                    summaries.push(MessageHeaderSummary {
                        uid,
                        subject,
                        from,
                        to,
                        date: date_str,
                        is_read,
                        is_flagged,
                        has_attachments,
                        size_bytes: size,
                    });
                }
            }

            let _ = imap_session.logout();
            Ok(json!({
                "mailbox": mailbox,
                "total_messages": total_found,
                "page": page,
                "limit": limit,
                "messages": summaries
            }))
        }

        "search_messages" => {
            let mailbox = get_str_arg(&request.arguments, "mailbox", "mailbox")
                .unwrap_or_else(|| "INBOX".to_string());
            let query_kw = get_str_arg(&request.arguments, "query", "query");
            let from_filter = get_str_arg(&request.arguments, "from", "from");
            let to_filter = get_str_arg(&request.arguments, "to", "to");
            let subject_filter = get_str_arg(&request.arguments, "subject", "subject");
            let since_date = get_str_arg(&request.arguments, "sinceDate", "since_date");
            let limit = get_u64_arg(&request.arguments, "limit", "limit").unwrap_or(20) as usize;

            let mut imap_session = connect_imap(&config)?;
            imap_session
                .select(&mailbox)
                .map_err(|e| format!("Failed to select mailbox '{}': {}", mailbox, e))?;

            let mut criteria = Vec::new();
            if let Some(kw) = query_kw {
                criteria.push(format!("TEXT \"{}\"", kw));
            }
            if let Some(f) = from_filter {
                criteria.push(format!("FROM \"{}\"", f));
            }
            if let Some(t) = to_filter {
                criteria.push(format!("TO \"{}\"", t));
            }
            if let Some(s) = subject_filter {
                criteria.push(format!("SUBJECT \"{}\"", s));
            }
            if let Some(d) = since_date {
                criteria.push(format!("SINCE \"{}\"", d));
            }

            let search_str = if criteria.is_empty() {
                "ALL".to_string()
            } else {
                criteria.join(" ")
            };

            let uids = imap_session
                .uid_search(&search_str)
                .map_err(|e| format!("Search error: {}", e))?;
            let mut uid_list: Vec<u32> = uids.into_iter().collect();
            uid_list.sort_unstable_by(|a, b| b.cmp(a));

            let total_found = uid_list.len();
            let paged_uids: Vec<u32> = uid_list.into_iter().take(limit).collect();
            let mut summaries: Vec<MessageHeaderSummary> = Vec::new();

            if !paged_uids.is_empty() {
                let seq_set = paged_uids
                    .iter()
                    .map(|u| u.to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                let messages = imap_session
                    .uid_fetch(&seq_set, "(UID FLAGS RFC822.SIZE RFC822.HEADER)")
                    .map_err(|e| format!("UID fetch error: {}", e))?;

                for msg in messages.iter() {
                    let uid = msg.uid.unwrap_or(0);
                    let size = msg.size.unwrap_or(0);
                    let is_read = msg
                        .flags()
                        .iter()
                        .any(|f| matches!(f, imap::types::Flag::Seen));
                    let is_flagged = msg
                        .flags()
                        .iter()
                        .any(|f| matches!(f, imap::types::Flag::Flagged));

                    let mut subject = String::new();
                    let mut from = String::new();
                    let mut to = Vec::new();
                    let mut date_str = None;
                    let mut has_attachments = false;

                    if let Some(header_bytes) = msg.header() {
                        if let Some(parsed) =
                            mail_parser::MessageParser::default().parse(header_bytes)
                        {
                            subject = parsed.subject().unwrap_or("(No Subject)").to_string();
                            from = parsed
                                .from()
                                .and_then(|f| f.first())
                                .map(|a| {
                                    if let Some(name) = a.name() {
                                        format!("{} <{}>", name, a.address().unwrap_or(""))
                                    } else {
                                        a.address().unwrap_or("").to_string()
                                    }
                                })
                                .unwrap_or_default();

                            if let Some(to_addrs) = parsed.to() {
                                for a in to_addrs.iter() {
                                    to.push(a.address().unwrap_or("").to_string());
                                }
                            }
                            date_str = parsed.date().map(|d| d.to_rfc3339());
                            has_attachments = parsed.attachment_count() > 0;
                        }
                    }

                    summaries.push(MessageHeaderSummary {
                        uid,
                        subject,
                        from,
                        to,
                        date: date_str,
                        is_read,
                        is_flagged,
                        has_attachments,
                        size_bytes: size,
                    });
                }
            }

            let _ = imap_session.logout();
            Ok(json!({
                "mailbox": mailbox,
                "search_criteria": search_str,
                "total_matches": total_found,
                "limit": limit,
                "messages": summaries
            }))
        }

        "read_message" => {
            let uid = get_u64_arg(&request.arguments, "uid", "uid")
                .ok_or_else(|| "Missing 'uid' parameter".to_string())? as u32;
            let mailbox = get_str_arg(&request.arguments, "mailbox", "mailbox")
                .unwrap_or_else(|| "INBOX".to_string());
            let mark_as_read = get_bool_arg(&request.arguments, "markAsRead", "mark_as_read", true);

            let mut imap_session = connect_imap(&config)?;
            imap_session
                .select(&mailbox)
                .map_err(|e| format!("Select mailbox error: {}", e))?;

            let messages = imap_session
                .uid_fetch(uid.to_string(), "RFC822")
                .map_err(|e| format!("Failed to fetch message UID {}: {}", uid, e))?;

            let raw_msg = messages
                .iter()
                .next()
                .ok_or_else(|| format!("Message UID {} not found in {}", uid, mailbox))?;
            let body = raw_msg
                .body()
                .ok_or_else(|| "Failed to read RFC822 message body".to_string())?;

            let parsed = mail_parser::MessageParser::default()
                .parse(body)
                .ok_or_else(|| "Failed to parse RFC822 MIME structure".to_string())?;

            let subject = parsed.subject().unwrap_or("(No Subject)").to_string();
            let from = parsed
                .from()
                .and_then(|f| f.first())
                .map(|a| a.address().unwrap_or(""))
                .unwrap_or("")
                .to_string();
            let date = parsed.date().map(|d| d.to_rfc3339());
            let message_id = parsed.message_id().unwrap_or("").to_string();

            let body_text = parsed.body_text(0).map(|s| s.to_string());
            let body_html = parsed.body_html(0).map(|s| s.to_string());

            let mut attachments: Vec<AttachmentInfo> = Vec::new();
            for (idx, att) in parsed.attachments().enumerate() {
                attachments.push(AttachmentInfo {
                    filename: att
                        .attachment_name()
                        .unwrap_or("unnamed_attachment")
                        .to_string(),
                    content_type: att
                        .content_type()
                        .map(|c| c.ctype())
                        .unwrap_or("application/octet-stream")
                        .to_string(),
                    size_bytes: att.contents().len(),
                    attachment_index: idx,
                });
            }

            if mark_as_read {
                let _ = imap_session.uid_store(uid.to_string(), "+FLAGS (\\Seen)");
            }

            let _ = imap_session.logout();

            Ok(json!({
                "uid": uid,
                "mailbox": mailbox,
                "subject": subject,
                "from": from,
                "date": date,
                "message_id": message_id,
                "body_text": body_text,
                "body_html": body_html,
                "attachments": attachments
            }))
        }

        "download_attachment" => {
            let uid = get_u64_arg(&request.arguments, "uid", "uid")
                .ok_or_else(|| "Missing 'uid' parameter".to_string())? as u32;
            let mailbox = get_str_arg(&request.arguments, "mailbox", "mailbox")
                .unwrap_or_else(|| "INBOX".to_string());
            let attachment_idx =
                get_u64_arg(&request.arguments, "attachmentIndex", "attachment_index")
                    .map(|v| v as usize);
            let target_name = get_str_arg(&request.arguments, "filename", "filename");

            let out_dir_param =
                get_str_arg(&request.arguments, "outputDirectory", "output_directory");
            let out_dir = resolve_dir(out_dir_param.as_deref());
            let _ = fs::create_dir_all(&out_dir);

            let mut imap_session = connect_imap(&config)?;
            imap_session
                .select(&mailbox)
                .map_err(|e| format!("Select mailbox error: {}", e))?;

            let messages = imap_session
                .uid_fetch(uid.to_string(), "RFC822")
                .map_err(|e| format!("Fetch error: {}", e))?;
            let raw_msg = messages
                .iter()
                .next()
                .ok_or_else(|| format!("Message UID {} not found", uid))?;
            let body = raw_msg.body().ok_or_else(|| "Empty body".to_string())?;

            let parsed = mail_parser::MessageParser::default()
                .parse(body)
                .ok_or_else(|| "Failed to parse MIME structure".to_string())?;

            let mut saved_files = Vec::new();

            for (idx, att) in parsed.attachments().enumerate() {
                let name = att.attachment_name().unwrap_or("attachment").to_string();
                let should_save = if let Some(target_idx) = attachment_idx {
                    target_idx == idx
                } else if let Some(ref req_name) = target_name {
                    name.eq_ignore_ascii_case(req_name)
                } else {
                    true
                };

                if should_save {
                    let out_path = PathBuf::from(&out_dir).join(&name);
                    fs::write(&out_path, att.contents()).map_err(|e| {
                        format!(
                            "Failed to write attachment to {}: {}",
                            out_path.display(),
                            e
                        )
                    })?;
                    saved_files.push(out_path.to_string_lossy().to_string());
                }
            }

            let _ = imap_session.logout();

            if saved_files.is_empty() {
                return Err("No matching attachments found to download".to_string());
            }

            Ok(json!({
                "status": "success",
                "saved_files": saved_files
            }))
        }

        "send_email" => {
            let to_str = get_str_arg(&request.arguments, "to", "to")
                .ok_or_else(|| "Missing 'to' parameter".to_string())?;
            if to_str.trim().is_empty() {
                return Err("Parameter 'to' cannot be empty".to_string());
            }
            let subject = get_str_arg(&request.arguments, "subject", "subject")
                .ok_or_else(|| "Missing 'subject' parameter".to_string())?;
            let body_text = get_str_arg(&request.arguments, "bodyText", "body_text")
                .ok_or_else(|| "Missing 'bodyText' parameter".to_string())?;
            let body_html = get_str_arg(&request.arguments, "bodyHtml", "body_html");

            let mut email_builder = Message::builder()
                .from(
                    config
                        .from_email
                        .parse()
                        .map_err(|e| format!("Invalid from address: {}", e))?,
                )
                .subject(subject);

            for to_addr in to_str.split(',') {
                let trimmed = to_addr.trim();
                if !trimmed.is_empty() {
                    email_builder = email_builder.to(trimmed
                        .parse()
                        .map_err(|e| format!("Invalid recipient '{}': {}", trimmed, e))?);
                }
            }

            if let Some(cc_str) = get_str_arg(&request.arguments, "cc", "cc") {
                for cc in cc_str.split(',') {
                    let trimmed = cc.trim();
                    if !trimmed.is_empty() {
                        email_builder = email_builder.cc(trimmed
                            .parse()
                            .map_err(|e| format!("Invalid CC '{}': {}", trimmed, e))?);
                    }
                }
            }

            let multipart = if let Some(html) = body_html {
                MultiPart::alternative()
                    .singlepart(
                        SinglePart::builder()
                            .header(ContentType::TEXT_PLAIN)
                            .body(body_text),
                    )
                    .singlepart(
                        SinglePart::builder()
                            .header(ContentType::TEXT_HTML)
                            .body(html),
                    )
            } else {
                MultiPart::alternative().singlepart(
                    SinglePart::builder()
                        .header(ContentType::TEXT_PLAIN)
                        .body(body_text),
                )
            };

            let email_msg = email_builder
                .multipart(multipart)
                .map_err(|e| format!("Failed to build MIME message: {}", e))?;

            let creds = Credentials::new(config.smtp_user.clone(), config.smtp_pass.clone());
            let transport = SmtpTransport::relay(&config.smtp_host)
                .map_err(|e| format!("SMTP relay configuration error: {}", e))?
                .port(config.smtp_port)
                .credentials(creds)
                .build();

            transport
                .send(&email_msg)
                .map_err(|e| format!("SMTP dispatch failed: {}", e))?;

            if config.preset.as_deref() != Some("gmail") {
                if let Ok(mut imap_session) = connect_imap(&config) {
                    let sent_box = detect_sent_mailbox(&mut imap_session, config.preset.as_deref());
                    let raw_bytes = email_msg.formatted();
                    let _ = imap_session.append(&sent_box, &raw_bytes);
                    let _ = imap_session.logout();
                }
            }

            Ok(json!({ "status": "sent", "to": to_str }))
        }

        "reply_email" => {
            let original_uid = get_u64_arg(&request.arguments, "originalUid", "original_uid")
                .ok_or_else(|| "Missing 'originalUid' parameter".to_string())?
                as u32;
            let mailbox = get_str_arg(&request.arguments, "mailbox", "mailbox")
                .unwrap_or_else(|| "INBOX".to_string());
            let reply_body = get_str_arg(&request.arguments, "replyBody", "reply_body")
                .ok_or_else(|| "Missing 'replyBody' parameter".to_string())?;
            if reply_body.trim().is_empty() {
                return Err("Parameter 'replyBody' cannot be empty".to_string());
            }
            let reply_all = get_bool_arg(&request.arguments, "replyAll", "reply_all", false);

            let mut imap_session = connect_imap(&config)?;
            imap_session
                .select(&mailbox)
                .map_err(|e| format!("Select mailbox error: {}", e))?;

            let messages = imap_session
                .uid_fetch(original_uid.to_string(), "RFC822")
                .map_err(|e| format!("Fetch error: {}", e))?;
            let raw_msg = messages
                .iter()
                .next()
                .ok_or_else(|| format!("Message UID {} not found", original_uid))?;
            let body = raw_msg.body().ok_or_else(|| "Empty body".to_string())?;

            let parsed = mail_parser::MessageParser::default()
                .parse(body)
                .ok_or_else(|| "Failed to parse original message MIME".to_string())?;

            let orig_msg_id = parsed.message_id().unwrap_or("").to_string();
            let orig_from = parsed
                .from()
                .and_then(|f| f.first())
                .map(|a| a.address().unwrap_or(""))
                .unwrap_or("");
            let orig_subject = parsed.subject().unwrap_or("");
            let reply_subject = if orig_subject.to_ascii_lowercase().starts_with("re:") {
                orig_subject.to_string()
            } else {
                format!("Re: {}", orig_subject)
            };

            let mut email_builder = Message::builder()
                .from(
                    config
                        .from_email
                        .parse()
                        .map_err(|e| format!("From address error: {}", e))?,
                )
                .to(orig_from
                    .parse()
                    .map_err(|e| format!("Recipient address error: {}", e))?)
                .subject(reply_subject);

            if !orig_msg_id.is_empty() {
                email_builder = email_builder
                    .in_reply_to(orig_msg_id.clone())
                    .references(orig_msg_id.clone());
            }

            if reply_all {
                if let Some(to_addrs) = parsed.to() {
                    for a in to_addrs.iter() {
                        if let Some(addr) = a.address() {
                            if addr != config.from_email && addr != orig_from {
                                if let Ok(parsed_addr) = addr.parse() {
                                    email_builder = email_builder.cc(parsed_addr);
                                }
                            }
                        }
                    }
                }
            }

            let reply_msg = email_builder
                .body(reply_body)
                .map_err(|e| format!("Failed to build reply message: {}", e))?;

            let creds = Credentials::new(config.smtp_user.clone(), config.smtp_pass.clone());
            let transport = SmtpTransport::relay(&config.smtp_host)
                .map_err(|e| format!("SMTP relay configuration error: {}", e))?
                .port(config.smtp_port)
                .credentials(creds)
                .build();

            transport
                .send(&reply_msg)
                .map_err(|e| format!("SMTP reply failed: {}", e))?;

            if config.preset.as_deref() != Some("gmail") {
                let sent_box = detect_sent_mailbox(&mut imap_session, config.preset.as_deref());
                let raw_bytes = reply_msg.formatted();
                let _ = imap_session.append(&sent_box, &raw_bytes);
            }

            let _ = imap_session.logout();

            Ok(json!({ "status": "replied", "in_reply_to": orig_msg_id, "to": orig_from }))
        }

        "draft_email" => {
            let to_str = get_str_arg(&request.arguments, "to", "to")
                .ok_or_else(|| "Missing 'to' parameter".to_string())?;
            if to_str.trim().is_empty() {
                return Err("Parameter 'to' cannot be empty".to_string());
            }
            let subject = get_str_arg(&request.arguments, "subject", "subject")
                .ok_or_else(|| "Missing 'subject' parameter".to_string())?;
            let body_text = get_str_arg(&request.arguments, "bodyText", "body_text")
                .ok_or_else(|| "Missing 'bodyText' parameter".to_string())?;

            let mut email_builder = Message::builder()
                .from(
                    config
                        .from_email
                        .parse()
                        .map_err(|e| format!("Invalid from address: {}", e))?,
                )
                .subject(subject);

            for to_addr in to_str.split(',') {
                let trimmed = to_addr.trim();
                if !trimmed.is_empty() {
                    email_builder = email_builder.to(trimmed
                        .parse()
                        .map_err(|e| format!("Invalid recipient '{}': {}", trimmed, e))?);
                }
            }

            let draft_msg = email_builder
                .body(body_text)
                .map_err(|e| format!("Draft message building failed: {}", e))?;

            let mut imap_session = connect_imap(&config)?;
            let drafts_box = detect_drafts_mailbox(&mut imap_session, config.preset.as_deref());
            let raw_bytes = draft_msg.formatted();

            imap_session
                .append(&drafts_box, &raw_bytes)
                .map_err(|e| format!("Failed to append draft to {}: {}", drafts_box, e))?;

            let _ = imap_session.logout();
            Ok(json!({ "status": "draft_saved", "mailbox": drafts_box }))
        }

        "manage_message_flags" => {
            let uid = get_u64_arg(&request.arguments, "uid", "uid")
                .ok_or_else(|| "Missing 'uid' parameter".to_string())? as u32;
            let mailbox = get_str_arg(&request.arguments, "mailbox", "mailbox")
                .unwrap_or_else(|| "INBOX".to_string());
            let action = get_str_arg(&request.arguments, "action", "action")
                .ok_or_else(|| "Missing 'action' parameter".to_string())?;

            let flag_cmd = match action.as_str() {
                "mark_read" => "+FLAGS (\\Seen)",
                "mark_unread" => "-FLAGS (\\Seen)",
                "star" => "+FLAGS (\\Flagged)",
                "unstar" => "-FLAGS (\\Flagged)",
                other => return Err(format!("Unsupported action '{}'", other)),
            };

            let mut imap_session = connect_imap(&config)?;
            imap_session
                .select(&mailbox)
                .map_err(|e| format!("Select mailbox error: {}", e))?;

            imap_session
                .uid_store(uid.to_string(), flag_cmd)
                .map_err(|e| format!("Flag store error: {}", e))?;

            let _ = imap_session.logout();
            Ok(json!({ "status": "success", "uid": uid, "action_performed": action }))
        }

        "move_message" => {
            let uid = get_u64_arg(&request.arguments, "uid", "uid")
                .ok_or_else(|| "Missing 'uid' parameter".to_string())? as u32;
            let src_mailbox = get_str_arg(&request.arguments, "sourceMailbox", "source_mailbox")
                .unwrap_or_else(|| "INBOX".to_string());
            let dst_mailbox = get_str_arg(
                &request.arguments,
                "destinationMailbox",
                "destination_mailbox",
            )
            .ok_or_else(|| "Missing 'destinationMailbox' parameter".to_string())?;
            if dst_mailbox.trim().is_empty() {
                return Err("Parameter 'destinationMailbox' cannot be empty".to_string());
            }

            let mut imap_session = connect_imap(&config)?;
            imap_session
                .select(&src_mailbox)
                .map_err(|e| format!("Select source mailbox error: {}", e))?;

            imap_session
                .uid_copy(uid.to_string(), &dst_mailbox)
                .map_err(|e| format!("Failed to copy UID {} to {}: {}", uid, dst_mailbox, e))?;
            imap_session
                .uid_store(uid.to_string(), "+FLAGS (\\Deleted)")
                .map_err(|e| format!("Failed to mark deleted in {}: {}", src_mailbox, e))?;
            imap_session
                .expunge()
                .map_err(|e| format!("Expunge error: {}", e))?;

            let _ = imap_session.logout();
            Ok(json!({ "status": "moved", "uid": uid, "from": src_mailbox, "to": dst_mailbox }))
        }

        unknown => Err(format!("Unknown tool: {}", unknown)),
    }
}
