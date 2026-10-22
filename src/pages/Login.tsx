import { SkillRegLogo } from "@/components/SkillRegLogo";
import { Titlebar } from "@/components/layout/Titlebar";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { loginInitiate, loginPoll, loginWithToken, openUrl, whoami } from "@/lib/api";
import { API_BASE_URL } from "@/lib/constants";
import { resolveDevicePolling } from "@/lib/device-polling";
import { useAuthStore } from "@/lib/store";
import { Check, Copy, ExternalLink, KeyRound, Loader2 } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";

interface DeviceState {
	userCode: string | null;
	verificationUrl: string | null;
	polling: boolean;
	error: string | null;
}

export function Login() {
	const setAuthenticated = useAuthStore((state) => state.setAuthenticated);
	const [advancedOpen, setAdvancedOpen] = useState(false);
	const [device, setDevice] = useState<DeviceState>({
		userCode: null,
		verificationUrl: null,
		polling: false,
		error: null,
	});
	const [copied, setCopied] = useState(false);
	const [token, setToken] = useState("");
	const [tokenLoading, setTokenLoading] = useState(false);
	const [tokenError, setTokenError] = useState<string | null>(null);
	const pollingRef = useRef(false);

	useEffect(
		() => () => {
			pollingRef.current = false;
		},
		[],
	);

	const startAuth = useCallback(async () => {
		pollingRef.current = false;
		setDevice({ userCode: null, verificationUrl: null, polling: false, error: null });

		try {
			const { deviceCode, userCode, verificationUrl, expiresIn, interval } = await loginInitiate();
			const fullUrl = `${API_BASE_URL}${verificationUrl}`;
			const polling = resolveDevicePolling({ expiresIn, interval });
			setDevice({ userCode, verificationUrl: fullUrl, polling: true, error: null });
			try {
				await openUrl(fullUrl);
			} catch {
				// The visible fallback link remains available.
			}

			pollingRef.current = true;
			for (let attempt = 0; attempt < polling.maxAttempts && pollingRef.current; attempt += 1) {
				await new Promise((resolve) => setTimeout(resolve, polling.intervalMs));
				if (!pollingRef.current) break;
				try {
					const result = await loginPoll(deviceCode);
					if (result.status === "complete" && result.token) {
						pollingRef.current = false;
						setDevice((current) => ({ ...current, polling: false }));
						setAuthenticated(await whoami());
						return;
					}
				} catch (error) {
					if (String(error).toLowerCase().includes("expired")) {
						pollingRef.current = false;
						setDevice({
							userCode: null,
							verificationUrl: null,
							polling: false,
							error: "Le code a expiré. Relancez la connexion.",
						});
						return;
					}
				}
			}
		} catch (error) {
			setDevice({
				userCode: null,
				verificationUrl: null,
				polling: false,
				error:
					error instanceof Error ? error.message : "La connexion est momentanément indisponible.",
			});
		}
	}, [setAuthenticated]);

	const cancel = () => {
		pollingRef.current = false;
		setDevice({ userCode: null, verificationUrl: null, polling: false, error: null });
	};

	const copyCode = async () => {
		if (!device.userCode) return;
		await navigator.clipboard.writeText(device.userCode);
		setCopied(true);
		window.setTimeout(() => setCopied(false), 2000);
	};

	const handleTokenLogin = async () => {
		setTokenError(null);
		setTokenLoading(true);
		try {
			await loginWithToken(token.trim());
			setAuthenticated(await whoami());
		} catch (error) {
			setTokenError(
				typeof error === "string"
					? error
					: error instanceof Error
						? error.message
						: "Ce jeton n’est pas valide.",
			);
		} finally {
			setTokenLoading(false);
		}
	};

	return (
		<div className="flex h-screen flex-col bg-background">
			<Titlebar />
			<main className="flex flex-1 items-center justify-center overflow-auto p-6">
				<div className="w-full max-w-sm space-y-6">
					<header className="flex flex-col items-center gap-3 text-center">
						<SkillRegLogo size={48} />
						<div className="space-y-1">
							<h1 className="text-xl font-semibold">Connexion à votre espace</h1>
							<p className="text-sm text-muted-foreground">
								Retrouvez les skills approuvées par votre entreprise.
							</p>
						</div>
					</header>

					{device.userCode ? (
						<section className="panel-inset rounded-xl p-6 space-y-5" aria-live="polite">
							<div className="space-y-1 text-center">
								<p className="font-medium">Terminez la connexion dans votre navigateur</p>
								<p className="text-sm text-muted-foreground">
									Utilisez ce code si la page vous le demande.
								</p>
							</div>
							<button
								type="button"
								onClick={copyCode}
								aria-label={`Copier le code ${device.userCode}`}
								className="flex min-h-16 w-full items-center justify-center gap-3 rounded-lg bg-surface px-4 font-mono text-2xl font-bold tracking-widest focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
							>
								{device.userCode}
								{copied ? (
									<Check className="size-4 text-accent" />
								) : (
									<Copy className="size-4 text-muted-foreground" />
								)}
							</button>
							<div className="flex items-center justify-center gap-2 text-sm text-muted-foreground">
								<Loader2 className="size-4 animate-spin" />
								En attente de votre autorisation
							</div>
							{device.verificationUrl && (
								<Button
									variant="outline"
									className="w-full"
									onClick={() => openUrl(device.verificationUrl ?? "")}
								>
									<ExternalLink className="size-4" />
									Rouvrir la page
								</Button>
							)}
							<Button variant="ghost" className="w-full" onClick={cancel}>
								Annuler
							</Button>
						</section>
					) : (
						<section className="panel-inset rounded-xl p-6 space-y-4">
							<Button className="w-full" size="lg" onClick={startAuth}>
								Se connecter
							</Button>
							{device.error && (
								<p className="text-sm text-destructive text-center" role="alert">
									{device.error}
								</p>
							)}
						</section>
					)}

					<div className="space-y-3">
						<Button
							variant="ghost"
							className="w-full"
							aria-expanded={advancedOpen}
							onClick={() => setAdvancedOpen((open) => !open)}
						>
							<KeyRound className="size-4" />
							Connexion avancée
						</Button>
						{advancedOpen && (
							<section className="panel-inset rounded-xl p-5 space-y-4">
								<div className="space-y-1">
									<p className="text-sm font-medium">Jeton d’accès</p>
									<p className="text-xs text-muted-foreground">
										Réservé au support et aux environnements sans navigateur.
									</p>
								</div>
								<Input
									type="password"
									placeholder="sr_live_... ou sk_..."
									value={token}
									onChange={(event) => setToken(event.target.value)}
									onKeyDown={(event) => {
										if (event.key === "Enter" && token.trim()) void handleTokenLogin();
									}}
								/>
								{tokenError && (
									<p className="text-sm text-destructive" role="alert">
										{tokenError}
									</p>
								)}
								<Button
									variant="outline"
									className="w-full"
									disabled={!token.trim() || tokenLoading}
									onClick={handleTokenLogin}
								>
									{tokenLoading && <Loader2 className="size-4 animate-spin" />}
									Utiliser ce jeton
								</Button>
							</section>
						)}
					</div>
				</div>
			</main>
		</div>
	);
}
