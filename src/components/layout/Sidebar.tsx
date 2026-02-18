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
		<aside className="flex h-full w-56 flex-col border-r border-sidebar-border bg-sidebar text-sidebar-foreground">
			<div className="flex h-14 items-center gap-2 border-b border-sidebar-border px-4">
				<SkillRegLogo size={28} />
				<span className="text-sm font-semibold">SkillReg</span>
			</div>

			<nav className="flex flex-1 flex-col gap-1 p-2">
				{NAV_ITEMS.map((item) => (
					<NavLink
						key={item.to}
						to={item.to}
						className={({ isActive }) =>
							cn(
								"flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors",
								isActive
									? "bg-primary/10 text-primary"
									: "text-muted-foreground hover:bg-sidebar-accent/50 hover:text-sidebar-foreground",
							)
						}
					>
						<item.icon className="size-4" />
						{item.label}
					</NavLink>
				))}
			</nav>

			<div className="border-t border-sidebar-border p-4">
				{user?.user.email && (
					<p className="text-xs text-muted-foreground truncate mb-1">{user.user.email}</p>
				)}
				<p className="text-xs text-muted-foreground/60">SkillReg Local v0.1.0</p>
			</div>
		</aside>
	);
}
