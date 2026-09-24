import { useAuthStore, useConfigStore, useManagedSkillsStore } from "@/lib/store";
import type {
	ManagedOverview,
	ManagedOverviewInstallation,
	ManagedSkillBinding,
	ManagedSkillStatus,
} from "@/lib/types";
import { Dashboard } from "@/pages/Dashboard";
import { renderWithRouter } from "@/test/render";
import { mockInvokeCommand } from "@/test/setup";
import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it } from "vitest";

function binding(status: ManagedSkillBinding["status"] = "ready"): ManagedSkillBinding {
	return {
		agent: "claude",
		status,
		lastCheckedAt: "2026-07-29T08:00:00Z",
		lastError: null,
		usageObservability: "unavailable",
	};
}

function installation(
	name: string,
	status: ManagedSkillStatus = "ready",
	options: {
		bindingStatus?: ManagedSkillBinding["status"];
		missingEnv?: boolean;
	} = {},
): ManagedOverviewInstallation {
	return {
		installation: {
			installationId: `${name}-id`,
			consumerOrg: "acme",
			sourceOrg: "acme",
			skillId: `${name}-skill`,
			skillName: name,
			origin: "registry",
			activeVersion: "4.2.0",
			status,
			installedAt: "2026-07-28T08:00:00Z",
			lastCheckedAt: "2026-07-29T08:00:00Z",
			lastUpdatedAt: "2026-07-28T08:00:00Z",
			lastError: null,
		},
		bindings: [binding(options.bindingStatus)],
		missingEnvVars: options.missingEnv
			? [
					{
						name: "CRM_TOKEN",
						description: "Accès au CRM",
						required: true,
						secret: true,
					},
				]
			: [],
	};
}

function overview(
	installations: ManagedOverviewInstallation[] = [],
	options: { autoUpdateEnabled?: boolean; connectedAgents?: number } = {},
): ManagedOverview {
	const connectedAgents = options.connectedAgents ?? 1;
	return {
		installations,
		agents:
			connectedAgents === 0
				? []
				: [
						{
							agent: "claude",
							state: "detected",
							detectedVersion: "1",
							requiresRestartAfterBinding: false,
						},
					],
		autoUpdateEnabled: options.autoUpdateEnabled ?? true,
	};
}

function configureDashboard(current: ManagedOverview) {
	let overviewValue = current;
	mockInvokeCommand("get_managed_overview", () => overviewValue);
	mockInvokeCommand("read_config", () => ({
		org: "acme",
		setupDone: true,
		autoUpdateEnabled: overviewValue.autoUpdateEnabled,
	}));
	mockInvokeCommand("write_config", () => undefined);
	mockInvokeCommand("set_auto_update_enabled", ({ enabled }: { enabled: boolean }) => {
		overviewValue = { ...overviewValue, autoUpdateEnabled: enabled };
	});
	mockInvokeCommand("check_managed_updates", () => ({
		checked: overviewValue.installations.length,
		available: 0,
		updated: 0,
		actionRequired: 0,
		failed: 0,
	}));
	mockInvokeCommand("run_managed_updates_now", () => ({
		checked: overviewValue.installations.length,
		available: 0,
		updated: 0,
		actionRequired: 0,
		failed: 0,
	}));
	mockInvokeCommand("repair_all_managed_skills", () => ({
		checked: overviewValue.installations.length,
		repaired: 0,
		conflicts: 0,
		failed: 0,
		modifiedContent: 0,
	}));
}

