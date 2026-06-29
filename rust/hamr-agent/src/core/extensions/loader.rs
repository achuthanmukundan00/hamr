//! Extension loader — discovers and loads extensions from filesystem paths.
//!
//! Port of `packages/coding-agent/src/core/extensions/loader.ts`.
//!
//! Discovers and loads extensions from filesystem paths. In the TypeScript version,
//! this uses `jiti` for dynamic imports of TypeScript modules. In Rust, this maps to:
//! - Factory-loaded extensions: from `Box<dyn ExtensionFactory>` closures registered at startup
//! - Rhai extensions: loaded from `*.rhai` files (feature-gated)
//!
//! The loader scans `~/.hamr/extensions/` and project-local `.hamr/extensions/`
//! directories, as well as explicitly configured paths.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use super::types::*;
use crate::core::event_bus::EventBus;
use crate::core::source_info::{
    SourceScope, SyntheticSourceInfoOptions, create_synthetic_source_info,
};

// ---------------------------------------------------------------------------
// Directory constants
// ---------------------------------------------------------------------------

const HAMR_CONFIG_DIR: &str = ".hamr";

// ---------------------------------------------------------------------------
// Extension filename patterns
// ---------------------------------------------------------------------------

/// Check if a filename is an extension file (JS or TS).
/// In Rust, we check for `.rhai` files when the `rhai-extensions` feature is enabled.
fn is_extension_file(name: &str) -> bool {
    name.ends_with(".ts")
        || name.ends_with(".js")
        || (cfg!(feature = "rhai-extensions") && name.ends_with(".rhai"))
}

// ---------------------------------------------------------------------------
// PiManifest — reads package.json `hamr.extensions` field
// ---------------------------------------------------------------------------

/// Manifest format from package.json `pi` or `hamr` field.
#[derive(Debug, Default, serde::Deserialize)]
struct PiManifest {
    #[serde(default)]
    extensions: Vec<String>,
}

/// Read the `pi` manifest from a package.json file.
fn read_pi_manifest(package_json_path: &str) -> Option<PiManifest> {
    let content = std::fs::read_to_string(package_json_path).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&content).ok()?;

    // Check both "hamr" and "pi" keys.
    let pi_section = parsed.get("hamr").or_else(|| parsed.get("pi"))?;

    if !pi_section.is_object() {
        return None;
    }

    let manifest: PiManifest = serde_json::from_value(pi_section.clone()).ok()?;
    if manifest.extensions.is_empty() {
        return None;
    }
    Some(manifest)
}

// ---------------------------------------------------------------------------
// Extension entry point resolution
// ---------------------------------------------------------------------------

/// Resolve extension entry points from a directory.
///
/// Checks for:
/// 1. package.json with `hamr.extensions` field — returns declared paths
/// 2. index.ts or index.js — returns the index file
///
/// Returns resolved paths or None if no entry points found.
///
/// Mirrors TS `resolveExtensionEntries()`.
fn resolve_extension_entries(dir: &str) -> Option<Vec<String>> {
    let package_json_path = format!("{dir}/package.json");
    let pkg_path = Path::new(&package_json_path);

    if pkg_path.exists() {
        if let Some(manifest) = read_pi_manifest(&package_json_path) {
            let entries: Vec<String> = manifest
                .extensions
                .iter()
                .map(|ext_path| {
                    let resolved = Path::new(dir).join(ext_path);
                    resolved.to_string_lossy().to_string()
                })
                .filter(|p| Path::new(p).exists())
                .collect();

            if !entries.is_empty() {
                return Some(entries);
            }
        }
    }

    // Check for index.ts or index.js
    let index_ts = format!("{dir}/index.ts");
    let index_js = format!("{dir}/index.js");
    if Path::new(&index_ts).exists() {
        return Some(vec![index_ts]);
    }
    if Path::new(&index_js).exists() {
        return Some(vec![index_js]);
    }

    None
}

// ---------------------------------------------------------------------------
// Extension file discovery
// ---------------------------------------------------------------------------

