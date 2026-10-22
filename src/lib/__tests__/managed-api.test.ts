import { mockInvokeCommand } from "@/test/setup";
import { describe, expect, it } from "vitest";
import {
	checkManagedUpdates,
	installManagedSkill,
	previewLocalSkillsImport,
	previewManagedSkillsMigration,
	repairManagedSkill,
	repairManagedSkills,
	runLocalSkillsImport,
	runManagedSkillsMigration,
	runManagedUpdatesNow,
	setAutoUpdateEnabled,
	switchActiveOrg,
	uninstallManagedSkill,
} from "../api";

describe("managed skill API", () => {
	it("installs without exposing a version, agent, scope, or project path choice", async () => {
		mockInvokeCommand(
			"install_managed_skill",
			(args: { consumerOrg: string; sourceOrg?: string; name: string }) => {
				expect(args).toEqual({
					consumerOrg: "acme",
					sourceOrg: "skillreg",
					name: "meeting-summary",
				});
				expect(args).not.toHaveProperty("version");
				expect(args).not.toHaveProperty("agent");
				expect(args).not.toHaveProperty("scope");
				expect(args).not.toHaveProperty("projectDir");
				return {
					installation: {
						installationId: "installation-id",
						consumerOrg: "acme",
						sourceOrg: "skillreg",
						skillId: "skill-id",
						skillName: "meeting-summary",
						activeVersion: "1.4.2",
						status: "ready",
						installedAt: "1",
						lastCheckedAt: "1",
						lastUpdatedAt: "1",
						lastError: null,
					},
					bindings: [],
					requiredEnvVars: [],
					warnings: [],
				};
			},
		);

		const result = await installManagedSkill({
			consumerOrg: "acme",
			sourceOrg: "skillreg",
			name: "meeting-summary",
		});

		expect(result.installation).not.toHaveProperty("contentPath");
	});

	it("checks and applies managed updates independently from the global toggle", async () => {
		const summary = {
			checked: 1,
			available: 1,
			updated: 0,
			actionRequired: 0,
			failed: 0,
		};
		mockInvokeCommand("check_managed_updates", (args: { force: boolean }) => {
			expect(args).toEqual({ force: true });
			return summary;
		});
		mockInvokeCommand("run_managed_updates_now", () => ({
			...summary,
			updated: 1,
		}));
		mockInvokeCommand("set_auto_update_enabled", (args: { enabled: boolean }) => {
			expect(args).toEqual({ enabled: false });
		});

		await expect(checkManagedUpdates(true)).resolves.toEqual(summary);
		await expect(runManagedUpdatesNow()).resolves.toMatchObject({ updated: 1 });
		await expect(setAutoUpdateEnabled(false)).resolves.toBeUndefined();
	});

	it("previews, defers, and repairs migration through dedicated commands", async () => {
		const preview = {
			items: [],
			managedCandidates: 0,
			projectScopeUntouched: 0,
			modifiedUntouched: 0,
			missingRecords: 0,
			conflicts: 0,
			unsupportedAgents: 0,
			alreadyMigrated: 0,
		};
		mockInvokeCommand("preview_managed_skills_migration", () => preview);
		mockInvokeCommand("run_managed_skills_migration", (args: { confirm: boolean }) => ({
			confirmed: args.confirm,
			migrated: 0,
			skipped: 0,
			conflicts: 0,
			errors: 0,
			preview,
		}));
		mockInvokeCommand("repair_all_managed_skills", () => ({
			checked: 0,
			repaired: 0,
			conflicts: 0,
			failed: 0,
			modifiedContent: 0,
		}));

		await expect(previewManagedSkillsMigration()).resolves.toEqual(preview);
		await expect(runManagedSkillsMigration(false)).resolves.toMatchObject({
			confirmed: false,
		});
		await expect(repairManagedSkills()).resolves.toMatchObject({ repaired: 0 });
	});

	it("previews and confirms local import without exposing paths or content", async () => {
		const preview = {
			items: [
				{
					skillName: "review-helper",
					classification: "importable",
					agents: ["claude"],
				},
			],
			importable: 1,
			conflicts: 0,
			externalLinksUntouched: 1,
			alreadyManaged: 0,
			invalidUntouched: 0,
		};
		mockInvokeCommand("preview_local_skills_import", (args) => {
			expect(args).toBeUndefined();
			return preview;
		});
		mockInvokeCommand("run_local_skills_import", (args: { confirm: boolean }) => {
			expect(args).toEqual({ confirm: true });
			expect(args).not.toHaveProperty("path");
			expect(args).not.toHaveProperty("content");
			return {
				confirmed: true,
				imported: 1,
				skipped: 1,
				conflicts: 0,
				errors: 0,
				preview,
			};
		});

		await expect(previewLocalSkillsImport()).resolves.toEqual(preview);
		await expect(runLocalSkillsImport(true)).resolves.toMatchObject({
			confirmed: true,
			imported: 1,
		});
	});

	it("uses installation IDs for lifecycle actions and switches organization explicitly", async () => {
		mockInvokeCommand("uninstall_managed_skill", (args: { installationId: string }) => {
			expect(args).toEqual({ installationId: "installation-id" });
			return {
				installationId: "installation-id",
				removed: true,
				alreadyRemoved: false,
				bindingsRemoved: 2,
				conflicts: 0,
				envValuesPreserved: true,
				cleanupDeferred: false,
			};
		});
		mockInvokeCommand("repair_managed_skill", (args: { installationId: string }) => {
			expect(args).toEqual({ installationId: "installation-id" });
			return {
				checked: 1,
				repaired: 1,
				conflicts: 0,
				failed: 0,
				modifiedContent: 0,
			};
		});
		mockInvokeCommand("switch_active_org", (args: { org: string }) => {
			expect(args).toEqual({ org: "globex" });
			return {
				previousOrg: "acme",
				activeOrg: "globex",
				bindingsRemoved: 2,
				bindingsCreated: 1,
				cachedSkillsReused: 1,
			};
		});

		await expect(uninstallManagedSkill("installation-id")).resolves.toMatchObject({
			removed: true,
		});
		await expect(repairManagedSkill("installation-id")).resolves.toMatchObject({
			repaired: 1,
		});
		await expect(switchActiveOrg("globex")).resolves.toMatchObject({
			activeOrg: "globex",
		});
	});
});
