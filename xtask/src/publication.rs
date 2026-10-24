//! Immutable GitHub publication and artifact-backed registry generation.
use crate::registry::NATIVE_TARGETS;
use crate::release::{self, Plugin, Result};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::Path, process::Command};
pub fn payloads(p: &Plugin) -> Vec<(String, String)> {
    if p.model == "wasm" {
        vec![("wasm".into(), format!("{}-{}.wasm", p.name, p.version))]
    } else {
        NATIVE_TARGETS
            .iter()
            .map(|(_, t, e)| {
                (
                    t.to_string(),
                    format!("{}-native-{}-{t}.{e}", p.name, p.version),
                )
            })
            .collect()
    }
}
fn tag(p: &Plugin) -> String {
    format!("{}-v{}", p.name, p.version)
}
pub(crate) fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
trait GitHub {
    fn run(&self, args: &[&str]) -> Result<String>;
}
struct Live;
impl GitHub for Live {
    fn run(&self, args: &[&str]) -> Result<String> {
        let o = Command::new("gh").args(args).output()?;
        if !o.status.success() {
            return Err(String::from_utf8_lossy(&o.stderr).into_owned().into());
        }
        Ok(String::from_utf8(o.stdout)?)
    }
}
fn get(client: &impl GitHub, repo: &str, path: &str) -> Result<Value> {
    Ok(serde_json::from_str(
        &client.run(&["api", &format!("repos/{repo}/{path}")])?,
    )?)
}
fn lookup(client: &impl GitHub, repo: &str, p: &Plugin) -> Result<Option<Value>> {
    match get(client, repo, &format!("releases/tags/{}", tag(p))) {
        Ok(v) => Ok(Some(v)),
        Err(e) if e.to_string().contains("HTTP 404") => Ok(None),
        Err(e) => Err(e),
    }
}
fn collision(e: &dyn std::error::Error) -> bool {
    let s = e.to_string().to_ascii_lowercase();
    s.contains("already_exists") || s.contains("already exists")
}
fn verify(
    client: &impl GitHub,
    repo: &str,
    p: &Plugin,
    r: &Value,
    body: &str,
    target: &str,
) -> Result<()> {
    if r["tag_name"] != tag(p)
        || r["draft"] != false
        || r["prerelease"] != false
        || r["body"].as_str().unwrap_or_default().trim_end() != body.trim_end()
    {
        return Err(format!(
            "{}: existing release differs from exact prepared notes/tag/status",
            tag(p)
        )
        .into());
    }
    if get(client, repo, &format!("commits/{}", tag(p)))?["sha"] != target {
        return Err("release tag differs from source SHA".into());
    }
    Ok(())
}
fn matches(a: &Value, digest: &str, size: u64) -> bool {
    a["digest"] == format!("sha256:{digest}") && a["size"] == size
}
fn asset<'a>(r: &'a Value, name: &str) -> Option<&'a Value> {
    r["assets"].as_array()?.iter().find(|a| a["name"] == name)
}
fn upload(client: &impl GitHub, repo: &str, p: &Plugin, path: &Path) -> Result<()> {
    let bytes = fs::read(path)?;
    let digest = hash(&bytes);
    let name = path
        .file_name()
        .ok_or("asset name missing")?
        .to_str()
        .ok_or("non UTF-8 filename")?;
    if let Some(a) = asset(
        &lookup(client, repo, p)?.ok_or("release disappeared")?,
        name,
    ) {
        if !matches(a, &digest, bytes.len() as u64) {
            return Err(format!(
                "{name}: existing bytes differ or digest unavailable; refusing overwrite"
            )
            .into());
        }
        return Ok(());
    }
    if let Err(e) = client.run(&[
        "release",
        "upload",
        &tag(p),
        path.to_str().ok_or("asset path invalid")?,
        "--repo",
        repo,
    ]) && !collision(e.as_ref())
    {
        return Err(e);
    }
    let r = lookup(client, repo, p)?.ok_or("release disappeared")?;
    if !asset(&r, name).is_some_and(|a| matches(a, &digest, bytes.len() as u64)) {
        return Err(format!("{name}: upload verification failed").into());
    }
    Ok(())
}
fn publish(
    client: &impl GitHub,
    root: &Path,
    repo: &str,
    p: &Plugin,
    dir: &Path,
    target: &str,
) -> Result<()> {
    let body = release::plugin_notes(root, p)?;
    let paths = payloads(p)
        .into_iter()
        .map(|(_, n)| dir.join(n))
        .collect::<Vec<_>>();
    for path in &paths {
        if !path.is_file() || path.metadata()?.len() == 0 {
            return Err(format!("missing/empty payload {}", path.display()).into());
        }
    }
    if lookup(client, repo, p)?.is_none() {
        let file = dir.join(format!("{}-notes.md", tag(p)));
        fs::write(&file, &body)?;
        if let Err(e) = client.run(&[
            "release",
            "create",
            &tag(p),
            "--repo",
            repo,
            "--target",
            target,
            "--title",
            &tag(p),
            "--notes-file",
            file.to_str().ok_or("notes path invalid")?,
        ]) && !collision(e.as_ref())
        {
            return Err(e);
        }
    }
    verify(
        client,
        repo,
        p,
        &lookup(client, repo, p)?.ok_or("release missing")?,
        &body,
        target,
    )?;
    for path in paths {
        let name = path.file_name().ok_or("name missing")?.to_string_lossy();
        let sidecar = path.with_file_name(format!("{name}.sha256"));
        fs::write(&sidecar, format!("{}  {name}\n", hash(&fs::read(&path)?)))?;
        upload(client, repo, p, &path)?;
        upload(client, repo, p, &sidecar)?;
    }
    Ok(())
}
pub fn entry(repo: &str, p: &Plugin, r: &Value) -> Result<Value> {
    let mut entry = json!({"checksums":{}});
    for (platform, name) in payloads(p) {
        let a = asset(r, &name).ok_or_else(|| format!("{name}: published payload missing"))?;
        let digest = a["digest"]
            .as_str()
            .and_then(|s| s.strip_prefix("sha256:"))
            .ok_or("payload SHA-256 missing")?;
        if digest.len() != 64
            || !digest.chars().all(|c| c.is_ascii_hexdigit())
            || a["size"].as_u64().unwrap_or(0) == 0
        {
            return Err("invalid payload digest/size".into());
        }
        let text = format!("{digest}  {name}\n");
        let sidecar = asset(r, &format!("{name}.sha256")).ok_or("checksum sidecar missing")?;
        if !matches(sidecar, &hash(text.as_bytes()), text.len() as u64) {
            return Err("checksum sidecar mismatched".into());
        }
        let url = format!(
            "https://github.com/{repo}/releases/download/{}/{name}",
            tag(p)
        );
        if a["browser_download_url"] != url
            || sidecar["browser_download_url"] != format!("{url}.sha256")
        {
            return Err("unexpected asset URL".into());
        }
        if platform == "wasm" {
            entry["url"] = json!(url)
        } else {
            entry["native"][&platform] = json!(url)
        }
        entry["checksums"][&name] = json!({"sha256":digest,"url":format!("{url}.sha256")});
    }
    Ok(entry)
}
pub fn command(args: &[String]) -> Result<()> {
    let opt = |key: &str| -> Result<&str> {
        let i = args
            .iter()
            .position(|s| s == key)
            .ok_or_else(|| format!("{key} required"))?;
        Ok(args.get(i + 1).ok_or("option value missing")?.as_str())
    };
    let repo = opt("--repo")?;
    if repo.split('/').count() != 2
        || !repo
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c) || c == '/')
    {
        return Err("expected --repo owner/repository".into());
    }
    let target = opt("--target")?;
    if target.len() != 40 || !target.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("--target must be full source commit SHA".into());
    }
    let root = Path::new(".");
    let client = &Live;
    if args[0] == "publish" {
        let p = release::plugins(root)?
            .into_iter()
            .find(|p| Some(&p.name) == args.get(1) && Some(&p.version) == args.get(2))
            .ok_or("plugin/version must match manifests")?;
        publish(
            &Live,
            root,
            repo,
            &p,
            Path::new(opt("--assets-dir")?),
            target,
        )?;
    } else {
        let all = release::pending(root)?;
        let mut index = release::registry(root)?;
        let mut evidence = json!({});
        // Validate every note before querying GitHub or changing any output.
        for p in &all {
            release::plugin_notes(root, p)?;
        }
        for p in all {
            let r = lookup(client, repo, &p)?.ok_or("release missing")?;
            verify(
                client,
                repo,
                &p,
                &r,
                &release::plugin_notes(root, &p)?,
                target,
            )?;
            let e = entry(repo, &p, &r)?;
            evidence[tag(&p)] = e.clone();
            index[&p.name]["versions"][&p.version] = e;
            index[&p.name]["latest"] = json!(p.version);
            index[&p.name]["description"] = json!(p.description);
        }
        if evidence.as_object().is_some_and(|o| !o.is_empty()) {
            index = crate::metadata::enrich_latest(root, repo, &index)?;
        }
        let out = Path::new(opt("--output")?);
        fs::create_dir_all(out)?;
        if out.canonicalize()? == root.join("registry").canonicalize()? {
            return Err("registry output must be separate from source; CI reviews/copies after verification".into());
        }
        let original = fs::read_to_string(root.join("registry/index.json"))?;
        fs::write(
            out.join("index.json"),
            if evidence.as_object().is_some_and(|o| o.is_empty()) {
                original
            } else {
                serde_json::to_string_pretty(&index)? + "\n"
            },
        )?;
        fs::write(
            out.join("release-evidence.json"),
            serde_json::to_string_pretty(&json!({"source_commit":target,"releases":evidence}))?
                + "\n",
        )?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn payloads_and_verified_registry() {
        let p = Plugin {
            name: "test".into(),
            version: "1.2.3".into(),
            model: "wasm".into(),
            folder: "".into(),
            description: "".into(),
        };
        let name = "test-1.2.3.wasm";
        let digest = hash(b"wasm");
        let url = "https://github.com/a/b/releases/download/test-v1.2.3/test-1.2.3.wasm";
        let side = format!("{digest}  {name}\n");
        let r = json!({"assets":[{"name":name,"size":4,"digest":format!("sha256:{digest}"),"browser_download_url":url},{"name":format!("{name}.sha256"),"size":side.len(),"digest":format!("sha256:{}",hash(side.as_bytes())),"browser_download_url":format!("{url}.sha256")}]});
        assert_eq!(entry("a/b", &p, &r).unwrap()["url"], url);
        let mut bad = r.clone();
        bad["assets"][1]["digest"] = json!("bad");
        assert!(entry("a/b", &p, &bad).is_err());
    }
    #[test]
    fn native_has_four_payloads() {
        let p = Plugin {
            name: "test".into(),
            version: "1.0.0".into(),
            model: "native".into(),
            folder: "".into(),
            description: "".into(),
        };
        assert_eq!(payloads(&p).len(), 4);
    }
}

