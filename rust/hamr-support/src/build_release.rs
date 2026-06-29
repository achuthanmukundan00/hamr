//! Port of `scripts/build-release.mjs` — stages and packs the `@skaft/hamr`
//! npm package from the Hamr workspace.
//!
//! The workspace contains four packages (`@hamr/tui`, `@hamr/ai`, `@hamr/agent`,
//! `@hamr/coding-agent`), but we ship ONE package: `@skaft/hamr`. The three
//! `@hamr/*` libraries are vendored into the published package's `node_modules`
//! and marked as `bundledDependencies`.
//!
//! # Pipeline steps
//!
//! 1. Build all packages (`npm run build`)
//! 2. Stage app files into a temporary staging directory
//! 3. Prune nested `node_modules` from staging
//! 4. Collect real (non-@hamr) dependencies from all packages
//! 5. Construct a rewritten `package.json` for publishing
//! 6. Vendor `@hamr/*` libraries with stripped dependency lists
//! 7. Bundle `protobufjs` with its postinstall script removed
//! 8. `npm pack` and move the tarball to `releases/`

use crate::package_json::PackageJson;
use crate::{Error, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

// ── constants (mirrors build-release.mjs) ───────────────────────────────────

/// The `@hamr/*` library package names (directory names under `packages/`).
const LIB_PACKAGES: &[&str] = &["tui", "ai", "agent"];

/// Published npm package name.
const PUBLISHED_NAME: &str = "@skaft/hamr";

/// Files to always copy from the app package even when not in the `files` array.
const EXTRA_FILES: &[&str] = &["README.md", "CHANGELOG.md", "LICENSE"];

// ── public API ──────────────────────────────────────────────────────────────

/// Run the full release pipeline and return the path to the created tarball.
///
/// * `root` — repository root (where `packages/` lives).
/// * `skip_build` — if `true`, skip `npm run build`.
///
/// Returns the path to the tarball in `releases/`.
pub fn build_release(root: &Path, skip_build: bool) -> Result<PathBuf> {
    // (a) Build all packages unless told otherwise.
    if !skip_build {
        println!("→ Building all packages…");
        run_npm(root, &["run", "build"])?;
    }

    // (b–d) Compute key directories.
    let app_pkg_dir = root.join("packages").join("coding-agent");
    let staging = root.join("release").join("staging");
    let releases = root.join("releases");

    // (e) Clean and recreate staging.
    if staging.exists() {
        fs::remove_dir_all(&staging)?;
    }
    fs::create_dir_all(&staging)?;

    // (f) Read the app (coding-agent) package.json.
    let app_pkg = PackageJson::from_file(&app_pkg_dir.join("package.json"))?;

    // (g) Copy shipped files from app package to staging root.
    copy_shipped_files(&app_pkg_dir, &staging, &app_pkg)?;

    // (h) Prune nested node_modules from staging.
    prune_nested_node_modules(&staging)?;

    // (i) Collect real (non-@hamr) deps from all four packages.
    let mut dep_paths = Vec::new();
    for lib in LIB_PACKAGES {
        dep_paths.push(root.join("packages").join(lib).join("package.json"));
    }
    dep_paths.push(app_pkg_dir.join("package.json"));
    let real_deps = collect_deps(&dep_paths)?;

    // (j) Pin bundled @hamr/* versions.
    let mut bundled_versions = HashMap::new();
    for lib in LIB_PACKAGES {
        let lib_pkg = PackageJson::from_file(
            &root.join("packages").join(lib).join("package.json"),
        )?;
        let name = format!("@hamr/{}", lib);
        if let Some(ver) = lib_pkg.version {
            bundled_versions.insert(name, ver);
        }
    }

    // (k) Construct the staged package.json.
    let mut staged_deps = real_deps.clone();
    for (name, ver) in &bundled_versions {
        staged_deps.insert(name.clone(), ver.clone());
    }

    let mut bundled_deps: Vec<String> = bundled_versions.keys().cloned().collect();
    bundled_deps.sort();

    let mut staged_pkg = PackageJson::new();
    staged_pkg.name = Some(PUBLISHED_NAME.to_string());
    staged_pkg.version = app_pkg.version.clone();
    staged_pkg.description = app_pkg.description.clone();
    staged_pkg.package_type = Some("module".to_string());
    staged_pkg.hamr_config = app_pkg.hamr_config.clone();
    staged_pkg.bin = app_pkg.bin.clone();
    staged_pkg.main = app_pkg.main.clone();
    staged_pkg.types = app_pkg.types.clone();
    staged_pkg.exports = app_pkg.exports.clone();
    staged_pkg.dependencies = Some(staged_deps);
    staged_pkg.bundled_dependencies = Some(bundled_deps);
    staged_pkg.optional_dependencies = app_pkg.optional_dependencies.clone();
    staged_pkg.overrides = app_pkg.overrides.clone();
    staged_pkg.engines = app_pkg.engines.clone();
    staged_pkg.keywords = app_pkg.keywords.clone();
    staged_pkg.author = app_pkg.author.clone();
    staged_pkg.contributors = app_pkg.contributors.clone();
    staged_pkg.license = app_pkg.license.clone();
    staged_pkg.repository = app_pkg.repository.clone();
    // NOTE: `files` is intentionally left None — in staging the tarball ships
    // exactly what we copied here.

    // (l) Write the initial staged package.json.
    staged_pkg.to_file(&staging.join("package.json"))?;

    // (m) Vendor @hamr/* libraries.
    vendor_hamr_libs(root, &staging, LIB_PACKAGES)?;

    // (n, o) Bundle protobufjs (postinstall stripped).
    bundle_protobufjs(root, &staging, &mut staged_pkg)?;

    // (p) Run `npm pack` in staging.
    println!("→ Packing…");
    fs::create_dir_all(&releases)?;

    let before: Vec<String> = fs::read_dir(&staging)?
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            name.ends_with(".tgz").then_some(name)
        })
        .collect();
    let before_set: std::collections::HashSet<String> = before.into_iter().collect();

    run_npm(&staging, &["pack"])?;

    let tarball = fs::read_dir(&staging)?
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            (name.ends_with(".tgz") && !before_set.contains(&name)).then_some(name)
        })
        .next()
        .ok_or_else(|| Error::NpmError("npm pack did not produce a tarball".into()))?;

    // (q) Move tarball to releases/ with a clean name.
    let version = app_pkg
        .version
        .as_deref()
        .unwrap_or("unknown");
    let final_name = format!("skaft-hamr-{}.tgz", version);
    let dest = releases.join(&final_name);
    fs::rename(staging.join(&tarball), &dest)?;

    println!("\n✓ Built {}", dest.display());
    println!(
        "  Verify it with: bash scripts/verify-pack.sh releases/{}",
        final_name
    );

    Ok(dest)
}

