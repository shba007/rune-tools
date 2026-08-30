// plugins/rune-git/src/lib.rs
use extism_pdk::*;
use rune_pdk::{ToolCallRequest, ToolDefinition};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::PathBuf;

#[host_fn("extism:host/user")]
extern "ExtismHost" {
    fn host_git_exec(input: String) -> String;
}

#[derive(Serialize, Deserialize)]
struct GitExecRequest {
    args: Vec<String>,
    cwd: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct GitExecResponse {
    success: bool,
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
}

fn resolve_repo(repo_path: Option<&str>) -> Result<PathBuf, String> {
    let raw = repo_path.unwrap_or(".");
    let target = PathBuf::from(raw);

    if let Ok(Some(allowed_root)) = config::get("allowed_dir") {
        let root = PathBuf::from(allowed_root);
        if target.is_relative() {
            Ok(root.join(target))
        } else {
            Ok(target)
        }
    } else {
        Ok(target)
    }
}

fn run_git(args: &[&str], repo_path: Option<&str>) -> Result<String, String> {
    let cwd = resolve_repo(repo_path)?;
    let req = GitExecRequest {
        args: args.iter().map(|s| s.to_string()).collect(),
        cwd: Some(cwd.to_string_lossy().to_string()),
    };

    let raw_req = serde_json::to_string(&req).map_err(|e| e.to_string())?;
    let raw_resp =
        unsafe { host_git_exec(raw_req) }.map_err(|e| format!("Host execution failed: {:?}", e))?;

    let resp: GitExecResponse = serde_json::from_str(&raw_resp)
        .map_err(|e| format!("Failed to parse host response: {}", e))?;

    if resp.success {
        let out = resp.stdout.trim().to_string();
        if out.is_empty() {
            Ok("OK (no output)".to_string())
        } else {
            Ok(out)
        }
    } else {
        let err = if !resp.stderr.is_empty() {
            resp.stderr.trim()
        } else {
            &resp.stdout
        };
        Err(format!("Git error: {}", err))
    }
}

fn execute_tool(request: ToolCallRequest) -> Result<Value, String> {
    let repo_arg = request.arguments.get("repo_path").and_then(|v| v.as_str());

    match request.name.as_str() {
        "git_status" => {
            let out = run_git(&["status"], repo_arg)?;
            Ok(json!({ "content": out }))
        }
        "git_diff_unstaged" => {
            let out = run_git(&["diff"], repo_arg)?;
            Ok(json!({ "content": out }))
        }
        "git_diff_staged" => {
            let out = run_git(&["diff", "--staged"], repo_arg)?;
            Ok(json!({ "content": out }))
        }
        "git_diff" => {
            let target = request
                .arguments
                .get("target")
                .and_then(|v| v.as_str())
                .unwrap_or("HEAD");
            let out = run_git(&["diff", target], repo_arg)?;
            Ok(json!({ "content": out }))
        }
        "git_commit" => {
            let message = request.arguments["message"]
                .as_str()
                .ok_or_else(|| "Missing 'message' argument".to_string())?;
            let out = run_git(&["commit", "-m", message], repo_arg)?;
            Ok(json!({ "content": out }))
        }
        "git_add" => {
            let files = request
                .arguments
                .get("files")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|x| x.as_str()).collect::<Vec<&str>>())
                .unwrap_or_else(|| vec!["."]);

            let mut args = vec!["add"];
            args.extend(files);
            let out = run_git(&args, repo_arg)?;
            Ok(json!({ "content": out }))
        }
        "git_reset" => {
            let out = run_git(&["reset"], repo_arg)?;
            Ok(json!({ "content": out }))
        }
        "git_log" => {
            let max_count = request
                .arguments
                .get("max_count")
                .and_then(|v| v.as_u64())
                .unwrap_or(10)
                .to_string();
            let count_flag = format!("-n{}", max_count);
            let out = run_git(&["log", &count_flag, "--oneline", "--decorate"], repo_arg)?;
            Ok(json!({ "content": out }))
        }
        "git_create_branch" => {
            let branch_name = request.arguments["branch_name"]
                .as_str()
                .ok_or_else(|| "Missing 'branch_name' argument".to_string())?;
            let out = run_git(&["branch", branch_name], repo_arg)?;
            Ok(json!({ "content": out }))
        }
        "git_checkout" => {
            let branch_name = request.arguments["branch_name"]
                .as_str()
                .ok_or_else(|| "Missing 'branch_name' argument".to_string())?;
            let out = run_git(&["checkout", branch_name], repo_arg)?;
            Ok(json!({ "content": out }))
        }
        "git_show" => {
            let revision = request
                .arguments
                .get("revision")
                .and_then(|v| v.as_str())
                .unwrap_or("HEAD");
            let out = run_git(&["show", revision], repo_arg)?;
            Ok(json!({ "content": out }))
        }
        "git_init" => {
            let out = run_git(&["init"], repo_arg)?;
            Ok(json!({ "content": out }))
        }
        unknown => Err(format!("Unknown git tool: {}", unknown)),
    }
}

