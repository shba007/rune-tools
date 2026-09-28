use rune_fetch::{definitions, operations};
use rune_pdk::test_plugin_contract;

test_plugin_contract!(
    definitions::tool_definitions,
    operations::execute_tool,
    resources: definitions::resource_definitions,
    operations::read_resource
);
