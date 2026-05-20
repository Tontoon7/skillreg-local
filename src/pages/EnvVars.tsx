import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
	deleteEnvVars,
	deleteOrgEnvVar,
	getOrgEnvVar,
	listOrgEnvVars,
	migrateLegacyEnvVars,
	migrateOrgEnvFileToSecureStore,
	previewLegacyEnvMigration,
	scanLocalSkills,
	setOrgEnvVar,
} from "@/lib/api";
import {
	type EnvMigrationSummary,
	type EnvStoredVariable,
	type EnvVariableInventoryItem,
	type OrgEnvVariable,
	buildEnvInventory,
} from "@/lib/env-inventory";
import { useConfigStore } from "@/lib/store";
import type { LocalSkill } from "@/lib/types";
import { cn } from "@/lib/utils";
import {
	AlertCircle,
	CheckCircle2,
	Eye,
	EyeOff,
	Info,
	Key,
	Loader2,
	Pencil,
	Plus,
	RefreshCw,
	Save,
	ShieldAlert,
	Trash2,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";

const ENV_NAME_PATTERN = /^[A-Z][A-Z0-9_]{2,}$/;

function normalizeName(name: string): string {
	return name.trim().toUpperCase();
}

function isValidEnvName(name: string): boolean {
	return ENV_NAME_PATTERN.test(normalizeName(name));
}

function maskValue(value: string): string {
	if (!value) return "";
	return "********";
}

function formatSkillRefs(variable: EnvVariableInventoryItem): string {
	const required = variable.requiredBy.map((skill) => skill.skillName);
	const optional = variable.optionalFor.map((skill) => skill.skillName);
	const names = [...new Set([...required, ...optional])].sort((a, b) => a.localeCompare(b));

	if (names.length === 0) return "No installed skill references this variable";
	if (names.length <= 3) return names.join(", ");
	return `${names.slice(0, 3).join(", ")} +${names.length - 3}`;
}

function configuredLabel(variable: EnvVariableInventoryItem): string {
	if (variable.configuredSource === "org") {
		return `Stored once for this org${variable.storageBackend ? ` in ${storageBackendLabel(variable.storageBackend)}` : ""}`;
	}
	if (variable.configuredSource === "legacy") return "Legacy value detected";
	return "Not set";
}

function storageBackendLabel(storage: string): string {
	switch (storage) {
		case "macos_keychain":
			return "macOS Keychain";
		case "windows_credential_manager":
			return "Windows Credential Manager";
		case "secret_service":
			return "Secret Service";
		case "secure_store":
			return "secure store";
		case "fallback_file":
			return "fallback file";
		default:
			return storage.replaceAll("_", " ");
	}
}

function SectionHeader({
	title,
	count,
	description,
}: {
	title: string;
	count?: number;
	description: string;
}) {
	return (
		<div className="flex items-end justify-between gap-3">
			<div>
				<h2 className="text-sm font-semibold">{title}</h2>
				<p className="text-xs text-muted-foreground">{description}</p>
			</div>
			{typeof count === "number" && (
				<Badge variant="outline" className="text-[11px]">
					{count}
				</Badge>
			)}
		</div>
	);
}

function EmptySection({ label }: { label: string }) {
	return (
		<div className="rounded-xl border border-dashed bg-muted/20 px-4 py-5 text-sm text-muted-foreground">
			{label}
		</div>
	);
}

export function EnvVars() {
	const org = useConfigStore((s) => s.config.org);
	const [skills, setSkills] = useState<LocalSkill[]>([]);
	const [orgEnvVars, setOrgEnvVars] = useState<OrgEnvVariable[]>([]);
	const [migrationSummary, setMigrationSummary] = useState<EnvMigrationSummary | null>(null);
	const [loading, setLoading] = useState(true);
	const [migrating, setMigrating] = useState(false);
	const [secureMigrating, setSecureMigrating] = useState(false);
	const [savingKey, setSavingKey] = useState<string | null>(null);
	const [newKey, setNewKey] = useState("");
	const [newValue, setNewValue] = useState("");
	const [addingVariable, setAddingVariable] = useState(false);
	const [error, setError] = useState<string | null>(null);

	const load = useCallback(async () => {
		if (!org) return;
		setLoading(true);
		setError(null);
		try {
			const [orgVars, migration, installed] = await Promise.all([
				listOrgEnvVars(org),
				previewLegacyEnvMigration(org),
				scanLocalSkills(),
			]);
			setOrgEnvVars(orgVars);
			setMigrationSummary(migration);
			setSkills(installed);
		} catch (e) {
			setError(typeof e === "string" ? e : "Failed to load environment inventory");
		} finally {
			setLoading(false);
		}
	}, [org]);

	useEffect(() => {
		load();
	}, [load]);

	const inventory = useMemo(() => {
		if (!org) return null;
		return buildEnvInventory({
			org,
			installedSkills: skills,
			orgEnvVars,
			legacyVariables: migrationSummary?.legacyVariables ?? [],
		});
	}, [org, skills, orgEnvVars, migrationSummary]);

	const fallbackOrgVars = useMemo(
		() => orgEnvVars.filter((variable) => variable.storage === "fallback_file"),
		[orgEnvVars],
	);

	const saveVariable = async (variable: EnvVariableInventoryItem, value: string) => {
		if (!org) return;
		const key = normalizeName(variable.name);
		setSavingKey(key);
		try {
			await setOrgEnvVar(org, key, value.trim());
			await load();
		} finally {
			setSavingKey(null);
		}
	};

	const addOrgVariable = async () => {
		if (!org) return;
		const key = normalizeName(newKey);
		if (!isValidEnvName(key)) {
			setError("Use an uppercase env name with at least 3 characters, for example OPENAI_API_KEY.");
			return;
		}
		if (!newValue.trim()) {
			setError("Enter a value before adding the variable.");
			return;
		}

		setAddingVariable(true);
		setSavingKey(key);
		setError(null);
		try {
			await setOrgEnvVar(org, key, newValue.trim());
			setNewKey("");
			setNewValue("");
			await load();
		} catch (e) {
			setError(typeof e === "string" ? e : "Failed to add environment variable");
		} finally {
			setSavingKey(null);
			setAddingVariable(false);
		}
	};

	const deleteVariable = async (variable: EnvVariableInventoryItem | EnvStoredVariable) => {
		if (!org) return;
		const key = normalizeName(variable.name);
		setSavingKey(key);
		try {
			if (variable.storage === "legacy") {
				await Promise.all(variable.storedInSkills.map((skill) => deleteEnvVars(org, skill, [key])));
			} else {
				await deleteOrgEnvVar(org, key);
			}
			await load();
		} finally {
			setSavingKey(null);
		}
	};

	const revealOrgValue = async (name: string) => {
		if (!org) return "";
		return (await getOrgEnvVar(org, normalizeName(name))) ?? "";
	};

	const migrateSafeLegacyValues = async () => {
		if (!org) return;
		setMigrating(true);
		setError(null);
		try {
			await migrateLegacyEnvVars(org);
			await load();
		} catch (e) {
			setError(typeof e === "string" ? e : "Failed to migrate legacy variables");
		} finally {
			setMigrating(false);
		}
	};

	const migrateFallbackFileValues = async () => {
		if (!org) return;
		setSecureMigrating(true);
		setError(null);
		try {
			const summary = await migrateOrgEnvFileToSecureStore(org);
			if (summary.failed.length > 0) {
				setError(
					`Secure store migration failed for ${summary.failed
						.map((failure) => failure.name)
						.join(", ")}`,
				);
			}
			await load();
		} catch (e) {
			setError(typeof e === "string" ? e : "Failed to migrate variables to secure store");
		} finally {
			setSecureMigrating(false);
		}
	};

	if (!org) {
		return (
			<div className="flex h-full flex-col items-center justify-center gap-3">
				<Key className="size-10 text-muted-foreground" />
				<p className="text-muted-foreground">Select an organization first</p>
			</div>
		);
	}

	if (loading) {
		return (
			<div className="flex h-full items-center justify-center">
				<Loader2 className="size-6 animate-spin text-muted-foreground" />
			</div>
		);
	}

	if (error || !inventory) {
		return (
			<div className="flex h-full flex-col items-center justify-center gap-3">
				<p className="text-sm text-destructive">{error || "Environment inventory unavailable"}</p>
				<Button variant="outline" size="sm" onClick={load}>
					Retry
				</Button>
			</div>
		);
	}

	return (
		<div className="flex max-w-5xl flex-col gap-5 p-6">
			<div className="flex items-center justify-between gap-4">
				<div>
					<h1 className="text-lg font-semibold">Environment Variables</h1>
					<p className="text-sm text-muted-foreground">
						{inventory.variables.length} referenced, {inventory.configured.length} configured,{" "}
						{inventory.orphanedStoredVariables.length} unused stored
					</p>
				</div>
				<Button variant="outline" size="sm" onClick={load} disabled={loading}>
					<RefreshCw className={cn("size-3.5", loading && "animate-spin")} />
					Refresh
				</Button>
			</div>

			<div
				className={cn(
					"rounded-xl border px-4 py-3",
					fallbackOrgVars.length > 0
						? "border-amber-500/20 bg-amber-500/5"
						: "border-emerald-500/20 bg-emerald-500/5",
				)}
			>
				<div className="flex items-start gap-2">
					<ShieldAlert
						className={cn(
							"mt-0.5 size-4",
							fallbackOrgVars.length > 0 ? "text-amber-400" : "text-emerald-400",
						)}
					/>
					<div className="flex flex-1 flex-wrap items-center justify-between gap-3">
						<p className="text-xs text-muted-foreground">
							{fallbackOrgVars.length > 0 ? (
								<>
									{fallbackOrgVars.length} org-level value
									{fallbackOrgVars.length > 1 ? "s are" : " is"} still in the local fallback file.
									New saves use the OS secure store when available.
								</>
							) : (
								<>Org-level values are stored in the OS secure store when available.</>
							)}
						</p>
						{fallbackOrgVars.length > 0 && (
							<Button
								variant="outline"
								size="sm"
								onClick={migrateFallbackFileValues}
								disabled={secureMigrating}
							>
								{secureMigrating ? (
									<Loader2 className="size-3.5 animate-spin" />
								) : (
									<Save className="size-3.5" />
								)}
								Move to secure store
							</Button>
						)}
					</div>
				</div>
			</div>

			<AddOrgVariableSection
				keyName={newKey}
				value={newValue}
				adding={addingVariable}
				canAdd={isValidEnvName(newKey) && newValue.trim().length > 0}
				onKeyChange={setNewKey}
				onValueChange={setNewValue}
				onAdd={addOrgVariable}
			/>

			<EnvSection
				title="Needs attention"
				description="Required variables declared by installed skills but not configured locally."
				variables={inventory.needsAttention}
				emptyLabel="No required variables are missing."
				tone="missing"
				savingKey={savingKey}
				onSave={saveVariable}
				onDelete={deleteVariable}
				onReveal={revealOrgValue}
			/>

			<EnvSection
				title="Configured"
				description="Variables available from the org-level store or compatible legacy files."
				variables={inventory.configured}
				emptyLabel="No variables are configured yet."
				tone="configured"
				savingKey={savingKey}
				onSave={saveVariable}
				onDelete={deleteVariable}
				onReveal={revealOrgValue}
			/>

			<EnvSection
				title="Optional"
				description="Optional variables declared by installed skills and not configured yet."
				variables={inventory.optional}
				emptyLabel="No optional variables are declared by installed skills."
				tone="optional"
				savingKey={savingKey}
				onSave={saveVariable}
				onDelete={deleteVariable}
				onReveal={revealOrgValue}
			/>

			<LegacyStatusSection
				summary={migrationSummary}
				migrating={migrating}
				onMigrate={migrateSafeLegacyValues}
			/>

			<UnusedStoredSection
				variables={inventory.orphanedStoredVariables}
				savingKey={savingKey}
				onDelete={deleteVariable}
			/>
		</div>
	);
}

function AddOrgVariableSection({
	keyName,
	value,
	adding,
	canAdd,
	onKeyChange,
	onValueChange,
	onAdd,
}: {
	keyName: string;
	value: string;
	adding: boolean;
	canAdd: boolean;
	onKeyChange: (value: string) => void;
	onValueChange: (value: string) => void;
	onAdd: () => Promise<void>;
}) {
	return (
		<section className="space-y-2">
			<SectionHeader
				title="Add org-level variable"
				description="Store a variable once for this local organization, even before a skill references it."
			/>
			<div className="rounded-xl border bg-card p-4">
				<div className="grid gap-2 xl:grid-cols-[minmax(0,1fr)_minmax(0,1.25fr)_auto]">
					<Input
						value={keyName}
						onChange={(event) => onKeyChange(normalizeName(event.target.value))}
						placeholder="OPENAI_API_KEY"
						className="h-9 font-mono text-xs"
						aria-label="Environment variable name"
					/>
					<Input
						value={value}
						onChange={(event) => onValueChange(event.target.value)}
						onKeyDown={(event) => {
							if (event.key === "Enter" && canAdd && !adding) {
								void onAdd();
							}
						}}
						type="password"
						placeholder="value"
						className="h-9 font-mono text-xs"
						aria-label="Environment variable value"
					/>
					<Button size="sm" className="h-9" disabled={adding || !canAdd} onClick={onAdd}>
						{adding ? <Loader2 className="size-3.5 animate-spin" /> : <Plus className="size-3.5" />}
						Add
					</Button>
				</div>
			</div>
		</section>
	);
}

function EnvSection({
	title,
	description,
	variables,
	emptyLabel,
	tone,
	savingKey,
	onSave,
	onDelete,
	onReveal,
}: {
	title: string;
	description: string;
	variables: EnvVariableInventoryItem[];
	emptyLabel: string;
	tone: "missing" | "configured" | "optional";
	savingKey: string | null;
	onSave: (variable: EnvVariableInventoryItem, value: string) => Promise<void>;
	onDelete: (variable: EnvVariableInventoryItem) => Promise<void>;
	onReveal: (name: string) => Promise<string>;
}) {
	return (
		<section className="space-y-2">
			<SectionHeader title={title} count={variables.length} description={description} />
			{variables.length === 0 ? (
				<EmptySection label={emptyLabel} />
			) : (
				<div className="overflow-hidden rounded-xl border bg-card">
					{variables.map((variable) => (
						<EnvVariableRow
							key={variable.name}
							variable={variable}
							tone={tone}
							saving={savingKey === variable.name}
							onSave={(value) => onSave(variable, value)}
							onDelete={() => onDelete(variable)}
							onReveal={() => onReveal(variable.name)}
						/>
					))}
				</div>
			)}
		</section>
	);
}

function EnvVariableRow({
	variable,
	tone,
	saving,
	onSave,
	onDelete,
	onReveal,
}: {
	variable: EnvVariableInventoryItem;
	tone: "missing" | "configured" | "optional";
	saving: boolean;
	onSave: (value: string) => Promise<void>;
	onDelete: () => Promise<void>;
	onReveal: () => Promise<string>;
}) {
	const [editing, setEditing] = useState(!variable.configured && variable.requiredBy.length > 0);
	const [draft, setDraft] = useState(variable.defaultValue || "");
	const [revealed, setRevealed] = useState(false);
	const [revealedValue, setRevealedValue] = useState("");
	const [loadingValue, setLoadingValue] = useState(false);
	const displayValue = revealed ? revealedValue : maskValue("configured");
	const canReveal = variable.configuredSource === "org";

	const badgeClass =
		tone === "missing"
			? "border-amber-500/30 bg-amber-500/10 text-amber-400"
			: tone === "configured"
				? "border-emerald-500/30 bg-emerald-500/10 text-emerald-400"
				: "border-blue-500/30 bg-blue-500/10 text-blue-400";
	const Icon = tone === "missing" ? AlertCircle : tone === "configured" ? CheckCircle2 : Info;

	const loadValue = async () => {
		if (!canReveal) return "";
		setLoadingValue(true);
		try {
			return await onReveal();
		} finally {
			setLoadingValue(false);
		}
	};

	const startEditing = async () => {
		const value = await loadValue();
		setDraft(value || variable.defaultValue || "");
		setEditing(true);
	};

	const toggleReveal = async () => {
		if (revealed) {
			setRevealed(false);
			setRevealedValue("");
			return;
		}
		const value = await loadValue();
		setRevealedValue(value);
		setRevealed(true);
	};

	return (
		<div className="grid grid-cols-1 gap-3 border-b px-4 py-3 last:border-0 lg:grid-cols-[1fr_auto]">
			<div className="min-w-0 space-y-1">
				<div className="flex flex-wrap items-center gap-2">
					<span className="font-mono text-sm font-medium">{variable.name}</span>
					<span
						className={cn(
							"inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-[11px] font-medium",
							badgeClass,
						)}
					>
						<Icon className="size-3" />
						{tone === "missing" ? "Missing" : tone === "configured" ? "Configured" : "Optional"}
					</span>
					{variable.secret && <Badge variant="outline">secret</Badge>}
					{variable.configuredSource === "legacy" && <Badge variant="outline">legacy</Badge>}
					{variable.storageBackend && (
						<Badge variant={variable.storageBackend === "fallback_file" ? "outline" : "secondary"}>
							{storageBackendLabel(variable.storageBackend)}
						</Badge>
					)}
					{variable.requiredBy.length > 0 && (
						<Badge variant="secondary">{variable.requiredBy.length} required</Badge>
					)}
					{variable.optionalFor.length > 0 && (
						<Badge variant="outline">{variable.optionalFor.length} optional</Badge>
					)}
				</div>
				{variable.description && (
					<p className="text-xs text-muted-foreground">{variable.description}</p>
				)}
				<p className="text-xs text-muted-foreground">
					Used by {formatSkillRefs(variable)} · {configuredLabel(variable)}
					{variable.storedInSkills.length > 0 &&
						` · legacy in ${variable.storedInSkills.join(", ")}`}
				</p>
			</div>

			<div className="flex w-full flex-wrap items-center justify-start gap-2 lg:w-auto lg:min-w-72 lg:justify-end">
				{editing ? (
					<>
						<Input
							value={draft}
							onChange={(event) => setDraft(event.target.value)}
							type={variable.secret ? "password" : "text"}
							placeholder={variable.defaultValue || variable.name}
							className="h-8 min-w-48 font-mono text-xs"
							autoFocus
						/>
						<Button
							size="sm"
							disabled={saving || !draft.trim()}
							onClick={async () => {
								await onSave(draft);
								setRevealed(false);
								setRevealedValue("");
								setEditing(false);
							}}
						>
							{saving ? (
								<Loader2 className="size-3.5 animate-spin" />
							) : (
								<Save className="size-3.5" />
							)}
							Save
						</Button>
					</>
				) : (
					<>
						<span className="min-w-32 truncate text-right font-mono text-xs text-muted-foreground">
							{variable.configured ? displayValue : "Not set"}
						</span>
						{variable.configured && canReveal && (
							<Button
								variant="ghost"
								size="icon"
								className="size-8"
								onClick={toggleReveal}
								disabled={loadingValue}
								title={revealed ? "Hide value" : "Reveal value"}
							>
								{loadingValue ? (
									<Loader2 className="size-3.5 animate-spin" />
								) : revealed ? (
									<EyeOff className="size-3.5" />
								) : (
									<Eye className="size-3.5" />
								)}
							</Button>
						)}
						<Button variant="outline" size="sm" disabled={saving} onClick={startEditing}>
							<Pencil className="size-3.5" />
							{variable.configured ? "Edit" : "Configure"}
						</Button>
						{variable.configured && (
							<Button
								variant="ghost"
								size="icon"
								className="size-8 text-destructive hover:text-destructive"
								disabled={saving}
								onClick={onDelete}
								title={
									variable.storage === "legacy" ? "Delete legacy value" : "Delete org-level value"
								}
							>
								{saving ? (
									<Loader2 className="size-3.5 animate-spin" />
								) : (
									<Trash2 className="size-3.5" />
								)}
							</Button>
						)}
					</>
				)}
			</div>
		</div>
	);
}

function LegacyStatusSection({
	summary,
	migrating,
	onMigrate,
}: {
	summary: EnvMigrationSummary | null;
	migrating: boolean;
	onMigrate: () => Promise<void>;
}) {
	const legacyCount = summary?.legacyVariables.length ?? 0;
	const migratable = summary?.migratable ?? [];
	const conflicts = summary?.conflicts ?? [];
	const backups =
		summary?.legacyVariables.filter((variable) => variable.status === "alreadyConfigured") ?? [];

	return (
		<section className="space-y-2">
			<SectionHeader
				title="Legacy files"
				count={legacyCount}
				description="Per-skill env files are read for compatibility and can be migrated safely."
			/>
			{legacyCount === 0 ? (
				<EmptySection label="No legacy per-skill env variables found." />
			) : (
				<div className="space-y-2 rounded-xl border bg-card p-4">
					{migratable.length > 0 && (
						<div className="flex flex-wrap items-center justify-between gap-3">
							<div>
								<p className="text-sm font-medium">{migratable.length} safe to migrate</p>
								<p className="text-xs text-muted-foreground">
									These variables have one identical value across legacy skill files.
								</p>
							</div>
							<Button size="sm" onClick={onMigrate} disabled={migrating}>
								{migrating ? (
									<Loader2 className="size-3.5 animate-spin" />
								) : (
									<Save className="size-3.5" />
								)}
								Migrate safe values
							</Button>
						</div>
					)}

					{conflicts.length > 0 && (
						<div className="space-y-2 rounded-lg border border-amber-500/20 bg-amber-500/5 p-3">
							<div className="flex items-center gap-2 text-amber-300">
								<AlertCircle className="size-4" />
								<p className="text-sm font-medium">Migration conflicts</p>
							</div>
							{conflicts.map((conflict) => (
								<div key={conflict.name} className="text-xs text-muted-foreground">
									<span className="font-mono text-foreground">{conflict.name}</span> has{" "}
									{conflict.valueCount} different saved values in {conflict.skills.join(", ")}.
								</div>
							))}
						</div>
					)}

					{backups.length > 0 && (
						<div className="space-y-1">
							<p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
								Legacy backups still present
							</p>
							<div className="flex flex-wrap gap-1.5">
								{backups.map((variable) => (
									<Badge key={variable.name} variant="outline" className="font-mono">
										{variable.name}
									</Badge>
								))}
							</div>
						</div>
					)}
				</div>
			)}
		</section>
	);
}

function UnusedStoredSection({
	variables,
	savingKey,
	onDelete,
}: {
	variables: EnvStoredVariable[];
	savingKey: string | null;
	onDelete: (variable: EnvStoredVariable) => Promise<void>;
}) {
	return (
		<section className="space-y-2">
			<SectionHeader
				title="Unused stored variables"
				count={variables.length}
				description="Local variables not referenced by installed SKILL.md declarations."
			/>
			{variables.length === 0 ? (
				<EmptySection label="No unused stored variables found." />
			) : (
				<div className="overflow-hidden rounded-xl border bg-card">
					{variables.map((variable) => (
						<UnusedStoredRow
							key={`${variable.storage}:${variable.name}`}
							variable={variable}
							saving={savingKey === variable.name}
							onDelete={() => onDelete(variable)}
						/>
					))}
				</div>
			)}
		</section>
	);
}

function UnusedStoredRow({
	variable,
	saving,
	onDelete,
}: {
	variable: EnvStoredVariable;
	saving: boolean;
	onDelete: () => Promise<void>;
}) {
	return (
		<div className="grid grid-cols-1 gap-3 border-b px-4 py-3 last:border-0 lg:grid-cols-[1fr_auto]">
			<div className="min-w-0 space-y-1">
				<div className="flex flex-wrap items-center gap-2">
					<span className="font-mono text-sm font-medium">{variable.name}</span>
					<Badge variant="outline">unused</Badge>
					<Badge variant="outline">{variable.storage === "org" ? "org-level" : "legacy"}</Badge>
					{variable.storageBackend && (
						<Badge variant="outline">{storageBackendLabel(variable.storageBackend)}</Badge>
					)}
				</div>
				<p className="text-xs text-muted-foreground">
					{variable.storage === "legacy"
						? `Stored in ${variable.storedInSkills.join(", ")}`
						: `Stored once for the selected organization${
								variable.storageBackend ? ` in ${storageBackendLabel(variable.storageBackend)}` : ""
							}`}
				</p>
			</div>
			<div className="flex w-full flex-wrap items-center justify-start gap-2 lg:w-auto lg:justify-end">
				<Button
					variant="ghost"
					size="icon"
					className="size-8 text-destructive hover:text-destructive"
					disabled={saving}
					onClick={onDelete}
					title={variable.storage === "legacy" ? "Delete legacy value" : "Delete org-level value"}
				>
					{saving ? <Loader2 className="size-3.5 animate-spin" /> : <Trash2 className="size-3.5" />}
				</Button>
			</div>
		</div>
	);
}
