import { Badge } from "@/components/ui/badge";
import type { ValidationLevel } from "@/lib/types";
import { cn } from "@/lib/utils";
import { BadgeCheck, ShieldAlert, ShieldCheck, ShieldQuestion } from "lucide-react";

const LEVEL_CONFIG = {
	certified: {
		label: "Certified",
		icon: BadgeCheck,
		className: "bg-primary/20 text-primary border-primary/30",
		hint: "Reviewed and maintained by SkillReg",
	},
	verified: {
		label: "Verified",
		icon: ShieldCheck,
		className: "bg-emerald-500/15 text-emerald-400 border-emerald-500/30",
		hint: "Passed every structure and security check",
	},
	scanned: {
		label: "Scanned",
		icon: ShieldAlert,
		className: "bg-amber-500/15 text-amber-400 border-amber-500/30",
		hint: "Validation ran and reported failing checks",
	},
	unvalidated: {
		label: "Unvalidated",
		icon: ShieldQuestion,
		className: "bg-muted text-muted-foreground border-border",
		hint: "Published before validation was available",
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
		<Badge variant="outline" className={cn("gap-1", config.className, className)} title={config.hint}>
			<Icon className="size-3" aria-hidden />
			{config.label}
		</Badge>
	);
}
