//! Audit declarations without invoking tools, including inactive draft plugins.
#![allow(dead_code)]
#[path = "../../../mcps/mhb-mconnect/src/definitions.rs"]
mod mhb_mconnect;
#[path = "../../../mcps/rune-audio/src/definitions.rs"]
mod rune_audio;
#[path = "../../../mcps/rune-browser/src/definitions.rs"]
mod rune_browser;
#[path = "../../../mcps/rune-email/src/definitions.rs"]
mod rune_email;
#[path = "../../../mcps/rune-fetch/src/definitions.rs"]
mod rune_fetch;
#[path = "../../../mcps/rune-figma/src/definitions.rs"]
mod rune_figma;
#[path = "../../../mcps/rune-filesystem/src/definitions.rs"]
mod rune_filesystem;
#[path = "../../../mcps/rune-git/src/definitions.rs"]
mod rune_git;
#[path = "../../../mcps/rune-image/src/definitions.rs"]
mod rune_image;
#[path = "../../../mcps/rune-memory/src/definitions.rs"]
mod rune_memory;
#[path = "../../../mcps/rune-print/src/definitions.rs"]
mod rune_print;
#[path = "../../../mcps/rune-scan/src/definitions.rs"]
mod rune_scan;
#[path = "../../../mcps/rune-sequential-thinking/src/definitions.rs"]
mod rune_sequential_thinking;
#[path = "../../../mcps/rune-slides/src/definitions.rs"]
mod rune_slides;
#[path = "../../../mcps/rune-ssh/src/definitions.rs"]
mod rune_ssh;
#[path = "../../../mcps/rune-time/src/definitions.rs"]
mod rune_time;
#[path = "../../../mcps/rune-video/src/definitions.rs"]
mod rune_video;
fn main() {
    let mut catalog = Vec::new();
    let tools = mhb_mconnect::tool_definitions();
    rune_pdk::testing::assert_valid_tool_rules(&tools);
    catalog.push(serde_json::json!({"plugin":"mhb-mconnect","tools":tools}));
    let tools = rune_audio::tool_definitions();
    rune_pdk::testing::assert_valid_tool_rules(&tools);
    catalog.push(serde_json::json!({"plugin":"rune-audio","tools":tools}));
    let tools = rune_browser::tool_definitions();
    rune_pdk::testing::assert_valid_tool_rules(&tools);
    catalog.push(serde_json::json!({"plugin":"rune-browser","tools":tools}));
    let tools = rune_email::tool_definitions();
    rune_pdk::testing::assert_valid_tool_rules(&tools);
    catalog.push(serde_json::json!({"plugin":"rune-email","tools":tools}));
    let tools = rune_fetch::tool_definitions();
    rune_pdk::testing::assert_valid_tool_rules(&tools);
    catalog.push(serde_json::json!({"plugin":"rune-fetch","tools":tools}));
    let tools = rune_figma::tool_definitions();
    rune_pdk::testing::assert_valid_tool_rules(&tools);
    catalog.push(serde_json::json!({"plugin":"rune-figma","tools":tools}));
    let tools = rune_filesystem::tool_definitions();
    rune_pdk::testing::assert_valid_tool_rules(&tools);
    catalog.push(serde_json::json!({"plugin":"rune-filesystem","tools":tools}));
    let tools = rune_git::tool_definitions();
    rune_pdk::testing::assert_valid_tool_rules(&tools);
    catalog.push(serde_json::json!({"plugin":"rune-git","tools":tools}));
    let tools = rune_image::tool_definitions();
    rune_pdk::testing::assert_valid_tool_rules(&tools);
    catalog.push(serde_json::json!({"plugin":"rune-image","tools":tools}));
    let tools = rune_memory::tool_definitions();
    rune_pdk::testing::assert_valid_tool_rules(&tools);
    catalog.push(serde_json::json!({"plugin":"rune-memory","tools":tools}));
    let tools = rune_print::tool_definitions();
    rune_pdk::testing::assert_valid_tool_rules(&tools);
    catalog.push(serde_json::json!({"plugin":"rune-print","tools":tools}));
    let tools = rune_scan::tool_definitions();
    rune_pdk::testing::assert_valid_tool_rules(&tools);
    catalog.push(serde_json::json!({"plugin":"rune-scan","tools":tools}));
    let tools = rune_sequential_thinking::tool_definitions();
    rune_pdk::testing::assert_valid_tool_rules(&tools);
    catalog.push(serde_json::json!({"plugin":"rune-sequential-thinking","tools":tools}));
    let tools = rune_slides::tool_definitions();
    rune_pdk::testing::assert_valid_tool_rules(&tools);
    catalog.push(serde_json::json!({"plugin":"rune-slides","tools":tools}));
    let tools = rune_ssh::tool_definitions();
    rune_pdk::testing::assert_valid_tool_rules(&tools);
    catalog.push(serde_json::json!({"plugin":"rune-ssh","tools":tools}));
    let tools = rune_time::tool_definitions();
    rune_pdk::testing::assert_valid_tool_rules(&tools);
    catalog.push(serde_json::json!({"plugin":"rune-time","tools":tools}));
    let tools = rune_video::tool_definitions();
    rune_pdk::testing::assert_valid_tool_rules(&tools);
    catalog.push(serde_json::json!({"plugin":"rune-video","tools":tools}));
    println!("{}", serde_json::to_string_pretty(&catalog).unwrap());
}
