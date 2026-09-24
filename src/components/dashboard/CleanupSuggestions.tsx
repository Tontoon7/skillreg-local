interface CleanupSuggestionsProps {
	enabled?: boolean;
}

export function CleanupSuggestions({ enabled = false }: CleanupSuggestionsProps) {
	if (!enabled) return null;

	return (
		<section className="panel-inset rounded-xl p-5" aria-labelledby="cleanup-title">
			<h2 id="cleanup-title" className="text-base font-semibold">
				Suggestions de nettoyage
			</h2>
			<p className="text-sm text-muted-foreground">
				Les recommandations apparaîtront lorsque l’observation d’usage sera disponible.
			</p>
		</section>
	);
}
