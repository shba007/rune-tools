use rune_pdk::{RiskLevel, ToolDefinition, ToolEffect, ToolRules};
use serde_json::json;

pub fn tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "scan_document".to_string(),
            description: "Scans a document from flatbed glass or ADF via eSCL AirScan directly into PDF or JPEG.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "outputPath": {
                        "type": "string",
                        "description": "Destination file path on host disk (.pdf or .jpg/.jpeg)"
                    },
                    "scannerIp": {
                        "type": "string",
                        "description": "Scanner IP address or hostname"
                    },
                    "resolutionDpi": {
                        "type": "integer",
                        "enum": [75, 100, 150, 200, 300, 400, 600, 1200],
                        "default": 300,
                        "description": "Scan resolution in DPI"
                    },
                    "colorMode": {
                        "type": "string",
                        "enum": ["Color", "Grayscale", "BlackAndWhite1"],
                        "default": "Color",
                        "description": "Color capture mode"
                    },
                    "inputSource": {
                        "type": "string",
                        "enum": ["Platen", "Adf"],
                        "default": "Platen",
                        "description": "Scanner input tray/glass source"
                    }
                },
                "required": ["outputPath", "scannerIp"]
            }),
            rules: Some(ToolRules::new(
                false,
                RiskLevel::Low,
                vec![ToolEffect::Write, ToolEffect::Network],
            )),
        },
        ToolDefinition {
            name: "get_scanner_capabilities".to_string(),
            description: "Queries supported scan resolutions, input platens, and color modes via eSCL AirScan.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "scannerIp": {
                        "type": "string",
                        "description": "Scanner IP address or hostname"
                    }
                },
                "required": ["scannerIp"]
            }),
            rules: Some(ToolRules::read_only_network(["*"])),
        },
    ]
}
