import assert from "node:assert/strict";
import test from "node:test";
import { canCompleteDesktopSetup, resolveDesktopSetupState } from "../src/lib/setup-state.ts";

const organizations = [
	{ slug: "acme", name: "Acme", role: "owner" },
	{ slug: "beta", name: "Beta", role: "member" },
];

test("requires a workspace when the authenticated account has no organizations", () => {
	assert.equal(resolveDesktopSetupState([], ""), "workspace-required");
	assert.equal(canCompleteDesktopSetup([], ""), false);
});

test("requires an organization selection when none is selected", () => {
	assert.equal(resolveDesktopSetupState(organizations, ""), "organization-required");
	assert.equal(canCompleteDesktopSetup(organizations, ""), false);
});

test("allows completion only for an organization returned by whoami", () => {
	assert.equal(resolveDesktopSetupState(organizations, "acme"), "ready");
	assert.equal(canCompleteDesktopSetup(organizations, "acme"), true);
	assert.equal(canCompleteDesktopSetup(organizations, "removed-org"), false);
});
