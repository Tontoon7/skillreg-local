import { Button } from "@/components/ui/button";
import { AlertTriangle, Loader2, X } from "lucide-react";
import { useEffect, useRef } from "react";

interface UninstallManagedSkillDialogProps {
	skillName: string;
	busy: boolean;
	fallbackFocusTo?: HTMLElement | null;
	onClose: () => void;
	onConfirm: () => void;
}

export function UninstallManagedSkillDialog({
	skillName,
	busy,
	fallbackFocusTo,
	onClose,
	onConfirm,
}: UninstallManagedSkillDialogProps) {
	const dialogRef = useRef<HTMLDialogElement>(null);
	const cancelRef = useRef<HTMLButtonElement>(null);

	useEffect(() => {
		const dialog = dialogRef.current;
		if (!dialog) return;
		const previousFocus = document.activeElement;
		dialog.showModal();
		cancelRef.current?.focus();
		return () => {
			dialog.close();
			if (previousFocus instanceof HTMLElement && previousFocus.isConnected) {
				previousFocus.focus();
			} else {
				fallbackFocusTo?.focus();
			}
		};
	}, [fallbackFocusTo]);

	return (
		<dialog
			ref={dialogRef}
			aria-modal="true"
			aria-busy={busy}
			aria-labelledby="managed-uninstall-title"
			aria-describedby="managed-uninstall-description"
			onCancel={(event) => {
				event.preventDefault();
				if (!busy) onClose();
			}}
			className="panel-raised fixed inset-0 m-auto max-h-[calc(100vh-2rem)] w-[calc(100%-2rem)] max-w-md space-y-5 overflow-y-auto rounded-xl p-6 text-foreground shadow-xl backdrop:bg-black/65"
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
				Vos accès enregistrés seront conservés. Une prochaine réinstallation pourra les réutiliser.
			</p>

			<div className="flex justify-end gap-2">
				<Button ref={cancelRef} variant="outline" onClick={onClose} disabled={busy}>
					Annuler
				</Button>
				<Button
					variant="destructive"
					className="bg-none bg-[#b91c1c] text-white hover:bg-[#991b1b]"
					onClick={onConfirm}
					disabled={busy}
				>
					{busy && <Loader2 className="size-4 animate-spin" />}
					Confirmer la désinstallation
				</Button>
			</div>
		</dialog>
	);
}
