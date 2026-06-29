//! verify-pack: Port of `scripts/verify-pack.sh` — smoke-test verification of the built
//! `@skaft/hamr` tarball.
//!
//! Installs the given tarball into a throwaway project with `npm install`, then checks:
//! - The `hamr` CLI binary is installed
//! - Bundled `@hamr/tui`, `@hamr/ai`, `@hamr/agent` are present
//! - `hamr --version` and `hamr --help` succeed
//! - No leaked source/config files in the package
//! - Protobufjs is bundled and has its postinstall stripped
//! - Global-prefix install works (`npm install -g --prefix`)
//!
//! The [`verify_pack`] function is the main entry point.  It returns a
//! [`VerifyResult`] summarising passes, failures, and per-check detail.

use crate::{Error, Result};
use std::io::Read;
use std::path::{Path, PathBuf};
use tokio::process::Command;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// The outcome of running the verification suite.
#[derive(Debug, Clone)]
pub struct VerifyResult {
    /// How many individual checks passed.
    pub passed: usize,
    /// How many individual checks failed.
    pub failed: usize,
    /// Each check, in the order it was run.
    pub checks: Vec<CheckResult>,
}

/// A single check result.
#[derive(Debug, Clone)]
pub struct CheckResult {
    /// Human-readable name for the check (e.g. `"hamr --version"`).
    pub name: String,
    /// Whether the check passed.
    pub passed: bool,
    /// Extra detail (stdout on success, error message on failure).
    pub detail: Option<String>,
}

/// Errors returned when a helper function fails fatally (not a verification
/// failure, but something that prevents us from proceeding).
fn fatal(msg: impl Into<String>) -> Error {
    Error::VerificationFailed(msg.into())
}

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

