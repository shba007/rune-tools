#[cfg(not(target_arch = "wasm32"))]
use crate::escl::EsclClient;
#[cfg(not(target_arch = "wasm32"))]
use crate::types::ScanDocumentResult;
#[cfg(target_arch = "wasm32")]
use crate::types::{CmdExecRequest, CmdExecResponse};
use rune_pdk::ToolCallRequest;
use serde_json::{Value, json};
use std::path::PathBuf;

// =========================================================================
// WASM32 Target Implementation (Host Subprocess Bridge)
// =========================================================================

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
        program: "rune-scan-native".to_string(),
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
            "rune-scan-native exited with failure".to_string()
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
// Native Target Implementation (Direct eSCL Network AirScan)
// =========================================================================

#[cfg(not(target_arch = "wasm32"))]
fn get_config(key: &str) -> Option<String> {
    std::env::var(key.to_ascii_uppercase())
        .or_else(|_| std::env::var(key))
        .ok()
        .filter(|s| !s.is_empty())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn resolve_path(relative_or_abs: &str) -> PathBuf {
    let target = PathBuf::from(relative_or_abs);
    if let Some(allowed_root) = get_config("allowed_dir") {
        let root = PathBuf::from(allowed_root);
        if target.is_relative() {
            root.join(target)
        } else {
            target
        }
    } else {
        target
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn get_str_arg(args: &Value, camel: &str, snake: &str) -> Option<String> {
    args.get(camel)
        .or_else(|| args.get(snake))
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

#[cfg(not(target_arch = "wasm32"))]
fn get_u64_arg(args: &Value, camel: &str, snake: &str) -> Option<u64> {
    args.get(camel)
        .or_else(|| args.get(snake))
        .and_then(Value::as_u64)
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_scanner_endpoint(scanner_input: &str, args: &Value) -> (String, u16, bool) {
    let mut clean_ip = scanner_input.trim().to_string();
    let use_https;

    if clean_ip.starts_with("https://") {
        clean_ip = clean_ip.trim_start_matches("https://").to_string();
        use_https = true;
    } else if clean_ip.starts_with("http://") {
        clean_ip = clean_ip.trim_start_matches("http://").to_string();
        use_https = false;
    } else {
        use_https = args
            .get("useHttps")
            .or_else(|| args.get("use_https"))
            .and_then(Value::as_bool)
            .or_else(|| {
                get_config("scanner_use_https").map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            })
            .unwrap_or(true);
    }

    let port = args
        .get("port")
        .and_then(Value::as_u64)
        .map(|v| v as u16)
        .or_else(|| get_config("scanner_port").and_then(|v| v.parse().ok()))
        .unwrap_or(if use_https { 443 } else { 80 });

    clean_ip = clean_ip.trim_end_matches('/').to_string();
    (clean_ip, port, use_https)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn execute_tool(request: ToolCallRequest) -> Result<Value, String> {
    match request.name.as_str() {
        "scan_document" | "printer_scan_document" => {
            let scanner_ip = get_str_arg(&request.arguments, "scannerIp", "scanner_ip")
                .or_else(|| get_str_arg(&request.arguments, "printerIp", "printer_ip"))
                .ok_or_else(|| "Missing 'scannerIp' parameter".to_string())?;

            if scanner_ip.trim().is_empty() {
                return Err("Parameter 'scannerIp' cannot be empty".to_string());
            }

            let output_path_str = get_str_arg(&request.arguments, "outputPath", "output_path")
                .ok_or_else(|| "Missing 'outputPath' parameter".to_string())?;

            if output_path_str.trim().is_empty() {
                return Err("Parameter 'outputPath' cannot be empty".to_string());
            }

            let dest = resolve_path(&output_path_str);
            if let Some(parent) = dest.parent() {
                let _ = std::fs::create_dir_all(parent);
            }

            let dpi = get_u64_arg(&request.arguments, "resolutionDpi", "resolution_dpi")
                .unwrap_or(300) as u32;
            let color_mode = get_str_arg(&request.arguments, "colorMode", "color_mode")
                .unwrap_or_else(|| "Color".to_string());
            let source = get_str_arg(&request.arguments, "inputSource", "input_source")
                .unwrap_or_else(|| "Platen".to_string());

            let (host, port, use_https) = resolve_scanner_endpoint(&scanner_ip, &request.arguments);
            let escl = EsclClient::new(&host, port, use_https);
            let (width, height, is_gray) = escl.start_scan(&source, &color_mode, dpi, &dest)?;

            let is_pdf = dest
                .extension()
                .map_or(false, |ext| ext.eq_ignore_ascii_case("pdf"));

            if is_pdf && let Ok(raw_jpg) = std::fs::read(&dest) {
                let pdf_bytes = crate::escl::jpeg_to_pdf(&raw_jpg, width, height, dpi, is_gray);
                let _ = std::fs::write(&dest, &pdf_bytes);
            }

            Ok(json!(ScanDocumentResult {
                status: "success".to_string(),
                saved_path: dest.to_string_lossy().to_string(),
                format: if is_pdf {
                    "application/pdf".to_string()
                } else {
                    "image/jpeg".to_string()
                },
            }))
        }

        "get_scanner_capabilities" | "printer_get_scanner_capabilities" => {
            let scanner_ip = get_str_arg(&request.arguments, "scannerIp", "scanner_ip")
                .or_else(|| get_str_arg(&request.arguments, "printerIp", "printer_ip"))
                .ok_or_else(|| "Missing 'scannerIp' parameter".to_string())?;

            if scanner_ip.trim().is_empty() {
                return Err("Parameter 'scannerIp' cannot be empty".to_string());
            }

            let (host, port, use_https) = resolve_scanner_endpoint(&scanner_ip, &request.arguments);
            let escl = EsclClient::new(&host, port, use_https);
            let caps = escl.query_capabilities(&host)?;

            Ok(json!(caps))
        }

        unknown => Err(format!("Unknown scan tool: {}", unknown)),
    }
}
