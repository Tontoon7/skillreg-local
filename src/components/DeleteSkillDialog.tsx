import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { deleteSkill } from "@/lib/api";
import { notify } from "@/lib/notifications";
import { AlertTriangle, Loader2, Trash2, X } from "lucide-react";
import { useEffect, useState } from "react";

interface DeleteSkillDialogProps {
	skillName: string;
	org: string;
	onClose: () => void;
	onDeleted: () => void;
}

export function DeleteSkillDialog({ skillName, org, onClose, onDeleted }: DeleteSkillDialogProps) {
	const [confirmText, setConfirmText] = useState("");
	const [loading, setLoading] = useState(false);
	const [error, setError] = useState<string | null>(null);

	// Close on Escape
	useEffect(() => {
		const handler = (e: KeyboardEvent) => {
			if (e.key === "Escape") onClose();
		};
		window.addEventListener("keydown", handler);
		return () => window.removeEventListener("keydown", handler);
	}, [onClose]);

	const ref = `@${org}/${skillName}`;
	const canDelete = confirmText === skillName && !loading;

	const handleDelete = async () => {
		if (!canDelete) return;
		setLoading(true);
		setError(null);
		try {
			await deleteSkill(org, skillName);
			notify("Skill deleted", ref);
			onDeleted();
		} catch (e) {
			const msg = typeof e === "string" ? e : "Delete failed";
			setError(msg);
			notify("Delete failed", msg);
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
					<h2 className="flex items-center gap-2 text-base font-semibold">
						<AlertTriangle className="size-4 text-destructive" />
						Delete skill
					</h2>
					<Button variant="ghost" size="icon" onClick={onClose} className="size-7">
						<X className="size-4" />
					</Button>
				</div>

				<div className="rounded-lg border border-destructive/30 bg-destructive/5 p-3 space-y-1">
					<p className="text-sm">
						This permanently deletes <span className="font-medium text-foreground">{ref}</span> and{" "}
						<span className="font-medium text-foreground">all its versions</span> from the registry.
					</p>
					<p className="text-xs text-muted-foreground">This action cannot be undone.</p>
				</div>

				<div className="space-y-1.5">
					<Label htmlFor="delete-confirm">
						Type <span className="font-mono text-foreground">{skillName}</span> to confirm
					</Label>
					<Input
						id="delete-confirm"
						value={confirmText}
						placeholder={skillName}
						onChange={(e) => {
							setConfirmText(e.target.value);
							setError(null);
						}}
					/>
				</div>

				{error && (
					<div className="rounded-lg border border-destructive/30 bg-destructive/5 p-3">
						<p className="text-sm text-destructive">{error}</p>
					</div>
				)}

				<div className="flex justify-end gap-3 pt-1">
					<Button variant="outline" size="sm" onClick={onClose} disabled={loading}>
						Cancel
					</Button>
					<Button variant="destructive" size="sm" onClick={handleDelete} disabled={!canDelete}>
						{loading ? (
							<Loader2 className="size-3.5 animate-spin" />
						) : (
							<Trash2 className="size-3.5" />
						)}
						Delete
					</Button>
				</div>
			</div>
		</div>
	);
}