/// Run the full tarball verification suite against `tarball`.
///
/// Steps:
/// 1. Extract the tarball into a temp directory.
/// 2. `npm install` the tarball into a fresh temp project.
/// 3. Run all inventory / CLI / leak / protobufjs / global-prefix checks.
///
/// Returns a [`VerifyResult`] even when some checks fail — the caller decides
/// how to interpret the counts.
pub async fn verify_pack(tarball: &Path) -> Result<VerifyResult> {
    let tarball = tarball
        .canonicalize()
        .map_err(|e| Error::Io(e))?;

    let mut checks: Vec<CheckResult> = Vec::new();

    // -- 1. Create temp project & install the tarball -----------------------
    let tmp_dir = tempfile::tempdir().map_err(Error::Io)?;

    // Write a minimal package.json so `npm install <tarball>` accepts it.
    let pkg_json = tmp_dir.path().join("package.json");
    std::fs::write(&pkg_json, r#"{"name":"verify-pack-tmp","private":true}"#).map_err(Error::Io)?;

    let install_log = tmp_dir.path().join("install.log");

    // npm install --no-audit --no-fund --loglevel=error <tarball>
    let install_result = npm_install(tmp_dir.path(), &tarball, &install_log).await;
    if install_result.success {
        checks.push(CheckResult {
            name: "npm install of tarball succeeded".into(),
            passed: true,
            detail: None,
        });
    } else {
        let log_text = std::fs::read_to_string(&install_log)
            .unwrap_or_else(|_| "<could not read install log>".into());
        checks.push(CheckResult {
            name: "npm install of tarball succeeded".into(),
            passed: false,
            detail: Some(log_text),
        });
        // Cannot continue without a successful install.
        return Ok(VerifyResult {
            passed: checks.iter().filter(|c| c.passed).count(),
            failed: checks.iter().filter(|c| !c.passed).count(),
            checks,
        });
    }

    let pkg_dir = tmp_dir.path().join("node_modules/@skaft/hamr");
    let bin_path = tmp_dir.path().join("node_modules/.bin/hamr");

    // -- 2. hamr CLI binary exists ------------------------------------------
    if bin_path.is_file() || bin_path.is_symlink() {
        checks.push(CheckResult {
            name: "hamr CLI binary installed".into(),
            passed: true,
            detail: None,
        });
    } else {
        checks.push(CheckResult {
            name: "hamr CLI binary installed".into(),
            passed: false,
            detail: Some(format!("missing at {}", bin_path.display())),
        });
    }

    // -- 3. Bundled @hamr/* libs --------------------------------------------
    for lib_name in ["tui", "ai", "agent"] {
        let lib_index = pkg_dir
            .join("node_modules")
            .join("@hamr")
            .join(lib_name)
            .join("dist")
            .join("index.js");
        let check_name = format!("bundled @hamr/{} present", lib_name);
        if lib_index.is_file() {
            checks.push(CheckResult { name: check_name, passed: true, detail: None });
        } else {
            checks.push(CheckResult {
                name: check_name,
                passed: false,
                detail: Some(format!("missing {}", lib_index.display())),
            });
        }
    }

    // -- 4. hamr --version --------------------------------------------------
    let ver_out = run_hamr_cmd(&bin_path, &["--version"]).await;
    match ver_out {
        Ok(ver) => checks.push(CheckResult {
            name: "hamr --version".into(),
            passed: true,
            detail: Some(ver.trim().to_string()),
        }),
        Err(e_msg) => checks.push(CheckResult {
            name: "hamr --version".into(),
            passed: false,
            detail: Some(e_msg),
        }),
    }

    // -- 5. hamr --help -----------------------------------------------------
    let help_result = run_hamr_cmd(&bin_path, &["--help"]).await;
    match help_result {
        Ok(_) => checks.push(CheckResult {
            name: "hamr --help".into(),
            passed: true,
            detail: None,
        }),
        Err(e_msg) => checks.push(CheckResult {
            name: "hamr --help".into(),
            passed: false,
            detail: Some(e_msg),
        }),
    }

    // -- 6. Leaked files ----------------------------------------------------
    match check_no_leaked_files(&pkg_dir) {
        Ok(leaked) => {
            for leak_name in ["src", "specs", "scripts", ".hamr.toml", "npm-shrinkwrap.json"] {
                let check_name = format!("no '{}' in package", leak_name);
                if leaked.iter().any(|l| *l == leak_name) {
                    checks.push(CheckResult {
                        name: check_name,
                        passed: false,
                        detail: Some("leaked".into()),
                    });
                } else {
                    checks.push(CheckResult { name: check_name, passed: true, detail: None });
                }
            }
        }
        Err(e) => {
            checks.push(CheckResult {
                name: "checking leaked files".into(),
                passed: false,
                detail: Some(e.to_string()),
            });
        }
    }

    // -- 7. Protobufjs bundle check (postinstall workaround) ---------------
    // a. Check protobufjs/package.json exists in tarball entries
    let tar_entries = check_tarball_contents(&tarball)?;
    let proto_pkg_entry = tar_entries
        .iter()
        .find(|e| e.contains("node_modules/protobufjs/package.json"));
    if proto_pkg_entry.is_some() {
        checks.push(CheckResult {
            name: "protobufjs bundled in tarball".into(),
            passed: true,
            detail: None,
        });
    } else {
        checks.push(CheckResult {
            name: "protobufjs bundled in tarball".into(),
            passed: false,
            detail: Some("NOT found in tarball — postinstall workaround missing".into()),
        });
    }

    // b. Check the bundled protobufjs has no postinstall script
    let proto_pkg_content = read_file_from_tarball(
        &tarball,
        "package/node_modules/protobufjs/package.json",
    );
    match proto_pkg_content {
        Ok(contents) => {
            let has_postinstall = check_protobufjs_postinstall(&contents);
            if has_postinstall {
                checks.push(CheckResult {
                    name: "protobufjs bundled copy has no postinstall".into(),
                    passed: false,
                    detail: Some("postinstall script still present".into()),
                });
            } else {
                checks.push(CheckResult {
                    name: "protobufjs bundled copy has no postinstall".into(),
                    passed: true,
                    detail: None,
                });
            }
        }
        Err(_) => {
            // maybe the tarball layout doesn't have the `package/` prefix —
            // try without it
            let alt = read_file_from_tarball(
                &tarball,
                "node_modules/protobufjs/package.json",
            );
            match alt {
                Ok(contents) => {
                    let has_postinstall = check_protobufjs_postinstall(&contents);
                    if has_postinstall {
                        checks.push(CheckResult {
                            name: "protobufjs bundled copy has no postinstall".into(),
                            passed: false,
                            detail: Some("postinstall script still present".into()),
                        });
                    } else {
                        checks.push(CheckResult {
                            name: "protobufjs bundled copy has no postinstall".into(),
                            passed: true,
                            detail: None,
                        });
                    }
                }
                Err(_) => {
                    checks.push(CheckResult {
                        name: "protobufjs bundled copy has no postinstall".into(),
                        passed: false,
                        detail: Some("could not read protobufjs package.json from tarball".into()),
                    });
                }
            }
        }
    }

    // -- 8. Global-prefix install ------------------------------------------
    let global_prefix = tmp_dir.path().join("global-prefix");
    std::fs::create_dir_all(&global_prefix).map_err(Error::Io)?;

    let global_install_log = tmp_dir.path().join("global-install.log");
    let global_install = npm_install_global_prefix(&global_prefix, &tarball, &global_install_log).await;

    if global_install.success {
        checks.push(CheckResult {
            name: "npm install -g --prefix succeeded".into(),
            passed: true,
            detail: None,
        });

        let global_bin = find_global_bin(&global_prefix, "hamr");
        if let Some(gbin) = global_bin {
            let gver = run_hamr_cmd(&gbin, &["--version"]).await;
            match gver {
                Ok(ver) => checks.push(CheckResult {
                    name: "global hamr --version".into(),
                    passed: true,
                    detail: Some(ver.trim().to_string()),
                }),
                Err(e_msg) => checks.push(CheckResult {
                    name: "global hamr --version".into(),
                    passed: false,
                    detail: Some(e_msg),
                }),
            }
        } else {
            checks.push(CheckResult {
                name: "global hamr --version".into(),
                passed: false,
                detail: Some(format!(
                    "global hamr binary not found under {}",
                    global_prefix.display()
                )),
            });
        }
    } else {
        let log_text = std::fs::read_to_string(&global_install_log)
            .unwrap_or_else(|_| "<could not read install log>".into());
        checks.push(CheckResult {
            name: "npm install -g --prefix succeeded".into(),
            passed: false,
            detail: Some(log_text),
        });
    }

    let passed = checks.iter().filter(|c| c.passed).count();
    let failed = checks.len() - passed;

    Ok(VerifyResult { passed, failed, checks })
}

// ---------------------------------------------------------------------------
// Tarball helpers
// ---------------------------------------------------------------------------

/// List all entry paths inside a `.tgz` / `.tar.gz` tarball.
///
/// Handles gzip-compressed tarballs via `flate2` + `tar`.
pub fn check_tarball_contents(tarball: &Path) -> Result<Vec<String>> {
    let file = std::fs::File::open(tarball).map_err(Error::Io)?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);

    let mut entries: Vec<String> = Vec::new();

    for entry in archive.entries().map_err(|e| fatal(format!("reading tar entries: {e}")))? {
        let entry = entry.map_err(|e| fatal(format!("reading tar entry: {e}")))?;
        let path = entry
            .path()
            .map_err(|e| fatal(format!("reading entry path: {e}")))?;
        entries.push(path.to_string_lossy().into_owned());
    }

    Ok(entries)
}

