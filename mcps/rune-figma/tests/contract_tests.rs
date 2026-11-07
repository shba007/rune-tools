use rune_figma::{
    definitions::{resource_definitions, tool_definitions},
    operations::{execute_tool, read_resource},
};
use rune_pdk::test_plugin_contract;

test_plugin_contract!(
    tool_definitions,
    execute_tool,
    resources: resource_definitions,
    read_resource
);
