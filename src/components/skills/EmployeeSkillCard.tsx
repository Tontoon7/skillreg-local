import { ValidationBadge } from "@/components/ValidationBadge";
import {
	SkillPrimaryAction,
	type SkillPrimaryActionKind,
} from "@/components/skills/SkillPrimaryAction";
import type { ValidationLevel } from "@/lib/types";
import { Building2, ChevronRight, ShieldCheck } from "lucide-react";
import { Link } from "react-router";

export interface EmployeeCatalogSkill {
	key: string;
	name: string;
	description: string | null;
	sourceOrg: string;
	sourceName: string;
	trustLabel: string;
	validationLevel?: ValidationLevel | null;
	tags: string[];
	detailHref: string;
}

interface EmployeeSkillCardProps {
	skill: EmployeeCatalogSkill;
	action: SkillPrimaryActionKind;
	onAction: () => void;
}

export function EmployeeSkillCard({ skill, action, onAction }: EmployeeSkillCardProps) {
	return (
		<article className="panel-inset flex min-h-36 flex-col gap-4 rounded-xl p-5 sm:flex-row sm:items-center">
			<div className="min-w-0 flex-1 space-y-3">
				<div className="flex flex-wrap items-center gap-2">
					<Link
						to={skill.detailHref}
						className="group inline-flex min-w-0 items-center gap-1 font-semibold hover:text-primary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
					>
						<span className="truncate">{skill.name}</span>
						<ChevronRight className="size-3.5 opacity-50 transition-transform group-hover:translate-x-0.5 motion-reduce:transition-none" />
					</Link>
					<ValidationBadge level={skill.validationLevel} />
				</div>
				<p className="line-clamp-2 text-sm text-secondary-foreground">
					{skill.description || "Une capability approuvée par votre entreprise."}
				</p>
				<div className="flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-muted-foreground">
					<span className="inline-flex items-center gap-1.5">
						<Building2 className="size-3.5" />
						{skill.sourceName}
					</span>
					<span className="inline-flex items-center gap-1.5">
						<ShieldCheck className="size-3.5" />
						{skill.trustLabel}
					</span>
				</div>
				{skill.tags.length > 0 && (
					<div className="flex flex-wrap gap-1.5">
						{skill.tags.slice(0, 4).map((tag) => (
							<span
								key={tag}
								className="rounded-full border border-border px-2 py-0.5 text-[11px] text-muted-foreground"
							>
								{tag}
							</span>
						))}
					</div>
				)}
			</div>
			<div className="shrink-0 self-start sm:self-center">
				<SkillPrimaryAction kind={action} skillName={skill.name} onAction={onAction} />
			</div>
		</article>
	);
}
