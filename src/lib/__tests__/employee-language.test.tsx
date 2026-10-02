import { Setup } from "@/pages/Setup";
import { renderWithRouter } from "@/test/render";
import { mockInvokeCommand } from "@/test/setup";
import { screen } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import { useAuthStore, useConfigStore } from "../store";

describe("employee-facing setup language", () => {
	beforeEach(() => {
		useAuthStore.setState({
			authenticated: true,
			loading: false,
			user: {
				user: {
					id: "user-1",
					email: "employee@example.com",
					name: "Employee",
				},
				orgs: [{ slug: "acme", name: "Acme", role: "member" }],
			},
		});
		useConfigStore.setState({
			config: { org: "acme" },
			loading: false,
		});
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
		mockInvokeCommand("detect_managed_agents", () => []);
		mockInvokeCommand("preview_managed_skills_migration", () => ({
			items: [],
			managedCandidates: 0,
			projectScopeUntouched: 0,
			modifiedUntouched: 0,
			missingRecords: 0,
			conflicts: 0,
			unsupportedAgents: 0,
			alreadyMigrated: 0,
		}));
	});

	it("does not ask the employee to choose an agent after confirming the organization", async () => {
		renderWithRouter(<Setup />, "/setup");

		expect(await screen.findByText("Vos assistants sont prêts")).toBeInTheDocument();
		expect(
			screen.queryByText(/default agent|scope|version|project directory|user directory/i),
		).not.toBeInTheDocument();
	});
});
