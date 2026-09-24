import { Badge } from "@/components/ui/badge";
import type { ValidationLevel } from "@/lib/types";
import { cn } from "@/lib/utils";
import { BadgeCheck, ShieldAlert, ShieldCheck, ShieldQuestion } from "lucide-react";

const LEVEL_CONFIG = {
	certified: {
		label: "Certifiée",
		icon: BadgeCheck,
		className: "bg-primary/20 text-primary border-primary/30",
		hint: "Revue et maintenue par SkillReg",
	},
	verified: {
		label: "Vérifiée",
		icon: ShieldCheck,
		className: "bg-accent/10 text-accent border-accent/30",
		hint: "Tous les contrôles de structure et de sécurité sont validés",
	},
	scanned: {
		label: "Analysée",
		icon: ShieldAlert,
		className: "bg-primary/10 text-primary border-primary/30",
		hint: "Les contrôles ont été exécutés et demandent votre attention",
	},
	unvalidated: {
		label: "Non vérifiée",
		icon: ShieldQuestion,
		className: "bg-muted text-muted-foreground border-border",
		hint: "Publiée avant la disponibilité des contrôles",
	},
} as const satisfies Record<ValidationLevel, unknown>;

type Props = {
	level: ValidationLevel | null | undefined;
	className?: string;
};

export function ValidationBadge({ level, className }: Props) {
	if (!level) return null;

	const config = LEVEL_CONFIG[level];
	const Icon = config.icon;

	return (
		<Badge
			variant="outline"
			className={cn("gap-1", config.className, className)}
			title={config.hint}
		>
			<Icon className="size-3" aria-hidden />
			{config.label}
		</Badge>
	);
}