#[cfg(test)]
mod retry_tests {
    use super::*;
    use std::cell::RefCell;
    const SHA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    #[derive(Default)]
    struct Fake {
        release: RefCell<Option<Value>>,
        uploads: RefCell<usize>,
        failure: RefCell<Option<String>>,
        collide: bool,
    }
    impl GitHub for Fake {
        fn run(&self, args: &[&str]) -> Result<String> {
            if args[0] == "api" {
                if args[1].contains("/commits/") {
                    return Ok(json!({"sha":SHA}).to_string());
                }
                return self
                    .release
                    .borrow()
                    .as_ref()
                    .map(Value::to_string)
                    .ok_or_else(|| "HTTP 404".into());
            }
            if let Some(f) = self.failure.borrow().as_ref() {
                return Err(f.clone().into());
            }
            if args[1] == "create" {
                let body = fs::read_to_string(
                    args[args.iter().position(|a| *a == "--notes-file").unwrap() + 1],
                )?;
                *self.release.borrow_mut() = Some(
                    json!({"tag_name":args[2],"body":body,"draft":false,"prerelease":false,"assets":[]}),
                );
            } else if args[1] == "upload" {
                let path = Path::new(args[3]);
                let data = fs::read(path)?;
                let name = path.file_name().unwrap().to_str().unwrap();
                let a = json!({"name":name,"size":data.len(),"digest":format!("sha256:{}",hash(&data)),"browser_download_url":format!("https://github.com/a/b/releases/download/{}/{name}",args[2])});
                self.release.borrow_mut().as_mut().unwrap()["assets"]
                    .as_array_mut()
                    .unwrap()
                    .push(a);
                *self.uploads.borrow_mut() += 1;
            } else {
                return Err("unexpected fake operation".into());
            }
            if self.collide {
                return Err("already_exists".into());
            }
            Ok(String::new())
        }
    }
    fn fixture(model: &str) -> (tempfile::TempDir, Plugin) {
        let t = tempfile::tempdir().unwrap();
        fs::create_dir(t.path().join("plugin")).unwrap();
        fs::create_dir(t.path().join("dist")).unwrap();
        let p = Plugin {
            name: "test".into(),
            version: "1.2.3".into(),
            model: model.into(),
            folder: "plugin".into(),
            description: "".into(),
        };
        fs::write(
            t.path().join("plugin/CHANGELOG.md"),
            "## test-v1.2.3\n\n- Read confined directories with bounded results.\n",
        )
        .unwrap();
        for (_, name) in payloads(&p) {
            fs::write(t.path().join("dist").join(name), b"payload").unwrap();
        }
        (t, p)
    }
    #[test]
    fn exact_notes_and_immutable_retries() {
        for model in ["wasm", "native"] {
            let (t, p) = fixture(model);
            let f = Fake::default();
            let root = t.path();
            publish(&f, root, "a/b", &p, &root.join("dist"), SHA).unwrap();
            let n = *f.uploads.borrow();
            assert_eq!(n, payloads(&p).len() * 2);
            publish(&f, root, "a/b", &p, &root.join("dist"), SHA).unwrap();
            assert_eq!(*f.uploads.borrow(), n);
            assert_eq!(
                f.release.borrow().as_ref().unwrap()["body"],
                release::plugin_notes(root, &p).unwrap()
            );
            assert!(entry("a/b", &p, f.release.borrow().as_ref().unwrap()).is_ok());
            fs::write(root.join("dist").join(&payloads(&p)[0].1), b"different").unwrap();
            assert!(
                publish(&f, root, "a/b", &p, &root.join("dist"), SHA)
                    .unwrap_err()
                    .to_string()
                    .contains("refusing overwrite")
            );
        }
    }
    #[test]
    fn collision_verified_and_server_failures_propagate() {
        let (t, p) = fixture("wasm");
        let root = t.path();
        let f = Fake {
            collide: true,
            ..Default::default()
        };
        publish(&f, root, "a/b", &p, &root.join("dist"), SHA).unwrap();
        for error in ["HTTP 403", "HTTP 500"] {
            let f = Fake {
                failure: RefCell::new(Some(error.into())),
                ..Default::default()
            };
            assert!(
                publish(&f, root, "a/b", &p, &root.join("dist"), SHA)
                    .unwrap_err()
                    .to_string()
                    .contains(error)
            );
        }
        assert!(!collision(&std::io::Error::other("HTTP 500")));
    }
    #[test]
    fn missing_payload_and_notes_fail_before_remote_access() {
        let (t, p) = fixture("wasm");
        let root = t.path();
        let f = Fake::default();
        fs::write(
            root.join("plugin/CHANGELOG.md"),
            "## test-v1.2.3\n\n- TODO\n",
        )
        .unwrap();
        assert!(publish(&f, root, "a/b", &p, &root.join("dist"), SHA).is_err());
        assert!(f.release.borrow().is_none());
        fs::write(
            root.join("plugin/CHANGELOG.md"),
            "## test-v1.2.3\n\n- Useful behavior.\n",
        )
        .unwrap();
        fs::remove_file(root.join("dist").join(&payloads(&p)[0].1)).unwrap();
        assert!(publish(&f, root, "a/b", &p, &root.join("dist"), SHA).is_err());
        assert!(f.release.borrow().is_none());
    }
    #[test]
    fn changed_notes_tag_or_missing_native_assets_fail() {
        let (t, p) = fixture("native");
        let root = t.path();
        let f = Fake::default();
        publish(&f, root, "a/b", &p, &root.join("dist"), SHA).unwrap();
        let r = f.release.borrow().as_ref().unwrap().clone();
        assert!(
            verify(
                &f,
                "a/b",
                &p,
                &r,
                &release::plugin_notes(root, &p).unwrap(),
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
            )
            .is_err()
        );
        let mut bad = r.clone();
        bad["body"] = json!("Different notes");
        assert!(
            verify(
                &f,
                "a/b",
                &p,
                &bad,
                &release::plugin_notes(root, &p).unwrap(),
                SHA
            )
            .is_err()
        );
        let mut bad = r;
        bad["assets"].as_array_mut().unwrap().pop();
        assert!(entry("a/b", &p, &bad).is_err());
    }
}
