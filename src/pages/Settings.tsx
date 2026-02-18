import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select } from "@/components/ui/select";
import { useAuthStore, useConfigStore } from "@/lib/store";
import type { AgentType, ScopeType } from "@/lib/types";
import { Loader2, LogOut, Moon, Save, Sun } from "lucide-react";
import { useEffect, useState } from "react";

export function Settings() {
	const config = useConfigStore((s) => s.config);
	const update = useConfigStore((s) => s.update);
	const doLogout = useAuthStore((s) => s.logout);
	const user = useAuthStore((s) => s.user);
	const [saving, setSaving] = useState(false);

	const [org, setOrg] = useState(config.org || "");
	const [defaultAgent, setDefaultAgent] = useState<AgentType>(config.defaultAgent || "claude");
	const [defaultScope, setDefaultScope] = useState<ScopeType>(config.defaultScope || "project");

	const [theme, setTheme] = useState<"dark" | "light">(() => {
		return document.documentElement.classList.contains("light") ? "light" : "dark";
	});

	useEffect(() => {
		if (theme === "light") {
			document.documentElement.classList.add("light");
		} else {
			document.documentElement.classList.remove("light");
		}
		localStorage.setItem("skillreg-theme", theme);
	}, [theme]);

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
				<div className="rounded-xl border bg-card p-4">
					<p className="text-sm font-medium">{user.user.name || user.user.email}</p>
					<p className="text-xs text-muted-foreground">{user.user.email}</p>
				</div>
			)}

			{/* Config */}
			<div className="rounded-xl border bg-card p-6 space-y-4">
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

			{/* Appearance */}
			<div className="rounded-xl border bg-card p-6 space-y-3">
				<Label>Appearance</Label>
				<div className="flex gap-2">
					<button
						type="button"
						onClick={() => setTheme("dark")}
						className={`flex flex-1 items-center justify-center gap-2 rounded-lg border p-3 text-sm transition-colors ${
							theme === "dark"
								? "border-primary bg-primary/5 text-foreground"
								: "text-muted-foreground hover:border-muted-foreground/30"
						}`}
					>
						<Moon className="size-4" />
						Dark
					</button>
					<button
						type="button"
						onClick={() => setTheme("light")}
						className={`flex flex-1 items-center justify-center gap-2 rounded-lg border p-3 text-sm transition-colors ${
							theme === "light"
								? "border-primary bg-primary/5 text-foreground"
								: "text-muted-foreground hover:border-muted-foreground/30"
						}`}
					>
						<Sun className="size-4" />
						Light
					</button>
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
