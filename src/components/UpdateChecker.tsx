import { relaunch } from "@tauri-apps/plugin-process";
import { check } from "@tauri-apps/plugin-updater";
import { Download, RefreshCw, X } from "lucide-react";
import { useEffect, useState } from "react";

type UpdateState =
	| { status: "idle" }
	| { status: "available"; version: string }
	| { status: "downloading"; progress: number }
	| { status: "ready" }
	| { status: "error"; message: string };

export function UpdateChecker() {
	const [state, setState] = useState<UpdateState>({ status: "idle" });
	const [dismissed, setDismissed] = useState(false);
	const [updateRef, setUpdateRef] = useState<Awaited<ReturnType<typeof check>> | null>(null);

	useEffect(() => {
		const checkForUpdate = async () => {
			try {
				const update = await check();
				if (update) {
					setState({ status: "available", version: update.version });
					setUpdateRef(update);
				}
			} catch {
				// Silently ignore update check failures
			}
		};

		checkForUpdate();
	}, []);

	const handleUpdate = async () => {
		if (!updateRef) return;

		try {
			let totalLength = 0;
			let downloaded = 0;

			setState({ status: "downloading", progress: 0 });

			await updateRef.downloadAndInstall((event) => {
				if (event.event === "Started") {
					totalLength = event.data.contentLength ?? 0;
				} else if (event.event === "Progress") {
					downloaded += event.data.chunkLength;
					const progress = totalLength > 0 ? Math.round((downloaded / totalLength) * 100) : 0;
					setState({ status: "downloading", progress });
				} else if (event.event === "Finished") {
					setState({ status: "ready" });
				}
			});

			setState({ status: "ready" });
		} catch (e) {
			setState({ status: "error", message: e instanceof Error ? e.message : "Update failed" });
		}
	};

	const handleRelaunch = async () => {
		await relaunch();
	};

	if (state.status === "idle" || dismissed) return null;

	return (
		<div className="flex items-center gap-3 border-b border-border bg-primary/10 px-4 py-2 text-sm">
			{state.status === "available" && (
				<>
					<Download className="size-4 shrink-0 text-primary" />
					<span className="flex-1">
						Version <strong>{state.version}</strong> disponible
					</span>
					<button
						type="button"
						onClick={handleUpdate}
						className="rounded-md bg-primary px-3 py-1 text-xs font-medium text-primary-foreground hover:bg-primary/90"
					>
						Mettre à jour
					</button>
					<button
						type="button"
						onClick={() => setDismissed(true)}
						className="text-muted-foreground hover:text-foreground"
					>
						<X className="size-4" />
					</button>
				</>
			)}

			{state.status === "downloading" && (
				<>
					<RefreshCw className="size-4 shrink-0 animate-spin text-primary" />
					<span className="flex-1">Téléchargement en cours… {state.progress}%</span>
					<div className="h-1.5 w-32 overflow-hidden rounded-full bg-muted">
						<div
							className="h-full rounded-full bg-primary transition-all"
							style={{ width: `${state.progress}%` }}
						/>
					</div>
				</>
			)}

			{state.status === "ready" && (
				<>
					<Download className="size-4 shrink-0 text-green-500" />
					<span className="flex-1">Mise à jour prête — redémarrage nécessaire</span>
					<button
						type="button"
						onClick={handleRelaunch}
						className="rounded-md bg-green-600 px-3 py-1 text-xs font-medium text-white hover:bg-green-700"
					>
						Redémarrer
					</button>
				</>
			)}

			{state.status === "error" && (
				<>
					<X className="size-4 shrink-0 text-destructive" />
					<span className="flex-1 text-destructive">Erreur : {state.message}</span>
					<button
						type="button"
						onClick={() => setDismissed(true)}
						className="text-muted-foreground hover:text-foreground"
					>
						<X className="size-4" />
					</button>
				</>
			)}
		</div>
	);
}
