use serde_json::{Value, json};
use std::io;
use std::path::{Component, Path};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub trait Broker {
    fn call(&self, request: Value) -> Result<Value, String>;
}

#[derive(Clone, Debug)]
pub struct Meta {
    pub is_dir: bool,
    pub is_file: bool,
    pub len: u64,
    pub modified: Option<SystemTime>,
    pub created: Option<SystemTime>,
    pub readonly: bool,
}

pub struct Entry {
    pub name: String,
    pub meta: Meta,
    pub is_symlink: bool,
}

pub trait ReadFs {
    fn stat(&self, rel: &Path) -> io::Result<Meta>;
    fn read(&self, rel: &Path) -> io::Result<Vec<u8>>;
    fn read_range(&self, rel: &Path, offset: u64, max: usize) -> io::Result<(Vec<u8>, u64)>;
    fn list(&self, rel: &Path) -> io::Result<Vec<Entry>>;
}

fn not_found(what: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::NotFound,
        format!("{what}: no such file or directory"),
    )
}

pub struct Scoped<'a> {
    pub root: &'a str,
}

#[cfg(not(target_arch = "wasm32"))]
mod scoped_impl {
    use super::*;
    use cap_std::fs::{Dir, Metadata};
    use std::io::{Read, Seek, SeekFrom};

    fn meta(m: &Metadata) -> Meta {
        Meta {
            is_dir: m.is_dir(),
            is_file: m.is_file(),
            len: m.len(),
            modified: m.modified().ok().map(|t| t.into_std()),
            created: m.created().ok().map(|t| t.into_std()),
            readonly: m.permissions().readonly(),
        }
    }

    impl Scoped<'_> {
        fn dir(&self) -> io::Result<Dir> {
            Dir::open_ambient_dir(self.root, cap_std::ambient_authority())
        }
    }

    impl ReadFs for Scoped<'_> {
        fn stat(&self, rel: &Path) -> io::Result<Meta> {
            Ok(meta(&self.dir()?.metadata(rel)?))
        }
        fn read(&self, rel: &Path) -> io::Result<Vec<u8>> {
            let mut bytes = Vec::new();
            self.dir()?.open(rel)?.read_to_end(&mut bytes)?;
            Ok(bytes)
        }
        fn read_range(&self, rel: &Path, offset: u64, max: usize) -> io::Result<(Vec<u8>, u64)> {
            let mut file = self.dir()?.open(rel)?.into_std();
            let total = file.metadata()?.len();
            file.seek(SeekFrom::Start(offset))?;
            let mut buf = vec![0u8; max];
            let n = file.read(&mut buf)?;
            buf.truncate(n);
            Ok((buf, total))
        }
        fn list(&self, rel: &Path) -> io::Result<Vec<Entry>> {
            let mut out = Vec::new();
            for entry in self.dir()?.open_dir(rel)?.entries()? {
                let entry = entry?;
                let ty = entry.file_type()?;
                out.push(Entry {
                    name: entry.file_name().to_string_lossy().into_owned(),
                    meta: meta(&entry.metadata()?),
                    is_symlink: ty.is_symlink(),
                });
            }
            out.sort_by(|a, b| a.name.cmp(&b.name));
            Ok(out)
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod scoped_impl {
    use super::*;
    use std::fs::{self, Metadata};
    use std::io::{Read, Seek, SeekFrom};

    fn meta(m: &Metadata) -> Meta {
        Meta {
            is_dir: m.is_dir(),
            is_file: m.is_file(),
            len: m.len(),
            modified: m.modified().ok(),
            created: m.created().ok(),
            readonly: m.permissions().readonly(),
        }
    }

    impl Scoped<'_> {
        fn full(&self, rel: &Path) -> std::path::PathBuf {
            Path::new(self.root).join(rel)
        }
    }

    impl ReadFs for Scoped<'_> {
        fn stat(&self, rel: &Path) -> io::Result<Meta> {
            Ok(meta(&fs::metadata(self.full(rel))?))
        }
        fn read(&self, rel: &Path) -> io::Result<Vec<u8>> {
            fs::read(self.full(rel))
        }
        fn read_range(&self, rel: &Path, offset: u64, max: usize) -> io::Result<(Vec<u8>, u64)> {
            let mut file = fs::File::open(self.full(rel))?;
            let total = file.metadata()?.len();
            file.seek(SeekFrom::Start(offset))?;
            let mut buf = vec![0u8; max];
            let n = file.read(&mut buf)?;
            buf.truncate(n);
            Ok((buf, total))
        }
        fn list(&self, rel: &Path) -> io::Result<Vec<Entry>> {
            let mut out = Vec::new();
            for entry in fs::read_dir(self.full(rel))? {
                let entry = entry?;
                let ty = entry.file_type()?;
                out.push(Entry {
                    name: entry.file_name().to_string_lossy().into_owned(),
                    meta: meta(&entry.metadata()?),
                    is_symlink: ty.is_symlink(),
                });
            }
            out.sort_by(|a, b| a.name.cmp(&b.name));
            Ok(out)
        }
    }
}

