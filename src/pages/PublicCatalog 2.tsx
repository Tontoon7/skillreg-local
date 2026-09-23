import { ValidationBadge } from "@/components/ValidationBadge";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Select } from "@/components/ui/select";
import { getCatalogPolicy, installCatalogSkill, listCatalogSkills } from "@/lib/api";
import { useConfigStore } from "@/lib/store";
import type { AgentType, CatalogPolicy, CatalogSkill, ScopeType } from "@/lib/types";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { Check, Download, Globe, Loader2, Search, ShieldOff } from "lucide-react";
import { useEffect, useState } from "react";

const AGENTS: AgentType[] = ["claude", "codex", "cursor"];
const SCOPES: ScopeType[] = ["user", "project"];

const MODE_HELP: Record<CatalogPolicy["mode"], string> = {
	blocked: "Your organization has disabled public catalog installs.",
	allowlist: "Only skills your administrator has approved can be installed.",
	validated_only: "Only skills meeting your organization's validation bar can be installed.",
	open: "Any published public skill can be installed.",
};

function SkillRow({
	skill,
	onInstall,
	installing,
	installed,
}: {
	skill: CatalogSkill;
	onInstall: (skill: CatalogSkill) => void;
	installing: boolean;
	installed: boolean;
}) {
	return (
		<div className="flex items-start gap-3 rounded-lg border border-border p-4">
			<div className="min-w-0 flex-1">
				<div className="flex flex-wrap items-center gap-2">
					<h3 className="truncate font-medium">{skill.name}</h3>
					<ValidationBadge level={skill.validation.level} />
					{skill.isFirstParty && (
						<Badge variant="outline" className="text-primary">
							Official
						</Badge>
					)}
				</div>
				<p className="truncate text-muted-foreground text-xs">
					@{skill.orgSlug} · v{skill.latestVersion} · {skill.totalDownloads} downloads
				</p>
				{skill.description && (
					<p className="mt-1 line-clamp-2 text-muted-foreground text-sm">{skill.description}</p>
				)}
			</div>

			<Button
				size="sm"
				variant={installed ? "secondary" : "default"}
				disabled={installing || installed}
				onClick={() => onInstall(skill)}
			>
				{installing ? (
					<Loader2 className="size-4 animate-spin" />
				) : installed ? (
					<Check className="size-4" />
				) : (
					<Download className="size-4" />
				)}
				{installed ? "Installed" : "Install"}
			</Button>
		</div>
	);
}

export function PublicCatalog() {
	const org = useConfigStore((s) => s.config?.org);
	const defaultAgent = useConfigStore((s) => s.config?.defaultAgent);
	const defaultScope = useConfigStore((s) => s.config?.defaultScope);

	const [policy, setPolicy] = useState<CatalogPolicy | null>(null);
	const [skills, setSkills] = useState<CatalogSkill[]>([]);
	const [query, setQuery] = useState("");
	const [loading, setLoading] = useState(true);
	const [error, setError] = useState<string | null>(null);
	const [installing, setInstalling] = useState<string | null>(null);
	const [installed, setInstalled] = useState<Set<string>>(new Set());
	const [agent, setAgent] = useState<AgentType>(defaultAgent ?? "claude");
	const [scope, setScope] = useState<ScopeType>(defaultScope ?? "user");

	useEffect(() => {
		if (!org) return;
		getCatalogPolicy(org)
			.then(setPolicy)
			.catch((e) => setError(String(e)));
	}, [org]);

	useEffect(() => {
		if (!policy?.canInstallFromCatalog) {
			setLoading(false);
			return;
		}
		setLoading(true);
		listCatalogSkills({ query, limit: 40 })
			.then((r) => setSkills(r.skills))
			.catch((e) => setError(String(e)))
			.finally(() => setLoading(false));
	}, [policy, query]);

	async function handleInstall(skill: CatalogSkill) {
		if (!org) return;
		const key = `${skill.orgSlug}/${skill.name}`;
		setInstalling(key);
		setError(null);

		try {
			let projectDir: string | undefined;
			if (scope === "project") {
				const picked = await openDialog({ directory: true, multiple: false });
				if (typeof picked !== "string") return;
				projectDir = picked;
			}

			await installCatalogSkill({
				sourceOrg: skill.orgSlug,
				name: skill.name,
				consumerOrg: org,
				agent,
				scope,
				projectDir,
			});
			setInstalled((prev) => new Set(prev).add(key));
		} catch (e) {
			setError(String(e));
		} finally {
			setInstalling(null);
		}
	}

	// The whole view is hidden when the org blocks catalog installs, so this
	// only shows if the policy changes while the page is open.
	if (policy && !policy.canInstallFromCatalog) {
		return (
			<div className="flex flex-col items-center justify-center gap-3 p-16 text-center">
				<ShieldOff className="size-8 text-muted-foreground" />
				<p className="font-medium">Public catalog disabled</p>
				<p className="max-w-sm text-muted-foreground text-sm">{MODE_HELP.blocked}</p>
			</div>
		);
	}

	return (
		<div className="p-6">
			<div className="mb-5 flex items-center gap-2">
				<Globe className="size-5 text-primary" />
				<h1 className="font-semibold text-lg">Public catalog</h1>
			</div>

			{policy && <p className="mb-4 text-muted-foreground text-sm">{MODE_HELP[policy.mode]}</p>}

			<div className="mb-5 flex flex-wrap items-center gap-2">
				<div className="relative min-w-56 flex-1">
					<Search className="-translate-y-1/2 absolute top-1/2 left-2.5 size-4 text-muted-foreground" />
					<Input
						value={query}
						onChange={(e) => setQuery(e.target.value)}
						placeholder="Search public skills…"
						className="pl-8"
					/>
				</div>
				<Select value={agent} onChange={(e) => setAgent(e.target.value as AgentType)}>
					{AGENTS.map((a) => (
						<option key={a} value={a}>
							{a}
						</option>
					))}
				</Select>
				<Select value={scope} onChange={(e) => setScope(e.target.value as ScopeType)}>
					{SCOPES.map((s) => (
						<option key={s} value={s}>
							{s}
						</option>
					))}
				</Select>
			</div>

			{error && (
				<p className="mb-4 rounded-lg border border-destructive/30 bg-destructive/10 p-3 text-destructive text-sm">
					{error}
				</p>
			)}

			{loading ? (
				<div className="flex justify-center p-12">
					<Loader2 className="size-6 animate-spin text-muted-foreground" />
				</div>
			) : skills.length === 0 ? (
				<p className="rounded-lg border border-border border-dashed p-10 text-center text-muted-foreground text-sm">
					{query ? "No skill matches your search." : "The catalog is being built."}
				</p>
			) : (
				<div className="space-y-3">
					{skills.map((skill) => {
						const key = `${skill.orgSlug}/${skill.name}`;
						return (
							<SkillRow
								key={key}
								skill={skill}
								onInstall={handleInstall}
								installing={installing === key}
								installed={installed.has(key)}
							/>
						);
					})}
				</div>
			)}
		</div>
	);
}
