import { requiresWorkspaceSetup } from "@/App";
import type { SkillregConfig, WhoamiResponse } from "@/lib/types";
import { describe, expect, it } from "vitest";

function user(orgs: string[]): WhoamiResponse {
	return {
		user: {
			id: "user-1",
			email: "employee@example.com",
			name: "Employee",
		},
		orgs: orgs.map((slug) => ({ slug, name: slug, role: "member" })),
	};
}

describe("workspace routing", () => {
	it.each([
		[{ setupDone: false, org: "acme" }, ["acme"], true],
		[{ setupDone: true }, ["acme"], true],
		[{ setupDone: true, org: "former-company" }, ["acme"], true],
		[{ setupDone: true, org: "acme" }, [], true],
		[{ setupDone: true, org: "acme" }, ["acme", "globex"], false],
	] satisfies Array<[SkillregConfig, string[], boolean]>)(
		"evaluates setup state for config %o and organizations %o",
		(config, organizations, expected) => {
			expect(requiresWorkspaceSetup(config, user(organizations))).toBe(expected);
		},
	);
});
