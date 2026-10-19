// xtask/src/hooks.rs
//
// `cargo xtask hooks` installs .cargo-husky/hooks/* into the repo's git hooks
// directory. cargo-husky does the same automatically when xtask's tests are
// compiled; this is the explicit, no-test-run way to do it.

use std::{fs, path::Path, process::Command};

const SOURCE_DIR: &str = ".cargo-husky/hooks";

pub fn install() -> Result<(), String> {
    let out = Command::new("git")
        .args(["rev-parse", "--git-path", "hooks"])
        .output()
        .map_err(|e| format!("failed to run git: {e}"))?;
    if !out.status.success() {
        return Err("not inside a git repository".into());
    }
    let dest = String::from_utf8_lossy(&out.stdout).trim().to_string();
    fs::create_dir_all(&dest).map_err(|e| e.to_string())?;

    let entries = fs::read_dir(SOURCE_DIR).map_err(|e| format!("cannot read {SOURCE_DIR}: {e}"))?;
    for entry in entries.flatten() {
        let src = entry.path();
        if !src.is_file() {
            continue;
        }
        let target = Path::new(&dest).join(entry.file_name());
        fs::copy(&src, &target).map_err(|e| format!("copy {}: {e}", src.display()))?;
        make_executable(&target)?;
        println!("installed {}", target.display());
    }
    Ok(())
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())
}

#[cfg(not(unix))]
fn make_executable(_: &Path) -> Result<(), String> {
    Ok(())
}
