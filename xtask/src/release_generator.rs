//! Cocogitto rendering with repository-owned dependency selection.
use super::{Change, Plugin, Prepared, Result};
use crate::cocogitto::{self, Commit};
use std::path::Path;
pub fn generate(p: &Plugin, previous: &str, changes: &[Change]) -> Result<Option<Prepared>> {
    let commits: Vec<_> = changes
        .iter()
        .map(|c| {
            let mut commit = Commit::new(c.id.clone(), c.message.clone());
            if c.id.len() == 40 && c.id.chars().all(|c| c.is_ascii_hexdigit()) {
                let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
                commit.author.timestamp = super::git(root, &["show", "-s", "--format=%ct", &c.id])
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
            }
            commit.author.name = c.author.clone();
            commit
        })
        .collect();
    let timestamp = commits
        .iter()
        .map(|c| c.author.timestamp)
        .max()
        .unwrap_or(0);
    let generated = cocogitto::generate(
        if previous.is_empty() {
            "0.0.0"
        } else {
            previous
        },
        commits,
        timestamp,
        Some(&p.name),
    )?;
    let Some(next) = generated.releases[0].version.as_deref() else {
        return Ok(None);
    };
    let version = if previous.is_empty() {
        p.version.clone()
    } else {
        next.to_string()
    };
    let notes = if previous.is_empty() {
        cocogitto::render_at(
            None,
            &version,
            &generated.releases[0].commits,
            timestamp,
            Some(&p.name),
        )?
    } else {
        generated.markdown
    };
    super::notes(&notes, &p.name, &version)?;
    Ok(Some(Prepared {
        name: p.name.clone(),
        version,
        notes,
    }))
}
pub fn history(root: &Path, p: &Plugin, range: &str) -> Result<Vec<Change>> {
    let text = super::git(
        root,
        &[
            "log",
            "--no-merges",
            "--reverse",
            "--format=%H%x00%an%x00%B%x00",
            range,
        ],
    )?;
    let fields: Vec<_> = text.split('\0').collect();
    let mut changes = vec![];
    for f in fields.chunks(3).filter(|f| f.len() == 3) {
        let id = f[0].trim().to_string();
        let paths = super::git(
            root,
            &[
                "diff-tree",
                "--root",
                "--no-commit-id",
                "--name-only",
                "-r",
                &id,
            ],
        )?
        .lines()
        .map(str::to_owned)
        .collect::<Vec<_>>();
        let paths = super::paths::runtime_paths(root, p, &id, paths)?;
        if super::affected(root, p, &paths)? {
            changes.push(Change {
                id,
                author: Some(f[1].into()),
                message: f[2].trim().into(),
                paths,
            });
        }
    }
    Ok(changes)
}
pub fn published_tag(root: &Path, p: &Plugin, version: &str) -> Result<String> {
    let tag = format!("{}-v{version}", p.name);
    super::git(
        root,
        &["rev-parse", "--verify", &format!("refs/tags/{tag}")],
    )?;
    super::git(root, &["merge-base", "--is-ancestor", &tag, "HEAD"])?;
    Ok(tag)
}
