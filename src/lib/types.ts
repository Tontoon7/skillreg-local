export type AgentType = "claude" | "codex" | "cursor";
export type ScopeType = "project" | "user";

// Config stored in ~/.skillreg/config.json
export interface SkillregConfig {
	token?: string;
	apiUrl?: string;
	org?: string;
	defaultAgent?: AgentType;
	defaultScope?: ScopeType;
	setupDone?: boolean;
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
	default?: string;
}

export interface InstallResult {
	name: string;
	version: string;
	path: string;
	filesCount: number;
	envVars: EnvVarDecl[];
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
}

export type SyncStatus = "synced" | "modified_locally" | "update_available" | "unknown";

export interface LocalSkillWithSync extends LocalSkill {
	syncStatus: SyncStatus;
	serverVersion?: string;
}
