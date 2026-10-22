import { Button } from "@/components/ui/button";
import { Check, Loader2, Settings2, ShieldAlert, Sparkles, UploadCloud } from "lucide-react";

export type SkillPrimaryActionKind =
	| "install"
	| "installing"
	| "installed"
	| "configure"
	| "update"
	| "repair"
	| "none";

interface SkillPrimaryActionProps {
	kind: SkillPrimaryActionKind;
	skillName: string;
	disabled?: boolean;
	onAction?: () => void;
}

export function SkillPrimaryAction({
	kind,
	skillName,
	disabled = false,
	onAction,
}: SkillPrimaryActionProps) {
	if (kind === "installed") {
		return (
			<span className="inline-flex min-h-8 items-center gap-1.5 rounded-full border border-accent/30 bg-accent/10 px-3 text-xs font-medium text-accent">
				<Check className="size-3.5" />
				Installée
			</span>
		);
	}
	if (kind === "none") {
		return (
			<span className="inline-flex min-h-8 items-center gap-1.5 text-xs font-medium text-accent">
				<Check className="size-3.5" />
				Prête
			</span>
		);
	}

	const content = {
		install: { label: "Installer", icon: Sparkles, variant: "default" as const },
		installing: { label: "Installation…", icon: Loader2, variant: "default" as const },
		configure: { label: "Configurer", icon: Settings2, variant: "outline" as const },
		update: { label: "Mettre à jour", icon: UploadCloud, variant: "default" as const },
		repair: { label: "Voir le problème", icon: ShieldAlert, variant: "outline" as const },
	}[kind];
	const Icon = content.icon;

	return (
		<Button
			type="button"
			size="sm"
			variant={content.variant}
			aria-label={`${content.label} ${skillName}`}
			disabled={disabled || kind === "installing"}
			onClick={onAction}
		>
			<Icon className={kind === "installing" ? "size-3.5 animate-spin" : "size-3.5"} />
			{content.label}
		</Button>
	);
}
