#[cfg(not(target_arch = "wasm32"))]
mod native {
    use crate::types::{AttachmentInfo, EmailAccountConfig, MessageHeaderSummary};
    use imap::Session;
    use lettre::message::{
        MultiPart, SinglePart,
        header::{ContentDisposition, ContentType},
    };
    use lettre::transport::smtp::authentication::Credentials;
    use lettre::{Message, SmtpTransport, Transport};
    use mail_parser::MimeHeaders;
    use native_tls::TlsConnector;
    use rune_pdk::ToolCallRequest;
    use serde_json::{Value, json};
    use std::fs;
    use std::path::{Path, PathBuf};

    fn get_payload_str(args: &Value, camel: &str, snake: &str) -> Option<String> {
        args.get(camel)
            .or_else(|| args.get(snake))
            .and_then(Value::as_str)
            .map(|s| s.trim().to_string())
    }

    fn get_config_str(args: &Value, camel: &str, snake: &str) -> Option<String> {
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

    pub fn resolve_dir(dir_param: Option<&str>) -> String {
        let explicit = dir_param.map(ToString::to_string).or_else(|| {
            std::env::var("OUTPUT_DIRECTORY")
                .or_else(|_| std::env::var("OUTPUT_DIR"))
                .or_else(|_| std::env::var("ALLOWED_DIR"))
                .ok()
        });

        let raw = explicit.unwrap_or_else(|| ".".to_string());
        let target = PathBuf::from(raw);

        if let Ok(allowed_root) = std::env::var("ALLOWED_DIR") {
            let root = PathBuf::from(allowed_root);
            if target.is_relative() {
                root.join(target).to_string_lossy().to_string()
            } else {
                target.to_string_lossy().to_string()
            }
        } else {
            target.to_string_lossy().to_string()
        }
    }

    fn guess_mime_type(path: &Path) -> &'static str {
        match path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str()
        {
            "txt" | "text" | "log" => "text/plain",
            "html" | "htm" => "text/html",
            "json" => "application/json",
            "pdf" => "application/pdf",
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "svg" => "image/svg+xml",
            "webp" => "image/webp",
            "csv" => "text/csv",
            "zip" => "application/zip",
            "tar" => "application/x-tar",
            "gz" => "application/gzip",
            "xml" => "application/xml",
            "mp3" => "audio/mpeg",
            "mp4" => "video/mp4",
            "doc" | "docx" => "application/msword",
            "xls" | "xlsx" => "application/vnd.ms-excel",
            "ppt" | "pptx" => "application/vnd.ms-powerpoint",
            _ => "application/octet-stream",
        }
    }

