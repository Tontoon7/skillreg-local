export type LocalSkillAction = "publish" | "propose";

export type LocalSkillActionInput = {
	name: string;
	path: string;
};

export type RegistrySkillActionInput = {
	name: string;
	latestVersion: string | null;
};

export type RegistrySkillLookup = Record<string, RegistrySkillActionInput>;

export function createRegistrySkillLookup(skills: RegistrySkillActionInput[]): RegistrySkillLookup {
	return Object.fromEntries(skills.map((skill) => [skill.name.toLowerCase(), skill]));
}

export function getSkillSlug(skill: Pick<LocalSkillActionInput, "path">): string {
	const dirName = skill.path.split("/").pop() || skill.path.split("\\").pop() || "";
	return dirName.toLowerCase();
}

export function getRegistrySkillForLocal(
	skill: LocalSkillActionInput,
	registrySkills: RegistrySkillLookup,
): RegistrySkillActionInput | undefined {
	return registrySkills[skill.name.toLowerCase()] ?? registrySkills[getSkillSlug(skill)];
}

export function getLocalSkillAction(
	skill: LocalSkillActionInput,
	registrySkills: RegistrySkillLookup,
): LocalSkillAction {
	const registrySkill = getRegistrySkillForLocal(skill, registrySkills);
	return registrySkill?.latestVersion ? "propose" : "publish";
}
