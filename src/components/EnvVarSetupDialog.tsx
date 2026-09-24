import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { listOrgEnvVars, setOrgEnvVar } from "@/lib/api";
import type { EnvVarDecl } from "@/lib/types";
import { KeyRound, Loader2, Save, X } from "lucide-react";
import { useEffect, useMemo, useState } from "react";

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
	const [configuredNames, setConfiguredNames] = useState<Set<string>>(new Set());
	const missingEnvVars = useMemo(
		() => envVars.filter((variable) => !configuredNames.has(normalizeName(variable.name))),
		[configuredNames, envVars],
	);

	useEffect(() => {
		const handleEscape = (event: KeyboardEvent) => {
			if (event.key === "Escape") onClose();
		};
		window.addEventListener("keydown", handleEscape);
		return () => window.removeEventListener("keydown", handleEscape);
	}, [onClose]);

	useEffect(() => {
		const initialValues = Object.fromEntries(
			envVars.map((variable) => [variable.name, variable.secret ? "" : (variable.default ?? "")]),
		);
		listOrgEnvVars(org)
			.then((existing) => {
				setConfiguredNames(new Set(existing.map((variable) => normalizeName(variable.name))));
			})
			.catch(() => {})
			.finally(() => {
				setValues(initialValues);
				setLoaded(true);
			});
	}, [envVars, org]);

	const handleSave = async () => {
		const missingRequired = missingEnvVars.filter(
			(variable) => variable.required && !values[variable.name]?.trim(),
		);
		if (missingRequired.length > 0) {
			setError(
				`Renseignez les accès obligatoires : ${missingRequired
					.map((variable) => variable.description || variable.name)
					.join(", ")}.`,
			);
			return;
		}

		const valuesToSave = missingEnvVars.flatMap((variable) => {
			const value = values[variable.name]?.trim();
			return value ? [[variable.name, value] as const] : [];
		});
		if (valuesToSave.length === 0) {
			onSaved();
			return;
		}

		setSaving(true);
		setError(null);
		try {
			await Promise.all(valuesToSave.map(([key, value]) => setOrgEnvVar(org, key, value)));
			onSaved();
		} catch (saveError) {
			setError(
				typeof saveError === "string" ? saveError : "Les accès n’ont pas pu être enregistrés.",
			);
		} finally {
			setSaving(false);
		}
	};

	return (
		<div className="fixed inset-0 z-50 flex items-center justify-center p-4">
			<button
				type="button"
				aria-label="Fermer"
				className="absolute inset-0 bg-black/65"
				onClick={onClose}
			/>
			<dialog
				open
				aria-labelledby="access-dialog-title"
				className="panel-raised relative z-10 m-0 w-full max-w-lg space-y-5 rounded-xl p-6 text-foreground shadow-xl"
			>
				<header className="flex items-start justify-between gap-4">
					<div className="flex gap-3">
						<div className="flex size-9 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
							<KeyRound className="size-4" />
						</div>
						<div className="space-y-1">
							<h2 id="access-dialog-title" className="text-base font-semibold">
								Configurer les accès
							</h2>
							<p className="text-sm text-muted-foreground">
								{skillName} a besoin de quelques informations pour fonctionner.
							</p>
						</div>
					</div>
					<Button
						variant="ghost"
						size="icon"
						aria-label="Fermer"
						onClick={onClose}
						className="size-8"
					>
						<X className="size-4" />
					</Button>
				</header>

				{!loaded ? (
					<div className="flex min-h-32 items-center justify-center">
						<Loader2 className="size-5 animate-spin text-muted-foreground" />
					</div>
				) : (
					<div className="max-h-80 space-y-4 overflow-y-auto">
						{missingEnvVars.length === 0 ? (
							<p className="rounded-lg border border-accent/20 bg-accent/5 p-3 text-sm text-accent">
								Tous les accès nécessaires sont déjà configurés.
							</p>
						) : (
							missingEnvVars.map((variable) => (
								<div key={variable.name} className="space-y-2">
									<div className="flex flex-wrap items-center gap-2">
										<Label htmlFor={`env-${variable.name}`}>
											{variable.description || variable.name}
										</Label>
										<Badge variant={variable.required ? "default" : "outline"}>
											{variable.required ? "Obligatoire" : "Facultatif"}
										</Badge>
									</div>
									{variable.description && (
										<p className="font-mono text-[11px] text-muted-foreground">{variable.name}</p>
									)}
									<Input
										id={`env-${variable.name}`}
										type={variable.secret ? "password" : "text"}
										autoComplete="off"
										placeholder={variable.default || "Saisissez la valeur"}
										value={values[variable.name] ?? ""}
										onChange={(event) =>
											setValues((current) => ({
												...current,
												[variable.name]: event.target.value,
											}))
										}
									/>
								</div>
							))
						)}
					</div>
				)}

				<p className="text-xs text-muted-foreground">
					Ces valeurs restent sur cet ordinateur et ne sont jamais ajoutées à la skill.
				</p>

				{error && (
					<p
						className="rounded-lg border border-destructive/30 bg-destructive/5 p-3 text-sm text-destructive"
						role="alert"
					>
						{error}
					</p>
				)}

				<footer className="flex justify-end gap-2">
					<Button variant="ghost" onClick={onClose} disabled={saving}>
						Plus tard
					</Button>
					<Button onClick={() => void handleSave()} disabled={saving || !loaded}>
						{saving ? <Loader2 className="size-4 animate-spin" /> : <Save className="size-4" />}
						Enregistrer
					</Button>
				</footer>
			</dialog>
		</div>
	);
}

function normalizeName(value: string): string {
	return value.trim().toUpperCase();
}
