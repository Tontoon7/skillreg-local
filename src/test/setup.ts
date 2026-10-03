import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterEach, vi } from "vitest";

type InvokeHandler = (args: unknown) => unknown | Promise<unknown>;

const invokeHandlers = new Map<string, InvokeHandler>();
const invokeMock = vi.fn(async (command: string, args?: unknown) => {
	const handler = invokeHandlers.get(command);
	if (!handler) {
		throw new Error(`No Tauri invoke mock registered for "${command}"`);
	}
	return handler(args);
});

const windowCommands = {
	minimize: vi.fn(),
	toggleMaximize: vi.fn(),
	close: vi.fn(),
};

vi.mock("@tauri-apps/api/core", () => ({
	invoke: invokeMock,
}));

vi.mock("@tauri-apps/api/window", () => ({
	getCurrentWindow: () => windowCommands,
}));

// jsdom has no native top layer, focus confinement, or Escape default action.
Object.defineProperties(HTMLDialogElement.prototype, {
	showModal: {
		configurable: true,
		value(this: HTMLDialogElement) {
			this.open = true;
		},
	},
	close: {
		configurable: true,
		value(this: HTMLDialogElement) {
			this.open = false;
		},
	},
});

export function mockInvokeCommand<TArgs, TResult>(
	command: string,
	handler: (args: TArgs) => TResult | Promise<TResult>,
) {
	invokeHandlers.set(command, (args) => handler(args as TArgs));
}

afterEach(() => {
	cleanup();
	invokeHandlers.clear();
	invokeMock.mockClear();
	windowCommands.minimize.mockClear();
	windowCommands.toggleMaximize.mockClear();
	windowCommands.close.mockClear();
});
