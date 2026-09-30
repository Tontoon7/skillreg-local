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
import { syncBuiltinESMExports } from "node:module";
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
	if (process.platform === "win32") {
		// The checkout's ACL may contain no inherited ACEs; give this fixture its own parent DACL.
		const acl = spawnSync(
			"powershell.exe",
			[
				"-NoProfile",
				"-NonInteractive",
				"-Command",
				`$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSHOME 'Modules/Microsoft.PowerShell.Security/Microsoft.PowerShell.Security.psd1')
$path = [System.IO.StreamReader]::new([Console]::OpenStandardInput(), [System.Text.Encoding]::UTF8).ReadToEnd()
$acl = [System.Security.AccessControl.DirectorySecurity]::new()
$acl.SetAccessRuleProtection($true, $false)
$sid = [System.Security.Principal.WindowsIdentity]::GetCurrent().User
$rule = [System.Security.AccessControl.FileSystemAccessRule]::new($sid, 'FullControl', 'ContainerInherit,ObjectInherit', 'None', 'Allow')
$acl.AddAccessRule($rule)
Set-Acl -LiteralPath $path -AclObject $acl`,
			],
			{ input: home, encoding: "utf8" },
		);
		assert.equal(acl.status, 0, acl.stderr);
	}
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

test("Windows rename-only metadata normalizes empty, scalar and array success output", async (t) => {
	const { winMetadata } = await import("../scripts/managed-skills-backup-restore.mjs");
	let stdout = "";
	t.mock.method(process.getBuiltinModule("child_process"), "spawnSync", () => ({
		status: 0,
		stdout,
	}));
	syncBuiltinESMExports();
	t.after(() => {
		t.mock.restoreAll();
		syncBuiltinESMExports();
	});
	const requests = [{ path: "source", destination: "destination" }];
	assert.deepEqual(winMetadata(requests), []);
	stdout = "\r\n";
	assert.deepEqual(winMetadata(requests), []);
	stdout = '"renamed"\r\n';
	assert.deepEqual(winMetadata(requests), ["renamed"]);
	stdout = '["renamed"]\r\n';
	assert.deepEqual(winMetadata(requests), ["renamed"]);
});

test("Windows metadata normalizes a single ACL and preserves arrays in request order", async (t) => {
	const { winMetadata } = await import("../scripts/managed-skills-backup-restore.mjs");
	const acl = "O:SYG:SYD:P(A;;FA;;;SY)";
	let stdout = JSON.stringify(acl);
	t.mock.method(process.getBuiltinModule("child_process"), "spawnSync", () => ({
		status: 0,
		stdout,
	}));
	syncBuiltinESMExports();
	t.after(() => {
		t.mock.restoreAll();
		syncBuiltinESMExports();
	});
	assert.deepEqual(winMetadata([{ path: "source" }]), [acl]);
	stdout = JSON.stringify([acl]);
	assert.deepEqual(winMetadata([{ path: "source" }]), [acl]);
	stdout = JSON.stringify(["renamed", acl]);
	assert.deepEqual(
		winMetadata([{ path: "source", destination: "destination" }, { path: "destination" }]),
		["renamed", acl],
	);
});

test("Windows metadata still rejects native failures, malformed JSON and missing ACL output", async (t) => {
	const { winMetadata } = await import("../scripts/managed-skills-backup-restore.mjs");
	let status = 1;
	let stdout = "";
	t.mock.method(process.getBuiltinModule("child_process"), "spawnSync", () => ({ status, stdout }));
	syncBuiltinESMExports();
	t.after(() => {
		t.mock.restoreAll();
		syncBuiltinESMExports();
	});
	const rename = { path: "source", destination: "destination" };
	assert.throws(() => winMetadata([rename]), /Required native metadata operation failed/);
	status = 0;
	assert.throws(() => winMetadata([{ path: "source" }]), SyntaxError);
	assert.throws(() => winMetadata([{ path: "source", acl: "private" }]), SyntaxError);
	assert.throws(() => winMetadata([rename, { path: "source" }]), SyntaxError);
	stdout = "invalid JSON";
	assert.throws(() => winMetadata([rename]), SyntaxError);
});

