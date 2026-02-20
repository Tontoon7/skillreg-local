import type { ReactNode } from "react";
import { UpdateChecker } from "../UpdateChecker";
import { Sidebar } from "./Sidebar";
import { Titlebar } from "./Titlebar";

export function AppShell({ children }: { children: ReactNode }) {
	return (
		<div className="flex h-screen flex-col overflow-hidden">
			<Titlebar />
			<UpdateChecker />
			<div className="flex flex-1 overflow-hidden">
				<Sidebar />
				<main className="flex-1 overflow-auto">{children}</main>
			</div>
		</div>
	);
}
