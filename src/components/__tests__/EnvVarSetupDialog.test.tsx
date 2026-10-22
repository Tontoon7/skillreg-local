import { EnvVarSetupDialog } from "@/components/EnvVarSetupDialog";
import { mockInvokeCommand } from "@/test/setup";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

const envVars = [{ name: "CRM_TOKEN", description: "Accès CRM", required: true, secret: true }];

describe("managed access configuration", () => {
	it("opens natively, focuses defer and prevents closing while saving", async () => {
		mockInvokeCommand("list_org_env_vars", () => []);
		let finishSave: (() => void) | undefined;
		mockInvokeCommand(
			"set_org_env_var",
			() =>
				new Promise<void>((resolve) => {
					finishSave = resolve;
				}),
		);
		const showModal = vi.spyOn(HTMLDialogElement.prototype, "showModal");
		const onClose = vi.fn();
		const onSaved = vi.fn();
		render(
			<EnvVarSetupDialog
				skillName="meeting-summary"
				org="acme"
				envVars={envVars}
				onClose={onClose}
				onSaved={onSaved}
			/>,
		);
		const input = await screen.findByLabelText("Accès CRM");
		expect(showModal).toHaveBeenCalledOnce();
		expect(screen.getByRole("button", { name: "Plus tard" })).toHaveFocus();
		await userEvent.type(input, "synthetic-access");
		await userEvent.click(screen.getByRole("button", { name: "Enregistrer" }));
		const cancel = new Event("cancel", { cancelable: true });
		fireEvent(screen.getByRole("dialog"), cancel);
		expect(cancel.defaultPrevented).toBe(true);
		expect(onClose).not.toHaveBeenCalled();
		expect(screen.getByRole("button", { name: "Fermer" })).toBeDisabled();
		expect(input).toBeDisabled();
		finishSave?.();
		await waitFor(() => expect(onSaved).toHaveBeenCalledOnce());
	});

	it("announces failure to inspect saved access and requires a successful retry before editing", async () => {
		let attempts = 0;
		mockInvokeCommand("list_org_env_vars", () => {
			attempts += 1;
			if (attempts === 1) throw new Error("key store unavailable");
			return [];
		});
		render(
			<EnvVarSetupDialog
				skillName="meeting-summary"
				org="acme"
				envVars={envVars}
				onClose={vi.fn()}
				onSaved={vi.fn()}
			/>,
		);
		expect(await screen.findByRole("alert")).toHaveTextContent(
			"Impossible de vérifier les accès existants",
		);
		expect(screen.getByRole("button", { name: "Enregistrer" })).toBeDisabled();
		await userEvent.click(screen.getByRole("button", { name: "Réessayer" }));
		expect(await screen.findByLabelText("Accès CRM")).toBeEnabled();
		expect(attempts).toBe(2);
	});
});
