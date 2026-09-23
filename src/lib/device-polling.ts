const DEFAULT_EXPIRES_IN_SECONDS = 15 * 60;
const DEFAULT_INTERVAL_SECONDS = 3;

export function resolveDevicePolling(input: {
	expiresIn?: number;
	interval?: number;
}): {
	intervalMs: number;
	maxAttempts: number;
} {
	const expiresIn =
		Number.isFinite(input.expiresIn) && (input.expiresIn ?? 0) > 0
			? Math.min(Math.round(input.expiresIn as number), 30 * 60)
			: DEFAULT_EXPIRES_IN_SECONDS;
	const interval =
		Number.isFinite(input.interval) && (input.interval ?? 0) > 0
			? Math.min(Math.max(Math.round(input.interval as number), 1), 30)
			: DEFAULT_INTERVAL_SECONDS;

	return {
		intervalMs: interval * 1_000,
		maxAttempts: Math.ceil(expiresIn / interval),
	};
}
