//! Port of `packages/coding-agent/src/core/trust_manager.ts`.
//!
//! Project trust store — persisted per-directory trust decisions with
//! parent-directory inheritance and file-based locking.

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A trust decision: true (trusted), false (not trusted), null (no decision).
pub type ProjectTrustDecision = Option<bool>;

/// An entry in the trust store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectTrustStoreEntry {
    pub path: String,
    pub decision: bool,
}

/// An update to apply to the trust store.
#[derive(Debug, Clone)]
pub struct ProjectTrustUpdate {
    pub path: String,
    pub decision: ProjectTrustDecision,
}

/// An option presented to the user in the trust prompt.
#[derive(Debug, Clone)]
pub struct ProjectTrustOption {
    pub label: String,
    pub trusted: bool,
    pub updates: Vec<ProjectTrustUpdate>,
    pub saved_path: Option<String>,
}

/// The trust file is a JSON object mapping canonical paths to decisions.
type TrustFile = BTreeMap<String, Option<bool>>;

// Resources that trigger the trust prompt when present in a project's .hamr dir.
const TRUST_REQUIRING_PROJECT_CONFIG_RESOURCES: &[&str] = &[
    "settings.json",
    "extensions",
    "skills",
    "prompts",
    "themes",
    "SYSTEM.md",
    "APPEND_SYSTEM.md",
];

// ---------------------------------------------------------------------------
// Path utilities
// ---------------------------------------------------------------------------

/// Canonicalize a path (resolve symlinks). Falls back to the raw path.
fn canonicalize_path(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Resolve a path (relative → absolute).
fn resolve_path(path: &str) -> PathBuf {
    let p = Path::new(path);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("/"))
            .join(p)
    }
}

/// Normalize cwd: resolve + canonicalize.
fn normalize_cwd(cwd: &str) -> PathBuf {
    canonicalize_path(&resolve_path(cwd))
}

// ---------------------------------------------------------------------------
// Trust file I/O
// ---------------------------------------------------------------------------

/// Read a trust.json file. Returns empty map if the file doesn't exist.
fn read_trust_file(path: &Path) -> Result<TrustFile, String> {
    if !path.exists() {
        return Ok(BTreeMap::new());
    }

    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read trust store {}: {}", path.display(), e))?;

    let parsed: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to read trust store {}: {}", path.display(), e))?;

    let obj = parsed
        .as_object()
        .ok_or_else(|| format!("Invalid trust store {}: expected an object", path.display()))?;

    let mut data: TrustFile = BTreeMap::new();
    for (key, value) in obj {
        match value {
            serde_json::Value::Bool(b) => {
                data.insert(key.clone(), Some(*b));
            }
            serde_json::Value::Null => {
                data.insert(key.clone(), None);
            }
            _ => {
                return Err(format!(
                    "Invalid trust store {}: value for {} must be true, false, or null",
                    path.display(),
                    serde_json::to_string(key).unwrap_or_else(|_| key.clone())
                ));
            }
        }
    }
    Ok(data)
}

