import { invoke } from "@tauri-apps/api/core";
import type {
	EnvMigrationSummary,
	LegacyCleanupSummary,
	OrgEnvVariable,
	SecureStoreMigrationSummary,
} from "./env-inventory";
import type {
	ActiveOrgSwitchReport,
	AgentType,
	AutoUpdateRunSummary,
	CatalogPolicy,
	CommandInstallResult,
	CommandRemoveResult,
	CommandUpdateResult,
	CommandVersion,
	DeviceFlowResponse,
	InstallResult,
	InstalledCommandRecord,
	LocalImportPreview,
	LocalImportReport,
	LocalSkill,
	ManagedAgent,
	ManagedInstallResult,
	ManagedOverview,
	ManagedReconcileReport,
	ManagedUninstallResult,
	ManagedUpdateSummary,
	MigrationPreview,
	MigrationReport,
	PaginatedCatalogSkills,
	PaginatedSkills,
	PollResponse,
	ProposalDetail,
	ProposalSummary,
	PublishCommandVersionInput,
	PushResult,
	RegistryCommand,
	RegistryCommandDetail,
	ScopeType,
	SearchResponse,
	SkillDetail,
	SkillregConfig,
	TrackedInstallation,
	UpdateInfo,
	WhoamiResponse,
} from "./types";

// Config
export const readConfig = () => invoke<SkillregConfig>("read_config");
export const writeConfig = (config: SkillregConfig) => invoke<void>("write_config", { config });
export const setLaunchAtLogin = (enabled: boolean) =>
	invoke<void>("set_launch_at_login", { enabled });
export const setAutoUpdateEnabled = (enabled: boolean) =>
	invoke<void>("set_auto_update_enabled", { enabled });

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

export const getCatalogPolicy = (org: string) =>
	invoke<CatalogPolicy>("get_catalog_policy", { org });

export const listCatalogSkills = (params: {
	query?: string;
	firstPartyOnly?: boolean;
	page?: number;
	limit?: number;
}) => invoke<PaginatedCatalogSkills>("list_catalog_skills", params);

export const installCatalogSkill = (params: {
	sourceOrg: string;
	name: string;
	version?: string;
	consumerOrg: string;
	agent: AgentType;
	scope: ScopeType;
	projectDir?: string;
	acceptVersionChange?: boolean;
}) => invoke<InstallResult>("install_catalog_skill", params);

export const installManagedSkill = (params: {
	consumerOrg: string;
	sourceOrg?: string;
	name: string;
}) => invoke<ManagedInstallResult>("install_managed_skill", params);

export const previewManagedSkillsMigration = () =>
	invoke<MigrationPreview>("preview_managed_skills_migration");

export const runManagedSkillsMigration = (confirm: boolean) =>
	invoke<MigrationReport>("run_managed_skills_migration", { confirm });

export const previewLocalSkillsImport = () =>
	invoke<LocalImportPreview>("preview_local_skills_import");

export const runLocalSkillsImport = (confirm: boolean) =>
	invoke<LocalImportReport>("run_local_skills_import", { confirm });

export const repairManagedSkill = (installationId: string) =>
	invoke<ManagedReconcileReport>("repair_managed_skill", { installationId });

export const repairManagedSkills = () =>
	invoke<ManagedReconcileReport>("repair_all_managed_skills");

export const uninstallManagedSkill = (installationId: string) =>
	invoke<ManagedUninstallResult>("uninstall_managed_skill", { installationId });

export const switchActiveOrg = (org: string) =>
	invoke<ActiveOrgSwitchReport>("switch_active_org", { org });

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

export const deleteSkill = (org: string, name: string) =>
	invoke<boolean>("delete_skill", { org, name });

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

// Slash commands
export const listCommands = (org: string) => invoke<RegistryCommand[]>("list_commands", { org });

export const getCommand = (org: string, name: string) =>
	invoke<RegistryCommandDetail>("get_command", { org, name });

export const publishCommandVersion = (
	org: string,
	name: string,
	input: PublishCommandVersionInput,
) => invoke<CommandVersion>("publish_command_version", { org, name, input });

export const pullCommand = (params: {
	org: string;
	name: string;
	version?: string;
	agent: string;
	scope: string;
	projectDir?: string;
}) => invoke<CommandInstallResult>("pull_command", params);

export const listLocalCommands = (params?: { org?: string; agent?: string; scope?: string }) =>
	invoke<InstalledCommandRecord[]>("list_local_commands", params ?? {});

export const removeCommand = (params: {
	org: string;
	name: string;
	agent?: string;
	scope?: string;
}) => invoke<CommandRemoveResult>("remove_command", params);

export const updateCommand = (params: {
	org?: string;
	name?: string;
	agent?: string;
	scope?: string;
	version?: string;
	force?: boolean;
}) => invoke<CommandUpdateResult>("update_command", params);

export const listTrackedInstallations = () =>
	invoke<TrackedInstallation[]>("list_tracked_installations");

export const setSkillAutoUpdate = (params: {
	org: string;
	name: string;
	agent: string;
	scope: string;
	projectDir?: string | null;
	enabled: boolean;
}) => invoke<void>("set_skill_auto_update", params);

export const runAutoUpdateNow = () => invoke<AutoUpdateRunSummary>("run_auto_update_now");

export const checkManagedUpdates = (force: boolean) =>
	invoke<ManagedUpdateSummary>("check_managed_updates", { force });

export const runManagedUpdatesNow = () => invoke<ManagedUpdateSummary>("run_managed_updates_now");

// Skills (local filesystem)
export const scanLocalSkills = (agent?: string, scope?: string) =>
	invoke<LocalSkill[]>("scan_local_skills", { agent, scope });

export const detectAgents = () => invoke<ManagedAgent[]>("detect_managed_agents");
export const getManagedOverview = () => invoke<ManagedOverview>("get_managed_overview");

// Env vars
export const getEnvVars = (org: string, skill: string) =>
	invoke<Record<string, string>>("get_env_vars", { org, skill });

export const getOrgEnvVar = (org: string, key: string) =>
	invoke<string | null>("get_org_env_var", { org, key });

export const setOrgEnvVar = (org: string, key: string, value: string) =>
	invoke<void>("set_org_env_var", { org, key, value });

export const deleteOrgEnvVar = (org: string, key: string) =>
	invoke<void>("delete_org_env_var", { org, key });

export const listOrgEnvVars = (org: string) =>
	invoke<OrgEnvVariable[]>("list_org_env_vars", { org });

export const previewLegacyEnvMigration = (org: string) =>
	invoke<EnvMigrationSummary>("preview_legacy_env_migration", { org });

export const migrateLegacyEnvVars = (org: string) =>
	invoke<EnvMigrationSummary>("migrate_legacy_env_vars", { org });

export const cleanupLegacyEnvVars = (org: string) =>
	invoke<LegacyCleanupSummary>("cleanup_legacy_env_vars", { org });

export const migrateOrgEnvFileToSecureStore = (org: string) =>
	invoke<SecureStoreMigrationSummary>("migrate_org_env_file_to_secure_store", { org });

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
