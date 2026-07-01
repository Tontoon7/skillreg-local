import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select } from "@/components/ui/select";
import { runAutoUpdateNow, setLaunchAtLogin } from "@/lib/api";
import { useAuthStore, useConfigStore } from "@/lib/store";
import type { AgentType, AutoUpdateRunSummary, ScopeType } from "@/lib/types";
import { Loader2, LogOut, RefreshCw, Save } from "lucide-react";
import { useEffect, useState } from "react";

const AUTO_UPDATE_INTERVALS = [
	{ label: "15 min", value: 15 },
	{ label: "30 min", value: 30 },
	{ label: "1 hour", value: 60 },
	{ label: "6 hours", value: 360 },
	{ label: "24 hours", value: 1440 },
];

export function Settings() {
	const config = useConfigStore((s) => s.config);
	const update = useConfigStore((s) => s.update);
	const doLogout = useAuthStore((s) => s.logout);
	const user = useAuthStore((s) => s.user);
	const [saving, setSaving] = useState(false);
	const [checking, setChecking] = useState(false);
	const [autoUpdateSummary, setAutoUpdateSummary] = useState<AutoUpdateRunSummary | null>(null);
	const [autoUpdateError, setAutoUpdateError] = useState<string | null>(null);

	const [org, setOrg] = useState(config.org || "");
	const [defaultAgent, setDefaultAgent] = useState<AgentType>(config.defaultAgent || "claude");
	const [defaultScope, setDefaultScope] = useState<ScopeType>(config.defaultScope || "project");
	const [autoUpdateEnabled, setAutoUpdateEnabled] = useState(config.autoUpdateEnabled ?? true);
	const [autoUpdateIntervalMinutes, setAutoUpdateIntervalMinutes] = useState(
		config.autoUpdateIntervalMinutes ?? 60,
	);
	const [launchAtLogin, setLaunchAtLoginValue] = useState(config.launchAtLogin ?? false);

	useEffect(() => {
		setOrg(config.org || "");
		setDefaultAgent(config.defaultAgent || "claude");
		setDefaultScope(config.defaultScope || "project");
		setAutoUpdateEnabled(config.autoUpdateEnabled ?? true);
		setAutoUpdateIntervalMinutes(config.autoUpdateIntervalMinutes ?? 60);
		setLaunchAtLoginValue(config.launchAtLogin ?? false);
	}, [config]);

	const handleSave = async () => {
		setSaving(true);
		try {
			await update({
				org,
				defaultAgent,
				defaultScope,
				setupDone: true,
				autoUpdateEnabled,
				autoUpdateIntervalMinutes,
				launchAtLogin,
			});
			await setLaunchAtLogin(launchAtLogin);
		} finally {
			setSaving(false);
		}
	};

	const handleCheckNow = async () => {
		setChecking(true);
		setAutoUpdateError(null);
		try {
			const summary = await runAutoUpdateNow();
			setAutoUpdateSummary(summary);
		} catch (e) {
			setAutoUpdateError(typeof e === "string" ? e : "Update check failed");
		} finally {
			setChecking(false);
		}
	};

	return (
		<div className="flex flex-col gap-6 p-6 max-w-lg">
			<h1 className="text-lg font-semibold">Settings</h1>

			{/* Account */}
			{user && (
				<div className="rounded-xl panel-inset p-4">
					<p className="text-sm font-medium">{user.user.name || user.user.email}</p>
					<p className="text-xs text-muted-foreground">{user.user.email}</p>
				</div>
			)}

			{/* Config */}
			<div className="rounded-xl panel-inset p-6 space-y-4">
				<div className="space-y-2">
					<Label htmlFor="org">Organization</Label>
					<Input
						id="org"
						value={org}
						onChange={(e) => setOrg(e.target.value)}
						placeholder="my-org"
					/>
				</div>

				<div className="space-y-2">
					<Label htmlFor="agent">Default Agent</Label>
					<Select
						id="agent"
						value={defaultAgent}
						onChange={(e) => setDefaultAgent(e.target.value as AgentType)}
					>
						<option value="claude">Claude</option>
						<option value="codex">Codex</option>
						<option value="cursor">Cursor</option>
					</Select>
				</div>

				<div className="space-y-2">
					<Label htmlFor="scope">Default Scope</Label>
					<Select
						id="scope"
						value={defaultScope}
						onChange={(e) => setDefaultScope(e.target.value as ScopeType)}
					>
						<option value="project">Project</option>
						<option value="user">User</option>
					</Select>
				</div>
			</div>

			<div className="rounded-xl panel-inset p-6 space-y-4">
				<div className="flex items-center justify-between gap-4">
					<Label htmlFor="auto-update-enabled">Automatic skill updates</Label>
					<input
						id="auto-update-enabled"
						type="checkbox"
						checked={autoUpdateEnabled}
						onChange={(e) => setAutoUpdateEnabled(e.target.checked)}
						className="size-4 accent-primary"
					/>
				</div>

				<div className="space-y-2">
					<Label htmlFor="auto-update-interval">Check interval</Label>
					<Select
						id="auto-update-interval"
						value={String(autoUpdateIntervalMinutes)}
						onChange={(e) => setAutoUpdateIntervalMinutes(Number(e.target.value))}
						disabled={!autoUpdateEnabled}
					>
						{AUTO_UPDATE_INTERVALS.map((interval) => (
							<option key={interval.value} value={interval.value}>
								{interval.label}
							</option>
						))}
					</Select>
				</div>

				<div className="flex items-center justify-between gap-4">
					<Label htmlFor="launch-at-login">Launch at login</Label>
					<input
						id="launch-at-login"
						type="checkbox"
						checked={launchAtLogin}
						onChange={(e) => setLaunchAtLoginValue(e.target.checked)}
						className="size-4 accent-primary"
					/>
				</div>

				<div className="flex items-center gap-3">
					<Button variant="outline" onClick={handleCheckNow} disabled={checking}>
						{checking ? (
							<Loader2 className="size-4 animate-spin" />
						) : (
							<RefreshCw className="size-4" />
						)}
						Check now
					</Button>
					{autoUpdateSummary && (
						<p className="text-xs text-muted-foreground">
							Last check: {autoUpdateSummary.checked} checked · {autoUpdateSummary.updated} updated
							· {autoUpdateSummary.skipped} skipped
							{autoUpdateSummary.failed > 0 ? ` · ${autoUpdateSummary.failed} failed` : ""}
						</p>
					)}
					{autoUpdateError && <p className="text-xs text-destructive">{autoUpdateError}</p>}
				</div>
			</div>

			{/* Actions */}
			<div className="flex items-center gap-3">
				<Button onClick={handleSave} disabled={saving}>
					{saving ? <Loader2 className="size-4 animate-spin" /> : <Save className="size-4" />}
					Save settings
				</Button>

				<Button variant="outline" onClick={doLogout}>
					<LogOut className="size-4" />
					Sign out
				</Button>
			</div>
		</div>
	);
}
