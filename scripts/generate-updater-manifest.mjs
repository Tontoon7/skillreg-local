import {
	copyFileSync,
	lstatSync,
	mkdirSync,
	readFileSync,
	readdirSync,
	writeFileSync,
} from "node:fs";
import { join, resolve } from "node:path";
import { parseArgs } from "node:util";

const platforms = [
	{ key: "darwin-aarch64", suffix: ".app.tar.gz", name: "SkillReg_aarch64.app.tar.gz" },
	{ key: "darwin-x86_64", suffix: ".app.tar.gz", name: "SkillReg_x64.app.tar.gz" },
	{ key: "linux-x86_64", suffix: ".AppImage" },
	{ key: "windows-x86_64", suffix: ".exe" },
];

function regularNonemptyFile(path) {
	const stat = lstatSync(path);
	if (!stat.isFile() || stat.size === 0) {
		throw new Error(`Expected a nonempty regular artifact: ${path}`);
	}
}

function prepare(values) {
	for (const option of ["artifacts", "output", "version", "base-url", "pub-date"]) {
		if (!values[option]) throw new Error(`Missing --${option}`);
	}
	if (!/^\d+\.\d+\.\d+(?:-[\w.-]+)?(?:\+[\w.-]+)?$/.test(values.version)) {
		throw new Error("Invalid release version");
	}
	const baseUrl = new URL(values["base-url"]);
	if (
		baseUrl.protocol !== "https:" ||
		baseUrl.search ||
		baseUrl.hash ||
		baseUrl.username ||
		baseUrl.password
	) {
		throw new Error(
			"Expected an HTTPS release download URL without credentials, query or fragment",
		);
	}
	if (!Number.isFinite(Date.parse(values["pub-date"]))) throw new Error("Invalid publication date");
	const artifacts = resolve(values.artifacts);
	const output = resolve(values.output);
	const manifest = {
		version: values.version,
		notes: `SkillReg v${values.version}`,
		pub_date: values["pub-date"],
		platforms: {},
	};
	const uploads = [];
	for (const platform of platforms) {
		const folder = join(artifacts, `build-${platform.key}`);
		if (!lstatSync(folder).isDirectory())
			throw new Error(`Invalid platform folder: ${platform.key}`);
		const candidates = readdirSync(folder).filter((file) => file.endsWith(platform.suffix));
		if (candidates.length !== 1) {
			throw new Error(`${platform.key}: expected exactly one ${platform.suffix} artifact`);
		}
		const source = join(folder, candidates[0]);
		regularNonemptyFile(source);
		regularNonemptyFile(`${source}.sig`);
		const signature = readFileSync(`${source}.sig`, "utf8").trim();
		if (!signature) throw new Error(`${platform.key}: empty signature`);
		const name = platform.name ?? candidates[0];
		manifest.platforms[platform.key] = {
			signature,
			url: `${baseUrl.href.replace(/\/$/, "")}/${encodeURIComponent(name)}`,
		};
		uploads.push({ source, name });
	}
	// Validate every platform before producing anything the workflow could publish.
	mkdirSync(output);
	for (const { source, name } of uploads) {
		copyFileSync(source, join(output, name));
		copyFileSync(`${source}.sig`, join(output, `${name}.sig`));
	}
	writeFileSync(join(output, "latest.json"), `${JSON.stringify(manifest, null, 2)}\n`, {
		flag: "wx",
	});
}

try {
	const { values } = parseArgs({
		options: Object.fromEntries(
			["artifacts", "output", "version", "base-url", "pub-date"].map((name) => [
				name,
				{ type: "string" },
			]),
		),
	});
	prepare(values);
} catch (error) {
	console.error(error instanceof Error ? error.message : "Unable to prepare updater artifacts");
	process.exitCode = 1;
}
