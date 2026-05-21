import type { AgentType, EnvVarDecl, LocalSkill, ScopeType } from "./types";

const ENV_NAME_PATTERN = /^[A-Z][A-Z0-9_]{2,}$/;
const SECRET_NAME_PATTERN = /(?:_KEY|_TOKEN|_SECRET|_PASSWORD)$/;

export interface LegacySkillEnvVars {
	skill: string;
	vars: Record<string, string>;
}

export interface OrgEnvVariable {
	name: string;
	configured: boolean;
	updatedAt: string | null;
	storage: "fallback_file" | string;
}

export type LegacyEnvVariableStatus = "migratable" | "conflict" | "alreadyConfigured";

export interface LegacyEnvVariableSummary {
	name: string;
	configured: boolean;
	skills: string[];
	valueCount: number;
	status: LegacyEnvVariableStatus;
}

export interface LegacyMigrationItem {
	name: string;
	skills: string[];
}

export interface LegacyMigrationConflict {
	name: string;
	skills: string[];
	valueCount: number;
}

export interface EnvMigrationSummary {
	migrated: LegacyMigrationItem[];
	migratable: LegacyMigrationItem[];
	conflicts: LegacyMigrationConflict[];
	legacyVariables: LegacyEnvVariableSummary[];
}

export interface LegacyCleanupItem {
	name: string;
	skills: string[];
}

export interface LegacyCleanupSkipped {
	name: string;
	skills: string[];
	reason: "valueMismatch" | "notConfigured" | string;
}

export interface LegacyCleanupSummary {
	cleaned: LegacyCleanupItem[];
	removedFiles: string[];
	skipped: LegacyCleanupSkipped[];
}

export interface SecureStoreMigrationItem {
	name: string;
}

export interface SecureStoreMigrationFailure {
	name: string;
	reason: string;
}

export interface SecureStoreMigrationSummary {
	migrated: SecureStoreMigrationItem[];
	failed: SecureStoreMigrationFailure[];
}

export interface EnvSkillReference {
	skillName: string;
	agent: AgentType;
	scope: ScopeType;
	path: string;
	version: string;
}

export type EnvVariableSource = "declared" | "detected" | "mixed";
export type EnvConfiguredSource = "org" | "legacy" | "none";
export type EnvStorageSource = "org" | "legacy";

export interface EnvVariableInventoryItem {
	name: string;
	configured: boolean;
	configuredSource: EnvConfiguredSource;
	secret: boolean;
	requiredBy: EnvSkillReference[];
	optionalFor: EnvSkillReference[];
	detectedIn: EnvSkillReference[];
	description: string | null;
	defaultValue: string | null;
	source: EnvVariableSource;
	updatedAt: string | null;
	storedInSkills: string[];
	legacyStatus: LegacyEnvVariableStatus | null;
	storage: EnvStorageSource | null;
	storageBackend: string | null;
}

export interface EnvStoredVariable {
	name: string;
	configured: boolean;
	referencedByInstalledSkills: boolean;
	updatedAt: string | null;
	storedInSkills: string[];
	storage: EnvStorageSource;
	storageBackend: string | null;
	legacyStatus: LegacyEnvVariableStatus | null;
}

export interface EnvInventory {
	org: string;
	variables: EnvVariableInventoryItem[];
	needsAttention: EnvVariableInventoryItem[];
	configured: EnvVariableInventoryItem[];
	optional: EnvVariableInventoryItem[];
	orphanedStoredVariables: EnvStoredVariable[];
}

export type EnvReadinessStatus = "ready" | "missing" | "optional" | "not_required";

export interface EnvReadinessSummary {
	status: EnvReadinessStatus;
	missingCount: number;
}

interface StoredValueSummary {
	name: string;
	configured: boolean;
	configuredSource: EnvConfiguredSource;
	storedInSkills: string[];
	updatedAt: string | null;
	storage: EnvStorageSource;
	storageBackend: string | null;
	legacyStatus: LegacyEnvVariableStatus | null;
}

