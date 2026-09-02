#!/usr/bin/env python3
"""
Runs after build-wasm/build-native have uploaded release assets. Re-derives
the same "what's new" list as detect-publishable.py and merges each new
version into registry/index.json, preserving every previously published
version untouched.

Release tag convention: "<plugin-name>-v<version>"
Asset naming convention:
  wasm:   "<plugin-name>-<version>.wasm"
  native: "<bin-name>-<version>-<target-triple>.<ext>"
"""
import json
import os
import subprocess
from pathlib import Path

REGISTRY_PATH = Path("registry/index.json")
REPO = os.environ["GITHUB_REPOSITORY"]  # e.g. "shba007/rune-tools"

NATIVE_TARGETS = [
    "x86_64-unknown-linux-gnu",
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
    "x86_64-pc-windows-msvc",
]
EXT_BY_TARGET = {
    "x86_64-pc-windows-msvc": "zip",
}


def release_asset_url(tag: str, filename: str) -> str:
    return f"https://github.com/{REPO}/releases/download/{tag}/{filename}"


def cargo_metadata() -> dict:
    raw = subprocess.check_output(["cargo", "metadata", "--no-deps", "--format-version", "1"])
    return json.loads(raw)


def main() -> None:
    registry = json.loads(REGISTRY_PATH.read_text()) if REGISTRY_PATH.exists() else {}
    metadata = cargo_metadata()

    changed = False

    for pkg in metadata["packages"]:
        manifest_path = Path(pkg["manifest_path"])
        if "plugins" not in manifest_path.parts:
            continue

        name = pkg["name"]
        version = pkg["version"]
        description = pkg.get("description") or ""

        entry = registry.setdefault(name, {"latest": version, "description": description, "versions": {}})

        if version in entry["versions"]:
            continue  # already recorded — this run didn't publish a new version for this plugin

        cdylib_target = next((t["name"] for t in pkg["targets"] if "cdylib" in t["kind"]), None)
        bin_targets = [t["name"] for t in pkg["targets"] if "bin" in t["kind"]]

        if not cdylib_target and not bin_targets:
            continue

        tag = f"{name}-v{version}"
        version_entry = {}

        if cdylib_target:
            version_entry["url"] = release_asset_url(tag, f"{name}-{version}.wasm")

        if bin_targets:
            native = {}
            for bin_name in bin_targets:
                for target in NATIVE_TARGETS:
                    ext = EXT_BY_TARGET.get(target, "tar.gz")
                    native[target] = release_asset_url(tag, f"{bin_name}-{version}-{target}.{ext}")
            version_entry["native"] = native

        entry["versions"][version] = version_entry
        entry["latest"] = version
        entry["description"] = description
        changed = True

    if changed:
        REGISTRY_PATH.parent.mkdir(parents=True, exist_ok=True)
        REGISTRY_PATH.write_text(json.dumps(registry, indent=2, sort_keys=True) + "\n")
        print("registry/index.json updated")
    else:
        print("No new publishable versions — registry unchanged")


if __name__ == "__main__":
    main()