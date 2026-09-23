import { useAuthStore, useConfigStore } from "@/lib/store";
import { Setup } from "@/pages/Setup";
import { renderWithRouter } from "@/test/render";
import { mockInvokeCommand } from "@/test/setup";
import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it } from "vitest";

const acme = { slug: "acme", name: "Acme", role: "member" };
const kairia = { slug: "kairia", name: "Kairia", role: "member" };

function configureCommands(
	preview = {
		items: [],
		managedCandidates: 0,
		projectScopeUntouched: 0,
		modifiedUntouched: 0,
		missingRecords: 0,
		conflicts: 0,
		unsupportedAgents: 0,
		alreadyMigrated: 0,
	},
) {
	mockInvokeCommand("read_config", () => useConfigStore.getState().config);
	mockInvokeCommand("write_config", () => undefined);
	mockInvokeCommand("switch_active_org", ({ org }: { org: string }) => {
		const previousOrg = useConfigStore.getState().config.org ?? null;
		useConfigStore.setState((state) => ({
			config: { ...state.config, org },
		}));
		return {
			previousOrg,
			activeOrg: org,
			bindingsRemoved: 0,
			bindingsCreated: 0,
			cachedSkillsReused: 0,
		};
	});
	mockInvokeCommand("detect_managed_agents", () => [
		{
			agent: "claude",
			state: "detected",
			detectedVersion: "1",
			requiresRestartAfterBinding: false,
		},
	]);
	mockInvokeCommand("preview_managed_skills_migration", () => preview);
}

