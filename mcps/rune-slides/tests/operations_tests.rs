use rune_pdk::ToolCallRequest;
use rune_slides::operations::execute_tool;
use serde_json::json;
use tempfile::tempdir;

#[test]
fn test_iterative_slide_workflow() {
    let dir = tempdir().unwrap();
    let project_path = dir.path().join("presentation.json");
    let output_path = dir.path().join("presentation.pptx");
    let path_str = project_path.to_str().unwrap();

    let res_init = execute_tool(ToolCallRequest {
        name: "slide_init".to_string(),
        arguments: json!({
            "projectPath": path_str,
            "title": "Rune Architecture",
            "themeName": "dark"
        }),
    })
    .unwrap();
    assert_eq!(res_init["status"], "success");

    let res_theme = execute_tool(ToolCallRequest {
        name: "slide_update_theme".to_string(),
        arguments: json!({
            "projectPath": path_str,
            "themeDefinition": "body { background: #000; color: #fff; }"
        }),
    })
    .unwrap();
    assert_eq!(res_theme["status"], "success");

    let res_add = execute_tool(ToolCallRequest {
        name: "slide_add_page".to_string(),
        arguments: json!({
            "projectPath": path_str,
            "slideTitle": "Modular Tools",
            "content": "LLMs can iteratively build decks page by page."
        }),
    })
    .unwrap();
    assert_eq!(res_add["totalSlides"], 2);

    let res_del = execute_tool(ToolCallRequest {
        name: "slide_delete_page".to_string(),
        arguments: json!({
            "projectPath": path_str,
            "index": 0
        }),
    })
    .unwrap();
    assert_eq!(res_del["remainingSlides"], 1);

    let res_export = execute_tool(ToolCallRequest {
        name: "slide_export".to_string(),
        arguments: json!({
            "projectPath": path_str,
            "outputPath": output_path.to_str().unwrap(),
            "format": "pptx"
        }),
    })
    .unwrap();
    assert_eq!(res_export["status"], "success");
    assert!(output_path.exists(), "Exported PPTX file must exist");
}
