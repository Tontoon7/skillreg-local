import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { BrowserRouter } from "react-router";
import { App } from "./App";
import "./styles/globals.css";

// Debug: capture uncaught errors
window.addEventListener("error", (e) => {
	console.error("[UNCAUGHT]", e.message, e.filename, e.lineno, e.colno);
});
window.addEventListener("unhandledrejection", (e) => {
	console.error("[UNHANDLED REJECTION]", e.reason);
});

// Restore theme preference
const savedTheme = localStorage.getItem("skillreg-theme");
if (savedTheme === "light") {
	document.documentElement.classList.add("light");
}

// biome-ignore lint/style/noNonNullAssertion: root element always exists
createRoot(document.getElementById("root")!).render(
	<StrictMode>
		<BrowserRouter>
			<App />
		</BrowserRouter>
	</StrictMode>,
);
