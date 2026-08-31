use extism::*;
use serde_json::Value;
use std::path::PathBuf;

fn get_wasm_path(plugin_name: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push(format!(
        "target/wasm32-wasip1/release/{}.wasm",
        plugin_name.replace('-', "_")
    ));
    path
}

#[test]
fn test_wasm_filesystem_execution() {
    let wasm_file = get_wasm_path("rune_filesystem");
    if !wasm_file.exists() {
        eprintln!(
            "WASM artifact not found, skipping. Run `cargo build --target wasm32-wasip1 --release`"
        );
        return;
    }

    let manifest = Manifest::new([Wasm::file(wasm_file)])
        .with_config([("allowed_dir".to_string(), ".".to_string())])
        .with_allowed_path(".", "/");

    let mut plugin = Plugin::new(&manifest, [], true).expect("WASM plugin compilation failed");

    // Test mcp_list_tools
    let tools_raw = plugin
        .call::<(), &str>("mcp_list_tools", ())
        .expect("Failed to call mcp_list_tools");
    let tools: Value = serde_json::from_str(tools_raw).unwrap();
    assert!(tools.as_array().unwrap().len() >= 13);

    // Test mcp_call_tool (read_text_file with paging)
    let call_input = serde_json::json!({
        "name": "read_text_file",
        "arguments": {
            "path": "Cargo.toml",
            "head": 5
        }
    })
    .to_string();

    let output_raw = plugin
        .call::<&str, &str>("mcp_call_tool", &call_input)
        .expect("Failed to call mcp_call_tool");
    let output: Value = serde_json::from_str(output_raw).unwrap();
    assert_eq!(output["status"], "success");
    assert!(output["result"]["content"]
        .as_str()
        .unwrap()
        .contains("[workspace]"));
}
