import { invoke } from "@tauri-apps/api/core";
import { describe, expect, it } from "vitest";
import { mockInvokeCommand } from "../setup";

describe("Tauri invoke test boundary", () => {
	it("dispatches a typed response only for the registered command", async () => {
		mockInvokeCommand<{ orgSlug: string }, { ready: boolean }>(
			"prepare_managed_skills",
			async ({ orgSlug }) => ({ ready: orgSlug === "acme" }),
		);

		await expect(
			invoke<{ ready: boolean }>("prepare_managed_skills", { orgSlug: "acme" }),
		).resolves.toEqual({ ready: true });
	});

	it("rejects an invoke command that the test did not register", async () => {
		await expect(invoke("unexpected_command")).rejects.toThrow(
			'No Tauri invoke mock registered for "unexpected_command"',
		);
	});
});
