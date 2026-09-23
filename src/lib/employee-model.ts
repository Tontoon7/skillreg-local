import type {
	AgentType,
	ManagedOverview,
	ManagedOverviewInstallation,
	ManagedSkillBinding,
} from "./types";

export type EmployeeHealth = "ready" | "action_required" | "offline" | "error";
export type EmployeeSkillState = "action_required" | "update_available" | "ready";

export interface EmployeeAction {
	kind: "configure" | "repair" | "update" | "detect_agents";
	installationId?: string;
	skillName?: string;
	title: string;
	description: string;
}

export interface EmployeeSkillRow {
	installationId: string;
	name: string;
	sourceOrg: string;
	state: EmployeeSkillState;
	statusLabel: string;
	availableIn: AgentType[];
	usageLabel: string;
	primaryAction: "configure" | "repair" | "update" | "none";
	missingEnvCount: number;
}

export interface EmployeeDashboardModel {
	health: EmployeeHealth;
	connectedAgents: number;
	installedSkills: number;
	updatesAvailable: number;
	autoUpdateEnabled: boolean;
	lastCheckedAt: string | null;
	requiredActions: EmployeeAction[];
	skills: EmployeeSkillRow[];
}

export function buildEmployeeDashboardModel(
	overview: ManagedOverview,
	options: { offline?: boolean } = {},
): EmployeeDashboardModel {
	const connectedAgents = overview.agents.filter((agent) => agent.state === "detected").length;
	const requiredActions: EmployeeAction[] = [];
	const skills = overview.installations.map((item) => {
		const row = buildSkillRow(item);
		if (row.primaryAction === "configure") {
			requiredActions.push({
				kind: "configure",
				installationId: row.installationId,
				skillName: row.name,
				title: `Configurer ${row.name}`,
				description: `${row.missingEnvCount} accès requis restent à renseigner.`,
			});
		} else if (row.primaryAction === "repair") {
			requiredActions.push({
				kind: "repair",
				installationId: row.installationId,
				skillName: row.name,
				title: `Vérifier ${row.name}`,
				description: "Un assistant nécessite votre attention.",
			});
		} else if (row.primaryAction === "update") {
			requiredActions.push({
				kind: "update",
				installationId: row.installationId,
				skillName: row.name,
				title: `Mettre à jour ${row.name}`,
				description: "Une mise à jour approuvée est disponible.",
			});
		}
		return row;
	});
	if (connectedAgents === 0) {
		requiredActions.unshift({
			kind: "detect_agents",
			title: "Connecter un assistant",
			description: "Relancez la détection pour rendre vos skills disponibles.",
		});
	}

	skills.sort(
		(left, right) =>
			skillPriority(left) - skillPriority(right) || left.name.localeCompare(right.name),
	);
	const updatesAvailable = skills.filter((skill) => skill.state === "update_available").length;
	const hasError = overview.installations.some(
		(item) =>
			item.installation.status === "error" ||
			item.bindings.some((binding) => binding.status === "error"),
	);
	const health: EmployeeHealth = options.offline
		? "offline"
		: hasError
			? "error"
			: requiredActions.some((action) => action.kind !== "update")
				? "action_required"
				: "ready";

	return {
		health,
		connectedAgents,
		installedSkills: skills.length,
		updatesAvailable,
		autoUpdateEnabled: overview.autoUpdateEnabled,
		lastCheckedAt: latestTimestamp(
			overview.installations.map((item) => item.installation.lastCheckedAt),
		),
		requiredActions,
		skills,
	};
}

function buildSkillRow(item: ManagedOverviewInstallation): EmployeeSkillRow {
	const missingEnvCount = item.missingEnvVars.filter((variable) => variable.required).length;
	const bindingNeedsAction = item.bindings.some((binding) =>
		["missing", "conflict", "unsupported", "error"].includes(binding.status),
	);
	const installationNeedsAction = ["action_required", "conflict", "error"].includes(
		item.installation.status,
	);
	let state: EmployeeSkillState = "ready";
	let statusLabel = "Prête";
	let primaryAction: EmployeeSkillRow["primaryAction"] = "none";
	if (missingEnvCount > 0) {
		state = "action_required";
		statusLabel = "À configurer";
		primaryAction = "configure";
	} else if (bindingNeedsAction || installationNeedsAction) {
		state = "action_required";
		statusLabel = "Action requise";
		primaryAction = "repair";
	} else if (item.installation.status === "update_available") {
		state = "update_available";
		statusLabel = "Mise à jour disponible";
		primaryAction = "update";
	}

	return {
		installationId: item.installation.installationId,
		name: item.installation.skillName,
		sourceOrg: item.installation.sourceOrg,
		state,
		statusLabel,
		availableIn: item.bindings
			.filter((binding) => binding.status === "ready" || binding.status === "needs_restart")
			.map((binding) => binding.agent),
		usageLabel: usageLabel(item.bindings),
		primaryAction,
		missingEnvCount,
	};
}

function usageLabel(bindings: ManagedSkillBinding[]): string {
	if (bindings.some((binding) => binding.usageObservability === "exact")) {
		return "Usage observable";
	}
	if (bindings.some((binding) => binding.usageObservability === "partial")) {
		return "Usage partiel";
	}
	return "Usage inconnu";
}

function skillPriority(skill: EmployeeSkillRow): number {
	if (skill.primaryAction === "configure") return 0;
	if (skill.state === "action_required") return 1;
	if (skill.state === "update_available") return 2;
	return 3;
}

function latestTimestamp(values: Array<string | null>): string | null {
	return (
		values
			.filter((value): value is string => value !== null)
			.sort((left, right) => right.localeCompare(left))[0] ?? null
	);
}
