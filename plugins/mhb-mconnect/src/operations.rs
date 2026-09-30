use crate::types::{
    EmailTemplateDetail, EmailTemplateSummary, RenderPreviewRequest, RenderPreviewResponse,
};
use rune_pdk::ToolCallRequest;
use serde_json::{Value, json};

fn get_base_url(args: &Value) -> String {
    let raw = args
        .get("baseUrl")
        .or_else(|| args.get("base_url"))
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .or_else(|| rune_pdk::get_config("MHB_BASE_URL"))
        .or_else(|| rune_pdk::get_config("BASE_URL"))
        .or_else(|| std::env::var("MHB_BASE_URL").ok())
        .or_else(|| std::env::var("BASE_URL").ok())
        .unwrap_or_else(|| "https://api.modesthumanbrands.com".to_string());
    raw.trim_end_matches('/').to_string()
}

fn get_str_arg(args: &Value, camel: &str, snake: &str) -> Result<String, String> {
    let val = args.get(camel).or_else(|| args.get(snake));
    match val {
        Some(Value::String(s)) => {
            if s.trim().is_empty() {
                Err(format!("Parameter '{}' cannot be empty", camel))
            } else {
                Ok(s.trim().to_string())
            }
        }
        Some(_) => Err(format!("Parameter '{}' must be a string", camel)),
        None => Err(format!("Missing '{}' parameter", camel)),
    }
}

pub fn execute_tool(mut request: ToolCallRequest) -> Result<Value, String> {
    if let Some(pos) = request.name.rfind("__") {
        request.name = request.name[pos + 2..].to_string();
    }

    let base_url = get_base_url(&request.arguments);
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    match request.name.as_str() {
        "mhb_list_templates" => {
            let url = format!("{}/api/interaction/email/template", base_url);
            let resp = client
                .get(&url)
                .send()
                .map_err(|e| format!("Network request failed: {}", e))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let err_text = resp.text().unwrap_or_default();
                return Err(format!(
                    "API returned error status [{}]: {}",
                    status, err_text
                ));
            }

            let templates: Vec<EmailTemplateSummary> = resp
                .json()
                .map_err(|e| format!("Failed to parse templates JSON: {}", e))?;

            Ok(json!({ "templates": templates }))
        }

        "mhb_get_template" => {
            let template_id = get_str_arg(&request.arguments, "templateId", "template_id")?;

            let url = format!(
                "{}/api/interaction/email/template/{}",
                base_url, template_id
            );
            let resp = client
                .get(&url)
                .send()
                .map_err(|e| format!("Network request failed: {}", e))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let err_text = resp.text().unwrap_or_default();
                return Err(format!(
                    "API returned error status [{}]: {}",
                    status, err_text
                ));
            }

            let detail: EmailTemplateDetail = resp
                .json()
                .map_err(|e| format!("Failed to parse template detail JSON: {}", e))?;

            Ok(json!(detail))
        }

        "mhb_render_template_preview" => {
            let template_id = get_str_arg(&request.arguments, "templateId", "template_id")?;

            let variables = request
                .arguments
                .get("variables")
                .ok_or_else(|| "Missing 'variables' parameter".to_string())?;

            if !variables.is_object() {
                return Err("Parameter 'variables' must be a JSON object".to_string());
            }

            let payload = RenderPreviewRequest {
                template_id,
                variables: variables.clone(),
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