interface VariableAccumulator {
	name: string;
	secret: boolean;
	requiredBy: EnvSkillReference[];
	optionalFor: EnvSkillReference[];
	detectedIn: EnvSkillReference[];
	description: string | null;
	defaultValue: string | null;
}

interface BuildEnvInventoryInput {
	org: string;
	installedSkills: LocalSkill[];
	orgEnvVars: OrgEnvVariable[];
	legacyVariables: LegacyEnvVariableSummary[];
}

function normalizeName(name: string): string {
	return name.trim().toUpperCase();
}

function isConfiguredValue(value: string | undefined): boolean {
	return typeof value === "string" && value.trim().length > 0;
}

function inferSecret(declaration: EnvVarDecl): boolean {
	return declaration.secret ?? SECRET_NAME_PATTERN.test(normalizeName(declaration.name));
}

function toSkillReference(skill: LocalSkill): EnvSkillReference {
	return {
		skillName: skill.name,
		agent: skill.agent,
		scope: skill.scope,
		path: skill.path,
		version: skill.version,
	};
}

function compareByName(a: { name: string }, b: { name: string }): number {
	return a.name.localeCompare(b.name);
}

function getStoredValueSummaries(
	orgEnvVars: OrgEnvVariable[],
	legacyVariables: LegacyEnvVariableSummary[],
): Map<string, StoredValueSummary> {
	const stored = new Map<string, StoredValueSummary>();

	for (const variable of orgEnvVars) {
		const name = normalizeName(variable.name);
		if (!name || !variable.configured) continue;

		stored.set(name, {
			name,
			configured: true,
			configuredSource: "org",
			storedInSkills: [],
			updatedAt: variable.updatedAt,
			storage: "org",
			storageBackend: variable.storage,
			legacyStatus: null,
		});
	}

	for (const legacy of legacyVariables) {
		const name = normalizeName(legacy.name);
		if (!name) continue;

		const existing = stored.get(name);
		if (existing) {
			existing.storedInSkills = [...new Set([...existing.storedInSkills, ...legacy.skills])].sort(
				(a, b) => a.localeCompare(b),
			);
			existing.legacyStatus = legacy.status;
			continue;
		}

		const configured = legacy.configured && legacy.status !== "conflict";
		stored.set(name, {
			name,
			configured,
			configuredSource: configured ? "legacy" : "none",
			storedInSkills: [...legacy.skills].sort((a, b) => a.localeCompare(b)),
			updatedAt: null,
			storage: "legacy",
			storageBackend: null,
			legacyStatus: legacy.status,
		});
	}

	for (const value of stored.values()) {
		value.storedInSkills.sort((a, b) => a.localeCompare(b));
	}

	return stored;
}

function uniqueDeclarations(declarations: EnvVarDecl[]): EnvVarDecl[] {
	const byName = new Map<string, EnvVarDecl>();

	for (const declaration of declarations) {
		const name = normalizeName(declaration.name);
		if (!ENV_NAME_PATTERN.test(name) || byName.has(name)) continue;
		byName.set(name, { ...declaration, name });
	}

	return [...byName.values()];
}

export function getConfiguredEnvValue(
	storedEnvVars: LegacySkillEnvVars[],
	variableName: string,
): string {
	const name = normalizeName(variableName);

	for (const skillEnv of storedEnvVars) {
		for (const [rawName, value] of Object.entries(skillEnv.vars)) {
			if (normalizeName(rawName) === name && isConfiguredValue(value)) {
				return value;
			}
		}
	}

	return "";
}

