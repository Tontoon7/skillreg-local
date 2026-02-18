import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { getEnvVars, setEnvVars } from "@/lib/api";
import type { EnvVarDecl } from "@/lib/types";
import { Key, Loader2, Save, SkipForward, X } from "lucide-react";
import { useEffect, useState } from "react";

interface EnvVarSetupDialogProps {
	skillName: string;
	org: string;
	envVars: EnvVarDecl[];
	onClose: () => void;
	onSaved: () => void;
}

export function EnvVarSetupDialog({
	skillName,
	org,
	envVars,
	onClose,
	onSaved,
}: EnvVarSetupDialogProps) {
	const [values, setValues] = useState<Record<string, string>>({});
	const [saving, setSaving] = useState(false);
	const [error, setError] = useState<string | null>(null);
	const [loaded, setLoaded] = useState(false);

	// Close on Escape
	useEffect(() => {
		const handler = (e: KeyboardEvent) => {
			if (e.key === "Escape") onClose();
		};
		window.addEventListener("keydown", handler);
		return () => window.removeEventListener("keydown", handler);
	}, [onClose]);

	// Pre-fill with defaults and existing values
	useEffect(() => {
		const init: Record<string, string> = {};
		for (const v of envVars) {
			init[v.name] = v.default ?? "";
		}

		getEnvVars(org, skillName)
			.then((existing) => {
				for (const [k, v] of Object.entries(existing)) {
					if (k in init) init[k] = v;
				}
			})
			.catch(() => {})
			.finally(() => {
				setValues(init);
				setLoaded(true);
			});
	}, [org, skillName, envVars]);

	const handleSave = async () => {
		// Validate required fields
		const missing = envVars.filter((v) => v.required && !values[v.name]?.trim());
		if (missing.length > 0) {
			setError(`Required: ${missing.map((v) => v.name).join(", ")}`);
			return;
		}

		// Only save non-empty values
		const toSave: Record<string, string> = {};
		for (const [k, v] of Object.entries(values)) {
			if (v.trim()) toSave[k] = v.trim();
		}

		if (Object.keys(toSave).length === 0) {
			onClose();
			return;
		}

		setSaving(true);
		setError(null);
		try {
			await setEnvVars(org, skillName, toSave);
			onSaved();
		} catch (e) {
			setError(typeof e === "string" ? e : "Failed to save variables");
		} finally {
			setSaving(false);
		}
	};

	return (
		<div className="fixed inset-0 z-50 flex items-center justify-center">
			<div className="absolute inset-0 bg-black/50" onClick={onClose} onKeyDown={undefined} />

			<div className="relative z-10 w-full max-w-lg rounded-xl border bg-card p-6 shadow-lg space-y-4">
				<div className="flex items-center justify-between">
					<div className="flex items-center gap-2">
						<Key className="size-4 text-muted-foreground" />
						<h2 className="text-base font-semibold">Configure Environment Variables</h2>
					</div>
					<Button variant="ghost" size="icon" onClick={onClose} className="size-7">
						<X className="size-4" />
					</Button>
				</div>

				<p className="text-sm text-muted-foreground">
					<span className="font-medium text-foreground">{skillName}</span> requires environment
					variables to work properly.
				</p>

				{!loaded ? (
					<div className="flex justify-center py-6">
						<Loader2 className="size-5 animate-spin text-muted-foreground" />
					</div>
				) : (
					<div className="space-y-3 max-h-80 overflow-y-auto">
						{envVars.map((v) => (
							<div key={v.name} className="space-y-1.5">
								<div className="flex items-center gap-2">
									<Label htmlFor={`env-${v.name}`} className="font-mono text-xs">
										{v.name}
									</Label>
									<Badge
										variant={v.required ? "default" : "outline"}
										className="text-[10px] px-1.5 py-0"
									>
										{v.required ? "required" : "optional"}
									</Badge>
								</div>
								{v.description && <p className="text-xs text-muted-foreground">{v.description}</p>}
								<Input
									id={`env-${v.name}`}
									placeholder={v.default || v.name}
									value={values[v.name] ?? ""}
									onChange={(e) => setValues((prev) => ({ ...prev, [v.name]: e.target.value }))}
									className="font-mono text-xs"
								/>
							</div>
						))}
					</div>
				)}

				{error && (
					<div className="rounded-lg border border-destructive/30 bg-destructive/5 p-3">
						<p className="text-sm text-destructive">{error}</p>
					</div>
				)}

				<div className="flex gap-3 pt-1">
					<Button variant="outline" size="sm" onClick={onClose} disabled={saving}>
						<SkipForward className="size-3.5" />
						Skip
					</Button>
					<Button size="sm" onClick={handleSave} disabled={saving || !loaded}>
						{saving ? <Loader2 className="size-3.5 animate-spin" /> : <Save className="size-3.5" />}
						Save
					</Button>
				</div>
			</div>
		</div>
	);
}