test("Windows metadata reports only validated native error codes", async (t) => {
	const { winMetadata } = await import("../scripts/managed-skills-backup-restore.mjs");
	let stdout = '{"nativeError":123}';
	t.mock.method(process.getBuiltinModule("child_process"), "spawnSync", () => ({
		status: 1,
		stdout,
		stderr: "private profile path and descriptor",
	}));
	syncBuiltinESMExports();
	t.after(() => {
		t.mock.restoreAll();
		syncBuiltinESMExports();
	});
	const requests = [{ path: "source", destination: "destination" }];
	assert.throws(() => winMetadata(requests), {
		message: "Required native metadata operation failed: powershell.exe (Win32 123)",
	});
	for (const output of [
		'{"nativeError":"private profile path"}',
		'{"nativeError":4294967296}',
		'{"nativeError":0}',
		'{"nativeError":123,"path":"private profile path"}',
		"private native error",
	]) {
		stdout = output;
		assert.throws(() => winMetadata(requests), {
			message: "Required native metadata operation failed: powershell.exe",
		});
	}
});

test("Windows protected DACL writes never fall through to the kernel security setter", async (t) => {
	const { winMetadata } = await import("../scripts/managed-skills-backup-restore.mjs");
	let nativeSource = "";
	t.mock.method(
		process.getBuiltinModule("child_process"),
		"spawnSync",
		(_command: string, args: string[]) => {
			nativeSource = Buffer.from(args[args.length - 1], "base64").toString("utf16le");
			return { status: 0, stdout: '["D:P(A;;FA;;;SY)"]' };
		},
	);
	syncBuiltinESMExports();
	t.after(() => {
		t.mock.restoreAll();
		syncBuiltinESMExports();
	});
	winMetadata([{ path: "fixture", acl: "private" }]);
	const branches = nativeSource.match(
		/if \(\(control & 0x1000\) != 0\) \{([^{}]+)\} else \{([^{}]+)\}\s*\} finally \{ LocalFree\(next\); \}/,
	);
	assert.ok(branches, "Protected and inherited DACL writes must use exclusive branches");
	assert.match(branches[1], /SetSecurityInfo\(handle, 1, flags \| 0x80000000u,/);
	assert.doesNotMatch(branches[1], /(?:SetKernelObjectSecurity|SetSecurityDescriptorControl)\(/);
	assert.match(
		branches[2],
		/if \(\(control & 0x0400\) != 0 && !SetSecurityDescriptorControl\(next, 0x0100, 0x0100\)\)/,
	);
	assert.match(branches[2], /SetKernelObjectSecurity\(handle, flags, next\)/);
	assert.doesNotMatch(branches[2], /SetSecurityInfo\(/);
});

if (process.platform === "win32") {
	test("Windows ACL cloning preserves auto-inheritance without copying the destination parent ACL", async (t) => {
		const paths = fixture();
		t.after(() => rmSync(paths.root, { recursive: true, force: true }));
		const { winMetadata } = await import("../scripts/managed-skills-backup-restore.mjs");
		const parent = join(paths.external, "different parent été");
		mkdirSync(parent);
		const [privateAcl] = winMetadata([{ path: parent, acl: "private" }]);
		winMetadata([{ path: parent, acl: `${privateAcl}(A;OICI;FR;;;BU)` }]);
		const sources = [
			join(paths.home, ".skillreg"),
			join(paths.home, ".skillreg/config.json"),
			join(paths.home, ".agents/skills/external"),
		];
		const destinations = [
			join(parent, "directory"),
			join(parent, "file.json"),
			join(parent, "link"),
		];
		mkdirSync(destinations[0]);
		writeFileSync(destinations[1], "copy");
		symlinkSync(paths.external, destinations[2], "junction");
		const before: string[] = winMetadata(sources.map((path) => ({ path })));
		for (const acl of before) {
			assert.match(acl, /D:AI\(/);
			assert.match(acl, /\(A;[^;]*ID;/);
			assert.doesNotMatch(acl, /;;;BU\)/);
		}
		const initial: string[] = winMetadata(destinations.map((path) => ({ path })));
		for (const acl of initial) assert.match(acl, /;;;BU\)/);
		assert.deepEqual(
			winMetadata(destinations.map((path, index) => ({ path, acl: before[index] }))),
			before,
		);
		assert.deepEqual(winMetadata(destinations.map((path) => ({ path }))), before);
		assert.equal(readFileSync(join(paths.external, "SKILL.md"), "utf8"), "external untouched");
	});

	test("Windows metadata applies and clones protected DACLs without parent inheritance", async (t) => {
		const paths = fixture();
		t.after(() => rmSync(paths.root, { recursive: true, force: true }));
		const { winMetadata } = await import("../scripts/managed-skills-backup-restore.mjs");
		const source = join(paths.home, ".skillreg/config.json");
		const destination = join(paths.external, "copy.json");
		const [before] = winMetadata([{ path: source, acl: "private" }]);
		assert.match(before, /D:P/);
		assert.doesNotMatch(before, /\(A;[^;]*ID;/);
		writeFileSync(destination, "copy");
		assert.match(before, /D:P(?:AI)?\(/);
		// Replace the complete DACL flags: Get-Acl can already return PAI.
		const variants = ["D:P", "D:PAI"].map((flags) => before.replace(/D:P(?:AI)?(?=\()/, flags));
		assert.notEqual(variants[0], variants[1]);
		for (const acl of variants) {
			// The file API marks the current inheritance model with AI, even for a protected DACL.
			// Normalize only this bookkeeping flag; owner, group, ACEs and protection must match exactly.
			const expected = acl.replace(/D:P(?:AI)?(?=\()/, "D:PAI");
			assert.deepEqual(winMetadata([{ path: source, acl }]), [expected]);
			assert.deepEqual(winMetadata([{ path: source }]), [expected]);
			assert.deepEqual(winMetadata([{ path: destination, acl }]), [expected]);
			assert.deepEqual(winMetadata([{ path: destination }]), [expected]);
			assert.doesNotMatch(expected, /\(A;[^;]*ID;/);
		}
		success(run("snapshot", paths));
		success(run("verify", paths));
	});

	test("Windows ACL cloning restores an explicit primary group", async (t) => {
		const paths = fixture();
		t.after(() => rmSync(paths.root, { recursive: true, force: true }));
		const { winMetadata } = await import("../scripts/managed-skills-backup-restore.mjs");
		const source = join(paths.home, ".skillreg/config.json");
		const before = winMetadata([{ path: source }])[0];
		const group = before.includes("G:BU") ? "AU" : "BU";
		const replacement = before.replace(/G:[^:]+(?=D:)/, `G:${group}`);
		assert.notEqual(replacement, before);
		const [changed] = winMetadata([{ path: source, acl: replacement }]);
		assert.equal(changed, replacement);
		success(run("snapshot", paths));
		success(run("verify", paths));
	});

	test("Windows native rename preserves inherited ACLs under a private parent and back", async (t) => {
		const paths = fixture();
		t.after(() => rmSync(paths.root, { recursive: true, force: true }));
		const { winMetadata } = await import("../scripts/managed-skills-backup-restore.mjs");
		const source = join(paths.home, ".skillreg");
		const parent = join(paths.root, `private parent été ${"long-path-".repeat(5)}`);
		const destination = join(parent, ".skillreg");
		mkdirSync(parent);
		winMetadata([{ path: parent, acl: "private" }]);
		const before = winMetadata([{ path: source }]);
		assert.match(before[0], /\(A;[^;]*ID;/);
		assert.doesNotMatch(before[0], /D:P/);
		mkdirSync(destination);
		writeFileSync(join(destination, "keep.txt"), "existing destination");
		assert.throws(
			() => winMetadata([{ path: source, destination }]),
			/Required native metadata operation failed: powershell.exe \(Win32 \d+\)/,
		);
		assert.equal(readFileSync(join(destination, "keep.txt"), "utf8"), "existing destination");
		assert.deepEqual(winMetadata([{ path: source }]), before);
		rmSync(destination, { recursive: true });
		assert.deepEqual(winMetadata([{ path: source, destination }]), ["renamed"]);
		assert.equal(existsSync(source), false);
		assert.deepEqual(winMetadata([{ path: destination }]), before);
		assert.equal(
			readFileSync(join(destination, "config.json"), "utf8"),
			'{"token":"fixture-access"}',
		);
		assert.deepEqual(winMetadata([{ path: destination, destination: source }]), ["renamed"]);
		assert.equal(existsSync(destination), false);
		assert.deepEqual(winMetadata([{ path: source }]), before);
	});
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
	const corrupted = run("verify", paths);
	assert.notEqual(corrupted.status, 0);
	assert.match(
		corrupted.stderr,
		/Backup inventory is incomplete or corrupted \(entry \d+: sha256, size\)/,
	);
	assert.equal(corrupted.stderr.includes(paths.backup), false);
	assert.equal(corrupted.stderr.includes("config.json"), false);
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
