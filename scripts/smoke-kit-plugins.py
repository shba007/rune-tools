"""Exercise real native/WASM or installed pilot plugins through Rune Kit.

Usage: python scripts/smoke-kit-plugins.py --rune PATH --git PATH_OR_NAME --time PATH_OR_NAME
No installation or owner configuration is changed by this script.
"""

import argparse
import json
import queue
import subprocess
import threading


class Gateway:
    def __init__(self, rune, plugin, restricted):
        self.process = subprocess.Popen(
            [rune, "run", plugin, *(["--read-only"] if restricted else [])],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            text=True, encoding="utf-8",
        )
        self.responses = queue.Queue()
        self.stderr = []
        self.counter = 0
        threading.Thread(target=self._read, daemon=True).start()
        threading.Thread(target=lambda: self.stderr.extend(self.process.stderr), daemon=True).start()

    def _read(self):
        for line in self.process.stdout:
            self.responses.put(json.loads(line))

    def send(self, method, params, notification=False):
        request = {"jsonrpc": "2.0", "method": method, "params": params}
        if not notification:
            self.counter += 1
            request["id"] = self.counter
        self.process.stdin.write(json.dumps(request) + "\n")
        self.process.stdin.flush()
        if notification:
            return None
        while True:
            response = self.responses.get(timeout=60)
            if response.get("id") == self.counter:
                assert "error" not in response, response
                return response["result"]

    def close(self):
        self.process.stdin.close()
        try:
            self.process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            self.process.kill()
            self.process.wait(timeout=10)


def check(rune, plugin, kind, restricted):
    gateway = Gateway(rune, plugin, restricted)
    try:
        initialized = gateway.send("initialize", {
            "protocolVersion": "2025-03-26", "capabilities": {},
            "clientInfo": {"name": "rune-tools-cycle1", "version": "1"},
        })
        gateway.send("notifications/initialized", {}, notification=True)
        tools = gateway.send("tools/list", {})["tools"]
        by_name = {tool["name"].split("__")[-1]: tool for tool in tools}
        expected = {"validate_commit_message"} if kind == "git" else {"get_current_time", "convert_time"}
        assert expected <= by_name.keys(), {"tools": list(by_name), "stderr": gateway.stderr}
        if restricted:
            assert set(by_name) == expected, by_name.keys()
        for tool in tools:
            rules = tool["inputSchema"]["x-rune-rules"]
            ann = tool["annotations"]
            assert ann["readOnlyHint"] == ann["readOnly"] == rules["read_only"]
            assert ann["destructiveHint"] == ann["destructive"] == bool({"exec", "delete"} & set(rules.get("effects", [])))
            assert ann["openWorldHint"] == ann["openWorld"] == ("network" in rules.get("effects", []))
            assert "idempotent" not in ann and "idempotentHint" not in ann
        samples = {
            "validate_commit_message": {"message": "fix: verify plugin annotations", "conventional": True},
            "get_current_time": {"timezone": "Asia/Kolkata"},
            "convert_time": {"time": "2026-10-21T12:00:00", "sourceTimezone": "UTC", "targetTimezone": "Asia/Kolkata"},
        }
        calls = {}
        for name in sorted(expected):
            tool = by_name[name]
            arguments = samples[name]
            schema = tool["inputSchema"]
            assert arguments.keys() <= schema["properties"].keys()
            assert set(schema.get("required", [])) <= arguments.keys()
            result = gateway.send("tools/call", {"name": tool["name"], "arguments": arguments})
            assert not result.get("isError", False), result
            text = "\n".join(part.get("text", "") for part in result.get("content", []))
            parsed = json.loads(text) if text else result
            assert parsed.get("status") != "error" and "error" not in parsed, parsed
            value = parsed.get("result", parsed)
            if name == "validate_commit_message":
                assert value["valid"] is True, value
            elif name == "get_current_time":
                assert value["timezone"] == "Asia/Kolkata" and value["utcOffset"] == "+05:30", value
            else:
                assert value["target"]["datetime"] == "2026-10-21T17:30:00+05:30", value
            calls[name] = result
        return {"plugin": plugin, "read_only": restricted, "initialize": initialized,
                "tool_names": sorted(by_name), "calls": calls}
    finally:
        gateway.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rune", required=True)
    parser.add_argument("--git", required=True)
    parser.add_argument("--time", required=True)
    parser.add_argument("--report")
    args = parser.parse_args()
    report = {"rune_version": subprocess.check_output([args.rune, "--version"], text=True).strip(),
              "exchanges": [check(args.rune, plugin, kind, restricted)
                            for kind, plugin in [("git", args.git), ("time", args.time)]
                            for restricted in [False, True]]}
    encoded = json.dumps(report, indent=2)
    if args.report:
        with open(args.report, "w", encoding="utf-8") as output:
            output.write(encoded + "\n")
    print(encoded)


if __name__ == "__main__":
    main()