describe("employee dashboard", () => {
	beforeEach(() => {
		useAuthStore.setState({
			authenticated: true,
			loading: false,
			user: {
				user: {
					id: "user-1",
					email: "employee@example.com",
					name: "Axel",
				},
				orgs: [{ slug: "acme", name: "Acme", role: "member" }],
			},
		});
		useConfigStore.setState({
			config: { org: "acme", setupDone: true, autoUpdateEnabled: true },
			loading: false,
		});
		useManagedSkillsStore.getState().reset();
	});

	it("puts health and the global automatic-update control first", async () => {
		configureDashboard(overview([installation("meeting-summary")]));
		const { container } = renderWithRouter(<Dashboard />);

		expect(await screen.findByText("Vos assistants sont prêts")).toBeInTheDocument();
		const toggle = screen.getByRole("switch", {
			name: "Maintenir mes skills à jour",
		});
		expect(toggle).toBeChecked();
		expect(screen.getByText("Activées")).toBeInTheDocument();
		expect(screen.getByRole("button", { name: "Vérifier maintenant" })).toBeInTheDocument();
		expect(screen.queryByText(/suggestions de nettoyage/i)).not.toBeInTheDocument();
		expect(container.firstElementChild).toHaveClass("min-w-0");
		expect(toggle.closest("section")).toHaveClass("min-w-0");
		expect(screen.getByRole("button", { name: "Vérifier maintenant" }).parentElement).toHaveClass(
			"w-full",
		);
	});

	it("reconciles managed bindings when the dashboard opens", async () => {
		configureDashboard(overview([installation("meeting-summary")]));
		let repairs = 0;
		mockInvokeCommand("repair_all_managed_skills", () => {
			repairs += 1;
			return {
				checked: 1,
				repaired: 1,
				conflicts: 0,
				failed: 0,
				modifiedContent: 0,
			};
		});

		renderWithRouter(<Dashboard />);

		await waitFor(() => expect(repairs).toBe(1));
	});

	it("updates the toggle optimistically and rolls it back when persistence fails", async () => {
		const current = overview([], { autoUpdateEnabled: false });
		configureDashboard(current);
		let rejectPersistence: ((reason?: unknown) => void) | undefined;
		mockInvokeCommand(
			"set_auto_update_enabled",
			() =>
				new Promise<void>((_resolve, reject) => {
					rejectPersistence = reject;
				}),
		);
		renderWithRouter(<Dashboard />);
		const toggle = await screen.findByRole("switch", {
			name: "Maintenir mes skills à jour",
		});

		await userEvent.click(toggle);
		expect(toggle).toBeChecked();
		rejectPersistence?.(new Error("disk unavailable"));

		await waitFor(() => expect(toggle).not.toBeChecked());
		expect(screen.getByRole("alert")).toHaveTextContent(
			"Impossible de modifier les mises à jour automatiques",
		);
	});

	it("keeps manual updates available while automatic updates are off", async () => {
		const current = overview([installation("meeting-summary", "update_available")], {
			autoUpdateEnabled: false,
		});
		configureDashboard(current);
		let updateRuns = 0;
		mockInvokeCommand("run_managed_updates_now", () => {
			updateRuns += 1;
			return {
				checked: 1,
				available: 0,
				updated: 1,
				actionRequired: 0,
				failed: 0,
			};
		});
		renderWithRouter(<Dashboard />);

		expect(await screen.findByText("1 mise à jour disponible")).toBeInTheDocument();
		await userEvent.click(screen.getByRole("button", { name: "Tout mettre à jour" }));
		expect(updateRuns).toBe(1);
	});

	it("directs an empty workspace to the catalog", async () => {
		configureDashboard(overview());
		renderWithRouter(<Dashboard />);

		const link = await screen.findByRole("link", { name: "Parcourir le catalogue" });
		expect(link).toHaveAttribute("href", "/catalog");
	});

	it("offers safe repair when no assistant is connected", async () => {
		const current = overview([installation("meeting-summary", "action_required")], {
			connectedAgents: 0,
		});
		configureDashboard(current);
		let repairs = 0;
		mockInvokeCommand("repair_all_managed_skills", () => {
			repairs += 1;
			return {
				checked: 1,
				repaired: 1,
				conflicts: 0,
				failed: 0,
				modifiedContent: 0,
			};
		});
		renderWithRouter(<Dashboard />);

		expect(await screen.findByText("Connecter un assistant")).toBeInTheDocument();
		expect(screen.getByText("Connecter un assistant").closest(".panel-inset")).toHaveClass(
			"flex-wrap",
		);
		await waitFor(() => expect(repairs).toBe(1));
		await userEvent.click(screen.getByRole("button", { name: "Réparer les connexions" }));
		expect(repairs).toBe(2);
	});

	it("opens configuration contextually and never offers a destructive conflict shortcut", async () => {
		const current = overview([
			installation("crm-brief", "ready", { missingEnv: true }),
			installation("meeting-summary", "conflict", { bindingStatus: "conflict" }),
		]);
		configureDashboard(current);
		renderWithRouter(<Dashboard />);

		const configureLink = await screen.findByRole("link", { name: "Configurer crm-brief" });
		expect(configureLink).toHaveAttribute("href", "/env?skill=crm-brief");
		expect(screen.queryByRole("button", { name: /forcer|supprimer/i })).not.toBeInTheDocument();
	});

	it("keeps the current organization and reports a failed switch", async () => {
		configureDashboard(overview([installation("meeting-summary")]));
		useAuthStore.setState((state) => ({
			...state,
			user: state.user
				? {
						...state.user,
						orgs: [
							{ slug: "acme", name: "Acme", role: "member" },
							{ slug: "globex", name: "Globex", role: "member" },
						],
					}
				: null,
		}));
		mockInvokeCommand("switch_active_org", () => {
			throw new Error("binding conflict");
		});
		renderWithRouter(<Dashboard />);
		const company = await screen.findByRole("combobox", { name: "Changer d’entreprise" });

		await userEvent.selectOptions(company, "globex");

		expect(await screen.findByRole("alert")).toHaveTextContent(
			"Impossible de changer d’entreprise",
		);
		expect(company).toHaveValue("acme");
	});
});
