pub mod testing;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Component, Path, PathBuf};

#[cfg(target_arch = "wasm32")]
pub use extism_pdk::*;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    #[default]
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolEffect {
    Read,
    Write,
    Delete,
    Send,
    Network,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ToolLimits {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_param: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub network_hosts: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_binaries: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ToolRules {
    #[serde(default)]
    pub read_only: bool,
    #[serde(default)]
    pub risk: RiskLevel,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<ToolEffect>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limits: Option<ToolLimits>,
}

impl ToolRules {
    pub fn new(read_only: bool, risk: RiskLevel, effects: Vec<ToolEffect>) -> Self {
        Self {
            read_only,
            risk,
            effects,
            limits: None,
        }
    }

    pub fn with_limits(mut self, limits: ToolLimits) -> Self {
        self.limits = Some(limits);
        self
    }

    pub fn read_only() -> Self {
        Self {
            read_only: true,
            risk: RiskLevel::Low,
            effects: vec![ToolEffect::Read],
            limits: None,
        }
    }

    pub fn read_only_fs(root_param: impl Into<String>) -> Self {
        Self {
            read_only: true,
            risk: RiskLevel::Low,
            effects: vec![ToolEffect::Read],
            limits: Some(ToolLimits {
                root_param: Some(root_param.into()),
                ..Default::default()
            }),
        }
    }

    pub fn read_only_network(hosts: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            read_only: true,
            risk: RiskLevel::Low,
            effects: vec![ToolEffect::Read, ToolEffect::Network],
            limits: Some(ToolLimits {
                network_hosts: hosts.into_iter().map(Into::into).collect(),
                ..Default::default()
            }),
        }
    }

    pub fn read_only_exec(binaries: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            read_only: true,
            risk: RiskLevel::Low,
            effects: vec![ToolEffect::Read],
            limits: Some(ToolLimits {
                allowed_binaries: binaries.into_iter().map(Into::into).collect(),
                ..Default::default()
            }),
        }
    }

    pub fn write_fs(root_param: impl Into<String>, risk: RiskLevel) -> Self {
        Self {
            read_only: false,
            risk,
            effects: vec![ToolEffect::Write],
            limits: Some(ToolLimits {
                root_param: Some(root_param.into()),
                ..Default::default()
            }),
        }
    }

    pub fn delete_fs(root_param: impl Into<String>, risk: RiskLevel) -> Self {
        Self {
            read_only: false,
            risk,
            effects: vec![ToolEffect::Delete],
            limits: Some(ToolLimits {
                root_param: Some(root_param.into()),
                ..Default::default()
            }),
        }
    }

    pub fn mutate_fs(
        root_param: impl Into<String>,
        risk: RiskLevel,
        effects: Vec<ToolEffect>,
    ) -> Self {
        Self {
            read_only: false,
            risk,
            effects,
            limits: Some(ToolLimits {
                root_param: Some(root_param.into()),
                ..Default::default()
            }),
        }
    }

    pub fn network(
        risk: RiskLevel,
        effects: Vec<ToolEffect>,
        hosts: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            read_only: false,
            risk,
            effects,
            limits: Some(ToolLimits {
                network_hosts: hosts.into_iter().map(Into::into).collect(),
                ..Default::default()
            }),
        }
    }

    pub fn exec(
        risk: RiskLevel,
        effects: Vec<ToolEffect>,
        binaries: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            read_only: false,
            risk,
            effects,
            limits: Some(ToolLimits {
                allowed_binaries: binaries.into_iter().map(Into::into).collect(),
                ..Default::default()
            }),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema", alias = "input_schema")]
    pub input_schema: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rules: Option<ToolRules>,
}

impl Serialize for ToolDefinition {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("ToolDefinition", 4)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("description", &self.description)?;

        if let Some(ref rules) = self.rules {
            let mut schema = self.input_schema.clone();
            if let Value::Object(ref mut map) = schema
                && !map.contains_key("x-rune-rules")
                && let Ok(rv) = serde_json::to_value(rules)
            {
                map.insert("x-rune-rules".to_string(), rv);
            }
            state.serialize_field("inputSchema", &schema)?;
            state.serialize_field("rules", rules)?;
        } else {
            state.serialize_field("inputSchema", &self.input_schema)?;
        }
        state.end()
    }
}

