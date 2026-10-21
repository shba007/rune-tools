#[cfg(not(target_arch = "wasm32"))]
use crate::escl::EsclClient;
#[cfg(target_arch = "wasm32")]
use crate::types::{CmdExecRequest, CmdExecResponse};
#[cfg(not(target_arch = "wasm32"))]
use crate::types::{
    PrintDocumentResult, PrinterInfo, ScanDocumentResult, ScannerCapabilitiesResult,
};
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
        program: "rune-print-native".to_string(),
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
            "rune-print-native exited with failure".to_string()
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
// Native Target Implementation (Direct OS Spooler & Network eSCL)
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
pub fn execute_tool(request: ToolCallRequest) -> Result<Value, String> {
    match request.name.as_str() {
        "printer_list_printers" => {
            let printers_list = printers::get_printers();
            let list: Vec<PrinterInfo> = printers_list
                .into_iter()
                .map(|p| PrinterInfo {
                    name: p.name,
                    system_name: p.system_name,
                    driver_name: p.driver_name,
                    uri: p.uri,
                    location: p.location,
                    is_default: p.is_default,
                    is_shared: p.is_shared,
                })
                .collect();
            Ok(json!({ "printers": list }))
        }

        "printer_print_document" => {
            let file_path_str = get_str_arg(&request.arguments, "filePath", "file_path")
                .ok_or_else(|| "Missing 'filePath' parameter".to_string())?;

            if file_path_str.trim().is_empty() {
                return Err("Parameter 'filePath' cannot be empty".to_string());
            }

            let path = resolve_path(&file_path_str);
            if !path.exists() {
                return Err(format!("File '{}' does not exist", path.display()));
            }

            let copies = get_u64_arg(&request.arguments, "copies", "copies").unwrap_or(1) as usize;
            let printer_name =
                get_str_arg(&request.arguments, "printerName", "printer_name").unwrap_or_default();

            for _ in 0..copies {
                #[cfg(target_os = "windows")]
                {
                    let ext = path
                        .extension()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_lowercase();

                    let (script, script_args) = if ext == "txt" || ext == "md" || ext == "log" {
                        if printer_name.trim().is_empty() {
                            (
                                "param($p) Get-Content -LiteralPath $p | Out-Printer",
                                vec![path.to_string_lossy().to_string()],
                            )
                        } else {
                            (
                                "param($p, $n) Get-Content -LiteralPath $p | Out-Printer -Name $n",
                                vec![path.to_string_lossy().to_string(), printer_name.clone()],
                            )
                        }
                    } else {
                        (
                            "param($p) Start-Process -FilePath $p -Verb Print -WindowStyle Hidden -Wait",
                            vec![path.to_string_lossy().to_string()],
                        )
                    };

                    let mut cmd = std::process::Command::new("powershell");
                    cmd.args(&["-NoProfile", "-NonInteractive", "-Command", script]);
                    for arg in script_args {
                        cmd.arg(arg);
                    }

                    let output = cmd
                        .output()
                        .map_err(|e| format!("Windows spooler execution error: {}", e))?;

                    if !output.status.success() {
                        return Err(format!(
                            "Print command failed: {}",
                            String::from_utf8_lossy(&output.stderr)
                        ));
                    }
                }

                #[cfg(not(target_os = "windows"))]
                {
                    let mut cmd = std::process::Command::new("lp");
                    if !printer_name.trim().is_empty() {
                        cmd.arg("-d").arg(&printer_name);
                    }
                    cmd.arg("--").arg(&path);
                    let output = cmd
                        .output()
                        .map_err(|e| format!("CUPS spooler error: {}", e))?;

                    if !output.status.success() {
                        return Err(format!(
                            "CUPS command failed: {}",
                            String::from_utf8_lossy(&output.stderr)
                        ));
                    }
                }
            }

            let os_printer_label = if printer_name.trim().is_empty() {
                "default"
            } else {
                &printer_name
            };

            Ok(json!(PrintDocumentResult {
                status: "success".to_string(),
                engine: "OS Spooler Native Pipeline".to_string(),
                file: path.to_string_lossy().to_string(),
                copies,
                os_printer: os_printer_label.to_string(),
            }))
        }

        "printer_scan_document" => {
            let printer_ip = get_str_arg(&request.arguments, "printerIp", "printer_ip")
                .ok_or_else(|| "Missing 'printerIp' parameter".to_string())?;
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

            let escl = EsclClient::new(&printer_ip, 80, false);
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

        "printer_get_scanner_capabilities" => {
            let printer_ip = get_str_arg(&request.arguments, "printerIp", "printer_ip")
                .ok_or_else(|| "Missing 'printerIp' parameter".to_string())?;
            let escl = EsclClient::new(&printer_ip, 80, false);
            let xml_caps = escl.get_capabilities()?;

            Ok(json!(ScannerCapabilitiesResult {
                printer_ip,
                capabilities_xml: xml_caps,
            }))
        }

        unknown => Err(format!("Unknown tool: {}", unknown)),
    }
}
