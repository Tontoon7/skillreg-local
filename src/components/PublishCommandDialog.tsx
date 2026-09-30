import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select } from "@/components/ui/select";
import { getCommand, publishCommandVersion } from "@/lib/api";
import {
	type CommandPublicationDraft,
	prepareCommandPublication,
	validateCommandPublication,
} from "@/lib/command-publishing";
import { AGENTS } from "@/lib/constants";
import type { CommandVersion } from "@/lib/types";
import { Loader2, Upload, X } from "lucide-react";
import { type FormEvent, useEffect, useId, useRef, useState } from "react";

type Props = {
	org: string;
	name: string;
	onClose: () => void;
	onPublished: (version: CommandVersion) => void;
};

export function PublishCommandDialog({ org, name, onClose, onPublished }: Props) {
	const [target, setTarget] = useState({ org, name });
	const [prepared, setPrepared] = useState<ReturnType<typeof prepareCommandPublication> | null>(
		null,
	);
	const [loading, setLoading] = useState(true);
	const [loadError, setLoadError] = useState<string | null>(null);
	const [publishError, setPublishError] = useState<string | null>(null);
	const [publishing, setPublishing] = useState(false);
	const [showErrors, setShowErrors] = useState(false);
	const dialogRef = useRef<HTMLDialogElement>(null);
	const versionRef = useRef<HTMLInputElement>(null);
	const mounted = useRef(false);
	const submitting = useRef(false);
	const id = useId();

	useEffect(() => {
		const previousFocus = document.activeElement;
		const dialog = dialogRef.current;
		mounted.current = true;
		dialog?.showModal();
		return () => {
			mounted.current = false;
			dialog?.close();
			if (previousFocus instanceof HTMLElement && previousFocus.isConnected) {
				previousFocus.focus();
			}
		};
	}, []);

	useEffect(() => {
		let active = true;
		setLoading(true);
		setLoadError(null);
		getCommand(target.org, target.name)
			.then((command) => {
				if (active) setPrepared(prepareCommandPublication(command));
			})
			.catch((error: unknown) => {
				if (active) setLoadError(errorMessage(error, "Could not load the command."));
			})
			.finally(() => {
				if (active) setLoading(false);
			});
		return () => {
			active = false;
		};
	}, [target]);

	useEffect(() => {
		if (!loading && !loadError) versionRef.current?.focus();
	}, [loading, loadError]);

	useEffect(() => {
		if (publishError && !publishing) versionRef.current?.focus();
	}, [publishError, publishing]);

	const validation = prepared
		? validateCommandPublication(prepared.draft, prepared.existingVersions)
		: null;
	const errors = showErrors ? validation?.errors : undefined;
	const updateDraft = (patch: Partial<CommandPublicationDraft>) => {
		setPrepared((current) =>
			current ? { ...current, draft: { ...current.draft, ...patch } } : current,
		);
	};
	const close = () => {
		if (!submitting.current) onClose();
	};
	const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
		event.preventDefault();
		if (submitting.current || loading || !validation) return;
		setShowErrors(true);
		if (!validation.input) {
			const field = Object.keys(validation.errors)[0];
			event.currentTarget.querySelector<HTMLElement>(`[name="${field}"]`)?.focus();
			return;
		}

		submitting.current = true;
		setPublishing(true);
		setPublishError(null);
		let result: CommandVersion;
		try {
			result = await publishCommandVersion(target.org, target.name, validation.input);
		} catch (error) {
			if (mounted.current) {
				setPublishError(errorMessage(error, "Command version publish failed."));
				setPublishing(false);
				submitting.current = false;
			}
			return;
		}
		if (mounted.current) {
			onPublished(result);
			onClose();
		}
	};

	return (
		<dialog
			ref={dialogRef}
			aria-labelledby={`${id}-title`}
			aria-describedby={`${id}-description`}
			onCancel={(event) => {
				event.preventDefault();
				close();
			}}
			className="fixed inset-0 m-auto max-h-[90vh] w-[calc(100%-2rem)] max-w-2xl overflow-y-auto rounded-xl border bg-card p-6 text-foreground shadow-lg backdrop:bg-black/50"
		>
			<div className="space-y-5">
				<div className="flex items-start justify-between gap-4">
					<div className="space-y-1">
						<h2 id={`${id}-title`} className="text-base font-semibold">
							Publish command version
						</h2>
						<p className="text-sm font-mono break-all">
							@{target.org}/{target.name}
						</p>
					</div>
					<Button
						type="button"
						variant="ghost"
						size="icon"
						className="size-7"
						aria-label="Close publication dialog"
						disabled={publishing}
						onClick={close}
					>
						<X className="size-4" />
					</Button>
				</div>
				<p id={`${id}-description`} className="text-sm text-muted-foreground">
					Publishing makes this the current registry version. Local installations stay unchanged.
				</p>
				{loading && (
					<output className="flex items-center gap-2 text-sm text-muted-foreground">
						<Loader2 className="size-4 animate-spin" /> Loading command…
					</output>
				)}
				{loadError && (
					<div className="space-y-3">
						<p role="alert" className="text-sm text-destructive">
							{loadError}
						</p>
						<Button
							type="button"
							variant="outline"
							onClick={() => setTarget((current) => ({ ...current }))}
						>
							Retry loading
						</Button>
					</div>
				)}
				{!loading && !loadError && prepared && (
					<form onSubmit={handleSubmit} noValidate className="space-y-5">
						<fieldset disabled={publishing} className="space-y-5">
							<div className="space-y-2">
								<Label htmlFor={`${id}-version`}>New version</Label>
								<Input
									ref={versionRef}
									id={`${id}-version`}
									name="version"
									required
									placeholder="1.0.1"
									value={prepared.draft.version}
									onChange={(event) => updateDraft({ version: event.target.value })}
									aria-invalid={Boolean(errors?.version)}
									aria-describedby={`${id}-current-version ${id}-version-error`}
								/>
								<p id={`${id}-current-version`} className="text-xs text-muted-foreground">
									Current version: {prepared.currentVersion ?? "No version published yet"}
								</p>
								<p id={`${id}-version-error`} className="text-xs text-destructive">
									{errors?.version}
								</p>
							</div>
							<div className="space-y-2">
								<Label htmlFor={`${id}-content`}>Command content</Label>
								<textarea
									id={`${id}-content`}
									name="content"
									required
									rows={9}
									value={prepared.draft.content}
									onChange={(event) => updateDraft({ content: event.target.value })}
									aria-invalid={Boolean(errors?.content)}
									aria-describedby={`${id}-content-hint ${id}-content-error`}
									className="w-full rounded-md border bg-background px-3 py-2 text-sm font-mono outline-none transition-colors focus:border-primary disabled:opacity-50"
								/>
								<p id={`${id}-content-hint`} className="text-xs text-muted-foreground">
									Raw command text. Leading and trailing whitespace is trimmed by the registry.
								</p>
								<p id={`${id}-content-error`} className="text-xs text-destructive">
									{errors?.content}
								</p>
							</div>
							<fieldset aria-describedby={`${id}-agents-error`} className="space-y-2">
								<legend className="text-sm font-medium">Compatible agents</legend>
								<div className="flex flex-wrap gap-4">
									{[...new Set([...AGENTS, ...prepared.draft.agentCompatibility])].map((agent) => (
										<Label key={agent} htmlFor={`${id}-agent-${agent}`}>
											<input
												id={`${id}-agent-${agent}`}
												name="agentCompatibility"
												type="checkbox"
												checked={prepared.draft.agentCompatibility.includes(agent)}
												onChange={(event) =>
													updateDraft({
														agentCompatibility: event.target.checked
															? [...prepared.draft.agentCompatibility, agent]
															: prepared.draft.agentCompatibility.filter(
																	(value) => value !== agent,
																),
													})
												}
												className="size-4 accent-primary"
											/>
											{agent}
										</Label>
									))}
								</div>
								<p id={`${id}-agents-error`} className="text-xs text-destructive">
									{errors?.agentCompatibility}
								</p>
							</fieldset>
							<div className="space-y-2">
								<Label htmlFor={`${id}-scope`}>Command scope</Label>
								<Select
									id={`${id}-scope`}
									name="scope"
									value={prepared.draft.scope}
									onChange={(event) => updateDraft({ scope: event.target.value })}
									aria-invalid={Boolean(errors?.scope)}
									aria-describedby={`${id}-scope-error`}
								>
									{!["org", "project", "user"].includes(prepared.draft.scope) && (
										<option value={prepared.draft.scope} disabled>
											Choose a command scope
										</option>
									)}
									<option value="org">Organization</option>
									<option value="project">Project</option>
									<option value="user">User</option>
								</Select>
								<p id={`${id}-scope-error`} className="text-xs text-destructive">
									{errors?.scope}
								</p>
							</div>
						</fieldset>
						{showErrors && validation && !validation.input && (
							<p role="alert" className="text-sm text-destructive">
								Correct the fields above before publishing.
							</p>
						)}
						{publishError && (
							<p
								role="alert"
								className="rounded-lg border border-destructive/30 bg-destructive/5 p-3 text-sm text-destructive"
							>
								{publishError}
							</p>
						)}
						<output className="block text-sm text-muted-foreground">
							{publishing ? "Publishing command version…" : ""}
						</output>
						<div className="flex gap-3">
							<Button type="button" variant="outline" onClick={close} disabled={publishing}>
								Cancel
							</Button>
							<Button type="submit" disabled={publishing}>
								{publishing ? (
									<Loader2 className="size-4 animate-spin" />
								) : (
									<Upload className="size-4" />
								)}
								Publish version
							</Button>
						</div>
					</form>
				)}
			</div>
		</dialog>
	);
}

function errorMessage(error: unknown, fallback: string): string {
	return typeof error === "string" ? error : error instanceof Error ? error.message : fallback;
}
