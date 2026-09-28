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
fn test_mhb_render_preview_validation() {
    // Missing templateId
    let req_missing_id = ToolCallRequest {
        name: "mhb_render_template_preview".to_string(),
        arguments: json!({ "variables": {} }),
    };
    let res = execute_tool(req_missing_id);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Missing 'templateId' parameter"));

    // Empty templateId
    let req_empty_id = ToolCallRequest {
        name: "mhb_render_template_preview".to_string(),
        arguments: json!({ "templateId": "   ", "variables": {} }),
    };
    let res = execute_tool(req_empty_id);
    assert!(res.is_err());
    assert!(
        res.unwrap_err()
            .contains("Parameter 'templateId' cannot be empty")
    );

    // Missing variables
    let req_missing_vars = ToolCallRequest {
        name: "mhb_render_template_preview".to_string(),
        arguments: json!({ "templateId": "test-id" }),
    };
    let res = execute_tool(req_missing_vars);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Missing 'variables' parameter"));

    // Invalid variables type (string instead of object)
    let req_invalid_vars = ToolCallRequest {
        name: "mhb_render_template_preview".to_string(),
        arguments: json!({ "templateId": "test-id", "variables": "not_an_object" }),
    };
    let res = execute_tool(req_invalid_vars);
    assert!(res.is_err());
    assert!(
        res.unwrap_err()
            .contains("Parameter 'variables' must be a JSON object")
    );
}

#[test]
fn test_mhb_router_prefix_handling() {
    let req = ToolCallRequest {
        name: "mhb_mconnect__mhb_get_template".to_string(),
        arguments: json!({}),
    };
    let res = execute_tool(req);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Missing 'templateId' parameter"));
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
            "templateId": "content-release",
            "variables": {
                "recipient": {
                    "name": "Sarah Jenkins",
                    "email": "sarah@example.com"
                },
                "emailSubject": "Three Ways to Deepen Your Creative Flow",
                "content": {
                    "badge": "New Release",
                    "title": "Three Ways to Deepen Your Creative Workflow",
                    "meta": "Posted by Modest Human Brands · 3 min read",
                    "imageUrl": "https://modesthumanbrands.com/images/hero-image-1.webp",
                    "excerpt": "You've been optimizing your routine for a while, and now you feel it's time to take your studio output to the next level. Here is how modern teams eliminate friction across production.",
                    "ctaLabel": "Read More",
                    "linkUrl": "https://modesthumanbrands.com/blog/creative-workflow"
                },
                "unsubscribeUrl": "https://modesthumanbrands.com/newsletter/unsubscribe",
                "trackingPixelUrl": "http://localhost:3001/api/track/open?e=test",
                "tracking": {
                    "emailId": "test-emailid-1",
                    "baseUrl": "http://localhost:3001"
                },
                "organization": {
                    "id": "modest-human-brands",
                    "name": "Modest Human Brands",
                    "legalName": "Modest Human Brands LLP",
                    "entityType": "LLP",
                    "tradeRelationship": "Primary",
                    "gstin": null,
                    "pan": "ABCDE0123F",
                    "address": "Abc Road, Near DEF, UIO - 1890",
                    "foundedYear": 2020,
                    "accountDetails": {
                        "accountName": "Modest Human Brands LLP",
                        "accountNumber": 1234567890,
                        "bankName": "HDFC Bank",
                        "ifscCode": "HDFC0001234"
                    },
                    "website": "https://modesthumanbrands.com",
                    "contactEmail": "hello@modesthumanbrands.com",
                    "billingEmail": "billing@modesthumanbrands.com",
                    "primaryContactId": "contact-1",
                    "organizationMemberIds": ["member-1"],
                    "branding": {
                        "logo": "https://modesthumanbrands.com/logo.svg",
                        "color": {
                            "primary": "#111827",
                            "accent": "#0284c7"
                        },
                        "font": "Exo2"
                    },
                    "phone": "+919999999999",
                    "whatsapp": "+919999999999",
                    "socials": {
                        "instagram": "https://www.instagram.com/modesthumanbrands/",
                        "facebook": "https://facebook.com/modesthumanbrands",
                        "linkedin": "https://linkedin.com/company/modesthumanbrands",
                        "youtube": "https://www.youtube.com/@modesthumanbrands"
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
        html.contains("Three Ways to Deepen Your Creative Workflow"),
        "Rendered preview should inject content name"
    );
    assert!(
        html.contains("Modest Human Brands"),
        "Rendered preview should inject organization details"
    );
}
