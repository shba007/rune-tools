use crate::types::CommitValidationReport;
use rune_pdk::ToolCallRequest;
use serde_json::{Value, json};
use std::path::PathBuf;

fn get_str_arg(args: &Value, camel: &str, snake: &str) -> Option<String> {
    if let Some(val) = args
        .get(camel)
        .or_else(|| args.get(snake))
        .and_then(Value::as_str)
    {
        return Some(val.to_string());
    }
    let env_key = snake.to_ascii_uppercase();
    std::env::var(&env_key).ok()
}

fn get_u64_arg(args: &Value, camel: &str, snake: &str) -> Option<u64> {
    if let Some(val) = args
        .get(camel)
        .or_else(|| args.get(snake))
        .and_then(Value::as_u64)
    {
        return Some(val);
    }
    let env_key = snake.to_ascii_uppercase();
    std::env::var(&env_key).ok().and_then(|v| v.parse().ok())
}

fn get_bool_arg(args: &Value, camel: &str, snake: &str, default: bool) -> bool {
    if let Some(val) = args
        .get(camel)
        .or_else(|| args.get(snake))
        .and_then(Value::as_bool)
    {
        return val;
    }
    let env_key = snake.to_ascii_uppercase();
    std::env::var(&env_key)
        .ok()
        .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
        .unwrap_or(default)
}

fn get_array_arg(args: &Value, camel: &str, snake: &str) -> Option<Vec<String>> {
    if let Some(arr) = args
        .get(camel)
        .or_else(|| args.get(snake))
        .and_then(Value::as_array)
    {
        return Some(
            arr.iter()
                .filter_map(|v| v.as_str().map(ToString::to_string))
                .collect(),
        );
    }
    let env_key = snake.to_ascii_uppercase();
    std::env::var(&env_key).ok().map(|v| {
        v.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    })
}

pub fn resolve_repo(repo_path: Option<&str>) -> Result<PathBuf, String> {
    let explicit_path = repo_path.map(ToString::to_string).or_else(|| {
        std::env::var("REPO_PATH")
            .ok()
            .or_else(|| std::env::var("ALLOWED_DIR").ok())
    });

    let raw = explicit_path.unwrap_or_else(|| ".".to_string());
    Ok(PathBuf::from(raw))
}

fn run_git(args: &[&str], repo_path: Option<&str>) -> Result<String, String> {
    let cwd = resolve_repo(repo_path)?;
    let git_bin = rune_pdk::resolve_binary_executable("git").unwrap_or_else(|_| "git".to_string());

    let output = std::process::Command::new(git_bin)
        .args(args)
        .current_dir(&cwd)
        .output()
        .map_err(|e| format!("Failed to spawn git binary in '{}': {}", cwd.display(), e))?;

    if output.status.success() {
        let out = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if out.is_empty() {
            Ok("OK (no output)".to_string())
        } else {
            Ok(out)
        }
    } else {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let final_err = if !err.is_empty() { err } else { stdout };
        Err(format!("Git error: {}", final_err))
    }
}