/// Discover extensions in a directory.
///
/// Discovery rules:
/// 1. Direct files: `extensions/*.ts` or `*.js` — load
/// 2. Subdirectory with index: `extensions/<name>/index.ts` or `index.js` — load
/// 3. Subdirectory with package.json: `extensions/<name>/package.json` with `hamr` field — load
///
/// No recursion beyond one level.
///
/// Mirrors TS `discoverExtensionsInDir()`.
fn discover_extensions_in_dir(dir: &str) -> Vec<String> {
    let dir_path = Path::new(dir);
    if !dir_path.exists() || !dir_path.is_dir() {
        return Vec::new();
    }

    let mut discovered: Vec<String> = Vec::new();

    let entries = match std::fs::read_dir(dir_path) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    for entry in entries.flatten() {
        let entry_path = entry.path();
        let file_name = match entry.file_name().to_str() {
            Some(n) => n.to_string(),
            None => continue,
        };

        // 1. Direct files: *.ts or *.js
        if entry_path.is_file() && is_extension_file(&file_name) {
            discovered.push(entry_path.to_string_lossy().to_string());
            continue;
        }

        // 2 & 3. Subdirectories
        if entry_path.is_dir() {
            let sub_dir = entry_path.to_string_lossy().to_string();
            if let Some(entries) = resolve_extension_entries(&sub_dir) {
                discovered.extend(entries);
            }
        }
    }

    discovered
}

// ---------------------------------------------------------------------------
// Create Extension
// ---------------------------------------------------------------------------

/// Create an Extension object with empty collections.
///
/// Mirrors TS `createExtension()`.
fn create_extension(extension_path: &str, resolved_path: &str) -> Extension {
    let (source, base_dir_str) = if extension_path.starts_with('<') && extension_path.ends_with('>')
    {
        let inner = &extension_path[1..extension_path.len() - 1];
        let source = inner.split(':').next().unwrap_or("temporary");
        (source.to_string(), None)
    } else {
        let base_dir = Path::new(resolved_path)
            .parent()
            .and_then(|p| p.to_str())
            .map(|s| s.to_string());
        ("local".to_string(), base_dir)
    };

    let scope = if source == "temporary" {
        SourceScope::Temporary
    } else {
        SourceScope::User
    };

    let source_info = create_synthetic_source_info(
        extension_path,
        SyntheticSourceInfoOptions {
            source,
            scope: Some(scope),
            origin: None,
            base_dir: base_dir_str,
        },
    );

    Extension::new(
        extension_path.to_string(),
        resolved_path.to_string(),
        source_info,
    )
}

// ---------------------------------------------------------------------------
// Load extension from path
// ---------------------------------------------------------------------------

/// Load a single extension from a file path.
///
/// In the TS version, this uses `jiti` to dynamically import TypeScript modules.
/// In Rust, this resolves the factory from the provided factory registry,
/// or attempts to load a `.rhai` file if the `rhai-extensions` feature is enabled.
///
/// `factories` is a map from file paths to `ExtensionFactory` closures.
/// Built-in extensions register their factories at startup.
///
/// Returns the loaded extension or an error string.
///
/// Mirrors TS `loadExtension()`.
async fn load_extension_from_path(
    extension_path: &str,
    cwd: &str,
    _event_bus: Arc<dyn EventBus>,
    runtime: &ExtensionRuntime,
    factories: &HashMap<String, ExtensionFactory>,
) -> (Option<Extension>, Option<String>) {
    // Resolve the path
    let resolved = if Path::new(extension_path).is_absolute() {
        extension_path.to_string()
    } else {
        let cwd_path = Path::new(cwd);
        let joined = cwd_path.join(extension_path);
        joined.to_string_lossy().to_string()
    };

    // Check if we have a registered factory for this path
    if let Some(factory) = factories
        .get(&resolved)
        .or_else(|| factories.get(extension_path))
    {
        // Create the extension object
        let ext = create_extension(extension_path, &resolved);

        // Create the ExtensionAPI and run the factory
        let api = crate::core::extensions::api_impl::ExtensionAPIImpl::new(
            ext,
            runtime.clone(),
            cwd.to_string(),
            _event_bus.clone(),
        );

        // Wrap in Arc for the factory, then consume after
        let api_arc = Arc::new(api);
        factory(api_arc.clone() as Arc<dyn ExtensionAPI>).await;

        // Extract the populated extension (unwrap Arc)
        let api = Arc::try_unwrap(api_arc).unwrap_or_else(|_| {
            panic!("ExtensionAPI Arc still has multiple references after factory call");
        });
        return (Some(api.into_extension()), None);
    }

    // Check for .rhai extensions (feature-gated)
    #[cfg(feature = "rhai-extensions")]
    {
        if resolved.ends_with(".rhai") {
            return match load_rhai_extension(&resolved) {
                Ok(ext) => (Some(ext), None),
                Err(e) => (None, Some(e)),
            };
        }
    }

    // No factory found: return placeholder extension
    let ext = create_extension(extension_path, &resolved);
    (Some(ext), None)
}

