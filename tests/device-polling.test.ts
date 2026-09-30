import assert from "node:assert/strict";
import test from "node:test";
import { resolveDevicePolling } from "../src/lib/device-polling.ts";

test("uses the device lifetime returned by the API", () => {
	assert.deepEqual(resolveDevicePolling({ expiresIn: 900, interval: 3 }), {
		intervalMs: 3_000,
		maxAttempts: 300,
	});
});

test("falls back safely when an older response omits timing fields", () => {
	assert.deepEqual(resolveDevicePolling({}), {
		intervalMs: 3_000,
		maxAttempts: 300,
	});
});
