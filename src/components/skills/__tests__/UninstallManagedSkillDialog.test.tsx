import { UninstallManagedSkillDialog } from "@/components/skills/UninstallManagedSkillDialog";
import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";

function Confirmation() {
	const [open, setOpen] = useState(false);
	return (
		<>
			<button type="button" onClick={() => setOpen(true)}>
				Désinstaller
			</button>
			{open && (
				<UninstallManagedSkillDialog
					skillName="meeting-summary"
					busy={false}
					onClose={() => setOpen(false)}
					onConfirm={() => setOpen(false)}
				/>
			)}
		</>
	);
}

describe("managed uninstall confirmation", () => {
	it("opens natively with safe initial focus and restores its keyboard trigger", async () => {
		const showModal = vi.spyOn(HTMLDialogElement.prototype, "showModal");
		const close = vi.spyOn(HTMLDialogElement.prototype, "close");
		const user = userEvent.setup();
		render(<Confirmation />);
		await user.tab();
		const trigger = screen.getByRole("button", { name: "Désinstaller" });
		expect(trigger).toHaveFocus();
		await user.keyboard("{Enter}");

		expect(showModal).toHaveBeenCalledOnce();
		expect(
			screen.getByRole("dialog", { name: "Désinstaller meeting-summary ?" }),
		).toHaveAccessibleDescription("La skill ne sera plus disponible dans vos assistants.");
		expect(screen.getByRole("button", { name: "Annuler" })).toHaveFocus();
		await user.keyboard("{Enter}");

		expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
		expect(close).toHaveBeenCalledOnce();
		expect(trigger).toHaveFocus();
	});

	it("handles native Escape cancellation and restores focus", async () => {
		render(<Confirmation />);
		const trigger = screen.getByRole("button", { name: "Désinstaller" });
		await userEvent.click(trigger);
		fireEvent(screen.getByRole("dialog"), new Event("cancel", { cancelable: true }));
		expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
		expect(trigger).toHaveFocus();
	});

	it("blocks native cancellation and repeated actions while uninstalling without reopening", async () => {
		const showModal = vi.spyOn(HTMLDialogElement.prototype, "showModal");
		const onClose = vi.fn();
		const onConfirm = vi.fn();
		const { rerender } = render(
			<UninstallManagedSkillDialog
				skillName="meeting-summary"
				busy={false}
				onClose={onClose}
				onConfirm={onConfirm}
			/>,
		);
		await userEvent.click(screen.getByRole("button", { name: "Confirmer la désinstallation" }));
		expect(onConfirm).toHaveBeenCalledOnce();
		rerender(
			<UninstallManagedSkillDialog
				skillName="meeting-summary"
				busy
				onClose={onClose}
				onConfirm={onConfirm}
			/>,
		);
		const dialog = screen.getByRole("dialog");
		const cancel = new Event("cancel", { cancelable: true });
		fireEvent(dialog, cancel);
		await userEvent.click(screen.getByRole("button", { name: "Confirmer la désinstallation" }));

		expect(cancel.defaultPrevented).toBe(true);
		expect(onClose).not.toHaveBeenCalled();
		expect(onConfirm).toHaveBeenCalledOnce();
		expect(dialog).toHaveAttribute("open");
		expect(dialog).toHaveAttribute("aria-busy", "true");
		expect(screen.getByRole("button", { name: "Annuler" })).toBeDisabled();
		expect(showModal).toHaveBeenCalledOnce();
	});
});
