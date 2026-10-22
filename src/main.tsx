import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { BrowserRouter } from "react-router";
import { App } from "./App";
import "./styles/globals.css";

// Native errors may contain credentials, signed URLs, or private filesystem paths.
window.addEventListener("error", (event) => {
	event.preventDefault();
	console.error("[UNCAUGHT]");
});
window.addEventListener("unhandledrejection", (event) => {
	event.preventDefault();
	console.error("[UNHANDLED REJECTION]");
});

// biome-ignore lint/style/noNonNullAssertion: root element always exists
createRoot(document.getElementById("root")!).render(
	<StrictMode>
		<BrowserRouter>
			<App />
		</BrowserRouter>
	</StrictMode>,
);