describe("employee setup", () => {
	beforeEach(() => {
		useAuthStore.setState({
			authenticated: true,
			loading: false,
			user: {
				user: { id: "user-1", email: "employee@example.com", name: "Employee" },
				orgs: [acme],
			},
		});
		useConfigStore.setState({ config: {}, loading: false });
		configureCommands();
	});

	it("selects a single organization automatically and detects assistants", async () => {
		renderWithRouter(<Setup />, "/setup");

		expect(await screen.findByText("Vos assistants sont prêts")).toBeInTheDocument();
		expect(
			screen.queryByText(/default agent|scope|version|project directory/i),
		).not.toBeInTheDocument();
	});

	it("shows company names only when a real organization choice is needed", async () => {
		useAuthStore.setState((state) => ({
			...state,
			user: state.user ? { ...state.user, orgs: [acme, kairia] } : null,
		}));
		renderWithRouter(<Setup />, "/setup");

		expect(screen.getByRole("button", { name: /Acme/ })).toBeInTheDocument();
		expect(screen.getByRole("button", { name: /Kairia/ })).toBeInTheDocument();
		expect(screen.queryByText(/^acme$/)).not.toBeInTheDocument();
	});

	it("does not reuse an organization preference that is no longer authorized", () => {
		useAuthStore.setState((state) => ({
			...state,
			user: state.user ? { ...state.user, orgs: [acme, kairia] } : null,
		}));
		useConfigStore.setState({ config: { org: "former-company" }, loading: false });
		renderWithRouter(<Setup />, "/setup");

		expect(screen.getByText("Choisissez votre entreprise")).toBeInTheDocument();
	});

	it("summarizes migration and lets the employee continue later without mutation", async () => {
		let migrationRuns = 0;
		configureCommands({
			items: [],
			managedCandidates: 2,
			projectScopeUntouched: 1,
			modifiedUntouched: 1,
			missingRecords: 0,
			conflicts: 0,
			unsupportedAgents: 0,
			alreadyMigrated: 0,
		});
		mockInvokeCommand("run_managed_skills_migration", () => {
			migrationRuns += 1;
		});
		renderWithRouter(<Setup />, "/setup");

		expect(
			await screen.findByText("SkillReg peut simplifier vos installations"),
		).toBeInTheDocument();
		expect(screen.getByText(/2 skills seront centralisées/)).toBeInTheDocument();
		await userEvent.click(screen.getByRole("button", { name: "Plus tard" }));

		expect(migrationRuns).toBe(0);
		expect(await screen.findByText("Vos assistants sont prêts")).toBeInTheDocument();
	});

	it("continues when assistant detection fails", async () => {
		mockInvokeCommand("detect_managed_agents", () => {
			throw new Error("detection failed");
		});
		renderWithRouter(<Setup />, "/setup");

		await waitFor(() => expect(screen.getByText("Vos assistants sont prêts")).toBeInTheDocument());
		expect(screen.getByText(/aucun assistant détecté/i)).toBeInTheDocument();
	});

	it("shows a retry action when organization preparation fails", async () => {
		let attempts = 0;
		mockInvokeCommand("switch_active_org", ({ org }: { org: string }) => {
			attempts += 1;
			if (attempts === 1) throw new Error("temporary failure");
			useConfigStore.setState((state) => ({
				config: { ...state.config, org },
			}));
			return {
				previousOrg: null,
				activeOrg: org,
				bindingsRemoved: 0,
				bindingsCreated: 0,
				cachedSkillsReused: 0,
			};
		});
		renderWithRouter(<Setup />, "/setup");

		expect(await screen.findByRole("alert")).toHaveTextContent("Impossible de préparer cet espace");
		await userEvent.click(screen.getByRole("button", { name: "Réessayer" }));

		expect(await screen.findByText("Vos assistants sont prêts")).toBeInTheDocument();
		expect(attempts).toBe(2);
	});

	it("keeps setup open when the final preference cannot be saved", async () => {
		mockInvokeCommand("write_config", () => {
			throw new Error("disk unavailable");
		});
		renderWithRouter(<Setup />, "/setup");
		expect(await screen.findByText("Vos assistants sont prêts")).toBeInTheDocument();

		await userEvent.click(screen.getByRole("button", { name: "Ouvrir SkillReg" }));

		expect(await screen.findByRole("alert")).toHaveTextContent(
			"Impossible de terminer la préparation",
		);
		expect(screen.getByRole("button", { name: "Ouvrir SkillReg" })).toBeEnabled();
	});

	it("lets an account without organization create one in the browser, then continues", async () => {
		const openedUrls: string[] = [];
		useAuthStore.setState((state) => ({
			...state,
			user: state.user ? { ...state.user, orgs: [] } : null,
		}));
		useConfigStore.setState({ config: { token: "sr_live_test" }, loading: false });
		mockInvokeCommand("open_url", ({ url }: { url: string }) => {
			openedUrls.push(url);
		});
		mockInvokeCommand("whoami", () => ({
			user: { id: "user-1", email: "employee@example.com", name: "Employee" },
			orgs: [acme],
		}));
		renderWithRouter(<Setup />, "/setup");

		expect(screen.getByText("Aucune entreprise disponible")).toBeInTheDocument();
		await userEvent.click(
			screen.getByRole("button", { name: "Créer mon espace dans le navigateur" }),
		);
		expect(openedUrls).toEqual(["https://app.skillreg.dev/onboarding?source=desktop"]);

		await userEvent.click(screen.getByRole("button", { name: "Actualiser" }));

		expect(await screen.findByText("Vos assistants sont prêts")).toBeInTheDocument();
		expect(useConfigStore.getState().config.org).toBe("acme");
	});

	it("keeps asking to create an organization while the refreshed account has none", async () => {
		useAuthStore.setState((state) => ({
			...state,
			user: state.user ? { ...state.user, orgs: [] } : null,
		}));
		useConfigStore.setState({ config: { token: "sr_live_test" }, loading: false });
		mockInvokeCommand("whoami", () => ({
			user: { id: "user-1", email: "employee@example.com", name: "Employee" },
			orgs: [],
		}));
		renderWithRouter(<Setup />, "/setup");

		await userEvent.click(screen.getByRole("button", { name: "Actualiser" }));

		expect(await screen.findByRole("alert")).toHaveTextContent("Aucun espace pour le moment");
		expect(screen.getByText("Aucune entreprise disponible")).toBeInTheDocument();
	});
});
