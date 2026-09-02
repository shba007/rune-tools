#!/usr/bin/env python3
"""
Diffs each `plugins/*` workspace member's Cargo.toml version against
registry/index.json and emits wasm/native GitHub Actions build matrices
containing only plugin versions that haven't been published yet.

Output: a single JSON object printed to stdout:
  {
    "publishable": {"include": [...]},
    "wasm":        {"include": [...]},
    "native":      {"include": [...]}
  }
"""
import json
import subprocess
from pathlib import Path

REGISTRY_PATH = Path("registry/index.json")

# Native sidecar build targets — mirrors rune-kit's own deploy.yml matrix.
NATIVE_TARGETS = [
    {"platform": "ubuntu-latest", "target": "x86_64-unknown-linux-gnu", "ext": "tar.gz"},
    {"platform": "macos-latest", "target": "aarch64-apple-darwin", "ext": "tar.gz"},
    {"platform": "macos-latest", "target": "x86_64-apple-darwin", "ext": "tar.gz"},
    {"platform": "windows-latest", "target": "x86_64-pc-windows-msvc", "ext": "zip"},
]


def load_registry() -> dict:
    if REGISTRY_PATH.exists():
        return json.loads(REGISTRY_PATH.read_text())
    return {}


def cargo_metadata() -> dict:
    raw = subprocess.check_output(["cargo", "metadata", "--no-deps", "--format-version", "1"])
    return json.loads(raw)


def main() -> None:
    registry = load_registry()
    metadata = cargo_metadata()

    publishable = []
    wasm_matrix = []
    native_matrix = []

    for pkg in metadata["packages"]:
        manifest_path = Path(pkg["manifest_path"])
        if "plugins" not in manifest_path.parts:
            continue  # skip crates/rune-pdk, crates/rune-sidecar, etc.

        name = pkg["name"]
        version = pkg["version"]
        description = pkg.get("description") or ""

        already_published = version in registry.get(name, {}).get("versions", {})
        if already_published:
            continue

        cdylib_target = next(
            (t["name"] for t in pkg["targets"] if "cdylib" in t["kind"]), None
        )
        bin_targets = [t["name"] for t in pkg["targets"] if "bin" in t["kind"]]

        if not cdylib_target and not bin_targets:
            continue  # nothing buildable — e.g. a lib-only helper crate

        publishable.append({
            "name": name,
            "version": version,
            "description": description,
            "has_wasm": cdylib_target is not None,
            "has_native": len(bin_targets) > 0,
        })

        if cdylib_target:
            wasm_matrix.append({
                "name": name,
                "version": version,
                "artifact_name": cdylib_target,
            })

        for bin_name in bin_targets:
            for nt in NATIVE_TARGETS:
                native_matrix.append({
                    "name": name,
                    "version": version,
                    "bin_name": bin_name,
                    "platform": nt["platform"],
                    "target": nt["target"],
                    "ext": nt["ext"],
                })

    print(json.dumps({
        "publishable": {"include": publishable},
        "wasm": {"include": wasm_matrix},
        "native": {"include": native_matrix},
    }))


if __name__ == "__main__":
    main()