impl ToolDefinition {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: Value,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema,
            rules: None,
        }
    }

    pub fn with_rules(mut self, rules: ToolRules) -> Self {
        self.set_rules(rules);
        self
    }

    pub fn set_rules(&mut self, rules: ToolRules) {
        if let Value::Object(ref mut map) = self.input_schema
            && let Ok(val) = serde_json::to_value(&rules)
        {
            map.insert("x-rune-rules".to_string(), val);
        }
        self.rules = Some(rules);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequest {
    pub name: String,
    pub arguments: Value,
}

#[derive(Serialize, Deserialize)]
pub struct Page<T> {
    pub items: T,
    pub cursor: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceDefinition {
    pub uri: String,
    pub name: String,
    pub description: String,
    #[serde(
        rename = "mimeType",
        alias = "mime_type",
        skip_serializing_if = "Option::is_none"
    )]
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceReadRequest {
    pub uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptArgument {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptDefinition {
    pub name: String,
    pub description: String,
    #[serde(rename = "arguments", alias = "args", default)]
    pub arguments: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptGetRequest {
    pub name: String,
    #[serde(default)]
    pub arguments: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BinaryProvisionStatus {
    Installed,
    Provisioning,
    Missing,
    UnsupportedPlatform,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryStatus {
    pub name: String,
    pub status: BinaryProvisionStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

pub fn get_config(key: &str) -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        extism_pdk::config::get(key).ok().flatten()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let upper = key.to_ascii_uppercase();
        let lower = key.to_ascii_lowercase();
        std::env::var(&upper)
            .or_else(|_| std::env::var(&lower))
            .or_else(|_| std::env::var(key))
            .ok()
    }
}

pub fn is_contract_test_mode() -> bool {
    std::env::var("RUNE_CONTRACT_TEST").as_deref() == Ok("1")
}

pub fn resolve_binary_executable(program: &str) -> Result<String, String> {
    #[cfg(target_arch = "wasm32")]
    {
        Ok(program.to_string())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        if is_contract_test_mode() {
            return Ok(program.to_string());
        }

        let upper_key = format!("{}_PATH", program.replace('-', "_").to_ascii_uppercase());
        if let Ok(explicit) = std::env::var(&upper_key) {
            let p = PathBuf::from(&explicit);
            if p.exists() {
                return Ok(p.to_string_lossy().to_string());
            }
        }

        let lower_key = format!("{}_path", program.replace('-', "_").to_ascii_lowercase());
        if let Ok(explicit) = std::env::var(&lower_key) {
            let p = PathBuf::from(&explicit);
            if p.exists() {
                return Ok(p.to_string_lossy().to_string());
            }
        }

        let exe_name = if cfg!(windows) && !program.ends_with(".exe") {
            format!("{}.exe", program)
        } else {
            program.to_string()
        };

        let local_dirs = [
            std::env::var("ALLOWED_DIR").ok(),
            std::env::var("OUTPUT_DIR").ok(),
            Some(".".to_string()),
        ];

        for base in local_dirs.into_iter().flatten() {
            let candidate_bin = PathBuf::from(&base).join("bin").join(&exe_name);
            if candidate_bin.is_file() {
                return Ok(candidate_bin.to_string_lossy().to_string());
            }
            let candidate_direct = PathBuf::from(&base).join(&exe_name);
            if candidate_direct.is_file() {
                return Ok(candidate_direct.to_string_lossy().to_string());
            }
        }

        if let Ok(path_var) = std::env::var("PATH") {
            for dir in std::env::split_paths(&path_var) {
                let candidate = dir.join(&exe_name);
                if candidate.is_file() {
                    return Ok(candidate.to_string_lossy().to_string());
                }
            }
        }

        if std::process::Command::new(program)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Ok(program.to_string());
        }

        Err(format!(
            "Required external binary '{}' not found. It was neither provisioned by rune-kit (checked environment variable '{}') nor found on the system PATH.",
            program, upper_key
        ))
    }
}

pub fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => match components.last() {
                Some(Component::Normal(_)) => {
                    components.pop();
                }
                Some(Component::RootDir) | Some(Component::Prefix(_)) => {}
                _ => components.push(Component::ParentDir),
            },
            c => components.push(c),
        }
    }
    components.into_iter().collect()
}

pub fn resolve_scoped_path(
    relative_or_abs: &str,
    allowed_root: Option<&str>,
) -> Result<PathBuf, String> {
    let clean_input = relative_or_abs.trim().replace('\\', "/");
    if clean_input.is_empty() {
        return Err("Path cannot be empty".to_string());
    }

    if let Some(root_str) = allowed_root.filter(|s| !s.trim().is_empty()) {
        let clean_root = root_str.trim().replace('\\', "/");
        let root_trimmed = clean_root.trim_end_matches('/');
        let root_path = PathBuf::from(root_trimmed);
        let normalized_root = normalize_path(&root_path);

        let is_absolute = clean_input.starts_with('/')
            || clean_input.contains(":/")
            || clean_input.starts_with("//")
            || Path::new(&clean_input).is_absolute();

        let full_path = if is_absolute {
            PathBuf::from(&clean_input)
        } else {
            root_path.join(&clean_input)
        };

        let normalized = normalize_path(&full_path);

        let root_str_norm = normalized_root.to_string_lossy();
        let norm_str = normalized.to_string_lossy();

        let is_contained = if cfg!(windows) {
            norm_str
                .to_ascii_lowercase()
                .starts_with(&root_str_norm.to_ascii_lowercase())
        } else {
            normalized.starts_with(&normalized_root)
        };

        if !is_contained {
            return Err(format!(
                "Access denied: path '{}' is outside allowed directory '{}'",
                relative_or_abs, root_str
            ));
        }

        Ok(normalized)
    } else {
        Ok(normalize_path(&PathBuf::from(clean_input)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path(Path::new("a/b/../c")), PathBuf::from("a/c"));
        assert_eq!(normalize_path(Path::new("a/./b/c")), PathBuf::from("a/b/c"));
    }

    #[test]
    fn test_resolve_scoped_path_within_root() {
        let res = resolve_scoped_path("file.txt", Some("/safe/dir"));
        assert!(res.is_ok());
        let path = res.unwrap();
        assert!(path.to_string_lossy().contains("safe"));
    }

    #[test]
    fn test_resolve_scoped_path_rejection_on_escape() {
        let res = resolve_scoped_path("../file.txt", Some("/safe/dir"));
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Access denied"));

        let res2 = resolve_scoped_path("sub/../../file.txt", Some("/safe/dir"));
        assert!(res2.is_err());
        assert!(res2.unwrap_err().contains("Access denied"));
    }

    #[test]
    fn test_resolve_scoped_path_rejection_on_absolute() {
        let res = resolve_scoped_path("/etc/shadow", Some("/safe/dir"));
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Access denied"));
    }

    #[test]
    fn test_resolve_scoped_path_empty_rejected() {
        let res = resolve_scoped_path("", Some("/safe/dir"));
        assert!(res.is_err());
    }

    #[test]
    fn test_tool_definition_rules_serialization() {
        let tool = ToolDefinition::new(
            "test_read",
            "Reads a file safely",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "file path" }
                }
            }),
        )
        .with_rules(ToolRules::read_only_fs("allowed_dir"));

        let serialized = serde_json::to_string(&tool).expect("serialize tool");
        let val: serde_json::Value = serde_json::from_str(&serialized).expect("parse json");

        // Verify top-level rules field
        assert_eq!(val["rules"]["read_only"], true);
        assert_eq!(val["rules"]["risk"], "low");
        assert_eq!(val["rules"]["effects"][0], "read");
        assert_eq!(val["rules"]["limits"]["root_param"], "allowed_dir");

        // Verify injected x-rune-rules inside inputSchema
        assert_eq!(val["inputSchema"]["x-rune-rules"]["read_only"], true);
        assert_eq!(val["inputSchema"]["x-rune-rules"]["risk"], "low");
        assert_eq!(val["inputSchema"]["x-rune-rules"]["effects"][0], "read");

        // Verify deserialization back to ToolDefinition
        let deserialized: ToolDefinition = serde_json::from_str(&serialized).expect("deserialize");
        assert_eq!(deserialized.name, "test_read");
        assert!(deserialized.rules.is_some());
        let rules = deserialized.rules.unwrap();
        assert!(rules.read_only);
        assert_eq!(rules.risk, RiskLevel::Low);
    }
}
