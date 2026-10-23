import importlib.util
import io
import json
from pathlib import Path
import queue
from types import SimpleNamespace
import unittest

spec = importlib.util.spec_from_file_location("kit_smoke", Path(__file__).parents[1] / "smoke-kit-plugins.py")
smoke = importlib.util.module_from_spec(spec)
spec.loader.exec_module(smoke)


class GatewayWireTests(unittest.TestCase):
    def gateway(self, legacy=False):
        gateway = smoke.Gateway.__new__(smoke.Gateway)
        gateway.legacy = legacy
        gateway.counter = 0
        gateway.process = SimpleNamespace(stdin=io.StringIO())
        gateway.responses = queue.Queue()
        for i in [1, 2]:
            gateway.responses.put({"jsonrpc": "2.0", "id": i, "result": {
                "resultType": "complete", "_meta": {"io.modelcontextprotocol/serverInfo": {"name": "fixture"}}}})
        return gateway

    def test_each_current_request_carries_version_and_empty_capabilities(self):
        gateway = self.gateway()
        gateway.send("server/discover", {})
        gateway.send("tools/list", {})
        requests = [json.loads(line) for line in gateway.process.stdin.getvalue().splitlines()]
        self.assertEqual([r["method"] for r in requests], ["server/discover", "tools/list"])
        for request in requests:
            meta = request["params"]["_meta"]
            self.assertEqual(meta["io.modelcontextprotocol/protocolVersion"], "2026-07-28")
            self.assertEqual(meta["io.modelcontextprotocol/clientCapabilities"], {})
            self.assertIn("name", meta["io.modelcontextprotocol/clientInfo"])
            self.assertIn("version", meta["io.modelcontextprotocol/clientInfo"])

    def test_legacy_fixture_is_explicit_and_keeps_its_old_wire(self):
        gateway = self.gateway(legacy=True)
        gateway.send("tools/list", {})
        self.assertNotIn("_meta", json.loads(gateway.process.stdin.getvalue())["params"])

    def test_missing_metadata_negative_probe_is_explicit(self):
        gateway = self.gateway()
        gateway.send("tools/list", {}, metadata=False)
        self.assertNotIn("_meta", json.loads(gateway.process.stdin.getvalue())["params"])


if __name__ == "__main__":
    unittest.main()
