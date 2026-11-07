use mhb_mconnect::{definitions::tool_definitions, operations::execute_tool};
use rune_pdk::test_plugin_contract;

test_plugin_contract!(tool_definitions, execute_tool);
