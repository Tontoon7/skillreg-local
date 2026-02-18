import { Loader2 } from "lucide-react";
import { useEffect } from "react";
import { Navigate, Route, Routes } from "react-router";
import { AppShell } from "./components/layout/AppShell";
import { useAuthStore, useConfigStore } from "./lib/store";
import { Catalog } from "./pages/Catalog";
import { Dashboard } from "./pages/Dashboard";
import { EnvVars } from "./pages/EnvVars";
import { Installed } from "./pages/Installed";
import { Login } from "./pages/Login";
import { Settings } from "./pages/Settings";
import { Setup } from "./pages/Setup";
import { SkillDetailPage } from "./pages/SkillDetail";

function AuthGate({ children }: { children: React.ReactNode }) {
	const { authenticated, loading } = useAuthStore();
	const { config, loading: configLoading } = useConfigStore();

	if (loading || configLoading) {
		return (
			<div className="flex h-screen items-center justify-center bg-background">
				<Loader2 className="size-6 animate-spin text-muted-foreground" />
			</div>
		);
	}

	if (!authenticated) {
		return <Navigate to="/login" replace />;
	}

	if (!config.setupDone) {
		return <Navigate to="/setup" replace />;
	}

	return <AppShell>{children}</AppShell>;
}

export function App() {
	const checkAuth = useAuthStore((s) => s.checkAuth);
	const loadConfig = useConfigStore((s) => s.load);

	useEffect(() => {
		checkAuth();
		loadConfig();
	}, [checkAuth, loadConfig]);

	return (
		<Routes>
			<Route path="/login" element={<LoginRoute />} />
			<Route path="/setup" element={<SetupRoute />} />
			<Route
				path="*"
				element={
					<AuthGate>
						<Routes>
							<Route path="/" element={<Dashboard />} />
							<Route path="/catalog" element={<Catalog />} />
							<Route path="/catalog/:name" element={<SkillDetailPage />} />
							<Route path="/installed" element={<Installed />} />
							<Route path="/env" element={<EnvVars />} />
							<Route path="/settings" element={<Settings />} />
						</Routes>
					</AuthGate>
				}
			/>
		</Routes>
	);
}

function LoginRoute() {
	const authenticated = useAuthStore((s) => s.authenticated);
	const setupDone = useConfigStore((s) => s.config.setupDone);
	if (authenticated && setupDone) return <Navigate to="/" replace />;
	if (authenticated && !setupDone) return <Navigate to="/setup" replace />;
	return <Login />;
}

function SetupRoute() {
	const authenticated = useAuthStore((s) => s.authenticated);
	const setupDone = useConfigStore((s) => s.config.setupDone);
	if (!authenticated) return <Navigate to="/login" replace />;
	if (setupDone) return <Navigate to="/" replace />;
	return <Setup />;
}
