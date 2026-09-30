type Organization = {
	slug: string;
};

export type DesktopSetupState = "workspace-required" | "organization-required" | "ready";

export function resolveDesktopSetupState(
	organizations: Organization[],
	selectedOrganization: string,
): DesktopSetupState {
	if (organizations.length === 0) return "workspace-required";
	if (!organizations.some((organization) => organization.slug === selectedOrganization)) {
		return "organization-required";
	}
	return "ready";
}

export function canCompleteDesktopSetup(
	organizations: Organization[],
	selectedOrganization: string,
): boolean {
	return resolveDesktopSetupState(organizations, selectedOrganization) === "ready";
}
