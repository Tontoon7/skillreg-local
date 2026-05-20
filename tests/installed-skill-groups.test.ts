import assert from "node:assert/strict";
import test from "node:test";
import { groupInstalledSkills } from "../src/lib/installed-skill-groups.ts";
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
		...overrides,
	};
}

test("groups the same installed skill across agents and scopes", () => {
	const groups = groupInstalledSkills([
		localSkill({ agent: "claude", scope: "user", path: "/Users/axel/.claude/skills/reviewer" }),
		localSkill({ agent: "codex", scope: "user", path: "/Users/axel/.codex/skills/reviewer" }),
		localSkill({
			name: "writer",
			version: "2.0.0",
			path: "/Users/axel/.cursor/skills/writer",
			agent: "cursor",
			scope: "user",
		}),
	]);

	assert.equal(groups.length, 2);
	assert.equal(groups[0]?.name, "reviewer");
	assert.deepEqual(
		groups[0]?.installations.map((installation) => `${installation.agent}:${installation.scope}`),
		["claude:user", "codex:user"],
	);
	assert.equal(groups[1]?.name, "writer");
});
