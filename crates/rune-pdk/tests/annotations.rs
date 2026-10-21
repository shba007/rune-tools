use rune_pdk::{RiskLevel, ToolDefinition, ToolEffect, ToolRules};
use serde_json::json;

#[test]
fn annotations_follow_rules_and_omit_unknown_idempotence() {
    for effect in [
        ToolEffect::Read,
        ToolEffect::Write,
        ToolEffect::Delete,
        ToolEffect::Exec,
        ToolEffect::Network,
        ToolEffect::Send,
    ] {
        let rules = ToolRules::new(effect == ToolEffect::Read, RiskLevel::Medium, vec![effect]);
        let tool = ToolDefinition::new("example", "Example tool", json!({"type":"object"}))
            .with_rules(rules);
        let wire = serde_json::to_value(tool).unwrap();
        let ann = &wire["annotations"];
        assert_eq!(ann["readOnly"], effect == ToolEffect::Read);
        assert_eq!(ann["readOnlyHint"], ann["readOnly"]);
        assert_eq!(
            ann["destructive"],
            matches!(effect, ToolEffect::Delete | ToolEffect::Exec)
        );
        assert_eq!(ann["destructiveHint"], ann["destructive"]);
        assert_eq!(ann["openWorld"], effect == ToolEffect::Network);
        assert_eq!(ann["openWorldHint"], ann["openWorld"]);
        assert!(ann.get("idempotent").is_none());
        assert!(ann.get("idempotentHint").is_none());
        assert_eq!(wire["rules"], wire["inputSchema"]["x-rune-rules"]);
    }
}

#[test]
fn missing_rules_have_no_annotations() {
    let wire =
        serde_json::to_value(ToolDefinition::new("unknown", "Unknown tool", json!({}))).unwrap();
    assert!(wire.get("annotations").is_none());
    assert!(wire.get("rules").is_none());
}

#[test]
fn external_and_unconfined_helpers_are_not_read_only() {
    for rules in [
        ToolRules::read_only_network(["*"]),
        ToolRules::read_only_exec(["git"]),
        ToolRules::read_only_fs("allowed_dir"),
    ] {
        assert!(!rules.read_only);
    }
}

#[test]
#[should_panic(expected = "read_only cannot")]
fn contract_rejects_read_only_delete() {
    let tool =
        ToolDefinition::new("unsafe_tool", "Unsafe tool", json!({"type":"object"})).with_rules(
            ToolRules::new(true, RiskLevel::Low, vec![ToolEffect::Delete]),
        );
    rune_pdk::testing::assert_valid_tool_rules(&[tool]);
}
