//! Plain dotenv files use Rust. Retain dotenvx decryption and advanced expansion.
use std::process::Command;
fn needs_dotenvx(text: &str) -> bool {
    text.contains("encrypted:")
        || text.contains("$(")
        || text.match_indices("${").any(|(i, _)| {
            let expression = text[i + 2..].split('}').next().unwrap_or_default();
            expression
                .chars()
                .any(|c| !c.is_ascii_alphanumeric() && c != '_')
        })
}
pub fn test_plugin(name: &str) -> bool {
    let path = format!("plugins/{name}/.env");
    let text = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(_) => {
            eprintln!("Cannot read {path}");
            return false;
        }
    };
    let mut command = if needs_dotenvx(&text) {
        let mut c = Command::new("dotenvx");
        c.args(["run", "-f", &path, "--", "cargo"]);
        c
    } else {
        let mut c = Command::new("cargo");
        let vars = dotenvy::from_read_iter(text.as_bytes());
        for item in vars {
            match item {
                Ok((k, v)) => {
                    if std::env::var_os(&k).is_none() {
                        c.env(k, v);
                    }
                }
                Err(_) => {
                    eprintln!("Invalid dotenv entry in {path}");
                    return false;
                }
            }
        }
        c
    };
    command.args([
        "test",
        "--locked",
        "-p",
        name,
        "--all-features",
        "--",
        "--show-output",
    ]);
    command.status().is_ok_and(|s| s.success())
}
pub fn requirements() -> crate::release::Result<()> {
    let mut encrypted = false;
    for p in crate::release::plugins(std::path::Path::new("."))? {
        let path = std::path::Path::new(&p.folder).join(".env");
        match std::fs::read_to_string(path) {
            Ok(s) => encrypted |= needs_dotenvx(&s),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    crate::release::output("dotenvx", encrypted)
}
#[cfg(test)]
mod tests {
    #[test]
    fn advanced_dotenv_capabilities_are_retained() {
        assert!(super::needs_dotenvx("KEY=encrypted:example"));
        assert!(super::needs_dotenvx("KEY=$(example-command)"));
        assert!(super::needs_dotenvx("KEY=${OTHER:-fallback}"));
        assert!(!super::needs_dotenvx("KEY=${OTHER}\nPLAIN='some value'"));
    }
    #[test]
    fn dotenv_values_and_substitution() {
        let vars: Vec<_> = dotenvy::from_read_iter(
            b"FIRST=local\nSECOND='with spaces'\nTHIRD=${FIRST}\n".as_slice(),
        )
        .collect::<Result<_, _>>()
        .unwrap();
        assert_eq!(vars[0], ("FIRST".into(), "local".into()));
        assert_eq!(vars[1].1, "with spaces");
        assert_eq!(vars[2].1, "local");
    }
}
