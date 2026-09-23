import { Sidebar } from "@/components/layout/Sidebar";
import { useAuthStore, useConfigStore } from "@/lib/store";
import { Settings } from "@/pages/Settings";
import { renderWithRouter } from "@/test/render";
import { mockInvokeCommand } from "@/test/setup";
import { screen, within } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";

describe("employee navigation", () => {
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
			config: {
				org: "acme",
				setupDone: true,
				autoUpdateEnabled: true,
				autoUpdateIntervalMinutes: 60,
			},
			loading: false,
		});
		mockInvokeCommand("get_catalog_policy", () => ({
			mode: "open",
			minimumValidationLevel: "scanned",
			allowFirstParty: true,
			canInstallFromCatalog: true,
		}));
	});

	it("contains exactly the four employee destinations", () => {
		renderWithRouter(<Sidebar />);

		const navigation = screen.getByRole("navigation", { name: "Navigation principale" });
		expect(
			within(navigation)
				.getAllByRole("link")
				.map((link) => link.textContent?.trim()),
		).toEqual(["Accueil", "Catalogue", "Mes skills", "Réglages"]);
	});

	it("keeps creator and diagnostic routes under advanced settings", () => {
		renderWithRouter(<Settings />, "/settings");

		expect(screen.queryByText("Default Agent")).not.toBeInTheDocument();
		expect(screen.queryByText("Default Scope")).not.toBeInTheDocument();
		expect(screen.getByRole("link", { name: "Commandes et automatisations" })).toHaveAttribute(
			"href",
			"/commands",
		);
		expect(screen.getByRole("link", { name: "Variables et accès" })).toHaveAttribute(
			"href",
			"/env",
		);
		expect(screen.getByText("Rassembler mes skills locales")).toBeInTheDocument();
	});
});
