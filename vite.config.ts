import { resolve } from "node:path";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
	plugins: [react(), tailwindcss()],
	resolve: {
		alias: {
			"@": resolve(__dirname, "src"),
		},
	},
	build: {
		rollupOptions: {
			output: {
				manualChunks: {
					"react-vendor": ["react", "react-dom", "react-router"],
					"markdown-vendor": ["react-markdown", "rehype-sanitize"],
					"tauri-vendor": [
						"@tauri-apps/api",
						"@tauri-apps/plugin-dialog",
						"@tauri-apps/plugin-notification",
						"@tauri-apps/plugin-process",
						"@tauri-apps/plugin-updater",
					],
				},
			},
		},
	},
	clearScreen: false,
	server: {
		port: 1420,
		strictPort: true,
		host: host || false,
		hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
		watch: { ignored: ["**/src-tauri/**"] },
	},
});
