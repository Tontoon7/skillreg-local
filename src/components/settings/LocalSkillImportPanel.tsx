import { Button } from "@/components/ui/button";
import { previewLocalSkillsImport, runLocalSkillsImport } from "@/lib/api";
import { useManagedSkillsStore } from "@/lib/store";
import type { LocalImportPreview } from "@/lib/types";
import { FolderSync, Loader2, RefreshCw, ShieldCheck } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";

function untouchedMessages(preview: LocalImportPreview): string[] {
	const messages: string[] = [];
	if (preview.conflicts === 1) messages.push("1 conflit restera inchangé.");
	if (preview.conflicts > 1) messages.push(`${preview.conflicts} conflits resteront inchangés.`);
	if (preview.externalLinksUntouched === 1) messages.push("1 lien externe restera inchangé.");
	if (preview.externalLinksUntouched > 1) {
		messages.push(`${preview.externalLinksUntouched} liens externes resteront inchangés.`);
	}
	if (preview.invalidUntouched === 1) {
		messages.push("1 skill non compatible restera inchangée.");
	}
	if (preview.invalidUntouched > 1) {
		messages.push(`${preview.invalidUntouched} skills non compatibles resteront inchangées.`);
	}
	return messages;
}

export function LocalSkillImportPanel() {
	const refreshManagedSkills = useManagedSkillsStore((state) => state.refresh);
	const [preview, setPreview] = useState<LocalImportPreview | null>(null);
	const [loading, setLoading] = useState(true);
	const [running, setRunning] = useState(false);
	const [error, setError] = useState<string | null>(null);
	const [result, setResult] = useState<string | null>(null);

	const loadPreview = useCallback(async () => {
		setLoading(true);
		setError(null);
		try {
			setPreview(await previewLocalSkillsImport());
		} catch {
			setPreview(null);
			setError("SkillReg ne peut pas encore examiner vos skills locales.");
		} finally {
			setLoading(false);
		}
	}, []);

	useEffect(() => {
		void loadPreview();
	}, [loadPreview]);

	const importableItems = useMemo(
		() =>
			preview?.items.filter(
				(item) =>
					item.classification === "importable" || item.classification === "duplicate_identical",
			) ?? [],
		[preview],
	);

	const handleImport = async () => {
		if (!preview || preview.importable === 0) return;
		setRunning(true);
		setError(null);
		setResult(null);
		try {
			const report = await runLocalSkillsImport(true);
			if (report.imported > 0) {
				setResult(
					report.imported === 1
						? "1 skill est maintenant gérée par SkillReg."
						: `${report.imported} skills sont maintenant gérées par SkillReg.`,
				);
			}
			if (report.errors > 0) {
				setError(
					report.imported > 0
						? report.errors === 1
							? "1 skill n’a pas pu être rassemblée. Son dossier original est resté intact."
							: `${report.errors} skills n’ont pas pu être rassemblées. Leurs dossiers originaux sont restés intacts.`
						: "La bascule n’a pas pu aboutir. Vos skills originales sont restées intactes.",
				);
			}
			await refreshManagedSkills().catch(() => undefined);
			setPreview(await previewLocalSkillsImport());
		} catch {
			setError("La bascule n’a pas pu aboutir. Vos skills originales sont restées intactes.");
		} finally {
			setRunning(false);
		}
	};

	const untouched = preview ? untouchedMessages(preview) : [];

	return (
		<section
			className="panel-inset min-w-0 space-y-4 rounded-xl p-5"
			aria-labelledby="local-import-title"
		>
			<div className="flex min-w-0 items-start gap-3">
				<div className="flex size-9 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
					<FolderSync className="size-4" aria-hidden="true" />
				</div>
				<div className="min-w-0 space-y-1">
					<h2 id="local-import-title" className="text-sm font-semibold">
						Rassembler mes skills locales
					</h2>
					<p className="text-xs text-muted-foreground">
						SkillReg peut réunir les copies présentes sur cet ordinateur et les afficher au même
						endroit.
					</p>
				</div>
			</div>

			{loading && (
				<div
					className="flex min-h-16 items-center gap-2 text-sm text-muted-foreground"
					aria-live="polite"
				>
					<Loader2 className="size-4 animate-spin" />
					Recherche des skills locales…
				</div>
			)}

			{!loading && error && !preview && (
				<div className="space-y-3">
					<p className="text-sm text-destructive" role="alert">
						{error}
					</p>
					<Button variant="outline" size="sm" onClick={() => void loadPreview()}>
						<RefreshCw className="size-3.5" />
						Réessayer
					</Button>
				</div>
			)}

			{!loading && preview && preview.importable === 0 && (
				<div className="space-y-3">
					<div className="flex min-h-16 items-start gap-3 rounded-lg bg-background/30 p-3">
						<ShieldCheck className="mt-0.5 size-4 shrink-0 text-accent" />
						<div className="space-y-1">
							<p className="text-sm font-medium">Aucune skill locale à importer</p>
							<p className="text-xs text-muted-foreground">
								{preview.alreadyManaged > 0
									? "Les skills reconnues sont déjà gérées par SkillReg."
									: "SkillReg n’a trouvé aucun dossier local compatible."}
							</p>
						</div>
					</div>
					{untouched.length > 0 && (
						<div className="space-y-1 text-xs text-muted-foreground">
							{untouched.map((message) => (
								<p key={message}>{message}</p>
							))}
						</div>
					)}
				</div>
			)}

			{!loading && preview && preview.importable > 0 && (
				<div className="space-y-4">
					<div className="space-y-1">
						<p className="text-sm font-medium">
							{preview.importable === 1
								? "1 skill peut être rassemblée"
								: `${preview.importable} skills peuvent être rassemblées`}
						</p>
						<p className="text-xs text-muted-foreground">
							Une seule copie sera conservée par skill, puis rendue disponible dans vos assistants
							compatibles.
						</p>
					</div>

					<ul className="grid gap-2 sm:grid-cols-2">
						{importableItems.slice(0, 6).map((item) => (
							<li
								key={item.skillName}
								className="truncate rounded-md border border-border/70 bg-background/30 px-3 py-2 text-xs"
							>
								{item.skillName}
							</li>
						))}
					</ul>
					{importableItems.length > 6 && (
						<p className="text-xs text-muted-foreground">
							Et {importableItems.length - 6} autre
							{importableItems.length - 6 > 1 ? "s" : ""}.
						</p>
					)}

					<div className="space-y-1 text-xs text-muted-foreground">
						{untouched.map((message) => (
							<p key={message}>{message}</p>
						))}
						<p>Aucun contenu n’est envoyé : rien n’est publié.</p>
					</div>

					<Button
						onClick={() => void handleImport()}
						disabled={running}
						className="w-full min-[1100px]:w-auto"
					>
						{running && <Loader2 className="size-4 animate-spin" />}
						Rassembler {preview.importable} skill{preview.importable > 1 ? "s" : ""}
					</Button>
				</div>
			)}

			{result && <output className="block text-sm text-accent">{result}</output>}
			{error && preview && (
				<p className="text-sm text-destructive" role="alert">
					{error}
				</p>
			)}
		</section>
	);
}
