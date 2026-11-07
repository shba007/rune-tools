use rune_pdk::ToolCallRequest;
use rune_scan::escl::jpeg_to_pdf;
use rune_scan::operations::execute_tool;
use serde_json::json;
use std::fs;
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

static HARDWARE_LOCK: Mutex<()> = Mutex::new(());

fn get_test_scanner_ip() -> Option<String> {
    std::env::var("TEST_SCANNER_IP")
        .or_else(|_| std::env::var("SCANNER_IP"))
        .or_else(|_| std::env::var("PRINTER_IP"))
        .ok()
        .filter(|s| !s.trim().is_empty())
}

fn setup_workspace() -> PathBuf {
    let base = std::env::var("TEST_BASE_DIR")
        .or_else(|_| std::env::var("ALLOWED_DIR"))
        .or_else(|_| std::env::var("OUTPUT_DIR"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir().join("rune-scan-test"));

    if !base.exists() {
        let _ = fs::create_dir_all(&base);
    }
    base
}

fn should_run_live_hardware_test() -> Option<String> {
    if std::env::var("CI").is_ok() {
        println!("[info] Skipping hardware test: Running in CI environment");
        return None;
    }

    let explicitly_enabled = std::env::var("TEST_LIVE_HARDWARE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
        || std::env::var("RUN_HARDWARE_TESTS")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

    if !explicitly_enabled {
        println!(
            "[info] Skipping hardware test: Set TEST_LIVE_HARDWARE=1 in .env to run against physical scanner"
        );
        return None;
    }

    let ip = get_test_scanner_ip()
        .expect("TEST_LIVE_HARDWARE=1 is set, but SCANNER_IP is not configured");

    let socket_addr = format!("{}:443", ip);
    let addr = socket_addr
        .parse()
        .unwrap_or_else(|_| panic!("Invalid socket address '{socket_addr}'"));

    TcpStream::connect_timeout(&addr, Duration::from_millis(2000)).unwrap_or_else(|e| {
        panic!("Failed to reach scanner at {socket_addr} on port 443: {e}");
    });

    Some(ip)
}

// =========================================================================
// 1. Pure Unit Tests
// =========================================================================

#[test]
fn test_jpeg_to_pdf_packaging() {
    let fake_jpeg = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46];
    let pdf = jpeg_to_pdf(&fake_jpeg, 2550, 3508, 300, false);
    let pdf_str = String::from_utf8_lossy(&pdf);

    assert!(pdf_str.starts_with("%PDF-1.4"));
    assert!(pdf_str.contains("/Width 2550"));
    assert!(pdf_str.contains("/Height 3508"));
    assert!(pdf_str.contains("/ColorSpace /DeviceRGB"));
    assert!(pdf_str.ends_with("%%EOF\n"));
}

// =========================================================================
// 2. Validation Tests
// =========================================================================

#[test]
fn test_scan_document_validation() {
    let dummy_ip = "127.0.0.1";

    let req_missing_path = ToolCallRequest {
        name: "scan_document".to_string(),
        arguments: json!({ "scannerIp": dummy_ip }),
    };
    let res_missing_path = execute_tool(req_missing_path);
    assert!(res_missing_path.is_err());
    assert!(
        res_missing_path
            .unwrap_err()
            .contains("Missing 'outputPath' parameter")
    );

    let req_missing_ip = ToolCallRequest {
        name: "scan_document".to_string(),
        arguments: json!({ "outputPath": "out.pdf" }),
    };
    let res_missing_ip = execute_tool(req_missing_ip);
    assert!(res_missing_ip.is_err());
    assert!(
        res_missing_ip
            .unwrap_err()
            .contains("Missing 'scannerIp' parameter")
    );

    let req_empty_path = ToolCallRequest {
        name: "scan_document".to_string(),
        arguments: json!({ "scannerIp": dummy_ip, "outputPath": "   " }),
    };
    let res_empty_path = execute_tool(req_empty_path);
    assert!(res_empty_path.is_err());
    assert!(
        res_empty_path
            .unwrap_err()
            .contains("Parameter 'outputPath' cannot be empty")
    );
}

#[test]
fn test_unknown_tool_routing() {
    let req = ToolCallRequest {
        name: "non_existent_scan_tool".to_string(),
        arguments: json!({}),
    };
    let res = execute_tool(req);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Unknown scan tool"));
}

// =========================================================================
// 3. Live Hardware Tests
// =========================================================================

#[test]
fn test_live_get_scanner_capabilities_live() {
    let Some(scanner_ip) = should_run_live_hardware_test() else {
        return;
    };

    let _guard = HARDWARE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let req = ToolCallRequest {
        name: "get_scanner_capabilities".to_string(),
        arguments: json!({ "scannerIp": scanner_ip }),
    };

    let res = execute_tool(req).expect("Failed to fetch scanner capabilities from hardware");
    println!(
        "\n=== Discovered Scanner Hardware Capabilities ===\n{}",
        serde_json::to_string_pretty(&res).unwrap()
    );
    assert!(res["supported_resolutions"].is_array());
    assert!(res["version"].as_str().is_some());
    assert!(res["capabilities_xml"].as_str().is_some());
}

#[test]
fn test_live_scan_document_to_pdf_e2e() {
    let Some(scanner_ip) = should_run_live_hardware_test() else {
        return;
    };

    let _guard = HARDWARE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let workspace = setup_workspace();
    let scan_output = workspace.join("real_scanned_document.pdf");
    if scan_output.exists() {
        let _ = fs::remove_file(&scan_output);
    }

    let req = ToolCallRequest {
        name: "scan_document".to_string(),
        arguments: json!({
            "scannerIp": scanner_ip,
            "outputPath": scan_output.to_str().unwrap(),
            "resolutionDpi": 300,
            "colorMode": "Color",
            "inputSource": "Platen"
        }),
    };

    let res = execute_tool(req).expect("Failed to execute real hardware scan");
    assert_eq!(res["status"], "success");
    assert_eq!(res["format"], "application/pdf");
    assert!(scan_output.exists(), "Scanned PDF was not saved to disk");
    assert!(
        fs::metadata(&scan_output).unwrap().len() > 5000,
        "Scanned PDF is unexpectedly empty"
    );

    println!("\nSuccessfully saved scan to: {}", scan_output.display());
}
