import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import test from "node:test";

function source(relativePath: string) {
	return readFileSync(new URL(`../${relativePath}`, import.meta.url), "utf8");
}

test("the desktop webview has a restrictive CSP", () => {
	const config = JSON.parse(source("src-tauri/tauri.conf.json"));
	const csp = config.app.security.csp;

	assert.equal(typeof csp, "string");
	assert.match(csp, /default-src 'self'/);
	assert.match(csp, /connect-src[^;]*ipc:/);
	assert.match(csp, /connect-src[^;]*http:\/\/ipc\.localhost/);
	assert.match(csp, /object-src 'none'/);
	assert.match(csp, /base-uri 'self'/);
});

test("updater artifacts use the native Tauri 2 format", () => {
	const config = JSON.parse(source("src-tauri/tauri.conf.json"));

	assert.equal(config.bundle.createUpdaterArtifacts, true);
});

test("security-sensitive Rust dependencies have patched minimums", () => {
	const cargo = source("src-tauri/Cargo.toml");

	assert.match(cargo, /tauri = \{ version = "2\.11\.[1-9]\d*"/);
	assert.match(cargo, /tauri-build = \{ version = "2\.6\.[1-9]\d*"/);
	assert.match(cargo, /tar = "0\.4\.46"/);
});

test("release installs exactly the reviewed JavaScript lockfile", () => {
	const workflow = source(".github/workflows/release.yml");

	assert.match(workflow, /pnpm install --frozen-lockfile/);
});

test("the production frontend is split into bounded chunks", () => {
	const viteConfig = source("vite.config.ts");

	assert.match(viteConfig, /manualChunks/);
	assert.match(viteConfig, /react-vendor/);
	assert.match(viteConfig, /markdown-vendor/);
});

test("local packaging disables updater signing without weakening releases", () => {
	const packageJson = JSON.parse(source("package.json"));
	const localConfigUrl = new URL("../src-tauri/tauri.local.conf.json", import.meta.url);

	assert.equal(
		packageJson.scripts["tauri:build:local"],
		"tauri build --config src-tauri/tauri.local.conf.json",
	);
	assert.equal(existsSync(localConfigUrl), true);

	const localConfig = JSON.parse(readFileSync(localConfigUrl, "utf8"));
	assert.equal(localConfig.bundle.createUpdaterArtifacts, false);
});
