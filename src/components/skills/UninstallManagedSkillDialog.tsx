import { Button } from "@/components/ui/button";
import { AlertTriangle, Loader2, X } from "lucide-react";
import { useEffect } from "react";

interface UninstallManagedSkillDialogProps {
	skillName: string;
	busy: boolean;
	onClose: () => void;
	onConfirm: () => void;
}

export function UninstallManagedSkillDialog({
	skillName,
	busy,
	onClose,
	onConfirm,
}: UninstallManagedSkillDialogProps) {
	useEffect(() => {
		const handleEscape = (event: KeyboardEvent) => {
			if (event.key === "Escape" && !busy) onClose();
		};
		window.addEventListener("keydown", handleEscape);
		return () => window.removeEventListener("keydown", handleEscape);
	}, [busy, onClose]);

	return (
		<div className="fixed inset-0 z-50 flex items-center justify-center p-4">
			<button
				type="button"
				aria-label="Fermer la confirmation"
				className="absolute inset-0 bg-black/65"
				onClick={onClose}
				disabled={busy}
			/>
			<dialog
				open
				aria-modal="true"
				aria-labelledby="managed-uninstall-title"
				aria-describedby="managed-uninstall-description"
				className="panel-raised relative z-10 m-0 w-full max-w-md space-y-5 rounded-xl p-6 text-foreground shadow-xl"
			>
				<header className="flex items-start justify-between gap-4">
					<div className="flex gap-3">
						<div className="flex size-9 shrink-0 items-center justify-center rounded-lg bg-destructive/10 text-destructive">
							<AlertTriangle className="size-4" />
						</div>
						<div className="space-y-1">
							<h2 id="managed-uninstall-title" className="text-base font-semibold">
								Désinstaller {skillName} ?
							</h2>
							<p id="managed-uninstall-description" className="text-sm text-muted-foreground">
								La skill ne sera plus disponible dans vos assistants.
							</p>
						</div>
					</div>
					<Button
						variant="ghost"
						size="icon"
						aria-label="Fermer"
						onClick={onClose}
						disabled={busy}
						className="size-8"
					>
						<X className="size-4" />
					</Button>
				</header>

				<p className="rounded-lg border border-border bg-background/40 p-3 text-sm text-muted-foreground">
					Vos accès enregistrés seront conservés. Une prochaine réinstallation pourra les
					réutiliser.
				</p>

				<div className="flex justify-end gap-2">
					<Button variant="outline" onClick={onClose} disabled={busy}>
						Annuler
					</Button>
					<Button variant="destructive" onClick={onConfirm} disabled={busy}>
						{busy && <Loader2 className="size-4 animate-spin" />}
						Confirmer la désinstallation
					</Button>
				</div>
			</dialog>
		</div>
	);
}