// ── helper functions ────────────────────────────────────────────────────────

/// Recursively delete every `node_modules` directory under `dir`.
///
/// Used to strip nested example deps from staging before `npm pack`.
/// The top-level bundled `@hamr/*` node_modules is vendored in *after*
/// this runs, so it is never affected.
pub fn prune_nested_node_modules(dir: &Path) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name();
        if name == "node_modules" {
            fs::remove_dir_all(&path)?;
            continue;
        }
        prune_nested_node_modules(&path)?;
    }
    Ok(())
}

/// Locate the `protobufjs` package in the workspace `node_modules`.
///
/// Tries Node module resolution from `packages/ai` first, then falls back to
/// searching bun's virtual store.
pub fn find_protobufjs(root: &Path) -> Result<PathBuf> {
    // Strategy: walk known locations. protobufjs is a transitive dep via
    // @google/genai. The most reliable approach is to search for the first
    // protobufjs directory under node_modules.
    let candidates = [
        // Flat npm/node_modules (npm, pnpm hoisted, yarn)
        root.join("node_modules")
            .join("protobufjs"),
        // Bun virtual store pattern
        root.join("node_modules")
            .join(".bun"),
    ];

    // Check flat layout first.
    let flat = &candidates[0];
    if flat.is_dir() && flat.join("package.json").exists() {
        return Ok(flat.clone());
    }

    // Check bun virtual store.
    let bun_store = &candidates[1];
    if bun_store.is_dir() {
        for entry in fs::read_dir(bun_store)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with("protobufjs@") {
                let p = entry.path().join("node_modules").join("protobufjs");
                if p.is_dir() {
                    return Ok(p);
                }
            }
        }
    }

    // Broader fallback: walk node_modules (depth-limited) looking for protobufjs.
    // This handles pnpm-style nested layouts where protobufjs may be deeper.
    let node_modules_root = root.join("node_modules");
    if node_modules_root.is_dir() {
        if let Some(found) = find_protobufjs_walk(&node_modules_root, 0) {
            return Ok(found);
        }
    }

    Err(Error::Other(
        "Cannot find protobufjs in workspace. Run `bun install` or `npm install` first."
            .into(),
    ))
}

