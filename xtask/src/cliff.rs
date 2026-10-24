//! Package-owned Conventional Commit interpretation, bumping and rendering.
use super::{Change, Plugin, Prepared, Result};
use git_cliff_core::{
    changelog::Changelog, commit::Commit, config::Config, release::Release, repo::Repository,
};
use std::{collections::BTreeSet, path::Path};

pub fn config() -> Result<Config> {
    Ok(include_str!("../../cliff.toml").parse()?)
}
pub fn generate(p: &Plugin, previous: &str, changes: &[Change]) -> Result<Option<Prepared>> {
    let commits = changes
        .iter()
        .map(|c| {
            let mut commit = Commit::new(c.id.clone(), c.message.clone());
            commit.author.name = c.author.clone();
            commit
        })
        .collect();
    let release = Release {
        commits,
        previous: if previous.is_empty() {
            None
        } else {
            Some(Box::new(Release {
                version: Some(format!("{}-v{previous}", p.name)),
                ..Default::default()
            }))
        },
        extra: Some(serde_json::json!({"repository":"https://github.com/shba007/rune-tools"})),
        ..Default::default()
    };
    let mut configuration = config()?;
    if previous.is_empty() {
        configuration.bump.initial_tag = Some(format!("{}-v{}", p.name, p.version));
    }
    let mut cliff = Changelog::new(vec![release], configuration, None)?;
    if cliff.releases.first().is_none_or(|r| r.commits.is_empty()) {
        return Ok(None);
    }
    let tag = cliff
        .bump_version()?
        .ok_or("git-cliff did not calculate a version")?;
    let version = tag
        .strip_prefix(&format!("{}-v", p.name))
        .ok_or("git-cliff returned a mismatched plugin tag")?
        .to_string();
    let calculated = semver::Version::parse(&version)?;
    if !previous.is_empty() && calculated <= semver::Version::parse(previous)? {
        return Err("git-cliff returned a non-increasing version".into());
    }
    let mut bytes = Vec::new();
    cliff.generate(&mut bytes)?;
    let notes = String::from_utf8(bytes)?.trim().to_string() + "\n";
    super::notes(&notes, &p.name, &version)?;
    Ok(Some(Prepared {
        name: p.name.clone(),
        version,
        notes,
    }))
}
/// git-cliff's native include/exclude path filtering handles monorepo history.
/// Dependency consumers add their shared crate paths; root-only lock changes
/// still need the repository's existing xtask-only bookkeeping exclusion.
pub fn history(root: &Path, p: &Plugin, range: &str) -> Result<Vec<Change>> {
    let repo = Repository::discover(root.to_path_buf())?;
    let mut paths = BTreeSet::from([
        format!("{}/**", p.folder),
        "Cargo.toml".into(),
        "Cargo.lock".into(),
    ]);
    for entry in std::fs::read_dir(root.join("crates"))? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let path = format!("crates/{}", entry.file_name().to_string_lossy());
            if super::depends(root, &p.folder, &path, &mut BTreeSet::new())? {
                paths.insert(format!("{path}/**"));
            }
        }
    }
    let include = paths
        .iter()
        .map(|p| glob::Pattern::new(p))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let exclude = [
        "**/*.md",
        "**/tests/**",
        "**/examples/**",
        "**/mcp.json",
        "**/.env",
    ]
    .iter()
    .map(|s| glob::Pattern::new(s))
    .collect::<std::result::Result<Vec<_>, _>>()?;
    let mut changes = vec![];
    for commit in repo
        .commits(Some(range), Some(include), Some(exclude), true)?
        .into_iter()
        .rev()
    {
        if commit.parent_count() > 1 {
            continue;
        }
        let id = commit.id().to_string();
        let paths = super::git(
            root,
            &["diff-tree", "--no-commit-id", "--name-only", "-r", &id],
        )?
        .lines()
        .map(str::to_string)
        .collect::<Vec<_>>();
        if !super::affected(root, p, &paths)? {
            continue;
        }
        changes.push(Change {
            id,
            message: commit.message().unwrap_or_default().into(),
            author: commit.author().name().ok().map(str::to_string),
            paths,
        });
    }
    Ok(changes)
}

pub fn published_tag(root: &Path, p: &Plugin, version: &str) -> Result<String> {
    let repo = Repository::discover(root.to_path_buf())?;
    let pattern = format!(r"^{}-v[0-9]+\.[0-9]+\.[0-9]+$", p.name).parse()?;
    let expected = format!("{}-v{version}", p.name);
    if !repo
        .tags(&Some(pattern), true, true)?
        .values()
        .any(|tag| tag.name == expected)
    {
        return Err(format!("{}: published baseline tag {expected} is missing or not an ancestor; fetch published tags and preserve promotion ancestry",p.name).into());
    }
    Ok(expected)
}
