import type { LocalSkill } from "./types";

const AGENT_ORDER = ["claude", "codex", "cursor"];
const SCOPE_ORDER = ["project", "user"];

export interface InstalledSkillGroup {
	name: string;
	description: string;
	tags: string[];
	version: string;
	modified_at: string | null;
	installations: LocalSkill[];
}

function compareInstallations(a: LocalSkill, b: LocalSkill): number {
	const agentDiff = AGENT_ORDER.indexOf(a.agent) - AGENT_ORDER.indexOf(b.agent);
	if (agentDiff !== 0) return agentDiff;

	const scopeDiff = SCOPE_ORDER.indexOf(a.scope) - SCOPE_ORDER.indexOf(b.scope);
	if (scopeDiff !== 0) return scopeDiff;

	return a.path.localeCompare(b.path);
}

function groupKey(skill: LocalSkill): string {
	return skill.name.toLowerCase();
}

function pickGroupSummary(installations: LocalSkill[]): Omit<InstalledSkillGroup, "installations"> {
	const sortedByModified = [...installations].sort((a, b) => {
		const aTime = Number(a.modified_at ?? 0);
		const bTime = Number(b.modified_at ?? 0);
		return bTime - aTime;
	});
	const primary = sortedByModified[0] ?? installations[0];

	return {
		name: primary.name,
		description: primary.description,
		tags: primary.tags,
		version: primary.version,
		modified_at: primary.modified_at,
	};
}

export function groupInstalledSkills(skills: LocalSkill[]): InstalledSkillGroup[] {
	const groups = new Map<string, LocalSkill[]>();

	for (const skill of skills) {
		const key = groupKey(skill);
		groups.set(key, [...(groups.get(key) ?? []), skill]);
	}

	return [...groups.values()]
		.map((installations) => ({
			...pickGroupSummary(installations),
			installations: [...installations].sort(compareInstallations),
		}))
		.sort((a, b) => a.name.localeCompare(b.name));
}
