import { SkillRegLogo } from "@/components/SkillRegLogo";
import { useAuthStore, useConfigStore } from "@/lib/store";
import { cn } from "@/lib/utils";
import { Home, Library, PackageCheck, Settings as SettingsIcon } from "lucide-react";
import { NavLink } from "react-router";

const NAV_ITEMS = [
	{ to: "/", label: "Accueil", icon: Home },
	{ to: "/catalog", label: "Catalogue", icon: Library },
	{ to: "/installed", label: "Mes skills", icon: PackageCheck },
	{ to: "/settings", label: "Réglages", icon: SettingsIcon },
];

export function Sidebar() {
	const user = useAuthStore((state) => state.user);
	const org = useConfigStore((state) => state.config.org);
	const activeOrganization = user?.orgs.find((organization) => organization.slug === org);

	return (
		<aside className="flex h-full w-56 shrink-0 flex-col brushed-metal channel-border-r text-sidebar-foreground">
			<div className="flex h-14 items-center gap-2 channel-border-b px-4">
				<SkillRegLogo size={28} />
				<span className="text-sm font-semibold [font-family:'Chakra_Petch',sans-serif] tracking-wide">
					SkillReg
				</span>
			</div>

			<nav aria-label="Navigation principale" className="flex flex-1 flex-col gap-1 p-2">
				<p className="px-3 py-2 text-[10px] font-semibold uppercase tracking-widest text-muted-foreground [font-family:'Chakra_Petch',sans-serif]">
					Navigation
				</p>
				{NAV_ITEMS.map((item) => (
					<NavLink
						key={item.to}
						to={item.to}
						end={item.to === "/"}
						className={({ isActive }) =>
							cn(
								"flex min-h-10 items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
								isActive
									? "panel-inset border-l-2 border-l-primary text-primary text-glow-amber"
									: "text-secondary-foreground hover:bg-secondary",
							)
						}
					>
						{({ isActive }) => (
							<>
								<span className={cn("led", isActive ? "led-amber" : "led-off")} />
								<item.icon className="size-4" aria-hidden="true" />
								{item.label}
							</>
						)}
					</NavLink>
				))}
			</nav>

			<div className="space-y-1 border-t border-sidebar-border p-4">
				{activeOrganization && (
					<p className="truncate text-xs font-medium text-secondary-foreground">
						{activeOrganization.name}
					</p>
				)}
				{user?.user.email && (
					<p className="truncate text-xs text-muted-foreground">{user.user.email}</p>
				)}
			</div>
		</aside>
	);
}
