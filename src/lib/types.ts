export type AgentType = "claude" | "codex" | "cursor";

export type AgentDetectionState = "detected" | "not_detected" | "unsupported" | "error";

export interface AgentDetection {
	agent: AgentType;
	state: AgentDetectionState;
	detectedVersion?: string;
	preferredPath?: string;
	legacySkillDirs: string[];
	requiresRestartAfterBinding: boolean;
	detailCode?: string;
}

export interface ManagedAgent {
	agent: AgentType;
	state: AgentDetectionState;
	detectedVersion?: string | null;
	requiresRestartAfterBinding: boolean;
	detailCode?: string | null;
}
export type ScopeType = "project" | "user";

// Config stored in ~/.skillreg/config.json
export interface SkillregConfig {
	token?: string;
	apiUrl?: string;
	org?: string;
	defaultAgent?: AgentType;
	defaultScope?: ScopeType;
	setupDone?: boolean;
	autoUpdateEnabled?: boolean;
	autoUpdateIntervalMinutes?: number;
	launchAtLogin?: boolean;
}

// Auth
export interface WhoamiResponse {
	user: {
		id: string;
		email: string;
		name: string | null;
	};
	orgs: Array<{
		slug: string;
		name: string;
		role: string;
	}>;
}

export interface DeviceFlowResponse {
	deviceCode: string;
	userCode: string;
	verificationUrl: string;
}

export interface PollResponse {
	status: "pending" | "complete";
	token?: string;
}

// Registry skills
export interface RegistrySkill {
	id: string;
	name: string;
	description: string | null;
	tags: string[];
	isPublic: boolean;
	latestVersion: string | null;
	totalDownloads: number;
	totalVersions: number;
	createdAt: string;
	updatedAt: string;
}

export type ValidationLevel = "unvalidated" | "scanned" | "verified" | "certified";

export interface ValidationCheck {
	id: string;
	category: "structure" | "security" | "trigger" | "behavior";
	status: "pass" | "warn" | "fail";
	message: string;
}

export interface ValidationResult {
	level: ValidationLevel;
	checks: ValidationCheck[];
	score: number;
	validatedAt: string;
	engineVersion: string;
}

export interface CatalogPolicy {
	mode: "blocked" | "allowlist" | "validated_only" | "open";
	minimumValidationLevel: ValidationLevel;
	allowFirstParty: boolean;
	canInstallFromCatalog: boolean;
}

export interface CatalogValidation {
	level: ValidationLevel;
	score: number;
	passed: number;
	warned: number;
	failed: number;
}

export interface CatalogSkill {
	name: string;
	description: string | null;
	tags: string[];
	orgSlug: string;
	orgName: string;
	isFirstParty: boolean;
	latestVersion: string;
	totalDownloads: number;
	installCommand: string;
	validation: CatalogValidation;
}

export interface PaginatedCatalogSkills {
	skills: CatalogSkill[];
	pagination: { page: number; limit: number; total: number; totalPages: number };
}

export interface SkillDetail extends RegistrySkill {
	isDeprecated: boolean;
	deprecatedMessage: string | null;
	latestVersionData: {
		version: string;
		tarballSize: number;
		sha256: string | null;
		skillMdContent: string | null;
		filesManifest: string[] | null;
		fileCount: number | null;
		downloads: number;
		validationLevel: ValidationLevel | null;
		validation: ValidationResult | null;
		createdAt: string;
	} | null;
	versions: SkillVersion[];
}

export interface SkillVersion {
	id: string;
	version: string;
	tarballSize: number;
	sha256: string | null;
	downloads: number;
	status: "approved" | "pending" | "rejected";
	validationLevel: ValidationLevel | null;
	createdAt: string;
}

export interface PaginatedSkills {
	skills: RegistrySkill[];
	pagination: {
		page: number;
		limit: number;
		total: number;
		totalPages: number;
	};
}

export interface SearchResult {
	id: string;
	name: string;
	description: string | null;
	tags: string[];
	latestVersion: string | null;
	totalDownloads: number;
	orgSlug: string;
}

export interface SearchResponse {
	results: SearchResult[];
	total: number;
}

export interface EnvVarDecl {
	name: string;
	description: string;
	required: boolean;
	secret?: boolean;
	default?: string;
}

export interface InstallResult {
	name: string;
	version: string;
	path: string;
	filesCount: number;
	envVars: EnvVarDecl[];
	sha256: string | null;
	contentHash: string;
}

export type ManagedSkillStatus =
	| "installing"
	| "ready"
	| "update_available"
	| "updating"
	| "action_required"
	| "conflict"
	| "error";

