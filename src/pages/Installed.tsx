import { AgentAvailability } from "@/components/skills/AgentAvailability";
import {
	SkillPrimaryAction,
	type SkillPrimaryActionKind,
} from "@/components/skills/SkillPrimaryAction";
import { UninstallManagedSkillDialog } from "@/components/skills/UninstallManagedSkillDialog";
import { Button, buttonVariants } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import type { EmployeeSkillRow } from "@/lib/employee-model";
import { useManagedSkillsStore } from "@/lib/store";
import type { ManagedOverviewInstallation } from "@/lib/types";
import { cn } from "@/lib/utils";
import {
	AlertTriangle,
	FolderOpen,
	Loader2,
	MoreHorizontal,
	PackageOpen,
	RefreshCw,
	Search,
	ShieldCheck,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { Link, useSearchParams } from "react-router";

type InstalledAction = "update" | "repair" | "uninstall";

export function Installed() {
	const model = useManagedSkillsStore((state) => state.model);
	const overview = useManagedSkillsStore((state) => state.overview);
	const loading = useManagedSkillsStore((state) => state.loading);
	const refreshing = useManagedSkillsStore((state) => state.refreshing);
	const error = useManagedSkillsStore((state) => state.error);
	const refresh = useManagedSkillsStore((state) => state.refresh);
	const updateNow = useManagedSkillsStore((state) => state.updateNow);
	const repair = useManagedSkillsStore((state) => state.repair);
	const uninstall = useManagedSkillsStore((state) => state.uninstall);
	const [searchParams] = useSearchParams();
	const [query, setQuery] = useState("");
	const [expandedSkill, setExpandedSkill] = useState<string | null>(searchParams.get("skill"));
	const [openMenu, setOpenMenu] = useState<string | null>(null);
	const [pendingUninstall, setPendingUninstall] = useState<EmployeeSkillRow | null>(null);
	const [busy, setBusy] = useState<InstalledAction | null>(null);
	const [actionError, setActionError] = useState<string | null>(null);

	useEffect(() => {
		void refresh();
	}, [refresh]);

	const filteredSkills = useMemo(() => {
		const normalized = query.trim().toLocaleLowerCase("fr");
		if (!normalized) return model?.skills ?? [];
		return (
			model?.skills.filter((skill) => skill.name.toLocaleLowerCase("fr").includes(normalized)) ?? []
		);
	}, [model?.skills, query]);

	const handleUpdate = async () => {
		setBusy("update");
		setActionError(null);
		try {
			await updateNow();
		} catch {
			setActionError("La mise à jour n’a pas pu être appliquée.");
		} finally {
			setBusy(null);
		}
	};

	const handleRepair = async (installationId: string) => {
		setBusy("repair");
		setActionError(null);
		try {
			await repair(installationId);
		} catch {
			setActionError("La réparation n’a pas pu aboutir.");
		} finally {
			setBusy(null);
		}
	};

	const handleUninstall = async () => {
		if (!pendingUninstall) return;
		const skill = pendingUninstall;
		setBusy("uninstall");
		setActionError(null);
		try {
			const result = await uninstall(skill.installationId);
			if (!result.removed && !result.alreadyRemoved) {
				setActionError(
					result.conflicts > 0
						? `${skill.name} n’a pas été désinstallée car une connexion locale a été modifiée. Aucun fichier n’a été supprimé.`
						: `${skill.name} n’a pas été désinstallée. Aucun fichier n’a été supprimé.`,
				);
			}
			setPendingUninstall(null);
		} catch {
			setPendingUninstall(null);
			setActionError(
				`La désinstallation de ${skill.name} n’a pas pu aboutir. Aucun fichier n’a été supprimé.`,
			);
		} finally {
			setBusy(null);
		}
	};

	if (loading && !model) {
		return <InstalledSkeleton />;
	}

	if (!model) {
		return (
			<div className="flex h-full flex-col items-center justify-center gap-3 p-6 text-center">
				<AlertTriangle className="size-8 text-destructive" />
				<p className="text-sm text-destructive">
					{error ?? "Vos installations ne sont pas disponibles."}
				</p>
				<Button variant="outline" onClick={() => void refresh()}>
					<RefreshCw className="size-4" />
					Réessayer
				</Button>
			</div>
		);
	}

	return (
		<div className="mx-auto flex w-full max-w-5xl flex-col gap-5 p-6">
			<header className="flex flex-wrap items-start justify-between gap-4">
				<div className="space-y-1">
					<h1 className="text-xl font-semibold">Mes skills</h1>
					<p className="text-sm text-muted-foreground">
						Une ligne par skill, quel que soit le nombre d’assistants connectés.
					</p>
				</div>
				<Button variant="outline" size="sm" onClick={() => void refresh()} disabled={refreshing}>
					{refreshing ? (
						<Loader2 className="size-3.5 animate-spin" />
					) : (
						<RefreshCw className="size-3.5" />
					)}
					Actualiser
				</Button>
			</header>

			{actionError && (
				<p
					className="rounded-lg border border-destructive/30 bg-destructive/5 p-3 text-sm text-destructive"
					role="alert"
				>
					{actionError}
				</p>
			)}

			{model.skills.length === 0 ? (
				<EmptyInstalled />
			) : (
				<>
					<div className="relative">
						<Search className="absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
						<Input
							type="search"
							value={query}
							onChange={(event) => setQuery(event.target.value)}
							placeholder="Rechercher dans mes skills"
							aria-label="Rechercher dans mes skills"
							className="pl-10"
						/>
					</div>

					{filteredSkills.length > 0 ? (
						<div className="space-y-3">
							{filteredSkills.map((skill) => {
								const installation = overview?.installations.find(
									(item) => item.installation.installationId === skill.installationId,
								);
								return (
									<InstalledSkillRow
										key={skill.installationId}
										skill={skill}
										installation={installation}
										expanded={expandedSkill === skill.name}
										busy={busy}
										onToggleDetails={() =>
											setExpandedSkill((current) => (current === skill.name ? null : skill.name))
										}
										onUpdate={() => void handleUpdate()}
										onRepair={() => void handleRepair(skill.installationId)}
										menuOpen={openMenu === skill.installationId}
										onToggleMenu={() =>
											setOpenMenu((current) =>
												current === skill.installationId ? null : skill.installationId,
											)
										}
										onUninstall={() => {
											setOpenMenu(null);
											setActionError(null);
											setPendingUninstall(skill);
										}}
									/>
								);
							})}
						</div>
					) : (
						<div className="panel-inset flex min-h-40 flex-col items-center justify-center gap-2 rounded-xl p-6 text-center">
							<FolderOpen className="size-7 text-muted-foreground" />
							<p className="text-sm text-muted-foreground">
								Aucune skill ne correspond à votre recherche.
							</p>
						</div>
					)}
				</>
			)}

			{pendingUninstall && (
				<UninstallManagedSkillDialog
					skillName={pendingUninstall.name}
					busy={busy === "uninstall"}
					onClose={() => {
						if (busy !== "uninstall") setPendingUninstall(null);
					}}
					onConfirm={() => void handleUninstall()}
				/>
			)}
		</div>
	);
}

function InstalledSkillRow({
	skill,
	installation,
	expanded,
	busy,
	onToggleDetails,
	onUpdate,
	onRepair,
	menuOpen,
	onToggleMenu,
	onUninstall,
}: {
	skill: EmployeeSkillRow;
	installation: ManagedOverviewInstallation | undefined;
	expanded: boolean;
	busy: InstalledAction | null;
	onToggleDetails: () => void;
	onUpdate: () => void;
	onRepair: () => void;
	menuOpen: boolean;
	onToggleMenu: () => void;
	onUninstall: () => void;
}) {
	const action = rowAction(skill);
	const actionHandler =
		action === "update" ? onUpdate : action === "repair" ? onToggleDetails : undefined;

	return (
		<article className="panel-inset overflow-hidden rounded-xl">
			<div className="flex flex-col gap-4 p-5 sm:flex-row sm:items-center">
				<div className="min-w-0 flex-1 space-y-2">
					<div className="flex flex-wrap items-center gap-2">
						<h2 className="truncate text-base font-semibold">{skill.name}</h2>
						<span
							className={cn(
								"text-xs font-medium",
								skill.state === "ready"
									? "text-accent"
									: skill.state === "update_available"
										? "text-primary"
										: "text-destructive",
							)}
						>
							{skill.statusLabel}
						</span>
					</div>
					<AgentAvailability availableAgents={skill.availableIn} compact />
				</div>

				<div className="flex shrink-0 items-center gap-2">
					{action === "configure" ? (
						<Link
							to={`/env?skill=${encodeURIComponent(skill.name)}`}
							aria-label={`Configurer ${skill.name}`}
							className={cn(buttonVariants({ variant: "outline", size: "sm" }))}
						>
							Configurer
						</Link>
					) : (
						<SkillPrimaryAction
							kind={action}
							skillName={skill.name}
							disabled={busy !== null}
							onAction={actionHandler}
						/>
					)}
					<div className="relative">
						<Button
							variant="ghost"
							size="icon"
							aria-label={`Plus d’actions pour ${skill.name}`}
							aria-haspopup="true"
							aria-expanded={menuOpen}
							onClick={onToggleMenu}
							disabled={busy !== null}
							className="size-8"
						>
							<MoreHorizontal className="size-4" />
						</Button>
						{menuOpen && (
							<div className="panel-raised absolute right-0 top-10 z-20 min-w-52 rounded-lg p-1 shadow-xl">
								<button
									type="button"
									aria-label={`Désinstaller ${skill.name}`}
									onClick={onUninstall}
									className="flex w-full items-center rounded-md px-3 py-2 text-left text-sm text-destructive transition-colors hover:bg-destructive/10 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
								>
									Désinstaller
								</button>
							</div>
						)}
					</div>
				</div>
			</div>

			{expanded && installation && (
				<div className="space-y-3 border-t border-border bg-background/30 px-5 py-4">
					<div className="flex items-center gap-2">
						<ShieldCheck className="size-4 text-primary" />
						<h3 className="text-sm font-semibold">État des connexions</h3>
					</div>
					<ul className="space-y-2">
						{installation.bindings.map((binding) => (
							<li key={binding.agent} className="flex items-center justify-between gap-4 text-sm">
								<span className="capitalize">{binding.agent}</span>
								<span className="text-muted-foreground">{bindingStatusLabel(binding.status)}</span>
							</li>
						))}
					</ul>
					<p className="text-xs text-muted-foreground">
						Un conflit reste intact tant qu’il n’est pas résolu explicitement.
					</p>
					<Button variant="outline" size="sm" onClick={onRepair} disabled={busy !== null}>
						{busy === "repair" && <Loader2 className="size-3.5 animate-spin" />}
						Réparer ce qui peut l’être
					</Button>
				</div>
			)}
		</article>
	);
}

function rowAction(skill: EmployeeSkillRow): SkillPrimaryActionKind {
	switch (skill.primaryAction) {
		case "configure":
			return "configure";
		case "repair":
			return "repair";
		case "update":
			return "update";
		default:
			return "none";
	}
}

function bindingStatusLabel(
	status: ManagedOverviewInstallation["bindings"][number]["status"],
): string {
	switch (status) {
		case "ready":
			return "Prête";
		case "needs_restart":
			return "Redémarrage nécessaire";
		case "missing":
			return "Connexion manquante";
		case "conflict":
			return "Conflit à vérifier";
		case "unsupported":
			return "Non compatible";
		default:
			return "Vérification nécessaire";
	}
}

function EmptyInstalled() {
	return (
		<div className="panel-inset flex min-h-56 flex-col items-center justify-center gap-3 rounded-xl p-8 text-center">
			<PackageOpen className="size-9 text-muted-foreground" />
			<div className="space-y-1">
				<p className="text-sm font-medium">Aucune skill installée</p>
				<p className="text-xs text-muted-foreground">
					Parcourez le catalogue approuvé pour ajouter votre première capability.
				</p>
			</div>
			<Link to="/catalog" className={cn(buttonVariants({ variant: "default", size: "sm" }))}>
				Parcourir le catalogue
			</Link>
		</div>
	);
}

function InstalledSkeleton() {
	return (
		<div className="mx-auto flex w-full max-w-5xl animate-pulse flex-col gap-4 p-6 motion-reduce:animate-none">
			<div className="h-12 rounded-lg bg-muted" />
			<div className="h-24 rounded-xl bg-card" />
			<div className="h-24 rounded-xl bg-card" />
		</div>
	);
}
