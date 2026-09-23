import { useAuthStore, useConfigStore, useManagedSkillsStore } from "@/lib/store";
import type { ManagedInstallResult, ManagedOverview } from "@/lib/types";
import { Catalog } from "@/pages/Catalog";
import { renderWithRouter } from "@/test/render";
import { mockInvokeCommand } from "@/test/setup";
import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it } from "vitest";

const emptyOverview: ManagedOverview = {
	installations: [],
	agents: [
		{
			agent: "claude",
			state: "detected",
			detectedVersion: "1",
			requiresRestartAfterBinding: false,
		},
	],
	autoUpdateEnabled: true,
};

const installResult: ManagedInstallResult = {
	installation: {
		installationId: "meeting-summary-id",
		consumerOrg: "acme",
		sourceOrg: "acme",
		skillId: "meeting-summary-skill",
		skillName: "meeting-summary",
		origin: "registry",
		activeVersion: "1.0.0",
		status: "ready",
		installedAt: "2026-07-29T08:00:00Z",
		lastCheckedAt: null,
		lastUpdatedAt: null,
		lastError: null,
	},
	bindings: [],
	requiredEnvVars: [],
	warnings: [],
};

function configureCatalog(overview: ManagedOverview = emptyOverview) {
	mockInvokeCommand("list_skills", () => ({
		skills: [
			{
				id: "skill-1",
				name: "meeting-summary",
				description: "Prépare un compte-rendu clair à partir de vos notes.",
				tags: ["réunions"],
				isPublic: false,
				latestVersion: "1.0.0",
				totalDownloads: 42,
				totalVersions: 3,
				createdAt: "2026-07-01T08:00:00Z",
				updatedAt: "2026-07-29T08:00:00Z",
			},
		],
		pagination: { page: 1, limit: 200, total: 1, totalPages: 1 },
	}));
	mockInvokeCommand("get_catalog_policy", () => ({
		mode: "open",
		minimumValidationLevel: "scanned",
		allowFirstParty: true,
		canInstallFromCatalog: true,
	}));
	mockInvokeCommand("list_catalog_skills", () => ({
		skills: [],
		pagination: { page: 1, limit: 40, total: 0, totalPages: 0 },
	}));
	mockInvokeCommand("get_managed_overview", () => overview);
}

describe("employee catalog", () => {
	beforeEach(() => {
		useAuthStore.setState({
			authenticated: true,
			loading: false,
			user: {
				user: { id: "user-1", email: "employee@example.com", name: "Employee" },
				orgs: [{ slug: "acme", name: "Acme", role: "member" }],
			},
		});
		useConfigStore.setState({
			config: { org: "acme", setupDone: true },
			loading: false,
		});
		useManagedSkillsStore.getState().reset();
		configureCatalog();
	});

	it("installs an approved skill once without agent, scope, path, or version choices", async () => {
		let received: Record<string, unknown> | undefined;
		let resolveInstall: ((result: ManagedInstallResult) => void) | undefined;
		mockInvokeCommand("install_managed_skill", (args: Record<string, unknown>) => {
			received = args;
			return new Promise<ManagedInstallResult>((resolve) => {
				resolveInstall = resolve;
			});
		});
		renderWithRouter(<Catalog />, "/catalog");

		const install = await screen.findByRole("button", {
			name: "Installer meeting-summary",
		});
		expect(screen.queryByText(/agent|scope|version|dossier de projet/i)).not.toBeInTheDocument();
		await userEvent.click(install);
		await userEvent.click(install);

		expect(received).toEqual({
			consumerOrg: "acme",
			sourceOrg: "acme",
			name: "meeting-summary",
		});
		expect(install).toBeDisabled();
		resolveInstall?.(installResult);
		await waitFor(() => expect(install).not.toBeDisabled());
	});

	it("shows a managed installation state instead of a second install action", async () => {
		configureCatalog({
			...emptyOverview,
			installations: [
				{
					installation: installResult.installation,
					bindings: [],
					missingEnvVars: [],
				},
			],
		});
		renderWithRouter(<Catalog />, "/catalog");

		expect(await screen.findByText("Installée")).toBeInTheDocument();
		expect(
			screen.queryByRole("button", { name: "Installer meeting-summary" }),
		).not.toBeInTheDocument();
	});
});
