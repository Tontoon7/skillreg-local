import type { AgentType } from "@/lib/types";
import { Bot, Check } from "lucide-react";

interface AgentAvailabilityProps {
	availableAgents: AgentType[];
	compact?: boolean;
}

const AGENT_LABELS: Record<AgentType, string> = {
	claude: "Claude",
	codex: "Codex",
	cursor: "Cursor",
};

export function AgentAvailability({ availableAgents, compact = false }: AgentAvailabilityProps) {
	if (availableAgents.length === 0) {
		return (
			<div className="flex items-start gap-2 text-sm text-muted-foreground">
				<Bot className="mt-0.5 size-4 shrink-0" aria-hidden="true" />
				<span>
					Aucun assistant compatible détecté. La skill sera reliée automatiquement dès qu’un
					assistant sera disponible.
				</span>
			</div>
		);
	}

	if (compact) {
		return (
			<p className="text-xs text-muted-foreground">
				Disponible dans {formatAgentList(availableAgents)}
			</p>
		);
	}

	return (
		<div className="space-y-2">
			<p className="text-sm text-secondary-foreground">
				Disponible dans {formatAgentList(availableAgents)}
			</p>
			<div className="flex flex-wrap gap-2" aria-label="Assistants détectés">
				{availableAgents.map((agent) => (
					<span
						key={agent}
						className="inline-flex items-center gap-1.5 rounded-full border border-accent/25 bg-accent/5 px-2.5 py-1 text-xs text-accent"
					>
						<Check className="size-3" aria-hidden="true" />
						{AGENT_LABELS[agent]}
					</span>
				))}
			</div>
		</div>
	);
}

export function formatAgentList(agents: AgentType[]): string {
	const labels = [...new Set(agents)].map((agent) => AGENT_LABELS[agent]);
	if (labels.length <= 1) return labels[0] ?? "";
	if (labels.length === 2) return `${labels[0]} et ${labels[1]}`;
	return `${labels.slice(0, -1).join(", ")} et ${labels[labels.length - 1]}`;
}