/// Read a single file's contents from inside a `.tgz` / `.tar.gz` tarball.
///
/// `entry_path` is the exact path inside the tarball, e.g.
/// `"package/node_modules/protobufjs/package.json"`.  The leading `package/`
/// is the npm-pack root directory; it may or may not be present depending on
/// how the tarball was created.
pub fn read_file_from_tarball(tarball: &Path, entry_path: &str) -> Result<String> {
    let file = std::fs::File::open(tarball).map_err(Error::Io)?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);

    for entry in archive.entries().map_err(|e| fatal(format!("reading tar entries: {e}")))? {
        let mut entry = entry.map_err(|e| fatal(format!("reading tar entry: {e}")))?;
        let path = entry
            .path()
            .map_err(|e| fatal(format!("reading entry path: {e}")))?;
        if path.to_string_lossy() == entry_path {
            let mut data = String::new();
            entry
                .read_to_string(&mut data)
                .map_err(|e| fatal(format!("reading entry data: {e}")))?;
            return Ok(data);
        }
    }

    Err(fatal(format!(
        "entry '{}' not found in tarball {}",
        entry_path,
        tarball.display()
    )))
}

// ---------------------------------------------------------------------------
// Package-directory checks
// ---------------------------------------------------------------------------

/// Check for leaked sensitive files in the installed package directory.
///
/// Returns the list of leaked files that were found (empty = good).
/// Checks: `src/`, `specs/`, `scripts/`, `.hamr.toml`, `npm-shrinkwrap.json`.
pub fn check_no_leaked_files(pkg_dir: &Path) -> Result<Vec<&'static str>> {
    let sensitive: &[&str] = &["src", "specs", "scripts", ".hamr.toml", "npm-shrinkwrap.json"];
    let mut leaked: Vec<&'static str> = Vec::new();

    for name in sensitive {
        let path = pkg_dir.join(name);
        if path.exists() {
            leaked.push(name);
        }
    }

    Ok(leaked)
}

