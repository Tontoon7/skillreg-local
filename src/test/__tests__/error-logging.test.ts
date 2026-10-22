import { describe, expect, it, vi } from "vitest";

vi.mock("react-dom/client", () => ({ createRoot: () => ({ render: vi.fn() }) }));
vi.mock("@/App", () => ({ App: () => null }));

describe("desktop error logging", () => {
	it("logs only stable codes for errors carrying private payloads", async () => {
		const log = vi.spyOn(console, "error").mockImplementation(() => undefined);
		await import("@/main");
		const syntheticToken = ["sr_live", "synthetic_test_value"].join("_");
		const signedUrl = `https://example.invalid/archive?signature=${syntheticToken}`;
		const error = new ErrorEvent("error", {
			cancelable: true,
			message: `${syntheticToken} ${signedUrl}`,
			filename: "/fixture/private-profile/access.json",
		});
		window.dispatchEvent(error);
		const rejection = new Event("unhandledrejection", { cancelable: true });
		Object.defineProperty(rejection, "reason", {
			value: new Error(`Failed to load ${signedUrl}: ${syntheticToken}`),
		});
		window.dispatchEvent(rejection);
		expect(log.mock.calls).toEqual([["[UNCAUGHT]"], ["[UNHANDLED REJECTION]"]]);
		expect(error.defaultPrevented).toBe(true);
		expect(rejection.defaultPrevented).toBe(true);
	});
});