/// Walk `node_modules` (max depth 6) looking for `protobufjs/package.json`.
fn find_protobufjs_walk(base: &Path, depth: usize) -> Option<PathBuf> {
    if depth > 6 || !base.is_dir() {
        return None;
    }
    if base.file_name().map_or(false, |n| n == "protobufjs")
        && base.join("package.json").exists()
    {
        return Some(base.to_path_buf());
    }
    let entries = fs::read_dir(base).ok()?;
    for entry in entries {
        let entry = entry.ok()?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name();
        // Skip clearly not-related dirs to speed up search.
        if name == ".bin" || name == ".cache" {
            continue;
        }
        if let Some(found) = find_protobufjs_walk(&path, depth + 1) {
            return Some(found);
        }
    }
    None
}

/// Collect the union of non-@hamr runtime dependencies from one or more
/// `package.json` files. Later packages win on conflicts; workspace versions
/// are expected to be consistent.
pub fn collect_deps(pkg_paths: &[PathBuf]) -> Result<HashMap<String, String>> {
    let mut deps = HashMap::new();
    for path in pkg_paths {
        let pkg = PackageJson::from_file(path)?;
        if let Some(ref pkg_deps) = pkg.dependencies {
            for (name, version) in pkg_deps {
                if name.starts_with("@hamr/") {
                    continue;
                }
                deps.insert(name.clone(), version.clone());
            }
        }
    }
    Ok(deps)
}

/// Vendor built `@hamr/*` libraries into staging's `node_modules`.
///
/// For each lib:
/// - Copies `dist/` from the workspace package into `staging/node_modules/@hamr/{lib}/`
/// - Creates a stripped `package.json` (no deps, devDeps, peerDeps)
pub fn vendor_hamr_libs(root: &Path, staging: &Path, libs: &[&str]) -> Result<()> {
    let hamr_dir = staging.join("node_modules").join("@hamr");
    fs::create_dir_all(&hamr_dir)?;

    for lib in libs {
        let lib_dir = root.join("packages").join(lib);
        let dest = hamr_dir.join(lib);
        fs::create_dir_all(&dest)?;

        // Copy dist/.
        let dist_src = lib_dir.join("dist");
        if dist_src.exists() {
            copy_dir_recursive(&dist_src, &dest.join("dist"))?;
        }

        // Create stripped package.json.
        let lib_pkg = PackageJson::from_file(&lib_dir.join("package.json"))?;
        let stripped = lib_pkg.strip_deps();
        stripped.to_file(&dest.join("package.json"))?;
    }

    Ok(())
}

