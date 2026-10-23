//! Public Kit 2026-07-28 acceptance client. No plugin ABI changes or installation.
use crate::release::Result;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    io::{BufRead, BufReader},
    path::Path,
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex, mpsc},
    thread,
    time::Duration,
};
const VERSION: &str = "2026-07-28";
static LEGACY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
fn request(id: u64, method: &str, mut params: Value, metadata: bool) -> Value {
    if metadata {
        let supplied = params.get("_meta").cloned().unwrap_or(json!({}));
        params["_meta"] = json!({"io.modelcontextprotocol/protocolVersion":VERSION,"io.modelcontextprotocol/clientCapabilities":{},"io.modelcontextprotocol/clientInfo":{"name":"rune-tools-acceptance","version":"1"}});
        for (k, v) in supplied.as_object().unwrap() {
            params["_meta"][k] = v.clone();
        }
    }
    json!({"jsonrpc":"2.0","id":id,"method":method,"params":params})
}
struct Gateway {
    child: Child,
    endpoint: String,
    client: reqwest::blocking::Client,
    stderr: Arc<Mutex<String>>,
    id: u64,
    legacy: bool,
}
impl Drop for Gateway {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
impl Gateway {
    fn new(rune: &str, plugin: &str, restricted: bool, params: &[String]) -> Result<Self> {
        let mut c = Command::new(rune);
        c.args(["serve", plugin, "--headless", "--port", "0"]);
        if restricted {
            c.arg("--read-only");
        }
        for p in params {
            c.args(["-p", p]);
        }
        let mut child = c
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()?;
        let stderr = child.stderr.take().ok_or("missing server stderr")?;
        let errors = Arc::new(Mutex::new(String::new()));
        let copy = Arc::clone(&errors);
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(stderr)
                .lines()
                .map_while(std::result::Result::ok)
            {
                copy.lock().unwrap().push_str(&line);
                if let Some(start) = line.find("http://") {
                    let url = line[start..]
                        .split_whitespace()
                        .next()
                        .unwrap()
                        .trim_end_matches(['"', ',', ';']);
                    let _ = tx.send(url.to_string());
                }
            }
        });
        let endpoint = match rx.recv_timeout(Duration::from_secs(30)) {
            Ok(url) => url,
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(
                    format!("Kit HTTP startup failed: {e}; {}", errors.lock().unwrap()).into(),
                );
            }
        };
        Ok(Self {
            child,
            endpoint,
            client: reqwest::blocking::Client::builder()
                .no_proxy()
                .timeout(Duration::from_secs(60))
                .build()?,
            stderr: errors,
            id: 0,
            legacy: false,
        })
    }
    fn raw(&mut self, method: &str, params: Value, metadata: bool) -> Result<Value> {
        self.id += 1;
        let body = request(self.id, method, params, metadata);
        let mut call = self
            .client
            .post(&self.endpoint)
            .header("Accept", "application/json, text/event-stream")
            .header(
                "MCP-Protocol-Version",
                body["params"]["_meta"]["io.modelcontextprotocol/protocolVersion"]
                    .as_str()
                    .unwrap_or(VERSION),
            )
            .header("Mcp-Method", method);
        if let Some(name) = body["params"]
            .get("name")
            .or_else(|| body["params"].get("uri"))
            .and_then(Value::as_str)
        {
            call = call.header("Mcp-Name", name);
        }
        let response = call
            .json(&body)
            .send()
            .map_err(|e| format!("Kit HTTP request to {} failed: {e:?}", self.endpoint))?;
        let status = response.status();
        let text = response.text()?;
        let json = if let Some(line) = text.lines().find_map(|l| l.strip_prefix("data:")) {
            line.trim()
        } else {
            text.trim()
        };
        let value: Value = serde_json::from_str(json)
            .map_err(|e| format!("HTTP {status}: invalid MCP response: {e}; {text}"))?;
        if value["id"] != self.id || value["jsonrpc"] != "2.0" {
            return Err(format!("HTTP {status}: mismatched response {value}").into());
        }
        if !status.is_success() && value.get("error").is_none() {
            return Err(format!("HTTP {status}: failure without MCP error: {value}").into());
        }
        Ok(value)
    }
    fn send(&mut self, method: &str, params: Value) -> Result<Value> {
        let r = self.raw(method, params, true)?;
        if r.get("error").is_some() {
            return Err(format!("RPC error: {r}").into());
        }
        let v = r["result"].clone();
        if !self.legacy
            && (v["resultType"] != "complete"
                || v["_meta"]["io.modelcontextprotocol/serverInfo"]["name"]
                    .as_str()
                    .is_none())
        {
            return Err(format!("invalid current MCP result: {r}").into());
        }
        Ok(v)
    }
    fn discover(&mut self) -> Result<Value> {
        let r = self.send("server/discover", json!({}))?;
        assert!(
            r["supportedVersions"]
                .as_array()
                .unwrap()
                .contains(&json!(VERSION))
        );
        assert_eq!(r["ttlMs"], 0);
        assert_eq!(r["cacheScope"], "private");
        assert!(r["capabilities"].get("tools").is_some());
        assert_eq!(
            self.raw("tools/list", json!({}), false)?["error"]["code"],
            -32020
        );
        assert_eq!(
            self.raw(
                "server/discover",
                json!({"_meta":{"io.modelcontextprotocol/protocolVersion":"unsupported"}}),
                true
            )?["error"]["code"],
            -32022
        );
        assert_eq!(
            self.raw("initialize", json!({}), true)?["error"]["code"],
            -32601
        );
        Ok(r)
    }
    fn tools(&mut self) -> Result<BTreeMap<String, Value>> {
        let r = self.send("tools/list", json!({}))?;
        let mut tools = BTreeMap::new();
        for tool in r["tools"].as_array().ok_or("tools missing")? {
            let rules = &tool["inputSchema"]["x-rune-rules"];
            let ann = &tool["annotations"];
            let effects = rules["effects"].as_array().cloned().unwrap_or_default();
            for (canonical, legacy, expected) in [
                ("readOnlyHint", "readOnly", rules["read_only"].clone()),
                (
                    "destructiveHint",
                    "destructive",
                    json!(effects.contains(&json!("exec")) || effects.contains(&json!("delete"))),
                ),
                (
                    "openWorldHint",
                    "openWorld",
                    json!(effects.contains(&json!("network"))),
                ),
            ] {
                assert_eq!(ann[canonical], expected);
                assert_eq!(ann[legacy], expected);
            }
            assert!(ann.get("idempotentHint").is_none() && ann.get("idempotent").is_none());
            assert!(tool["inputSchema"]["properties"].is_object());
            let name = tool["name"]
                .as_str()
                .ok_or("name missing")?
                .split("__")
                .last()
                .unwrap()
                .to_string();
            tools.insert(name, tool.clone());
        }
        Ok(tools)
    }
    fn call(&mut self, name: &str, args: Value) -> Result<Value> {
        let r = self.send("tools/call", json!({"name":name,"arguments":args}))?;
        let text = r["content"][0]["text"]
            .as_str()
            .ok_or("text response missing")?;
        match serde_json::from_str(text) {
            Ok(value) => Ok(value),
            Err(_) if r["isError"] == true => Ok(json!({"status":"error","error":text})),
            Err(e) => Err(e.into()),
        }
    }
}
fn success(v: Value) -> Value {
    assert_eq!(v["status"], "success", "{v}");
    v["result"].clone()
}
fn refused(v: Value) {
    assert_eq!(v["status"], "error", "{v}");
}
fn pilot(rune: &str, plugin: &str, kind: &str, restricted: bool) -> Result<Value> {
    let mut g = Gateway::new(rune, plugin, restricted, &[])?;
    let discovery = g.discover()?;
    let tools = g.tools()?;
    let samples = if kind == "git" {
        vec![(
            "validate_commit_message",
            json!({"message":"fix: verify plugin annotations","conventional":true}),
        )]
    } else {
        vec![
            ("get_current_time", json!({"timezone":"Asia/Kolkata"})),
            (
                "convert_time",
                json!({"time":"2026-10-21T12:00:00","sourceTimezone":"UTC","targetTimezone":"Asia/Kolkata"}),
            ),
        ]
    };
    if restricted {
        assert_eq!(tools.len(), samples.len())
    }
    let mut calls = json!({});
    for (name, args) in samples {
        let t = tools.get(name).ok_or("pilot tool missing")?;
        for key in args.as_object().unwrap().keys() {
            assert!(t["inputSchema"]["properties"].get(key).is_some())
        }
        for key in t["inputSchema"]["required"]
            .as_array()
            .into_iter()
            .flatten()
        {
            assert!(args.get(key.as_str().unwrap()).is_some())
        }
        let result = g.call(t["name"].as_str().unwrap(), args)?;
        assert_ne!(result["status"], "error");
        let v = result.get("result").unwrap_or(&result);
        match name {
            "validate_commit_message" => assert_eq!(v["valid"], true),
            "get_current_time" => {
                assert_eq!(v["timezone"], "Asia/Kolkata");
                assert_eq!(v["utcOffset"], "+05:30");
            }
            _ => assert_eq!(v["target"]["datetime"], "2026-10-21T17:30:00+05:30"),
        };
        calls[name] = result;
    }
    Ok(
        json!({"plugin":plugin,"read_only":restricted,"discovery":discovery,"tools":tools.values().collect::<Vec<_>>(),"calls":calls}),
    )
}
fn filesystem(rune: &str, plugin: &str, fixture: &Path, require_link: bool) -> Result<Value> {
    fs::create_dir_all(fixture)?;
    let fixture = fixture.canonicalize()?;
    let root = fixture.join("root");
    let outside = fixture.join("root-other");
    fs::create_dir_all(root.join("child"))?;
    fs::create_dir_all(&outside)?;
    fs::write(root.join("note.txt"), "inside marker\nsecond\nthird\n")?;
    fs::write(outside.join("secret.txt"), "outside marker")?;
    fs::write(
        root.join("pages.txt"),
        ("x".repeat(100) + "\n").repeat(3000),
    )?;
    fs::write(root.join("long.txt"), vec![b'x'; 65537])?;
    fs::write(root.join("large.txt"), vec![b'x'; 8 * 1024 * 1024 + 1])?;
    fs::create_dir_all(root.join("many"))?;
    for i in 0..220 {
        fs::write(root.join("many").join(format!("entry-{i:03}")), "")?;
    }
    let params = vec![format!("allowed_dir={}", root.display())];
    let mut normal = Gateway::new(rune, plugin, false, &params)?;
    normal.discover()?;
    let full = normal.tools()?;
    assert_eq!(full.len(), 13);
    let readonly: Vec<_> = full
        .iter()
        .filter(|(_, t)| t["annotations"]["readOnlyHint"] == true)
        .map(|(n, _)| n.as_str())
        .collect();
    assert_eq!(readonly, ["list_directory", "read_text_file"]);
    drop(normal);
    let mut g = Gateway::new(rune, plugin, true, &params)?;
    let discovery = g.discover()?;
    let tools = g.tools()?;
    assert_eq!(
        tools.keys().map(String::as_str).collect::<Vec<_>>(),
        ["list_directory", "read_text_file"]
    );
    for (n, t) in &tools {
        assert_eq!(t, &full[n]);
        assert_eq!(
            t["inputSchema"]["x-rune-rules"]["limits"]["root_param"],
            "allowed_dir"
        );
    }
    let read = tools["read_text_file"]["name"].as_str().unwrap();
    let list = tools["list_directory"]["name"].as_str().unwrap();
    let first = success(g.call(read, json!({"path":"note.txt","lineLimit":1}))?);
    assert_eq!(first["content"], "inside marker");
    assert_eq!(first["nextLineOffset"], 1);
    assert_eq!(
        success(g.call(
            read,
            json!({"path":"note.txt","lineOffset":1,"lineLimit":1})
        )?)["content"],
        "second"
    );
    assert_eq!(
        success(g.call(read, json!({"path":"note.txt","tail":1}))?)["content"],
        "third"
    );
    assert_eq!(
        success(g.call(read, json!({"path":root.join("note.txt"),"head":1}))?)["content"],
        "inside marker"
    );
    assert!(
        success(g.call(list, json!({"path":"."}))?)["content"]
            .as_str()
            .unwrap()
            .contains("[DIR] child")
    );
    let mut refusals = vec![];
    for path in [
        "../root-other/secret.txt".into(),
        outside.join("secret.txt").display().to_string(),
        outside
            .join("secret.txt")
            .display()
            .to_string()
            .replace('\\', "/"),
        "child/../../root-other/secret.txt".into(),
    ] {
        let v = g.call(read, json!({"path":path}))?;
        refused(v.clone());
        refusals.push(v);
    }
    refused(g.call(list, json!({"path":outside}))?);
    for key in ["allowedDir", "allowed_dir"] {
        let mut args = json!({"path":"note.txt"});
        args[key] = json!(outside);
        refused(g.call(read, args)?);
    }
    for args in [
        json!({"path":"large.txt"}),
        json!({"path":"long.txt"}),
        json!({"path":"note.txt","lineLimit":2001}),
        json!({"path":"note.txt","head":1,"tail":1}),
        json!({"path":"note.txt","tail":-1}),
    ] {
        refused(g.call(read, args)?);
    }
    let text = success(g.call(read, json!({"path":"pages.txt"}))?);
    assert!(text["content"].as_str().unwrap().len() <= 65536);
    assert!(text["linesReturned"].as_u64().unwrap() <= 2000);
    assert_eq!(text["hasMore"], true);
    let directory = success(g.call(list, json!({"path":"many"}))?);
    assert_eq!(directory["entriesReturned"], 200);
    assert_eq!(directory["truncated"], true);
    assert!(directory["content"].as_str().unwrap().len() <= 65536);
    refused(g.call(list, json!({"path":"many","maxEntries":201}))?);
    let mut race = json!({"status":"NOT RUN: link absent"});
    let linked = root.join("escape").exists();
    assert!(
        linked || !require_link,
        "Real outside link/junction required"
    );
    if linked {
        assert_eq!(
            fs::read_to_string(root.join("escape/secret.txt"))?,
            "outside marker"
        );
        refused(g.call(read, json!({"path":"escape/secret.txt"}))?);
        refused(g.call(list, json!({"path":"escape"}))?);
        // Alternate the real outside link with an in-root directory; restore without recursive removal.
        let active = root.join("race");
        let parked = root.join("race-link");
        let safe = root.join("race-safe");
        assert!(
            !active.exists() && !parked.exists() && !safe.exists(),
            "use fresh race names"
        );
        fs::create_dir(&active)?;
        fs::write(active.join("secret.txt"), "inside race")?;
        fs::rename(root.join("escape"), &parked)?;
        let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let copy = Arc::clone(&stop);
        let a = active.clone();
        let p = parked.clone();
        let s = safe.clone();
        let thread = thread::spawn(move || {
            let mut swaps = 0;
            while !copy.load(std::sync::atomic::Ordering::Relaxed) {
                if !s.exists() {
                    let _ = fs::rename(&a, &s);
                }
                if p.exists() && fs::rename(&p, &a).is_ok() {
                    swaps += 1;
                }
                thread::sleep(Duration::from_millis(1));
                if !p.exists() {
                    let _ = fs::rename(&a, &p);
                }
                if !a.exists() {
                    let _ = fs::rename(&s, &a);
                }
                thread::sleep(Duration::from_millis(1));
            }
            swaps
        });
        let mut inside = 0;
        let mut denied = 0;
        let run = (|| -> Result<()> {
            for _ in 0..60 {
                let v = g.call(read, json!({"path":"race/secret.txt"}))?;
                if v["status"] == "success" {
                    if v["result"]["content"] != "inside race" {
                        return Err("replacement race exposed unexpected content".into());
                    }
                    inside += 1;
                } else {
                    refused(v);
                    denied += 1;
                }
            }
            Ok(())
        })();
        stop.store(true, std::sync::atomic::Ordering::Relaxed);
        let swaps = thread.join().map_err(|_| "race thread failed")?;
        if safe.exists() {
            if active.exists() {
                fs::rename(&active, &parked)?;
            }
            fs::rename(&safe, &active)?;
        }
        fs::rename(&parked, root.join("escape"))?;
        fs::remove_file(active.join("secret.txt"))?;
        fs::remove_dir(&active)?;
        run?;
        assert!(swaps > 0);
        race = json!({"swaps":swaps,"inside_successes":inside,"refusals":denied,"outside_successes":0});
    }
    let prefix = read.strip_suffix("read_text_file").unwrap();
    let write=g.raw("tools/call",json!({"name":format!("{prefix}write_file"),"arguments":{"path":"blocked.txt","content":"forbidden"}}),true)?;
    assert_eq!(write["error"]["code"], -32600);
    assert!(!root.join("blocked.txt").exists());
    drop(g);
    for params in [
        vec![],
        vec![format!("allowed_dir={}", fixture.join("missing").display())],
    ] {
        let mut missing = match Gateway::new(rune, plugin, true, &params) {
            Ok(g) => g,
            Err(e)
                if !params.is_empty() && e.to_string().contains("existing absolute directory") =>
            {
                continue;
            }
            Err(e) => return Err(e),
        };
        match missing.discover() {
            Ok(_) => {
                let ts = missing.tools()?;
                let tool = ts.values().next().ok_or("no missing-root tools")?;
                refused(missing.call(tool["name"].as_str().unwrap(), json!({"path":"."}))?);
            }
            Err(e) => {
                assert!(!params.is_empty(), "missing root failed unexpectedly: {e}");
                let errors = missing.stderr.lock().unwrap().to_ascii_lowercase();
                assert!(
                    ["missing", "directory", "os error 2"]
                        .iter()
                        .any(|s| errors.contains(s)),
                    "{errors}"
                );
                assert!(!missing.child.wait()?.success());
            }
        }
    }
    Ok(
        json!({"protocol":VERSION,"plugin":plugin,"normal_tools":full.values().collect::<Vec<_>>(),"tools":tools.values().collect::<Vec<_>>(),"discovery":discovery,"read":first,"text_budget":text,"directory_budget":directory,"refusals":refusals,"replacement_race":race,"real_link_escape":if linked{"refused"}else{"NOT RUN"},"write_refusal":write,"missing_invalid_root":"refused"}),
    )
}
pub fn command(args: &[String]) -> Result<()> {
    if args.iter().any(|s| s == "--legacy-protocol") {
        return Err("Use the historical stdio harness from Git history; current acceptance requires Kit HTTP".into());
    }
    LEGACY.store(
        args.iter().any(|s| s == "--legacy-protocol"),
        std::sync::atomic::Ordering::Relaxed,
    );
    let opt = |k: &str| -> Result<&str> {
        Ok(args
            .get(
                args.iter()
                    .position(|a| a == k)
                    .ok_or_else(|| format!("{k} required"))?
                    + 1,
            )
            .ok_or("option value missing")?
            .as_str())
    };
    let rune = opt("--rune")?;
    let mut report = if args[0] == "smoke-filesystem" {
        filesystem(
            rune,
            opt("--plugin")?,
            Path::new(opt("--fixture")?),
            args.iter().any(|a| a == "--require-link"),
        )?
    } else {
        let mut exchanges = vec![];
        for (kind, plugin) in [("git", opt("--git")?), ("time", opt("--time")?)] {
            for restricted in [false, true] {
                exchanges.push(pilot(rune, plugin, kind, restricted)?);
            }
        }
        json!({"protocol":VERSION,"exchanges":exchanges})
    };
    if LEGACY.load(std::sync::atomic::Ordering::Relaxed) {
        report["protocol"] = json!("legacy-2025-03-26");
    }
    let version = Command::new(rune).arg("--version").output()?;
    assert!(version.status.success());
    report["kit_version"] = json!(String::from_utf8(version.stdout)?.trim());
    fs::write(
        opt("--report")?,
        serde_json::to_string_pretty(&report)? + "\n",
    )?;
    println!(
        "{} smoke checks passed; report: {}",
        args[0],
        opt("--report")?
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn current_request_metadata() {
        let r = request(1, "tools/list", json!({}), true);
        assert_eq!(
            r["params"]["_meta"]["io.modelcontextprotocol/protocolVersion"],
            VERSION
        );
        assert!(r["params"]["_meta"]["io.modelcontextprotocol/clientCapabilities"].is_object());
        assert!(
            request(2, "tools/list", json!({}), false)["params"]
                .get("_meta")
                .is_none()
        );
        let r = request(
            3,
            "server/discover",
            json!({"_meta":{"io.modelcontextprotocol/protocolVersion":"unsupported"}}),
            true,
        );
        assert_eq!(
            r["params"]["_meta"]["io.modelcontextprotocol/protocolVersion"],
            "unsupported"
        );
    }
}
