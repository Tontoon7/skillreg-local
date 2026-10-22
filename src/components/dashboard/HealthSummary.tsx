import type { EmployeeHealth } from "@/lib/employee-model";
import { cn } from "@/lib/utils";
import { AlertTriangle, CheckCircle2, CloudOff, Loader2, ShieldAlert } from "lucide-react";

interface HealthSummaryProps {
	health: EmployeeHealth;
	connectedAgents: number;
	installedSkills: number;
	refreshing?: boolean;
}

const HEALTH_CONTENT: Record<
	EmployeeHealth,
	{ title: string; description: string; icon: typeof CheckCircle2; tone: string }
> = {
	ready: {
		title: "Vos assistants sont prêts",
		description: "Vos skills disponibles sont vérifiées et prêtes à être utilisées.",
		icon: CheckCircle2,
		tone: "text-accent",
	},
	action_required: {
		title: "Certaines skills demandent votre attention",
		description: "Suivez les actions proposées pour terminer leur préparation.",
		icon: AlertTriangle,
		tone: "text-primary",
	},
	offline: {
		title: "État local disponible hors connexion",
		description:
			"Vos skills restent accessibles. La vérification en ligne reprendra automatiquement.",
		icon: CloudOff,
		tone: "text-primary",
	},
	error: {
		title: "Une vérification est nécessaire",
		description: "SkillReg a conservé votre dernière configuration utilisable.",
		icon: ShieldAlert,
		tone: "text-destructive",
	},
};

export function HealthSummary({
	health,
	connectedAgents,
	installedSkills,
	refreshing = false,
}: HealthSummaryProps) {
	const content = HEALTH_CONTENT[health];
	const Icon = content.icon;
	const agentLabel =
		connectedAgents === 0
			? "Aucun assistant connecté"
			: `${connectedAgents} assistant${connectedAgents > 1 ? "s" : ""} connecté${
					connectedAgents > 1 ? "s" : ""
				}`;
	const skillLabel = `${installedSkills} skill${installedSkills > 1 ? "s" : ""} installée${
		installedSkills > 1 ? "s" : ""
	}`;

	return (
		<section
			className="panel-inset flex min-h-36 items-center gap-5 rounded-xl p-6"
			aria-labelledby="health-title"
			aria-live="polite"
		>
			<div
				className={cn(
					"flex size-12 shrink-0 items-center justify-center rounded-full bg-surface",
					content.tone,
				)}
			>
				<Icon className="size-6" aria-hidden="true" />
			</div>
			<div className="min-w-0 flex-1 space-y-2">
				<div className="flex items-center gap-2">
					<h1 id="health-title" className="text-xl font-semibold">
						{content.title}
					</h1>
					{refreshing && (
						<Loader2
							className="size-4 animate-spin text-muted-foreground"
							aria-label="Actualisation en cours"
						/>
					)}
				</div>
				<p className="text-sm text-muted-foreground">{content.description}</p>
				<div className="flex flex-wrap gap-x-4 gap-y-1 text-xs text-secondary-foreground">
					<span>{agentLabel}</span>
					<span aria-hidden="true">·</span>
					<span>{skillLabel}</span>
				</div>
			</div>
		</section>
	);
}