export function buildEnvInventory({
	org,
	installedSkills,
	orgEnvVars,
	legacyVariables,
}: BuildEnvInventoryInput): EnvInventory {
	const stored = getStoredValueSummaries(orgEnvVars, legacyVariables);
	const byVariable = new Map<string, VariableAccumulator>();

	for (const skill of installedSkills) {
		const reference = toSkillReference(skill);

		for (const declaration of uniqueDeclarations(skill.env_vars ?? [])) {
			const name = normalizeName(declaration.name);
			const existing =
				byVariable.get(name) ??
				({
					name,
					secret: inferSecret(declaration),
					requiredBy: [],
					optionalFor: [],
					detectedIn: [],
					description: null,
					defaultValue: null,
				} satisfies VariableAccumulator);

			existing.secret = existing.secret || inferSecret(declaration);
			if (!existing.description && declaration.description) {
				existing.description = declaration.description;
			}
			if (!existing.defaultValue && declaration.default) {
				existing.defaultValue = declaration.default;
			}

			if (declaration.required !== false) {
				existing.requiredBy.push(reference);
			} else {
				existing.optionalFor.push(reference);
			}

			byVariable.set(name, existing);
		}
	}

	const variables = [...byVariable.values()]
		.map((variable) => {
			const storedValue = stored.get(variable.name);

			return {
				name: variable.name,
				configured: storedValue?.configured ?? false,
				configuredSource: storedValue?.configuredSource ?? "none",
				secret: variable.secret,
				requiredBy: variable.requiredBy,
				optionalFor: variable.optionalFor,
				detectedIn: variable.detectedIn,
				description: variable.description,
				defaultValue: variable.defaultValue,
				source: "declared" as const,
				updatedAt: storedValue?.updatedAt ?? null,
				storedInSkills: storedValue?.storedInSkills ?? [],
				legacyStatus: storedValue?.legacyStatus ?? null,
				storage: storedValue?.storage ?? null,
				storageBackend: storedValue?.storageBackend ?? null,
			};
		})
		.sort(compareByName);

	const referencedNames = new Set(variables.map((variable) => variable.name));
	const orphanedStoredVariables = [...stored.values()]
		.filter((variable) => !referencedNames.has(variable.name))
		.map((variable) => ({
			name: variable.name,
			configured: variable.configured,
			referencedByInstalledSkills: false,
			updatedAt: variable.updatedAt,
			storedInSkills: variable.storedInSkills,
			storage: variable.storage,
			storageBackend: variable.storageBackend,
			legacyStatus: variable.legacyStatus,
		}))
		.sort(compareByName);

	return {
		org,
		variables,
		needsAttention: variables
			.filter((variable) => !variable.configured && variable.requiredBy.length > 0)
			.sort(compareByName),
		configured: variables.filter((variable) => variable.configured).sort(compareByName),
		optional: variables
			.filter(
				(variable) =>
					!variable.configured &&
					variable.requiredBy.length === 0 &&
					variable.optionalFor.length > 0,
			)
			.sort(compareByName),
		orphanedStoredVariables,
	};
}

export function getEnvStatusForSkills(
	skills: LocalSkill[],
	inventory: EnvInventory,
): EnvReadinessSummary {
	const configuredNames = new Set(
		inventory.variables.filter((variable) => variable.configured).map((variable) => variable.name),
	);
	const requiredNames = new Set<string>();
	const optionalNames = new Set<string>();

	for (const skill of skills) {
		for (const declaration of uniqueDeclarations(skill.env_vars ?? [])) {
			const name = normalizeName(declaration.name);
			if (declaration.required !== false) {
				requiredNames.add(name);
			} else {
				optionalNames.add(name);
			}
		}
	}

	if (requiredNames.size === 0 && optionalNames.size === 0) {
		return { status: "not_required", missingCount: 0 };
	}

	const missingCount = [...requiredNames].filter((name) => !configuredNames.has(name)).length;
	if (missingCount > 0) {
		return { status: "missing", missingCount };
	}

	if (requiredNames.size > 0) {
		return { status: "ready", missingCount: 0 };
	}

	return { status: "optional", missingCount: 0 };
}
