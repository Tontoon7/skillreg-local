import { EnvVarSetupDialog } from "@/components/EnvVarSetupDialog";
import {
	type EmployeeCatalogSkill,
	EmployeeSkillCard,
} from "@/components/skills/EmployeeSkillCard";
import type { SkillPrimaryActionKind } from "@/components/skills/SkillPrimaryAction";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { getCatalogPolicy, listCatalogSkills, listSkills } from "@/lib/api";
import { useAuthStore, useConfigStore, useManagedSkillsStore } from "@/lib/store";
import type { EnvVarDecl } from "@/lib/types";
import { Library, Search, X } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useNavigate } from "react-router";

export function Catalog() {
	const org = useConfigStore((state) => state.config.org);
	const user = useAuthStore((state) => state.user);
	const model = useManagedSkillsStore((state) => state.model);
	const installingKeys = useManagedSkillsStore((state) => state.installingKeys);
	const refreshManaged = useManagedSkillsStore((state) => state.refresh);
	const install = useManagedSkillsStore((state) => state.install);
	const updateNow = useManagedSkillsStore((state) => state.updateNow);
	const navigate = useNavigate();
	const [skills, setSkills] = useState<EmployeeCatalogSkill[]>([]);
	const [query, setQuery] = useState("");
	const [loading, setLoading] = useState(true);
	const [error, setError] = useState<string | null>(null);
	const [pendingConfiguration, setPendingConfiguration] = useState<{
		skillName: string;
		envVars: EnvVarDecl[];
	} | null>(null);

	const activeOrganization = user?.orgs.find((organization) => organization.slug === org);

	const load = useCallback(async () => {
		if (!org) return;
		setLoading(true);
		setError(null);
		try {
			const [privateResult, policyResult] = await Promise.all([
				listSkills({ org, page: 1, limit: 200, sort: "updated" }),
				getCatalogPolicy(org).catch(() => null),
			]);
			const publicResult = policyResult?.canInstallFromCatalog
				? await listCatalogSkills({ page: 1, limit: 100 }).catch(() => null)
				: null;

			const entries = new Map<string, EmployeeCatalogSkill>();
			for (const skill of privateResult.skills) {
				const key = catalogKey(org, skill.name);
				entries.set(key, {
					key,
					name: skill.name,
					description: skill.description,
					sourceOrg: org,
					sourceName: activeOrganization?.name ?? "Votre entreprise",
					trustLabel: "Approuvée par votre entreprise",
					tags: skill.tags,
					detailHref: `/catalog/${encodeURIComponent(skill.name)}`,
				});
			}
			for (const skill of publicResult?.skills ?? []) {
				const key = catalogKey(skill.orgSlug, skill.name);
				if (entries.has(key)) continue;
				entries.set(key, {
					key,
					name: skill.name,
					description: skill.description,
					sourceOrg: skill.orgSlug,
					sourceName: skill.orgName,
					trustLabel: skill.isFirstParty ? "Éditée par SkillReg" : "Source autorisée",
					validationLevel: skill.validation.level,
					tags: skill.tags,
					detailHref: `/catalog/${encodeURIComponent(skill.name)}?source=${encodeURIComponent(
						skill.orgSlug,
					)}`,
				});
			}
			setSkills([...entries.values()]);
		} catch (loadError) {
			setError(
				typeof loadError === "string"
					? loadError
					: "Le catalogue n’est pas disponible pour le moment.",
			);
		} finally {
			setLoading(false);
		}
	}, [activeOrganization?.name, org]);

	useEffect(() => {
		void load();
		void refreshManaged();
	}, [load, refreshManaged]);

	const filteredSkills = useMemo(() => {
		const normalized = query.trim().toLocaleLowerCase("fr");
		if (!normalized) return skills;
		return skills.filter(
			(skill) =>
				skill.name.toLocaleLowerCase("fr").includes(normalized) ||
				skill.description?.toLocaleLowerCase("fr").includes(normalized) ||
				skill.tags.some((tag) => tag.toLocaleLowerCase("fr").includes(normalized)),
		);
	}, [query, skills]);

	const resolveAction = (skill: EmployeeCatalogSkill): SkillPrimaryActionKind => {
		const key = catalogKey(org ?? "", skill.sourceOrg, skill.name);
		if (installingKeys.includes(key)) return "installing";
		const installed = model?.skills.find(
			(row) => row.name === skill.name && row.sourceOrg === skill.sourceOrg,
		);
		if (!installed) return "install";
		switch (installed.primaryAction) {
			case "configure":
				return "configure";
			case "repair":
				return "repair";
			case "update":
				return "update";
			default:
				return "installed";
		}
	};

	const handleAction = async (skill: EmployeeCatalogSkill) => {
		if (!org) return;
		const action = resolveAction(skill);
		setError(null);
		try {
			if (action === "install") {
				const result = await install({
					consumerOrg: org,
					sourceOrg: skill.sourceOrg,
					name: skill.name,
				});
				if (result.requiredEnvVars.length > 0) {
					setPendingConfiguration({
						skillName: skill.name,
						envVars: result.requiredEnvVars,
					});
				}
				return;
			}
			if (action === "configure") {
				navigate(`/env?skill=${encodeURIComponent(skill.name)}`);
				return;
			}
			if (action === "repair") {
				navigate(`/installed?skill=${encodeURIComponent(skill.name)}`);
				return;
			}
			if (action === "update") {
				await updateNow();
			}
		} catch (actionError) {
			setError(
				typeof actionError === "string" ? actionError : "Cette action n’a pas pu être effectuée.",
			);
		}
	};

	if (!org) {
		return (
			<div className="flex h-full flex-col items-center justify-center gap-3 p-6 text-center">
				<Library className="size-9 text-muted-foreground" />
				<p className="text-sm text-muted-foreground">
					Choisissez d’abord votre entreprise dans les réglages.
				</p>
			</div>
		);
	}

	return (
		<div className="mx-auto flex w-full max-w-5xl flex-col gap-5 p-6">
			<header className="space-y-1">
				<h1 className="text-xl font-semibold">Catalogue</h1>
				<p className="text-sm text-muted-foreground">
					Ajoutez en un clic les skills approuvées pour votre travail.
				</p>
			</header>

			<div className="relative">
				<Search className="absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
				<Input
					type="search"
					value={query}
					onChange={(event) => setQuery(event.target.value)}
					placeholder="Que souhaitez-vous accomplir ?"
					aria-label="Rechercher une skill"
					className="h-11 pl-10 pr-10"
				/>
				{query && (
					<button
						type="button"
						aria-label="Effacer la recherche"
						onClick={() => setQuery("")}
						className="absolute right-2 top-1/2 flex size-8 -translate-y-1/2 items-center justify-center rounded-md text-muted-foreground hover:bg-secondary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
					>
						<X className="size-4" />
					</button>
				)}
			</div>

			{error && (
				<div
					className="flex items-center justify-between gap-4 rounded-lg border border-destructive/30 bg-destructive/5 px-4 py-3 text-sm text-destructive"
					role="alert"
				>
					<span>{error}</span>
					<Button variant="outline" size="sm" onClick={() => void load()}>
						Réessayer
					</Button>
				</div>
			)}

			{loading ? (
				<CatalogSkeleton />
			) : filteredSkills.length > 0 ? (
				<div className="space-y-3">
					{filteredSkills.map((skill) => (
						<EmployeeSkillCard
							key={skill.key}
							skill={skill}
							action={resolveAction(skill)}
							onAction={() => void handleAction(skill)}
						/>
					))}
				</div>
			) : (
				<div className="panel-inset flex min-h-52 flex-col items-center justify-center gap-2 rounded-xl p-8 text-center">
					<Library className="size-8 text-muted-foreground" />
					<p className="text-sm font-medium">
						{query ? "Aucune skill ne correspond à votre recherche" : "Le catalogue est vide"}
					</p>
					<p className="text-xs text-muted-foreground">
						{query
							? "Essayez de décrire le résultat attendu avec d’autres mots."
							: "Votre administrateur peut publier ou autoriser de nouvelles skills."}
					</p>
				</div>
			)}

			{pendingConfiguration && (
				<EnvVarSetupDialog
					skillName={pendingConfiguration.skillName}
					org={org}
					envVars={pendingConfiguration.envVars}
					onClose={() => setPendingConfiguration(null)}
					onSaved={() => {
						setPendingConfiguration(null);
						void refreshManaged();
					}}
				/>
			)}
		</div>
	);
}

function catalogKey(sourceOrg: string, name: string): string;
function catalogKey(consumerOrg: string, sourceOrg: string, name: string): string;
function catalogKey(first: string, second: string, third?: string): string {
	if (third) return `${first}/${second}/${third}`;
	return `${first}/${first}/${second}`;
}

function CatalogSkeleton() {
	return (
		<div className="space-y-3 animate-pulse motion-reduce:animate-none">
			<div className="h-36 rounded-xl bg-card" />
			<div className="h-36 rounded-xl bg-card" />
			<div className="h-36 rounded-xl bg-card" />
		</div>
	);
}
