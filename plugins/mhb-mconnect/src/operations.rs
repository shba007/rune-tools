#[cfg(target_arch = "wasm32")]
use crate::types::{CmdExecRequest, CmdExecResponse};
#[cfg(not(target_arch = "wasm32"))]
use crate::types::{
    EmailTemplateDetail, EmailTemplateSummary, RenderPreviewRequest, RenderPreviewResponse,
};
use rune_pdk::ToolCallRequest;
use serde_json::{Value, json};

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
        program: "rune-mhb-mconnect-native".to_string(),
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
            "rune-mhb-mconnect-native exited with failure".to_string()
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

#[cfg(not(target_arch = "wasm32"))]
fn get_base_url(args: &Value) -> String {
    args.get("baseUrl")
        .or_else(|| args.get("base_url"))
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .or_else(|| std::env::var("MHB_BASE_URL").ok())
        .or_else(|| std::env::var("BASE_URL").ok())
        .unwrap_or_else(|| "https://api.modesthumanbrands.com".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn get_str_arg(args: &Value, camel: &str, snake: &str) -> Option<String> {
    args.get(camel)
        .or_else(|| args.get(snake))
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn execute_tool(request: ToolCallRequest) -> Result<Value, String> {
    let base_url = get_base_url(&request.arguments);
    let client = reqwest::blocking::Client::new();

    match request.name.as_str() {
        "mhb_list_templates" => {
            let url = format!("{}/api/interaction/email/template", base_url);
            let resp = client
                .get(&url)
                .send()
                .map_err(|e| format!("Network request failed: {}", e))?;

            if !resp.status().is_success() {
                return Err(format!("API returned error status: {}", resp.status()));
            }

            let templates: Vec<EmailTemplateSummary> = resp
                .json()
                .map_err(|e| format!("Failed to parse templates JSON: {}", e))?;

            Ok(json!({ "templates": templates }))
        }

        "mhb_get_template" => {
            let template_id = get_str_arg(&request.arguments, "templateId", "template_id")
                .ok_or_else(|| "Missing 'templateId' parameter".to_string())?;

            if template_id.trim().is_empty() {
                return Err("Parameter 'templateId' cannot be empty".to_string());
            }

            let url = format!(
                "{}/api/interaction/email/template/{}",
                base_url, template_id
            );
            let resp = client
                .get(&url)
                .send()
                .map_err(|e| format!("Network request failed: {}", e))?;

            if !resp.status().is_success() {
                return Err(format!("API returned error status: {}", resp.status()));
            }

            let detail: EmailTemplateDetail = resp
                .json()
                .map_err(|e| format!("Failed to parse template detail JSON: {}", e))?;

            Ok(json!(detail))
        }

        "mhb_render_template_preview" => {
            let template_id = get_str_arg(&request.arguments, "templateId", "template_id")
                .ok_or_else(|| "Missing 'templateId' parameter".to_string())?;

            let variables = request
                .arguments
                .get("variables")
                .cloned()
                .unwrap_or_else(|| json!({}));

            let payload = RenderPreviewRequest {
                template_id,
                variables,
            };

            let url = format!("{}/api/interaction/email/template/preview", base_url);
            let resp = client
                .post(&url)
                .json(&payload)
                .send()
                .map_err(|e| format!("Network request failed: {}", e))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let err_text = resp.text().unwrap_or_default();
                return Err(format!("API preview failed [{}]: {}", status, err_text));
            }

            let preview: RenderPreviewResponse = resp
                .json()
                .map_err(|e| format!("Failed to parse preview response JSON: {}", e))?;

            Ok(json!(preview))
        }

        unknown => Err(format!("Unknown tool: {}", unknown)),
    }
}
