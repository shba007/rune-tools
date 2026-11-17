//! Staged mode against a simulated `host_file_access` broker. The fake keeps staged work in
//! memory (like Kit's staging area), reads through it, and never writes to the real folder.
use rune_filesystem::access::Broker;
use rune_filesystem::call_tool_in;
use rune_filesystem::operations::{Mode, execute_tool_in};
use rune_pdk::ToolCallRequest;
use serde_json::{Value, json};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

struct Fake {
    root: String,
    files: RefCell<BTreeMap<String, Option<Vec<u8>>>>,
    dirs: RefCell<BTreeMap<String, bool>>,
    log: RefCell<Vec<(String, String)>>,
}

#[derive(PartialEq, Clone, Copy)]
enum Kind {
    File,
    Dir,
}

impl Fake {
    fn new(dir: &TempDir) -> Self {
        Self {
            root: dir.path().to_str().unwrap().replace('\\', "/"),
            files: Default::default(),
            dirs: Default::default(),
            log: Default::default(),
        }
    }

    fn kind(&self, path: &str) -> Option<Kind> {
        if let Some(entry) = self.files.borrow().get(path) {
            return entry.as_ref().map(|_| Kind::File);
        }
        if let Some(present) = self.dirs.borrow().get(path) {
            return present.then_some(Kind::Dir);
        }
        let meta = fs::metadata(path).ok()?;
        Some(if meta.is_dir() { Kind::Dir } else { Kind::File })
    }

    fn parent(path: &str) -> &str {
        path.rsplit_once('/').map_or("", |(parent, _)| parent)
    }

    fn names(&self, path: &str) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(path)
            .map(|rd| {
                rd.flatten()
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        let staged: Vec<(String, bool)> = self
            .files
            .borrow()
            .iter()
            .map(|(k, v)| (k.clone(), v.is_some()))
            .chain(self.dirs.borrow().iter().map(|(k, v)| (k.clone(), *v)))
            .collect();
        for (key, present) in staged {
            if Self::parent(&key) == path {
                let name = key.rsplit('/').next().unwrap().to_string();
                names.retain(|n| n != &name);
                if present {
                    names.push(name);
                }
            }
        }
        names.sort();
        names
    }

    fn untouched(&self) -> bool {
        // The simulated host never modifies the real folder; tests assert that separately.
        true
    }
}

impl Broker for Fake {
    fn call(&self, request: Value) -> Result<Value, String> {
        assert_eq!(request["schema_version"], 1);
        let op = request["operation"].as_str().unwrap().to_string();
        let path = request["path"].as_str().unwrap().to_string();
        self.log.borrow_mut().push((op.clone(), path.clone()));
        assert!(
            path == self.root || path.starts_with(&format!("{}/", self.root)),
            "broker path escaped the root: {path}"
        );
        let data: Vec<u8> = request["data"]
            .as_array()
            .map(|a| a.iter().map(|b| b.as_u64().unwrap() as u8).collect())
            .unwrap_or_default();
        let parent_ok = |p: &str| {
            let parent = Self::parent(p);
            self.kind(parent) == Some(Kind::Dir)
        };
        match op.as_str() {
            "read_file" => match self.kind(&path) {
                Some(Kind::File) => {
                    let staged = self.files.borrow().get(&path).cloned().flatten();
                    let bytes = staged.unwrap_or_else(|| fs::read(&path).unwrap());
                    Ok(json!({"schema_version":1,"staged":false,"data":bytes}))
                }
                Some(Kind::Dir) => Err("Path is a folder".into()),
                None => Err("File is deleted or missing".into()),
            },
            "write_file" => {
                if self.kind(&path) == Some(Kind::Dir) {
                    return Err("Path is a folder".into());
                }
                if !parent_ok(&path) {
                    return Err("Parent folder does not exist".into());
                }
                self.files.borrow_mut().insert(path, Some(data));
                Ok(json!({"schema_version":1,"staged":true}))
            }
            "delete_file" => {
                if self.kind(&path) != Some(Kind::File) {
                    return Err("File does not exist".into());
                }
                self.files.borrow_mut().insert(path, None);
                Ok(json!({"schema_version":1,"staged":true}))
            }
            "create_directory" => {
                if self.kind(&path).is_none() {
                    if !parent_ok(&path) {
                        return Err("Parent folder does not exist".into());
                    }
                    self.dirs.borrow_mut().insert(path, true);
                }
                Ok(json!({"schema_version":1,"staged":true}))
            }
            "remove_directory" => {
                if !self.names(&path).is_empty() {
                    return Err("Folder is not empty".into());
                }
                self.dirs.borrow_mut().insert(path, false);
                Ok(json!({"schema_version":1,"staged":true}))
            }
            "list_directory" => {
                Ok(json!({"schema_version":1,"staged":false,"entries":self.names(&path)}))
            }
            "stat" => {
                let stat = match self.kind(&path) {
                    None => json!({"exists": false}),
                    Some(kind) => {
                        let size = match kind {
                            Kind::File => Some(
                                self.files
                                    .borrow()
                                    .get(&path)
                                    .cloned()
                                    .flatten()
                                    .map(|b| b.len() as u64)
                                    .or_else(|| fs::metadata(&path).ok().map(|m| m.len()))
                                    .unwrap(),
                            ),
                            Kind::Dir => None,
                        };
                        json!({"exists": true, "kind": if kind == Kind::File {"file"} else {"directory"},
                               "size": size, "modified": 1_700_000_000u64})
                    }
                };
                Ok(json!({"schema_version":1,"staged":false,"stat":stat}))
            }
            other => Err(format!("unsupported operation {other}")),
        }
    }
}

fn request(name: &str, arguments: Value) -> ToolCallRequest {
    ToolCallRequest {
        name: name.into(),
        arguments,
    }
}

fn run(fake: &Fake, name: &str, arguments: Value) -> Result<Value, String> {
    let mode = Mode::Staged {
        root: &fake.root,
        broker: fake,
    };
    execute_tool_in(request(name, arguments), &mode)
}

fn ops(fake: &Fake) -> Vec<String> {
    fake.log
        .borrow()
        .iter()
        .map(|(op, path)| format!("{op} {}", path.strip_prefix(&fake.root).unwrap_or(path)))
        .collect()
}

fn setup() -> (TempDir, Fake) {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("a.txt"), "one\ntwo\n").unwrap();
    fs::create_dir(dir.path().join("sub")).unwrap();
    fs::write(dir.path().join("sub").join("b.txt"), "bee").unwrap();
    let fake = Fake::new(&dir);
    assert!(fake.untouched());
    (dir, fake)
}