/// Check that the required `@hamr/*` bundled libraries are present.
///
/// Looks for `node_modules/@hamr/<lib>/dist/index.js` for each of `tui`, `ai`,
/// `agent`.  Returns the list of library names that are **missing**.
pub fn check_bundled_libs(pkg_dir: &Path) -> Result<Vec<String>> {
    let required: &[&str] = &["tui", "ai", "agent"];
    let mut missing: Vec<String> = Vec::new();

    for &lib in required {
        let idx = pkg_dir
            .join("node_modules")
            .join("@hamr")
            .join(lib)
            .join("dist")
            .join("index.js");
        if !idx.is_file() {
            missing.push(lib.to_string());
        }
    }

    Ok(missing)
}

// ---------------------------------------------------------------------------
// Protobufjs postinstall check
// ---------------------------------------------------------------------------

/// Parse the `package.json` content and return `true` if a `postinstall`
/// script is defined in `scripts`.
pub fn check_protobufjs_postinstall(package_json_contents: &str) -> bool {
    let parsed: serde_json::Value = match serde_json::from_str(package_json_contents) {
        Ok(v) => v,
        Err(_) => return false, // if we can't parse, treat as no postinstall
    };

    parsed
        .get("scripts")
        .and_then(|scripts| scripts.get("postinstall"))
        .and_then(|v| v.as_str())
        .is_some()
}

// ---------------------------------------------------------------------------
// npm / node helpers (async — tokio::process::Command)
// ---------------------------------------------------------------------------

struct CmdResult {
    success: bool,
}

/// Run `npm install --no-audit --no-fund --loglevel=error <tarball>` inside
/// `cwd`, capturing stdout+stderr into `log_path`.
async fn npm_install(cwd: &Path, tarball: &Path, log_path: &Path) -> CmdResult {
    let output = Command::new("npm")
        .args([
            "install",
            "--no-audit",
            "--no-fund",
            "--loglevel=error",
        ])
        .arg(tarball.as_os_str())
        .current_dir(cwd)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .await;

    write_command_log(log_path, &output);

    CmdResult {
        success: output.as_ref().map(|o| o.status.success()).unwrap_or(false),
    }
}

/// Run `npm install -g --prefix <prefix> --no-audit --no-fund --loglevel=error <tarball>`.
async fn npm_install_global_prefix(
    prefix: &Path,
    tarball: &Path,
    log_path: &Path,
) -> CmdResult {
    let output = Command::new("npm")
        .args([
            "install",
            "-g",
            "--prefix",
        ])
        .arg(prefix.as_os_str())
        .args([
            "--no-audit",
            "--no-fund",
            "--loglevel=error",
        ])
        .arg(tarball.as_os_str())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .await;

    write_command_log(log_path, &output);

    CmdResult {
        success: output.as_ref().map(|o| o.status.success()).unwrap_or(false),
    }
}

/// Run the hamr binary with the given arguments.  Returns `Ok(stdout)` on
/// success (exit 0) or `Err(combined stderr+stdout)` on failure.
async fn run_hamr_cmd(bin: &Path, args: &[&str]) -> std::result::Result<String, String> {
    let output = Command::new(bin)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .await
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        Ok(stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        Err(format!("exit={}\nstdout:\n{stdout}\nstderr:\n{stderr}", output.status))
    }
}

