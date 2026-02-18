import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select } from "@/components/ui/select";
import { pushSkill } from "@/lib/api";
import { notify } from "@/lib/notifications";
import type { LocalSkill, PushResult } from "@/lib/types";
import { Check, Loader2, Package, Upload, X } from "lucide-react";
import { useEffect, useState } from "react";

interface PublishDialogProps {
	skill: LocalSkill;
	org: string;
	onClose: () => void;
	onPublished: () => void;
}

export function PublishDialog({ skill, org, onClose, onPublished }: PublishDialogProps) {
	const [version, setVersion] = useState(skill.version);
	const [tag, setTag] = useState("latest");
	const [loading, setLoading] = useState(false);
	const [dryRunResult, setDryRunResult] = useState<PushResult | null>(null);
	const [publishResult, setPublishResult] = useState<PushResult | null>(null);
	const [error, setError] = useState<string | null>(null);

	// Close on Escape
	useEffect(() => {
		const handler = (e: KeyboardEvent) => {
			if (e.key === "Escape") onClose();
		};
		window.addEventListener("keydown", handler);
		return () => window.removeEventListener("keydown", handler);
	}, [onClose]);

	const handleDryRun = async () => {
		setLoading(true);
		setError(null);
		setDryRunResult(null);
		try {
			const result = await pushSkill({
				org,
				dirPath: skill.path,
				version: version || undefined,
				tag: tag || undefined,
				dryRun: true,
			});
			setDryRunResult(result);
		} catch (e) {
			setError(typeof e === "string" ? e : "Dry run failed");
		} finally {
			setLoading(false);
		}
	};

	const handlePublish = async () => {
		setLoading(true);
		setError(null);
		setPublishResult(null);
		try {
			const result = await pushSkill({
				org,
				dirPath: skill.path,
				version: version || undefined,
				tag: tag || undefined,
				dryRun: false,
			});
			setPublishResult(result);
			onPublished();
			notify("Skill published", `${result.name} v${result.version}`);
		} catch (e) {
			const msg = typeof e === "string" ? e : "Publish failed";
			setError(msg);
			notify("Publish failed", msg);
		} finally {
			setLoading(false);
		}
	};

	return (
		<div className="fixed inset-0 z-50 flex items-center justify-center">
			{/* Backdrop */}
			<div className="absolute inset-0 bg-black/50" onClick={onClose} onKeyDown={undefined} />

			{/* Panel */}
			<div className="relative z-10 w-full max-w-md rounded-xl border bg-card p-6 shadow-lg space-y-4">
				<div className="flex items-center justify-between">
					<h2 className="text-base font-semibold">Publish Skill</h2>
					<Button variant="ghost" size="icon" onClick={onClose} className="size-7">
						<X className="size-4" />
					</Button>
				</div>

				{/* Skill info */}
				<div className="rounded-lg border p-3 space-y-1">
					<div className="flex items-center gap-2">
						<span className="text-sm font-medium">{skill.name}</span>
						<Badge variant="secondary">{skill.version}</Badge>
					</div>
					<p className="text-xs text-muted-foreground/60 font-mono truncate">{skill.path}</p>
					{skill.tags.length > 0 && (
						<div className="flex flex-wrap gap-1 mt-1">
							{skill.tags.slice(0, 4).map((t) => (
								<Badge key={t} variant="outline" className="text-xs">
									{t}
								</Badge>
							))}
						</div>
					)}
				</div>

				{/* Version + Tag */}
				<div className="grid grid-cols-2 gap-3">
					<div className="space-y-1.5">
						<Label htmlFor="pub-version">Version</Label>
						<Input
							id="pub-version"
							placeholder="1.0.0"
							value={version}
							onChange={(e) => {
								setVersion(e.target.value);
								setDryRunResult(null);
								setPublishResult(null);
								setError(null);
							}}
						/>
					</div>
					<div className="space-y-1.5">
						<Label htmlFor="pub-tag">Tag</Label>
						<Select id="pub-tag" value={tag} onChange={(e) => setTag(e.target.value)}>
							<option value="latest">latest</option>
							<option value="beta">beta</option>
							<option value="alpha">alpha</option>
						</Select>
					</div>
				</div>

				<p className="text-xs text-muted-foreground">
					Publishing to <span className="font-medium text-foreground">{org}</span>
				</p>

				{/* Dry run result */}
				{dryRunResult && (
					<div className="rounded-lg border border-accent/30 bg-accent/5 p-3 space-y-1">
						<p className="text-sm font-medium text-accent">Dry run successful</p>
						<div className="text-sm space-y-0.5">
							<p>
								<span className="text-muted-foreground">Version:</span> {dryRunResult.version}
							</p>
							<p>
								<span className="text-muted-foreground">Size:</span>{" "}
								{formatBytes(dryRunResult.size)}
							</p>
							<p className="font-mono text-xs">
								<span className="text-muted-foreground">SHA256:</span> {dryRunResult.sha256}
							</p>
						</div>
					</div>
				)}

				{/* Publish result */}
				{publishResult && (
					<div className="rounded-lg border border-accent/30 bg-accent/5 p-3 space-y-1">
						<div className="flex items-center gap-2 text-accent">
							<Check className="size-4" />
							<span className="text-sm font-medium">Published successfully</span>
						</div>
						<p className="text-sm">
							{publishResult.name}@{publishResult.version} — {formatBytes(publishResult.size)}
						</p>
					</div>
				)}

				{/* Error */}
				{error && (
					<div className="rounded-lg border border-destructive/30 bg-destructive/5 p-3">
						<p className="text-sm text-destructive">{error}</p>
					</div>
				)}

				{/* Actions */}
				<div className="flex gap-3 pt-1">
					<Button
						variant="outline"
						size="sm"
						onClick={handleDryRun}
						disabled={loading || !!publishResult}
					>
						{loading ? (
							<Loader2 className="size-3.5 animate-spin" />
						) : (
							<Package className="size-3.5" />
						)}
						Dry Run
					</Button>
					<Button size="sm" onClick={handlePublish} disabled={loading || !!publishResult}>
						{loading ? (
							<Loader2 className="size-3.5 animate-spin" />
						) : (
							<Upload className="size-3.5" />
						)}
						Publish
					</Button>
				</div>
			</div>
		</div>
	);
}

function formatBytes(bytes: number): string {
	if (bytes < 1024) return `${bytes} B`;
	if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
	return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
