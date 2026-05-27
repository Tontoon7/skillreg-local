import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Select } from "@/components/ui/select";
import {
	listCommands,
	listLocalCommands,
	pullCommand,
	removeCommand,
	updateCommand,
} from "@/lib/api";
import { notify } from "@/lib/notifications";
import { useConfigStore } from "@/lib/store";
import type { AgentType, InstalledCommandRecord, RegistryCommand, ScopeType } from "@/lib/types";
import { cn } from "@/lib/utils";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import {
	Check,
	Download,
	FolderOpen,
	Loader2,
	RefreshCw,
	Search,
	Terminal,
	Trash2,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useNavigate } from "react-router";

const AGENTS: AgentType[] = ["claude", "codex", "cursor"];
const SCOPES: ScopeType[] = ["project", "user"];

type CommandInstallAgent = AgentType | "all";

function getCompatibleAgents(command: RegistryCommand): AgentType[] {
	const compatible = command.agentCompatibility.filter((agent): agent is AgentType =>
		AGENTS.includes(agent as AgentType),
	);
	return compatible.length > 0 ? compatible : AGENTS;
}

function isCommandInstalled(command: RegistryCommand, localCommands: InstalledCommandRecord[]) {
	return localCommands.some((record) => record.name === command.name);
}

function commandKey(record: InstalledCommandRecord) {
	return `${record.org}:${record.name}:${record.agent}:${record.scope}:${record.path}`;
}

function formatCommandRef(org: string, name: string) {
	return `@${org}/${name}`;
}

function displayPath(path: string) {
	const normalized = path.replaceAll("\\", "/");
	const parts = normalized.split("/");
	if (parts.length <= 4) return path;
	return `.../${parts.slice(-4).join("/")}`;
}

