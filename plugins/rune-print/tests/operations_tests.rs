use rune_pdk::ToolCallRequest;
use rune_print::escl::jpeg_to_pdf;
use rune_print::operations::execute_tool;
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

// Prevents concurrent tests from locking up physical hardware
static HARDWARE_LOCK: Mutex<()> = Mutex::new(());

fn get_test_printer_ip() -> Option<String> {
    std::env::var("TEST_PRINTER_IP")
        .or_else(|_| std::env::var("PRINTER_IP"))
        .ok()
        .filter(|s| !s.trim().is_empty())
}

fn setup_workspace() -> PathBuf {
    let base = std::env::var("TEST_BASE_DIR")
        .or_else(|_| std::env::var("ALLOWED_DIR"))
        .or_else(|_| std::env::var("OUTPUT_DIR"))
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("rune-print-test"));

    if !base.exists() {
        let _ = fs::create_dir_all(&base);
    }
    base
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
fn test_printer_print_document_validation() {
    let req_missing = ToolCallRequest {
        name: "printer_print_document".to_string(),
        arguments: json!({}),
    };
    let res_missing = execute_tool(req_missing);
    assert!(res_missing.is_err());
    assert!(
        res_missing
            .unwrap_err()
            .contains("Missing 'filePath' parameter")
    );

    let req_empty = ToolCallRequest {
        name: "printer_print_document".to_string(),
        arguments: json!({ "filePath": "   " }),
    };
    let res_empty = execute_tool(req_empty);
    assert!(res_empty.is_err());
    assert!(
        res_empty
            .unwrap_err()
            .contains("Parameter 'filePath' cannot be empty")
    );
}

#[test]
fn test_printer_scan_document_validation() {
    let dummy_ip = get_test_printer_ip().unwrap_or_else(|| "127.0.0.1".to_string());

    let req_missing = ToolCallRequest {
        name: "printer_scan_document".to_string(),
        arguments: json!({ "printerIp": dummy_ip }),
    };
    let res_missing = execute_tool(req_missing);
    assert!(res_missing.is_err());
    assert!(
        res_missing
            .unwrap_err()
            .contains("Missing 'outputPath' parameter")
    );

    let req_empty = ToolCallRequest {
        name: "printer_scan_document".to_string(),
        arguments: json!({ "printerIp": dummy_ip, "outputPath": "   " }),
    };
    let res_empty = execute_tool(req_empty);
    assert!(res_empty.is_err());
    assert!(
        res_empty
            .unwrap_err()
            .contains("Parameter 'outputPath' cannot be empty")
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
// 3. Real OS Spooler & Hardware Integration Tests
// =========================================================================

#[test]
fn test_live_printer_list_printers_live() {
    if std::env::var("CI").is_ok() {
        eprintln!("Skipping hardware test: Running in CI");
        return;
    }

    let _guard = HARDWARE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let req = ToolCallRequest {
        name: "printer_list_printers".to_string(),
        arguments: json!({}),
    };

    let res = execute_tool(req).expect("Failed to fetch OS printers");
    assert!(
        res["printers"].is_array(),
        "Expected an array of OS printers"
    );
}

#[test]
fn test_live_printer_get_scanner_capabilities_live() {
    if std::env::var("CI").is_ok() {
        eprintln!("Skipping hardware test: Running in CI");
        return;
    }

    let Some(printer_ip) = get_test_printer_ip() else {
        eprintln!("Skipping hardware test: TEST_PRINTER_IP or PRINTER_IP not set in environment");
        return;
    };

    let _guard = HARDWARE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let req = ToolCallRequest {
        name: "printer_get_scanner_capabilities".to_string(),
        arguments: json!({ "printerIp": printer_ip }),
    };

    let res = execute_tool(req).expect("Failed to fetch scanner capabilities from hardware");
    let xml = res["capabilities_xml"]
        .as_str()
        .expect("Expected XML response");
    assert!(
        xml.contains("ScannerCapabilities") || xml.contains("pwg:Version"),
        "Invalid eSCL XML returned by scanner: {}",
        xml
    );
}

#[test]
fn test_live_print_document_e2e() {
    if std::env::var("CI").is_ok() {
        eprintln!("Skipping hardware test: Running in CI");
        return;
    }

    let _guard = HARDWARE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let workspace = setup_workspace();
    let print_file = workspace.join("real_print_sample.txt");
    fs::write(
        &print_file,
        "=== RUNE SYSTEM TEST PAGE ===\nTarget: Default OS Printer\nProtocol: OS Spooler\nStatus: Verified\n",
    )
    .expect("Failed to stage real print test file");

    let req = ToolCallRequest {
        name: "printer_print_document".to_string(),
        arguments: json!({
            "filePath": print_file.to_str().unwrap(),
            "copies": 1
        }),
    };

    let res = execute_tool(req).expect("Failed to dispatch print job to OS spooler");
    assert_eq!(res["status"], "success");
    assert_eq!(res["copies"], 1);
    assert_eq!(res["engine"], "OS Spooler Native Pipeline");
}

#[test]
fn test_live_scan_document_to_pdf_e2e() {
    if std::env::var("CI").is_ok() {
        eprintln!("Skipping hardware test: Running in CI");
        return;
    }

    let Some(printer_ip) = get_test_printer_ip() else {
        eprintln!("Skipping hardware test: TEST_PRINTER_IP or PRINTER_IP not set in environment");
        return;
    };

    let _guard = HARDWARE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let workspace = setup_workspace();
    let scan_output = workspace.join("real_scanned_document.pdf");
    if scan_output.exists() {
        let _ = fs::remove_file(&scan_output);
    }

    let req = ToolCallRequest {
        name: "printer_scan_document".to_string(),
        arguments: json!({
            "printerIp": printer_ip,
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
}
