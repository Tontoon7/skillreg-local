import assert from "node:assert/strict";
import test from "node:test";
import {
	type CommandPublicationDraft,
	prepareCommandPublication,
	validateCommandPublication,
} from "../src/lib/command-publishing.ts";
import type { CommandVersion, RegistryCommandDetail } from "../src/lib/types.ts";

function version(overrides: Partial<CommandVersion> = {}): CommandVersion {
	return {
		version: "1.0.0",
		content: "  Review café ☕\n\n  Keep indentation.\n ",
		agentCompatibility: ["codex"],
		scope: "org",
		...overrides,
	};
}

function command(overrides: Partial<RegistryCommandDetail> = {}): RegistryCommandDetail {
	return {
		name: "review",
		description: "Review the diff",
		latestVersion: "1.0.0",
		totalVersions: 1,
		agentCompatibility: ["claude", "cursor"],
		scope: "project",
		versions: [version()],
		...overrides,
	};
}

function draft(overrides: Partial<CommandPublicationDraft> = {}): CommandPublicationDraft {
	return {
		version: "1.0.1",
		content: "Review the diff",
		agentCompatibility: ["codex"],
		scope: "org",
		...overrides,
	};
}

test("prefills the latest version by number, preserving its raw content and metadata", () => {
	const current = version();
	const prepared = prepareCommandPublication(
		command({ versions: [version({ version: "2.0.0", content: "Other" }), current] }),
	);
	assert.equal(prepared.currentVersion, "1.0.0");
	assert.deepEqual(prepared.existingVersions, ["2.0.0", "1.0.0"]);
	assert.deepEqual(prepared.draft, {
		version: "",
		content: current.content,
		agentCompatibility: ["codex"],
		scope: "org",
	});
	prepared.draft.agentCompatibility.push("claude");
	assert.deepEqual(current.agentCompatibility, ["codex"]);
});

test("falls back to the first version only when latestVersion is absent", () => {
	const prepared = prepareCommandPublication(command({ latestVersion: null }));
	assert.equal(prepared.currentVersion, "1.0.0");
	assert.throws(
		() => prepareCommandPublication(command({ latestVersion: "missing" })),
		/current version.*missing/i,
	);
	assert.throws(() => prepareCommandPublication(command({ versions: [] })), /current version/i);
});

test("uses command metadata for a command without versions", () => {
	assert.deepEqual(
		prepareCommandPublication(command({ latestVersion: null, versions: [] })).draft,
		{ version: "", content: "", agentCompatibility: ["claude", "cursor"], scope: "project" },
	);
});

test("falls back to command metadata only for empty agents or absent scope", () => {
	for (const scope of [undefined, null]) {
		const prepared = prepareCommandPublication(
			command({ versions: [version({ agentCompatibility: [], scope })] }),
		);
		assert.deepEqual(prepared.draft.agentCompatibility, ["claude", "cursor"]);
		assert.equal(prepared.draft.scope, "project");
	}
	const prepared = prepareCommandPublication(
		command({
			agentCompatibility: [],
			versions: [version({ agentCompatibility: [], scope: "unknown" })],
		}),
	);
	assert.deepEqual(prepared.draft.agentCompatibility, []);
	assert.equal(prepared.draft.scope, "unknown");
	assert.equal(validateCommandPublication(prepared.draft, []).input, null);
});

test("requires a new version and rejects already published versions after trimming", () => {
	for (const value of ["", "  ", " 1.0.0 "]) {
		const result = validateCommandPublication(draft({ version: value }), ["1.0.0"]);
		assert.ok(result.errors.version);
		assert.equal(result.input, null);
	}
});

test("uses the API version syntax including prerelease and build suffixes", () => {
	for (const value of [
		"1.0.1",
		"0.0.0",
		"01.02.03",
		"1.0.0-beta.1",
		"1.0.0+build_1",
		"1.0.0-rc.1+42",
	]) {
		assert.equal(validateCommandPublication(draft({ version: value }), []).input?.version, value);
	}
	for (const value of ["1", "1.0", "v1.0.0", "1.0.0-", "1.0.0+", "1.0.0-beta-1", "1.0.0 foo"]) {
		assert.ok(validateCommandPublication(draft({ version: value }), []).errors.version, value);
	}
});

test("normalizes the version but sends the raw content without adding or replacing text", () => {
	const content = " \nReview café 😀\n\n  Keep   internal whitespace\n ";
	assert.deepEqual(validateCommandPublication(draft({ version: " 1.0.1 ", content }), []).input, {
		version: "1.0.1",
		content,
		agentCompatibility: ["codex"],
		scope: "org",
	});
});

test("rejects blank content and enforces the limit after trimming", () => {
	for (const content of ["", " \n\t ", "é".repeat(20_001)]) {
		assert.ok(validateCommandPublication(draft({ content }), []).errors.content);
	}
	for (const content of ["x", ` \n${"é".repeat(20_000)}\n `]) {
		assert.ok(validateCommandPublication(draft({ content }), []).input);
	}
});

test("counts UTF-16 units like the API, including astral emoji", () => {
	assert.ok(validateCommandPublication(draft({ content: "😀".repeat(10_000) }), []).input);
	assert.ok(
		validateCommandPublication(draft({ content: `${"😀".repeat(10_000)}x` }), []).errors.content,
	);
});

test("uses JavaScript whitespace rules for the content length and empty check", () => {
	const content = `\uFEFF${"é".repeat(20_000)}\uFEFF`;
	assert.equal(validateCommandPublication(draft({ content }), []).input?.content, content);
	assert.ok(validateCommandPublication(draft({ content: "\uFEFF" }), []).errors.content);
	assert.equal(
		validateCommandPublication(draft({ content: "\u0085" }), []).input?.content,
		"\u0085",
	);
});

test("requires one to three distinct supported agents", () => {
	for (const agentCompatibility of [
		[],
		["other"],
		["codex", "codex"],
		["claude", "codex", "cursor", "other"],
	]) {
		assert.ok(
			validateCommandPublication(draft({ agentCompatibility }), []).errors.agentCompatibility,
		);
	}
	assert.ok(
		validateCommandPublication(draft({ agentCompatibility: ["claude", "codex", "cursor"] }), [])
			.input,
	);
});

test("accepts command scopes including org and rejects unknown or empty scopes", () => {
	for (const scope of ["org", "project", "user"]) {
		assert.equal(validateCommandPublication(draft({ scope }), []).input?.scope, scope);
	}
	for (const scope of ["", "unknown"]) {
		assert.ok(validateCommandPublication(draft({ scope }), []).errors.scope);
	}
});