/// Write stdout+stderr of a command to a log file.
fn write_command_log(log_path: &Path, output: &std::result::Result<std::process::Output, std::io::Error>) {
    match output {
        Ok(o) => {
            let mut log = Vec::new();
            log.extend_from_slice(&o.stdout);
            log.extend_from_slice(&o.stderr);
            let _ = std::fs::write(log_path, &log);
        }
        Err(_) => {
            let _ = std::fs::write(log_path, b"<command failed to start>");
        }
    }
}

/// Locate a global binary installed via `npm install -g --prefix <dir>`.
///
/// npm may place the binary under `<dir>/bin/<name>` on Unix, or
/// `<dir>/<name>.cmd` on Windows.
fn find_global_bin(prefix: &Path, name: &str) -> Option<PathBuf> {
    // npm with --prefix creates the standard layout: .../bin/<name>
    let unix_bin = prefix.join("bin").join(name);
    if unix_bin.is_file() || unix_bin.is_symlink() {
        return Some(unix_bin);
    }

    // Windows: <prefix>/<name>.cmd
    let win_bin = prefix.join(format!("{name}.cmd"));
    if win_bin.is_file() {
        return Some(win_bin);
    }

    None
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- VerifyResult tests ---

    #[test]
    fn verify_result_tracks_passes_and_failures() {
        let checks = vec![
            CheckResult { name: "a".into(), passed: true, detail: None },
            CheckResult { name: "b".into(), passed: false, detail: Some("oops".into()) },
            CheckResult { name: "c".into(), passed: true, detail: Some("ok".into()) },
        ];
        let result = VerifyResult { passed: 2, failed: 1, checks };
        assert_eq!(result.passed, 2);
        assert_eq!(result.failed, 1);
        assert_eq!(result.checks.len(), 3);
        assert!(result.checks[0].passed);
        assert!(!result.checks[1].passed);
        assert!(result.checks[2].passed);
        assert_eq!(result.checks[1].detail.as_deref(), Some("oops"));
    }

    // --- check_no_leaked_files tests ---

    #[test]
    fn check_no_leaked_files_clean_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let leaked = check_no_leaked_files(tmp.path()).unwrap();
        assert!(leaked.is_empty());
    }

    #[test]
    fn check_no_leaked_files_detects_src_and_hamr_toml() {
        let tmp = tempfile::tempdir().unwrap();
        // Create leaked items
        std::fs::create_dir(tmp.path().join("src")).unwrap();
        std::fs::write(tmp.path().join(".hamr.toml"), b"fake config").unwrap();
        // Also create a script dir
        std::fs::create_dir(tmp.path().join("scripts")).unwrap();

        let mut leaked = check_no_leaked_files(tmp.path()).unwrap();
        leaked.sort(); // deterministic order for assertion
        assert_eq!(leaked, vec![".hamr.toml", "scripts", "src"]);
    }

    #[test]
    fn check_no_leaked_files_detects_all_sensitive_names() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir(tmp.path().join("src")).unwrap();
        std::fs::create_dir(tmp.path().join("specs")).unwrap();
        std::fs::create_dir(tmp.path().join("scripts")).unwrap();
        std::fs::write(tmp.path().join(".hamr.toml"), b"").unwrap();
        std::fs::write(tmp.path().join("npm-shrinkwrap.json"), b"").unwrap();

        let mut leaked = check_no_leaked_files(tmp.path()).unwrap();
        leaked.sort();
        assert_eq!(
            leaked,
            vec![".hamr.toml", "npm-shrinkwrap.json", "scripts", "specs", "src"]
        );
    }

    // --- check_bundled_libs tests ---

    #[test]
    fn check_bundled_libs_all_present() {
        let tmp = tempfile::tempdir().unwrap();
        // Create mock @hamr/{tui,ai,agent}/dist/index.js structure
        for lib in ["tui", "ai", "agent"] {
            let dir = tmp.path()
                .join("node_modules/@hamr")
                .join(lib)
                .join("dist");
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("index.js"), b"// mock").unwrap();
        }

        let missing = check_bundled_libs(tmp.path()).unwrap();
        assert!(missing.is_empty());
    }

    #[test]
    fn check_bundled_libs_detects_missing() {
        let tmp = tempfile::tempdir().unwrap();
        // Only create tui, leave ai and agent missing
        let dir = tmp.path()
            .join("node_modules/@hamr/tui/dist");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("index.js"), b"// mock").unwrap();

        let mut missing = check_bundled_libs(tmp.path()).unwrap();
        missing.sort();
        assert_eq!(missing, vec!["agent", "ai"]);
    }

    #[test]
    fn check_bundled_libs_detects_missing_indexjs() {
        let tmp = tempfile::tempdir().unwrap();
        // Create the dir but not the index.js
        let dir = tmp.path()
            .join("node_modules/@hamr/ai/dist");
        std::fs::create_dir_all(&dir).unwrap();
        // no index.js written

        let missing = check_bundled_libs(tmp.path()).unwrap();
        assert!(missing.contains(&"ai".to_string()));
    }

    // --- Tarball tests (mock tarball) ---

    /// Helper: create a minimal .tar.gz on disk for testing.
    /// Returns `(TempDir, PathBuf)` — the caller must keep `_tmp` alive while using `tarball_path`.
    fn create_mock_tarball(entries: &[(&str, &str)]) -> (tempfile::TempDir, PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let tarball_path = tmp.path().join("test.tar.gz");

        let file = std::fs::File::create(&tarball_path).unwrap();
        let gz = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut tar = tar::Builder::new(gz);

        for (path, contents) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_path(path).unwrap();
            header.set_size(contents.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            tar.append(&header, contents.as_bytes()).unwrap();
        }

        tar.into_inner().unwrap().finish().unwrap();

        (tmp, tarball_path)
    }

    #[test]
    fn check_tarball_contents_lists_entries() {
        let (_tmp, tarball) = create_mock_tarball(&[
            ("package/package.json", r#"{"name":"skaft-hamr"}"#),
            ("package/node_modules/protobufjs/package.json", r#"{"name":"protobufjs","scripts":{"postinstall":"echo hi"}}"#),
            ("package/dist/index.js", "// main"),
        ]);

        let entries = check_tarball_contents(&tarball).unwrap();
        assert_eq!(entries.len(), 3);
        assert!(entries.contains(&"package/package.json".to_string()));
        assert!(entries.contains(&"package/node_modules/protobufjs/package.json".to_string()));
        assert!(entries.contains(&"package/dist/index.js".to_string()));
    }

    #[test]
    fn read_file_from_tarball_extracts_entry() {
        let (_tmp, tarball) = create_mock_tarball(&[
            ("package/package.json", r#"{"name":"test-pkg","version":"1.0.0"}"#),
            ("package/hello.txt", "hello world"),
        ]);

        let pkg_json = read_file_from_tarball(&tarball, "package/package.json").unwrap();
        assert!(pkg_json.contains("\"name\":\"test-pkg\""));
        assert!(pkg_json.contains("\"version\":\"1.0.0\""));

        let hello = read_file_from_tarball(&tarball, "package/hello.txt").unwrap();
        assert_eq!(hello, "hello world");
    }

    #[test]
    fn read_file_from_tarball_errors_on_missing() {
        let (_tmp, tarball) = create_mock_tarball(&[
            ("package/package.json", r#"{}"#),
        ]);

        let result = read_file_from_tarball(&tarball, "nonexistent.txt");
        assert!(result.is_err());
    }

    // --- Protobufjs postinstall detection ---

    #[test]
    fn check_protobufjs_postinstall_detects_present() {
        let contents = r#"{"name":"protobufjs","scripts":{"postinstall":"echo bad","test":"mocha"}}"#;
        assert!(check_protobufjs_postinstall(contents));
    }

    #[test]
    fn check_protobufjs_postinstall_detects_absent() {
        let contents = r#"{"name":"protobufjs","scripts":{"test":"mocha"}}"#;
        assert!(!check_protobufjs_postinstall(contents));
    }

    #[test]
    fn check_protobufjs_postinstall_no_scripts_field() {
        let contents = r#"{"name":"protobufjs"}"#;
        assert!(!check_protobufjs_postinstall(contents));
    }

    #[test]
    fn check_protobufjs_postinstall_broken_json() {
        let contents = r#"not valid json"#;
        // Should not panic — returns false
        assert!(!check_protobufjs_postinstall(contents));
    }
}
