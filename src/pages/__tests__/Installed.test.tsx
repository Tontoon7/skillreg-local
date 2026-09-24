import { useConfigStore, useManagedSkillsStore } from "@/lib/store";
import type { ManagedOverview, ManagedSkillBinding } from "@/lib/types";
import { Installed } from "@/pages/Installed";
import { renderWithRouter } from "@/test/render";
import { mockInvokeCommand } from "@/test/setup";
import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it } from "vitest";

function binding(
	agent: ManagedSkillBinding["agent"],
	status: ManagedSkillBinding["status"] = "ready",
): ManagedSkillBinding {
	return {
		agent,
		status,
		lastCheckedAt: "2026-07-29T08:00:00Z",
		lastError: null,
		usageObservability: "unavailable",
	};
}

function configureOverview(
	status: ManagedOverview["installations"][number]["installation"]["status"] = "ready",
	bindings: ManagedSkillBinding[] = [binding("claude"), binding("codex"), binding("cursor")],
) {
	mockInvokeCommand("get_managed_overview", () => ({
		installations: [
			{
				installation: {
					installationId: "meeting-summary-id",
					consumerOrg: "acme",
					sourceOrg: "acme",
					skillId: "skill-1",
					skillName: "meeting-summary",
					activeVersion: "9.4.1",
					status,
					installedAt: "2026-07-29T08:00:00Z",
					lastCheckedAt: "2026-07-29T08:00:00Z",
					lastUpdatedAt: "2026-07-29T08:00:00Z",
					lastError: null,
				},
				bindings,
				missingEnvVars: [],
			},
		],
		agents: [],
		autoUpdateEnabled: true,
	}));
}

describe("managed installed skills", () => {
	beforeEach(() => {
		useConfigStore.setState({
			config: { org: "acme", setupDone: true },
			loading: false,
		});
		useManagedSkillsStore.getState().reset();
	});

	it("renders one canonical skill row for all assistant bindings without versions or paths", async () => {
		configureOverview();
		renderWithRouter(<Installed />, "/installed");

		expect(await screen.findAllByText("meeting-summary")).toHaveLength(1);
		expect(screen.getByText(/Disponible dans Claude, Codex et Cursor/)).toBeInTheDocument();
		expect(screen.queryByText("9.4.1")).not.toBeInTheDocument();
		expect(screen.queryByText(/user|project|\\.claude|\\.codex/i)).not.toBeInTheDocument();
	});

	it("offers an update as the primary action", async () => {
		configureOverview("update_available");
		renderWithRouter(<Installed />, "/installed");

		expect(
			await screen.findByRole("button", { name: "Mettre à jour meeting-summary" }),
		).toBeInTheDocument();
	});

	it("shows a problem without ever offering force", async () => {
		configureOverview("conflict", [binding("claude", "conflict")]);
		renderWithRouter(<Installed />, "/installed");

		expect(
			await screen.findByRole("button", { name: "Voir le problème meeting-summary" }),
		).toBeInTheDocument();
		expect(screen.queryByRole("button", { name: /forcer/i })).not.toBeInTheDocument();
	});

	it("links an empty workspace back to the catalog", async () => {
		mockInvokeCommand("get_managed_overview", () => ({
			installations: [],
			agents: [],
			autoUpdateEnabled: true,
		}));
		renderWithRouter(<Installed />, "/installed");

		expect(await screen.findByRole("link", { name: "Parcourir le catalogue" })).toHaveAttribute(
			"href",
			"/catalog",
		);
	});

	it("confirms managed uninstall by installation ID and explains that saved access is kept", async () => {
		let removed = false;
		let received: Record<string, unknown> | undefined;
		mockInvokeCommand("get_managed_overview", () =>
			removed
				? { installations: [], agents: [], autoUpdateEnabled: true }
				: {
						installations: [
							{
								installation: {
									installationId: "meeting-summary-id",
									consumerOrg: "acme",
									sourceOrg: "acme",
									skillId: "skill-1",
									skillName: "meeting-summary",
									activeVersion: "9.4.1",
									status: "ready",
									installedAt: "1",
									lastCheckedAt: "1",
									lastUpdatedAt: "1",
									lastError: null,
								},
								bindings: [binding("claude")],
								missingEnvVars: [],
							},
						],
						agents: [],
						autoUpdateEnabled: true,
					},
		);
		mockInvokeCommand("uninstall_managed_skill", (args: Record<string, unknown>) => {
			received = args;
			removed = true;
			return {
				installationId: "meeting-summary-id",
				removed: true,
				alreadyRemoved: false,
				bindingsRemoved: 1,
				conflicts: 0,
				envValuesPreserved: true,
				cleanupDeferred: false,
			};
		});
		renderWithRouter(<Installed />, "/installed");

		await userEvent.click(
			await screen.findByRole("button", {
				name: "Plus d’actions pour meeting-summary",
			}),
		);
		await userEvent.click(screen.getByRole("button", { name: "Désinstaller meeting-summary" }));
		expect(screen.getByText(/accès enregistrés seront conservés/i)).toBeInTheDocument();
		await userEvent.click(screen.getByRole("button", { name: "Confirmer la désinstallation" }));

		expect(received).toEqual({ installationId: "meeting-summary-id" });
		expect(await screen.findByText("Aucune skill installée")).toBeInTheDocument();
	});

	it("keeps a skill visible when uninstall is blocked by an owned-binding conflict", async () => {
		configureOverview();
		mockInvokeCommand("uninstall_managed_skill", () => ({
			installationId: "meeting-summary-id",
			removed: false,
			alreadyRemoved: false,
			bindingsRemoved: 0,
			conflicts: 1,
			envValuesPreserved: true,
			cleanupDeferred: false,
		}));
		renderWithRouter(<Installed />, "/installed");

		await userEvent.click(
			await screen.findByRole("button", {
				name: "Plus d’actions pour meeting-summary",
			}),
		);
		await userEvent.click(screen.getByRole("button", { name: "Désinstaller meeting-summary" }));
		await userEvent.click(screen.getByRole("button", { name: "Confirmer la désinstallation" }));

		expect(await screen.findByRole("alert")).toHaveTextContent(
			"meeting-summary n’a pas été désinstallée",
		);
		expect(screen.getByText("meeting-summary")).toBeInTheDocument();
	});
});
