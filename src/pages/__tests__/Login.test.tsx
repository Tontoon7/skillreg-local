import { Login } from "@/pages/Login";
import { renderWithRouter } from "@/test/render";
import { mockInvokeCommand } from "@/test/setup";
import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";

describe("employee login", () => {
	it("starts browser authentication from the primary action", async () => {
		let initiated = 0;
		mockInvokeCommand("login_initiate", () => {
			initiated += 1;
			return {
				deviceCode: "device-code",
				userCode: "ABCD-EFGH",
				verificationUrl: "/device",
			};
		});
		mockInvokeCommand("open_url", () => undefined);
		mockInvokeCommand("login_poll", () => ({ status: "pending" }));
		renderWithRouter(<Login />, "/login");

		await userEvent.click(screen.getByRole("button", { name: "Se connecter" }));

		expect(initiated).toBe(1);
		expect(await screen.findByText("ABCD-EFGH")).toBeInTheDocument();
	});

	it("keeps manual token login behind advanced disclosure", async () => {
		renderWithRouter(<Login />, "/login");

		expect(screen.queryByPlaceholderText(/sr_live/i)).not.toBeInTheDocument();
		await userEvent.click(screen.getByRole("button", { name: "Connexion avancée" }));

		expect(screen.getByPlaceholderText(/sr_live/i)).toBeInTheDocument();
	});
});
