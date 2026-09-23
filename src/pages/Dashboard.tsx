import { AutoUpdateControl } from "@/components/dashboard/AutoUpdateControl";
import { CleanupSuggestions } from "@/components/dashboard/CleanupSuggestions";
import { HealthSummary } from "@/components/dashboard/HealthSummary";
import { ManagedSkillList } from "@/components/dashboard/ManagedSkillList";
import { RequiredActions } from "@/components/dashboard/RequiredActions";
import { Button } from "@/components/ui/button";
import { useAuthStore, useConfigStore, useManagedSkillsStore } from "@/lib/store";
import { AlertTriangle, RefreshCw } from "lucide-react";
import { useEffect, useState } from "react";

type DashboardAction = "toggle" | "check" | "update" | "repair";

export function Dashboard() {
	const user = useAuthStore((state) => state.user);
	const org = useConfigStore((state) => state.config.org);
	const setOrg = useConfigStore((state) => state.setOrg);
	const model = useManagedSkillsStore((state) => state.model);
	const loading = useManagedSkillsStore((state) => state.loading);
	const refreshing = useManagedSkillsStore((state) => state.refreshing);
	const storeError = useManagedSkillsStore((state) => state.error);
	const refresh = useManagedSkillsStore((state) => state.refresh);
	const setAutomaticUpdates = useManagedSkillsStore((state) => state.setAutomaticUpdates);
	const checkUpdates = useManagedSkillsStore((state) => state.checkUpdates);
	const updateNow = useManagedSkillsStore((state) => state.updateNow);
	const repairAll = useManagedSkillsStore((state) => state.repairAll);
	const [busy, setBusy] = useState<DashboardAction | null>(null);
	const [actionError, setActionError] = useState<string | null>(null);
	const [organizationError, setOrganizationError] = useState<string | null>(null);
	const [organizationSwitching, setOrganizationSwitching] = useState(false);
	const [notice, setNotice] = useState<string | null>(null);

	useEffect(() => {
		if (!org) return;
		void repairAll().catch(() => {
			void refresh().catch(() => undefined);
		});
	}, [org, refresh, repairAll]);

	const handleToggle = async (enabled: boolean) => {
		setBusy("toggle");
		setActionError(null);
		setNotice(null);
		try {
			await setAutomaticUpdates(enabled);
		} catch {
			setActionError("Impossible de modifier les mises à jour automatiques.");
		} finally {
			setBusy(null);
		}
	};

	const handleCheck = async () => {
		setBusy("check");
		setActionError(null);
		setNotice(null);
		try {
			const summary = await checkUpdates(true);
			setNotice(
				summary.available > 0
					? `${summary.available} mise${summary.available > 1 ? "s" : ""} à jour disponible${
							summary.available > 1 ? "s" : ""
						}.`
					: "Toutes vos skills sont à jour.",
			);
		} catch {
			setActionError("La vérification n’a pas pu aboutir.");
		} finally {
			setBusy(null);
		}
	};

	const handleUpdate = async () => {
		setBusy("update");
		setActionError(null);
		setNotice(null);
		try {
			const summary = await updateNow();
			setNotice(
				summary.updated > 0
					? `${summary.updated} skill${summary.updated > 1 ? "s" : ""} mise${
							summary.updated > 1 ? "s" : ""
						} à jour.`
					: "Aucune mise à jour à appliquer.",
			);
		} catch {
			setActionError("Les mises à jour n’ont pas pu être appliquées.");
		} finally {
			setBusy(null);
		}
	};

	const handleRepair = async () => {
		setBusy("repair");
		setActionError(null);
		setNotice(null);
		try {
			const report = await repairAll();
			setNotice(
				report.repaired > 0
					? `${report.repaired} connexion${report.repaired > 1 ? "s" : ""} réparée${
							report.repaired > 1 ? "s" : ""
						}.`
					: "Les connexions ont été vérifiées.",
			);
		} catch {
			setActionError("La réparation n’a pas pu aboutir.");
		} finally {
			setBusy(null);
		}
	};

	const handleOrganizationChange = async (targetOrg: string) => {
		if (!targetOrg || targetOrg === org) return;
		setOrganizationSwitching(true);
		setOrganizationError(null);
		setNotice(null);
		try {
			await setOrg(targetOrg);
		} catch {
			setOrganizationError(
				"Impossible de changer d’entreprise. Vos skills actuelles restent disponibles.",
			);
		} finally {
			setOrganizationSwitching(false);
		}
	};

	if (loading && !model) {
		return <DashboardSkeleton />;
	}

	if (!model) {
		return (
			<div className="flex h-full flex-col items-center justify-center gap-4 p-6 text-center">
				<div className="flex size-11 items-center justify-center rounded-full bg-destructive/10 text-destructive">
					<AlertTriangle className="size-5" />
				</div>
				<div className="space-y-1">
					<h1 className="text-lg font-semibold">État local indisponible</h1>
					<p className="max-w-sm text-sm text-muted-foreground">
						{storeError ?? "SkillReg ne peut pas encore lire vos installations."}
					</p>
				</div>
				<Button variant="outline" onClick={() => void refresh()}>
					<RefreshCw className="size-4" />
					Réessayer
				</Button>
			</div>
		);
	}

	const updateBusy = busy === "toggle" || busy === "check" || busy === "update" ? busy : null;
	const requiredActionBusy = busy === "repair" || busy === "update" ? busy : null;

	return (
		<div className="mx-auto flex min-w-0 w-full max-w-5xl flex-col gap-5 p-6">
			<header className="flex min-h-9 flex-wrap items-center justify-between gap-4">
				<div>
					<p className="text-xs font-semibold uppercase tracking-widest text-muted-foreground">
						Accueil
					</p>
					{user?.user.name && (
						<p className="text-sm text-secondary-foreground">Bonjour {user.user.name}</p>
					)}
				</div>
				{user && user.orgs.length > 1 && (
					<select
						aria-label="Changer d’entreprise"
						value={org ?? ""}
						onChange={(event) => void handleOrganizationChange(event.target.value)}
						disabled={organizationSwitching}
						className="h-9 max-w-full rounded-md border border-border bg-input px-3 text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
					>
						{user.orgs.map((organization) => (
							<option key={organization.slug} value={organization.slug}>
								{organization.name}
							</option>
						))}
					</select>
				)}
			</header>

			{organizationError && (
				<p
					className="rounded-lg border border-destructive/30 bg-destructive/5 px-3 py-2 text-sm text-destructive"
					role="alert"
				>
					{organizationError}
				</p>
			)}

			<HealthSummary
				health={model.health}
				connectedAgents={model.connectedAgents}
				installedSkills={model.installedSkills}
				refreshing={refreshing}
			/>

			<AutoUpdateControl
				enabled={model.autoUpdateEnabled}
				updatesAvailable={model.updatesAvailable}
				lastCheckedAt={model.lastCheckedAt}
				busy={updateBusy}
				error={actionError}
				onToggle={(enabled) => void handleToggle(enabled)}
				onCheck={() => void handleCheck()}
				onUpdateAll={() => void handleUpdate()}
			/>

			{notice && (
				<output className="rounded-lg border border-accent/20 bg-accent/5 px-3 py-2 text-sm text-accent">
					{notice}
				</output>
			)}

			<RequiredActions
				actions={model.requiredActions}
				busy={requiredActionBusy}
				onRepair={() => void handleRepair()}
				onUpdate={() => void handleUpdate()}
			/>

			<ManagedSkillList skills={model.skills} />

			<CleanupSuggestions enabled={false} />
		</div>
	);
}

function DashboardSkeleton() {
	return (
		<div className="mx-auto flex min-w-0 w-full max-w-5xl animate-pulse flex-col gap-5 p-6 motion-reduce:animate-none">
			<div className="h-9 w-full rounded-lg bg-muted" />
			<div className="h-36 rounded-xl bg-card" />
			<div className="h-40 rounded-xl bg-card" />
			<div className="h-44 rounded-xl bg-card" />
		</div>
	);
}
