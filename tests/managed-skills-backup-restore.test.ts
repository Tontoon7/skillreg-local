import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import {
	chmodSync,
	existsSync,
	linkSync,
	lstatSync,
	mkdirSync,
	mkdtempSync,
	readFileSync,
	readlinkSync,
	realpathSync,
	rmSync,
	symlinkSync,
	writeFileSync,
} from "node:fs";
import { join, parse, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const script = fileURLToPath(
	new URL("../scripts/managed-skills-backup-restore.mjs", import.meta.url),
);

function fixture() {
	const root = mkdtempSync(resolve(".rollback-fixture-"));
	const home = join(root, "profil été");
	const backup = join(root, "before");
	const recovery = join(root, "after");
	mkdirSync(home);
	for (const directory of [
		".skillreg/env",
		".claude/skills/original",
		".agents/skills",
		"project",
	]) {
		mkdirSync(join(home, directory), { recursive: true });
	}
	writeFileSync(join(home, ".skillreg/config.json"), '{"token":"fixture-access"}', { mode: 0o600 });
	writeFileSync(join(home, ".skillreg/env/acme.json"), '{"fixture":"offline"}', { mode: 0o600 });
	writeFileSync(join(home, ".skillreg/installed.json"), '{"version":1,"installations":[]}');
	writeFileSync(join(home, ".claude/skills/original/SKILL.md"), "legacy original");
	writeFileSync(join(home, "project/keep.txt"), "project untouched");
	const external = join(root, "external");
	mkdirSync(external);
	writeFileSync(join(external, "SKILL.md"), "external untouched");
	symlinkSync(
		external,
		join(home, ".agents/skills/external"),
		process.platform === "win32" ? "junction" : "dir",
	);
	return { root, home, backup, recovery, external };
}

function run(command: string, paths: ReturnType<typeof fixture>, extra: string[] = []) {
	return spawnSync(
		process.execPath,
		[script, command, "--home", paths.home, "--backup", paths.backup, ...extra],
		{ encoding: "utf8" },
	);
}

function success(result: ReturnType<typeof run>) {
	assert.equal(result.status, 0, result.stderr);
	return JSON.parse(result.stdout);
}

test("complete snapshot restores files, permissions and external links, preserving later data separately", (t) => {
	const paths = fixture();
	t.after(() => rmSync(paths.root, { recursive: true, force: true }));
	const target = readlinkSync(join(paths.home, ".agents/skills/external"));
	success(run("snapshot", paths));
	success(run("verify", paths));
	writeFileSync(join(paths.home, ".skillreg/config.json"), '{"token":"later-fixture-access"}');
	writeFileSync(join(paths.home, ".claude/skills/original/SKILL.md"), "modified after snapshot");
	mkdirSync(join(paths.home, ".cursor"));
	writeFileSync(join(paths.home, ".cursor/later.txt"), "new data");
	const preview = success(run("restore", paths));
	assert.equal(preview.status, "preview");
	assert.equal(
		readFileSync(join(paths.home, ".claude/skills/original/SKILL.md"), "utf8"),
		"modified after snapshot",
	);
	assert.equal(existsSync(paths.recovery), false);
	success(run("restore", paths, ["--apply", "--recovery", paths.recovery]));
	assert.equal(
		readFileSync(join(paths.home, ".skillreg/config.json"), "utf8"),
		'{"token":"fixture-access"}',
	);
	assert.equal(
		readFileSync(join(paths.home, ".skillreg/env/acme.json"), "utf8"),
		'{"fixture":"offline"}',
	);
	assert.equal(
		readFileSync(join(paths.home, ".claude/skills/original/SKILL.md"), "utf8"),
		"legacy original",
	);
	assert.equal(
		readFileSync(join(paths.recovery, "data/.claude/skills/original/SKILL.md"), "utf8"),
		"modified after snapshot",
	);
	assert.equal(readFileSync(join(paths.recovery, "data/.cursor/later.txt"), "utf8"), "new data");
	assert.equal(existsSync(join(paths.home, ".cursor")), false);
	assert.equal(readlinkSync(join(paths.home, ".agents/skills/external")), target);
	assert.equal(readFileSync(join(paths.external, "SKILL.md"), "utf8"), "external untouched");
	assert.equal(readFileSync(join(paths.home, "project/keep.txt"), "utf8"), "project untouched");
	if (process.platform !== "win32") {
		assert.equal(lstatSync(join(paths.home, ".skillreg/config.json")).mode & 0o777, 0o600);
	}
	success(run("restore", paths, ["--apply", "--recovery", paths.recovery]));
});

test("arguments, overlapping paths and aliases are rejected before profile mutation", (t) => {
	const paths = fixture();
	t.after(() => rmSync(paths.root, { recursive: true, force: true }));
	assert.notEqual(spawnSync(process.execPath, [script, "snapshot"]).status, 0);
	assert.notEqual(run("snapshot", { ...paths, home: "." }).status, 0);
	assert.notEqual(run("snapshot", { ...paths, backup: join(paths.home, "nested") }).status, 0);
	const alias = join(paths.root, "alias");
	symlinkSync(paths.home, alias, process.platform === "win32" ? "junction" : "dir");
	assert.notEqual(run("snapshot", { ...paths, backup: join(alias, "nested") }).status, 0);
	success(run("snapshot", paths));
	assert.notEqual(run("restore", paths, ["--apply"]).status, 0);
	assert.notEqual(
		run("restore", paths, ["--apply", "--recovery", join(paths.backup, "nested")]).status,
		0,
	);
	assert.equal(
		readFileSync(join(paths.home, ".skillreg/config.json"), "utf8"),
		'{"token":"fixture-access"}',
	);
});

test("a volume root is rejected before requiring or reading a backup", () => {
	const result = spawnSync(
		process.execPath,
		[script, "snapshot", "--home", parse(resolve(".")).root],
		{ encoding: "utf8" },
	);
	assert.notEqual(result.status, 0);
	assert.match(result.stderr, /A volume root cannot be used/);
});

test("missing, corrupted and path-traversal inventories cannot restore or write outside the profile", (t) => {
	const paths = fixture();
	t.after(() => rmSync(paths.root, { recursive: true, force: true }));
	assert.notEqual(run("verify", paths).status, 0);
	success(run("snapshot", paths));
	const inventoryPath = join(paths.backup, "inventory.json");
	const inventoryText = readFileSync(inventoryPath, "utf8");
	const inventory = JSON.parse(inventoryText);
	inventory.entries[0].path = "../outside";
	writeFileSync(inventoryPath, JSON.stringify(inventory));
	assert.notEqual(run("restore", paths, ["--apply", "--recovery", paths.recovery]).status, 0);
	assert.equal(existsSync(paths.recovery), false);
	writeFileSync(inventoryPath, inventoryText);
	writeFileSync(join(paths.backup, "data/.skillreg/config.json"), "corrupted");
	assert.notEqual(run("verify", paths).status, 0);
	assert.notEqual(run("restore", paths, ["--apply", "--recovery", paths.recovery]).status, 0);
	assert.equal(
		readFileSync(join(paths.home, ".skillreg/config.json"), "utf8"),
		'{"token":"fixture-access"}',
	);
});

test("an interrupted root swap resumes from the same verified journal without dropping either state", async (t) => {
	const paths = fixture();
	t.after(() => rmSync(paths.root, { recursive: true, force: true }));
	success(run("snapshot", paths));
	writeFileSync(join(paths.home, ".skillreg/config.json"), "post-migration");
	const { execute } = await import("../scripts/managed-skills-backup-restore.mjs");
	assert.throws(
		() =>
			execute(
				[
					"restore",
					"--home",
					paths.home,
					"--backup",
					paths.backup,
					"--apply",
					"--recovery",
					paths.recovery,
				],
				{
					afterRename: () => {
						throw new Error("simulated interruption");
					},
				},
			),
		/Restore profile root: simulated interruption/,
	);
	assert.equal(
		readFileSync(join(paths.recovery, "data/.skillreg/config.json"), "utf8"),
		"post-migration",
	);
	const moved = {
		...paths,
		home: join(paths.recovery, "moved"),
		backup: join(paths.root, "moved-check"),
	};
	success(run("snapshot", moved));
	const retained = JSON.parse(readFileSync(join(moved.backup, "inventory.json"), "utf8"));
	const before = JSON.parse(readFileSync(join(paths.recovery, "inventory.json"), "utf8"));
	assert.deepEqual(
		retained.entries,
		before.entries.filter(
			(entry: { path: string }) =>
				entry.path === ".skillreg" || entry.path.startsWith(".skillreg/"),
		),
		"The retained inventory changed before activation",
	);
	success(run("restore", paths, ["--apply", "--recovery", paths.recovery]));
	assert.equal(
		readFileSync(join(paths.home, ".skillreg/config.json"), "utf8"),
		'{"token":"fixture-access"}',
	);
	assert.equal(
		JSON.parse(readFileSync(join(paths.recovery, "restore-journal.json"), "utf8")).status,
		"complete",
	);
});

test("a changed profile after interruption is rejected without overwriting the new content", async (t) => {
	const paths = fixture();
	t.after(() => rmSync(paths.root, { recursive: true, force: true }));
	success(run("snapshot", paths));
	const { execute } = await import("../scripts/managed-skills-backup-restore.mjs");
	assert.throws(
		() =>
			execute(
				[
					"restore",
					"--home",
					paths.home,
					"--backup",
					paths.backup,
					"--apply",
					"--recovery",
					paths.recovery,
				],
				{
					afterRename: () => {
						throw new Error("simulated interruption");
					},
				},
			),
		/simulated interruption/,
	);
	writeFileSync(
		join(paths.home, ".claude/skills/original/SKILL.md"),
		"changed during interruption",
	);
	assert.notEqual(run("restore", paths, ["--apply", "--recovery", paths.recovery]).status, 0);
	assert.equal(
		readFileSync(join(paths.home, ".claude/skills/original/SKILL.md"), "utf8"),
		"changed during interruption",
	);
});

test("file and link roots retain their metadata without changing external contents", (t) => {
	const paths = fixture();
	t.after(() => rmSync(paths.root, { recursive: true, force: true }));
	symlinkSync(
		paths.external,
		join(paths.home, ".cursor"),
		process.platform === "win32" ? "junction" : "dir",
	);
	writeFileSync(join(paths.home, ".codex"), "root file before restore");
	success(run("snapshot", paths));
	const inventory = JSON.parse(readFileSync(join(paths.backup, "inventory.json"), "utf8"));
	assert.equal(
		inventory.entries.some((entry: { path: string }) => entry.path === ".cursor/SKILL.md"),
		false,
	);
	assert.equal(realpathSync(join(paths.backup, "data/.cursor")), realpathSync(paths.external));
	writeFileSync(join(paths.home, ".codex"), "root file after snapshot");
	if (process.platform !== "win32") chmodSync(join(paths.home, ".skillreg/config.json"), 0o640);
	success(run("restore", paths, ["--apply", "--recovery", paths.recovery]));
	assert.equal(lstatSync(join(paths.home, ".cursor")).isSymbolicLink(), true);
	assert.equal(realpathSync(join(paths.home, ".cursor")), realpathSync(paths.external));
	assert.equal(readFileSync(join(paths.external, "SKILL.md"), "utf8"), "external untouched");
	assert.equal(readFileSync(join(paths.home, ".codex"), "utf8"), "root file before restore");
	assert.equal(
		readFileSync(join(paths.recovery, "moved/.codex"), "utf8"),
		"root file after snapshot",
	);
	const checked = { ...paths, backup: join(paths.root, "checked") };
	success(run("snapshot", checked));
	const restored = JSON.parse(readFileSync(join(checked.backup, "inventory.json"), "utf8"));
	assert.deepEqual(restored.entries, inventory.entries);
});

test("hard links to files outside the profile are rejected before a snapshot is created", (t) => {
	const paths = fixture();
	t.after(() => rmSync(paths.root, { recursive: true, force: true }));
	linkSync(join(paths.external, "SKILL.md"), join(paths.home, ".skillreg/hard-link"));
	const result = run("snapshot", paths);
	assert.notEqual(result.status, 0);
	assert.match(result.stderr, /hard links/);
	assert.equal(existsSync(paths.backup), false);
});

test("ACL metadata survives restoration and a linked backup data directory is rejected", (t) => {
	const paths = fixture();
	t.after(() => rmSync(paths.root, { recursive: true, force: true }));
	const config = join(paths.home, ".skillreg/config.json");
	const modules = join(paths.root, "modules");
	if (process.platform === "win32") {
		const module = join(modules, "Microsoft.PowerShell.Security");
		mkdirSync(module, { recursive: true });
		writeFileSync(
			join(module, "Microsoft.PowerShell.Security.psd1"),
			"@{ ModuleVersion='1.0.0'; RootModule='Security.psm1'; FunctionsToExport=@('Get-Acl','Set-Acl') }",
		);
		writeFileSync(join(module, "Security.psm1"), "throw 'Incompatible inherited Security module'");
	}
	const acl =
		process.platform === "darwin"
			? spawnSync("/bin/chmod", ["+a", "everyone allow read", config], { encoding: "utf8" })
			: process.platform === "linux"
				? spawnSync("setfacl", ["-m", "u:65534:r", "--", config], { encoding: "utf8" })
				: spawnSync(
						"powershell.exe",
						[
							"-NoProfile",
							"-NonInteractive",
							"-Command",
							"$ErrorActionPreference='Stop'; $env:PSModulePath=$env:SKILLREG_TEST_MODULES + ';' + $env:PSModulePath; Import-Module (Join-Path $PSHOME 'Modules/Microsoft.PowerShell.Security/Microsoft.PowerShell.Security.psd1'); $paths=[System.IO.StreamReader]::new([Console]::OpenStandardInput(), [System.Text.Encoding]::UTF8).ReadToEnd() | ConvertFrom-Json; foreach ($path in $paths) { $acl=Get-Acl -LiteralPath $path; $acl.SetAccessRuleProtection($true,$true); $sid=[System.Security.Principal.SecurityIdentifier]::new('S-1-5-32-545'); $rule=[System.Security.AccessControl.FileSystemAccessRule]::new($sid,'Read','Allow'); $acl.AddAccessRule($rule); Set-Acl -LiteralPath $path -AclObject $acl }",
						],
						{
							input: JSON.stringify([config, join(paths.home, ".claude")]),
							encoding: "utf8",
							env: { ...process.env, SKILLREG_TEST_MODULES: modules },
						},
					);
	assert.equal(acl.status, 0, acl.stderr);
	success(run("snapshot", paths));
	const before = JSON.parse(readFileSync(join(paths.backup, "inventory.json"), "utf8"));
	if (process.platform === "win32") {
		const aclOf = (path: string): string =>
			before.entries.find((entry: { path: string }) => entry.path === path).acl;
		assert.match(aclOf(".claude"), /D:P/);
		assert.match(aclOf(".skillreg/config.json"), /D:P/);
		assert.doesNotMatch(aclOf(".skillreg"), /D:P/);
		assert.match(aclOf(".skillreg"), /\(A;[^;]*ID;/);
	}
	writeFileSync(config, "post-migration");
	success(run("restore", paths, ["--apply", "--recovery", paths.recovery]));
	const checked = { ...paths, backup: join(paths.root, "checked") };
	success(run("snapshot", checked));
	const after = JSON.parse(readFileSync(join(checked.backup, "inventory.json"), "utf8"));
	assert.deepEqual(after.entries, before.entries);
	rmSync(join(paths.backup, "data"), { recursive: true });
	symlinkSync(
		paths.home,
		join(paths.backup, "data"),
		process.platform === "win32" ? "junction" : "dir",
	);
	assert.notEqual(run("verify", paths).status, 0);
});

test("interruption after activation and before final journal rename is recoverable", async (t) => {
	const paths = fixture();
	t.after(() => rmSync(paths.root, { recursive: true, force: true }));
	success(run("snapshot", paths));
	const { execute } = await import("../scripts/managed-skills-backup-restore.mjs");
	let renamed = 0;
	const inventories: { actual: unknown; expected: unknown; label: string }[] = [];
	assert.throws(
		() =>
			execute(
				[
					"restore",
					"--home",
					paths.home,
					"--backup",
					paths.backup,
					"--apply",
					"--recovery",
					paths.recovery,
				],
				{
					afterRename: () => {
						if (++renamed !== 2) return;
						for (const [home, backup, expected] of [
							[paths.home, join(paths.root, "activated"), "all"],
							[join(paths.recovery, "moved"), join(paths.root, "moved-check"), ".skillreg"],
						]) {
							execute(["snapshot", "--home", home, "--backup", backup]);
							const actual = JSON.parse(readFileSync(join(backup, "inventory.json"), "utf8"));
							const original = JSON.parse(
								readFileSync(join(paths.backup, "inventory.json"), "utf8"),
							);
							inventories.push({
								actual: actual.entries,
								expected: original.entries.filter(
									(entry: { path: string }) =>
										expected === "all" ||
										entry.path === expected ||
										entry.path.startsWith(`${expected}/`),
								),
								label: expected === "all" ? "activated" : "retained",
							});
						}
						throw new Error("interruption after activation");
					},
				},
			),
		/interruption after activation/,
	);
	assert.equal(inventories.length, 2);
	for (const { actual, expected, label } of inventories) {
		assert.deepEqual(actual, expected, `The ${label} inventory changed during rename`);
	}
	success(run("restore", paths, ["--apply", "--recovery", paths.recovery]));
	const journal = join(paths.recovery, "restore-journal.json");
	const complete = JSON.parse(readFileSync(journal, "utf8"));
	writeFileSync(`${journal}.next`, JSON.stringify(complete));
	writeFileSync(journal, JSON.stringify({ ...complete, status: "pending" }));
	success(run("restore", paths, ["--apply", "--recovery", paths.recovery]));
	assert.equal(existsSync(`${journal}.next`), false);
	assert.equal(JSON.parse(readFileSync(journal, "utf8")).status, "complete");
});
