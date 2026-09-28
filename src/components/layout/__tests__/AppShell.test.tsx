import { AppShell } from "@/components/layout/AppShell";
import { useAuthStore, useConfigStore } from "@/lib/store";
import { renderWithRouter } from "@/test/render";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/plugin-updater", () => ({
	check: vi.fn(async () => null),
}));

describe("application shell", () => {
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
	});

	it("lets the content column shrink beside a fixed sidebar", () => {
		const { container } = renderWithRouter(
			<AppShell>
				<div>Contenu</div>
			</AppShell>,
		);

		expect(container.querySelector("aside")).toHaveClass("shrink-0");
		expect(container.querySelector("main")).toHaveClass("min-w-0");
	});
});
