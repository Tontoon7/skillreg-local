import { Titlebar } from "@/components/layout/Titlebar";
import { Button } from "@/components/ui/button";
import { openUrl } from "@/lib/api";
import { canCompleteDesktopSetup, resolveDesktopSetupState } from "@/lib/setup-state";
import { useAuthStore, useConfigStore } from "@/lib/store";
import type { AgentType, ScopeType } from "@/lib/types";
import { ArrowLeft, ArrowRight, Check, ExternalLink, Loader2, RefreshCw } from "lucide-react";
import { useEffect, useState } from "react";

type Step = 1 | 2 | 3;

export function Setup() {
	const user = useAuthStore((s) => s.user);
	const refreshOrganizations = useAuthStore((s) => s.checkAuth);
	const config = useConfigStore((s) => s.config);
	const update = useConfigStore((s) => s.update);

	const [step, setStep] = useState<Step>(1);
	const [org, setOrg] = useState(config.org || user?.orgs[0]?.slug || "");
	const [agent, setAgent] = useState<AgentType>(config.defaultAgent || "claude");
	const [scope, setScope] = useState<ScopeType>(config.defaultScope || "project");
	const [saving, setSaving] = useState(false);
	const [refreshing, setRefreshing] = useState(false);
	const [workspaceError, setWorkspaceError] = useState<string | null>(null);

	const orgs = user?.orgs || [];
	const setupState = resolveDesktopSetupState(orgs, org);

	useEffect(() => {
		if (orgs.length === 1 && org !== orgs[0].slug) {
			setOrg(orgs[0].slug);
		} else if (org && !orgs.some((organization) => organization.slug === org)) {
			setOrg("");
		}
	}, [org, orgs]);

	const createWorkspace = async () => {
		setWorkspaceError(null);
		try {
			await openUrl("https://app.skillreg.dev/onboarding?source=desktop");
		} catch {
			setWorkspaceError("Could not open the browser. Visit app.skillreg.dev/onboarding.");
		}
	};

	const refresh = async () => {
		setRefreshing(true);
		setWorkspaceError(null);
		try {
			const refreshedUser = await refreshOrganizations();
			if (!refreshedUser) {
				setWorkspaceError(
					"Could not refresh your account. Check your connection and sign in again.",
				);
				return;
			}
			const nextOrganizations = refreshedUser.orgs;
			if (nextOrganizations.length === 0) {
				setWorkspaceError(
					"No workspace found yet. Finish creating it in the browser, then refresh.",
				);
				return;
			}
			if (!nextOrganizations.some((organization) => organization.slug === org)) {
				setOrg(nextOrganizations[0].slug);
			}
		} finally {
			setRefreshing(false);
		}
	};

	const finish = async () => {
		if (!canCompleteDesktopSetup(orgs, org)) {
			setWorkspaceError("Select a valid workspace before finishing setup.");
			setStep(1);
			return;
		}
		setSaving(true);
		try {
			await update({ org, defaultAgent: agent, defaultScope: scope, setupDone: true });
		} finally {
			setSaving(false);
		}
	};

	return (
		<div className="flex h-screen flex-col bg-background">
			<Titlebar />
			<div className="flex flex-1 flex-col items-center justify-center">
				<div className="w-full max-w-md space-y-6">
					{/* Progress */}
					<div className="flex items-center justify-center gap-2">
						{[1, 2, 3].map((s) => (
							<div
								key={s}
								className={`h-1.5 w-12 rounded-full transition-colors ${
									s <= step ? "bg-primary" : "bg-muted"
								}`}
							/>
						))}
					</div>

					<p className="text-center text-xs text-muted-foreground">Step {step} of 3</p>

					{step === 1 && (
						<div className="rounded-xl border bg-card p-6 space-y-4">
							<div className="text-center space-y-1">
								<p className="text-sm font-medium">Select your organization</p>
								<p className="text-xs text-muted-foreground">
									Choose the org you want to work with
								</p>
							</div>

							<div className="space-y-2">
								{orgs.map((o) => (
									<button
										type="button"
										key={o.slug}
										onClick={() => setOrg(o.slug)}
										className={`flex w-full items-center justify-between rounded-lg border p-3 text-left text-sm transition-colors ${
											org === o.slug
												? "border-primary bg-primary/5"
												: "hover:border-muted-foreground/30"
										}`}
									>
										<div>
											<p className="font-medium">{o.name}</p>
											<p className="text-xs text-muted-foreground capitalize">{o.role}</p>
										</div>
										{org === o.slug && <Check className="size-4 text-primary" />}
									</button>
								))}
								{setupState === "workspace-required" && (
									<div className="space-y-3 rounded-lg border border-dashed p-4 text-center">
										<div>
											<p className="text-sm font-medium">Create your first workspace</p>
											<p className="mt-1 text-xs text-muted-foreground">
												A workspace is required to publish, invite teammates, and install private
												skills.
											</p>
										</div>
										<div className="grid gap-2">
											<Button type="button" onClick={createWorkspace}>
												<ExternalLink className="size-4" />
												Create workspace in browser
											</Button>
											<Button
												type="button"
												variant="outline"
												onClick={refresh}
												disabled={refreshing}
											>
												<RefreshCw className={`size-4 ${refreshing ? "animate-spin" : ""}`} />
												{refreshing ? "Refreshing..." : "Refresh organizations"}
											</Button>
										</div>
									</div>
								)}
							</div>

							{workspaceError && (
								<p className="rounded-md bg-destructive/10 p-3 text-xs text-destructive">
									{workspaceError}
								</p>
							)}

							<Button
								className="w-full"
								onClick={() => setStep(2)}
								disabled={setupState !== "ready"}
							>
								Next
								<ArrowRight className="size-4" />
							</Button>
						</div>
					)}

					{step === 2 && (
						<div className="rounded-xl border bg-card p-6 space-y-4">
							<div className="text-center space-y-1">
								<p className="text-sm font-medium">Default agent</p>
								<p className="text-xs text-muted-foreground">Which AI agent do you use?</p>
							</div>

							<div className="space-y-2">
								{(["claude", "codex", "cursor"] as const).map((a) => (
									<button
										type="button"
										key={a}
										onClick={() => setAgent(a)}
										className={`flex w-full items-center justify-between rounded-lg border p-3 text-left text-sm transition-colors ${
											agent === a
												? "border-primary bg-primary/5"
												: "hover:border-muted-foreground/30"
										}`}
									>
										<span className="font-medium capitalize">{a}</span>
										{agent === a && <Check className="size-4 text-primary" />}
									</button>
								))}
							</div>

							<div className="flex gap-2">
								<Button variant="outline" className="flex-1" onClick={() => setStep(1)}>
									<ArrowLeft className="size-4" />
									Back
								</Button>
								<Button className="flex-1" onClick={() => setStep(3)}>
									Next
									<ArrowRight className="size-4" />
								</Button>
							</div>
						</div>
					)}

					{step === 3 && (
						<div className="rounded-xl border bg-card p-6 space-y-4">
							<div className="text-center space-y-1">
								<p className="text-sm font-medium">Default scope</p>
								<p className="text-xs text-muted-foreground">Where should skills be installed?</p>
							</div>

							<div className="space-y-2">
								<button
									type="button"
									onClick={() => setScope("project")}
									className={`flex w-full items-center justify-between rounded-lg border p-3 text-left text-sm transition-colors ${
										scope === "project"
											? "border-primary bg-primary/5"
											: "hover:border-muted-foreground/30"
									}`}
								>
									<div>
										<p className="font-medium">Project</p>
										<p className="text-xs text-muted-foreground">
											Install in current project directory
										</p>
									</div>
									{scope === "project" && <Check className="size-4 text-primary" />}
								</button>
								<button
									type="button"
									onClick={() => setScope("user")}
									className={`flex w-full items-center justify-between rounded-lg border p-3 text-left text-sm transition-colors ${
										scope === "user"
											? "border-primary bg-primary/5"
											: "hover:border-muted-foreground/30"
									}`}
								>
									<div>
										<p className="font-medium">User</p>
										<p className="text-xs text-muted-foreground">
											Install globally for current user
										</p>
									</div>
									{scope === "user" && <Check className="size-4 text-primary" />}
								</button>
							</div>

							<div className="flex gap-2">
								<Button variant="outline" className="flex-1" onClick={() => setStep(2)}>
									<ArrowLeft className="size-4" />
									Back
								</Button>
								<Button
									className="flex-1"
									onClick={finish}
									disabled={saving || !canCompleteDesktopSetup(orgs, org)}
								>
									{saving ? (
										<Loader2 className="size-4 animate-spin" />
									) : (
										<Check className="size-4" />
									)}
									Finish
								</Button>
							</div>
						</div>
					)}
				</div>
			</div>
		</div>
	);
}