export type ManagedBindingStatus =
	| "ready"
	| "missing"
	| "conflict"
	| "unsupported"
	| "needs_restart"
	| "error";

export type ManagedErrorCode =
	| "MANAGED_PATH_OUTSIDE_ROOT"
	| "MANIFEST_INVALID"
	| "MANIFEST_WRITE_FAILED"
	| "ARCHIVE_CHECKSUM_MISMATCH"
	| "ARCHIVE_UNSAFE"
	| "CONTENT_MODIFIED"
	| "BINDING_CONFLICT"
	| "BINDING_UNSUPPORTED"
	| "BINDING_CREATE_FAILED"
	| "BINDING_VERIFY_FAILED"
	| "SKILL_SOURCE_NAME_CONFLICT"
	| "ROLLBACK_FAILED"
	| "MIGRATION_REQUIRES_ACTION"
	| "REGISTRY_REQUEST_FAILED"
	| "DOWNLOAD_METADATA_INVALID"
	| "MANAGED_INSTALL_BUSY"
	| "MANAGED_INSTALLATION_NOT_FOUND"
	| "ACTIVE_ORGANIZATION_UNAUTHORIZED"
	| "ACTIVE_ORGANIZATION_CONFLICT"
	| "AUTHENTICATION_REQUIRED"
	| "LOCAL_CONFIGURATION_INVALID";

export interface ManagedErrorRecord {
	code: ManagedErrorCode;
	parameters?: Record<string, string>;
	occurredAt?: string | null;
}

export interface ManagedSkillInstallation {
	installationId: string;
	consumerOrg: string;
	sourceOrg: string;
	skillId: string | null;
	skillName: string;
	origin: "registry" | "local";
	activeVersion: string;
	status: ManagedSkillStatus;
	installedAt: string;
	lastCheckedAt: string | null;
	lastUpdatedAt: string | null;
	lastError: ManagedErrorRecord | null;
}

export interface ManagedSkillBinding {
	agent: AgentType;
	status: ManagedBindingStatus;
	lastCheckedAt: string | null;
	lastError: ManagedErrorRecord | null;
	usageObservability: "exact" | "partial" | "unavailable" | "error";
}

export type ManagedWarningCode =
	| "binding_conflict"
	| "binding_unsupported"
	| "binding_error"
	| "skill_source_name_conflict"
	| "cleanup_deferred";

export interface ManagedWarning {
	code: ManagedWarningCode;
	agent: AgentType | null;
}

export interface ManagedInstallResult {
	installation: ManagedSkillInstallation;
	bindings: ManagedSkillBinding[];
	requiredEnvVars: EnvVarDecl[];
	warnings: ManagedWarning[];
}

export interface ManagedUninstallResult {
	installationId: string;
	removed: boolean;
	alreadyRemoved: boolean;
	bindingsRemoved: number;
	conflicts: number;
	envValuesPreserved: boolean;
	cleanupDeferred: boolean;
}

export interface ActiveOrgSwitchReport {
	previousOrg: string | null;
	activeOrg: string;
	bindingsRemoved: number;
	bindingsCreated: number;
	cachedSkillsReused: number;
}

export interface ManagedOverviewInstallation {
	installation: ManagedSkillInstallation;
	bindings: ManagedSkillBinding[];
	missingEnvVars: EnvVarDecl[];
}

export interface ManagedOverview {
	installations: ManagedOverviewInstallation[];
	agents: ManagedAgent[];
	autoUpdateEnabled: boolean;
}

export type MigrationClassification =
	| "managed_candidate"
	| "project_scope_leave_untouched"
	| "modified_leave_untouched"
	| "missing_leave_record"
	| "unmanaged_conflict"
	| "duplicate_identical"
	| "duplicate_divergent"
	| "unsupported_agent"
	| "already_migrated";

export interface MigrationPreviewItem {
	consumerOrg: string;
	sourceOrg: string;
	skillName: string;
	classification: MigrationClassification;
	agents: AgentType[];
}

export interface MigrationPreview {
	items: MigrationPreviewItem[];
	managedCandidates: number;
	projectScopeUntouched: number;
	modifiedUntouched: number;
	missingRecords: number;
	conflicts: number;
	unsupportedAgents: number;
	alreadyMigrated: number;
}

export interface MigrationReport {
	confirmed: boolean;
	migrated: number;
	skipped: number;
	conflicts: number;
	errors: number;
	preview: MigrationPreview;
}