pub struct Brokered<'a> {
    pub root: &'a str,
    pub broker: &'a dyn Broker,
}

impl Brokered<'_> {
    pub fn abs(&self, rel: &Path) -> String {
        let root = self.root.replace('\\', "/");
        let mut path = root.trim_end_matches('/').to_string();
        for part in rel.components() {
            if let Component::Normal(name) = part {
                path.push('/');
                path.push_str(&name.to_string_lossy());
            }
        }
        if path.is_empty() { "/".into() } else { path }
    }

    fn request(&self, operation: &str, rel: &Path, data: Option<&[u8]>) -> io::Result<Value> {
        let mut request =
            json!({"schema_version": 1, "operation": operation, "path": self.abs(rel)});
        if let Some(data) = data {
            request["data"] = json!(data);
        }
        let response = self.broker.call(request).map_err(io::Error::other)?;
        if response["schema_version"] != 1 {
            return Err(io::Error::other(
                "Unsupported host_file_access schema_version; update rune-kit",
            ));
        }
        Ok(response)
    }

    pub fn write(&self, rel: &Path, data: &[u8]) -> io::Result<()> {
        self.request("write_file", rel, Some(data)).map(|_| ())
    }

    pub fn delete(&self, rel: &Path) -> io::Result<()> {
        self.request("delete_file", rel, None).map(|_| ())
    }

    pub fn create_dir(&self, rel: &Path) -> io::Result<()> {
        self.request("create_directory", rel, None).map(|_| ())
    }

    pub fn remove_dir(&self, rel: &Path) -> io::Result<()> {
        self.request("remove_directory", rel, None).map(|_| ())
    }

    pub fn create_dir_all(&self, rel: &Path) -> io::Result<()> {
        let mut current = std::path::PathBuf::new();
        for part in rel.components() {
            if let Component::Normal(name) = part {
                current.push(name);
                self.create_dir(&current)?;
            }
        }
        Ok(())
    }

    fn stat_opt(&self, rel: &Path) -> io::Result<Option<Meta>> {
        let response = self.request("stat", rel, None)?;
        let stat = &response["stat"];
        if stat["exists"] != true {
            return Ok(None);
        }
        let is_dir = stat["kind"] == "directory";
        let modified = stat["modified"]
            .as_u64()
            .map(|secs| UNIX_EPOCH + Duration::from_secs(secs));
        Ok(Some(Meta {
            is_dir,
            is_file: !is_dir,
            len: stat["size"].as_u64().unwrap_or(0),
            modified,
            created: None,
            readonly: false,
        }))
    }
}

impl ReadFs for Brokered<'_> {
    fn stat(&self, rel: &Path) -> io::Result<Meta> {
        self.stat_opt(rel)?
            .ok_or_else(|| not_found(&rel.display().to_string()))
    }

    fn read(&self, rel: &Path) -> io::Result<Vec<u8>> {
        let response = self.request("read_file", rel, None)?;
        response["data"]
            .as_array()
            .map(|bytes| {
                bytes
                    .iter()
                    .map(|b| b.as_u64().unwrap_or(0) as u8)
                    .collect()
            })
            .ok_or_else(|| io::Error::other("host_file_access returned no data"))
    }

    fn read_range(&self, rel: &Path, offset: u64, max: usize) -> io::Result<(Vec<u8>, u64)> {
        let data = self.read(rel)?;
        let total = data.len() as u64;
        let start = (offset as usize).min(data.len());
        let end = start.saturating_add(max).min(data.len());
        Ok((data[start..end].to_vec(), total))
    }

    fn list(&self, rel: &Path) -> io::Result<Vec<Entry>> {
        let response = self.request("list_directory", rel, None)?;
        let names = response["entries"]
            .as_array()
            .ok_or_else(|| io::Error::other("host_file_access returned no entries"))?;
        let mut out = Vec::new();
        for name in names.iter().filter_map(Value::as_str) {
            if let Some(meta) = self.stat_opt(&rel.join(name))? {
                out.push(Entry {
                    name: name.to_string(),
                    meta,
                    is_symlink: false,
                });
            }
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }
}
