import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));
const fixtures = [
	["darwin-aarch64", "SkillReg.app.tar.gz", "SkillReg_aarch64.app.tar.gz"],
	["darwin-x86_64", "SkillReg.app.tar.gz", "SkillReg_x64.app.tar.gz"],
	["linux-x86_64", "SkillReg_0.3.25_amd64.AppImage", "SkillReg_0.3.25_amd64.AppImage"],
	["windows-x86_64", "SkillReg_0.3.25_x64-setup.exe", "SkillReg_0.3.25_x64-setup.exe"],
];

function fixture(t: test.TestContext) {
	const directory = mkdtempSync(join(root, "node_modules/.updater-test-"));
	t.after(() => rmSync(directory, { recursive: true, force: true }));
	const artifacts = join(directory, "artifacts");
	const output = join(directory, "updater");
	for (const [platform, name] of fixtures) {
		const folder = join(artifacts, `build-${platform}`);
		mkdirSync(folder, { recursive: true });
		writeFileSync(join(folder, name), `artifact-${platform}`);
		writeFileSync(join(folder, `${name}.sig`), `signature-${platform}\n`);
	}
	// NSIS is the chosen Windows updater even when an MSI is also packaged.
	writeFileSync(join(artifacts, "build-windows-x86_64", "SkillReg.msi"), "msi");
	writeFileSync(join(artifacts, "build-windows-x86_64", "SkillReg.msi.sig"), "msi-signature");
	const run = () =>
		spawnSync(
			process.execPath,
			[
				join(root, "scripts/generate-updater-manifest.mjs"),
				"--artifacts",
				artifacts,
				"--output",
				output,
				"--version",
				"0.3.25",
				"--base-url",
				"https://github.com/Tontoon7/skillreg-local/releases/download/v0.3.25",
				"--pub-date",
				"2026-09-24T12:00:00Z",
			],
			{ encoding: "utf8" },
		);
	return { artifacts, output, run };
}

test("native updater artifacts and their exact signatures are prepared for all platforms", (t) => {
	const { output, run } = fixture(t);
	const result = run();
	assert.equal(result.status, 0, result.stderr);
	const manifest = JSON.parse(readFileSync(join(output, "latest.json"), "utf8"));
	assert.equal(manifest.version, "0.3.25");
	assert.equal(manifest.pub_date, "2026-09-24T12:00:00Z");
	assert.equal(Object.keys(manifest.platforms).length, 4);
	for (const [platform, , name] of fixtures) {
		assert.equal(manifest.platforms[platform].signature, `signature-${platform}`);
		assert.ok(manifest.platforms[platform].url.endsWith(`/${name}`));
		assert.equal(readFileSync(join(output, name), "utf8"), `artifact-${platform}`);
		assert.equal(readFileSync(join(output, `${name}.sig`), "utf8"), `signature-${platform}\n`);
	}
});

for (const [platform, name] of fixtures) {
	for (const missing of [name, `${name}.sig`]) {
		test(`missing ${platform}/${missing} refuses a partial updater`, (t) => {
			const { artifacts, output, run } = fixture(t);
			rmSync(join(artifacts, `build-${platform}`, missing));
			const result = run();
			assert.notEqual(result.status, 0);
			assert.match(result.stderr, new RegExp(platform));
			assert.equal(existsSync(output), false);
		});
	}
}

test("a missing platform, empty signature or ambiguous artifact fails before output", (t) => {
	const { artifacts, output, run } = fixture(t);
	const folder = join(artifacts, "build-linux-x86_64");
	const signature = join(folder, "SkillReg_0.3.25_amd64.AppImage.sig");
	writeFileSync(signature, "\n");
	assert.notEqual(run().status, 0);
	assert.equal(existsSync(output), false);
	writeFileSync(signature, "signature-linux-x86_64");
	writeFileSync(join(folder, "stale.AppImage"), "stale");
	assert.notEqual(run().status, 0);
	assert.equal(existsSync(output), false);
	rmSync(folder, { recursive: true });
	assert.notEqual(run().status, 0);
	assert.equal(existsSync(output), false);
});
