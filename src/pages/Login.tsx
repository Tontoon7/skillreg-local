import { SkillRegLogo } from "@/components/SkillRegLogo";
import { Titlebar } from "@/components/layout/Titlebar";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { loginInitiate, loginPoll, loginWithToken, openUrl, whoami } from "@/lib/api";
import { API_BASE_URL } from "@/lib/constants";
import { useAuthStore } from "@/lib/store";
import { Copy, ExternalLink, Key, Loader2 } from "lucide-react";
import { useCallback, useRef, useState } from "react";

type AuthMode = "browser" | "token";

interface DeviceState {
	userCode: string | null;
	polling: boolean;
	error: string | null;
}

export function Login() {
	const setAuthenticated = useAuthStore((s) => s.setAuthenticated);
	const [mode, setMode] = useState<AuthMode>("browser");
	const [device, setDevice] = useState<DeviceState>({
		userCode: null,
		polling: false,
		error: null,
	});
	const [copied, setCopied] = useState(false);
	const pollingRef = useRef(false);

	// Token login state
	const [token, setToken] = useState("");
	const [tokenLoading, setTokenLoading] = useState(false);
	const [tokenError, setTokenError] = useState<string | null>(null);

	const startAuth = useCallback(async () => {
		setDevice({ userCode: null, polling: false, error: null });

		try {
			const { deviceCode, userCode, verificationUrl } = await loginInitiate();
			const fullUrl = `${API_BASE_URL}${verificationUrl}`;

			setDevice({ userCode, polling: true, error: null });

			try {
				await openUrl(fullUrl);
			} catch {
				// User can open manually
			}

			pollingRef.current = true;
			for (let i = 0; i < 100 && pollingRef.current; i++) {
				await new Promise((r) => setTimeout(r, 3000));
				if (!pollingRef.current) break;

				try {
					const result = await loginPoll(deviceCode);
					if (result.status === "complete" && result.token) {
						pollingRef.current = false;
						setDevice((p) => ({ ...p, polling: false }));
						const user = await whoami();
						setAuthenticated(user);
						return;
					}
				} catch (e) {
					if (e instanceof Error && String(e).includes("expired")) {
						pollingRef.current = false;
						setDevice({ userCode: null, polling: false, error: "Code expired. Try again." });
						return;
					}
				}
			}

			if (pollingRef.current) {
				pollingRef.current = false;
				setDevice({ userCode: null, polling: false, error: "Login timed out. Try again." });
			}
		} catch (e) {
			setDevice({
				userCode: null,
				polling: false,
				error: e instanceof Error ? e.message : "Authentication failed",
			});
		}
	}, [setAuthenticated]);

	const cancel = () => {
		pollingRef.current = false;
		setDevice({ userCode: null, polling: false, error: null });
	};

	const copyCode = async () => {
		if (!device.userCode) return;
		await navigator.clipboard.writeText(device.userCode);
		setCopied(true);
		setTimeout(() => setCopied(false), 2000);
	};

	const handleTokenLogin = async () => {
		setTokenError(null);
		setTokenLoading(true);
		try {
			await loginWithToken(token.trim());
			const user = await whoami();
			setAuthenticated(user);
		} catch (e) {
			setTokenError(typeof e === "string" ? e : e instanceof Error ? e.message : "Invalid token");
		} finally {
			setTokenLoading(false);
		}
	};

	return (
		<div className="flex h-screen flex-col bg-background">
			<Titlebar />
			<div className="flex flex-1 flex-col items-center justify-center">
				<div className="w-full max-w-sm space-y-6">
					<div className="flex flex-col items-center gap-3">
						<SkillRegLogo size={48} />
						<h1 className="text-xl font-semibold">SkillReg</h1>
						<p className="text-sm text-muted-foreground text-center">
							Sign in to manage your skills
						</p>
					</div>

					{/* Mode tabs */}
					<div className="flex rounded-lg border bg-card p-1">
						<button
							type="button"
							onClick={() => setMode("browser")}
							className={`flex-1 rounded-md px-3 py-1.5 text-sm font-medium transition-colors ${
								mode === "browser"
									? "bg-primary text-primary-foreground"
									: "text-muted-foreground hover:text-foreground"
							}`}
						>
							<ExternalLink className="mr-1.5 inline-block size-3.5 align-[-2px]" />
							Browser
						</button>
						<button
							type="button"
							onClick={() => setMode("token")}
							className={`flex-1 rounded-md px-3 py-1.5 text-sm font-medium transition-colors ${
								mode === "token"
									? "bg-primary text-primary-foreground"
									: "text-muted-foreground hover:text-foreground"
							}`}
						>
							<Key className="mr-1.5 inline-block size-3.5 align-[-2px]" />
							Token
						</button>
					</div>

					{mode === "browser" ? (
						<>
							{!device.userCode && !device.polling ? (
								<div className="rounded-xl border bg-card p-6 space-y-4">
									<Button className="w-full" size="lg" onClick={startAuth}>
										<ExternalLink className="size-4" />
										Sign in with browser
									</Button>
									{device.error && (
										<p className="text-sm text-destructive text-center">{device.error}</p>
									)}
								</div>
							) : (
								<div className="rounded-xl border bg-card p-6 space-y-4">
									<div className="text-center space-y-1">
										<p className="text-sm font-medium">Enter this code</p>
										<p className="text-xs text-muted-foreground">
											A browser window has opened. Enter the code below to sign in.
										</p>
									</div>

									<button
										type="button"
										onClick={copyCode}
										className="flex w-full items-center justify-center gap-2 rounded-lg bg-surface p-4 font-mono text-2xl font-bold tracking-widest transition-colors hover:bg-muted"
									>
										{device.userCode}
										<Copy className="size-4 text-muted-foreground" />
									</button>
									{copied && <p className="text-xs text-accent text-center">Copied to clipboard</p>}

									<div className="flex items-center justify-center gap-2 text-sm text-muted-foreground">
										<Loader2 className="size-4 animate-spin" />
										Waiting for authorization...
									</div>

									{device.error && (
										<p className="text-sm text-destructive text-center">{device.error}</p>
									)}

									<div className="flex gap-2">
										<Button variant="outline" size="sm" className="flex-1" onClick={cancel}>
											Cancel
										</Button>
										<Button variant="outline" size="sm" className="flex-1" onClick={startAuth}>
											Try again
										</Button>
									</div>
								</div>
							)}
						</>
					) : (
						<div className="rounded-xl border bg-card p-6 space-y-4">
							<div className="text-center space-y-1">
								<p className="text-sm font-medium">Paste your API token</p>
								<p className="text-xs text-muted-foreground">
									Get a token from your organization settings on app.skillreg.dev
								</p>
							</div>

							<Input
								type="password"
								placeholder="sr_live_... or sk_..."
								value={token}
								onChange={(e) => setToken(e.target.value)}
								onKeyDown={(e) => e.key === "Enter" && token.trim() && handleTokenLogin()}
							/>

							{tokenError && <p className="text-sm text-destructive text-center">{tokenError}</p>}

							<Button
								className="w-full"
								size="lg"
								disabled={!token.trim() || tokenLoading}
								onClick={handleTokenLogin}
							>
								{tokenLoading ? (
									<Loader2 className="size-4 animate-spin" />
								) : (
									<Key className="size-4" />
								)}
								Sign in with token
							</Button>
						</div>
					)}
				</div>
			</div>
		</div>
	);
}
