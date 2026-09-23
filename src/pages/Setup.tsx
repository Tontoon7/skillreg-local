import { OrganizationPicker } from "@/components/OrganizationPicker";
import { Titlebar } from "@/components/layout/Titlebar";
import { Button } from "@/components/ui/button";
import { detectAgents, previewManagedSkillsMigration, runManagedSkillsMigration } from "@/lib/api";
import { useAuthStore, useConfigStore } from "@/lib/store";
import type { ManagedAgent, MigrationPreview } from "@/lib/types";
import { AlertTriangle, Check, Loader2, RefreshCw, Sparkles } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

type SetupPhase = "select_company" | "preparing" | "migration" | "ready";

export function Setup() {
	const user = useAuthStore((state) => state.user);
	const config = useConfigStore((state) => state.config);
	const update = useConfigStore((state) => state.update);
	const setOrg = useConfigStore((state) => state.setOrg);
	const organizations = user?.orgs ?? [];
	const initialOrganization = useMemo(() => {
		const remembered = organizations.find((organization) => organization.slug === config.org);
		if (remembered) return remembered.slug;
		return organizations.length === 1 ? organizations[0].slug : "";
	}, [config.org, organizations]);
	const [organization, setOrganization] = useState(initialOrganization);
	const [phase, setPhase] = useState<SetupPhase>(
		initialOrganization ? "preparing" : "select_company",
	);
	const [agents, setAgents] = useState<ManagedAgent[]>([]);
	const [detectionFailed, setDetectionFailed] = useState(false);
	const [migration, setMigration] = useState<MigrationPreview | null>(null);
	const [busy, setBusy] = useState(false);
	const [error, setError] = useState<string | null>(null);
	const preparationRef = useRef<string | null>(null);

	const prepareWorkspace = useCallback(
		async (selectedOrganization: string) => {
			if (preparationRef.current === selectedOrganization) return;
			preparationRef.current = selectedOrganization;
			setPhase("preparing");
			setError(null);
			try {
				await setOrg(selectedOrganization);
				const [agentResult, migrationResult] = await Promise.allSettled([
					detectAgents(),
					previewManagedSkillsMigration(),
				]);
				if (agentResult.status === "fulfilled") {
					setAgents(agentResult.value);
					setDetectionFailed(false);
				} else {
					setAgents([]);
					setDetectionFailed(true);
				}
				if (
					migrationResult.status === "fulfilled" &&
					migrationNeedsAttention(migrationResult.value)
				) {
					setMigration(migrationResult.value);
					setPhase("migration");
				} else {
					setMigration(null);
					setPhase("ready");
				}
			} catch {
				preparationRef.current = null;
				setError("Impossible de préparer cet espace. Vérifiez votre connexion puis réessayez.");
			}
		},
		[setOrg],
	);

	useEffect(() => {
		if (phase === "preparing" && organization) {
			void prepareWorkspace(organization);
		}
	}, [organization, phase, prepareWorkspace]);

	const chooseOrganization = () => {
		if (!organization) return;
		preparationRef.current = null;
		void prepareWorkspace(organization);
	};

	const continueMigration = async () => {
		setBusy(true);
		setError(null);
		try {
			await runManagedSkillsMigration(true);
			setPhase("ready");
		} catch {
			setError("Certaines installations demandent encore votre attention.");
		} finally {
			setBusy(false);
		}
	};

	const retryDetection = async () => {
		setBusy(true);
		try {
			const detected = await detectAgents();
			setAgents(detected);
			setDetectionFailed(false);
		} catch {
			setAgents([]);
			setDetectionFailed(true);
		} finally {
			setBusy(false);
		}
	};

	const finish = async () => {
		setBusy(true);
		setError(null);
		try {
			await update({ org: organization, setupDone: true });
		} catch {
			setError("Impossible de terminer la préparation. Réessayez dans un instant.");
		} finally {
			setBusy(false);
		}
	};

	return (
		<div className="flex h-screen flex-col bg-background">
			<Titlebar />
			<main className="flex flex-1 items-center justify-center overflow-auto p-6">
				<div className="w-full max-w-md">
					{phase === "select_company" && organizations.length > 0 && (
						<OrganizationPicker
							organizations={organizations}
							selected={organization}
							onSelect={setOrganization}
							onContinue={chooseOrganization}
						/>
					)}

					{phase === "select_company" && organizations.length === 0 && (
						<section className="panel-inset rounded-xl p-6 text-center space-y-3">
							<AlertTriangle className="mx-auto size-7 text-primary" />
							<h1 className="text-lg font-semibold">Aucune entreprise disponible</h1>
							<p className="text-sm text-muted-foreground">
								Demandez à votre administrateur de vous inviter à un espace SkillReg.
							</p>
						</section>
					)}

					{phase === "preparing" && (
						<section
							className="panel-inset rounded-xl p-8 text-center space-y-4"
							aria-live="polite"
						>
							{error ? (
								<>
									<AlertTriangle className="mx-auto size-7 text-primary" />
									<div className="space-y-1">
										<h1 className="text-lg font-semibold">Préparation interrompue</h1>
										<p className="text-sm text-muted-foreground" role="alert">
											{error}
										</p>
									</div>
									<Button className="w-full" onClick={() => void prepareWorkspace(organization)}>
										Réessayer
									</Button>
								</>
							) : (
								<>
									<Loader2 className="mx-auto size-7 animate-spin text-primary" />
									<div className="space-y-1">
										<h1 className="text-lg font-semibold">Préparation de vos assistants</h1>
										<p className="text-sm text-muted-foreground">
											SkillReg vérifie ce qui est déjà disponible sur cet ordinateur.
										</p>
									</div>
								</>
							)}
						</section>
					)}

					{phase === "migration" && migration && (
						<section className="panel-inset rounded-xl p-6 space-y-5">
							<div className="space-y-1">
								<h1 className="text-lg font-semibold">
									SkillReg peut simplifier vos installations
								</h1>
								<p className="text-sm text-muted-foreground">
									Vos copies existantes restent protégées jusqu’à la fin de l’opération.
								</p>
							</div>
							<ul className="space-y-2 text-sm">
								<li>{migration.managedCandidates} skills seront centralisées.</li>
								<li>
									{migration.projectScopeUntouched} installations de projet resteront inchangées.
								</li>
								<li>
									{migration.modifiedUntouched} skills modifiées resteront à leur emplacement.
								</li>
							</ul>
							{error && (
								<p className="text-sm text-destructive" role="alert">
									{error}
								</p>
							)}
							<div className="flex gap-2">
								<Button variant="outline" className="flex-1" onClick={() => setPhase("ready")}>
									Plus tard
								</Button>
								<Button className="flex-1" onClick={continueMigration} disabled={busy}>
									{busy && <Loader2 className="size-4 animate-spin" />}
									Continuer
								</Button>
							</div>
						</section>
					)}

					{phase === "ready" && (
						<section className="panel-raised rounded-xl p-7 space-y-6">
							<div className="space-y-3 text-center">
								<div className="mx-auto flex size-11 items-center justify-center rounded-full bg-accent/10 text-accent">
									{agents.length > 0 ? (
										<Check className="size-5" />
									) : (
										<Sparkles className="size-5" />
									)}
								</div>
								<div className="space-y-1">
									<h1 className="text-lg font-semibold">Vos assistants sont prêts</h1>
									<p className="text-sm text-muted-foreground">
										{agents.length > 0
											? `${agents.filter((agent) => agent.state === "detected").length} assistant(s) détecté(s).`
											: "Aucun assistant détecté pour le moment. Vous pouvez quand même parcourir le catalogue."}
									</p>
								</div>
							</div>
							{detectionFailed && (
								<p className="text-sm text-primary text-center">
									La détection n’a pas abouti, sans bloquer votre accès.
								</p>
							)}
							{error && (
								<p className="text-sm text-destructive text-center" role="alert">
									{error}
								</p>
							)}
							<div className="space-y-2">
								<Button className="w-full" onClick={finish} disabled={busy}>
									{busy && <Loader2 className="size-4 animate-spin" />}
									Ouvrir SkillReg
								</Button>
								<Button variant="ghost" className="w-full" onClick={retryDetection} disabled={busy}>
									<RefreshCw className="size-4" />
									Relancer la détection
								</Button>
							</div>
						</section>
					)}
				</div>
			</main>
		</div>
	);
}

function migrationNeedsAttention(preview: MigrationPreview): boolean {
	return (
		preview.managedCandidates > 0 ||
		preview.modifiedUntouched > 0 ||
		preview.conflicts > 0 ||
		preview.missingRecords > 0
	);
}
