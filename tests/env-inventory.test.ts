import assert from "node:assert/strict";
import test from "node:test";
import {
	type LegacyEnvVariableSummary,
	type OrgEnvVariable,
	buildEnvInventory,
	getEnvStatusForSkills,
} from "../src/lib/env-inventory.ts";
import type { LocalSkill } from "../src/lib/types.ts";

function localSkill(overrides: Partial<LocalSkill>): LocalSkill {
	return {
		name: "reviewer",
		version: "1.0.0",
		description: "Review code",
		tags: ["review"],
		path: "/Users/axel/.claude/skills/reviewer",
		agent: "claude",
		scope: "user",
		content_hash: "hash-a",
		modified_at: "1710000000",
		env_vars: [],
		...overrides,
	};
}

function orgVariable(overrides: Partial<OrgEnvVariable>): OrgEnvVariable {
	return {
		name: "OPENAI_API_KEY",
		configured: true,
		updatedAt: "2026-05-20T10:00:00.000Z",
		storage: "secure_store",
		...overrides,
	};
}

function legacyVariable(overrides: Partial<LegacyEnvVariableSummary>): LegacyEnvVariableSummary {
	return {
		name: "OPENAI_API_KEY",
		configured: true,
		skills: ["reviewer"],
		valueCount: 1,
		status: "migratable",
		...overrides,
	};
}

test("uses one org-level variable for every installed skill that declares it", () => {
	const inventory = buildEnvInventory({
		org: "kairia",
		installedSkills: [
			localSkill({
				name: "reviewer",
				env_vars: [
					{
						name: "OPENAI_API_KEY",
						description: "OpenAI API key",
						required: true,
					},
				],
			}),
			localSkill({
				name: "writer",
				agent: "codex",
				path: "/Users/axel/.codex/skills/writer",
				env_vars: [
					{
						name: "OPENAI_API_KEY",
						description: "OpenAI key for writing",
						required: true,
					},
					{
						name: "WRITER_MODEL",
						description: "Model override",
						required: false,
						default: "gpt-5.4",
					},
				],
			}),
		],
		orgEnvVars: [orgVariable({ name: "OPENAI_API_KEY" })],
		legacyVariables: [],
	});

	assert.equal(inventory.org, "kairia");
	assert.equal(inventory.variables.length, 2);

	const openai = inventory.variables.find((variable) => variable.name === "OPENAI_API_KEY");
	assert.ok(openai);
	assert.equal(openai.configured, true);
	assert.equal(openai.configuredSource, "org");
	assert.equal(openai.storageBackend, "secure_store");
	assert.equal(openai.secret, true);
	assert.equal(openai.requiredBy.length, 2);
	assert.deepEqual(
		openai.requiredBy.map((skill) => `${skill.skillName}:${skill.agent}:${skill.scope}`),
		["reviewer:claude:user", "writer:codex:user"],
	);

	const optional = inventory.variables.find((variable) => variable.name === "WRITER_MODEL");
	assert.ok(optional);
	assert.equal(optional.configured, false);
	assert.equal(optional.requiredBy.length, 0);
	assert.equal(optional.optionalFor.length, 1);
	assert.equal(optional.defaultValue, "gpt-5.4");
});

test("separates missing, configured, optional, and unused stored variables", () => {
	const inventory = buildEnvInventory({
		org: "kairia",
		installedSkills: [
			localSkill({
				name: "github-reviewer",
				env_vars: [
					{
						name: "GITHUB_TOKEN",
						description: "GitHub token",
						required: true,
					},
					{
						name: "GITHUB_OWNER",
						description: "Default owner",
						required: false,
					},
				],
			}),
		],
		orgEnvVars: [orgVariable({ name: "SLACK_BOT_TOKEN" })],
		legacyVariables: [],
	});

	assert.deepEqual(
		inventory.needsAttention.map((variable) => variable.name),
		["GITHUB_TOKEN"],
	);
	assert.deepEqual(
		inventory.configured.map((variable) => variable.name),
		[],
	);
	assert.deepEqual(
		inventory.optional.map((variable) => variable.name),
		["GITHUB_OWNER"],
	);
	assert.deepEqual(
		inventory.orphanedStoredVariables.map((variable) => variable.name),
		["SLACK_BOT_TOKEN"],
	);
	assert.equal(inventory.orphanedStoredVariables[0]?.referencedByInstalledSkills, false);
	assert.equal(inventory.orphanedStoredVariables[0]?.storage, "org");
});

