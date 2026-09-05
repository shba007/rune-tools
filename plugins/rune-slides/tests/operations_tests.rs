use rune_pdk::ToolCallRequest;
use rune_slides::operations::execute_tool;
use serde_json::json;
use tempfile::tempdir;

#[test]
fn test_iterative_slide_workflow() {
    let dir = tempdir().unwrap();
    let project_path = dir.path().join("presentation.json");
    let output_path = dir.path().join("output.html");
    let path_str = project_path.to_str().unwrap();

    // 1. Init
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

    // 2. Update Theme
    let res_theme = execute_tool(ToolCallRequest {
        name: "slide_update_theme".to_string(),
        arguments: json!({
            "projectPath": path_str,
            "themeDefinition": "body { background: #000; color: #fff; }"
        }),
    })
    .unwrap();
    assert_eq!(res_theme["status"], "success");

    // 3. Add Page
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

    // 4. Delete Page (remove the initial default title slide at index 0)
    let res_del = execute_tool(ToolCallRequest {
        name: "slide_delete_page".to_string(),
        arguments: json!({
            "projectPath": path_str,
            "index": 0
        }),
    })
    .unwrap();
    assert_eq!(res_del["remainingSlides"], 1);

    // 5. Export
    let res_export = execute_tool(ToolCallRequest {
        name: "slide_export".to_string(),
        arguments: json!({
            "projectPath": path_str,
            "outputPath": output_path.to_str().unwrap()
        }),
    })
    .unwrap();
    assert_eq!(res_export["status"], "success");
    assert!(output_path.exists(), "Exported HTML file must exist");
}