/// Write a trust.json file. Entries are sorted by key.
fn write_trust_file(path: &Path, data: &TrustFile) -> io::Result<()> {
    // Build sorted map, filtering to valid values
    let mut sorted = serde_json::Map::new();
    for (key, value) in data {
        let json_val = match value {
            Some(true) => serde_json::Value::Bool(true),
            Some(false) => serde_json::Value::Bool(false),
            None => serde_json::Value::Null,
        };
        sorted.insert(key.clone(), json_val);
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let json_str = serde_json::to_string_pretty(&serde_json::Value::Object(sorted))?;
    let mut file = fs::File::create(path)?;
    file.write_all(json_str.as_bytes())?;
    file.write_all(b"\n")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Locking
// ---------------------------------------------------------------------------

/// Acquire an exclusive filesystem lock on the trust directory using flock.
/// Returns a guard that releases the lock on drop.
struct TrustLockGuard {
    _file: fs::File,
}

fn acquire_trust_lock_sync(path: &Path) -> Result<TrustLockGuard, String> {
    let trust_dir = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(trust_dir).map_err(|e| format!("Failed to create trust dir: {}", e))?;

    let lock_path = path.with_extension("json.lock");

    let max_attempts = 25;
    let base_delay_ms: u64 = 20;
    let max_delay_ms: u64 = 2000;

    for attempt in 1..=max_attempts {
        match fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(&lock_path)
        {
            Ok(file) => {
                // Try to acquire an exclusive lock
                #[cfg(unix)]
                {
                    use std::os::unix::io::AsRawFd;
                    let fd = file.as_raw_fd();
                    let rc = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
                    if rc == 0 {
                        return Ok(TrustLockGuard { _file: file });
                    }
                    let err = io::Error::last_os_error();
                    if err.raw_os_error() != Some(libc::EWOULDBLOCK) && attempt == max_attempts {
                        return Err(format!("Failed to acquire lock: {}", err));
                    }
                }
                #[cfg(not(unix))]
                {
                    // On non-unix, just proceed without real locking
                    return Ok(TrustLockGuard { _file: file });
                }
            }
            Err(e) => {
                if attempt == max_attempts {
                    return Err(format!("Failed to create lock file: {}", e));
                }
            }
        }

        // Exponential backoff with max cap
        let delay_ms = std::cmp::min(base_delay_ms * 2u64.pow(attempt - 1), max_delay_ms);
        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
    }

    Err("Failed to acquire trust store lock".to_string())
}

fn with_trust_file_lock<T, F: FnOnce() -> T>(path: &Path, f: F) -> T {
    let _guard = acquire_trust_lock_sync(path).ok();
    f()
}

// ---------------------------------------------------------------------------
// Trust lookup
// ---------------------------------------------------------------------------

/// Walk up from cwd to find the nearest trust entry in the trust file.
fn find_nearest_trust_entry(data: &TrustFile, cwd: &Path) -> Option<ProjectTrustStoreEntry> {
    let mut current_dir = cwd.to_path_buf();

    loop {
        let current_str = current_dir.to_string_lossy().to_string();
        if let Some(decision) = data.get(&current_str) {
            match decision {
                Some(true) | Some(false) => {
                    return Some(ProjectTrustStoreEntry {
                        path: current_str,
                        decision: decision.unwrap(),
                    });
                }
                None => {
                    // null entry — skip, continue walking up
                }
            }
        }

        let parent_dir = match current_dir.parent() {
            Some(p) => p.to_path_buf(),
            None => return None,
        };

        if parent_dir == current_dir {
            return None;
        }
        current_dir = parent_dir;
    }
}

/// Get the parent path of a trust entry, or None if at root.
pub fn get_project_trust_parent_path(cwd: &str) -> Option<String> {
    let trust_path = normalize_cwd(cwd);
    let parent_dir = trust_path.parent()?;
    let parent_str = parent_dir.to_string_lossy().to_string();
    let trust_str = trust_path.to_string_lossy().to_string();
    if parent_str == trust_str {
        None
    } else {
        Some(parent_str)
    }
}

// ---------------------------------------------------------------------------
// Trust options
// ---------------------------------------------------------------------------

/// Options for generating trust options.
pub struct ProjectTrustOptionsInput {
    pub include_session_only: bool,
}

impl Default for ProjectTrustOptionsInput {
    fn default() -> Self {
        ProjectTrustOptionsInput {
            include_session_only: false,
        }
    }
}

/// Generate the list of trust options shown to the user.
pub fn get_project_trust_options(
    cwd: &str,
    options: Option<&ProjectTrustOptionsInput>,
) -> Vec<ProjectTrustOption> {
    let trust_path = normalize_cwd(cwd);
    let trust_str = trust_path.to_string_lossy().to_string();
    let include_session_only = options.map(|o| o.include_session_only).unwrap_or(false);

    let mut trust_options = vec![ProjectTrustOption {
        label: "Trust".to_string(),
        trusted: true,
        updates: vec![ProjectTrustUpdate {
            path: trust_str.clone(),
            decision: Some(true),
        }],
        saved_path: Some(trust_str.clone()),
    }];

    if let Some(parent_path) = get_project_trust_parent_path(cwd) {
        trust_options.push(ProjectTrustOption {
            label: format!("Trust parent folder ({})", parent_path),
            trusted: true,
            updates: vec![
                ProjectTrustUpdate {
                    path: parent_path.clone(),
                    decision: Some(true),
                },
                ProjectTrustUpdate {
                    path: trust_str.clone(),
                    decision: None,
                },
            ],
            saved_path: Some(parent_path),
        });
    }

    if include_session_only {
        trust_options.push(ProjectTrustOption {
            label: "Trust (this session only)".to_string(),
            trusted: true,
            updates: vec![],
            saved_path: None,
        });
    }

    trust_options.push(ProjectTrustOption {
        label: "Do not trust".to_string(),
        trusted: false,
        updates: vec![ProjectTrustUpdate {
            path: trust_str.clone(),
            decision: Some(false),
        }],
        saved_path: Some(trust_str),
    });

    if include_session_only {
        trust_options.push(ProjectTrustOption {
            label: "Do not trust (this session only)".to_string(),
            trusted: false,
            updates: vec![],
            saved_path: None,
        });
    }

    trust_options
}

// ---------------------------------------------------------------------------
// hasTrustRequiringProjectResources
// ---------------------------------------------------------------------------

/// Returns true when cwd has project-local resources that must be gated by
/// project trust: trust-requiring entries under cwd/.hamr, or .agents/skills
/// in cwd or one of its ancestors.
pub fn has_trust_requiring_project_resources(cwd: &str) -> bool {
    let home_dir = canonicalize_path(&resolve_path(&std::env::var("HOME").unwrap_or_else(|_| {
        #[cfg(unix)]
        {
            unsafe {
                let pw = libc::getpwuid(libc::getuid());
                if !pw.is_null() {
                    let pw_dir = std::ffi::CStr::from_ptr((*pw).pw_dir);
                    return pw_dir.to_string_lossy().to_string();
                }
            }
            "/tmp".to_string()
        }
        #[cfg(not(unix))]
        {
            "/tmp".to_string()
        }
    })));
    let user_agents_skills_dir = home_dir.join(".agents").join("skills");
    let mut current_dir = canonicalize_path(&resolve_path(cwd));

    // Check for .hamr resources in current directory
    let config_dir = current_dir.join(".hamr");
    if TRUST_REQUIRING_PROJECT_CONFIG_RESOURCES
        .iter()
        .any(|entry| config_dir.join(entry).exists())
    {
        return true;
    }

    // Walk up
    loop {
        let agents_skills_dir = current_dir.join(".agents").join("skills");
        if agents_skills_dir != user_agents_skills_dir && agents_skills_dir.exists() {
            return true;
        }

        let parent_dir = match current_dir.parent() {
            Some(p) => p.to_path_buf(),
            None => return false,
        };

        if parent_dir == current_dir {
            return false;
        }
        current_dir = parent_dir;
    }
}

// ---------------------------------------------------------------------------
// ProjectTrustStore
// ---------------------------------------------------------------------------

/// Persistent store for project trust decisions.
pub struct ProjectTrustStore {
    trust_path: PathBuf,
}

impl ProjectTrustStore {
    /// Create a new trust store that persists to `{agent_dir}/trust.json`.
    pub fn new(agent_dir: &Path) -> Self {
        let trust_path = resolve_path(&agent_dir.join("trust.json").to_string_lossy().to_string());
        ProjectTrustStore { trust_path }
    }

    /// Get the trust decision for a directory (null if no decision exists).
    pub fn get(&self, cwd: &str) -> ProjectTrustDecision {
        self.get_entry(cwd).map(|e| e.decision)
    }

    /// Get the nearest trust entry for a directory.
    pub fn get_entry(&self, cwd: &str) -> Option<ProjectTrustStoreEntry> {
        with_trust_file_lock(&self.trust_path, || {
            let data = read_trust_file(&self.trust_path).unwrap_or_default();
            find_nearest_trust_entry(&data, &normalize_cwd(cwd))
        })
    }

    /// Set a trust decision for a directory.
    pub fn set(&self, cwd: &str, decision: ProjectTrustDecision) {
        self.set_many(&[ProjectTrustUpdate {
            path: cwd.to_string(),
            decision,
        }])
    }

    /// Apply multiple trust updates atomically.
    pub fn set_many(&self, decisions: &[ProjectTrustUpdate]) {
        with_trust_file_lock(&self.trust_path, || {
            let mut data = read_trust_file(&self.trust_path).unwrap_or_default();

            for update in decisions {
                let key = normalize_cwd(&update.path).to_string_lossy().to_string();
                match update.decision {
                    None => {
                        data.remove(&key);
                    }
                    Some(decision) => {
                        data.insert(key, Some(decision));
                    }
                }
            }

            let _ = write_trust_file(&self.trust_path, &data);
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_stores_decisions_and_inherits_from_parent() {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let agent_dir = temp_dir.path().join("agent");
        fs::create_dir_all(&agent_dir).unwrap();

        let parent_dir = temp_dir.path().join("trusted-parent");
        let child_dir = parent_dir.join("project");
        fs::create_dir_all(&child_dir).unwrap();

        let store = ProjectTrustStore::new(&agent_dir);

        let child_str = child_dir.to_string_lossy().to_string();
        let parent_str = parent_dir.to_string_lossy().to_string();

        // Initially no decision
        assert_eq!(store.get(&child_str), None);

        // Set parent to trusted → child inherits
        store.set(&parent_str, Some(true));
        assert_eq!(store.get(&child_str), Some(true));

        // Override child to not trusted
        store.set(&child_str, Some(false));
        assert_eq!(store.get(&child_str), Some(false));

        // Clear child → reverts to parent
        store.set(&child_str, None);
        assert_eq!(store.get(&child_str), Some(true));
    }

    #[test]
    fn test_detects_trust_requiring_project_resources() {
        let temp_dir = TempDir::new().expect("failed to create temp dir");

        // Save original HOME
        let original_home = std::env::var("HOME").ok();
        unsafe {
            std::env::set_var("HOME", temp_dir.path().to_string_lossy().as_ref());
        }

        let cwd = temp_dir.path().join("project");
        fs::create_dir_all(&cwd).unwrap();
        // Create global .agents/skills — should be ignored
        fs::create_dir_all(temp_dir.path().join(".agents").join("skills")).unwrap();

        // No project resources → false
        assert!(!has_trust_requiring_project_resources(
            &temp_dir.path().to_string_lossy()
        ));
        assert!(!has_trust_requiring_project_resources(
            &cwd.to_string_lossy()
        ));

        // Create .hamr/settings.json at temp_dir → true for temp_dir
        fs::create_dir_all(temp_dir.path().join(".hamr")).unwrap();
        fs::write(temp_dir.path().join(".hamr").join("settings.json"), "{}").unwrap();
        assert!(has_trust_requiring_project_resources(
            &temp_dir.path().to_string_lossy()
        ));

        // Clean up temp_dir .hamr, create project-level .hamr/settings.json
        fs::remove_dir_all(temp_dir.path().join(".hamr")).unwrap();
        fs::create_dir_all(cwd.join(".hamr")).unwrap();
        fs::write(cwd.join(".hamr").join("settings.json"), "{}").unwrap();
        assert!(has_trust_requiring_project_resources(
            &cwd.to_string_lossy()
        ));

        // Clean up, create project-level .agents/skills
        fs::remove_dir_all(cwd.join(".hamr")).unwrap();
        fs::create_dir_all(cwd.join(".agents").join("skills")).unwrap();
        assert!(has_trust_requiring_project_resources(
            &cwd.to_string_lossy()
        ));

        // Restore HOME
        unsafe {
            if let Some(h) = original_home {
                std::env::set_var("HOME", h);
            } else {
                std::env::remove_var("HOME");
            }
        }
    }
}
