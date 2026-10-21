#!/usr/bin/env bun
import { existsSync } from "node:fs";
import { mkdir, readdir, readFile, rm, cp, writeFile, mkdtemp } from "node:fs/promises";
import { join, basename, extname } from "node:path";
import { tmpdir, homedir } from "node:os";
import { $ } from "bun";

const repoUrl = process.argv[2];
const customName = process.argv[3];

if (!repoUrl) {
	console.error("Usage: bun install-skill.ts <github_direct_url> [master_name]");
	console.error("Example: bun install-skill.ts https://github.com/DietrichGebert/ponytail.git");
	process.exit(1);
}

const repoBasename = basename(repoUrl, ".git");
const masterName = customName || repoBasename;

const zcodeBase = process.env.ZCODE_DIR || join(homedir(), ".zcode");
const targetSkillDir = join(zcodeBase, "skills", masterName);
const targetCommandDir = join(zcodeBase, "commands", masterName);

const tmpBase = await mkdtemp(join(tmpdir(), "zcode-install-"));

// Default argument hints for known command types if omitted in source
const DEFAULT_ARG_HINTS: Record<string, string> = {
	audit: "[file | directory | manifest]",
	review: "[target | diff | commit]",
	debt: "[target file | component]",
	gain: "[target comparison]",
	ponytail: "[prompt | instruction]",
	help: "",
};

function stripNamespacePrefix(name: string, prefix: string): string {
	const regex = new RegExp(`^${prefix}[-_]`, "i");
	return name.replace(regex, "");
}