    pub fn resolve_account_config(args: &Value) -> Result<EmailAccountConfig, String> {
        let preset = get_config_str(args, "preset", "preset")
            .or_else(|| std::env::var("EMAIL_PRESET").ok())
            .map(|s| s.to_ascii_lowercase());

        let email = get_config_str(args, "email", "email")
            .or_else(|| std::env::var("EMAIL_USER").ok())
            .or_else(|| std::env::var("IMAP_USER").ok())
            .or_else(|| std::env::var("SMTP_USER").ok())
            .ok_or_else(|| "Missing required email address/username".to_string())?;

        let pass = get_config_str(args, "password", "password")
            .or_else(|| std::env::var("EMAIL_PASSWORD").ok())
            .or_else(|| std::env::var("IMAP_PASSWORD").ok())
            .or_else(|| std::env::var("SMTP_PASSWORD").ok())
            .ok_or_else(|| "Missing required email password".to_string())?;

        if email.trim().is_empty() {
            return Err("Email address cannot be empty".to_string());
        }
        if pass.trim().is_empty() {
            return Err("Email password cannot be empty".to_string());
        }

        let display_name = get_config_str(args, "displayName", "display_name")
            .or_else(|| std::env::var("EMAIL_DISPLAY_NAME").ok());

        let (def_imap_host, def_imap_port, def_smtp_host, def_smtp_port) = match preset.as_deref() {
            Some("gmail") => ("imap.gmail.com", 993, "smtp.gmail.com", 465),
            Some("hostinger") => ("imap.hostinger.com", 993, "smtp.hostinger.com", 465),
            Some("outlook") => ("outlook.office365.com", 993, "smtp.office365.com", 587),
            _ => ("", 993, "", 465),
        };

        let imap_host = get_config_str(args, "imapHost", "imap_host")
            .unwrap_or_else(|| def_imap_host.to_string());
        let smtp_host = get_config_str(args, "smtpHost", "smtp_host")
            .unwrap_or_else(|| def_smtp_host.to_string());

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

    fn build_smtp_transport(config: &EmailAccountConfig) -> Result<SmtpTransport, String> {
        let creds = Credentials::new(config.smtp_user.clone(), config.smtp_pass.clone());
        let builder = if config.smtp_port == 587 {
            SmtpTransport::starttls_relay(&config.smtp_host)
        } else {
            SmtpTransport::relay(&config.smtp_host)
        };

        let transport = builder
            .map_err(|e| format!("SMTP relay configuration error: {}", e))?
            .port(config.smtp_port)
            .credentials(creds)
            .build();

        Ok(transport)
    }

    pub fn execute_tool(request: ToolCallRequest) -> Result<Value, String> {
        const KNOWN_TOOLS: &[&str] = &[
            "verify_email_connection",
            "list_mailboxes",
            "list_messages",
            "search_messages",
            "read_message",
            "download_attachment",
            "send_email",
            "reply_email",
            "draft_email",
            "manage_message_flags",
            "move_message",
        ];

        if !KNOWN_TOOLS.contains(&request.name.as_str()) {
            return Err(format!("Unknown tool: {}", request.name));
        }

        let config = resolve_account_config(&request.arguments)?;

        match request.name.as_str() {
            "verify_email_connection" => {
                let mut imap_session = connect_imap(&config)?;
                let mailboxes = imap_session
                    .list(None, Some("*"))
                    .map_err(|e| format!("Failed to list mailboxes: {}", e))?;
                let mb_count = mailboxes.len();
                let _ = imap_session.logout();

                let transport = build_smtp_transport(&config)?;
                let smtp_tested = transport
                    .test_connection()
                    .map_err(|e| format!("SMTP test connection failed: {}", e))?;

                Ok(json!({
                    "status": "connected",
                    "account": config.from_email,
                    "imap": { "host": config.imap_host, "port": config.imap_port, "authenticated": true, "mailboxCount": mb_count },
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
                let mailbox = get_payload_str(&request.arguments, "mailbox", "mailbox")
                    .unwrap_or_else(|| "INBOX".to_string());
                let limit =
                    get_u64_arg(&request.arguments, "limit", "limit").unwrap_or(20) as usize;
                let page = get_u64_arg(&request.arguments, "page", "page").unwrap_or(1) as usize;
                let unread_only =
                    get_bool_arg(&request.arguments, "unreadOnly", "unread_only", false);

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
                let paged_uids: Vec<u32> =
                    uid_list.into_iter().skip(start_idx).take(limit).collect();

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

                        if let Some(header_bytes) = msg.header()
                            && let Some(parsed) =
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
                    "totalMessages": total_found,
                    "page": page,
                    "limit": limit,
                    "messages": summaries
                }))
            }

            "search_messages" => {
                let mailbox = get_payload_str(&request.arguments, "mailbox", "mailbox")
                    .unwrap_or_else(|| "INBOX".to_string());
                let query_kw = get_payload_str(&request.arguments, "query", "query");
                let from_filter = get_payload_str(&request.arguments, "from", "from");
                let to_filter = get_payload_str(&request.arguments, "to", "to");
                let subject_filter = get_payload_str(&request.arguments, "subject", "subject");
                let since_date = get_payload_str(&request.arguments, "sinceDate", "since_date");
                let limit =
                    get_u64_arg(&request.arguments, "limit", "limit").unwrap_or(20) as usize;

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

                        if let Some(header_bytes) = msg.header()
                            && let Some(parsed) =
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
                    "searchCriteria": search_str,
                    "totalMatches": total_found,
                    "limit": limit,
                    "messages": summaries
                }))
            }