fn disk_snapshot(root: &Path) -> Vec<String> {
    let mut out: Vec<String> = walkdir::WalkDir::new(root)
        .into_iter()
        .flatten()
        .map(|e| {
            format!(
                "{}:{}",
                e.path().strip_prefix(root).unwrap().display(),
                if e.file_type().is_file() {
                    fs::read_to_string(e.path()).unwrap()
                } else {
                    String::new()
                }
            )
        })
        .collect();
    out.sort();
    out
}

#[test]
fn staged_write_goes_through_the_broker_and_reads_back() {
    let (dir, fake) = setup();
    let before = disk_snapshot(dir.path());
    let res = run(
        &fake,
        "write_file",
        json!({"path": "new/deep/c.txt", "content": "hello"}),
    )
    .unwrap();
    assert_eq!(res["staged"], true);
    assert!(
        res["content"]
            .as_str()
            .unwrap()
            .contains("wrote 5 bytes to new/deep/c.txt")
    );
    // Parent folders are created shallowest first, then the file.
    let log = ops(&fake);
    assert_eq!(
        log,
        [
            "create_directory /new",
            "create_directory /new/deep",
            "write_file /new/deep/c.txt"
        ]
    );
    assert_eq!(disk_snapshot(dir.path()), before, "real folder untouched");
    let read = run(&fake, "read_text_file", json!({"path": "new/deep/c.txt"})).unwrap();
    assert_eq!(read["content"], "hello");
    let abs = format!("{}/new/deep/c.txt", fake.root);
    let read = run(&fake, "read_text_file", json!({"path": abs})).unwrap();
    assert_eq!(read["content"], "hello");
}

#[test]
fn staged_edit_file_and_dry_run() {
    let (dir, fake) = setup();
    let edits = json!([{"oldText": "two", "newText": "TWO"}]);
    let dry = run(
        &fake,
        "edit_file",
        json!({"path": "a.txt", "edits": edits, "dryRun": true}),
    )
    .unwrap();
    assert!(dry.get("staged").is_none());
    assert!(!ops(&fake).iter().any(|o| o.starts_with("write_file")));
    let res = run(&fake, "edit_file", json!({"path": "a.txt", "edits": edits})).unwrap();
    assert_eq!(res["staged"], true);
    assert!(res["content"].as_str().unwrap().contains("+ TWO"));
    let read = run(&fake, "read_text_file", json!({"path": "a.txt"})).unwrap();
    assert_eq!(read["content"], "one\nTWO");
    assert_eq!(
        fs::read_to_string(dir.path().join("a.txt")).unwrap(),
        "one\ntwo\n"
    );
    let missing = run(
        &fake,
        "edit_file",
        json!({"path": "a.txt", "edits": [{"oldText": "zzz", "newText": "y"}]}),
    );
    assert!(missing.unwrap_err().contains("was not found"));
}

