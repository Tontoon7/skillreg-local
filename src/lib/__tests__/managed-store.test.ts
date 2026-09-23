import { mockInvokeCommand } from "@/test/setup";
import { beforeEach, describe, expect, it } from "vitest";
import { useConfigStore, useManagedSkillsStore } from "../store";
import type { ManagedInstallResult, ManagedOverview } from "../types";

const overview: ManagedOverview = {
	installations: [
		{
			installation: {
				installationId: "installation-id",
				consumerOrg: "acme",
				sourceOrg: "skillreg",
				skillId: "skill-id",
				skillName: "meeting-summary",
				origin: "registry",
				activeVersion: "1.0.0",
				status: "ready",
				installedAt: "1",
				lastCheckedAt: "1",
				lastUpdatedAt: "1",
				lastError: null,
			},
			bindings: [],
			missingEnvVars: [],
		},
	],
	agents: [],
	autoUpdateEnabled: true,
};

const installResult: ManagedInstallResult = {
	installation: overview.installations[0].installation,
	bindings: [],
	requiredEnvVars: [],
	warnings: [],
};

describe("managed skills store", () => {
	beforeEach(() => {
		useManagedSkillsStore.getState().reset();
		useConfigStore.setState({
			config: { org: "acme", setupDone: true },
			loading: false,
		});
	});

	it("preserves the last local model when refresh becomes offline", async () => {
		mockInvokeCommand("get_managed_overview", () => overview);
		await useManagedSkillsStore.getState().refresh();
		expect(useManagedSkillsStore.getState().model?.installedSkills).toBe(1);

		mockInvokeCommand("get_managed_overview", () => {
			throw new Error("offline");
		});
		await useManagedSkillsStore.getState().refresh();

		expect(useManagedSkillsStore.getState().model?.installedSkills).toBe(1);
		expect(useManagedSkillsStore.getState().model?.health).toBe("offline");
		expect(useManagedSkillsStore.getState().offline).toBe(true);
	});

	it("deduplicates concurrent installs of the same managed skill", async () => {
		let installs = 0;
		let resolveInstall: ((result: ManagedInstallResult) => void) | undefined;
		mockInvokeCommand("install_managed_skill", () => {
			installs += 1;
			return new Promise<ManagedInstallResult>((resolve) => {
				resolveInstall = resolve;
			});
		});
		mockInvokeCommand("get_managed_overview", () => overview);
		const params = {
			consumerOrg: "acme",
			sourceOrg: "skillreg",
			name: "meeting-summary",
		};

		const first = useManagedSkillsStore.getState().install(params);
		const second = useManagedSkillsStore.getState().install(params);

		expect(first).toBe(second);
		expect(installs).toBe(1);
		resolveInstall?.(installResult);
		await Promise.all([first, second]);
		expect(useManagedSkillsStore.getState().installingKeys).toEqual([]);
	});

	it("refreshes after uninstalling one managed installation by ID", async () => {
		let removed = false;
		let receivedInstallationId: string | undefined;
		mockInvokeCommand(
			"uninstall_managed_skill",
			({ installationId }: { installationId: string }) => {
				receivedInstallationId = installationId;
				removed = true;
				return {
					installationId,
					removed: true,
					alreadyRemoved: false,
					bindingsRemoved: 1,
					conflicts: 0,
					envValuesPreserved: true,
					cleanupDeferred: false,
				};
			},
		);
		mockInvokeCommand("get_managed_overview", () =>
			removed ? { ...overview, installations: [] } : overview,
		);

		await useManagedSkillsStore.getState().refresh();
		await expect(
			useManagedSkillsStore.getState().uninstall("installation-id"),
		).resolves.toMatchObject({ removed: true });

		expect(receivedInstallationId).toBe("installation-id");
		expect(useManagedSkillsStore.getState().model?.installedSkills).toBe(0);
	});

	it("repairs one managed installation by ID before refreshing", async () => {
		let receivedInstallationId: string | undefined;
		mockInvokeCommand("repair_managed_skill", ({ installationId }: { installationId: string }) => {
			receivedInstallationId = installationId;
			return {
				checked: 1,
				repaired: 1,
				conflicts: 0,
				failed: 0,
				modifiedContent: 0,
			};
		});
		mockInvokeCommand("get_managed_overview", () => overview);

		await expect(useManagedSkillsStore.getState().repair("installation-id")).resolves.toMatchObject(
			{ repaired: 1 },
		);

		expect(receivedInstallationId).toBe("installation-id");
		expect(useManagedSkillsStore.getState().overview).toEqual(overview);
	});

	it("switches the active organization through native reconciliation before updating UI state", async () => {
		const events: string[] = [];
		useConfigStore.setState({
			config: { org: "acme", setupDone: true, autoUpdateEnabled: false },
			loading: false,
		});
		mockInvokeCommand("switch_active_org", ({ org }: { org: string }) => {
			events.push(`switch:${org}`);
			return {
				previousOrg: "acme",
				activeOrg: org,
				bindingsRemoved: 1,
				bindingsCreated: 1,
				cachedSkillsReused: 1,
			};
		});
		mockInvokeCommand("read_config", () => {
			throw new Error("setOrg should trust the committed native switch");
		});

		await useConfigStore.getState().setOrg("globex");

		expect(events).toEqual(["switch:globex"]);
		expect(useConfigStore.getState().config.org).toBe("globex");
		expect(useConfigStore.getState().config.setupDone).toBe(true);
		expect(useConfigStore.getState().config.autoUpdateEnabled).toBe(false);
	});
});
