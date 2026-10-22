import { LocalSkillImportPanel } from "@/components/settings/LocalSkillImportPanel";
import { useManagedSkillsStore } from "@/lib/store";
import type { LocalImportPreview } from "@/lib/types";
import { mockInvokeCommand } from "@/test/setup";
import { screen, waitFor } from "@testing-library/react";
import { render } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

function preview(overrides: Partial<LocalImportPreview> = {}): LocalImportPreview {
	return {
		items: [],
		importable: 0,
		conflicts: 0,
		externalLinksUntouched: 0,
		alreadyManaged: 0,
		invalidUntouched: 0,
		...overrides,
	};
}

describe("local skill import panel", () => {
	beforeEach(() => {
		useManagedSkillsStore.getState().reset();
	});

	it("shows a business-language preview and keeps conflicts and external links untouched", async () => {
		mockInvokeCommand("preview_local_skills_import", () =>
			preview({
				items: [
					{
						skillName: "review-helper",
						classification: "importable",
						agents: ["claude"],
					},
					{
						skillName: "shared-skill",
						classification: "duplicate_identical",
						agents: ["claude", "codex"],
					},
				],
				importable: 2,
				conflicts: 2,
				externalLinksUntouched: 2,
				invalidUntouched: 2,
			}),
		);

		render(<LocalSkillImportPanel />);

		expect(await screen.findByText("2 skills peuvent être rassemblées")).toBeInTheDocument();
		expect(screen.getByText("review-helper")).toBeInTheDocument();
		expect(screen.getByText("shared-skill")).toBeInTheDocument();
		expect(screen.getByText("2 conflits resteront inchangés.")).toBeInTheDocument();
		expect(screen.getByText("2 liens externes resteront inchangés.")).toBeInTheDocument();
		expect(screen.getByText("2 skills non compatibles resteront inchangées.")).toBeInTheDocument();
		expect(screen.getByText(/rien n’est publié/i)).toBeInTheDocument();
		expect(screen.queryByText(/scope|version|répertoire utilisateur/i)).not.toBeInTheDocument();
	});

	it("confirms the import, refreshes the dashboard model, and shows the result", async () => {
		let previews = 0;
		const before = preview({
			items: [
				{
					skillName: "review-helper",
					classification: "importable",
					agents: ["claude"],
				},
			],
			importable: 1,
		});
		mockInvokeCommand("preview_local_skills_import", () => {
			previews += 1;
			return previews === 1 ? before : preview({ alreadyManaged: 1 });
		});
		mockInvokeCommand("run_local_skills_import", (args: { confirm: boolean }) => {
			expect(args).toEqual({ confirm: true });
			return {
				confirmed: true,
				imported: 1,
				skipped: 0,
				conflicts: 0,
				errors: 0,
				preview: before,
			};
		});
		const refresh = vi.fn(async () => undefined);
		useManagedSkillsStore.setState({ refresh });
		render(<LocalSkillImportPanel />);

		expect(await screen.findByText("1 skill peut être rassemblée")).toBeInTheDocument();
		await userEvent.click(await screen.findByRole("button", { name: "Rassembler 1 skill" }));

		expect(
			await screen.findByText("1 skill est maintenant gérée par SkillReg."),
		).toBeInTheDocument();
		expect(refresh).toHaveBeenCalledOnce();
		await waitFor(() => expect(previews).toBe(2));
	});

	it("stays visible when there is nothing to import", async () => {
		mockInvokeCommand("preview_local_skills_import", () => preview({ invalidUntouched: 1 }));

		render(<LocalSkillImportPanel />);

		expect(await screen.findByText("Aucune skill locale à importer")).toBeInTheDocument();
		expect(screen.getByText("1 skill non compatible restera inchangée.")).toBeInTheDocument();
		expect(screen.getByText("Rassembler mes skills locales")).toBeInTheDocument();
	});

	it("offers a retry when the preview cannot be read", async () => {
		let attempts = 0;
		mockInvokeCommand("preview_local_skills_import", () => {
			attempts += 1;
			if (attempts === 1) throw new Error("read failed");
			return preview();
		});
		render(<LocalSkillImportPanel />);

		await userEvent.click(await screen.findByRole("button", { name: "Réessayer" }));

		expect(await screen.findByText("Aucune skill locale à importer")).toBeInTheDocument();
		expect(attempts).toBe(2);
	});
});
