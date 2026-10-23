import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { dirname, normalize } from "node:path";
import test from "node:test";

function source(relativePath: string) {
	return readFileSync(new URL(`../${relativePath}`, import.meta.url), "utf8");
}

test("the README introduces SkillReg Desktop and where to get it", () => {
	const readme = source("README.md");

	assert.match(readme, /^# .*SkillReg/m);
	assert.ok(readme.indexOf("https://skillreg.dev/download") > 0);
	assert.ok(readme.includes("https://skillreg.dev)") || readme.includes("https://skillreg.dev\n"));
	assert.ok(readme.includes("npm i -g @skillreg/cli"));
	for (const agent of ["Claude Code", "Codex", "Cursor"]) {
		assert.ok(readme.includes(agent), `${agent} is not mentioned`);
	}
});

test("the README does not announce an unqualified build or distribution channel", () => {
	const readme = source("README.md");

	assert.doesNotMatch(readme, /\/releases/);
	assert.doesNotMatch(readme, /latest\.json/);
	assert.doesNotMatch(readme, /notari[zs]/i);
	assert.doesNotMatch(readme, /\bsigned\b/i);
	assert.doesNotMatch(readme, /\bv?\d+\.\d+\.\d+\b/);
});

test("README images exist and are not excluded by .gitignore", () => {
	const readme = source("README.md");
	const references = [
		...[...readme.matchAll(/!\[[^\]]*\]\(([^)\s]+)/g)].map((match) => match[1]),
		...[...readme.matchAll(/<img[^>]*\ssrc="([^"]+)"/g)].map((match) => match[1]),
	];

	for (const reference of references) {
		if (/^https?:\/\//.test(reference)) continue;
		const path = normalize(reference);
		assert.ok(existsSync(new URL(`../${path}`, import.meta.url)), `${reference} is missing`);
		// *.png, *.jpg and *.jpeg are ignored outside src-tauri/icons/ and would not be committed.
		const ignored = /\.(png|jpe?g)$/i.test(path) && dirname(path) !== "src-tauri/icons";
		assert.ok(!ignored, `${reference} would be ignored by .gitignore`);
	}
});