/// Load a Rhai extension.
///
/// Feature-gated behind `rhai-extensions`.
#[cfg(feature = "rhai-extensions")]
fn load_rhai_extension(path: &str) -> Result<Extension, String> {
    use crate::core::extensions::types::{Extension, SourceInfo};
    use std::collections::HashMap;

    let mut engine = rhai::Engine::new();

    // Compile the script
    let ast = engine
        .compile_file(std::path::PathBuf::from(path))
        .map_err(|e| format!("Rhai compile error in {path}: {e}"))?;

    // Create a scope and evaluate the script
    let mut scope = rhai::Scope::new();
    engine
        .run_ast_with_scope(&mut scope, &ast)
        .map_err(|e| format!("Rhai runtime error in {path}: {e}"))?;

    // Build a minimal extension from the script path
    Ok(Extension {
        path: path.to_string(),
        resolved_path: path.to_string(),
        source_info: SourceInfo {
            kind: "rhai".to_string(),
            path: Some(path.to_string()),
            module_path: None,
            package_name: None,
        },
        handlers: HashMap::new(),
        tools: HashMap::new(),
        message_renderers: HashMap::new(),
        role_message_renderers: HashMap::new(),
        commands: HashMap::new(),
        flags: HashMap::new(),
        shortcuts: HashMap::new(),
    })
}

// ---------------------------------------------------------------------------
// Build a directory path for extensions
// ---------------------------------------------------------------------------