test("keeps compatible legacy variables readable until they are migrated", () => {
	const reviewer = localSkill({
		name: "reviewer",
		env_vars: [{ name: "OPENAI_API_KEY", description: "", required: true }],
	});
	const writer = localSkill({
		name: "writer",
		agent: "codex",
		path: "/Users/axel/.codex/skills/writer",
		env_vars: [{ name: "OPENAI_API_KEY", description: "", required: true }],
	});
	const inventory = buildEnvInventory({
		org: "kairia",
		installedSkills: [reviewer, writer],
		orgEnvVars: [],
		legacyVariables: [legacyVariable({ name: "OPENAI_API_KEY", skills: ["reviewer", "writer"] })],
	});

	const openai = inventory.variables.find((variable) => variable.name === "OPENAI_API_KEY");
	assert.ok(openai);
	assert.equal(openai.configured, true);
	assert.equal(openai.configuredSource, "legacy");
	assert.deepEqual(openai.storedInSkills, ["reviewer", "writer"]);
	assert.deepEqual(getEnvStatusForSkills([reviewer, writer], inventory), {
		status: "ready",
		missingCount: 0,
	});
});

test("does not treat conflicting legacy values as configured", () => {
	const skill = localSkill({
		name: "github-reviewer",
		env_vars: [{ name: "GITHUB_TOKEN", description: "", required: true }],
	});
	const inventory = buildEnvInventory({
		org: "kairia",
		installedSkills: [skill],
		orgEnvVars: [],
		legacyVariables: [
			legacyVariable({
				name: "GITHUB_TOKEN",
				skills: ["github-reviewer", "release-notes"],
				valueCount: 2,
				status: "conflict",
			}),
		],
	});

	const token = inventory.variables.find((variable) => variable.name === "GITHUB_TOKEN");
	assert.ok(token);
	assert.equal(token.configured, false);
	assert.equal(token.configuredSource, "none");
	assert.deepEqual(
		inventory.needsAttention.map((variable) => variable.name),
		["GITHUB_TOKEN"],
	);
});

test("summarizes env readiness for installed skill groups", () => {
	const missingSkill = localSkill({
		name: "missing-env",
		env_vars: [{ name: "GITHUB_TOKEN", description: "", required: true }],
	});
	const readySkill = localSkill({
		name: "ready-env",
		env_vars: [{ name: "OPENAI_API_KEY", description: "", required: true }],
	});
	const optionalSkill = localSkill({
		name: "optional-env",
		env_vars: [{ name: "MODEL_NAME", description: "", required: false }],
	});
	const plainSkill = localSkill({ name: "plain" });

	const inventory = buildEnvInventory({
		org: "kairia",
		installedSkills: [missingSkill, readySkill, optionalSkill, plainSkill],
		orgEnvVars: [orgVariable({ name: "OPENAI_API_KEY" })],
		legacyVariables: [],
	});

	assert.deepEqual(getEnvStatusForSkills([readySkill], inventory), {
		status: "ready",
		missingCount: 0,
	});
	assert.deepEqual(getEnvStatusForSkills([missingSkill], inventory), {
		status: "missing",
		missingCount: 1,
	});
	assert.deepEqual(getEnvStatusForSkills([optionalSkill], inventory), {
		status: "optional",
		missingCount: 0,
	});
	assert.deepEqual(getEnvStatusForSkills([plainSkill], inventory), {
		status: "not_required",
		missingCount: 0,
	});
});
