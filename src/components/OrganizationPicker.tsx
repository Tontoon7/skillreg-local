import { Button } from "@/components/ui/button";
import { Check, Loader2 } from "lucide-react";

interface OrganizationOption {
	slug: string;
	name: string;
	role: string;
}

interface OrganizationPickerProps {
	organizations: OrganizationOption[];
	selected: string;
	onSelect: (slug: string) => void;
	onContinue: () => void;
	loading?: boolean;
}

export function OrganizationPicker({
	organizations,
	selected,
	onSelect,
	onContinue,
	loading = false,
}: OrganizationPickerProps) {
	return (
		<section className="panel-inset rounded-xl p-6 space-y-5" aria-labelledby="company-title">
			<div className="space-y-1 text-center">
				<h1 id="company-title" className="text-lg font-semibold">
					Choisissez votre entreprise
				</h1>
				<p className="text-sm text-muted-foreground">
					Vos skills et règles d’accès dépendent de cet espace.
				</p>
			</div>

			<div className="space-y-2">
				{organizations.map((organization) => {
					const active = selected === organization.slug;
					return (
						<button
							type="button"
							key={organization.slug}
							aria-pressed={active}
							onClick={() => onSelect(organization.slug)}
							className={`flex min-h-12 w-full items-center justify-between rounded-lg border px-4 py-3 text-left transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring ${
								active
									? "border-primary bg-primary/5 text-foreground"
									: "border-border bg-card hover:border-muted-foreground/40"
							}`}
						>
							<span className="font-medium">{organization.name}</span>
							{active && <Check className="size-4 text-primary" aria-hidden="true" />}
						</button>
					);
				})}
			</div>

			<Button className="w-full" onClick={onContinue} disabled={!selected || loading}>
				{loading && <Loader2 className="size-4 animate-spin" />}
				Continuer
			</Button>
		</section>
	);
}