export type LocalImportClassification =
	| "importable"
	| "duplicate_identical"
	| "duplicate_divergent"
	| "external_link_untouched"
	| "already_managed"
	| "invalid_untouched";

export interface LocalImportPreviewItem {
	skillName: string;
	classification: LocalImportClassification;
	agents: AgentType[];
}

export interface LocalImportPreview {
	items: LocalImportPreviewItem[];
	importable: number;
	conflicts: number;
	externalLinksUntouched: number;
	alreadyManaged: number;
	invalidUntouched: number;
}

export interface LocalImportReport {
	confirmed: boolean;
	imported: number;
	skipped: number;
	conflicts: number;
	errors: number;
	preview: LocalImportPreview;
}

export interface ManagedReconcileReport {
	checked: number;
	repaired: number;
	conflicts: number;
	failed: number;
	modifiedContent: number;
}

export interface ManagedUpdateSummary {
	checked: number;
	available: number;
	updated: number;
	actionRequired: number;
	failed: number;
}

export interface PushResult {
	name: string;
	version: string;
	size: number;
	sha256: string;
	dryRun: boolean;
}

export interface UpdateInfo {
	name: string;
	localVersion: string;
	serverVersion: string;
	agent: string;
	scope: string;
}

export interface RegistryCommand {
	id?: string | null;
	name: string;
	description: string;
	latestVersion: string | null;
	totalVersions: number;
	agentCompatibility: AgentType[];
	scope: string;
}

export interface CommandVersion {
	id?: string | null;
	version: string;
	content: string;
	agentCompatibility: AgentType[];
	scope?: string | null;
}

export interface RegistryCommandDetail extends RegistryCommand {
	versions: CommandVersion[];
}

export interface InstalledCommandRecord {
	org: string;
	name: string;
	version: string;
	agent: AgentType;
	scope: ScopeType;
	path: string;
	contentSha256: string;
	installedAt: string;
}

export interface CommandInstallResult {
	org: string;
	name: string;
	version: string;
	scope: ScopeType;
	paths: string[];
}

export interface CommandUpdateSkipped {
	org: string;
	name: string;
	version: string;
	agent: AgentType;
	scope: ScopeType;
	path: string;
	contentSha256: string;
	installedAt: string;
	reason: string;
}

export interface CommandUpdateResult {
	updated: InstalledCommandRecord[];
	skipped: CommandUpdateSkipped[];
}

export interface CommandRemoveResult {
	removed: InstalledCommandRecord[];
}

export interface TrackedInstallation {
	org: string;
	name: string;
	version: string;
	agent: AgentType;
	scope: ScopeType;
	projectDir: string | null;
	installPath: string;
	contentHash: string;
	/** Publisher org when installed from the public catalog. */
	sourceOrg?: string | null;
	sha256: string | null;
	autoUpdateEnabled: boolean | null;
	lastCheckedAt: string | null;
	lastUpdatedAt: string | null;
	lastError: string | null;
}

export interface AutoUpdatedSkill {
	name: string;
	agent: string;
	scope: string;
	oldVersion: string;
	newVersion: string;
}

export interface AutoUpdateSkippedSkill {
	name: string;
	agent: string;
	scope: string;
	version: string;
	reason: string;
}

export interface AutoUpdateRunSummary {
	checked: number;
	updated: number;
	skipped: number;
	failed: number;
	updatedSkills: AutoUpdatedSkill[];
	skippedSkills: AutoUpdateSkippedSkill[];
}

export interface ProposalActor {
	id: string;
	name: string | null;
	email: string | null;
}

export interface ProposalSummary {
	id: string;
	skillId: string;
	title: string;
	intent: string;
	baseVersion: string;
	status: "open" | "selected" | "rejected" | "published" | "needs_update" | string;
	createdAt: string;
	updatedAt: string;
	createdBy: ProposalActor;
	reviewedAt: string | null;
	reviewedBy: ProposalActor | null;
	rejectionReason: string | null;
	publishedVersion: string | null;
}

export interface ProposalDetail extends ProposalSummary {
	skillMdContent: string;
}

// Local skills
export interface LocalSkill {
	name: string;
	version: string;
	description: string;
	tags: string[];
	path: string;
	agent: AgentType;
	scope: ScopeType;
	content_hash: string;
	modified_at: string | null;
	env_vars?: EnvVarDecl[];
}

export type SyncStatus =
	| "managed_synced"
	| "managed_update_available"
	| "managed_modified_locally"
	| "managed_auto_update_disabled"
	| "local_only";

export interface LocalSkillWithSync extends LocalSkill {
	syncStatus: SyncStatus;
	serverVersion?: string;
}