            "read_message" => {
                let uid = get_u64_arg(&request.arguments, "uid", "uid")
                    .ok_or_else(|| "Missing 'uid' parameter".to_string())?
                    as u32;
                if uid == 0 {
                    return Err("Parameter 'uid' must be a positive integer".to_string());
                }
                let mailbox = get_payload_str(&request.arguments, "mailbox", "mailbox")
                    .unwrap_or_else(|| "INBOX".to_string());
                let mark_as_read =
                    get_bool_arg(&request.arguments, "markAsRead", "mark_as_read", true);

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
                    "messageId": message_id,
                    "bodyText": body_text,
                    "bodyHtml": body_html,
                    "attachments": attachments
                }))
            }

            "download_attachment" => {
                let uid = get_u64_arg(&request.arguments, "uid", "uid")
                    .ok_or_else(|| "Missing 'uid' parameter".to_string())?
                    as u32;
                if uid == 0 {
                    return Err("Parameter 'uid' must be a positive integer".to_string());
                }
                let mailbox = get_payload_str(&request.arguments, "mailbox", "mailbox")
                    .unwrap_or_else(|| "INBOX".to_string());
                let attachment_idx =
                    get_u64_arg(&request.arguments, "attachmentIndex", "attachment_index")
                        .map(|v| v as usize);
                let target_name = get_payload_str(&request.arguments, "filename", "filename");

                let out_dir_param =
                    get_payload_str(&request.arguments, "outputDirectory", "output_directory");
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
                    let raw_name = att.attachment_name().unwrap_or("attachment");
                    let name = Path::new(raw_name)
                        .file_name()
                        .map(|f| f.to_string_lossy().to_string())
                        .unwrap_or_else(|| "attachment".to_string());

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
                    "savedFiles": saved_files
                }))
            }

            "send_email" => {
                let to_str = get_payload_str(&request.arguments, "to", "to")
                    .ok_or_else(|| "Missing 'to' parameter".to_string())?;
                if to_str.trim().is_empty() {
                    return Err("Parameter 'to' cannot be empty".to_string());
                }
                let subject = get_payload_str(&request.arguments, "subject", "subject")
                    .ok_or_else(|| "Missing 'subject' parameter".to_string())?;
                if subject.trim().is_empty() {
                    return Err("Parameter 'subject' cannot be empty".to_string());
                }
                let body_text = get_payload_str(&request.arguments, "bodyText", "body_text")
                    .ok_or_else(|| "Missing 'bodyText' parameter".to_string())?;
                if body_text.trim().is_empty() {
                    return Err("Parameter 'bodyText' cannot be empty".to_string());
                }
                let body_html = get_payload_str(&request.arguments, "bodyHtml", "body_html");

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

                if let Some(cc_str) = get_payload_str(&request.arguments, "cc", "cc") {
                    for cc in cc_str.split(',') {
                        let trimmed = cc.trim();
                        if !trimmed.is_empty() {
                            email_builder = email_builder.cc(trimmed
                                .parse()
                                .map_err(|e| format!("Invalid CC '{}': {}", trimmed, e))?);
                        }
                    }
                }

                if let Some(bcc_str) = get_payload_str(&request.arguments, "bcc", "bcc") {
                    for bcc in bcc_str.split(',') {
                        let trimmed = bcc.trim();
                        if !trimmed.is_empty() {
                            email_builder = email_builder.bcc(
                                trimmed
                                    .parse()
                                    .map_err(|e| format!("Invalid BCC '{}': {}", trimmed, e))?,
                            );
                        }
                    }
                }

                let multipart = if let Some(html) = body_html {
                    MultiPart::alternative()
                        .singlepart(
                            SinglePart::builder()
                                .header(ContentType::TEXT_PLAIN)
                                .body(body_text.clone()),
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
                            .body(body_text.clone()),
                    )
                };

                let attachment_paths: Vec<String> = request
                    .arguments
                    .get("attachmentPaths")
                    .or_else(|| request.arguments.get("attachment_paths"))
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|p| p.as_str().map(ToString::to_string))
                            .collect()
                    })
                    .unwrap_or_default();

                let email_msg = if !attachment_paths.is_empty() {
                    let mut mixed = MultiPart::mixed().multipart(multipart);
                    for path_str in attachment_paths {
                        let resolved = resolve_dir(Some(&path_str));
                        let path = PathBuf::from(&resolved);
                        let file_bytes = fs::read(&path).map_err(|e| {
                            format!("Failed to read attachment '{}': {}", path_str, e)
                        })?;
                        let filename = path
                            .file_name()
                            .map(|f| f.to_string_lossy().to_string())
                            .unwrap_or_else(|| "attachment.bin".to_string());
                        let mime = guess_mime_type(&path);
                        let content_type = ContentType::parse(mime).unwrap_or_else(|_| {
                            ContentType::parse("application/octet-stream").unwrap()
                        });

                        mixed = mixed.singlepart(
                            SinglePart::builder()
                                .header(content_type)
                                .header(ContentDisposition::attachment(&filename))
                                .body(file_bytes),
                        );
                    }
                    email_builder
                        .multipart(mixed)
                        .map_err(|e| format!("Failed to build MIME message: {}", e))?
                } else {
                    email_builder
                        .multipart(multipart)
                        .map_err(|e| format!("Failed to build MIME message: {}", e))?
                };

                let transport = build_smtp_transport(&config)?;

                transport
                    .send(&email_msg)
                    .map_err(|e| format!("SMTP dispatch failed: {}", e))?;

                if config.preset.as_deref() != Some("gmail")
                    && let Ok(mut imap_session) = connect_imap(&config)
                {
                    let sent_box = detect_sent_mailbox(&mut imap_session, config.preset.as_deref());
                    let raw_bytes = email_msg.formatted();
                    let _ = imap_session.append(&sent_box, &raw_bytes);
                    let _ = imap_session.logout();
                }

                Ok(json!({ "status": "sent", "to": to_str }))
            }

            "reply_email" => {
                let original_uid = get_u64_arg(&request.arguments, "originalUid", "original_uid")
                    .ok_or_else(|| "Missing 'originalUid' parameter".to_string())?
                    as u32;
                if original_uid == 0 {
                    return Err("Parameter 'originalUid' must be a positive integer".to_string());
                }
                let mailbox = get_payload_str(&request.arguments, "mailbox", "mailbox")
                    .unwrap_or_else(|| "INBOX".to_string());
                let reply_body = get_payload_str(&request.arguments, "replyBody", "reply_body")
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

                if reply_all && let Some(to_addrs) = parsed.to() {
                    for a in to_addrs.iter() {
                        if let Some(addr) = a.address()
                            && addr != config.from_email
                            && addr != orig_from
                            && let Ok(parsed_addr) = addr.parse()
                        {
                            email_builder = email_builder.cc(parsed_addr);
                        }
                    }
                }

                let reply_msg = email_builder
                    .body(reply_body)
                    .map_err(|e| format!("Failed to build reply message: {}", e))?;

                let transport = build_smtp_transport(&config)?;

                transport
                    .send(&reply_msg)
                    .map_err(|e| format!("SMTP reply failed: {}", e))?;

                if config.preset.as_deref() != Some("gmail") {
                    let sent_box = detect_sent_mailbox(&mut imap_session, config.preset.as_deref());
                    let raw_bytes = reply_msg.formatted();
                    let _ = imap_session.append(&sent_box, &raw_bytes);
                }

                let _ = imap_session.logout();

                Ok(json!({ "status": "replied", "inReplyTo": orig_msg_id, "to": orig_from }))
            }

            "draft_email" => {
                let to_str = get_payload_str(&request.arguments, "to", "to")
                    .ok_or_else(|| "Missing 'to' parameter".to_string())?;
                if to_str.trim().is_empty() {
                    return Err("Parameter 'to' cannot be empty".to_string());
                }
                let subject = get_payload_str(&request.arguments, "subject", "subject")
                    .ok_or_else(|| "Missing 'subject' parameter".to_string())?;
                if subject.trim().is_empty() {
                    return Err("Parameter 'subject' cannot be empty".to_string());
                }
                let body_text = get_payload_str(&request.arguments, "bodyText", "body_text")
                    .ok_or_else(|| "Missing 'bodyText' parameter".to_string())?;
                if body_text.trim().is_empty() {
                    return Err("Parameter 'bodyText' cannot be empty".to_string());
                }

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
                    .ok_or_else(|| "Missing 'uid' parameter".to_string())?
                    as u32;
                if uid == 0 {
                    return Err("Parameter 'uid' must be a positive integer".to_string());
                }
                let mailbox = get_payload_str(&request.arguments, "mailbox", "mailbox")
                    .unwrap_or_else(|| "INBOX".to_string());
                let action = get_payload_str(&request.arguments, "action", "action")
                    .ok_or_else(|| "Missing 'action' parameter".to_string())?;
                if action.trim().is_empty() {
                    return Err("Parameter 'action' cannot be empty".to_string());
                }

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
                Ok(json!({ "status": "success", "uid": uid, "actionPerformed": action }))
            }

            "move_message" => {
                let uid = get_u64_arg(&request.arguments, "uid", "uid")
                    .ok_or_else(|| "Missing 'uid' parameter".to_string())?
                    as u32;
                if uid == 0 {
                    return Err("Parameter 'uid' must be a positive integer".to_string());
                }
                let src_mailbox =
                    get_payload_str(&request.arguments, "sourceMailbox", "source_mailbox")
                        .unwrap_or_else(|| "INBOX".to_string());
                let dst_mailbox = get_payload_str(
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
                Ok(
                    json!({ "status": "moved", "uid": uid, "sourceMailbox": src_mailbox, "destinationMailbox": dst_mailbox }),
                )
            }

            unknown => Err(format!("Unknown tool: {}", unknown)),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use native::*;

#[cfg(target_arch = "wasm32")]
mod wasm {
    use crate::types::EmailAccountConfig;
    use rune_pdk::ToolCallRequest;
    use serde_json::Value;

    pub fn resolve_dir(_dir_param: Option<&str>) -> String {
        ".".to_string()
    }

    pub fn resolve_account_config(_args: &Value) -> Result<EmailAccountConfig, String> {
        Err(
            "Email account configuration is only available in the native execution model."
                .to_string(),
        )
    }

    pub fn execute_tool(request: ToolCallRequest) -> Result<Value, String> {
        Err(format!(
            "Tool '{}' requires native network socket access (IMAP/SMTP). 'rune-email' is a native plugin and must be executed using its native binary ('rune-email-native').",
            request.name
        ))
    }
}

#[cfg(target_arch = "wasm32")]
pub use wasm::*;
