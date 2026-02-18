export function SkillRegLogo({ size = 28, className }: { size?: number; className?: string }) {
	return (
		<svg
			xmlns="http://www.w3.org/2000/svg"
			viewBox="0 0 120 120"
			width={size}
			height={size}
			className={className}
			role="img"
			aria-label="SkillReg"
		>
			<rect x="10" y="10" width="100" height="100" rx="22" fill="#1E293B" />
			<rect x="32" y="35" width="40" height="6" rx="3" fill="#F8FAFC" opacity="0.9" />
			<rect x="32" y="49" width="52" height="6" rx="3" fill="#F8FAFC" opacity="0.65" />
			<rect x="32" y="63" width="46" height="6" rx="3" fill="#F8FAFC" opacity="0.4" />
			<rect x="32" y="77" width="36" height="6" rx="3" fill="#F8FAFC" opacity="0.2" />
			<circle cx="26" cy="38" r="3" fill="#6366F1" />
			<circle cx="26" cy="52" r="3" fill="#F8FAFC" opacity="0.65" />
			<circle cx="26" cy="66" r="3" fill="#F8FAFC" opacity="0.4" />
			<circle cx="26" cy="80" r="3" fill="#F8FAFC" opacity="0.2" />
		</svg>
	);
}
