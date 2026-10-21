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

/// Native sidecar build targets — mirrors rune-kit's own deploy.yml matrix.
const NATIVE_TARGETS: [(&str, &str, &str); 4] = [
    ("ubuntu-latest", "x86_64-unknown-linux-gnu", "tar.gz"),
    ("macos-latest", "aarch64-apple-darwin", "tar.gz"),
    ("macos-latest", "x86_64-apple-darwin", "tar.gz"),
    ("windows-latest", "x86_64-pc-windows-msvc", "zip"),
];

struct Plugin {
    name: String,
    version: String,
    description: String,
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
            let names_of = |kind: &str| -> Vec<String> {
                targets
                    .iter()
                    .filter(|t| {
                        t["kind"]
                            .as_array()
                            .is_some_and(|k| k.iter().any(|k| k == kind))
                    })
                    .filter_map(|t| t["name"].as_str().map(String::from))
                    .collect()
            };
            let cdylib = names_of("cdylib").into_iter().next();
            let bins = names_of("bin");
            if cdylib.is_none() && bins.is_empty() {
                return None; // nothing buildable — e.g. a lib-only helper crate
            }

            Some(Plugin {
                name: pkg["name"].as_str()?.to_string(),
                version: pkg["version"].as_str()?.to_string(),
                description: pkg["description"].as_str().unwrap_or_default().to_string(),
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

fn asset_url(repo: &str, tag: &str, file: &str) -> String {
    format!("https://github.com/{repo}/releases/download/{tag}/{file}")
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

/// Merges every unrecorded plugin version into registry/index.json, leaving
/// all previously published versions untouched. Run after build-wasm /
/// build-native have uploaded their release assets.
pub fn update_registry(metadata: &Value) -> Result<(), String> {
    let repo = env::var("GITHUB_REPOSITORY")
        .map_err(|_| "GITHUB_REPOSITORY is not set (e.g. shba007/rune-tools)".to_string())?;
    let mut registry = load_registry()?;
    let mut changed = false;

    for p in discover(metadata) {
        if registry[&p.name]["versions"].get(&p.version).is_some() {
            continue; // this run didn't publish a new version for this plugin
        }

        let tag = format!("{}-v{}", p.name, p.version);
        let mut version_entry = json!({});

        if p.cdylib.is_some() {
            version_entry["url"] = json!(asset_url(
                &repo,
                &tag,
                &format!("{}-{}.wasm", p.name, p.version)
            ));
        }
        if !p.bins.is_empty() {
            let mut native = json!({});
            for bin_name in &p.bins {
                for (_, target, ext) in NATIVE_TARGETS {
                    native[target] = json!(asset_url(
                        &repo,
                        &tag,
                        &format!("{bin_name}-{}-{target}.{ext}", p.version)
                    ));
                }
            }
            version_entry["native"] = native;
        }

        let entry = &mut registry[&p.name];
        if !entry.is_object() {
            *entry = json!({ "versions": {} });
        }
        entry["versions"][&p.version] = version_entry;
        entry["latest"] = json!(p.version);
        entry["description"] = json!(p.description);
        changed = true;
    }

    if changed {
        if let Some(dir) = Path::new(REGISTRY_PATH).parent() {
            fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let pretty = serde_json::to_string_pretty(&registry).map_err(|e| e.to_string())?;
        fs::write(REGISTRY_PATH, pretty + "\n").map_err(|e| e.to_string())?;
        println!("{REGISTRY_PATH} updated");
    } else {
        println!("No new publishable versions — registry unchanged");
    }
    Ok(())
}
