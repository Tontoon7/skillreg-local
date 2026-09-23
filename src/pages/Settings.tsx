import { LocalSkillImportPanel } from "@/components/settings/LocalSkillImportPanel";
import { Button, buttonVariants } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { Select } from "@/components/ui/select";
import { setLaunchAtLogin } from "@/lib/api";
import { useAuthStore, useConfigStore } from "@/lib/store";
import { cn } from "@/lib/utils";
import { ArrowUpRight, KeyRound, Loader2, LogOut, Save, TerminalSquare } from "lucide-react";
import { useEffect, useState } from "react";
import { Link } from "react-router";

const AUTO_UPDATE_INTERVALS = [
	{ label: "Toutes les 15 minutes", value: 15 },
	{ label: "Toutes les 30 minutes", value: 30 },
	{ label: "Toutes les heures", value: 60 },
	{ label: "Toutes les 6 heures", value: 360 },
	{ label: "Une fois par jour", value: 1440 },
];

export function Settings() {
	const config = useConfigStore((state) => state.config);
	const update = useConfigStore((state) => state.update);
	const doLogout = useAuthStore((state) => state.logout);
	const user = useAuthStore((state) => state.user);
	const activeOrganization = user?.orgs.find((organization) => organization.slug === config.org);
	const [saving, setSaving] = useState(false);
	const [saved, setSaved] = useState(false);
	const [error, setError] = useState<string | null>(null);
	const [autoUpdateIntervalMinutes, setAutoUpdateIntervalMinutes] = useState(
		config.autoUpdateIntervalMinutes ?? 60,
	);
	const [launchAtLogin, setLaunchAtLoginValue] = useState(config.launchAtLogin ?? false);

	useEffect(() => {
		setAutoUpdateIntervalMinutes(config.autoUpdateIntervalMinutes ?? 60);
		setLaunchAtLoginValue(config.launchAtLogin ?? false);
	}, [config.autoUpdateIntervalMinutes, config.launchAtLogin]);

	const handleSave = async () => {
		setSaving(true);
		setSaved(false);
		setError(null);
		try {
			await update({
				setupDone: true,
				autoUpdateIntervalMinutes,
				launchAtLogin,
			});
			await setLaunchAtLogin(launchAtLogin);
			setSaved(true);
		} catch {
			setError("Les réglages n’ont pas pu être enregistrés.");
		} finally {
			setSaving(false);
		}
	};

	return (
		<div className="mx-auto flex w-full max-w-3xl flex-col gap-6 p-6">
			<header className="space-y-1">
				<h1 className="text-xl font-semibold">Réglages</h1>
				<p className="text-sm text-muted-foreground">
					Votre compte, le démarrage de l’application et les outils avancés.
				</p>
			</header>

			{user && (
				<section className="panel-inset rounded-xl p-5" aria-labelledby="account-title">
					<h2 id="account-title" className="text-sm font-semibold">
						Compte
					</h2>
					<div className="mt-3 flex flex-wrap items-center justify-between gap-4">
						<div>
							<p className="text-sm font-medium">{user.user.name || user.user.email}</p>
							<p className="text-xs text-muted-foreground">{user.user.email}</p>
							{activeOrganization && (
								<p className="mt-1 text-xs text-secondary-foreground">
									Entreprise : {activeOrganization.name}
								</p>
							)}
						</div>
						<Button variant="outline" size="sm" onClick={() => void doLogout()}>
							<LogOut className="size-3.5" />
							Se déconnecter
						</Button>
					</div>
				</section>
			)}

			<section className="panel-inset space-y-4 rounded-xl p-5" aria-labelledby="app-title">
				<div>
					<h2 id="app-title" className="text-sm font-semibold">
						Application
					</h2>
					<p className="text-xs text-muted-foreground">
						Choisissez quand SkillReg démarre sur cet ordinateur.
					</p>
				</div>
				<label className="flex min-h-10 items-center justify-between gap-4 text-sm">
					<span>Ouvrir SkillReg au démarrage</span>
					<input
						type="checkbox"
						checked={launchAtLogin}
						onChange={(event) => setLaunchAtLoginValue(event.target.checked)}
						className="size-4 accent-primary"
					/>
				</label>
			</section>

			<LocalSkillImportPanel />

			<section className="panel-inset space-y-5 rounded-xl p-5" aria-labelledby="advanced-title">
				<div>
					<h2 id="advanced-title" className="text-sm font-semibold">
						Avancé
					</h2>
					<p className="text-xs text-muted-foreground">
						Outils destinés aux auteurs, à l’automatisation et au support.
					</p>
				</div>

				<div className="space-y-2">
					<Label htmlFor="auto-update-interval">Fréquence des vérifications</Label>
					<Select
						id="auto-update-interval"
						value={String(autoUpdateIntervalMinutes)}
						onChange={(event) => setAutoUpdateIntervalMinutes(Number(event.target.value))}
					>
						{AUTO_UPDATE_INTERVALS.map((interval) => (
							<option key={interval.value} value={interval.value}>
								{interval.label}
							</option>
						))}
					</Select>
					<p className="text-xs text-muted-foreground">
						L’interrupteur principal des mises à jour se trouve sur l’accueil.
					</p>
				</div>

				<div className="grid gap-2 sm:grid-cols-2">
					<Link
						to="/commands"
						className={cn(
							buttonVariants({ variant: "outline" }),
							"h-auto min-h-14 justify-start whitespace-normal px-4 py-3 text-left",
						)}
					>
						<TerminalSquare className="size-4" />
						<span className="flex-1">Commandes et automatisations</span>
						<ArrowUpRight className="size-3.5" />
					</Link>
					<Link
						to="/env"
						className={cn(
							buttonVariants({ variant: "outline" }),
							"h-auto min-h-14 justify-start whitespace-normal px-4 py-3 text-left",
						)}
					>
						<KeyRound className="size-4" />
						<span className="flex-1">Variables et accès</span>
						<ArrowUpRight className="size-3.5" />
					</Link>
				</div>
			</section>

			<div className="flex flex-wrap items-center gap-3">
				<Button onClick={() => void handleSave()} disabled={saving}>
					{saving ? <Loader2 className="size-4 animate-spin" /> : <Save className="size-4" />}
					Enregistrer
				</Button>
				{saved && <output className="text-sm text-accent">Réglages enregistrés.</output>}
				{error && (
					<p className="text-sm text-destructive" role="alert">
						{error}
					</p>
				)}
			</div>
		</div>
	);
}
