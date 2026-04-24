import { ProposeDialog } from "@/components/ProposeDialog";
import { PublishDialog } from "@/components/PublishDialog";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
	listSkillProposals,
	listSkills,
	pullSkill,
	scanLocalSkills,
	uninstallSkill,
} from "@/lib/api";
import {
	type RegistrySkillLookup,
	createRegistrySkillLookup,
	getLocalSkillAction,
	getRegistrySkillForLocal,
} from "@/lib/local-skill-actions";
import { notify } from "@/lib/notifications";
import { useConfigStore } from "@/lib/store";
import { tagStyle } from "@/lib/tag-colors";
import type {
	LocalSkill,
	ProposalSummary,
	RegistrySkill,
	SyncStatus,
	UpdateInfo,
} from "@/lib/types";
import { cn } from "@/lib/utils";
import {
	ArrowUpCircle,
	Check,
	CircleDot,
	Clock,
	FolderOpen,
	HardDrive,
	Loader2,
	RefreshCw,
	Search,
	Send,
	Trash2,
	Upload,
	X,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

function timeAgo(value: string): string {
	const now = Date.now();
	const then = /^\d+$/.test(value) ? Number(value) * 1000 : new Date(value).getTime();
	const diff = now - then;
	const minutes = Math.floor(diff / 60000);
	if (minutes < 1) return "just now";
	if (minutes < 60) return `${minutes}m ago`;
	const hours = Math.floor(minutes / 60);
	if (hours < 24) return `${hours}h ago`;
	const days = Math.floor(hours / 24);
	if (days < 30) return `${days}d ago`;
	const months = Math.floor(days / 30);
	if (months < 12) return `${months}mo ago`;
	const years = Math.floor(days / 365);
	return `${years}y ago`;
}

type AgentFilter = "claude" | "codex" | "cursor";
type ScopeFilter = "project" | "user";
type InstalledSyncStatus = SyncStatus | "checking";

const AGENTS: AgentFilter[] = ["claude", "codex", "cursor"];
const SCOPES: ScopeFilter[] = ["project", "user"];
const REGISTRY_PAGE_SIZE = 200;

async function loadAllRegistrySkills(org: string): Promise<RegistrySkill[]> {
	const firstPage = await listSkills({ org, page: 1, limit: REGISTRY_PAGE_SIZE });
	const remainingPages = Math.max(0, firstPage.pagination.totalPages - 1);
	if (remainingPages === 0) return firstPage.skills;

	const rest = await Promise.allSettled(
		Array.from({ length: remainingPages }, (_, index) =>
			listSkills({ org, page: index + 2, limit: REGISTRY_PAGE_SIZE }),
		),
	);

	return [
		...firstPage.skills,
		...rest.flatMap((page) => (page.status === "fulfilled" ? page.value.skills : [])),
	];
}

function getUpdatesFromRegistry(
	skills: LocalSkill[],
	registrySkills: RegistrySkillLookup,
): UpdateInfo[] {
	return skills.flatMap((skill) => {
		const registrySkill = getRegistrySkillForLocal(skill, registrySkills);
		const serverVersion = registrySkill?.latestVersion;

		if (!serverVersion || skill.version === "-" || serverVersion === skill.version) {
			return [];
		}

		return [
			{
				name: skill.name,
				localVersion: skill.version,
				serverVersion,
				agent: skill.agent,
				scope: skill.scope,
			},
		];
	});
}

function getSyncStatus(
	skill: LocalSkill,
	updates: UpdateInfo[],
	registrySkills: RegistrySkillLookup,
	registryLoading: boolean,
): InstalledSyncStatus {
	if (registryLoading) return "checking";

	const hasUpdate = updates.some(
		(u) => u.name === skill.name && u.agent === skill.agent && u.scope === skill.scope,
	);
	if (hasUpdate) return "update_available";

	if (getRegistrySkillForLocal(skill, registrySkills)?.latestVersion) return "synced";
	return "unknown";
}

function StatusBadge({ status, update }: { status: InstalledSyncStatus; update?: UpdateInfo }) {
	switch (status) {
		case "checking":
			return (
				<span className="inline-flex items-center gap-1 rounded-full bg-muted px-2 py-0.5 text-[11px] font-medium text-muted-foreground border border-border">
					<Loader2 className="size-3 animate-spin" />
					Checking
				</span>
			);
		case "synced":
			return (
				<span className="inline-flex items-center gap-1 rounded-full bg-emerald-500/10 px-2 py-0.5 text-[11px] font-medium text-emerald-400 border border-emerald-500/20">
					<Check className="size-3" />
					Up to date
				</span>
			);
		case "update_available":
			return (
				<span className="inline-flex items-center gap-1 rounded-full bg-amber-500/10 px-2 py-0.5 text-[11px] font-medium text-amber-400 border border-amber-500/20">
					<ArrowUpCircle className="size-3" />
					{update ? `${update.localVersion} → ${update.serverVersion}` : "Update"}
				</span>
			);
		case "unknown":
			return (
				<span className="inline-flex items-center gap-1 rounded-full bg-blue-500/10 px-2 py-0.5 text-[11px] font-medium text-blue-400 border border-blue-500/20">
					<HardDrive className="size-3" />
					Local only
				</span>
			);
		default:
			return null;
	}
}

export function Installed() {
	const org = useConfigStore((s) => s.config.org);
	const [skills, setSkills] = useState<LocalSkill[]>([]);
	const [updates, setUpdates] = useState<UpdateInfo[]>([]);
	const [registrySkills, setRegistrySkills] = useState<RegistrySkillLookup>({});
	const [recentProposals, setRecentProposals] = useState<Record<string, ProposalSummary[]>>({});
	const [loading, setLoading] = useState(true);
	const [registryLoading, setRegistryLoading] = useState(false);
	const [error, setError] = useState<string | null>(null);
	const [actionLoading, setActionLoading] = useState<string | null>(null);
	const [publishSkill, setPublishSkill] = useState<LocalSkill | null>(null);
	const [proposeSkill, setProposeSkill] = useState<LocalSkill | null>(null);

	const [search, setSearch] = useState("");
	const [debouncedSearch, setDebouncedSearch] = useState("");
	const [selectedAgents, setSelectedAgents] = useState<AgentFilter[]>([]);
	const [selectedScopes, setSelectedScopes] = useState<ScopeFilter[]>([]);

	const debounceRef = useRef<ReturnType<typeof setTimeout>>(undefined);
	const registryRequestRef = useRef(0);

	const loadRegistryMetadata = useCallback(
		async (localSkills: LocalSkill[]) => {
			const requestId = ++registryRequestRef.current;

			if (!org) {
				setUpdates([]);
				setRegistrySkills({});
				setRecentProposals({});
				setRegistryLoading(false);
				return;
			}

			setRegistryLoading(true);
			setUpdates([]);
			setRegistrySkills({});
			setRecentProposals({});

			let registryLookup: RegistrySkillLookup = {};
			try {
				const catalogSkills = await loadAllRegistrySkills(org);
				if (registryRequestRef.current !== requestId) return;

				registryLookup = createRegistrySkillLookup(catalogSkills);
				setRegistrySkills(registryLookup);
				setUpdates(getUpdatesFromRegistry(localSkills, registryLookup));
			} catch (e) {
				if (registryRequestRef.current !== requestId) return;
				console.warn("Failed to load registry metadata for installed skills", e);
				return;
			} finally {
				if (registryRequestRef.current === requestId) {
					setRegistryLoading(false);
				}
			}

			const proposalResults = await Promise.allSettled(
				localSkills
					.filter((skill) => getLocalSkillAction(skill, registryLookup) === "propose")
					.map(async (skill) => ({
						skillName: skill.name,
						proposals: await listSkillProposals(org, skill.name),
					})),
			);
			if (registryRequestRef.current !== requestId) return;

			const nextProposals: Record<string, ProposalSummary[]> = {};
			for (const item of proposalResults) {
				if (item.status === "fulfilled") {
					nextProposals[item.value.skillName] = item.value.proposals.slice(0, 3);
				}
			}
			setRecentProposals(nextProposals);
		},
		[org],
	);

	const load = useCallback(async () => {
		registryRequestRef.current += 1;
		setLoading(true);
		setRegistryLoading(false);
		setError(null);
		try {
			const result = await scanLocalSkills();
			setSkills(result);
			setLoading(false);
			void loadRegistryMetadata(result);
		} catch (e) {
			setError(typeof e === "string" ? e : "Failed to scan local skills");
			setLoading(false);
		}
	}, [loadRegistryMetadata]);

	useEffect(() => {
		load();
	}, [load]);

	const handleSearch = (value: string) => {
		setSearch(value);
		clearTimeout(debounceRef.current);
		debounceRef.current = setTimeout(() => setDebouncedSearch(value), 200);
	};

	const toggleAgent = (agent: AgentFilter) => {
		setSelectedAgents((prev) =>
			prev.includes(agent) ? prev.filter((a) => a !== agent) : [...prev, agent],
		);
	};

	const toggleScope = (scope: ScopeFilter) => {
		setSelectedScopes((prev) =>
			prev.includes(scope) ? prev.filter((s) => s !== scope) : [...prev, scope],
		);
	};

	const filteredSkills = useMemo(() => {
		let result = skills;

		if (debouncedSearch) {
			const q = debouncedSearch.toLowerCase();
			result = result.filter(
				(s) =>
					s.name.toLowerCase().includes(q) ||
					s.description.toLowerCase().includes(q) ||
					s.tags.some((t) => t.toLowerCase().includes(q)),
			);
		}

		if (selectedAgents.length > 0) {
			result = result.filter((s) => selectedAgents.includes(s.agent as AgentFilter));
		}

		if (selectedScopes.length > 0) {
			result = result.filter((s) => selectedScopes.includes(s.scope as ScopeFilter));
		}

		return [...result].sort((a, b) => a.name.localeCompare(b.name));
	}, [skills, debouncedSearch, selectedAgents, selectedScopes]);

	const stats = useMemo(() => {
		const updateCount = registryLoading ? 0 : updates.length;
		const localOnly = registryLoading
			? 0
			: skills.filter((s) => getLocalSkillAction(s, registrySkills) === "publish").length;
		return { total: skills.length, updateCount, localOnly };
	}, [skills, updates, registrySkills, registryLoading]);

	const presentAgents = useMemo(
		() => AGENTS.filter((a) => skills.some((s) => s.agent === a)),
		[skills],
	);
	const presentScopes = useMemo(
		() => SCOPES.filter((sc) => skills.some((s) => s.scope === sc)),
		[skills],
	);

	const hasActiveFilters =
		debouncedSearch || selectedAgents.length > 0 || selectedScopes.length > 0;

	const handleUninstall = async (skill: LocalSkill) => {
		const key = `${skill.agent}:${skill.scope}:${skill.name}`;
		setActionLoading(key);
		try {
			await uninstallSkill(skill.name, skill.agent, skill.scope);
			setSkills((prev) =>
				prev.filter(
					(s) => !(s.name === skill.name && s.agent === skill.agent && s.scope === skill.scope),
				),
			);
			notify("Skill uninstalled", skill.name);
		} catch (e) {
			notify("Operation failed", typeof e === "string" ? e : "Uninstall failed");
		} finally {
			setActionLoading(null);
		}
	};

	const handleUpdate = async (update: UpdateInfo) => {
		if (!org) return;
		const key = `${update.agent}:${update.scope}:${update.name}`;
		setActionLoading(key);
		try {
			await pullSkill({
				org,
				name: update.name,
				version: update.serverVersion,
				agent: update.agent,
				scope: update.scope,
			});
			setUpdates((prev) => prev.filter((u) => u.name !== update.name));
			await load();
			notify("Skill updated", `${update.name} v${update.serverVersion}`);
		} catch (e) {
			notify("Operation failed", typeof e === "string" ? e : "Update failed");
		} finally {
			setActionLoading(null);
		}
	};

	const getUpdate = (skill: LocalSkill) =>
		updates.find(
			(u) => u.name === skill.name && u.agent === skill.agent && u.scope === skill.scope,
		);

	const getProposalTone = (status: ProposalSummary["status"]) => {
		switch (status) {
			case "published":
				return "text-emerald-400 border-emerald-500/20 bg-emerald-500/10";
			case "rejected":
				return "text-red-400 border-red-500/20 bg-red-500/10";
			case "selected":
				return "text-sky-400 border-sky-500/20 bg-sky-500/10";
			case "needs_update":
				return "text-amber-400 border-amber-500/20 bg-amber-500/10";
			default:
				return "text-muted-foreground border-border bg-muted/40";
		}
	};

	if (loading) {
		return (
			<div className="flex items-center justify-center h-full">
				<Loader2 className="size-6 animate-spin text-muted-foreground" />
			</div>
		);
	}

	if (error) {
		return (
			<div className="flex flex-col items-center justify-center gap-3 h-full">
				<p className="text-sm text-destructive">{error}</p>
				<Button variant="outline" size="sm" onClick={load}>
					Retry
				</Button>
			</div>
		);
	}

	return (
		<div className="flex flex-col gap-4 p-6">
			{/* Header */}
			<div className="flex items-center justify-between">
				<h1 className="text-lg font-semibold">Installed Skills</h1>
				<Button variant="outline" size="sm" onClick={load} disabled={loading}>
					<RefreshCw className={cn("size-3.5", registryLoading && "animate-spin")} />
					Refresh
				</Button>
			</div>

			{/* Stats */}
			{skills.length > 0 && (
				<div className="flex items-center gap-4 text-sm">
					<span className="text-muted-foreground">
						<span className="font-medium text-foreground">{stats.total}</span> installed
					</span>
					{registryLoading && org && (
						<span className="text-muted-foreground">checking registry</span>
					)}
					{stats.updateCount > 0 && (
						<span className="text-amber-400">
							<span className="font-medium">{stats.updateCount}</span> update
							{stats.updateCount > 1 ? "s" : ""}
						</span>
					)}
					{stats.localOnly > 0 && (
						<span className="text-blue-400">
							<span className="font-medium">{stats.localOnly}</span> local only
						</span>
					)}
				</div>
			)}

			{skills.length === 0 ? (
				<div className="flex flex-col items-center justify-center gap-2 py-12">
					<FolderOpen className="size-10 text-muted-foreground" />
					<p className="text-muted-foreground">No skills installed</p>
				</div>
			) : (
				<>
					{/* Search */}
					<div className="relative">
						<Search className="absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
						<Input
							placeholder="Search installed skills..."
							value={search}
							onChange={(e) => handleSearch(e.target.value)}
							className="pl-9"
						/>
					</div>

					{/* Filters */}
					{(presentAgents.length > 1 || presentScopes.length > 1) && (
						<div className="flex items-center gap-3 flex-wrap">
							{presentAgents.length > 1 &&
								presentAgents.map((agent) => {
									const active = selectedAgents.includes(agent);
									return (
										<button
											key={agent}
											type="button"
											onClick={() => toggleAgent(agent)}
											className={cn(
												"inline-flex items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-xs font-medium transition-all cursor-pointer",
												active
													? "bg-primary/15 text-primary border-primary/40"
													: "bg-transparent text-muted-foreground border-border hover:border-muted-foreground/40",
											)}
										>
											<CircleDot className="size-3" />
											{agent}
											{active && <X className="size-3 opacity-70" />}
										</button>
									);
								})}
							{presentAgents.length > 1 && presentScopes.length > 1 && (
								<div className="h-4 w-px bg-border" />
							)}
							{presentScopes.length > 1 &&
								presentScopes.map((scope) => {
									const active = selectedScopes.includes(scope);
									return (
										<button
											key={scope}
											type="button"
											onClick={() => toggleScope(scope)}
											className={cn(
												"inline-flex items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-xs font-medium transition-all cursor-pointer",
												active
													? "bg-primary/15 text-primary border-primary/40"
													: "bg-transparent text-muted-foreground border-border hover:border-muted-foreground/40",
											)}
										>
											{scope}
											{active && <X className="size-3 opacity-70" />}
										</button>
									);
								})}
							{hasActiveFilters && (
								<button
									type="button"
									onClick={() => {
										setSearch("");
										setDebouncedSearch("");
										setSelectedAgents([]);
										setSelectedScopes([]);
									}}
									className="text-xs text-muted-foreground hover:text-foreground transition-colors"
								>
									Clear all
								</button>
							)}
						</div>
					)}

					{/* Skills list */}
					{filteredSkills.length > 0 ? (
						<div className="space-y-2">
							{filteredSkills.map((skill) => {
								const update = getUpdate(skill);
								const status = getSyncStatus(
									skill,
									updates,
									registrySkills,
									registryLoading && !!org,
								);
								const primaryAction = registryLoading
									? null
									: getLocalSkillAction(skill, registrySkills);
								const key = `${skill.agent}:${skill.scope}:${skill.name}`;
								const isLoading = actionLoading === key;

								return (
									<div
										key={key}
										className="flex items-center justify-between rounded-xl border bg-card p-4 gap-4"
									>
										<div className="flex flex-col gap-1.5 min-w-0 flex-1">
											<div className="flex items-center gap-2 flex-wrap">
												<span className="font-medium truncate">{skill.name}</span>
												<Badge variant="secondary">{skill.version}</Badge>
												<StatusBadge status={status} update={update} />
											</div>
											{skill.description && (
												<p className="text-sm text-muted-foreground line-clamp-1">
													{skill.description}
												</p>
											)}
											<div className="flex items-center gap-2 flex-wrap">
												{skill.tags.length > 0 && (
													<div className="flex flex-wrap gap-1">
														{skill.tags.slice(0, 4).map((tag) => (
															<span
																key={tag}
																className="inline-flex items-center rounded-full border px-2 py-0 text-[11px] font-medium"
																style={tagStyle(tag, false)}
															>
																{tag}
															</span>
														))}
													</div>
												)}
												<span className="text-[11px] text-muted-foreground/50">
													{skill.agent} · {skill.scope}
													{skill.modified_at && (
														<>
															{" · "}
															<Clock className="inline size-2.5 -mt-px" />
															{"  "}
															{timeAgo(skill.modified_at)}
														</>
													)}
												</span>
											</div>
											{recentProposals[skill.name] && recentProposals[skill.name].length > 0 && (
												<div className="flex flex-wrap items-center gap-2 pt-1">
													<span className="text-[11px] uppercase tracking-wide text-muted-foreground/60">
														Recent proposals
													</span>
													{recentProposals[skill.name].map((proposal) => (
														<span
															key={proposal.id}
															className={cn(
																"inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-[11px] font-medium",
																getProposalTone(proposal.status),
															)}
														>
															{proposal.title}
															<span className="opacity-70">· {timeAgo(proposal.createdAt)}</span>
														</span>
													))}
												</div>
											)}
										</div>

										<div className="flex items-center gap-2 shrink-0">
											{update && (
												<Button
													variant="outline"
													size="sm"
													disabled={isLoading}
													onClick={() => handleUpdate(update)}
													className="text-accent"
												>
													{isLoading ? (
														<Loader2 className="size-3.5 animate-spin" />
													) : (
														<ArrowUpCircle className="size-3.5" />
													)}
													Update
												</Button>
											)}
											{org && (
												<Button
													variant="outline"
													size="sm"
													disabled={isLoading || registryLoading}
													onClick={() => {
														if (!primaryAction) return;
														if (primaryAction === "propose") {
															setProposeSkill(skill);
														} else {
															setPublishSkill(skill);
														}
													}}
												>
													{registryLoading ? (
														<Loader2 className="size-3.5 animate-spin" />
													) : primaryAction === "propose" ? (
														<Send className="size-3.5" />
													) : (
														<Upload className="size-3.5" />
													)}
													{registryLoading
														? "Checking"
														: primaryAction === "propose"
															? "Propose"
															: "Publish"}
												</Button>
											)}
											<Button
												variant="ghost"
												size="sm"
												disabled={isLoading}
												onClick={() => handleUninstall(skill)}
												className="text-destructive hover:text-destructive"
											>
												{isLoading ? (
													<Loader2 className="size-3.5 animate-spin" />
												) : (
													<Trash2 className="size-3.5" />
												)}
											</Button>
										</div>
									</div>
								);
							})}
						</div>
					) : (
						<div className="flex flex-col items-center justify-center gap-2 py-12">
							<FolderOpen className="size-10 text-muted-foreground" />
							<p className="text-muted-foreground">No skills match your filters</p>
							<Button
								variant="outline"
								size="sm"
								onClick={() => {
									setSearch("");
									setDebouncedSearch("");
									setSelectedAgents([]);
									setSelectedScopes([]);
								}}
							>
								Clear filters
							</Button>
						</div>
					)}
				</>
			)}

			{publishSkill && org && (
				<PublishDialog
					skill={publishSkill}
					org={org}
					onClose={() => setPublishSkill(null)}
					onPublished={() => {
						setPublishSkill(null);
						load();
					}}
				/>
			)}

			{proposeSkill && org && (
				<ProposeDialog
					skill={proposeSkill}
					org={org}
					onClose={() => setProposeSkill(null)}
					onSubmitted={(proposal) => {
						const skillName = proposeSkill.name;
						setProposeSkill(null);
						setRecentProposals((prev) => ({
							...prev,
							[skillName]: [
								proposal,
								...(prev[skillName] || []).filter((item) => item.id !== proposal.id),
							].slice(0, 3),
						}));
					}}
				/>
			)}
		</div>
	);
}
