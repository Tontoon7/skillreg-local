import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select } from "@/components/ui/select";
import { useAuthStore, useConfigStore } from "@/lib/store";
import type { AgentType, ScopeType } from "@/lib/types";
import { Loader2, LogOut, Save } from "lucide-react";
import { useState } from "react";

export function Settings() {
	const config = useConfigStore((s) => s.config);
	const update = useConfigStore((s) => s.update);
	const doLogout = useAuthStore((s) => s.logout);
	const user = useAuthStore((s) => s.user);
	const [saving, setSaving] = useState(false);

	const [org, setOrg] = useState(config.org || "");
	const [defaultAgent, setDefaultAgent] = useState<AgentType>(config.defaultAgent || "claude");
	const [defaultScope, setDefaultScope] = useState<ScopeType>(config.defaultScope || "project");

	const handleSave = async () => {
		setSaving(true);
		try {
			await update({ org, defaultAgent, defaultScope, setupDone: true });
		} finally {
			setSaving(false);
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
