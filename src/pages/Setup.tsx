import { Titlebar } from "@/components/layout/Titlebar";
import { Button } from "@/components/ui/button";
import { useAuthStore, useConfigStore } from "@/lib/store";
import type { AgentType, ScopeType } from "@/lib/types";
import { ArrowLeft, ArrowRight, Check, Loader2 } from "lucide-react";
import { useState } from "react";

type Step = 1 | 2 | 3;

export function Setup() {
	const user = useAuthStore((s) => s.user);
	const config = useConfigStore((s) => s.config);
	const update = useConfigStore((s) => s.update);

	const [step, setStep] = useState<Step>(1);
	const [org, setOrg] = useState(config.org || user?.orgs[0]?.slug || "");
	const [agent, setAgent] = useState<AgentType>(config.defaultAgent || "claude");
	const [scope, setScope] = useState<ScopeType>(config.defaultScope || "project");
	const [saving, setSaving] = useState(false);

	const orgs = user?.orgs || [];

	const finish = async () => {
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
								{orgs.length === 0 && (
									<p className="text-sm text-muted-foreground text-center py-4">
										No organizations found
									</p>
								)}
							</div>

							<Button className="w-full" onClick={() => setStep(2)} disabled={!org}>
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
								<Button className="flex-1" onClick={finish} disabled={saving}>
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
