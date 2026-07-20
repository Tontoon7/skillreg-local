import { SkillRegLogo } from "@/components/SkillRegLogo";
import { getCatalogPolicy } from "@/lib/api";
import { useAuthStore, useConfigStore } from "@/lib/store";
import { cn } from "@/lib/utils";
import { FolderOpen, Globe, Key, LayoutDashboard, Package, Settings, Terminal } from "lucide-react";
import { useEffect, useState } from "react";
import { NavLink } from "react-router";

const NAV_ITEMS = [
	{ to: "/", label: "Dashboard", icon: LayoutDashboard },
	{ to: "/catalog", label: "Catalog", icon: Package },
	{ to: "/commands", label: "Commands", icon: Terminal },
	{ to: "/installed", label: "Installed", icon: FolderOpen },
	{ to: "/env", label: "Env Vars", icon: Key },
	{ to: "/settings", label: "Settings", icon: Settings },
];

export function Sidebar() {
	const user = useAuthStore((s) => s.user);
	const org = useConfigStore((s) => s.config?.org);
	const [showPublicCatalog, setShowPublicCatalog] = useState(false);

	// The public catalog entry only appears when the organization allows
	// installing from it. Presentation only — the server re-evaluates the policy
	// on every install.
	useEffect(() => {
		if (!org) {
			setShowPublicCatalog(false);
			return;
		}
		let active = true;
		getCatalogPolicy(org)
			.then((policy) => active && setShowPublicCatalog(policy.canInstallFromCatalog))
			.catch(() => active && setShowPublicCatalog(false));
		return () => {
			active = false;
		};
	}, [org]);

	const navItems = showPublicCatalog
		? [
				...NAV_ITEMS.slice(0, 2),
				{ to: "/public-catalog", label: "Public catalog", icon: Globe },
				...NAV_ITEMS.slice(2),
			]
		: NAV_ITEMS;

	return (
		<aside className="flex h-full w-56 flex-col brushed-metal channel-border-r text-sidebar-foreground">
			<div className="flex h-14 items-center gap-2 channel-border-b px-4">
				<SkillRegLogo size={28} />
				<span className="text-sm font-semibold [font-family:'Chakra_Petch',sans-serif] tracking-wide">
					SkillReg
				</span>
			</div>

			<nav className="flex flex-1 flex-col gap-1 p-2">
				<p className="text-[10px] uppercase tracking-widest font-semibold text-muted-foreground [font-family:'Chakra_Petch',sans-serif] px-3 py-2">
					Navigation
				</p>
				{navItems.map((item) => (
					<NavLink
						key={item.to}
						to={item.to}
						className={({ isActive }) =>
							cn(
								"flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-all",
								isActive
									? "panel-inset text-primary text-glow-amber border-l-2 border-l-primary"
									: "text-secondary-foreground hover:bg-secondary",
							)
						}
					>
						{({ isActive }) => (
							<>
								<span className={cn("led", isActive ? "led-amber" : "led-off")} />
								<item.icon className="size-4" />
								{item.label}
							</>
						)}
					</NavLink>
				))}
			</nav>

			<div className="border-t border-sidebar-border p-4">
				{user?.user.email && (
					<p className="text-xs text-muted-foreground truncate">{user.user.email}</p>
				)}
			</div>
		</aside>
	);
}
