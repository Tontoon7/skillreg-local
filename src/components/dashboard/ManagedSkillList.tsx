import { buttonVariants } from "@/components/ui/button";
import type { EmployeeSkillRow } from "@/lib/employee-model";
import { cn } from "@/lib/utils";
import { ArrowRight, PackageOpen } from "lucide-react";
import { Link } from "react-router";

interface ManagedSkillListProps {
	skills: EmployeeSkillRow[];
}

const AGENT_LABELS = {
	claude: "Claude",
	codex: "Codex",
	cursor: "Cursor",
} as const;

export function ManagedSkillList({ skills }: ManagedSkillListProps) {
	return (
		<section className="space-y-3" aria-labelledby="managed-skills-title">
			<div className="flex items-end justify-between gap-4">
				<div>
					<h2 id="managed-skills-title" className="text-base font-semibold">
						Mes skills
					</h2>
					<p className="text-xs text-muted-foreground">
						Une seule installation, partout où elle est utile.
					</p>
				</div>
				{skills.length > 0 && (
					<Link to="/installed" className="text-xs font-medium text-primary hover:underline">
						Voir tout
					</Link>
				)}
			</div>

			{skills.length === 0 ? (
				<div className="panel-inset flex min-h-36 flex-col items-center justify-center gap-3 rounded-xl p-6 text-center">
					<PackageOpen className="size-7 text-muted-foreground" />
					<div className="space-y-1">
						<p className="text-sm font-medium">Aucune skill installée</p>
						<p className="text-xs text-muted-foreground">
							Découvrez les capabilities approuvées par votre entreprise.
						</p>
					</div>
					<Link to="/catalog" className={cn(buttonVariants({ variant: "outline", size: "sm" }))}>
						Parcourir le catalogue
						<ArrowRight className="size-3.5" />
					</Link>
				</div>
			) : (
				<div className="panel-inset divide-y divide-border overflow-hidden rounded-xl">
					{skills.map((skill) => (
						<div key={skill.installationId} className="flex min-h-16 items-center gap-4 px-4 py-3">
							<span
								className={cn(
									"led",
									skill.state === "ready"
										? "led-cyan"
										: skill.state === "update_available"
											? "led-amber"
											: "led-red",
								)}
								aria-hidden="true"
							/>
							<div className="min-w-0 flex-1">
								<p className="truncate text-sm font-medium">{skill.name}</p>
								<p className="truncate text-xs text-muted-foreground">
									{skill.availableIn.length > 0
										? `Disponible dans ${skill.availableIn
												.map((agent) => AGENT_LABELS[agent])
												.join(", ")}`
										: "En attente de connexion à un assistant"}
								</p>
							</div>
							<span
								className={cn(
									"text-xs font-medium",
									skill.state === "ready"
										? "text-accent"
										: skill.state === "update_available"
											? "text-primary"
											: "text-destructive",
								)}
							>
								{skill.statusLabel}
							</span>
						</div>
					))}
				</div>
			)}
		</section>
	);
}
