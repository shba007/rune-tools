use mhb_mconnect::operations::execute_tool;
use rune_pdk::ToolCallRequest;
use serde_json::json;

// =========================================================================
// 1. Validation & Unit Tests
// =========================================================================

#[test]
fn test_mhb_missing_template_id_validation() {
    let req = ToolCallRequest {
        name: "mhb_get_template".to_string(),
        arguments: json!({}),
    };
    let res = execute_tool(req);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Missing 'templateId' parameter"));
}

#[test]
fn test_mhb_empty_template_id_validation() {
    let req = ToolCallRequest {
        name: "mhb_get_template".to_string(),
        arguments: json!({ "templateId": "   " }),
    };
    let res = execute_tool(req);
    assert!(res.is_err());
    assert!(
        res.unwrap_err()
            .contains("Parameter 'templateId' cannot be empty")
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
// 2. Live API Integration Tests
// =========================================================================

#[test]
fn test_live_mhb_list_templates() {
    if std::env::var("CI").is_ok() {
        eprintln!("Skipping live API test: Running in CI environment");
        return;
    }

    let req = ToolCallRequest {
        name: "mhb_list_templates".to_string(),
        arguments: json!({}),
    };

    match execute_tool(req) {
        Ok(res) => {
            let templates = res["templates"]
                .as_array()
                .expect("Expected templates array");
            assert!(
                !templates.is_empty(),
                "Expected at least one template from API"
            );
            println!(
                "Successfully listed {} templates from live API",
                templates.len()
            );
        }
        Err(e) => {
            eprintln!("Live API test skipped or connection failed: {}", e);
        }
    }
}

#[test]
fn test_live_mhb_get_template_and_preview() {
    if std::env::var("CI").is_ok() {
        eprintln!("Skipping live API test: Running in CI environment");
        return;
    }

    // 1. Fetch specific template schema
    let req_get = ToolCallRequest {
        name: "mhb_get_template".to_string(),
        arguments: json!({
            "templateId": "internship-completion-certificate"
        }),
    };

    let detail_res = match execute_tool(req_get) {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Live API test skipped or connection failed: {}", e);
            return;
        }
    };

    assert_eq!(detail_res["id"], "internship-completion-certificate");
    assert!(detail_res["variables"].is_object());

    // 2. Render template preview with payload variables
    let req_preview = ToolCallRequest {
        name: "mhb_render_template_preview".to_string(),
        arguments: json!({
            "templateId": "internship-completion-certificate",
            "variables": {
                "recipientName": "Alex Mercer",
                "recipientRole": "Senior Marketing Intern",
                "scopeOfWork": "Digital Campaign Management",
                "startDate": "June 1, 2025",
                "endDate": "December 31, 2025",
                "organization": {
                    "id": "modest-human-brands",
                    "name": "Modest Human Brands",
                    "address": "Abc Road, Near DEF, UIO - 1890",
                    "website": "https://modesthumanbrands.com",
                    "branding": {
                        "logo": "https://modesthumanbrands.com/logo.svg",
                        "color": {
                            "primary": "#2B2B2B",
                            "accent": "#4A85FF"
                        },
                        "font": "Exo2"
                    }
                }
            }
        }),
    };

    let preview_res = execute_tool(req_preview).expect("Failed to render template preview");
    let html = preview_res["contentHtml"]
        .as_str()
        .expect("Expected contentHtml string");

    assert!(
        html.contains("<!DOCTYPE html"),
        "Rendered preview should be valid HTML"
    );
    assert!(
        html.contains("Alex Mercer"),
        "Rendered preview should inject recipient name"
    );
    assert!(
        html.contains("Modest Human Brands"),
        "Rendered preview should inject organization details"
    );

    println!(
        "Successfully rendered template preview (HTML length: {} bytes)",
        html.len()
    );
}
