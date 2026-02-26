import { SkillRegLogo } from "@/components/SkillRegLogo";
import { useAuthStore } from "@/lib/store";
import { cn } from "@/lib/utils";
import { FolderOpen, Key, LayoutDashboard, Package, Settings } from "lucide-react";
import { NavLink } from "react-router";

const NAV_ITEMS = [
	{ to: "/", label: "Dashboard", icon: LayoutDashboard },
	{ to: "/catalog", label: "Catalog", icon: Package },
	{ to: "/installed", label: "Installed", icon: FolderOpen },
	{ to: "/env", label: "Env Vars", icon: Key },
	{ to: "/settings", label: "Settings", icon: Settings },
];

export function Sidebar() {
	const user = useAuthStore((s) => s.user);

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
				{NAV_ITEMS.map((item) => (
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
