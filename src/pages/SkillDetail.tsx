import { EnvVarSetupDialog } from "@/components/EnvVarSetupDialog";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Select } from "@/components/ui/select";
import { getSkill, pullSkill } from "@/lib/api";
import { useConfigStore } from "@/lib/store";
import type { AgentType, EnvVarDecl, ScopeType, SkillDetail as SkillDetailType } from "@/lib/types";
import { cn } from "@/lib/utils";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import {
	ArrowLeft,
	Check,
	Clock,
	Download,
	FileText,
	FolderOpen,
	Hash,
	Loader2,
	Package,
	User,
} from "lucide-react";
import React, { useEffect, useMemo, useState } from "react";
import Markdown from "react-markdown";
import { useNavigate, useParams } from "react-router";
import rehypeSanitize from "rehype-sanitize";

type Tab = "instructions" | "versions" | "files";

// Strip YAML frontmatter and return metadata + body separately
function splitFrontmatter(content: string): {
	meta: Record<string, string>;
	body: string;
} {
	const match = content.match(/^---\s*\n([\s\S]*?)\n---\s*\n?([\s\S]*)$/);
	if (!match) return { meta: {}, body: content };

	const meta: Record<string, string> = {};
	let currentKey = "";
	let currentVal = "";

	for (const line of match[1].split("\n")) {
		// Indented line = continuation of previous value
		if (/^\s{2,}/.test(line) && currentKey) {
			const trimmed = line.trim();
			if (trimmed) {
				currentVal += (currentVal ? " " : "") + trimmed;
				meta[currentKey] = currentVal;
			}
			continue;
		}
		// Top-level key: value
		const kv = line.match(/^([a-zA-Z][\w.-]*)\s*:\s*(.*)$/);
		if (kv) {
			currentKey = kv[1];
			const raw = kv[2].trim();
			if (raw === "|" || raw === ">") {
				currentVal = "";
				meta[currentKey] = "";
			} else {
				currentVal = raw.replace(/^["']|["']$/g, "");
				meta[currentKey] = currentVal;
			}
		}
	}

	return { meta, body: match[2] };
}

class ErrorBoundary extends React.Component<
	{ children: React.ReactNode; fallback?: React.ReactNode },
	{ error: string | null }
> {
	state = { error: null as string | null };
	static getDerivedStateFromError(err: Error) {
		return { error: `${err.message}\n${err.stack}` };
	}
	render() {
		if (this.state.error) {
			return (
				this.props.fallback ?? (
					<div className="p-6 space-y-2">
						<p className="text-sm text-destructive font-medium">Rendering error</p>
						<pre className="text-xs text-muted-foreground bg-muted rounded p-3 overflow-auto whitespace-pre-wrap">
							{this.state.error}
						</pre>
					</div>
				)
			);
		}
		return this.props.children;
	}
}

export function SkillDetailPage() {
	return (
		<ErrorBoundary>
			<SkillDetailInner />
		</ErrorBoundary>
	);
}

function SkillDetailInner() {
	const { name } = useParams<{ name: string }>();
	const org = useConfigStore((s) => s.config.org);
	const config = useConfigStore((s) => s.config);
	const navigate = useNavigate();

	const [skill, setSkill] = useState<SkillDetailType | null>(null);
	const [loading, setLoading] = useState(true);
	const [error, setError] = useState<string | null>(null);
	const [tab, setTab] = useState<Tab>("instructions");

	const [agent, setAgent] = useState<AgentType>(config.defaultAgent || "claude");
	const [scope, setScope] = useState<ScopeType>(config.defaultScope || "project");
	const [selectedVersion, setSelectedVersion] = useState<string>("");
	const [projectDir, setProjectDir] = useState<string | null>(null);
	const [installing, setInstalling] = useState(false);
	const [installed, setInstalled] = useState(false);
	const [installError, setInstallError] = useState<string | null>(null);
	const [detectedEnvVars, setDetectedEnvVars] = useState<EnvVarDecl[]>([]);
	const [showEnvDialog, setShowEnvDialog] = useState(false);

	useEffect(() => {
		if (!org || !name) return;
		setLoading(true);
		getSkill(org, name)
			.then((s) => {
				setSkill(s);
				if (s.latestVersion) setSelectedVersion(s.latestVersion);
			})
			.catch((e) => setError(typeof e === "string" ? e : "Failed to load skill"))
			.finally(() => setLoading(false));
	}, [org, name]);

	const parsed = useMemo(() => {
		const raw = skill?.latestVersionData?.skillMdContent;
		if (!raw) return null;
		try {
			return splitFrontmatter(raw);
		} catch {
			return { meta: {} as Record<string, string>, body: raw };
		}
	}, [skill?.latestVersionData?.skillMdContent]);

	const pickProjectDir = async () => {
		const selected = await openDialog({ directory: true, title: "Select project folder" });
		if (typeof selected === "string") {
			setProjectDir(selected);
			setInstalled(false);
			setInstallError(null);
		}
	};

	const handleInstall = async () => {
		if (!org || !name) return;
		if (scope === "project" && !projectDir) {
			setInstallError("Select a project folder first");
			return;
		}
		setInstalling(true);
		setInstallError(null);
		try {
			const result = await pullSkill({
				org,
				name,
				version: selectedVersion || undefined,
				agent,
				scope,
				projectDir: scope === "project" ? (projectDir ?? undefined) : undefined,
			});
			setInstalled(true);
			if (result.envVars.length > 0) {
				setDetectedEnvVars(result.envVars);
				setShowEnvDialog(true);
			}
		} catch (e) {
			setInstallError(typeof e === "string" ? e : "Install failed");
		} finally {
			setInstalling(false);
		}
	};

	if (loading) {
		return (
			<div className="flex items-center justify-center h-full">
				<Loader2 className="size-6 animate-spin text-muted-foreground" />
			</div>
		);
	}

	if (error || !skill) {
		return (
			<div className="flex flex-col items-center justify-center gap-3 h-full">
				<p className="text-sm text-destructive">{error || "Skill not found"}</p>
				<Button variant="outline" size="sm" onClick={() => navigate("/catalog")}>
					<ArrowLeft className="size-4" />
					Back to catalog
				</Button>
			</div>
		);
	}

	const versionData = skill.latestVersionData;
	const meta = parsed?.meta ?? {};
	const hasExtraMeta = meta.compatibility || meta.metadata || meta.author;

	return (
		<div className="flex flex-col h-full">
			{/* Header */}
			<div className="flex items-center gap-3 border-b p-4">
				<Button variant="ghost" size="sm" onClick={() => navigate("/catalog")}>
					<ArrowLeft className="size-4" />
				</Button>
				<div className="flex-1 min-w-0">
					<div className="flex items-center gap-2">
						<h1 className="text-lg font-semibold truncate">{skill.name}</h1>
						{skill.latestVersion && <Badge variant="secondary">{skill.latestVersion}</Badge>}
						{skill.isDeprecated && <Badge variant="destructive">Deprecated</Badge>}
					</div>
					{skill.description && (
						<p className="text-sm text-muted-foreground truncate">{skill.description}</p>
					)}
				</div>
			</div>

			<div className="flex flex-1 overflow-hidden">
				{/* Main content */}
				<div className="flex-1 overflow-auto">
					{/* Tabs */}
					<div className="flex border-b px-4">
						{(["instructions", "versions", "files"] as const).map((t) => (
							<button
								key={t}
								type="button"
								onClick={() => setTab(t)}
								className={cn(
									"border-b-2 px-4 py-2.5 text-sm font-medium capitalize transition-colors",
									tab === t
										? "border-primary text-foreground"
										: "border-transparent text-muted-foreground hover:text-foreground",
								)}
							>
								{t}
							</button>
						))}
					</div>

					<div className="p-6">
						{tab === "instructions" && (
							<div>
								{/* Frontmatter metadata card */}
								{hasExtraMeta && (
									<div className="flex flex-wrap gap-x-6 gap-y-2 rounded-lg border bg-card p-4 mb-6 text-sm">
										{meta.compatibility && (
											<div className="flex items-center gap-1.5 text-muted-foreground">
												<Package className="size-3.5 shrink-0" />
												<span>{meta.compatibility}</span>
											</div>
										)}
										{(meta.author || meta.metadata) && (
											<div className="flex items-center gap-1.5 text-muted-foreground">
												<User className="size-3.5 shrink-0" />
												<span>{meta.author || "—"}</span>
											</div>
										)}
									</div>
								)}

								{/* Markdown body */}
								{parsed?.body?.trim() ? (
									<div className="prose max-w-none">
										<Markdown rehypePlugins={[rehypeSanitize]}>{parsed.body}</Markdown>
									</div>
								) : (
									<p className="text-muted-foreground">No instructions available</p>
								)}
							</div>
						)}

						{tab === "versions" && (
							<div className="space-y-2">
								{skill.versions && skill.versions.length > 0 ? (
									skill.versions.map((v) => (
										<div
											key={v.id}
											className="flex items-center justify-between rounded-lg border p-3"
										>
											<div className="flex items-center gap-3">
												<Badge variant={v.status === "approved" ? "secondary" : "outline"}>
													{v.version}
												</Badge>
												<span className="text-xs text-muted-foreground capitalize">{v.status}</span>
											</div>
											<div className="flex items-center gap-4 text-xs text-muted-foreground">
												<span className="flex items-center gap-1">
													<Download className="size-3" />
													{v.downloads}
												</span>
												<span>{formatBytes(v.tarballSize)}</span>
												<span className="flex items-center gap-1">
													<Clock className="size-3" />
													{formatDate(v.createdAt)}
												</span>
											</div>
										</div>
									))
								) : (
									<p className="text-sm text-muted-foreground">No versions</p>
								)}
							</div>
						)}

						{tab === "files" && (
							<div className="space-y-1">
								{versionData?.filesManifest && versionData.filesManifest.length > 0 ? (
									versionData.filesManifest.map((f) => (
										<div
											key={f}
											className="flex items-center gap-2 rounded px-2 py-1 text-sm text-muted-foreground hover:bg-muted/50"
										>
											<FileText className="size-3.5 shrink-0" />
											<span className="truncate font-mono text-xs">{f}</span>
										</div>
									))
								) : (
									<p className="text-sm text-muted-foreground">No file manifest</p>
								)}
							</div>
						)}
					</div>
				</div>

				{/* Sidebar metadata */}
				<aside className="w-64 shrink-0 border-l overflow-auto p-4 space-y-5">
					{/* Install */}
					<div className="space-y-3">
						<div className="space-y-1.5">
							<label htmlFor="install-agent" className="text-xs font-medium text-muted-foreground">
								Agent
							</label>
							<Select
								id="install-agent"
								value={agent}
								onChange={(e) => setAgent(e.target.value as AgentType)}
							>
								<option value="claude">Claude</option>
								<option value="codex">Codex</option>
								<option value="cursor">Cursor</option>
							</Select>
						</div>
						<div className="space-y-1.5">
							<label htmlFor="install-scope" className="text-xs font-medium text-muted-foreground">
								Scope
							</label>
							<Select
								id="install-scope"
								value={scope}
								onChange={(e) => {
									setScope(e.target.value as ScopeType);
									setInstalled(false);
									setInstallError(null);
								}}
							>
								<option value="project">Project</option>
								<option value="user">User</option>
							</Select>
						</div>
						{scope === "project" && (
							<div className="space-y-1.5">
								<span className="text-xs font-medium text-muted-foreground">Project folder</span>
								<Button
									variant="outline"
									size="sm"
									className="w-full justify-start gap-2 font-normal"
									onClick={pickProjectDir}
								>
									<FolderOpen className="size-3.5 shrink-0" />
									<span className="truncate text-xs">
										{projectDir ? projectDir.split("/").pop() : "Select folder..."}
									</span>
								</Button>
								{projectDir && (
									<p className="truncate text-[11px] text-muted-foreground" title={projectDir}>
										{projectDir}
									</p>
								)}
							</div>
						)}
						<div className="space-y-1.5">
							<label
								htmlFor="install-version"
								className="text-xs font-medium text-muted-foreground"
							>
								Version
							</label>
							<Select
								id="install-version"
								value={selectedVersion}
								onChange={(e) => {
									setSelectedVersion(e.target.value);
									setInstalled(false);
									setInstallError(null);
								}}
							>
								{skill.versions?.filter((v) => v.status === "approved").length ? (
									skill.versions
										.filter((v) => v.status === "approved")
										.map((v) => (
											<option key={v.id} value={v.version}>
												{v.version}
												{v.version === skill.latestVersion ? " (latest)" : ""}
											</option>
										))
								) : (
									<option value={skill.latestVersion || ""}>{skill.latestVersion || "—"}</option>
								)}
							</Select>
						</div>

						<Button className="w-full" onClick={handleInstall} disabled={installing || installed}>
							{installing ? (
								<Loader2 className="size-4 animate-spin" />
							) : installed ? (
								<Check className="size-4" />
							) : (
								<Download className="size-4" />
							)}
							{installed
								? "Installed"
								: installing
									? "Installing..."
									: `Install ${selectedVersion || ""}`}
						</Button>
						{installError && <p className="text-xs text-destructive">{installError}</p>}
					</div>

					{/* Metadata */}
					<div className="space-y-3 text-sm">
						<MetaRow icon={Download} label="Downloads" value={String(skill.totalDownloads)} />
						<MetaRow icon={Package} label="Versions" value={String(skill.totalVersions)} />
						{versionData && (
							<MetaRow icon={FileText} label="Size" value={formatBytes(versionData.tarballSize)} />
						)}
						{versionData?.sha256 && (
							<MetaRow icon={Hash} label="SHA256" value={`${versionData.sha256.slice(0, 12)}...`} />
						)}
						<MetaRow icon={Clock} label="Updated" value={formatDate(skill.updatedAt)} />
					</div>

					{/* Tags */}
					{skill.tags.length > 0 && (
						<div className="space-y-2">
							<p className="text-xs font-medium text-muted-foreground">Tags</p>
							<div className="flex flex-wrap gap-1">
								{skill.tags.map((tag) => (
									<Badge key={tag} variant="outline" className="text-xs">
										{tag}
									</Badge>
								))}
							</div>
						</div>
					)}
				</aside>
			</div>

			{showEnvDialog && org && name && (
				<EnvVarSetupDialog
					skillName={name}
					org={org}
					envVars={detectedEnvVars}
					onClose={() => setShowEnvDialog(false)}
					onSaved={() => setShowEnvDialog(false)}
				/>
			)}
		</div>
	);
}

function MetaRow({
	icon: Icon,
	label,
	value,
}: { icon: React.ComponentType<{ className?: string }>; label: string; value: string }) {
	return (
		<div className="flex items-center justify-between">
			<span className="flex items-center gap-1.5 text-muted-foreground">
				<Icon className="size-3.5" />
				{label}
			</span>
			<span className="text-foreground font-mono text-xs">{value}</span>
		</div>
	);
}

function formatBytes(bytes: number): string {
	if (bytes < 1024) return `${bytes} B`;
	if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
	return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function formatDate(iso: string): string {
	return new Date(iso).toLocaleDateString("en-US", {
		month: "short",
		day: "numeric",
		year: "numeric",
	});
}