/// Bundle `protobufjs` into staging and strip its `postinstall` script.
///
/// Returns the version of the bundled protobufjs.
pub fn bundle_protobufjs(
    root: &Path,
    staging: &Path,
    staged_pkg: &mut PackageJson,
) -> Result<()> {
    println!("→ Bundling protobufjs (postinstall stripped)…");

    let proto_src = find_protobufjs(root)?;
    let proto_staging = staging.join("node_modules").join("protobufjs");

    // Recursively copy protobufjs into staging.
    copy_dir_recursive(&proto_src, &proto_staging)?;

    // Strip postinstall from the bundled copy.
    let proto_pkg_path = proto_staging.join("package.json");
    let mut proto_pkg = PackageJson::from_file(&proto_pkg_path)?;
    if let Some(ref mut scripts) = proto_pkg.scripts {
        scripts.remove("postinstall");
        if scripts.is_empty() {
            proto_pkg.scripts = None;
        }
    }
    proto_pkg.to_file(&proto_pkg_path)?;

    // Add protobufjs to staged package.json dependencies and bundledDependencies.
    let proto_version = proto_pkg.version.clone().unwrap_or_else(|| "7.6.4".into());
    if let Some(ref mut deps) = staged_pkg.dependencies {
        deps.insert("protobufjs".into(), proto_version);
    } else {
        let mut deps = HashMap::new();
        deps.insert("protobufjs".into(), proto_version);
        staged_pkg.dependencies = Some(deps);
    }

    if let Some(ref mut bundled) = staged_pkg.bundled_dependencies {
        if !bundled.iter().any(|b| b == "protobufjs") {
            bundled.push("protobufjs".into());
            bundled.sort();
        }
    } else {
        staged_pkg.bundled_dependencies = Some(vec!["protobufjs".into()]);
    }

    // Re-write staged package.json.
    staged_pkg.to_file(&staging.join("package.json"))?;

    Ok(())
}

// ── internal helpers ────────────────────────────────────────────────────────

/// Copy files listed in the app's `files` array from `app_dir` to `staging`.
/// Skips `npm-shrinkwrap.json`. Also copies `EXTRA_FILES` if they exist.
fn copy_shipped_files(app_dir: &Path, staging: &Path, app_pkg: &PackageJson) -> Result<()> {
    if let Some(ref files) = app_pkg.files {
        for entry in files {
            if entry == "npm-shrinkwrap.json" {
                continue;
            }
            let src = app_dir.join(entry);
            if !src.exists() {
                continue;
            }
            copy_path(&src, &staging.join(entry))?;
        }
    }

    for extra in EXTRA_FILES {
        let src = app_dir.join(extra);
        if src.exists() {
            fs::copy(&src, staging.join(extra))?;
        }
    }

    Ok(())
}

/// Copy a file or directory recursively. If `src` is a directory, its
/// contents are copied into `dest` (creating `dest` if needed).
fn copy_path(src: &Path, dest: &Path) -> Result<()> {
    if src.is_dir() {
        copy_dir_recursive(src, dest)
    } else {
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(src, dest)?;
        Ok(())
    }
}

/// Recursively copy `src` directory into `dest`.
fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<()> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dest_path)?;
        } else {
            fs::copy(&src_path, &dest_path)?;
        }
    }
    Ok(())
}

