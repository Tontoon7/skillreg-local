import { EnvVarSetupDialog } from "@/components/EnvVarSetupDialog";
import { ValidationBadge } from "@/components/ValidationBadge";
import { AgentAvailability } from "@/components/skills/AgentAvailability";
import {
	SkillPrimaryAction,
	type SkillPrimaryActionKind,
} from "@/components/skills/SkillPrimaryAction";
import { Button } from "@/components/ui/button";
import { getSkill } from "@/lib/api";
import { useAuthStore, useConfigStore, useManagedSkillsStore } from "@/lib/store";
import type { EnvVarDecl, SkillDetail as SkillDetailType } from "@/lib/types";
import {
	AlertTriangle,
	ArrowLeft,
	Building2,
	Database,
	Lightbulb,
	ShieldCheck,
} from "lucide-react";
import React, { useEffect, useMemo, useState } from "react";
import Markdown from "react-markdown";
import { useNavigate, useParams, useSearchParams } from "react-router";
import rehypeSanitize from "rehype-sanitize";

function splitFrontmatter(content: string): { body: string } {
	const match = content.match(/^---\s*\n[\s\S]*?\n---\s*\n?([\s\S]*)$/);
	return { body: match?.[1] ?? content };
}

class ErrorBoundary extends React.Component<{ children: React.ReactNode }, { failed: boolean }> {
	state = { failed: false };

	static getDerivedStateFromError() {
		return { failed: true };
	}

	render() {
		if (this.state.failed) {
			return (
				<div className="flex h-full flex-col items-center justify-center gap-3 p-6 text-center">
					<AlertTriangle className="size-8 text-destructive" />
					<h1 className="text-lg font-semibold">Cette skill ne peut pas être affichée</h1>
					<p className="text-sm text-muted-foreground">
						Revenez au catalogue puis réessayez dans quelques instants.
					</p>
				</div>
			);
		}
		return this.props.children;
	}
}

export function SkillDetailPage() {
	return (
		<ErrorBoundary>
			<SkillDetail />
		</ErrorBoundary>
	);
}