function normalizeToMarkdown(rawContent: string, fileName: string, isToml: boolean): string {
	const baseTitle = basename(fileName, extname(fileName));
	const fallbackHint = DEFAULT_ARG_HINTS[baseTitle.toLowerCase()] ?? "";

	let description = `${baseTitle} command`;
	let argHint = fallbackHint;
	let body = "";

	if (isToml) {
		try {
			const parsed = Bun.TOML.parse(rawContent) as Record<string, any>;
			const node = parsed.command && typeof parsed.command === "object" ? parsed.command : parsed;

			description = (
				node.description ||
				node.desc ||
				node.summary ||
				`${baseTitle} command`
			).toString().replace(/"/g, '\\"');

			argHint = (
				node["argument-hint"] ||
				node.argumentHint ||
				node.argument_hint ||
				node["arg-hint"] ||
				node.argHint ||
				node.args ||
				fallbackHint
			).toString().replace(/"/g, '\\"');

			body =
				node.prompt ||
				node.content ||
				node.instructions ||
				node.template ||
				node.message ||
				node.body;

			if (!body) {
				const remainingKeys = Object.keys(node).filter(
					(k) => !["description", "desc", "summary", "name", "title", "argument-hint", "argumentHint", "arg-hint", "args"].includes(k)
				);
				body = remainingKeys
					.map((k) => (typeof node[k] === "string" ? node[k] : JSON.stringify(node[k], null, 2)))
					.join("\n\n");
			}
		} catch {
			body = rawContent;
		}
	} else {
		// Process existing Markdown
		const trimmed = rawContent.trim();
		const frontmatterMatch = trimmed.match(/^---\r?\n([\s\S]*?)\r?\n---\r?\n([\s\S]*)$/);

		if (frontmatterMatch) {
			const yamlBlock = frontmatterMatch[1];
			body = frontmatterMatch[2].trim();

			const descMatch = yamlBlock.match(/description:\s*["']?(.*?)["']?$/m);
			if (descMatch) description = descMatch[1].replace(/"/g, '\\"');

			const hintMatch = yamlBlock.match(/(?:argument-hint|argumentHint|arg-hint):\s*["']?(.*?)["']?$/m);
			if (hintMatch) argHint = hintMatch[1].replace(/"/g, '\\"');
		} else {
			const headingMatch = trimmed.match(/^#+\s+(.+)$/m);
			if (headingMatch) description = headingMatch[1].replace(/"/g, '\\"').trim();
			body = trimmed;
		}
	}

	const frontmatter = [
		"---",
		`description: "${description}"`,
		argHint ? `argument-hint: "${argHint}"` : null,
		"---",
		"",
		(body || "").toString().trim(),
		"",
	].filter((line) => line !== null).join("\n");

	return frontmatter;
}

async function findFilesRecursively(dir: string, matcher: (file: string) => boolean): Promise<string[]> {
	const results: string[] = [];
	if (!existsSync(dir)) return results;

	const entries = await readdir(dir, { withFileTypes: true });
	for (const entry of entries) {
		const fullPath = join(dir, entry.name);
		if (entry.isDirectory()) {
			results.push(...(await findFilesRecursively(fullPath, matcher)));
		} else if (matcher(entry.name)) {
			results.push(fullPath);
		}
	}
	return results;
}

try {
	console.log(`==> Cloning ${repoUrl} (shallow)...`);
	await $`git clone --depth 1 --quiet ${repoUrl} ${tmpBase}/repo`;

	const repoPath = join(tmpBase, "repo");

	// -------------------------------------------------------------
	// 1. Process & Install Skills (~/.zcode/skills/<masterName>)
	// -------------------------------------------------------------
	await rm(targetSkillDir, { recursive: true, force: true });
	await mkdir(targetSkillDir, { recursive: true });

	const skillsRepoDir = join(repoPath, "skills");
	const rootSkillMd = join(repoPath, "SKILL.md");

	if (existsSync(skillsRepoDir)) {
		console.log(`==> Extracting skills into: ${targetSkillDir}`);
		const skillEntries = await readdir(skillsRepoDir, { withFileTypes: true });

		for (const entry of skillEntries) {
			const entryPath = join(skillsRepoDir, entry.name);
			if (entry.isDirectory() && existsSync(join(entryPath, "SKILL.md"))) {
				if (entry.name === masterName) {
					console.log(`    -> Flattening primary skill '${entry.name}' to root (${targetSkillDir}/SKILL.md)`);
					await cp(entryPath, targetSkillDir, { recursive: true });
				} else {
					const cleanSkillName = stripNamespacePrefix(entry.name, masterName);
					console.log(`    -> Nesting companion sub-skill: ${cleanSkillName}/`);
					await cp(entryPath, join(targetSkillDir, cleanSkillName), { recursive: true });
				}
			}
		}
	} else if (existsSync(rootSkillMd)) {
		console.log(`==> Found root SKILL.md. Installing into ${targetSkillDir}...`);
		await cp(rootSkillMd, join(targetSkillDir, "SKILL.md"));

		for (const folder of ["modes", "prompts", "rules", "references", "scripts", "assets"]) {
			const folderPath = join(repoPath, folder);
			if (existsSync(folderPath)) {
				await cp(folderPath, join(targetSkillDir, folder), { recursive: true });
			}
		}
	}

	// -------------------------------------------------------------
	// 2. Process, Convert & Install Commands (~/.zcode/commands/<masterName>)
	// -------------------------------------------------------------
	const commandCandidates = [
		join(repoPath, "commands"),
		join(repoPath, ".claude", "commands"),
		join(repoPath, ".zcode", "commands"),
	];

	let sourceCommandsDir: string | null = null;
	for (const candidate of commandCandidates) {
		if (existsSync(candidate)) {
			sourceCommandsDir = candidate;
			break;
		}
	}

	if (sourceCommandsDir) {
		await rm(targetCommandDir, { recursive: true, force: true });
		await mkdir(targetCommandDir, { recursive: true });

		console.log(`==> Converting and formatting commands into: ${targetCommandDir}`);

		const commandFiles = await findFilesRecursively(
			sourceCommandsDir,
			(f) => f.endsWith(".toml") || f.endsWith(".md")
		);

		for (const file of commandFiles) {
			const ext = extname(file);
			const isToml = ext === ".toml";
			const rawBaseName = basename(file, ext);
			const cleanName = stripNamespacePrefix(rawBaseName, masterName);

			const targetMdPath = join(targetCommandDir, `${cleanName}.md`);
			const raw = await readFile(file, "utf-8");
			const formattedMd = normalizeToMarkdown(raw, cleanName, isToml);

			await writeFile(targetMdPath, formattedMd, "utf-8");
			console.log(`    -> Generated ${masterName}/${cleanName}.md`);
		}
	}

	console.log("\n==> Done!");
	console.log(`Skills:   ${targetSkillDir}`);
	console.log(`Commands: ${targetCommandDir}`);
} finally {
	await rm(tmpBase, { recursive: true, force: true });
}