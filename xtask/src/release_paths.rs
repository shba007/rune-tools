//! Root manifest/lock changes select only packages whose inputs actually changed.
use super::{Plugin, Result, git};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

fn workspace_keys(
    root: &Path,
    folder: &str,
    seen: &mut BTreeSet<String>,
) -> Result<BTreeSet<String>> {
    if !seen.insert(folder.into()) {
        return Ok(BTreeSet::new());
    }
    let doc: toml::Value =
        toml::from_str(&fs::read_to_string(root.join(folder).join("Cargo.toml"))?)?;
    fn collect(value: &toml::Value, keys: &mut BTreeSet<String>, local: &mut BTreeSet<String>) {
        if let Some(table) = value.as_table() {
            for (key, value) in table {
                if matches!(key.as_str(), "dependencies" | "build-dependencies") {
                    if let Some(table) = value.as_table() {
                        for (name, value) in table {
                            if value.get("workspace").and_then(toml::Value::as_bool) == Some(true) {
                                keys.insert(name.clone());
                            }
                            local.insert(name.clone());
                        }
                    }
                } else if key != "dev-dependencies" {
                    collect(value, keys, local)
                }
            }
        }
    }
    let mut keys = BTreeSet::new();
    let mut local = BTreeSet::new();
    collect(&doc, &mut keys, &mut local);
    for name in local {
        let folder = format!("crates/{name}");
        if root.join(&folder).join("Cargo.toml").exists() {
            keys.extend(workspace_keys(root, &folder, seen)?);
        }
    }
    Ok(keys)
}
fn manifest_inputs(text: &str, folder: &str, keys: &BTreeSet<String>) -> Result<toml::Value> {
    let mut doc: toml::Value = toml::from_str(text)?;
    if let Some(workspace) = doc.get_mut("workspace").and_then(toml::Value::as_table_mut) {
        if let Some(deps) = workspace
            .get_mut("dependencies")
            .and_then(toml::Value::as_table_mut)
        {
            deps.retain(|name, _| keys.contains(name));
        }
        if let Some(members) = workspace
            .get_mut("members")
            .and_then(toml::Value::as_array_mut)
        {
            members.retain(|m| m.as_str() == Some(folder));
        }
    }
    Ok(doc)
}
fn lock_inputs(text: &str, name: &str) -> Result<BTreeMap<String, toml::Value>> {
    let doc: toml::Value = toml::from_str(text)?;
    let packages = doc["package"]
        .as_array()
        .ok_or("Cargo.lock lacks package records")?;
    let key = |p: &toml::Value| {
        format!(
            "{}@{}:{}",
            p["name"].as_str().unwrap_or_default(),
            p["version"].as_str().unwrap_or_default(),
            p.get("source")
                .and_then(toml::Value::as_str)
                .unwrap_or_default()
        )
    };
    let mut queue = packages
        .iter()
        .enumerate()
        .filter(|(_, p)| p["name"].as_str() == Some(name))
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    let mut inputs = BTreeMap::new();
    while let Some(index) = queue.pop() {
        let package = &packages[index];
        let id = key(package);
        if inputs.contains_key(&id) {
            continue;
        }
        let mut normalized = package.clone();
        let mut dependencies = BTreeSet::new();
        for dep in package
            .get("dependencies")
            .and_then(toml::Value::as_array)
            .into_iter()
            .flatten()
        {
            let parts = dep
                .as_str()
                .ok_or("invalid lock dependency")?
                .split_whitespace()
                .collect::<Vec<_>>();
            for (i, candidate) in packages.iter().enumerate() {
                if candidate["name"].as_str() == parts.first().copied()
                    && (parts.len() == 1 || candidate["version"].as_str() == parts.get(1).copied())
                {
                    dependencies.insert(key(candidate));
                    queue.push(i);
                }
            }
        }
        // Version qualifiers may disappear when an unrelated duplicate is removed.
        // Compare resolved identities, not Cargo.lock's abbreviated edge spelling.
        normalized
            .as_table_mut()
            .ok_or("invalid lock package")?
            .insert(
                "dependencies".into(),
                toml::Value::Array(dependencies.into_iter().map(toml::Value::String).collect()),
            );
        inputs.insert(id, normalized);
    }
    Ok(inputs)
}
pub(super) fn runtime_paths(
    root: &Path,
    p: &Plugin,
    id: &str,
    mut paths: Vec<String>,
) -> Result<Vec<String>> {
    for file in ["Cargo.toml", "Cargo.lock"] {
        if !paths.iter().any(|s| s == file) {
            continue;
        }
        let new = git(root, &["show", &format!("{id}:{file}")])?;
        let Ok(old) = git(root, &["show", &format!("{id}^:{file}")]) else {
            continue;
        };
        let same = if file == "Cargo.toml" {
            let keys = workspace_keys(root, &p.folder, &mut BTreeSet::new())?;
            manifest_inputs(&old, &p.folder, &keys)? == manifest_inputs(&new, &p.folder, &keys)?
        } else {
            lock_inputs(&old, &p.name)? == lock_inputs(&new, &p.name)?
        };
        if same {
            paths.retain(|s| s != file)
        }
    }
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lock_edges_ignore_unrelated_versions_and_abbreviation_changes() {
        let old = "[[package]]\nname='plugin'\nversion='1.0.0'\ndependencies=['dep 1.0.0']\n[[package]]\nname='dep'\nversion='1.0.0'\n[[package]]\nname='dep'\nversion='2.0.0'\n";
        let new = "[[package]]\nname='plugin'\nversion='1.0.0'\ndependencies=['dep']\n[[package]]\nname='dep'\nversion='1.0.0'\n";
        assert_eq!(
            lock_inputs(old, "plugin").unwrap(),
            lock_inputs(new, "plugin").unwrap()
        );
        assert_ne!(
            lock_inputs(old, "plugin").unwrap(),
            lock_inputs(
                &new.replace("name='dep'\nversion='1.0.0'", "name='dep'\nversion='1.0.1'"),
                "plugin"
            )
            .unwrap()
        );
    }
    #[test]
    fn unused_workspace_dependencies_do_not_select_a_plugin() {
        let old = "[workspace]\nmembers=['plugins/a','xtask']\n[workspace.dependencies]\nused='1'\nunused='7'\n";
        let new = "[workspace]\nmembers=['plugins/a','plugins/b','xtask']\n[workspace.dependencies]\nused='1'\n";
        let keys = BTreeSet::from(["used".into()]);
        assert_eq!(
            manifest_inputs(old, "plugins/a", &keys).unwrap(),
            manifest_inputs(new, "plugins/a", &keys).unwrap()
        );
        assert_ne!(
            manifest_inputs(old, "plugins/a", &keys).unwrap(),
            manifest_inputs(&new.replace("used='1'", "used='2'"), "plugins/a", &keys).unwrap()
        );
    }
    #[test]
    fn release_tool_migration_does_not_release_plugins() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        for p in super::super::plugins(root).unwrap() {
            let changes = super::super::generator::history(root, &p, "c94ca35^..c94ca35").unwrap();
            assert!(changes.is_empty(), "{}: {:?}", p.name, changes);
        }
    }
}
