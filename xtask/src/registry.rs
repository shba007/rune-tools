// xtask/src/registry.rs
//
// Release-matrix detection and registry/index.json maintenance, used by
// publish.yml. Replaces the former scripts/{detect-publishable,update-registry}.py
// so the pipeline needs no Python.
//
// Release tag convention: "<plugin-name>-v<version>"
// Asset naming convention:
//   wasm:   "<plugin-name>-<version>.wasm"
//   native: "<bin-name>-<version>-<target-triple>.<ext>"

use serde_json::{Value, json};
use std::{env, fs, io::Write, path::Path};

const REGISTRY_PATH: &str = "registry/index.json";

pub fn execution_model(manifest: &Path) -> String {
    let path = manifest.parent().unwrap().join("plugin.toml");
    let raw = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let config: toml::Value = toml::from_str(&raw).expect("valid plugin.toml");
    let model = config["plugin"]["execution_model"]
        .as_str()
        .expect("plugin.execution_model must be declared");
    assert!(
        matches!(model, "wasm" | "native"),
        "unknown execution model: {model}"
    );
    model.to_string()
}

fn select_targets(name: &str, model: &str, targets: &[Value]) -> (Option<String>, Vec<String>) {
    let has_kind = |t: &Value, kind: &str| {
        t["kind"]
            .as_array()
            .is_some_and(|k| k.iter().any(|v| v == kind))
    };
    if model == "wasm" {
        let wasm = targets
            .iter()
            .find(|t| has_kind(t, "cdylib"))
            .and_then(|t| t["name"].as_str())
            .expect("WASM plugin must have a cdylib");
        (Some(wasm.to_string()), vec![])
    } else {
        let native_name = format!("{name}-native");
        assert!(
            targets
                .iter()
                .any(|t| has_kind(t, "bin") && t["name"] == native_name),
            "native plugin must have {native_name}"
        );
        (None, vec![native_name])
    }
}

/// Native sidecar build targets — mirrors rune-kit's own deploy.yml matrix.
pub(crate) const NATIVE_TARGETS: [(&str, &str, &str); 4] = [
    ("ubuntu-latest", "x86_64-unknown-linux-gnu", "tar.gz"),
    ("macos-latest", "aarch64-apple-darwin", "tar.gz"),
    ("macos-latest", "x86_64-apple-darwin", "tar.gz"),
    ("windows-latest", "x86_64-pc-windows-msvc", "zip"),
];

struct Plugin {
    name: String,
    version: String,
    cdylib: Option<String>,
    bins: Vec<String>,
}

/// Every `plugins/*` package that produces a wasm or native artifact.
fn discover(metadata: &Value) -> Vec<Plugin> {
    let packages = metadata["packages"].as_array().cloned().unwrap_or_default();

    packages
        .iter()
        .filter_map(|pkg| {
            let manifest = Path::new(pkg["manifest_path"].as_str()?);
            if !manifest.components().any(|c| c.as_os_str() == "plugins") {
                return None; // crates/rune-pdk, crates/rune-sidecar, xtask, etc.
            }

            let targets = pkg["targets"].as_array().cloned().unwrap_or_default();
            let (cdylib, bins) =
                select_targets(pkg["name"].as_str()?, &execution_model(manifest), &targets);
            if cdylib.is_none() && bins.is_empty() {
                return None; // nothing buildable — e.g. a lib-only helper crate
            }

            Some(Plugin {
                name: pkg["name"].as_str()?.to_string(),
                version: pkg["version"].as_str()?.to_string(),
                cdylib,
                bins,
            })
        })
        .collect()
}

fn load_registry() -> Result<Value, String> {
    match fs::read_to_string(REGISTRY_PATH) {
        Ok(raw) => serde_json::from_str(&raw)
            .map_err(|e| format!("{REGISTRY_PATH} is not valid JSON: {e}")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(json!({})),
        Err(e) => Err(format!("cannot read {REGISTRY_PATH}: {e}")),
    }
}

/// Diffs each plugin's Cargo.toml version against registry/index.json and emits
/// wasm/native build matrices containing only versions that aren't published
/// yet. Writes step outputs to `$GITHUB_OUTPUT` when set, else prints JSON.
pub fn detect_publishable(metadata: &Value) -> Result<(), String> {
    let registry = load_registry()?;
    let (mut wasm, mut native) = (Vec::new(), Vec::new());

    for p in discover(metadata) {
        if registry[&p.name]["versions"].get(&p.version).is_some() {
            continue;
        }

        if let Some(artifact_name) = &p.cdylib {
            wasm.push(json!({
                "name": p.name,
                "version": p.version,
                "artifact_name": artifact_name,
            }));
        }
        for bin_name in &p.bins {
            for (platform, target, ext) in NATIVE_TARGETS {
                native.push(json!({
                    "name": p.name,
                    "version": p.version,
                    "bin_name": bin_name,
                    "platform": platform,
                    "target": target,
                    "ext": ext,
                }));
            }
        }
    }

    let outputs = [
        ("wasm_matrix", json!({ "include": wasm }).to_string()),
        ("native_matrix", json!({ "include": native }).to_string()),
        ("has_wasm", (!wasm.is_empty()).to_string()),
        ("has_native", (!native.is_empty()).to_string()),
    ];

    match env::var("GITHUB_OUTPUT") {
        Ok(path) => {
            let mut file = fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(&path)
                .map_err(|e| format!("cannot open {path}: {e}"))?;
            for (key, value) in &outputs {
                writeln!(file, "{key}={value}").map_err(|e| e.to_string())?;
            }
        }
        Err(_) => {
            for (key, value) in &outputs {
                println!("{key}={value}");
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_release_excludes_cli_wrapper_and_unused_wasm() {
        let targets = vec![
            json!({"name":"rune_git","kind":["cdylib","rlib"]}),
            json!({"name":"rune-git","kind":["bin"]}),
            json!({"name":"rune-git-native","kind":["bin"]}),
        ];
        assert_eq!(
            select_targets("rune-git", "native", &targets),
            (None, vec!["rune-git-native".to_string()])
        );
        assert_eq!(
            select_targets("rune-git", "wasm", &targets),
            (Some("rune_git".to_string()), vec![])
        );
    }
}