#[plugin_fn]
pub fn mcp_info(_: ()) -> FnResult<String> {
    let info = json!({
        "name": env!("CARGO_PKG_NAME"),
        "version": env!("CARGO_PKG_VERSION"),
        "description": option_env!("CARGO_PKG_DESCRIPTION")
    });
    Ok(serde_json::to_string(&info)?)
}

#[plugin_fn]
pub fn mcp_list_tools(_: ()) -> FnResult<String> {
    let tools = vec![
        ToolDefinition {
            name: "git_status".to_string(),
            description: "Shows the working tree status using host git.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "repo_path": { "type": "string", "description": "Path to the repository" }
                }
            }),
        },
        ToolDefinition {
            name: "git_diff_unstaged".to_string(),
            description: "Shows changes in the working tree not yet staged.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "repo_path": { "type": "string", "description": "Path to the repository" }
                }
            }),
        },
        ToolDefinition {
            name: "git_diff_staged".to_string(),
            description: "Shows changes that are staged for commit.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "repo_path": { "type": "string", "description": "Path to the repository" }
                }
            }),
        },
        ToolDefinition {
            name: "git_diff".to_string(),
            description: "Shows changes between branches or commits.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "repo_path": { "type": "string", "description": "Path to the repository" },
                    "target": { "type": "string", "description": "Target commit/branch (default: HEAD)" }
                }
            }),
        },
        ToolDefinition {
            name: "git_commit".to_string(),
            description: "Records changes to the repository with a commit message.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "repo_path": { "type": "string", "description": "Path to the repository" },
                    "message": { "type": "string", "description": "The commit message" }
                },
                "required": ["message"]
            }),
        },
        ToolDefinition {
            name: "git_add".to_string(),
            description: "Adds file contents to the staging area.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "repo_path": { "type": "string", "description": "Path to the repository" },
                    "files": { "type": "array", "items": { "type": "string" }, "description": "List of files to add (default: all)" }
                }
            }),
        },
        ToolDefinition {
            name: "git_reset".to_string(),
            description: "Resets current HEAD to the specified state.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "repo_path": { "type": "string", "description": "Path to the repository" }
                }
            }),
        },
        ToolDefinition {
            name: "git_log".to_string(),
            description: "Shows commit logs from the current branch.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "repo_path": { "type": "string", "description": "Path to the repository" },
                    "max_count": { "type": "integer", "default": 10, "description": "Max number of commits to return" }
                }
            }),
        },
        ToolDefinition {
            name: "git_create_branch".to_string(),
            description: "Creates a new branch from current HEAD.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "repo_path": { "type": "string", "description": "Path to the repository" },
                    "branch_name": { "type": "string", "description": "Name of the new branch" }
                },
                "required": ["branch_name"]
            }),
        },
        ToolDefinition {
            name: "git_checkout".to_string(),
            description: "Switches branches in the repository.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "repo_path": { "type": "string", "description": "Path to the repository" },
                    "branch_name": { "type": "string", "description": "Branch to switch to" }
                },
                "required": ["branch_name"]
            }),
        },
        ToolDefinition {
            name: "git_show".to_string(),
            description: "Shows commit metadata and patch details.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "repo_path": { "type": "string", "description": "Path to the repository" },
                    "revision": { "type": "string", "default": "HEAD", "description": "Commit SHA or ref" }
                }
            }),
        },
        ToolDefinition {
            name: "git_init".to_string(),
            description: "Initializes a new empty Git repository.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "repo_path": { "type": "string", "description": "Directory path to initialize" }
                }
            }),
        },
    ];

    Ok(serde_json::to_string(&tools)?)
}

#[plugin_fn]
pub fn mcp_call_tool(input: String) -> FnResult<String> {
    let request: ToolCallRequest = serde_json::from_str(&input)?;
    let result = execute_tool(request);

    let output = match result {
        Ok(val) => json!({ "status": "success", "result": val }),
        Err(err) => json!({ "status": "error", "error": err }),
    };

    Ok(serde_json::to_string(&output)?)
}
