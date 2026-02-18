import { useAuthStore, useConfigStore } from "@/lib/store";
import { cn } from "@/lib/utils";
import { LayoutDashboard } from "lucide-react";

export function Dashboard() {
	const user = useAuthStore((s) => s.user);
	const config = useConfigStore((s) => s.config);
	const setOrg = useConfigStore((s) => s.setOrg);

	return (
		<div className="flex flex-col gap-6 p-6">
			<h1 className="text-lg font-semibold">Dashboard</h1>

			{user?.orgs && user.orgs.length > 0 ? (
				<div className="grid grid-cols-2 gap-4 lg:grid-cols-3">
					{user.orgs.map((org) => (
						<button
							type="button"
							key={org.slug}
							onClick={() => setOrg(org.slug)}
							className={cn(
								"flex flex-col gap-2 rounded-xl border p-5 text-left transition-colors",
								config.org === org.slug
									? "border-primary/50 bg-primary/5"
									: "hover:border-muted-foreground/30",
							)}
						>
							<p className="font-medium">{org.name}</p>
							<p className="text-xs text-muted-foreground capitalize">{org.role}</p>
							{config.org === org.slug && <span className="text-xs text-primary">Active</span>}
						</button>
					))}
				</div>
			) : (
				<div className="flex flex-col items-center justify-center gap-2 py-12">
					<LayoutDashboard className="size-10 text-muted-foreground" />
					<p className="text-muted-foreground">No organizations found</p>
				</div>
			)}
		</div>
	);
}
