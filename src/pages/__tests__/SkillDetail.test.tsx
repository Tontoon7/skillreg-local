import { useAuthStore, useConfigStore, useManagedSkillsStore } from "@/lib/store";
import type { ManagedInstallResult, ManagedOverview, SkillDetail } from "@/lib/types";
import { SkillDetailPage } from "@/pages/SkillDetail";
import { mockInvokeCommand } from "@/test/setup";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter, Route, Routes } from "react-router";
import { beforeEach, describe, expect, it } from "vitest";

const detail: SkillDetail = {
	id: "skill-1",
	name: "meeting-summary",
	description: "Transforme vos notes en compte-rendu clair.",
	tags: ["réunions"],
	isPublic: false,
	latestVersion: "1.2.0",
	totalDownloads: 42,
	totalVersions: 3,
	createdAt: "2026-07-01T08:00:00Z",
	updatedAt: "2026-07-29T08:00:00Z",
	isDeprecated: false,
	deprecatedMessage: null,
	latestVersionData: {
		version: "1.2.0",
		tarballSize: 1024,
		sha256: "abc",
		skillMdContent:
			"---\nname: meeting-summary\ndescription: Résume une réunion\n---\n## Résultat\nUn compte-rendu structuré avec les décisions et prochaines étapes.",
		filesManifest: ["SKILL.md"],
		fileCount: 1,
		downloads: 42,
		validationLevel: "verified",
		validation: null,
		createdAt: "2026-07-29T08:00:00Z",
	},
	versions: [
		{
			id: "version-1",
			version: "1.2.0",
			tarballSize: 1024,
			sha256: "abc",
			downloads: 42,
			status: "approved",
			validationLevel: "verified",
			createdAt: "2026-07-29T08:00:00Z",
		},
	],
};

const overview: ManagedOverview = {
	installations: [],
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
	],
	autoUpdateEnabled: true,
};

const installResult: ManagedInstallResult = {
	installation: {
		installationId: "meeting-summary-id",
		consumerOrg: "acme",
		sourceOrg: "acme",
		skillId: "skill-1",
		skillName: "meeting-summary",
		origin: "registry",
		activeVersion: "1.2.0",
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

function renderDetail() {
	return render(
		<MemoryRouter initialEntries={["/catalog/meeting-summary"]}>
			<Routes>
				<Route path="/catalog/:name" element={<SkillDetailPage />} />
			</Routes>
		</MemoryRouter>,
	);
}

describe("employee skill detail", () => {
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
		mockInvokeCommand("get_skill", () => detail);
		mockInvokeCommand("get_managed_overview", () => overview);
		mockInvokeCommand("list_org_env_vars", () => []);
	});

	it("presents outcomes and detected assistants without technical install choices", async () => {
		let received: Record<string, unknown> | undefined;
		mockInvokeCommand("install_managed_skill", (args: Record<string, unknown>) => {
			received = args;
			return installResult;
		});
		renderDetail();

		expect(await screen.findByText("Ce que vous pouvez faire")).toBeInTheDocument();
		expect(screen.getByText("Exemples de demandes")).toBeInTheDocument();
		expect(screen.getByText("Données ou accès requis")).toBeInTheDocument();
		expect(screen.getByText(/Disponible dans Claude et Codex/)).toBeInTheDocument();
		expect(screen.queryByRole("checkbox")).not.toBeInTheDocument();
		expect(
			screen.queryByText(/scope|project folder|select version|files manifest/i),
		).not.toBeInTheDocument();

		await userEvent.click(screen.getByRole("button", { name: "Installer meeting-summary" }));
		expect(received).toEqual({
			consumerOrg: "acme",
			sourceOrg: "acme",
			name: "meeting-summary",
		});
	});

	it("keeps a skill installed but marked to configure when required access is deferred", async () => {
		mockInvokeCommand("install_managed_skill", () => ({
			...installResult,
			requiredEnvVars: [
				{
					name: "CRM_TOKEN",
					description: "Accès au CRM",
					required: true,
					secret: true,
				},
			],
		}));
		renderDetail();

		await userEvent.click(await screen.findByRole("button", { name: "Installer meeting-summary" }));
		expect(await screen.findByText("Configurer les accès")).toBeInTheDocument();
		await userEvent.click(screen.getByRole("button", { name: "Plus tard" }));

		expect(screen.getByText("À configurer")).toBeInTheDocument();
	});
});
