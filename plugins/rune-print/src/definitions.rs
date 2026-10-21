use rune_pdk::{RiskLevel, ToolDefinition, ToolEffect, ToolRules};
use serde_json::json;

pub fn tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "printer_list_printers".to_string(),
            description: "Lists all local, USB, and network printers configured in the host OS (Windows Spooler / CUPS).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
            rules: Some(ToolRules::read_only_network(["*"])),
        },
        ToolDefinition {
            name: "printer_print_document".to_string(),
            description: "Dispatches TXT, PDF, or image files to an OS printer. The host OS spooler automatically handles driver conversions.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "filePath": {
                        "type": "string",
                        "description": "Path to document on disk"
                    },
                    "printerName": {
                        "type": "string",
                        "description": "Exact OS printer name. Omit to use the system default."
                    },
                    "copies": {
                        "type": "integer",
                        "default": 1,
                        "description": "Number of document copies to print"
                    }
                },
                "required": ["filePath"]
            }),
            rules: Some(ToolRules::new(
                false,
                RiskLevel::High,
                vec![ToolEffect::Send],
            )),
        },
        ToolDefinition {
            name: "printer_scan_document".to_string(),
            description: "Scans a document from the flatbed/ADF via eSCL AirScan directly into PDF or JPEG.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "outputPath": {
                        "type": "string",
                        "description": "Destination file path on host disk"
                    },
                    "printerIp": {
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
                "required": ["outputPath", "printerIp"]
            }),
            rules: Some(ToolRules::new(
                false,
                RiskLevel::Low,
                vec![ToolEffect::Write, ToolEffect::Network],
            )),
        },
        ToolDefinition {
            name: "printer_get_scanner_capabilities".to_string(),
            description: "Queries supported scan resolutions and platen dimensions via eSCL.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "printerIp": {
                        "type": "string",
                        "description": "Scanner IP address or hostname"
                    }
                },
                "required": ["printerIp"]
            }),
            rules: Some(ToolRules::read_only_network(["*"])),
        },
    ]
}
