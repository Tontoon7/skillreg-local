import { create } from "zustand";
import {
	checkManagedUpdates,
	getManagedOverview,
	installManagedSkill,
	logout,
	readConfig,
	repairManagedSkill,
	repairManagedSkills,
	runManagedSkillsMigration,
	runManagedUpdatesNow,
	setAutoUpdateEnabled,
	switchActiveOrg,
	uninstallManagedSkill,
	whoami,
	writeConfig,
} from "./api";
import { type EmployeeDashboardModel, buildEmployeeDashboardModel } from "./employee-model";
import type {
	AgentType,
	ManagedInstallResult,
	ManagedOverview,
	ManagedReconcileReport,
	ManagedUninstallResult,
	ManagedUpdateSummary,
	MigrationReport,
	ScopeType,
	SkillregConfig,
	WhoamiResponse,
} from "./types";

interface AuthState {
	authenticated: boolean;
	loading: boolean;
	user: WhoamiResponse | null;
	checkAuth: () => Promise<WhoamiResponse | null>;
	setAuthenticated: (user: WhoamiResponse) => void;
	logout: () => Promise<void>;
}

export const useAuthStore = create<AuthState>((set) => ({
	authenticated: false,
	loading: true,
	user: null,

	checkAuth: async () => {
		try {
			const config = await readConfig();
			if (!config.token) {
				set({ authenticated: false, loading: false, user: null });
				return null;
			}
			const user = await whoami();
			set({ authenticated: true, loading: false, user });
			return user;
		} catch {
			set({ authenticated: false, loading: false, user: null });
			return null;
		}
	},

	setAuthenticated: (user) => {
		set({ authenticated: true, loading: false, user });
		useConfigStore.getState().load();
	},

	logout: async () => {
		await logout();
		set({ authenticated: false, user: null });
		useManagedSkillsStore.getState().reset();
	},
}));

interface ConfigState {
	config: SkillregConfig;
	loading: boolean;
	load: () => Promise<void>;
	update: (updates: Partial<SkillregConfig>) => Promise<void>;
	setOrg: (org: string) => Promise<void>;
	setDefaults: (agent: AgentType, scope: ScopeType) => Promise<void>;
}

export const useConfigStore = create<ConfigState>((set, get) => ({
	config: {},
	loading: true,

	load: async () => {
		const config = await readConfig();
		set({ config, loading: false });
	},

	update: async (updates) => {
		const fresh = await readConfig();
		const merged = { ...fresh, ...updates };
		await writeConfig(merged);
		set({ config: merged });
	},

	setOrg: async (org) => {
		const report = await switchActiveOrg(org);
		set((state) => ({
			config: { ...state.config, org: report.activeOrg },
			loading: false,
		}));
	},

	setDefaults: async (agent, scope) => {
		await get().update({ defaultAgent: agent, defaultScope: scope, setupDone: true });
	},
}));

export interface ManagedInstallParams {
	consumerOrg: string;
	sourceOrg?: string;
	name: string;
}

interface ManagedSkillsState {
	overview: ManagedOverview | null;
	model: EmployeeDashboardModel | null;
	loading: boolean;
	refreshing: boolean;
	offline: boolean;
	error: string | null;
	installingKeys: string[];
	refresh: () => Promise<void>;
	install: (params: ManagedInstallParams) => Promise<ManagedInstallResult>;
	uninstall: (installationId: string) => Promise<ManagedUninstallResult>;
	checkUpdates: (force?: boolean) => Promise<ManagedUpdateSummary>;
	updateNow: () => Promise<ManagedUpdateSummary>;
	runMigration: (confirm: boolean) => Promise<MigrationReport>;
	setAutomaticUpdates: (enabled: boolean) => Promise<void>;
	repair: (installationId: string) => Promise<ManagedReconcileReport>;
	repairAll: () => Promise<ManagedReconcileReport>;
	reset: () => void;
}

const pendingManagedInstalls = new Map<string, Promise<ManagedInstallResult>>();

export const useManagedSkillsStore = create<ManagedSkillsState>((set, get) => ({
	overview: null,
	model: null,
	loading: true,
	refreshing: false,
	offline: false,
	error: null,
	installingKeys: [],

	refresh: async () => {
		const hasLocalState = get().overview !== null;
		set(hasLocalState ? { refreshing: true, error: null } : { loading: true, error: null });
		try {
			const overview = await getManagedOverview();
			set({
				overview,
				model: buildEmployeeDashboardModel(overview),
				loading: false,
				refreshing: false,
				offline: false,
				error: null,
			});
		} catch (error) {
			const overview = get().overview;
			const currentModel = get().model;
			const offlineModel: EmployeeDashboardModel | null = overview
				? buildEmployeeDashboardModel(overview, { offline: true })
				: currentModel
					? { ...currentModel, health: "offline" }
					: null;
			set({
				model: offlineModel,
				loading: false,
				refreshing: false,
				offline: true,
				error: errorMessage(error),
			});
		}
	},

	install: (params) => {
		const key = managedInstallKey(params);
		const pending = pendingManagedInstalls.get(key);
		if (pending) return pending;

		set((state) => ({
			installingKeys: state.installingKeys.includes(key)
				? state.installingKeys
				: [...state.installingKeys, key],
		}));
		const operation = installManagedSkill(params)
			.then(async (result) => {
				await get().refresh();
				return result;
			})
			.finally(() => {
				pendingManagedInstalls.delete(key);
				set((state) => ({
					installingKeys: state.installingKeys.filter((candidate) => candidate !== key),
				}));
			});
		pendingManagedInstalls.set(key, operation);
		return operation;
	},

	uninstall: async (installationId) => {
		const result = await uninstallManagedSkill(installationId);
		await get().refresh();
		return result;
	},

	checkUpdates: async (force = false) => {
		const summary = await checkManagedUpdates(force);
		await get().refresh();
		return summary;
	},

	updateNow: async () => {
		const summary = await runManagedUpdatesNow();
		await get().refresh();
		return summary;
	},

	runMigration: async (confirm) => {
		const report = await runManagedSkillsMigration(confirm);
		await get().refresh();
		return report;
	},

	setAutomaticUpdates: async (enabled) => {
		const previous = get().overview;
		if (previous) {
			const optimistic = { ...previous, autoUpdateEnabled: enabled };
			set({
				overview: optimistic,
				model: buildEmployeeDashboardModel(optimistic, { offline: get().offline }),
			});
		}
		try {
			await setAutoUpdateEnabled(enabled);
			await useConfigStore.getState().load();
		} catch (error) {
			if (previous) {
				set({
					overview: previous,
					model: buildEmployeeDashboardModel(previous, { offline: get().offline }),
				});
			}
			throw error;
		}
	},

	repair: async (installationId) => {
		const report = await repairManagedSkill(installationId);
		await get().refresh();
		return report;
	},

	repairAll: async () => {
		const report = await repairManagedSkills();
		await get().refresh();
		return report;
	},

	reset: () => {
		pendingManagedInstalls.clear();
		set({
			overview: null,
			model: null,
			loading: true,
			refreshing: false,
			offline: false,
			error: null,
			installingKeys: [],
		});
	},
}));

function managedInstallKey(params: ManagedInstallParams): string {
	return `${params.consumerOrg}/${params.sourceOrg ?? params.consumerOrg}/${params.name}`;
}

function errorMessage(error: unknown): string {
	if (typeof error === "string") return error;
	if (error instanceof Error) return error.message;
	return "Impossible d’actualiser l’état local.";
}
