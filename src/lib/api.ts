import { invoke } from "@tauri-apps/api/core";
import type {
	DeviceFlowResponse,
	InstallResult,
	LocalSkill,
	PaginatedSkills,
	PollResponse,
	ProposalDetail,
	ProposalSummary,
	PushResult,
	SearchResponse,
	SkillDetail,
	SkillregConfig,
	UpdateInfo,
	WhoamiResponse,
} from "./types";

// Config
export const readConfig = () => invoke<SkillregConfig>("read_config");
export const writeConfig = (config: SkillregConfig) => invoke<void>("write_config", { config });

// Auth — all HTTP goes through Rust
export const loginInitiate = () => invoke<DeviceFlowResponse>("login_initiate");
export const loginPoll = (deviceCode: string) => invoke<PollResponse>("login_poll", { deviceCode });
export const loginWithToken = (token: string) => invoke<boolean>("login_with_token", { token });
export const whoami = () => invoke<WhoamiResponse>("whoami");
export const logout = () => invoke<void>("logout");

// Skills (registry)
export const listSkills = (params: {
	org: string;
	page?: number;
	limit?: number;
	search?: string;
	sort?: string;
	tags?: string[];
}) => invoke<PaginatedSkills>("list_skills", params);

export const getSkill = (org: string, name: string) =>
	invoke<SkillDetail>("get_skill", { org, name });

export const searchSkills = (query: string, org?: string) =>
	invoke<SearchResponse>("search_skills", { query, org });

export const pullSkill = (params: {
	org: string;
	name: string;
	version?: string;
	agent: string;
	scope: string;
	projectDir?: string;
}) => invoke<InstallResult>("pull_skill", params);

export const pushSkill = (params: {
	org: string;
	dirPath: string;
	version?: string;
	tag?: string;
	dryRun: boolean;
}) => invoke<PushResult>("push_skill", params);

export const proposeSkillChange = (params: {
	org: string;
	dirPath: string;
	title: string;
	intent: string;
}) => invoke<ProposalSummary>("propose_skill_change", params);

export const listSkillProposals = (org: string, name: string) =>
	invoke<ProposalSummary[]>("list_skill_proposals", { org, name });

export const getSkillProposal = (org: string, name: string, proposalId: string) =>
	invoke<ProposalDetail>("get_skill_proposal", { org, name, proposalId });

export const uninstallSkill = (name: string, agent: string, scope: string, projectDir?: string) =>
	invoke<boolean>("uninstall_skill", { name, agent, scope, projectDir });

export const checkUpdates = (org: string, localSkills: LocalSkill[]) =>
	invoke<UpdateInfo[]>("check_updates", { org, localSkills });

// Skills (local filesystem)
export const scanLocalSkills = (agent?: string, scope?: string) =>
	invoke<LocalSkill[]>("scan_local_skills", { agent, scope });

// Env vars
export const getEnvVars = (org: string, skill: string) =>
	invoke<Record<string, string>>("get_env_vars", { org, skill });

export const setEnvVars = (org: string, skill: string, vars: Record<string, string>) =>
	invoke<void>("set_env_vars", { org, skill, vars });

export const deleteEnvVars = (org: string, skill: string, keys: string[]) =>
	invoke<void>("delete_env_vars", { org, skill, keys });

export const listAllEnvVars = (org: string) =>
	invoke<Array<{ skill: string; vars: Record<string, string> }>>("list_all_env_vars", { org });

export const importEnvFile = (org: string, skill: string, filePath: string) =>
	invoke<Record<string, string>>("import_env_file", { org, skill, filePath });

// Browser
export const openUrl = (url: string) => invoke<void>("open_url", { url });