export function Commands() {
	const navigate = useNavigate();
	const config = useConfigStore((s) => s.config);
	const org = config.org;

	const [commands, setCommands] = useState<RegistryCommand[]>([]);
	const [localCommands, setLocalCommands] = useState<InstalledCommandRecord[]>([]);
	const [loading, setLoading] = useState(true);
	const [error, setError] = useState<string | null>(null);
	const [search, setSearch] = useState("");
	const [debouncedSearch, setDebouncedSearch] = useState("");
	const [installAgent, setInstallAgent] = useState<CommandInstallAgent>(
		config.defaultAgent || "claude",
	);
	const [scope, setScope] = useState<ScopeType>(config.defaultScope || "project");
	const [projectDir, setProjectDir] = useState<string | null>(null);
	const [actionLoading, setActionLoading] = useState<string | null>(null);

	const debounceRef = useRef<ReturnType<typeof setTimeout>>(undefined);

	const load = useCallback(async () => {
		if (!org) return;
		setLoading(true);
		setError(null);
		try {
			const [registry, local] = await Promise.all([
				listCommands(org),
				listLocalCommands({ org }).catch(() => []),
			]);
			setCommands(registry);
			setLocalCommands(local);
		} catch (e) {
			setError(typeof e === "string" ? e : "Failed to load commands");
		} finally {
			setLoading(false);
		}
	}, [org]);

	useEffect(() => {
		load();
	}, [load]);

	const handleSearch = (value: string) => {
		setSearch(value);
		clearTimeout(debounceRef.current);
		debounceRef.current = setTimeout(() => setDebouncedSearch(value), 200);
	};

	const filteredCommands = useMemo(() => {
		const q = debouncedSearch.trim().toLowerCase();
		if (!q) return commands;
		return commands.filter(
			(command) =>
				command.name.toLowerCase().includes(q) ||
				command.description.toLowerCase().includes(q) ||
				command.agentCompatibility.some((agent) => agent.toLowerCase().includes(q)),
		);
	}, [commands, debouncedSearch]);

	const filteredLocalCommands = useMemo(() => {
		const q = debouncedSearch.trim().toLowerCase();
		if (!q) return localCommands;
		return localCommands.filter(
			(record) =>
				record.name.toLowerCase().includes(q) ||
				record.agent.toLowerCase().includes(q) ||
				record.scope.toLowerCase().includes(q),
		);
	}, [localCommands, debouncedSearch]);

	const pickProjectDir = async () => {
		const selected = await openDialog({ directory: true, title: "Select project folder" });
		if (typeof selected === "string") setProjectDir(selected);
	};

	const handleInstall = async (command: RegistryCommand) => {
		if (!org) return;
		if (scope === "project" && !projectDir) {
			notify("Project folder required", "Select a project folder before installing.");
			return;
		}

		const compatibleAgents = getCompatibleAgents(command);
		if (installAgent !== "all" && !compatibleAgents.includes(installAgent)) {
			notify("Agent not supported", `/${command.name} cannot be installed for ${installAgent}.`);
			return;
		}

		const key = `install:${command.name}`;
		setActionLoading(key);
		try {
			const result = await pullCommand({
				org,
				name: command.name,
				agent: installAgent,
				scope,
				projectDir: scope === "project" ? (projectDir ?? undefined) : undefined,
			});
			await load();
			notify(
				"Command installed",
				`/${result.name}@${result.version} installed to ${result.paths.length} target${
					result.paths.length > 1 ? "s" : ""
				}.`,
			);
		} catch (e) {
			notify("Install failed", typeof e === "string" ? e : "Command install failed");
		} finally {
			setActionLoading(null);
		}
	};

	const handleUpdate = async (record: InstalledCommandRecord) => {
		const key = `update:${commandKey(record)}`;
		setActionLoading(key);
		try {
			const result = await updateCommand({
				org: record.org,
				name: record.name,
				agent: record.agent,
				scope: record.scope,
			});
			await load();
			const updated = result.updated.length;
			const skipped = result.skipped[0]?.reason;
			notify(
				updated > 0 ? "Command updated" : "No update applied",
				updated > 0 ? `/${record.name} updated.` : skipped || "Command is already up to date.",
			);
		} catch (e) {
			notify("Update failed", typeof e === "string" ? e : "Command update failed");
		} finally {
			setActionLoading(null);
		}
	};

	const handleRemove = async (record: InstalledCommandRecord) => {
		const key = `remove:${commandKey(record)}`;
		setActionLoading(key);
		try {
			const result = await removeCommand({
				org: record.org,
				name: record.name,
				agent: record.agent,
				scope: record.scope,
			});
			setLocalCommands((prev) =>
				prev.filter(
					(candidate) =>
						!result.removed.some((removed) => commandKey(removed) === commandKey(candidate)),
				),
			);
			notify("Command removed", `/${record.name} removed from ${record.agent}.`);
		} catch (e) {
			notify("Remove failed", typeof e === "string" ? e : "Command remove failed");
		} finally {
			setActionLoading(null);
		}
	};

	if (!org) {
		return (
			<div className="flex h-full flex-col items-center justify-center gap-3">
				<Terminal className="size-10 text-muted-foreground" />
				<p className="text-muted-foreground">Select an organization first</p>
				<Button variant="outline" size="sm" onClick={() => navigate("/settings")}>
					Go to Settings
				</Button>
			</div>
		);
	}

	if (loading) {
		return (
			<div className="flex h-full items-center justify-center">
				<Loader2 className="size-6 animate-spin text-muted-foreground" />
			</div>
		);
	}

	return (
		<div className="flex flex-col gap-4 p-6">
			<div className="flex items-center justify-between gap-3">
				<div>
					<h1 className="text-lg font-semibold">{org} — Commands</h1>
					<p className="text-sm text-muted-foreground">
						Install slash commands for Claude, Codex, and Cursor.
					</p>
				</div>
				<Button variant="outline" size="sm" onClick={load}>
					<RefreshCw className="size-3.5" />
					Refresh
				</Button>
			</div>

			{error && (
				<div className="rounded-lg border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive">
					{error}
				</div>
			)}

			<div className="flex flex-wrap items-center gap-3">
				<div className="relative min-w-64 flex-1">
					<Search className="absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
					<Input
						placeholder="Search commands..."
						value={search}
						onChange={(e) => handleSearch(e.target.value)}
						className="pl-9"
					/>
				</div>
				<Select
					value={installAgent}
					onChange={(e) => setInstallAgent(e.target.value as CommandInstallAgent)}
					className="w-auto"
				>
					<option value="all">All compatible</option>
					{AGENTS.map((agent) => (
						<option key={agent} value={agent}>
							{agent}
						</option>
					))}
				</Select>
				<Select
					value={scope}
					onChange={(e) => setScope(e.target.value as ScopeType)}
					className="w-auto"
				>
					{SCOPES.map((value) => (
						<option key={value} value={value}>
							{value}
						</option>
					))}
				</Select>
				{scope === "project" && (
					<Button variant="outline" size="sm" onClick={pickProjectDir} className="max-w-56">
						<FolderOpen className="size-3.5 shrink-0" />
						<span className="truncate">
							{projectDir ? projectDir.split("/").pop() : "Project folder"}
						</span>
					</Button>
				)}
			</div>

			<div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_minmax(320px,420px)]">
				<section className="space-y-3">
					<div className="flex items-center justify-between">
						<h2 className="text-sm font-medium text-muted-foreground">Registry</h2>
						<span className="text-xs text-muted-foreground">
							{filteredCommands.length} command{filteredCommands.length > 1 ? "s" : ""}
						</span>
					</div>

					{filteredCommands.length === 0 ? (
						<EmptyState
							label={debouncedSearch ? "No commands match your search" : "No commands found"}
						/>
					) : (
						<div className="space-y-2">
							{filteredCommands.map((command) => {
								const installed = isCommandInstalled(command, localCommands);
								const compatibleAgents = getCompatibleAgents(command);
								const incompatible =
									installAgent !== "all" && !compatibleAgents.includes(installAgent);
								const loadingKey = actionLoading === `install:${command.name}`;

								return (
									<div key={command.id ?? command.name} className="rounded-lg border bg-card p-4">
										<div className="flex items-start justify-between gap-4">
											<div className="min-w-0 flex-1 space-y-2">
												<div className="flex flex-wrap items-center gap-2">
													<span className="font-medium">/{command.name}</span>
													{command.latestVersion && (
														<Badge variant="secondary">{command.latestVersion}</Badge>
													)}
													{installed && (
														<span className="inline-flex items-center gap-1 rounded-full border border-emerald-500/20 bg-emerald-500/10 px-2 py-0.5 text-[11px] font-medium text-emerald-400">
															<Check className="size-3" />
															Installed
														</span>
													)}
												</div>
												<p className="line-clamp-2 text-sm text-muted-foreground">
													{command.description}
												</p>
												<div className="flex flex-wrap gap-1">
													{compatibleAgents.map((agent) => (
														<Badge key={agent} variant="outline" className="text-[11px]">
															{agent}
														</Badge>
													))}
													<Badge variant="outline" className="text-[11px]">
														{command.scope}
													</Badge>
													{command.totalVersions > 0 && (
														<Badge variant="outline" className="text-[11px]">
															{command.totalVersions} versions
														</Badge>
													)}
												</div>
											</div>
											<Button
												size="sm"
												onClick={() => handleInstall(command)}
												disabled={loadingKey || incompatible}
												title={incompatible ? `Not compatible with ${installAgent}` : undefined}
											>
												{loadingKey ? (
													<Loader2 className="size-3.5 animate-spin" />
												) : (
													<Download className="size-3.5" />
												)}
												Install
											</Button>
										</div>
									</div>
								);
							})}
						</div>
					)}
				</section>

				<section className="space-y-3">
					<div className="flex items-center justify-between">
						<h2 className="text-sm font-medium text-muted-foreground">Local installs</h2>
						<span className="text-xs text-muted-foreground">
							{filteredLocalCommands.length} installed
						</span>
					</div>

					{filteredLocalCommands.length === 0 ? (
						<EmptyState label="No local commands installed" />
					) : (
						<div className="space-y-2">
							{filteredLocalCommands.map((record) => {
								const updateLoading = actionLoading === `update:${commandKey(record)}`;
								const removeLoading = actionLoading === `remove:${commandKey(record)}`;
								return (
									<div key={commandKey(record)} className="rounded-lg border bg-card p-3">
										<div className="flex items-start justify-between gap-3">
											<div className="min-w-0 space-y-1">
												<div className="flex flex-wrap items-center gap-2">
													<span className="text-sm font-medium">/{record.name}</span>
													<Badge variant="secondary">{record.version}</Badge>
												</div>
												<p className="text-xs text-muted-foreground">
													{formatCommandRef(record.org, record.name)}
												</p>
												<div className="flex flex-wrap gap-1">
													<Badge variant="outline" className="text-[11px]">
														{record.agent}
													</Badge>
													<Badge variant="outline" className="text-[11px]">
														{record.scope}
													</Badge>
												</div>
												<p
													className="truncate font-mono text-[11px] text-muted-foreground/70"
													title={record.path}
												>
													{displayPath(record.path)}
												</p>
											</div>
											<div className="flex shrink-0 items-center gap-1">
												<Button
													variant="outline"
													size="sm"
													onClick={() => handleUpdate(record)}
													disabled={updateLoading || removeLoading}
													title="Update command"
												>
													{updateLoading ? (
														<Loader2 className="size-3.5 animate-spin" />
													) : (
														<RefreshCw className="size-3.5" />
													)}
												</Button>
												<Button
													variant="ghost"
													size="sm"
													onClick={() => handleRemove(record)}
													disabled={updateLoading || removeLoading}
													title="Remove command"
													className={cn("text-muted-foreground", "hover:text-destructive")}
												>
													{removeLoading ? (
														<Loader2 className="size-3.5 animate-spin" />
													) : (
														<Trash2 className="size-3.5" />
													)}
												</Button>
											</div>
										</div>
									</div>
								);
							})}
						</div>
					)}
				</section>
			</div>
		</div>
	);
}

function EmptyState({ label }: { label: string }) {
	return (
		<div className="flex flex-col items-center justify-center gap-2 rounded-lg border border-dashed py-10">
			<Terminal className="size-8 text-muted-foreground" />
			<p className="text-sm text-muted-foreground">{label}</p>
		</div>
	);
}