#[test]
fn staged_create_directory_is_mkdir_p() {
    let (dir, fake) = setup();
    let res = run(&fake, "create_directory", json!({"path": "x/y/z"})).unwrap();
    assert_eq!(res["staged"], true);
    assert_eq!(
        ops(&fake),
        [
            "create_directory /x",
            "create_directory /x/y",
            "create_directory /x/y/z"
        ]
    );
    assert!(!dir.path().join("x").exists());
    let info = run(&fake, "get_file_info", json!({"path": "x/y/z"})).unwrap();
    let info: Value = serde_json::from_str(info["content"].as_str().unwrap()).unwrap();
    assert_eq!(info["is_directory"], true);
}

#[test]
fn staged_move_file_is_read_write_delete() {
    let (dir, fake) = setup();
    let res = run(
        &fake,
        "move_file",
        json!({"source": "a.txt", "destination": "sub/moved.txt"}),
    )
    .unwrap();
    assert_eq!(res["staged"], true);
    assert_eq!(
        ops(&fake)
            .into_iter()
            .filter(|o| !o.starts_with("stat"))
            .collect::<Vec<_>>(),
        [
            "read_file /a.txt",
            "write_file /sub/moved.txt",
            "delete_file /a.txt"
        ]
    );
    assert!(dir.path().join("a.txt").exists(), "real file still there");
    let gone = run(&fake, "get_file_info", json!({"path": "a.txt"}));
    assert!(gone.is_err());
    let moved = run(&fake, "read_text_file", json!({"path": "sub/moved.txt"})).unwrap();
    assert_eq!(moved["content"], "one\ntwo");
    let clash = run(
        &fake,
        "move_file",
        json!({"source": "sub/b.txt", "destination": "sub/moved.txt"}),
    );
    assert!(clash.unwrap_err().contains("already exists"));
}

#[test]
fn staged_move_directory_recurses_and_removes_the_source() {
    let (dir, fake) = setup();
    run(
        &fake,
        "move_file",
        json!({"source": "sub", "destination": "renamed"}),
    )
    .unwrap();
    let log = ops(&fake);
    let position = |needle: &str| log.iter().position(|o| o == needle).unwrap();
    assert!(position("create_directory /renamed") < position("write_file /renamed/b.txt"));
    assert!(position("delete_file /sub/b.txt") < position("remove_directory /sub"));
    assert!(dir.path().join("sub/b.txt").exists());
    let list = run(&fake, "list_directory", json!({"path": "."})).unwrap();
    let content = list["content"].as_str().unwrap();
    assert!(content.contains("[DIR] renamed") && !content.contains("[DIR] sub"));
    let moved = run(&fake, "read_text_file", json!({"path": "renamed/b.txt"})).unwrap();
    assert_eq!(moved["content"], "bee");
    let into_self = run(
        &fake,
        "move_file",
        json!({"source": "renamed", "destination": "renamed/inner"}),
    );
    assert!(into_self.unwrap_err().contains("into itself"));
}

#[test]
fn staged_reads_see_staged_content_in_every_read_tool() {
    let (_dir, fake) = setup();
    run(
        &fake,
        "write_file",
        json!({"path": "sub/new.txt", "content": "fresh"}),
    )
    .unwrap();

    let list = run(&fake, "list_directory", json!({"path": "sub"})).unwrap();
    assert!(list["content"].as_str().unwrap().contains("[FILE] new.txt"));

    let sizes = run(
        &fake,
        "list_directory_with_sizes",
        json!({"path": "sub", "sortBy": "size"}),
    )
    .unwrap();
    assert!(sizes["content"].as_str().unwrap().contains("new.txt"));

    let tree = run(&fake, "directory_tree", json!({"path": "."})).unwrap();
    assert!(tree["content"].as_str().unwrap().contains("new.txt"));
    let excluded = run(
        &fake,
        "directory_tree",
        json!({"path": ".", "excludePatterns": ["*.txt"]}),
    )
    .unwrap();
    assert!(!excluded["content"].as_str().unwrap().contains("new.txt"));

    let found = run(
        &fake,
        "search_files",
        json!({"path": ".", "pattern": "new.*"}),
    )
    .unwrap();
    assert_eq!(
        found["content"],
        format!("{}/sub/new.txt", fake.root.trim_end_matches('/'))
    );

    let many = run(
        &fake,
        "read_multiple_files",
        json!({"paths": ["sub/new.txt", "a.txt"]}),
    )
    .unwrap();
    let text = many["content"].as_str().unwrap();
    assert!(text.contains("fresh") && text.contains("one"));

    let info = run(&fake, "get_file_info", json!({"path": "sub/new.txt"})).unwrap();
    let info: Value = serde_json::from_str(info["content"].as_str().unwrap()).unwrap();
    assert_eq!(info["size_bytes"], 5);
    assert_eq!(info["is_file"], true);

    let media = run(&fake, "read_media_file", json!({"path": "sub/new.txt"})).unwrap();
    assert_eq!(media["paging"]["totalBytes"], 5);

    let allowed = run(&fake, "list_allowed_directories", json!({})).unwrap();
    assert!(allowed["content"].as_str().unwrap().contains(&fake.root));
}

