// xtask/src/main.rs
//
// Repo-local build orchestrator, invoked as `cargo xtask <command>` via the
// `.cargo/config.toml` alias. It's a normal Rust binary, not a Cargo
// built-in — which is the point: Cargo builds for exactly one --target per
// invocation, so a wasm32-wasip1 cdylib build and a host-native binary
// build are always two separate `cargo build` processes. This wraps both
// into a single typed command instead of a shell `&&` chain, with real
// error propagation (a failed wasm build stops the run instead of `&&`
// silently skipping straight to reporting shell success/failure only).

use serde_json::Value;
use std::process::{Command, ExitCode};

const WASM_TARGET: &str = "wasm32-wasip1";

struct PluginTargets {
    name: String,
    has_wasm: bool,
    has_native: bool,
}

fn cargo_metadata() -> Value {
    let output = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .output()
        .expect("failed to run `cargo metadata` — is cargo on PATH?");
    serde_json::from_slice(&output.stdout).expect("failed to parse `cargo metadata` output")
}

fn discover_plugins(metadata: &Value, filter: Option<&str>) -> Vec<PluginTargets> {
    let packages = metadata["packages"]
        .as_array()
        .expect("metadata.packages missing");

    packages
        .iter()
        .filter_map(|pkg| {
            let manifest_path = pkg["manifest_path"].as_str()?;
            if !manifest_path.contains("/plugins/") && !manifest_path.contains("\\plugins\\") {
                return None; // skip crates/rune-pdk, crates/rune-sidecar, xtask itself, etc.
            }

            let name = pkg["name"].as_str()?.to_string();
            if let Some(f) = filter
                && name != f
            {
                return None;
            }

            let targets = pkg["targets"].as_array().cloned().unwrap_or_default();
            let has_wasm = targets.iter().any(|t| {
                t["kind"]
                    .as_array()
                    .map(|k| k.iter().any(|k| k == "cdylib"))
                    .unwrap_or(false)
            });
            let has_native = targets.iter().any(|t| {
                t["kind"]
                    .as_array()
                    .map(|k| k.iter().any(|k| k == "bin"))
                    .unwrap_or(false)
            });

            Some(PluginTargets {
                name,
                has_wasm,
                has_native,
            })
        })
        .collect()
}

fn run_cmd(program: &str, args: &[&str]) -> bool {
    println!("→ {} {}", program, args.join(" "));
    let status = Command::new(program)
        .args(args)
        .status()
        .unwrap_or_else(|e| panic!("failed to invoke {}: {}", program, e));
    status.success()
}

fn run_cargo(args: &[&str]) -> bool {
    run_cmd("cargo", args)
}

/// Mirrors build_plugin: one command per plugin. Every plugin has its own
/// .env by convention (even if empty), so this always runs through dotenvx
/// rather than branching on whether the file happens to exist.
fn test_plugin(name: &str) -> bool {
    let env_file = format!("plugins/{}/.env", name);
    run_cmd(
        "dotenvx",
        &[
            "run",
            "-f",
            &env_file,
            "--",
            "cargo",
            "test",
            "-p",
            name,
            "--all-features",
        ],
    )
}

fn build_plugin(plugin: &PluginTargets, wasm: bool, native: bool) -> bool {
    let mut ok = true;

    if wasm && plugin.has_wasm {
        ok &= run_cargo(&[
            "build",
            "-p",
            &plugin.name,
            "--target",
            WASM_TARGET,
            "--release",
            "--lib",
        ]);
    }

    if native && plugin.has_native {
        let bin_name = format!("{}-native", plugin.name);
        ok &= run_cargo(&[
            "build",
            "-p",
            &plugin.name,
            "--release",
            "--bin",
            &bin_name,
            "--features",
            "native",
        ]);
    }

    ok
}

fn print_usage() {
    eprintln!(
        "Usage:\n  \
         cargo xtask build <plugin-name> [--wasm-only | --native-only]\n  \
         cargo xtask build-all [--wasm-only | --native-only]\n  \
         cargo xtask test <plugin-name>\n  \
         cargo xtask test-all"
    );
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let wasm_only = args.iter().any(|a| a == "--wasm-only");
    let native_only = args.iter().any(|a| a == "--native-only");
    let build_wasm = !native_only;
    let build_native = !wasm_only;

    let command = args.first().map(String::as_str);
    let is_build = matches!(command, Some("build") | Some("build-all"));
    let is_test = matches!(command, Some("test") | Some("test-all"));
    let wants_single = matches!(command, Some("build") | Some("test"));

    if !is_build && !is_test {
        print_usage();
        return ExitCode::FAILURE;
    }

    let metadata = cargo_metadata();

    let plugins = if wants_single {
        let Some(name) = args
            .get(1)
            .filter(|a| !a.starts_with("--"))
            .map(String::as_str)
        else {
            eprintln!(
                "error: `cargo xtask {}` requires a plugin name",
                command.unwrap()
            );
            print_usage();
            return ExitCode::FAILURE;
        };
        let found = discover_plugins(&metadata, Some(name));
        if found.is_empty() {
            eprintln!("error: no plugin named '{}' found under plugins/", name);
            return ExitCode::FAILURE;
        }
        found
    } else {
        discover_plugins(&metadata, None)
    };

    let mut all_ok = true;
    for plugin in &plugins {
        println!("== {} ==", plugin.name);
        all_ok &= if is_build {
            build_plugin(plugin, build_wasm, build_native)
        } else {
            test_plugin(&plugin.name)
        };
    }

    if all_ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