function SkillDetail() {
	const { name } = useParams<{ name: string }>();
	const [searchParams] = useSearchParams();
	const consumerOrg = useConfigStore((state) => state.config.org);
	const user = useAuthStore((state) => state.user);
	const overview = useManagedSkillsStore((state) => state.overview);
	const model = useManagedSkillsStore((state) => state.model);
	const installingKeys = useManagedSkillsStore((state) => state.installingKeys);
	const refresh = useManagedSkillsStore((state) => state.refresh);
	const install = useManagedSkillsStore((state) => state.install);
	const updateNow = useManagedSkillsStore((state) => state.updateNow);
	const navigate = useNavigate();
	const sourceOrg = searchParams.get("source") || consumerOrg;
	const [skill, setSkill] = useState<SkillDetailType | null>(null);
	const [loading, setLoading] = useState(true);
	const [error, setError] = useState<string | null>(null);
	const [actionError, setActionError] = useState<string | null>(null);
	const [localState, setLocalState] = useState<"ready" | "configure" | null>(null);
	const [pendingEnvVars, setPendingEnvVars] = useState<EnvVarDecl[]>([]);
	const [showEnvDialog, setShowEnvDialog] = useState(false);

	useEffect(() => {
		if (!sourceOrg || !name) return;
		setLoading(true);
		setError(null);
		getSkill(sourceOrg, name)
			.then(setSkill)
			.catch((loadError) =>
				setError(typeof loadError === "string" ? loadError : "Cette skill n’est pas disponible."),
			)
			.finally(() => setLoading(false));
		void refresh();
	}, [name, refresh, sourceOrg]);

	const parsedBody = useMemo(() => {
		const content = skill?.latestVersionData?.skillMdContent;
		return content ? splitFrontmatter(content).body : "";
	}, [skill?.latestVersionData?.skillMdContent]);

	const installedRow = model?.skills.find(
		(row) => row.name === name && row.sourceOrg === sourceOrg,
	);
	const overviewInstallation = overview?.installations.find(
		(item) => item.installation.skillName === name && item.installation.sourceOrg === sourceOrg,
	);
	const detectedAgents =
		overview?.agents.filter((agent) => agent.state === "detected").map((agent) => agent.agent) ??
		[];
	const installingKey =
		consumerOrg && sourceOrg && name ? `${consumerOrg}/${sourceOrg}/${name}` : "";
	const action = resolvePrimaryAction({
		localState,
		installedAction: installedRow?.primaryAction,
		installing: installingKeys.includes(installingKey),
	});
	const needsConfiguration =
		localState === "configure" || installedRow?.primaryAction === "configure";

	const handlePrimaryAction = async () => {
		if (!consumerOrg || !sourceOrg || !name) return;
		setActionError(null);
		try {
			if (action === "install") {
				const result = await install({
					consumerOrg,
					sourceOrg,
					name,
				});
				if (result.requiredEnvVars.length > 0) {
					setPendingEnvVars(result.requiredEnvVars);
					setLocalState("configure");
					setShowEnvDialog(true);
				} else {
					setLocalState("ready");
				}
				return;
			}
			if (action === "configure") {
				const envVars = overviewInstallation?.missingEnvVars ?? pendingEnvVars;
				if (envVars.length > 0) {
					setPendingEnvVars(envVars);
					setShowEnvDialog(true);
				} else {
					navigate(`/env?skill=${encodeURIComponent(name)}`);
				}
				return;
			}
			if (action === "update") {
				await updateNow();
				return;
			}
			if (action === "repair") {
				navigate(`/installed?skill=${encodeURIComponent(name)}`);
			}
		} catch (primaryError) {
			setActionError(
				typeof primaryError === "string" ? primaryError : "Cette action n’a pas pu être effectuée.",
			);
		}
	};

	if (loading) {
		return <SkillDetailSkeleton />;
	}

	if (error || !skill) {
		return (
			<div className="flex h-full flex-col items-center justify-center gap-3 p-6 text-center">
				<AlertTriangle className="size-8 text-destructive" />
				<p className="text-sm text-destructive">{error || "Skill introuvable"}</p>
				<Button variant="outline" size="sm" onClick={() => navigate("/catalog")}>
					<ArrowLeft className="size-4" />
					Retour au catalogue
				</Button>
			</div>
		);
	}

	const sourceName =
		user?.orgs.find((organization) => organization.slug === sourceOrg)?.name ??
		sourceOrg ??
		"Source autorisée";

	return (
		<div className="mx-auto flex w-full max-w-5xl flex-col gap-6 p-6">
			<Button variant="ghost" size="sm" className="w-fit" onClick={() => navigate("/catalog")}>
				<ArrowLeft className="size-4" />
				Retour au catalogue
			</Button>

			<header className="panel-raised flex flex-col gap-5 rounded-xl p-6 sm:flex-row sm:items-start">
				<div className="min-w-0 flex-1 space-y-3">
					<div className="flex flex-wrap items-center gap-2">
						<h1 className="text-2xl font-semibold">{skill.name}</h1>
						<ValidationBadge level={skill.latestVersionData?.validationLevel} />
						{needsConfiguration && (
							<span className="rounded-full border border-primary/30 bg-primary/10 px-2.5 py-1 text-xs font-medium text-primary">
								À configurer
							</span>
						)}
					</div>
					<p className="max-w-2xl text-base text-secondary-foreground">
						{skill.description || "Une skill approuvée pour vous aider dans votre travail."}
					</p>
					<div className="flex flex-wrap gap-x-4 gap-y-1 text-xs text-muted-foreground">
						<span className="inline-flex items-center gap-1.5">
							<Building2 className="size-3.5" />
							{sourceName}
						</span>
						<span className="inline-flex items-center gap-1.5">
							<ShieldCheck className="size-3.5" />
							Approuvée par votre entreprise
						</span>
					</div>
					{skill.isDeprecated && (
						<p className="rounded-lg border border-primary/30 bg-primary/5 p-3 text-sm text-primary">
							{skill.deprecatedMessage ||
								"Cette skill n’est plus recommandée. Contactez votre administrateur."}
						</p>
					)}
				</div>
				<div className="shrink-0">
					<SkillPrimaryAction
						kind={action}
						skillName={skill.name}
						onAction={() => void handlePrimaryAction()}
					/>
				</div>
			</header>

			{actionError && (
				<p
					className="rounded-lg border border-destructive/30 bg-destructive/5 p-3 text-sm text-destructive"
					role="alert"
				>
					{actionError}
				</p>
			)}

			<div className="grid gap-5 lg:grid-cols-[minmax(0,1fr)_18rem]">
				<div className="space-y-5">
					<section className="panel-inset rounded-xl p-6" aria-labelledby="outcomes-title">
						<h2 id="outcomes-title" className="mb-4 text-lg font-semibold">
							Ce que vous pouvez faire
						</h2>
						{parsedBody.trim() ? (
							<div className="prose max-w-none">
								<Markdown rehypePlugins={[rehypeSanitize]}>{parsedBody}</Markdown>
							</div>
						) : (
							<p className="text-sm text-secondary-foreground">
								{skill.description ||
									"Utilisez cette skill depuis votre assistant pour accomplir la tâche décrite."}
							</p>
						)}
					</section>

					<section className="panel-inset rounded-xl p-6" aria-labelledby="examples-title">
						<div className="mb-4 flex items-center gap-2">
							<Lightbulb className="size-4 text-primary" />
							<h2 id="examples-title" className="text-lg font-semibold">
								Exemples de demandes
							</h2>
						</div>
						<ul className="space-y-2 text-sm text-secondary-foreground">
							<li className="rounded-lg bg-surface px-4 py-3">
								« Utilise {skill.name} pour m’aider à obtenir ce résultat. »
							</li>
							<li className="rounded-lg bg-surface px-4 py-3">
								« Guide-moi étape par étape et résume les prochaines actions. »
							</li>
						</ul>
					</section>
				</div>

				<aside className="space-y-5">
					<section className="panel-inset rounded-xl p-5" aria-labelledby="availability-title">
						<h2 id="availability-title" className="mb-3 text-sm font-semibold">
							Vos assistants
						</h2>
						<AgentAvailability availableAgents={detectedAgents} />
					</section>

					<section className="panel-inset rounded-xl p-5" aria-labelledby="access-title">
						<div className="mb-3 flex items-center gap-2">
							<Database className="size-4 text-primary" />
							<h2 id="access-title" className="text-sm font-semibold">
								Données ou accès requis
							</h2>
						</div>
						<p className="text-sm text-muted-foreground">
							SkillReg vous indiquera après l’installation si un accès est nécessaire. Les valeurs
							restent stockées sur cet ordinateur.
						</p>
					</section>
				</aside>
			</div>

			{showEnvDialog && consumerOrg && name && (
				<EnvVarSetupDialog
					skillName={name}
					org={consumerOrg}
					envVars={pendingEnvVars}
					onClose={() => setShowEnvDialog(false)}
					onSaved={() => {
						setShowEnvDialog(false);
						setLocalState("ready");
						void refresh();
					}}
				/>
			)}
		</div>
	);
}

function resolvePrimaryAction({
	localState,
	installedAction,
	installing,
}: {
	localState: "ready" | "configure" | null;
	installedAction: "configure" | "repair" | "update" | "none" | undefined;
	installing: boolean;
}): SkillPrimaryActionKind {
	if (installing) return "installing";
	if (localState === "configure") return "configure";
	if (localState === "ready") return "installed";
	if (!installedAction) return "install";
	if (installedAction === "none") return "installed";
	return installedAction;
}

function SkillDetailSkeleton() {
	return (
		<div className="mx-auto flex w-full max-w-5xl animate-pulse flex-col gap-5 p-6 motion-reduce:animate-none">
			<div className="h-9 w-36 rounded-lg bg-muted" />
			<div className="h-44 rounded-xl bg-card" />
			<div className="grid gap-5 lg:grid-cols-[minmax(0,1fr)_18rem]">
				<div className="h-80 rounded-xl bg-card" />
				<div className="h-56 rounded-xl bg-card" />
			</div>
		</div>
	);
}