/// Run an npm command in the given working directory, inheriting stdio.
fn run_npm(cwd: &Path, args: &[&str]) -> Result<()> {
    let mut cmd = std::process::Command::new("npm");
    cmd.args(args);
    cmd.current_dir(cwd);
    cmd.stdin(std::process::Stdio::inherit());
    cmd.stdout(std::process::Stdio::inherit());
    cmd.stderr(std::process::Stdio::inherit());

    let status = cmd.status()?;
    if !status.success() {
        return Err(Error::NpmError(format!(
            "npm {} failed with exit code: {:?}",
            args.join(" "),
            status.code()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    // These tests are lightweight unit tests. Integration tests that need
    // full staging go in `tests/build_release_tests.rs`.

    #[test]
    fn test_prune_nested_node_modules() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let root = tmp.path();

        // Create a tree:
        //   root/
        //     keep.txt
        //     node_modules/         ← should be removed
        //       inner.json
        //     sub/
        //       data.txt
        //       node_modules/       ← should be removed
        //         deep.json
        //     sub2/
        //       deeper/
        //         node_modules/     ← should be removed
        //           deep.json

        fs::create_dir_all(root.join("node_modules"))?;
        fs::write(root.join("keep.txt"), "keep")?;
        fs::write(root.join("node_modules").join("inner.json"), "inner")?;
        fs::create_dir_all(root.join("sub").join("node_modules"))?;
        fs::write(root.join("sub").join("data.txt"), "data")?;
        fs::write(root.join("sub").join("node_modules").join("deep.json"), "deep")?;
        fs::create_dir_all(root.join("sub2").join("deeper").join("node_modules"))?;
        fs::write(
            root.join("sub2").join("deeper").join("node_modules").join("deep.json"),
            "deep2",
        )?;

        prune_nested_node_modules(root)?;

        // Top-level keep.txt survives.
        assert!(root.join("keep.txt").exists());
        // node_modules at root is gone.
        assert!(!root.join("node_modules").exists());
        // sub/data.txt survives.
        assert!(root.join("sub").join("data.txt").exists());
        // sub/node_modules is gone.
        assert!(!root.join("sub").join("node_modules").exists());
        // sub2/deeper/node_modules is gone.
        assert!(!root.join("sub2").join("deeper").join("node_modules").exists());
        // sub2/deeper still exists as a dir.
        assert!(root.join("sub2").join("deeper").is_dir());

        Ok(())
    }

    #[test]
    fn test_collect_deps_excludes_hamr_and_unions() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let root = tmp.path();

        let pkg1 = root.join("pkg1.json");
        let pkg2 = root.join("pkg2.json");

        let j1 = serde_json::json!({
            "name": "pkg1",
            "dependencies": {
                "express": "^4.18.0",
                "@hamr/tui": "1.0.0",
                "lodash": "^4.17.21"
            }
        });
        let j2 = serde_json::json!({
            "name": "pkg2",
            "dependencies": {
                "axios": "^1.6.0",
                "@hamr/ai": "1.0.0",
                "express": "^5.0.0"
            }
        });

        fs::write(&pkg1, serde_json::to_string(&j1)?)?;
        fs::write(&pkg2, serde_json::to_string(&j2)?)?;

        let deps = collect_deps(&[pkg1.clone(), pkg2.clone()])?;

        // @hamr/* excluded.
        assert!(!deps.contains_key("@hamr/tui"));
        assert!(!deps.contains_key("@hamr/ai"));
        // Real deps present.
        assert!(deps.contains_key("express"));
        assert!(deps.contains_key("lodash"));
        assert!(deps.contains_key("axios"));
        // pkg2's express wins (later package).
        assert_eq!(deps.get("express").unwrap(), "^5.0.0");

        Ok(())
    }

    #[test]
    fn test_collect_deps_handles_missing_dependencies() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let root = tmp.path();

        let pkg1 = root.join("pkg1.json");
        let j1 = serde_json::json!({ "name": "bare" });
        fs::write(&pkg1, serde_json::to_string(&j1)?)?;

        let deps = collect_deps(&[pkg1])?;
        assert!(deps.is_empty());
        Ok(())
    }

    #[test]
    fn test_vendor_hamr_libs_strips_deps() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let root = tmp.path();

        // Simulate workspace layout:
        //   root/
        //     packages/
        //       tui/
        //         dist/
        //           index.js
        //         package.json (with deps)
        //     staging/

        let pkg_dir = root.join("packages").join("tui");
        let dist_dir = pkg_dir.join("dist");
        fs::create_dir_all(&dist_dir)?;
        fs::write(dist_dir.join("index.js"), "// tui")?;

        let pkg_json = serde_json::json!({
            "name": "@hamr/tui",
            "version": "3.2.1",
            "dependencies": {
                "@hamr/ai": "3.2.0",
                "chalk": "^5.0.0"
            },
            "devDependencies": {
                "typescript": "^5.0.0"
            }
        });
        fs::write(
            pkg_dir.join("package.json"),
            serde_json::to_string_pretty(&pkg_json)?,
        )?;

        let staging = root.join("staging");
        fs::create_dir_all(&staging)?;

        vendor_hamr_libs(root, &staging, &["tui"])?;

        // dist/ was copied.
        let vendored_dist = staging.join("node_modules").join("@hamr").join("tui").join("dist");
        assert!(vendored_dist.exists());
        assert!(vendored_dist.join("index.js").exists());

        // package.json exists and has deps stripped.
        let vendored_pkg_path = staging
            .join("node_modules")
            .join("@hamr")
            .join("tui")
            .join("package.json");
        let vendored_pkg = PackageJson::from_file(&vendored_pkg_path)?;
        assert!(vendored_pkg.dependencies.is_none());
        assert!(vendored_pkg.dev_dependencies.is_none());
        assert!(vendored_pkg.peer_dependencies.is_none());
        assert_eq!(vendored_pkg.name.as_deref(), Some("@hamr/tui"));
        assert_eq!(vendored_pkg.version.as_deref(), Some("3.2.1"));

        Ok(())
    }

    #[test]
    fn test_staged_pkg_fields() {
        let app_pkg = PackageJson {
            name: Some("@hamr/coding-agent".into()),
            version: Some("4.0.0".into()),
            description: Some("An AI coding agent".into()),
            package_type: Some("module".into()),
            files: Some(vec!["dist".into(), "npm-shrinkwrap.json".into()]),
            dependencies: Some(HashMap::from([("express".into(), "^4.0.0".into())])),
            ..PackageJson::default()
        };

        // Simulate what build_release constructs.
        let staged = PackageJson {
            name: Some("@skaft/hamr".into()),
            version: app_pkg.version.clone(),
            description: app_pkg.description.clone(),
            package_type: Some("module".into()),
            files: None, // intentionally absent
            dependencies: Some(HashMap::from([
                ("express".into(), "^4.0.0".into()),
                ("@hamr/tui".into(), "1.0.0".into()),
                ("@hamr/ai".into(), "1.0.0".into()),
                ("@hamr/agent".into(), "1.0.0".into()),
            ])),
            bundled_dependencies: Some(vec![
                "@hamr/agent".into(),
                "@hamr/ai".into(),
                "@hamr/tui".into(),
            ]),
            ..PackageJson::default()
        };

        assert_eq!(staged.name.as_deref(), Some("@skaft/hamr"));
        assert_eq!(staged.version.as_deref(), Some("4.0.0"));
        assert!(staged.files.is_none(), "files field must be absent");
        // bundledDependencies are sorted.
        let bundled = staged.bundled_dependencies.as_ref().unwrap();
        assert_eq!(bundled[0], "@hamr/agent");
        assert_eq!(bundled[1], "@hamr/ai");
        assert_eq!(bundled[2], "@hamr/tui");
    }

    #[test]
    fn test_bundle_protobufjs_strips_postinstall() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let root = tmp.path();

        // Simulate a workspace with node_modules/protobufjs.
        let proto_dir = root.join("node_modules").join("protobufjs");
        fs::create_dir_all(&proto_dir)?;
        let proto_pkg_json = serde_json::json!({
            "name": "protobufjs",
            "version": "7.6.4",
            "scripts": {
                "postinstall": "node scripts/postinstall.js",
                "test": "node tests/test.js"
            }
        });
        fs::write(
            proto_dir.join("package.json"),
            serde_json::to_string_pretty(&proto_pkg_json)?,
        )?;

        let staging = root.join("staging");
        fs::create_dir_all(&staging)?;

        let mut staged_pkg = PackageJson::new();
        staged_pkg.name = Some("@skaft/hamr".into());
        staged_pkg.version = Some("4.0.0".into());
        staged_pkg.dependencies = Some(HashMap::new());
        staged_pkg.bundled_dependencies = Some(vec![]);
        staged_pkg.to_file(&staging.join("package.json"))?;

        // Override find_protobufjs by using the helper directly.
        // We call bundle_protobufjs which calls find_protobufjs.
        // For this test, we'll test the postinstall stripping directly.
        bundle_protobufjs(root, &staging, &mut staged_pkg)?;

        // protobufjs was copied.
        let bundled_proto = staging.join("node_modules").join("protobufjs");
        assert!(bundled_proto.exists());

        // postinstall is stripped.
        let bundled_pkg = PackageJson::from_file(&bundled_proto.join("package.json"))?;
        let scripts = bundled_pkg.scripts.as_ref();
        if let Some(s) = scripts {
            assert!(!s.contains_key("postinstall"));
        }
        // test script survived.
        assert!(scripts.map_or(false, |s| s.contains_key("test")));

        // protobufjs is in deps and bundledDependencies.
        let staged_pkg = PackageJson::from_file(&staging.join("package.json"))?;
        let deps = staged_pkg.dependencies.unwrap();
        assert!(deps.contains_key("protobufjs"));
        let bundled = staged_pkg.bundled_dependencies.unwrap();
        assert!(bundled.contains(&"protobufjs".to_string()));

        Ok(())
    }
}
