import type {
	AgentType,
	CommandPublicationScope,
	PublishCommandVersionInput,
	RegistryCommandDetail,
} from "./types";

export interface CommandPublicationDraft {
	version: string;
	content: string;
	agentCompatibility: string[];
	scope: string;
}

export type CommandPublicationErrors = Partial<Record<keyof CommandPublicationDraft, string>>;

export function prepareCommandPublication(command: RegistryCommandDetail) {
	const current =
		command.latestVersion === null
			? command.versions[0]
			: command.versions.find((version) => version.version === command.latestVersion);
	if (command.latestVersion !== null && !current) {
		throw new Error(
			`The current version ${command.latestVersion} could not be loaded. Please retry.`,
		);
	}

	const draft: CommandPublicationDraft = {
		version: "",
		content: current?.content ?? "",
		agentCompatibility: [
			...(current?.agentCompatibility.length
				? current.agentCompatibility
				: command.agentCompatibility),
		],
		scope: current?.scope ?? command.scope,
	};
	return {
		draft,
		currentVersion: current?.version ?? null,
		existingVersions: command.versions.map((version) => version.version),
	};
}

function isAgentType(agent: string): agent is AgentType {
	return agent === "claude" || agent === "codex" || agent === "cursor";
}

function isCommandPublicationScope(scope: string): scope is CommandPublicationScope {
	return scope === "org" || scope === "project" || scope === "user";
}

export function validateCommandPublication(
	draft: CommandPublicationDraft,
	existingVersions: string[],
): { errors: CommandPublicationErrors; input: PublishCommandVersionInput | null } {
	const errors: CommandPublicationErrors = {};
	const version = draft.version.trim();
	if (!version) {
		errors.version = "Enter a new version.";
	} else if (!/^\d+\.\d+\.\d+(-[\w.]+)?(\+[\w.]+)?$/.test(version)) {
		errors.version = "Use a version such as 1.0.1, 1.0.1-beta.1 or 1.0.1+build.1.";
	} else if (existingVersions.includes(version)) {
		errors.version = "This version already exists. Enter a different version.";
	}

	const contentLength = draft.content.trim().length;
	if (contentLength === 0) {
		errors.content = "Enter the command content.";
	} else if (contentLength > 20_000) {
		errors.content = "Command content is too long (maximum 20,000 characters).";
	}

	const agents = draft.agentCompatibility.filter(isAgentType);
	if (
		agents.length === 0 ||
		agents.length > 3 ||
		agents.length !== draft.agentCompatibility.length ||
		new Set(agents).size !== agents.length
	) {
		errors.agentCompatibility = "Select one to three distinct agents: claude, codex or cursor.";
	}
	if (!isCommandPublicationScope(draft.scope)) {
		errors.scope = "Choose an organization, project or user command scope.";
	}
	if (Object.keys(errors).length > 0 || !isCommandPublicationScope(draft.scope)) {
		return { errors, input: null };
	}
	return {
		errors,
		input: { version, content: draft.content, agentCompatibility: agents, scope: draft.scope },
	};
}
