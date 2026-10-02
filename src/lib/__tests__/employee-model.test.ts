import { describe, expect, it } from "vitest";
import { buildEmployeeDashboardModel } from "../employee-model";
import type {
	ManagedOverview,
	ManagedOverviewInstallation,
	ManagedSkillBinding,
	ManagedSkillStatus,
} from "../types";

const binding = (
	agent: ManagedSkillBinding["agent"],
	status: ManagedSkillBinding["status"] = "ready",
): ManagedSkillBinding => ({
	agent,
	status,
	lastCheckedAt: "1",
	lastError: null,
	usageObservability: "unavailable",
});

const managed = (
	name: string,
	status: ManagedSkillStatus,
	bindings: ManagedSkillBinding[],
	missingEnvVars: ManagedOverviewInstallation["missingEnvVars"] = [],
): ManagedOverviewInstallation => ({
	installation: {
		installationId: `${name}-id`,
		consumerOrg: "acme",
		sourceOrg: "skillreg",
		skillId: `${name}-skill-id`,
		skillName: name,
		origin: "registry",
		activeVersion: "1.2.3",
		status,
		installedAt: "1",
		lastCheckedAt: "2",
		lastUpdatedAt: "1",
		lastError: null,
	},
	bindings,
	missingEnvVars,
});

const overview = (installations: ManagedOverviewInstallation[]): ManagedOverview => ({
	installations,
	agents: [
		{
			agent: "claude",
			state: "detected",
			detectedVersion: "1",
			requiresRestartAfterBinding: false,
		},
		{
			agent: "codex",
			state: "detected",
			detectedVersion: "1",
			requiresRestartAfterBinding: false,
		},
		{
			agent: "cursor",
			state: "detected",
			detectedVersion: "1",
			requiresRestartAfterBinding: false,
		},
	],
	autoUpdateEnabled: false,
});

describe("employee dashboard model", () => {
	it("deduplicates three bindings into one skill row and keeps unknown usage explicit", () => {
		const model = buildEmployeeDashboardModel(
			overview([
				managed("meeting-summary", "ready", [
					binding("claude"),
					binding("codex"),
					binding("cursor"),
				]),
			]),
		);

		expect(model.installedSkills).toBe(1);
		expect(model.connectedAgents).toBe(3);
		expect(model.skills).toHaveLength(1);
		expect(model.skills[0].availableIn).toEqual(["claude", "codex", "cursor"]);
		expect(model.skills[0].usageLabel).toBe("Usage inconnu");
	});

	it("creates one contextual action per installation and sorts action, update, then ready", () => {
		const model = buildEmployeeDashboardModel(
			overview([
				managed("ready", "ready", [binding("claude")]),
				managed("update", "update_available", [binding("claude")]),
				managed("conflict", "action_required", [
					binding("claude", "conflict"),
					binding("codex", "conflict"),
				]),
				managed(
					"configure",
					"ready",
					[binding("claude")],
					[
						{
							name: "CRM_TOKEN",
							description: "CRM access",
							required: true,
							secret: true,
						},
					],
				),
			]),
		);

		expect(model.updatesAvailable).toBe(1);
		expect(model.requiredActions.filter((action) => action.kind === "repair")).toHaveLength(1);
		expect(model.requiredActions.filter((action) => action.kind === "configure")).toHaveLength(1);
		expect(model.skills.map((skill) => skill.name)).toEqual([
			"configure",
			"conflict",
			"update",
			"ready",
		]);
		expect(model.skills.find((skill) => skill.name === "configure")?.statusLabel).toBe(
			"À configurer",
		);
	});

	it("keeps updates visible while automatic updates are off and models offline explicitly", () => {
		const model = buildEmployeeDashboardModel(
			overview([managed("update", "update_available", [binding("claude")])]),
			{ offline: true },
		);

		expect(model.autoUpdateEnabled).toBe(false);
		expect(model.updatesAvailable).toBe(1);
		expect(model.health).toBe("offline");
	});

	it("never adds legacy technical choices to the employee model", () => {
		const serialized = JSON.stringify(
			buildEmployeeDashboardModel(overview([managed("ready", "ready", [binding("claude")])])),
		);

		expect(serialized).not.toMatch(/scope|projectDir|selectedVersion/);
	});
});
