import { create } from "zustand";
import { logout, readConfig, whoami, writeConfig } from "./api";
import type { AgentType, ScopeType, SkillregConfig, WhoamiResponse } from "./types";

interface AuthState {
	authenticated: boolean;
	loading: boolean;
	user: WhoamiResponse | null;
	checkAuth: () => Promise<void>;
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
				return;
			}
			const user = await whoami();
			set({ authenticated: true, loading: false, user });
		} catch {
			set({ authenticated: false, loading: false, user: null });
		}
	},

	setAuthenticated: (user) => set({ authenticated: true, loading: false, user }),

	logout: async () => {
		await logout();
		set({ authenticated: false, user: null });
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
		const merged = { ...get().config, ...updates };
		await writeConfig(merged);
		set({ config: merged });
	},

	setOrg: async (org) => {
		await get().update({ org });
	},

	setDefaults: async (agent, scope) => {
		await get().update({ defaultAgent: agent, defaultScope: scope, setupDone: true });
	},
}));
