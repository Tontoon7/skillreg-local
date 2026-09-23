import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { Loader2, RefreshCw, RotateCw } from "lucide-react";

interface AutoUpdateControlProps {
	enabled: boolean;
	updatesAvailable: number;
	lastCheckedAt: string | null;
	busy?: "toggle" | "check" | "update" | null;
	error?: string | null;
	onToggle: (enabled: boolean) => void;
	onCheck: () => void;
	onUpdateAll: () => void;
}

export function AutoUpdateControl({
	enabled,
	updatesAvailable,
	lastCheckedAt,
	busy = null,
	error = null,
	onToggle,
	onCheck,
	onUpdateAll,
}: AutoUpdateControlProps) {
	return (
		<section className="panel-raised min-w-0 rounded-xl p-5" aria-labelledby="auto-update-title">
			<div className="grid min-w-0 grid-cols-[minmax(0,1fr)_auto] items-start gap-4">
				<div className="min-w-0 max-w-xl space-y-1">
					<div className="flex flex-wrap items-center gap-2">
						<h2 id="auto-update-title" className="font-semibold">
							Maintenir mes skills à jour
						</h2>
						<span className={cn("text-xs font-semibold", enabled ? "text-accent" : "text-primary")}>
							{enabled ? "Activées" : "Désactivées"}
						</span>
					</div>
					<p className="text-sm text-muted-foreground">
						SkillReg applique automatiquement les versions approuvées par votre entreprise.
					</p>
				</div>

				<button
					type="button"
					role="switch"
					aria-checked={enabled}
					aria-labelledby="auto-update-title"
					disabled={busy === "toggle"}
					onClick={() => onToggle(!enabled)}
					className={cn(
						"relative h-7 w-12 shrink-0 rounded-full border transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-60",
						enabled ? "border-accent/70 bg-accent/25" : "border-border bg-background",
					)}
				>
					<span
						className={cn(
							"absolute top-1 size-[18px] rounded-full transition-transform motion-reduce:transition-none",
							enabled
								? "translate-x-6 bg-accent shadow-[0_0_8px_var(--glow-cyan)]"
								: "translate-x-1 bg-muted-foreground",
						)}
					/>
				</button>
			</div>

			<div className="mt-5 flex min-w-0 flex-col items-stretch gap-3 border-t border-border/80 pt-4 min-[1100px]:flex-row min-[1100px]:items-center min-[1100px]:justify-between">
				<div className="min-w-0 space-y-0.5">
					{!enabled && updatesAvailable > 0 && (
						<p className="text-sm font-medium text-primary">
							{updatesAvailable} mise{updatesAvailable > 1 ? "s" : ""} à jour disponible
							{updatesAvailable > 1 ? "s" : ""}
						</p>
					)}
					<p className="text-xs text-muted-foreground">
						{lastCheckedAt
							? `Dernière vérification : ${formatLastCheck(lastCheckedAt)}`
							: "Aucune vérification effectuée"}
					</p>
					{error && (
						<p className="text-xs text-destructive" role="alert">
							{error}
						</p>
					)}
				</div>

				<div className="flex w-full flex-wrap gap-2 min-[1100px]:w-auto">
					<Button
						variant="outline"
						size="sm"
						onClick={onCheck}
						disabled={busy !== null}
						className="flex-1 min-[1100px]:flex-none"
					>
						{busy === "check" ? (
							<Loader2 className="size-3.5 animate-spin" />
						) : (
							<RefreshCw className="size-3.5" />
						)}
						Vérifier maintenant
					</Button>
					{!enabled && updatesAvailable > 0 && (
						<Button
							size="sm"
							onClick={onUpdateAll}
							disabled={busy !== null}
							className="flex-1 min-[1100px]:flex-none"
						>
							{busy === "update" ? (
								<Loader2 className="size-3.5 animate-spin" />
							) : (
								<RotateCw className="size-3.5" />
							)}
							Tout mettre à jour
						</Button>
					)}
				</div>
			</div>
		</section>
	);
}

function formatLastCheck(value: string): string {
	const date = new Date(value);
	if (Number.isNaN(date.getTime())) return "date inconnue";
	return new Intl.DateTimeFormat("fr-FR", {
		dateStyle: "medium",
		timeStyle: "short",
	}).format(date);
}
