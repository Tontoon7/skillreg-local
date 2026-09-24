import { Button, buttonVariants } from "@/components/ui/button";
import type { EmployeeAction } from "@/lib/employee-model";
import { cn } from "@/lib/utils";
import { AlertTriangle, KeyRound, Link2, Loader2, RotateCw } from "lucide-react";
import { Link } from "react-router";

interface RequiredActionsProps {
	actions: EmployeeAction[];
	busy?: "repair" | "update" | null;
	onRepair: () => void;
	onUpdate: () => void;
}

export function RequiredActions({
	actions,
	busy = null,
	onRepair,
	onUpdate,
}: RequiredActionsProps) {
	if (actions.length === 0) return null;

	return (
		<section className="space-y-3" aria-labelledby="required-actions-title">
			<div>
				<h2 id="required-actions-title" className="text-base font-semibold">
					Actions requises
				</h2>
				<p className="text-xs text-muted-foreground">
					Seulement les actions que vous pouvez résoudre ici.
				</p>
			</div>
			<div className="space-y-2">
				{actions.map((action) => (
					<div
						key={`${action.kind}:${action.installationId ?? "global"}`}
						className="panel-inset flex min-h-20 flex-wrap items-center gap-4 rounded-xl px-4 py-3"
					>
						<ActionIcon kind={action.kind} />
						<div className="min-w-0 flex-1">
							<h3 className="text-sm font-semibold">{action.title}</h3>
							<p className="text-xs text-muted-foreground">{action.description}</p>
						</div>
						<ActionControl action={action} busy={busy} onRepair={onRepair} onUpdate={onUpdate} />
					</div>
				))}
			</div>
		</section>
	);
}

function ActionIcon({ kind }: { kind: EmployeeAction["kind"] }) {
	const className = "size-4";
	if (kind === "configure") {
		return (
			<div className="flex size-9 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
				<KeyRound className={className} />
			</div>
		);
	}
	if (kind === "update") {
		return (
			<div className="flex size-9 shrink-0 items-center justify-center rounded-lg bg-accent/10 text-accent">
				<RotateCw className={className} />
			</div>
		);
	}
	return (
		<div className="flex size-9 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
			{kind === "detect_agents" ? (
				<Link2 className={className} />
			) : (
				<AlertTriangle className={className} />
			)}
		</div>
	);
}

function ActionControl({
	action,
	busy,
	onRepair,
	onUpdate,
}: {
	action: EmployeeAction;
	busy: RequiredActionsProps["busy"];
	onRepair: () => void;
	onUpdate: () => void;
}) {
	if (action.kind === "configure" && action.skillName) {
		return (
			<Link
				to={`/env?skill=${encodeURIComponent(action.skillName)}`}
				aria-label={`Configurer ${action.skillName}`}
				className={cn(
					buttonVariants({ variant: "outline", size: "sm" }),
					"w-full min-[1100px]:w-auto",
				)}
			>
				Configurer
			</Link>
		);
	}
	if (action.kind === "update") {
		return (
			<Button
				variant="outline"
				size="sm"
				onClick={onUpdate}
				disabled={busy !== null}
				className="w-full min-[1100px]:w-auto"
			>
				{busy === "update" && <Loader2 className="size-3.5 animate-spin" />}
				Mettre à jour
			</Button>
		);
	}

	const label =
		action.kind === "detect_agents"
			? "Réparer les connexions"
			: `Réparer ${action.skillName ?? "la skill"}`;
	return (
		<Button
			variant="outline"
			size="sm"
			aria-label={label}
			onClick={onRepair}
			disabled={busy !== null}
			className="w-full min-[1100px]:w-auto"
		>
			{busy === "repair" && <Loader2 className="size-3.5 animate-spin" />}
			{action.kind === "detect_agents" ? "Réparer les connexions" : "Réparer"}
		</Button>
	);
}
