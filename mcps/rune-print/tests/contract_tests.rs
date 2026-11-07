use rune_pdk::test_plugin_contract;
use rune_print::{definitions::tool_definitions, operations::execute_tool};

test_plugin_contract!(tool_definitions, execute_tool);
