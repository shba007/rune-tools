import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StdioClientTransport } from "@modelcontextprotocol/sdk/client/stdio.js";

async function listTools() {
	const transport = new StdioClientTransport({
		command: "D:/Projects/Practice/rune-kit/target/release/rune.exe",
		args: ["run", "rune_fs", "-p", "allowed_dir=D:/Projects/Practice/rune-kit/test-dir"]
	});

	const client = new Client({ name: "mcp-lister", version: "1.0.0" });
	await client.connect(transport);

	const response = await client.listTools();
	console.log(JSON.stringify(response.tools, null, 2));
	await client.close();
}

listTools();