// Deterministic hue per tag name
export function getTagHue(tag: string): number {
	let hash = 0;
	for (let i = 0; i < tag.length; i++) {
		hash = tag.charCodeAt(i) + ((hash << 5) - hash);
	}
	return Math.abs(hash) % 360;
}

export function tagStyle(tag: string, active: boolean): React.CSSProperties {
	const hue = getTagHue(tag);
	return {
		backgroundColor: `hsla(${hue}, 65%, 50%, ${active ? 0.22 : 0.1})`,
		color: `hsl(${hue}, 75%, 55%)`,
		borderColor: `hsla(${hue}, 65%, 50%, ${active ? 0.5 : 0.2})`,
	};
}