pub fn execute_tool(request: ToolCallRequest) -> Result<Value, String> {
    let repo_arg = get_str_arg(&request.arguments, "repoPath", "repo_path");
    let repo_ref = repo_arg.as_deref();

    match request.name.as_str() {
        "git_status" => {
            let out = run_git(&["status"], repo_ref)?;
            Ok(json!({ "content": out }))
        }
        "git_diff_unstaged" => {
            let out = run_git(&["diff"], repo_ref)?;
            Ok(json!({ "content": out }))
        }
        "git_diff_staged" => {
            let out = run_git(&["diff", "--staged"], repo_ref)?;
            Ok(json!({ "content": out }))
        }
        "git_diff" => {
            let target = get_str_arg(&request.arguments, "target", "target")
                .unwrap_or_else(|| "HEAD".to_string());
            let out = run_git(&["diff", &target], repo_ref)?;
            Ok(json!({ "content": out }))
        }
        "git_commit" => {
            let message = get_str_arg(&request.arguments, "message", "message")
                .ok_or_else(|| "Missing 'message' argument".to_string())?;
            if message.trim().is_empty() {
                return Err("Parameter 'message' cannot be empty".to_string());
            }
            let out = run_git(&["commit", "-m", &message], repo_ref)?;
            Ok(json!({ "content": out }))
        }
        "git_add" => {
            let files = get_array_arg(&request.arguments, "files", "files")
                .unwrap_or_else(|| vec![".".to_string()]);
            let mut args = vec!["add".to_string()];
            args.extend(files);
            let args_ref: Vec<&str> = args.iter().map(AsRef::as_ref).collect();
            let out = run_git(&args_ref, repo_ref)?;
            Ok(json!({ "content": out }))
        }
        "git_reset" => {
            let out = run_git(&["reset"], repo_ref)?;
            Ok(json!({ "content": out }))
        }
        "git_log" => {
            let max_count = get_u64_arg(&request.arguments, "maxCount", "max_count").unwrap_or(10);
            let count_flag = format!("-n{}", max_count);
            let out = run_git(&["log", &count_flag, "--oneline", "--decorate"], repo_ref)?;
            Ok(json!({ "content": out }))
        }
        "git_create_branch" => {
            let branch_name = get_str_arg(&request.arguments, "branchName", "branch_name")
                .ok_or_else(|| "Missing 'branchName' argument".to_string())?;
            if branch_name.trim().is_empty() {
                return Err("Parameter 'branchName' cannot be empty".to_string());
            }
            let out = run_git(&["branch", &branch_name], repo_ref)?;
            Ok(json!({ "content": out }))
        }
        "git_checkout" => {
            let branch_name = get_str_arg(&request.arguments, "branchName", "branch_name")
                .ok_or_else(|| "Missing 'branchName' argument".to_string())?;
            if branch_name.trim().is_empty() {
                return Err("Parameter 'branchName' cannot be empty".to_string());
            }
            let out = run_git(&["checkout", &branch_name], repo_ref)?;
            Ok(json!({ "content": out }))
        }
        "git_show" => {
            let revision = get_str_arg(&request.arguments, "revision", "revision")
                .unwrap_or_else(|| "HEAD".to_string());
            let out = run_git(&["show", &revision], repo_ref)?;
            Ok(json!({ "content": out }))
        }
        "git_init" => {
            let out = run_git(&["init"], repo_ref)?;
            Ok(json!({ "content": out }))
        }
        "validate_commit_message" | "git_validate_commit_message" => {
            let message = get_str_arg(&request.arguments, "message", "message")
                .ok_or_else(|| "Missing 'message' argument".to_string())?;
            let max_subject_length =
                get_u64_arg(&request.arguments, "maxSubjectLength", "max_subject_length")
                    .unwrap_or(72) as usize;
            let enforce_conventional =
                get_bool_arg(&request.arguments, "conventional", "conventional", false);

            let report =
                validate_commit_message_content(&message, max_subject_length, enforce_conventional);
            Ok(json!(report))
        }
        unknown => Err(format!("Unknown git tool: {}", unknown)),
    }
}

pub fn validate_commit_message_content(
    message: &str,
    max_subject_length: usize,
    enforce_conventional: bool,
) -> CommitValidationReport {
    let mut issues = Vec::new();
    let mut lines = message.lines();
    let subject = lines.next().unwrap_or("").trim().to_string();
    let body_lines: Vec<&str> = lines.collect();

    let subject_len = subject.chars().count();
    if subject.is_empty() {
        issues.push("Commit subject line cannot be empty".to_string());
    } else if subject_len > max_subject_length {
        issues.push(format!(
            "Subject line exceeds {} characters (actual: {})",
            max_subject_length, subject_len
        ));
    }

    if let Some(first_body_line) = body_lines.first()
        && !first_body_line.trim().is_empty()
    {
        issues.push("Second line must be empty to separate subject from body".to_string());
    }

    if enforce_conventional
        && !subject.is_empty()
        && let Err(err) = check_conventional_format(&subject)
    {
        issues.push(err);
    }

    let body = if body_lines.is_empty() {
        None
    } else {
        let joined = body_lines.join("\n").trim().to_string();
        if joined.is_empty() {
            None
        } else {
            Some(joined)
        }
    };

    CommitValidationReport {
        valid: issues.is_empty(),
        subject,
        subject_length: subject_len,
        body,
        issues,
    }
}

fn check_conventional_format(subject: &str) -> Result<(), String> {
    const VALID_TYPES: &[&str] = &[
        "feat", "fix", "docs", "style", "refactor", "perf", "test", "build", "ci", "chore",
        "revert",
    ];

    let (prefix, rest) = subject.split_once(':').ok_or_else(|| {
        "Missing colon separator in conventional commit format (<type>: <description>)".to_string()
    })?;

    if !rest.starts_with(' ') || rest.trim().is_empty() {
        return Err("Missing space after colon or empty description in commit subject".to_string());
    }

    let raw_prefix = prefix.strip_suffix('!').unwrap_or(prefix);
    let commit_type = if let Some((c_type, scope_part)) = raw_prefix.split_once('(') {
        if !scope_part.ends_with(')') || scope_part.len() <= 1 {
            return Err(
                "Invalid scope syntax in conventional commit prefix (expected '<type>(<scope>):')"
                    .to_string(),
            );
        }
        c_type.trim()
    } else {
        raw_prefix.trim()
    };

    if !VALID_TYPES.contains(&commit_type.to_ascii_lowercase().as_str()) {
        return Err(format!(
            "Unknown conventional commit type '{}'. Allowed types: {}",
            commit_type,
            VALID_TYPES.join(", ")
        ));
    }

    Ok(())
}