fn get_agent_dir() -> String {
    if let Ok(dir) = std::env::var("HAMR_AGENT_DIR") {
        return dir;
    }
    if let Ok(dir) = std::env::var("PI_AGENT_DIR") {
        return dir;
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    format!("{home}/{HAMR_CONFIG_DIR}/agent")
}

// ---------------------------------------------------------------------------
// Load extensions from paths
// ---------------------------------------------------------------------------

/// Load extensions from a list of file paths.
///
/// `factories` is a map from module paths to `ExtensionFactory` closures,
/// which are used to load compiled extensions.
///
/// Mirrors TS `loadExtensions()`.
pub async fn load_extensions(
    paths: &[String],
    cwd: &str,
    event_bus: Option<Arc<dyn EventBus>>,
    runtime: Option<ExtensionRuntime>,
    factories: &HashMap<String, ExtensionFactory>,
) -> LoadExtensionsResult {
    let mut extensions: Vec<Extension> = Vec::new();
    let mut errors: Vec<ExtensionLoadError> = Vec::new();
    let _resolved_event_bus = event_bus.unwrap_or_else(|| {
        // Create a no-op event bus
        let bus = crate::core::event_bus::create_event_bus();
        Arc::new(bus)
    });
    let _resolved_runtime = runtime.unwrap_or_else(ExtensionRuntime::new);

    for ext_path in paths {
        let (ext, error) = load_extension_from_path(
            ext_path,
            cwd,
            _resolved_event_bus.clone(),
            &_resolved_runtime,
            factories,
        )
        .await;

        if let Some(err) = error {
            errors.push(ExtensionLoadError {
                path: ext_path.clone(),
                error: err,
            });
            continue;
        }

        if let Some(ext) = ext {
            extensions.push(ext);
        }
    }

    LoadExtensionsResult { extensions, errors }
}

// ---------------------------------------------------------------------------
// Load extension from inline factory
// ---------------------------------------------------------------------------

/// Create an Extension from an inline factory function.
///
/// Mirrors TS `loadExtensionFromFactory()`.
pub async fn load_extension_from_factory(
    factory: ExtensionFactory,
    cwd: &str,
    event_bus: Arc<dyn EventBus>,
    runtime: &ExtensionRuntime,
    extension_path: Option<&str>,
) -> Extension {
    let path = extension_path.unwrap_or("<inline>");
    let ext = create_extension(path, path);

    // Create the ExtensionAPI and run the factory
    let api = crate::core::extensions::api_impl::ExtensionAPIImpl::new(
        ext,
        runtime.clone(),
        cwd.to_string(),
        event_bus,
    );

    // Factories and the handlers they register may retain the API for later
    // actions. That is normal extension behavior, so the populated extension
    // must be snapshotted instead of treating a retained Arc as a leak.
    let api_arc = Arc::new(api);

    // Run the factory in a scoped block so the clone is dropped cleanly
    {
        let api_for_factory: Arc<dyn ExtensionAPI> = api_arc.clone();
        factory(api_for_factory).await;
    }

    // Drop the factory closure to release any captured references
    drop(factory);

    // Recover the populated extension when no handler retained the API;
    // otherwise snapshot it without emitting a false-positive startup warning.
    match Arc::try_unwrap(api_arc) {
        Ok(api) => api.into_extension(),
        Err(arc) => arc.clone_extension(),
    }
}

// ---------------------------------------------------------------------------
// Discover and load extensions
// ---------------------------------------------------------------------------

/// Discover and load extensions from standard locations.
///
/// Scans:
/// 1. Project-local extensions: `cwd/.hamr/extensions/`
/// 2. Global extensions: `agent_dir/extensions/`
/// 3. Explicitly configured paths (files or directories)
///
/// `factories` is a map of pre-registered extension factories keyed by module path.
///
/// Mirrors TS `discoverAndLoadExtensions()`.
pub async fn discover_and_load_extensions(
    configured_paths: &[String],
    cwd: &str,
    agent_dir: Option<&str>,
    event_bus: Option<Arc<dyn EventBus>>,
    factories: &HashMap<String, ExtensionFactory>,
) -> LoadExtensionsResult {
    let resolved_cwd = cwd;
    let resolved_agent_dir = agent_dir.unwrap_or_else(|| {
        // Leak the string for static lifetime; in practice we use it below
        Box::leak(get_agent_dir().into_boxed_str())
    });

    let mut all_paths: Vec<String> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    let mut add_path = |p: &str| {
        let resolved = std::path::absolute(p)
            .map(|a| a.to_string_lossy().to_string())
            .unwrap_or_else(|_| p.to_string());
        if !seen.contains(&resolved) {
            seen.insert(resolved.clone());
            all_paths.push(resolved);
        }
    };

    // 1. Project-local extensions: cwd/.hamr/extensions/
    let local_ext_dir = format!("{resolved_cwd}/{HAMR_CONFIG_DIR}/extensions");
    for p in discover_extensions_in_dir(&local_ext_dir) {
        add_path(&p);
    }

    // 2. Global extensions: agent_dir/extensions/
    let global_ext_dir = format!("{resolved_agent_dir}/extensions");
    for p in discover_extensions_in_dir(&global_ext_dir) {
        add_path(&p);
    }

    // 3. Explicitly configured paths
    for configured in configured_paths {
        let configured_path = if Path::new(configured).is_absolute() {
            configured.clone()
        } else {
            let joined = Path::new(resolved_cwd).join(configured);
            joined.to_string_lossy().to_string()
        };

        let path = Path::new(&configured_path);
        if path.exists() && path.is_dir() {
            // Check for package.json manifest or index.ts
            if let Some(entries) = resolve_extension_entries(&configured_path) {
                for entry in entries {
                    add_path(&entry);
                }
                continue;
            }
            // No explicit entries: discover individual files
            for p in discover_extensions_in_dir(&configured_path) {
                add_path(&p);
            }
            continue;
        }

        add_path(&configured_path);
    }

    load_extensions(&all_paths, resolved_cwd, event_bus, None, factories).await
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_extension_file() {
        assert!(is_extension_file("foo.ts"));
        assert!(is_extension_file("foo.js"));
        assert!(!is_extension_file("foo.txt"));
        assert!(!is_extension_file("foo.json"));
        assert!(!is_extension_file(""));
    }

    #[test]
    fn test_discover_extensions_in_dir_nonexistent() {
        let paths = discover_extensions_in_dir("/nonexistent/path");
        assert!(paths.is_empty());
    }

    #[test]
    fn test_resolve_extension_entries_no_package_json() {
        // Create a temp dir with index.ts
        let dir = tempfile::tempdir().unwrap();
        let index_path = dir.path().join("index.ts");
        std::fs::write(&index_path, "// test").unwrap();

        let dir_str = dir.path().to_string_lossy().to_string();
        let entries = resolve_extension_entries(&dir_str);
        assert!(entries.is_some());
        let entries = entries.unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].ends_with("index.ts"));
    }

    #[test]
    fn test_read_pi_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let pkg_path = dir.path().join("package.json");
        std::fs::write(&pkg_path, r#"{"pi": {"extensions": ["./dist/index.js"]}}"#).unwrap();

        let manifest = read_pi_manifest(&pkg_path.to_string_lossy());
        assert!(manifest.is_some());
        assert_eq!(manifest.unwrap().extensions[0], "./dist/index.js");
    }

    // -----------------------------------------------------------------------
    // Discovery: direct files
    // -----------------------------------------------------------------------

    #[test]
    fn test_discover_direct_ts_files() {
        let dir = tempfile::tempdir().unwrap();
        let ext_dir = dir.path().join("extensions");
        std::fs::create_dir(&ext_dir).unwrap();
        std::fs::write(ext_dir.join("foo.ts"), "// test").unwrap();
        std::fs::write(ext_dir.join("bar.ts"), "// test").unwrap();

        let paths = discover_extensions_in_dir(&ext_dir.to_string_lossy());
        assert_eq!(paths.len(), 2);
        let basenames: Vec<String> = paths
            .iter()
            .map(|p| {
                std::path::Path::new(p)
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            })
            .collect();
        assert!(basenames.contains(&"foo.ts".to_string()));
        assert!(basenames.contains(&"bar.ts".to_string()));
    }

    #[test]
    fn test_discover_direct_js_files() {
        let dir = tempfile::tempdir().unwrap();
        let ext_dir = dir.path().join("extensions");
        std::fs::create_dir(&ext_dir).unwrap();
        std::fs::write(ext_dir.join("foo.js"), "// test").unwrap();

        let paths = discover_extensions_in_dir(&ext_dir.to_string_lossy());
        assert_eq!(paths.len(), 1);
        assert!(paths[0].ends_with("foo.js"));
    }

    #[test]
    fn test_discover_ignores_non_extension_files() {
        let dir = tempfile::tempdir().unwrap();
        let ext_dir = dir.path().join("extensions");
        std::fs::create_dir(&ext_dir).unwrap();
        std::fs::write(ext_dir.join("readme.md"), "# docs").unwrap();
        std::fs::write(ext_dir.join("config.json"), "{}").unwrap();

        let paths = discover_extensions_in_dir(&ext_dir.to_string_lossy());
        assert!(paths.is_empty());
    }

    // -----------------------------------------------------------------------
    // Discovery: subdirectories
    // -----------------------------------------------------------------------

    #[test]
    fn test_discover_subdirectory_with_index_ts() {
        let dir = tempfile::tempdir().unwrap();
        let ext_dir = dir.path().join("extensions");
        std::fs::create_dir(&ext_dir).unwrap();
        let subdir = ext_dir.join("my-extension");
        std::fs::create_dir(&subdir).unwrap();
        std::fs::write(subdir.join("index.ts"), "// test").unwrap();

        let paths = discover_extensions_in_dir(&ext_dir.to_string_lossy());
        assert_eq!(paths.len(), 1);
        assert!(paths[0].contains("my-extension"));
        assert!(paths[0].ends_with("index.ts"));
    }

    #[test]
    fn test_discover_subdirectory_with_index_js() {
        let dir = tempfile::tempdir().unwrap();
        let ext_dir = dir.path().join("extensions");
        std::fs::create_dir(&ext_dir).unwrap();
        let subdir = ext_dir.join("my-extension");
        std::fs::create_dir(&subdir).unwrap();
        std::fs::write(subdir.join("index.js"), "// test").unwrap();

        let paths = discover_extensions_in_dir(&ext_dir.to_string_lossy());
        assert_eq!(paths.len(), 1);
        assert!(paths[0].ends_with("index.js"));
    }

    #[test]
    fn test_discover_prefers_index_ts_over_index_js() {
        let dir = tempfile::tempdir().unwrap();
        let ext_dir = dir.path().join("extensions");
        std::fs::create_dir(&ext_dir).unwrap();
        let subdir = ext_dir.join("my-extension");
        std::fs::create_dir(&subdir).unwrap();
        std::fs::write(subdir.join("index.ts"), "// ts version").unwrap();
        std::fs::write(subdir.join("index.js"), "// js version").unwrap();

        let paths = discover_extensions_in_dir(&ext_dir.to_string_lossy());
        assert_eq!(paths.len(), 1);
        // index.ts is checked before index.js in resolve_extension_entries
        assert!(paths[0].ends_with("index.ts"));
    }

    #[test]
    fn test_discover_subdirectory_with_package_json_pi_field() {
        let dir = tempfile::tempdir().unwrap();
        let ext_dir = dir.path().join("extensions");
        std::fs::create_dir(&ext_dir).unwrap();
        let subdir = ext_dir.join("my-package");
        let src_dir = subdir.join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(src_dir.join("main.ts"), "// test").unwrap();
        std::fs::write(
            subdir.join("package.json"),
            r#"{"name":"my-package","pi":{"extensions":["./src/main.ts"]}}"#,
        )
        .unwrap();

        let paths = discover_extensions_in_dir(&ext_dir.to_string_lossy());
        assert_eq!(paths.len(), 1);
        assert!(paths[0].contains("src"));
        assert!(paths[0].ends_with("main.ts"));
    }

    #[test]
    fn test_discover_package_json_multiple_extensions() {
        let dir = tempfile::tempdir().unwrap();
        let ext_dir = dir.path().join("extensions");
        std::fs::create_dir(&ext_dir).unwrap();
        let subdir = ext_dir.join("my-package");
        std::fs::create_dir(&subdir).unwrap();
        std::fs::write(subdir.join("ext1.ts"), "// test").unwrap();
        std::fs::write(subdir.join("ext2.ts"), "// test").unwrap();
        std::fs::write(
            subdir.join("package.json"),
            r#"{"name":"my-package","pi":{"extensions":["./ext1.ts","./ext2.ts"]}}"#,
        )
        .unwrap();

        let paths = discover_extensions_in_dir(&ext_dir.to_string_lossy());
        assert_eq!(paths.len(), 2);
    }

    #[test]
    fn test_discover_package_json_precedence_over_index() {
        let dir = tempfile::tempdir().unwrap();
        let ext_dir = dir.path().join("extensions");
        std::fs::create_dir(&ext_dir).unwrap();
        let subdir = ext_dir.join("my-package");
        std::fs::create_dir(&subdir).unwrap();
        std::fs::write(subdir.join("index.ts"), "// index").unwrap();
        std::fs::write(subdir.join("custom.ts"), "// custom").unwrap();
        std::fs::write(
            subdir.join("package.json"),
            r#"{"name":"my-package","pi":{"extensions":["./custom.ts"]}}"#,
        )
        .unwrap();

        let paths = discover_extensions_in_dir(&ext_dir.to_string_lossy());
        assert_eq!(paths.len(), 1);
        assert!(paths[0].ends_with("custom.ts"));
    }

    #[test]
    fn test_discover_package_json_without_pi_field_falls_back_to_index() {
        let dir = tempfile::tempdir().unwrap();
        let ext_dir = dir.path().join("extensions");
        std::fs::create_dir(&ext_dir).unwrap();
        let subdir = ext_dir.join("my-package");
        std::fs::create_dir(&subdir).unwrap();
        std::fs::write(subdir.join("index.ts"), "// test").unwrap();
        std::fs::write(
            subdir.join("package.json"),
            r#"{"name":"my-package","version":"1.0.0"}"#,
        )
        .unwrap();

        let paths = discover_extensions_in_dir(&ext_dir.to_string_lossy());
        assert_eq!(paths.len(), 1);
        assert!(paths[0].ends_with("index.ts"));
    }

    #[test]
    fn test_discover_ignores_subdirectory_without_index_or_package_json() {
        let dir = tempfile::tempdir().unwrap();
        let ext_dir = dir.path().join("extensions");
        std::fs::create_dir(&ext_dir).unwrap();
        let subdir = ext_dir.join("not-an-extension");
        std::fs::create_dir(&subdir).unwrap();
        std::fs::write(subdir.join("helper.ts"), "// not an entry point").unwrap();

        let paths = discover_extensions_in_dir(&ext_dir.to_string_lossy());
        assert!(paths.is_empty());
    }

    #[test]
    fn test_discover_does_not_recurse_beyond_one_level() {
        let dir = tempfile::tempdir().unwrap();
        let ext_dir = dir.path().join("extensions");
        std::fs::create_dir(&ext_dir).unwrap();
        let container = ext_dir.join("container");
        let nested = container.join("nested");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join("index.ts"), "// too deep").unwrap();

        let paths = discover_extensions_in_dir(&ext_dir.to_string_lossy());
        assert!(paths.is_empty());
    }

    #[test]
    fn test_discover_mixed_direct_files_and_subdirectories() {
        let dir = tempfile::tempdir().unwrap();
        let ext_dir = dir.path().join("extensions");
        std::fs::create_dir(&ext_dir).unwrap();

        // Direct file
        std::fs::write(ext_dir.join("direct.ts"), "// test").unwrap();

        // Subdirectory with index
        let sub1 = ext_dir.join("with-index");
        std::fs::create_dir(&sub1).unwrap();
        std::fs::write(sub1.join("index.ts"), "// test").unwrap();

        // Subdirectory with package.json
        let sub2 = ext_dir.join("with-manifest");
        std::fs::create_dir(&sub2).unwrap();
        std::fs::write(sub2.join("entry.ts"), "// test").unwrap();
        std::fs::write(
            sub2.join("package.json"),
            r#"{"name":"pkg","pi":{"extensions":["./entry.ts"]}}"#,
        )
        .unwrap();

        let paths = discover_extensions_in_dir(&ext_dir.to_string_lossy());
        assert_eq!(paths.len(), 3);
    }

    #[test]
    fn test_discover_skips_nonexistent_paths_in_package_json() {
        let dir = tempfile::tempdir().unwrap();
        let ext_dir = dir.path().join("extensions");
        std::fs::create_dir(&ext_dir).unwrap();
        let subdir = ext_dir.join("my-package");
        std::fs::create_dir(&subdir).unwrap();
        std::fs::write(subdir.join("exists.ts"), "// test").unwrap();
        std::fs::write(
            subdir.join("package.json"),
            r#"{"name":"pkg","pi":{"extensions":["./exists.ts","./missing.ts"]}}"#,
        )
        .unwrap();

        let paths = discover_extensions_in_dir(&ext_dir.to_string_lossy());
        assert_eq!(paths.len(), 1);
        assert!(paths[0].ends_with("exists.ts"));
        assert!(!paths[0].ends_with("missing.ts"));
    }

    // -----------------------------------------------------------------------
    // Resolve extension entries
    // -----------------------------------------------------------------------

    #[test]
    fn test_resolve_extension_entries_with_package_json() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"pi":{"extensions":["./dist/main.ts"]}}"#,
        )
        .unwrap();
        let dist = dir.path().join("dist");
        std::fs::create_dir(&dist).unwrap();
        std::fs::write(dist.join("main.ts"), "// test").unwrap();

        let entries = resolve_extension_entries(&dir.path().to_string_lossy());
        assert!(entries.is_some());
        let entries = entries.unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].ends_with("dist/main.ts"));
    }

    #[test]
    fn test_resolve_extension_entries_with_index_js() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("index.js"), "// test").unwrap();

        let entries = resolve_extension_entries(&dir.path().to_string_lossy());
        assert!(entries.is_some());
        assert_eq!(entries.unwrap().len(), 1);
    }

    #[test]
    fn test_resolve_extension_entries_no_index_or_package_json() {
        let dir = tempfile::tempdir().unwrap();
        // Empty directory
        let entries = resolve_extension_entries(&dir.path().to_string_lossy());
        assert!(entries.is_none());
    }

    // -----------------------------------------------------------------------
    // Read pi manifest
    // -----------------------------------------------------------------------

    #[test]
    fn test_read_pi_manifest_with_hamr_key() {
        let dir = tempfile::tempdir().unwrap();
        let pkg_path = dir.path().join("package.json");
        std::fs::write(&pkg_path, r#"{"hamr": {"extensions": ["./ext.ts"]}}"#).unwrap();

        let manifest = read_pi_manifest(&pkg_path.to_string_lossy());
        assert!(manifest.is_some());
        assert_eq!(manifest.unwrap().extensions[0], "./ext.ts");
    }

    #[test]
    fn test_read_pi_manifest_empty_extensions() {
        let dir = tempfile::tempdir().unwrap();
        let pkg_path = dir.path().join("package.json");
        std::fs::write(&pkg_path, r#"{"pi": {"extensions": []}}"#).unwrap();

        let manifest = read_pi_manifest(&pkg_path.to_string_lossy());
        assert!(manifest.is_none());
    }

    #[test]
    fn test_read_pi_manifest_missing_file_returns_none() {
        let manifest = read_pi_manifest("/nonexistent/package.json");
        assert!(manifest.is_none());
    }

    #[test]
    fn test_read_pi_manifest_invalid_json_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let pkg_path = dir.path().join("package.json");
        std::fs::write(&pkg_path, "not valid json").unwrap();

        let manifest = read_pi_manifest(&pkg_path.to_string_lossy());
        assert!(manifest.is_none());
    }

    #[test]
    fn test_read_pi_manifest_missing_pi_section_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let pkg_path = dir.path().join("package.json");
        std::fs::write(&pkg_path, r#"{"name": "foo", "version": "1.0.0"}"#).unwrap();

        let manifest = read_pi_manifest(&pkg_path.to_string_lossy());
        assert!(manifest.is_none());
    }

    #[test]
    fn test_read_pi_manifest_pi_not_object_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let pkg_path = dir.path().join("package.json");
        std::fs::write(&pkg_path, r#"{"pi": "string-value"}"#).unwrap();

        let manifest = read_pi_manifest(&pkg_path.to_string_lossy());
        assert!(manifest.is_none());
    }

    // -----------------------------------------------------------------------
    // JS-extension-specific tests (require Node.js import() — cannot test in Rust)
    // -----------------------------------------------------------------------

    #[ignore = "requires Node.js extension loader (import() + jiti)"]
    #[test]
    fn test_discover_loads_extensions_with_commands() {
        // TS extensions-discovery.test.ts: loads extension that registers commands
    }

    #[ignore = "requires Node.js extension loader (import() + jiti)"]
    #[test]
    fn test_discover_loads_extensions_with_tools() {
        // TS extensions-discovery.test.ts: loads extension that registers tools
    }

    #[ignore = "requires Node.js extension loader (import() + jiti)"]
    #[test]
    fn test_discover_reports_invalid_extension_code() {
        // TS extensions-discovery.test.ts: invalid extension code causes error
    }

    #[ignore = "requires Node.js extension loader (import() + jiti)"]
    #[test]
    fn test_discover_reports_extension_throw_during_init() {
        // TS extensions-discovery.test.ts: extension that throws during init
    }

    #[ignore = "requires Node.js extension loader (import() + jiti)"]
    #[test]
    fn test_discover_reports_missing_default_export() {
        // TS extensions-discovery.test.ts: no default export = error
    }

    #[ignore = "requires Node.js extension loader (import() + jiti)"]
    #[test]
    fn test_discover_loads_extensions_with_handlers() {
        // TS extensions-discovery.test.ts: loads extension with event handlers
    }

    #[ignore = "requires Node.js extension loader (import() + jiti)"]
    #[test]
    fn test_discover_loads_extensions_with_shortcuts() {
        // TS extensions-discovery.test.ts: loads extension with shortcuts
    }

    #[ignore = "requires Node.js extension loader (import() + jiti)"]
    #[test]
    fn test_discover_loads_extensions_with_flags() {
        // TS extensions-discovery.test.ts: loads extension with flags
    }

    #[ignore = "requires Node.js extension loader (import() + jiti)"]
    #[test]
    fn test_discover_loads_extensions_with_renderers() {
        // TS extensions-discovery.test.ts: loads extension with message renderers
    }

    #[ignore = "requires Node.js extension loader (import() + jiti)"]
    #[test]
    fn test_discover_handles_explicit_paths() {
        // TS extensions-discovery.test.ts: loads from explicitly configured paths
    }

    #[ignore = "requires Node.js extension loader (import() + jiti)"]
    #[test]
    fn test_discover_resolves_from_own_node_modules() {
        // TS extensions-discovery.test.ts: extension with its own node_modules
    }

    #[ignore = "requires Node.js extension loader (import() + jiti)"]
    #[test]
    fn test_discover_load_extensions_directly_without_discovery() {
        // TS extensions-discovery.test.ts: loadExtensions only loads explicit paths
    }

    #[ignore = "requires Node.js extension loader (import() + jiti)"]
    #[test]
    fn test_discover_load_extensions_empty_paths() {
        // TS extensions-discovery.test.ts: loadExtensions with empty paths
    }
}
