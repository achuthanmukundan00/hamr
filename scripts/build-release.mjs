#!/usr/bin/env node
// build-release.mjs — produce a single publishable @skaft/hamr tarball.
//
// The repo is a 4-package workspace (@hamr/tui, @hamr/ai, @hamr/agent,
// @hamr/coding-agent), but we ship ONE package: @skaft/hamr. To avoid bundling
// JS (which would break runtime path resolution for the image worker, wasm,
// and jiti extension loading), we instead vendor the three @hamr/* libraries
// into the published package's node_modules and mark them as bundledDependencies.
// The consumer's install is therefore byte-identical to the dev runtime.
//
// Steps: build all packages -> stage @skaft/hamr (rewritten package.json with
// the union of real deps + bundled @hamr/* libs) -> `npm pack` -> releases/.
//
// Usage: node scripts/build-release.mjs [--no-build]

import { execSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync, readdirSync, renameSync, statSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const LIB_PACKAGES = ["tui", "ai", "agent"]; // bundled @hamr/* libraries
const APP_PKG_DIR = join(ROOT, "packages", "coding-agent");
const STAGING = join(ROOT, "release", "staging");
const RELEASES = join(ROOT, "releases");
const PUBLISHED_NAME = "@skaft/hamr";

const run = (cmd, cwd = ROOT) => execSync(cmd, { cwd, stdio: "inherit" });
const readJson = (p) => JSON.parse(readFileSync(p, "utf8"));

// Locate the protobufjs package directory in the workspace. protobufjs is a
// transitive dep (@google/genai → protobufjs) and may live in bun's virtual
// store or a flat npm node_modules, depending on the package manager.
function findProtobufjs() {
	// Node module resolution from packages/ai (where @google/genai declares it)
	const aiRequire = createRequire(join(ROOT, "packages", "ai", "package.json"));
	try {
		return dirname(aiRequire.resolve("protobufjs/package.json"));
	} catch {}
	// bun virtual store: node_modules/.bun/protobufjs@<ver>/node_modules/protobufjs
	const bunStore = join(ROOT, "node_modules", ".bun");
	if (existsSync(bunStore)) {
		for (const entry of readdirSync(bunStore)) {
			if (entry.startsWith("protobufjs@")) {
				const p = join(bunStore, entry, "node_modules", "protobufjs");
				if (existsSync(p)) return p;
			}
		}
	}
	throw new Error("Cannot find protobufjs in workspace. Run `bun install` or `npm install` first.");
}

// Recursively delete every `node_modules` directory under `dir`. Used to strip
// nested example deps from staging before pack. The top-level bundled @hamr/*
// node_modules is vendored in *after* this runs, so it is never affected.
function pruneNestedNodeModules(dir) {
	for (const name of readdirSync(dir)) {
		const full = join(dir, name);
		if (!statSync(full).isDirectory()) continue;
		if (name === "node_modules") {
			rmSync(full, { recursive: true, force: true });
			continue;
		}
		pruneNestedNodeModules(full);
	}
}

function main() {
	const skipBuild = process.argv.includes("--no-build");

	if (!skipBuild) {
		console.log("→ Building all packages…");
		run("npm run build");
	}

	console.log("→ Staging", PUBLISHED_NAME, "…");
	rmSync(STAGING, { recursive: true, force: true });
	mkdirSync(STAGING, { recursive: true });

	const appPkg = readJson(join(APP_PKG_DIR, "package.json"));

	// Copy the app package's shipped files into the staging root. We bundle the
	// @hamr/* libs ourselves, so any inherited npm-shrinkwrap.json (a leftover of
	// the old pi publish flow) must NOT ship — it would force the consumer's tree
	// and conflict with bundledDependencies.
	for (const entry of appPkg.files ?? []) {
		if (entry === "npm-shrinkwrap.json") continue;
		const src = join(APP_PKG_DIR, entry);
		if (!existsSync(src)) continue;
		cpSync(src, join(STAGING, entry), { recursive: true });
	}
	// README is referenced by the npm page even though it's not in `files`.
	for (const extra of ["README.md", "CHANGELOG.md", "LICENSE"]) {
		const src = join(APP_PKG_DIR, extra);
		if (existsSync(src)) cpSync(src, join(STAGING, extra));
	}

	// Prune any nested node_modules dragged in by recursive copies of shipped
	// dirs (e.g. examples/extensions/*/node_modules). These are an example's own
	// dev/build deps — they bloat the tarball and embed host-arch native binaries
	// (esbuild/rollup/fsevents) that are wrong for other platforms. The only
	// node_modules we intentionally ship is the top-level bundled @hamr/* libs,
	// added later.
	pruneNestedNodeModules(STAGING);

	// Union of real (non-@hamr) runtime deps across all four packages, so the
	// vendored libraries can resolve their own dependencies from the hoisted
	// top-level node_modules of the installed package.
	const deps = {};
	const collect = (pkgJsonPath) => {
		const json = readJson(pkgJsonPath);
		for (const [name, ver] of Object.entries(json.dependencies ?? {})) {
			if (name.startsWith("@hamr/")) continue;
			deps[name] = ver; // later packages win; workspace versions are consistent
		}
	};
	for (const lib of LIB_PACKAGES) collect(join(ROOT, "packages", lib, "package.json"));
	collect(join(APP_PKG_DIR, "package.json"));

	// Pin the bundled libraries to their concrete versions.
	const bundled = {};
	for (const lib of LIB_PACKAGES) {
		const libPkg = readJson(join(ROOT, "packages", lib, "package.json"));
		bundled[`@hamr/${lib}`] = libPkg.version;
	}

	const stagedPkg = {
		name: PUBLISHED_NAME,
		version: appPkg.version,
		description: appPkg.description,
		type: "module",
		hamrConfig: appPkg.hamrConfig,
		bin: appPkg.bin,
		main: appPkg.main,
		types: appPkg.types,
		exports: appPkg.exports,
		// `files` is intentionally omitted: in staging the tarball ships exactly
		// what we copied here (everything except node_modules/ is included by
		// default; node_modules/ is included via bundledDependencies below).
		dependencies: { ...deps, ...bundled },
		bundledDependencies: Object.keys(bundled).sort(),
		optionalDependencies: appPkg.optionalDependencies,
		overrides: appPkg.overrides,
		engines: appPkg.engines,
		keywords: appPkg.keywords,
		author: appPkg.author,
		contributors: appPkg.contributors,
		license: appPkg.license,
		repository: appPkg.repository,
	};
	writeFileSync(join(STAGING, "package.json"), `${JSON.stringify(stagedPkg, null, 2)}\n`);

	// Vendor the built @hamr/* libraries into node_modules so they ship inside
	// the tarball (bundledDependencies copies them from here at pack time).
	for (const lib of LIB_PACKAGES) {
		const libDir = join(ROOT, "packages", lib);
		const dest = join(STAGING, "node_modules", "@hamr", lib);
		mkdirSync(dest, { recursive: true });
		cpSync(join(libDir, "dist"), join(dest, "dist"), { recursive: true });
		cpSync(join(libDir, "package.json"), join(dest, "package.json"));
	}

	// Targeted workaround for protobufjs@7.6.4 under `npm install -g --prefix DIR
	// file.tgz`. npm@11 has a lifecycle-vs-extraction timing bug on this exact
	// install path: for transitive deps of a locally-supplied tarball, lifecycle
	// scripts fire before the dep's own files are fully staged in their final
	// location. protobufjs/scripts/postinstall.js opens with a bare (uncaught)
	//   require(path.join(__dirname, "..", "package.json"))
	// If package.json hasn't landed at __dirname/../ yet, that throws and npm
	// reports a postinstall failure. The same script runs fine for direct registry
	// installs of protobufjs@7.6.4 because those don't go through the tarball
	// staging path. All 7.x releases carry this postinstall; 8.x drops it but
	// @google/genai pins ^7.5.4.
	//
	// Fix: bundle a copy of protobufjs with the postinstall key stripped from
	// package.json. npm extracts it from the tarball rather than fetching from the
	// registry, so the script never runs. This is not generic vendoring — only
	// protobufjs is treated this way.
	console.log("→ Bundling protobufjs (postinstall stripped)…");
	const protoStagedDir = join(STAGING, "node_modules", "protobufjs");
	cpSync(findProtobufjs(), protoStagedDir, { recursive: true });
	const protoStagedPkg = join(protoStagedDir, "package.json");
	const protoPkg = readJson(protoStagedPkg);
	if (protoPkg.scripts?.postinstall) {
		delete protoPkg.scripts.postinstall;
		writeFileSync(protoStagedPkg, `${JSON.stringify(protoPkg, null, 2)}\n`);
	}
	stagedPkg.bundledDependencies.push("protobufjs");
	stagedPkg.bundledDependencies.sort();
	// Re-write staged package.json to include protobufjs in bundledDependencies.
	writeFileSync(join(STAGING, "package.json"), `${JSON.stringify(stagedPkg, null, 2)}\n`);

	console.log("→ Packing…");
	mkdirSync(RELEASES, { recursive: true });
	const before = new Set(readdirSync(STAGING).filter((f) => f.endsWith(".tgz")));
	run("npm pack", STAGING);
	const tarball = readdirSync(STAGING).find((f) => f.endsWith(".tgz") && !before.has(f));
	if (!tarball) throw new Error("npm pack did not produce a tarball");
	const finalName = `skaft-hamr-${appPkg.version}.tgz`;
	renameSync(join(STAGING, tarball), join(RELEASES, finalName));

	console.log(`\n✓ Built ${join("releases", finalName)}`);
	console.log(`  Verify it with: bash scripts/verify-pack.sh releases/${finalName}`);
}

main();
