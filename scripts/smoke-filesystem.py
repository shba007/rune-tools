"""Check actual filesystem WASM through released Kit, without installing anything.

Fixture files are retained under --fixture (which must be disposable).
For a real Windows junction test, first create <fixture>/root/escape pointing
to <fixture>/root-other, then pass --require-link. No link is removed here.
"""
import argparse
import importlib.util
import json
from pathlib import Path
import subprocess
import threading
import os

spec = importlib.util.spec_from_file_location("kit_smoke", Path(__file__).with_name("smoke-kit-plugins.py"))
kit = importlib.util.module_from_spec(spec)
spec.loader.exec_module(kit)


def check(rune, plugin, fixture, require_link=False, legacy=False):
    fixture = Path(fixture).resolve()
    root = fixture / "root"
    outside = fixture / "root-other"
    root.mkdir(parents=True, exist_ok=True)
    outside.mkdir(exist_ok=True)
    (root / "child").mkdir(exist_ok=True)
    (root / "note.txt").write_text("inside marker\nsecond\nthird\n", encoding="utf-8")
    (outside / "secret.txt").write_text("outside marker", encoding="utf-8")
    (root / "pages.txt").write_text(("x" * 100 + "\n") * 3000, encoding="utf-8")
    (root / "long.txt").write_bytes(b"x" * 65537)
    (root / "large.txt").write_bytes(b"x" * (8 * 1024 * 1024 + 1))
    many = root / "many"
    many.mkdir(exist_ok=True)
    for i in range(220):
        (many / f"entry-{i:03}").touch()
    normal = kit.Gateway(rune, plugin, False, ["allowed_dir=" + root.as_posix()], legacy=legacy)
    try:
        normal.discover()
        discovered = normal.send("tools/list", {})["tools"]
        approved = set()
        for tool in discovered:
            ann = tool["annotations"]
            rules = tool["inputSchema"]["x-rune-rules"]
            assert ann["readOnly"] == ann["readOnlyHint"] == rules["read_only"]
            assert ann["destructive"] == ann["destructiveHint"] == bool({"delete", "exec"} & set(rules.get("effects", [])))
            assert ann["openWorld"] == ann["openWorldHint"] == ("network" in rules.get("effects", []))
            assert "idempotent" not in ann and "idempotentHint" not in ann
            if rules["read_only"]:
                approved.add(tool["name"].split("__")[-1])
        assert approved == {"read_text_file", "list_directory"}
    finally:
        normal.close()
    gateway = kit.Gateway(rune, plugin, True, ["allowed_dir=" + root.as_posix()], legacy=legacy)
    report = {"root": root.as_posix(), "plugin": plugin, "normal_discovery_count": len(discovered),
              "normal_tools": discovered, "refusals": []}
    try:
        report["discovery"] = gateway.discover()
        tools = gateway.send("tools/list", {})["tools"]
        names = {t["name"].split("__")[-1]: t for t in tools}
        assert set(names) == {"read_text_file", "list_directory"}, names
        report["tools"] = tools
        for tool in tools:
            ann = tool["annotations"]
            assert ann["readOnly"] is True and ann["readOnlyHint"] is True
            assert ann["destructive"] is False and ann["destructiveHint"] is False
            assert ann["openWorld"] is False and ann["openWorldHint"] is False
            assert tool["inputSchema"]["x-rune-rules"]["limits"]["root_param"] == "allowed_dir"

        def call(name, args, refuse=False):
            result = gateway.send("tools/call", {"name": names[name]["name"], "arguments": args})
            text = result["content"][0]["text"]
            try:
                value = json.loads(text)
            except json.JSONDecodeError:
                assert result.get("isError"), result
                value = {"status": "error", "error": text}
            if refuse:
                assert value.get("status") == "error" or result.get("isError"), value
                report["refusals"].append({"tool": name, "arguments": args,
                                           "mcp_is_error": result.get("isError", False), "response": value})
                return value
            assert value.get("status") == "success" and not result.get("isError"), value
            return value["result"]

        report["read"] = call("read_text_file", {"path": "note.txt", "lineLimit": 1})
        assert report["read"]["content"] == "inside marker"
        assert report["read"]["nextLineOffset"] == 1
        assert call("read_text_file", {"path": "note.txt", "lineOffset": 1, "lineLimit": 1})["content"] == "second"
        assert call("read_text_file", {"path": "note.txt", "tail": 1})["content"] == "third"
        assert call("read_text_file", {"path": (root / "note.txt").as_posix(), "head": 1})["content"] == "inside marker"
        report["listing"] = call("list_directory", {"path": "."})
        assert "[DIR] child" in report["listing"]["content"]
        for path in ["../root-other/secret.txt", (outside / "secret.txt").as_posix(), str(outside / "secret.txt"), "child/../../root-other/secret.txt"]:
            call("read_text_file", {"path": path}, True)
        call("list_directory", {"path": outside.as_posix()}, True)
        for key in ["allowedDir", "allowed_dir"]:
            call("read_text_file", {"path": "note.txt", key: outside.as_posix()}, True)
        for arguments in [{"path": "large.txt"}, {"path": "long.txt"}, {"path": "note.txt", "lineLimit": 2001}, {"path": "note.txt", "head": 1, "tail": 1}, {"path": "note.txt", "tail": -1}]:
            call("read_text_file", arguments, True)
        bounded = call("read_text_file", {"path": "pages.txt"})
        assert len(bounded["content"].encode()) <= 65536 and bounded["linesReturned"] <= 2000 and bounded["hasMore"]
        report["text_budget"] = {k: v for k, v in bounded.items() if k != "content"}
        bounded = call("list_directory", {"path": "many"})
        assert bounded["entriesReturned"] == 200 and bounded["truncated"] and len(bounded["content"].encode()) <= 65536
        report["directory_budget"] = {k: v for k, v in bounded.items() if k != "content"}
        call("list_directory", {"path": "many", "maxEntries": 201}, True)
        linked = (root / "escape").exists()
        assert linked or not require_link, "Real escape link required; create the junction first"
        if linked:
            assert (root / "escape" / "secret.txt").read_text() == "outside marker", "Escape fixture must actually reach outside on host"
            call("read_text_file", {"path": "escape/secret.txt"}, True)
            call("list_directory", {"path": "escape"}, True)
            report["real_link_escape"] = "refused"
            # Swap an in-root directory with the real outside junction while
            # the gateway reads. This exercises resolution during replacement.
            active, parked, safe = root / "race", root / "race-link", root / "race-safe"
            assert not any(p.exists() for p in [active, parked, safe]), "Use a fresh fixture for race testing"
            active.mkdir()
            (active / "secret.txt").write_text("inside race", encoding="utf-8")
            os.rename(root / "escape", parked)
            stop = threading.Event()
            swaps = [0]
            def swap():
                while not stop.is_set():
                    try:
                        if not safe.exists():
                            os.rename(active, safe)
                        if parked.exists():
                            os.rename(parked, active)
                            swaps[0] += 1
                        stop.wait(0.001)
                        os.rename(active, parked)
                        os.rename(safe, active)
                        stop.wait(0.001)
                    except OSError:
                        # Windows may deny renaming a directory currently open.
                        # Resume the partially completed swap on the next pass.
                        stop.wait(0.001)
            thread = threading.Thread(target=swap)
            thread.start()
            successes, refusals = 0, 0
            try:
                for _ in range(60):
                    value = gateway.send("tools/call", {"name": names["read_text_file"]["name"], "arguments": {"path": "race/secret.txt"}})
                    value = json.loads(value["content"][0]["text"])
                    if value.get("status") == "success":
                        assert value["result"]["content"] == "inside race", value
                        successes += 1
                    else:
                        assert value.get("status") == "error", value
                        refusals += 1
            finally:
                stop.set()
                thread.join(timeout=10)
                assert not thread.is_alive()
                # Restore the fixture names without recursive deletion.
                if safe.exists():
                    if active.exists(): os.rename(active, parked)
                    os.rename(safe, active)
                os.rename(parked, root / "escape")
                (active / "secret.txt").unlink()
                active.rmdir()
            assert swaps[0] > 0, "No replacement actually occurred"
            report["replacement_race"] = {"swaps": swaps[0], "inside_successes": successes, "refusals": refusals, "outside_successes": 0}
        else:
            report["real_link_escape"] = "NOT RUN: platform link fixture absent"
        prefix = names["read_text_file"]["name"].removesuffix("read_text_file")
        response = gateway.send("tools/call", {"name": prefix + "write_file", "arguments": {"path": "blocked.txt", "content": "forbidden"}}, allow_error=True)
        assert "error" in response, response
        assert not (root / "blocked.txt").exists()
        report["write_refusal"] = response
    finally:
        gateway.close()
    for params in [[], ["allowed_dir=" + (fixture / "missing").as_posix()]]:
        missing = kit.Gateway(rune, plugin, True, params, legacy=legacy)
        try:
            # Kit versions may reject an invalid preopen before discovery.
            missing.discover()
            tool = missing.send("tools/list", {})["tools"][0]
            response = missing.send("tools/call", {"name": tool["name"], "arguments": {"path": "."}})
            assert json.loads(response["content"][0]["text"])["status"] == "error", response
        except RuntimeError as error:
            assert params and missing.process.returncode != 0, str(error)
            assert any(s in str(error).lower() for s in ["missing", "directory", "os error 2"]), str(error)
            report["invalid_root_startup_error"] = str(error)
        finally:
            missing.close()
    report["missing_invalid_root"] = "refused"
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rune", required=True)
    parser.add_argument("--plugin", required=True)
    parser.add_argument("--fixture", required=True)
    parser.add_argument("--require-link", action="store_true")
    parser.add_argument("--report", required=True)
    parser.add_argument("--legacy-protocol", action="store_true", help="Explicit Kit <=0.4.0 compatibility fixture")
    args = parser.parse_args()
    report = check(args.rune, args.plugin, args.fixture, args.require_link, args.legacy_protocol)
    report["protocol"] = "legacy-2025-03-26" if args.legacy_protocol else "2026-07-28"
    report["kit_version"] = subprocess.check_output([args.rune, "--version"], text=True).strip()
    Path(args.report).write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps({k:v for k,v in report.items() if k not in {"tools", "normal_tools", "refusals"}}, indent=2))


if __name__ == "__main__":
    main()