#[test]
fn staged_paths_stay_confined_to_the_root() {
    let (_dir, fake) = setup();
    let outside = tempfile::tempdir().unwrap();
    let outside_file = outside.path().join("secret.txt");
    fs::write(&outside_file, "secret").unwrap();
    let bad = [
        "../escape.txt".to_string(),
        "sub/../../escape.txt".to_string(),
        outside_file.to_str().unwrap().to_string(),
        "C:/Windows/win.ini".to_string(),
        "/etc/passwd".to_string(),
        "sub/CON".to_string(),
    ];
    for path in bad {
        for (name, args) in [
            ("write_file", json!({"path": path, "content": "x"})),
            ("create_directory", json!({"path": path})),
            ("read_text_file", json!({"path": path})),
            ("read_media_file", json!({"path": path})),
            ("get_file_info", json!({"path": path})),
            ("list_directory", json!({"path": path})),
            ("directory_tree", json!({"path": path})),
            ("search_files", json!({"path": path, "pattern": "*"})),
            ("read_multiple_files", json!({"paths": [path]})),
            ("move_file", json!({"source": path, "destination": "z.txt"})),
            ("move_file", json!({"source": "a.txt", "destination": path})),
        ] {
            let error = run(&fake, name, args).expect_err(&format!("{name} accepted {path}"));
            assert!(error.contains("Access denied"), "{name} {path}: {error}");
        }
    }
    assert!(
        fake.log.borrow().is_empty(),
        "no broker call for denied paths"
    );
}

#[test]
fn staged_public_result_carries_the_staged_flag() {
    let (_dir, fake) = setup();
    let mode = Mode::Staged {
        root: &fake.root,
        broker: &fake,
    };
    let out = call_tool_in(
        request("write_file", json!({"path": "p.txt", "content": "x"})),
        &mode,
    );
    assert_eq!(out["structuredContent"]["staged"], true);
    assert_eq!(out["isError"], false);

    // A truncated text preview is rebuilt, so the flag must survive that path too.
    let big = "y".repeat(2 * 1024 * 1024);
    run(
        &fake,
        "write_file",
        json!({"path": "big.txt", "content": big}),
    )
    .unwrap();
    let out = call_tool_in(
        request(
            "edit_file",
            json!({"path": "big.txt", "edits": [{"oldText": big, "newText": "z"}]}),
        ),
        &mode,
    );
    assert_eq!(out["structuredContent"]["truncated"], true);
    assert_eq!(out["structuredContent"]["staged"], true);

    let read = call_tool_in(request("get_file_info", json!({"path": "a.txt"})), &mode);
    assert!(read["structuredContent"].get("staged").is_none());
}

#[test]
fn broker_errors_become_actionable_tool_errors() {
    let (_dir, fake) = setup();
    let err = run(&fake, "write_file", json!({"path": "sub", "content": "x"})).unwrap_err();
    assert!(err.contains("Failed to write file") && err.contains("folder"));
    let err = run(&fake, "read_text_file", json!({"path": "nope.txt"})).unwrap_err();
    assert!(err.contains("no such file or directory"));
    let err = run(&fake, "read_text_file", json!({"path": "sub"})).unwrap_err();
    assert!(err.contains("not a regular text file"));
}

#[test]
fn direct_mode_is_unchanged_and_never_reports_staged() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("direct/out.txt");
    let out = call_tool_in(
        request(
            "write_file",
            json!({"path": file.to_str().unwrap(), "content": "abc"}),
        ),
        &Mode::Direct,
    );
    assert_eq!(fs::read_to_string(&file).unwrap(), "abc");
    assert!(out["structuredContent"].get("staged").is_none());
    assert!(
        out["structuredContent"]["content"]
            .as_str()
            .unwrap()
            .contains("Successfully wrote 3 bytes")
    );
}
