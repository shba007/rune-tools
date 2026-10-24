//! Additive metadata for the latest published plugins. Downloads are never executed.
use crate::release::{self, Plugin, Result};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, io::Read, path::Path, time::Duration};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Configuration {
    schema_version: u32,
    fields: BTreeMap<String, Field>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Field {
    #[serde(rename = "type")]
    kind: String,
    description: String,
    // Required for the relevant operations unless supplied by their call arguments.
    required: bool,
    secret: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    // Native environment names, in precedence order. Values are serialized as strings.
    env: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    // WASM Extism configuration key; not an environment variable.
    config_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    default: Option<Value>,
}

fn configuration(raw: &str) -> Result<Value> {
    let manifest: toml::Value = toml::from_str(raw)?;
    let config: Configuration = manifest
        .get("configuration")
        .ok_or("plugin.toml must declare configuration, including an empty fields table")?
        .clone()
        .try_into()?;
    if config.schema_version != 1 {
        return Err("unsupported configuration schema_version".into());
    }
    for (key, field) in &config.fields {
        if key.is_empty()
            || !key
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
            || field.description.trim().is_empty()
            || (field.env.is_empty() && field.config_key.is_none())
            || field.env.iter().any(|s| {
                s.is_empty()
                    || !s
                        .chars()
                        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
            })
            || field
                .config_key
                .as_ref()
                .is_some_and(|s| s.trim().is_empty())
        {
            return Err(format!("invalid configuration field {key}").into());
        }
        if !matches!(
            field.kind.as_str(),
            "string" | "integer" | "number" | "boolean"
        ) {
            return Err(format!("{key}: unsupported configuration type").into());
        }
        if let Some(default) = &field.default {
            let valid = match field.kind.as_str() {
                "string" => default.is_string(),
                "integer" => default.is_i64() || default.is_u64(),
                "number" => default.is_number(),
                "boolean" => default.is_boolean(),
                _ => false,
            };
            if !valid || field.secret {
                return Err(format!("{key}: invalid or secret configuration default").into());
            }
        }
    }
    Ok(serde_json::to_value(config)?)
}

trait Published {
    fn release(&self, repo: &str, tag: &str) -> Result<Value>;
    fn digest(&self, url: &str, size: u64) -> Result<String>;
}

struct Http(Client);

impl Published for Http {
    fn release(&self, repo: &str, tag: &str) -> Result<Value> {
        Ok(self
            .0
            .get(format!(
                "https://api.github.com/repos/{repo}/releases/tags/{tag}"
            ))
            .send()?
            .error_for_status()?
            .json()?)
    }

    fn digest(&self, url: &str, size: u64) -> Result<String> {
        let mut response = self.0.get(url).send()?.error_for_status()?;
        digest_download(&mut response, size).map_err(|e| format!("{url}: {e}").into())
    }
}

fn digest_download(reader: &mut impl Read, size: u64) -> Result<String> {
    let mut hash = Sha256::new();
    let mut count = 0_u64;
    let mut buffer = [0_u8; 65536];
    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        count += n as u64;
        if count > size {
            return Err("download exceeds published size".into());
        }
        hash.update(&buffer[..n]);
    }
    if count == 0 || count != size {
        return Err("empty or truncated download".into());
    }
    Ok(hash.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

fn verified_checksums(
    remote: &impl Published,
    repo: &str,
    p: &Plugin,
    entry: &Value,
) -> Result<Value> {
    let tag = format!("{}-v{}", p.name, p.version);
    let release = remote.release(repo, &tag)?;
    if release["tag_name"] != tag || release["draft"] != false || release["prerelease"] != false {
        return Err(format!("{tag}: unexpected release tag/status").into());
    }
    let assets = release["assets"]
        .as_array()
        .ok_or("release assets missing")?;
    let expected = crate::publication::payloads(p);
    let native_count = entry
        .get("native")
        .and_then(Value::as_object)
        .map_or(0, |v| v.len());
    if (p.model == "native" && (native_count != expected.len() || entry.get("url").is_some()))
        || (p.model == "wasm" && entry.get("native").is_some())
    {
        return Err(format!("{}: unexpected latest platform mapping", p.name).into());
    }
    let mut checksums = json!({});
    for (platform, name) in expected {
        let url = format!("https://github.com/{repo}/releases/download/{tag}/{name}");
        let recorded = if platform == "wasm" {
            &entry["url"]
        } else {
            &entry["native"][&platform]
        };
        if recorded != &url {
            return Err(
                format!("{name}: registry URL differs from expected published payload").into(),
            );
        }
        let matching: Vec<_> = assets.iter().filter(|a| a["name"] == name).collect();
        if matching.len() != 1 {
            return Err(format!("{name}: missing/duplicate published asset").into());
        }
        let asset = matching[0];
        if asset["browser_download_url"] != url {
            return Err(format!("{name}: unexpected asset URL").into());
        }
        let size = asset["size"]
            .as_u64()
            .filter(|s| *s > 0)
            .ok_or("empty/invalid asset size")?;
        eprintln!("Verifying {name} ({size} bytes)");
        let digest = remote.digest(&url, size)?;
        if let Some(advertised) = asset.get("digest").filter(|v| !v.is_null())
            && advertised != &format!("sha256:{digest}")
        {
            return Err(format!("{name}: published digest mismatch").into());
        }
        if let Some(previous) = entry.get("checksums").and_then(|c| c.get(&name))
            && previous["sha256"] != digest
        {
            return Err(format!("{name}: existing registry digest mismatch").into());
        }
        let mut checksum = json!({"sha256":digest,"size":size});
        let side_name = format!("{name}.sha256");
        if let Some(side) = assets.iter().find(|a| a["name"] == side_name) {
            let text = format!("{digest}  {name}\n");
            let side_url = format!("{url}.sha256");
            if side["browser_download_url"] != side_url
                || side["size"] != text.len() as u64
                || remote.digest(&side_url, text.len() as u64)?
                    != crate::publication::hash(text.as_bytes())
            {
                return Err(format!("{name}: checksum sidecar mismatch").into());
            }
            checksum["url"] = json!(side_url);
        } else if entry["checksums"][&name].get("url").is_some() {
            return Err(format!("{name}: recorded checksum sidecar disappeared").into());
        }
        checksums[&name] = checksum;
    }
    Ok(checksums)
}

fn enrich(root: &Path, repo: &str, index: &Value, remote: &impl Published) -> Result<Value> {
    let mut result = index.clone();
    for p in release::plugins(root)? {
        if index[&p.name]["latest"] != p.version {
            return Err(format!(
                "{}: source version must match published latest before metadata backfill",
                p.name
            )
            .into());
        }
        let config = configuration(&fs::read_to_string(
            root.join(&p.folder).join("plugin.toml"),
        )?)?;
        let entry = &index[&p.name]["versions"][&p.version];
        if !entry.is_object() {
            return Err(format!("{}: latest version entry missing", p.name).into());
        }
        let checksums = verified_checksums(remote, repo, &p, entry)?;
        let updated = &mut result[&p.name]["versions"][&p.version];
        updated["configuration"] = config;
        updated["checksums"] = checksums;
    }
    Ok(result)
}

pub fn enrich_latest(root: &Path, repo: &str, index: &Value) -> Result<Value> {
    if repo.split('/').count() != 2
        || repo.split('/').any(|s| s.is_empty())
        || !repo
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_./".contains(c))
    {
        return Err("expected repository owner/name".into());
    }
    let client = Client::builder()
        .user_agent("rune-tools-registry")
        .connect_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(300))
        .build()?;
    enrich(root, repo, index, &Http(client))
}

/// Writes a candidate only, never source, Git state, or GitHub releases.
pub fn command(args: &[String]) -> Result<()> {
    let option = |name: &str| -> Result<&str> {
        let i = args
            .iter()
            .position(|s| s == name)
            .ok_or_else(|| format!("{name} required"))?;
        Ok(args
            .get(i + 1)
            .filter(|s| !s.starts_with("--"))
            .ok_or("option value missing")?
            .as_str())
    };
    let root = Path::new(".");
    let out = Path::new(option("--output")?);
    fs::create_dir_all(out)?;
    if out.canonicalize()? == root.join("registry").canonicalize()? {
        return Err("metadata output must be separate from source registry".into());
    }
    let index = release::registry(root)?;
    let candidate = enrich_latest(root, option("--repo")?, &index)?;
    fs::write(
        out.join("index.json"),
        serde_json::to_string_pretty(&candidate)? + "\n",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake {
        release: Value,
        digests: BTreeMap<String, String>,
    }
    impl Published for Fake {
        fn release(&self, _: &str, _: &str) -> Result<Value> {
            Ok(self.release.clone())
        }
        fn digest(&self, url: &str, _: u64) -> Result<String> {
            self.digests
                .get(url)
                .cloned()
                .ok_or_else(|| "download failed".into())
        }
    }

    fn fixture() -> (Plugin, Value, Fake) {
        let p = Plugin {
            name: "test".into(),
            version: "1.0.0".into(),
            model: "wasm".into(),
            folder: "".into(),
            description: "".into(),
        };
        let url = "https://github.com/a/b/releases/download/test-v1.0.0/test-1.0.0.wasm";
        let digest = crate::publication::hash(b"payload");
        let release = json!({"tag_name":"test-v1.0.0","draft":false,"prerelease":false,"assets":[{"name":"test-1.0.0.wasm","browser_download_url":url,"size":7,"digest":format!("sha256:{digest}")}]});
        (
            p,
            json!({"url":url}),
            Fake {
                release,
                digests: BTreeMap::from([(url.into(), digest)]),
            },
        )
    }

    #[test]
    fn hashes_legacy_download_without_inventing_sidecar() {
        let (p, entry, remote) = fixture();
        let c = verified_checksums(&remote, "a/b", &p, &entry).unwrap();
        assert_eq!(c["test-1.0.0.wasm"]["size"], 7);
        assert!(c["test-1.0.0.wasm"].get("url").is_none());
        let mut legacy = remote;
        legacy.release["assets"][0]
            .as_object_mut()
            .unwrap()
            .remove("digest");
        assert_eq!(verified_checksums(&legacy, "a/b", &p, &entry).unwrap(), c);
    }

    #[test]
    fn refuses_corrupt_missing_or_replaced_assets() {
        let (p, entry, remote) = fixture();
        let mut bad = Fake {
            release: remote.release.clone(),
            digests: remote.digests.clone(),
        };
        bad.release["assets"][0]["digest"] = json!("sha256:wrong");
        assert!(verified_checksums(&bad, "a/b", &p, &entry).is_err());
        bad.release["assets"] = json!([]);
        assert!(verified_checksums(&bad, "a/b", &p, &entry).is_err());
        let mut old = entry.clone();
        old["checksums"]["test-1.0.0.wasm"] = json!({"sha256":"wrong"});
        assert!(verified_checksums(&remote, "a/b", &p, &old).is_err());
        let mut failed = remote;
        failed.digests.clear();
        assert!(verified_checksums(&failed, "a/b", &p, &entry).is_err());
    }

    #[test]
    fn rejects_unknown_fields_wrong_defaults_and_secret_defaults() {
        let text = "[configuration]\nschema_version=1\n[configuration.fields.password]\ntype='string'\ndescription='Account password'\nrequired=true\nsecret=true\nenv=['PASSWORD']\n";
        assert!(configuration(text).is_ok());
        assert!(configuration(&format!("{text}default='secret'\n")).is_err());
        assert!(configuration(&format!("{text}typo=true\n")).is_err());
        assert!(configuration(&text.replace("type='string'", "type='invalid'")).is_err());
        assert!(configuration(&text.replace("env=['PASSWORD']", "env=[]")).is_err());
        assert!(configuration(&text.replace("secret=true", "secret=false\ndefault=42")).is_err());
    }

    #[test]
    fn all_active_plugins_declare_configuration() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let plugins = release::plugins(root).unwrap();
        assert_eq!(plugins.len(), 14);
        for p in plugins {
            configuration(&fs::read_to_string(root.join(p.folder).join("plugin.toml")).unwrap())
                .unwrap();
        }
    }

    #[test]
    fn checked_in_latest_metadata_covers_every_download() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let index = release::registry(root).unwrap();
        for p in release::plugins(root).unwrap() {
            let Some(latest) = index[&p.name]["latest"].as_str() else {
                continue;
            };
            let entry = &index[&p.name]["versions"][latest];
            let mut urls = Vec::new();
            if let Some(url) = entry["url"].as_str() {
                urls.push(url);
            }
            if let Some(native) = entry["native"].as_object() {
                urls.extend(native.values().map(|v| v.as_str().unwrap()));
            }
            assert!(!urls.is_empty(), "{}: no downloads", p.name);
            let checksums = entry["checksums"]
                .as_object()
                .expect("latest checksums required");
            assert_eq!(checksums.len(), urls.len(), "{}: checksum coverage", p.name);
            for url in urls {
                let name = url.rsplit('/').next().unwrap();
                let checksum = &checksums[name];
                let digest = checksum["sha256"].as_str().unwrap();
                assert!(digest.len() == 64 && digest.chars().all(|c| c.is_ascii_hexdigit()));
                assert!(checksum["size"].as_u64().unwrap() > 0);
                if let Some(sidecar) = checksum.get("url") {
                    assert_eq!(sidecar, &format!("{url}.sha256"));
                }
            }
            let _: Configuration = serde_json::from_value(entry["configuration"].clone()).unwrap();
            if latest == p.version {
                let manifest =
                    fs::read_to_string(root.join(&p.folder).join("plugin.toml")).unwrap();
                assert_eq!(
                    entry["configuration"],
                    configuration(&manifest).unwrap(),
                    "{}: configuration drift",
                    p.name
                );
            }
        }
    }

    #[test]
    fn streaming_digest_rejects_empty_truncated_and_oversized_downloads() {
        assert_eq!(
            digest_download(&mut &b"payload"[..], 7).unwrap(),
            crate::publication::hash(b"payload")
        );
        assert!(digest_download(&mut &b""[..], 0).is_err());
        assert!(digest_download(&mut &b"short"[..], 6).is_err());
        assert!(digest_download(&mut &b"long"[..], 3).is_err());
    }

    #[test]
    fn verifies_sidecar_bytes_when_present() {
        let (p, entry, mut remote) = fixture();
        let url = entry["url"].as_str().unwrap();
        let digest = &remote.digests[url];
        let text = format!("{digest}  test-1.0.0.wasm\n");
        let side_url = format!("{url}.sha256");
        remote.release["assets"].as_array_mut().unwrap().push(json!({"name":"test-1.0.0.wasm.sha256","browser_download_url":side_url,"size":text.len()}));
        remote
            .digests
            .insert(side_url.clone(), crate::publication::hash(text.as_bytes()));
        assert_eq!(
            verified_checksums(&remote, "a/b", &p, &entry).unwrap()["test-1.0.0.wasm"]["url"],
            side_url
        );
        remote.digests.insert(side_url, "bad".into());
        assert!(verified_checksums(&remote, "a/b", &p, &entry).is_err());
    }

    #[test]
    fn requires_every_native_platform_and_preserves_urls() {
        let (mut p, _, mut remote) = fixture();
        p.model = "native".into();
        remote.release["assets"] = json!([]);
        remote.digests.clear();
        let mut entry = json!({"native":{}});
        for (platform, name) in crate::publication::payloads(&p) {
            let url = format!("https://github.com/a/b/releases/download/test-v1.0.0/{name}");
            entry["native"][platform] = json!(url);
            remote.release["assets"]
                .as_array_mut()
                .unwrap()
                .push(json!({"name":name,"browser_download_url":url,"size":7}));
            remote
                .digests
                .insert(url, crate::publication::hash(b"payload"));
        }
        let original = entry.clone();
        assert_eq!(
            verified_checksums(&remote, "a/b", &p, &entry)
                .unwrap()
                .as_object()
                .unwrap()
                .len(),
            4
        );
        assert_eq!(entry, original);
        remote.release["assets"].as_array_mut().unwrap().pop();
        assert!(verified_checksums(&remote, "a/b", &p, &entry).is_err());
    }

    #[test]
    fn enrichment_is_additive_and_refuses_source_version_drift() {
        let (p, entry, remote) = fixture();
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        fs::create_dir_all(root.join("plugins/test")).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers=['plugins/test']\n",
        )
        .unwrap();
        let manifest = "[package]\nname='test'\nversion='1.0.0'\n";
        fs::write(root.join("plugins/test/Cargo.toml"), manifest).unwrap();
        let plugin = "[plugin]\nname='test'\nversion='1.0.0'\nexecution_model='wasm'\n[configuration]\nschema_version=1\n[configuration.fields]\n";
        fs::write(root.join("plugins/test/plugin.toml"), plugin).unwrap();
        let index = json!({"test":{"latest":p.version,"description":"keep","versions":{"0.9.0":{"url":"historical","custom":true},"1.0.0":entry}}});
        let enriched = enrich(root, "a/b", &index, &remote).unwrap();
        assert_eq!(
            enriched["test"]["versions"]["0.9.0"],
            index["test"]["versions"]["0.9.0"]
        );
        assert_eq!(enriched["test"]["description"], "keep");
        assert_eq!(enriched["test"]["versions"]["1.0.0"]["url"], entry["url"]);
        assert_eq!(enrich(root, "a/b", &enriched, &remote).unwrap(), enriched);
        fs::write(
            root.join("plugins/test/Cargo.toml"),
            manifest.replace("1.0.0", "1.1.0"),
        )
        .unwrap();
        fs::write(
            root.join("plugins/test/plugin.toml"),
            plugin.replace("1.0.0", "1.1.0"),
        )
        .unwrap();
        assert!(enrich(root, "a/b", &index, &remote).is_err());
    }
}
