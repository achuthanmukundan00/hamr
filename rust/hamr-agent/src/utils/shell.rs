//! Port of `packages/coding-agent/src/utils/shell.ts`.
//!
//! Cross-platform shell detection, environment setup, process management.

use std::collections::HashSet;
use std::process::Command;
use std::sync::Mutex;

/// Shell configuration for spawning commands.
#[derive(Debug, Clone)]
pub struct ShellConfig {
    pub shell: String,
    pub args: Vec<String>,
}

/// Find bash executable on PATH (cross-platform).
fn find_bash_on_path() -> Option<String> {
    let result = if cfg!(target_os = "windows") {
        Command::new("where").args(["bash.exe"]).output().ok()?
    } else {
        Command::new("which").arg("bash").output().ok()?
    };

    if result.status.success() {
        let stdout = String::from_utf8_lossy(&result.stdout);
        let first = stdout.lines().next()?.trim();
        if !first.is_empty() {
            // Check the path actually exists (Windows `where` can return ghosts)
            if std::path::Path::new(first).exists() {
                return Some(first.to_string());
            }
        }
    }
    None
}

/// Resolve shell configuration based on platform and an optional explicit shell path.
///
/// Resolution order:
/// 1. User-specified `custom_shell_path`
/// 2. On Windows: Git Bash in known locations, then bash on PATH
/// 3. On Unix: `/bin/bash`, then bash on PATH, then fallback to `sh`
pub fn get_shell_config(custom_shell_path: Option<&str>) -> Result<ShellConfig, String> {
    // 1. Check user-specified shell
    if let Some(path) = custom_shell_path {
        if !path.is_empty() {
            if std::path::Path::new(path).exists() {
                return Ok(ShellConfig {
                    shell: path.to_string(),
                    args: vec!["-c".to_string()],
                });
            }
            return Err(format!("Custom shell path not found: {path}"));
        }
    }

    if cfg!(target_os = "windows") {
        // 2. Try Git Bash in known locations
        let paths = vec![
            format!(
                "{}\\Git\\bin\\bash.exe",
                std::env::var("ProgramFiles").unwrap_or_default()
            ),
            format!(
                "{}\\Git\\bin\\bash.exe",
                std::env::var("ProgramFiles(x86)").unwrap_or_default()
            ),
        ];
        for p in &paths {
            if std::path::Path::new(p).exists() {
                return Ok(ShellConfig {
                    shell: p.clone(),
                    args: vec!["-c".to_string()],
                });
            }
        }

        // 3. Fallback to bash on PATH
        if let Some(bash_path) = find_bash_on_path() {
            return Ok(ShellConfig {
                shell: bash_path,
                args: vec!["-c".to_string()],
            });
        }

        return Err(
            "No bash shell found. Options:\n  1. Install Git for Windows: https://git-scm.com/download/win\n  2. Add your bash to PATH (Cygwin, MSYS2, etc.)\n  3. Set shellPath in settings.json"
                .to_string(),
        );
    }

    // Unix: try /bin/bash, then bash on PATH, then fallback to sh
    if std::path::Path::new("/bin/bash").exists() {
        return Ok(ShellConfig {
            shell: "/bin/bash".to_string(),
            args: vec!["-c".to_string()],
        });
    }

    if let Some(bash_path) = find_bash_on_path() {
        return Ok(ShellConfig {
            shell: bash_path,
            args: vec!["-c".to_string()],
        });
    }

    Ok(ShellConfig {
        shell: "sh".to_string(),
        args: vec!["-c".to_string()],
    })
}

/// Build the shell environment, ensuring the bin directory is on PATH.
pub fn get_shell_env(bin_dir: &str) -> std::collections::HashMap<String, String> {
    let mut env: std::collections::HashMap<String, String> = std::env::vars().collect();

    let path_key = env
        .keys()
        .find(|k| k.to_lowercase() == "path")
        .cloned()
        .unwrap_or_else(|| "PATH".to_string());

    let current_path = env.get(&path_key).cloned().unwrap_or_default();
    let path_entries: Vec<&str> = current_path.split(':').filter(|s| !s.is_empty()).collect();
    let has_bin_dir = path_entries.iter().any(|p| *p == bin_dir);

    if !has_bin_dir {
        let updated_path = if current_path.is_empty() {
            bin_dir.to_string()
        } else {
            format!("{bin_dir}:{current_path}")
        };
        env.insert(path_key, updated_path);
    }

    env
}

/// Sanitize binary output for display/storage.
///
/// Removes characters that crash string-width or cause display issues:
/// - Control characters (except tab, newline, carriage return)
/// - Unicode Format characters (which can crash string-width)
pub fn sanitize_binary_output(s: &str) -> String {
    s.chars()
        .filter(|&c| {
            let code = c as u32;

            // Allow tab, newline, carriage return
            if code == 0x09 || code == 0x0a || code == 0x0d {
                return true;
            }

            // Filter out control characters (0x00-0x1F)
            if code <= 0x1f {
                return false;
            }

            // Filter out Unicode format characters
            if (0xfff9..=0xfffb).contains(&code) {
                return false;
            }

            true
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Detached child process tracking (for clean shutdown)
// ---------------------------------------------------------------------------

static TRACKED_PIDS: std::sync::LazyLock<Mutex<HashSet<u32>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashSet::new()));

/// Track a detached child PID so it can be killed on shutdown.
pub fn track_detached_child_pid(pid: u32) {
    if let Ok(mut pids) = TRACKED_PIDS.lock() {
        pids.insert(pid);
    }
}

/// Stop tracking a child PID.
pub fn untrack_detached_child_pid(pid: u32) {
    if let Ok(mut pids) = TRACKED_PIDS.lock() {
        pids.remove(&pid);
    }
}

/// Kill all tracked detached children.
pub fn kill_tracked_detached_children() {
    let pids: Vec<u32> = TRACKED_PIDS
        .lock()
        .ok()
        .map(|p| p.iter().copied().collect())
        .unwrap_or_default();
    for pid in pids {
        kill_process_tree(pid);
        untrack_detached_child_pid(pid);
    }
}

/// Kill a process and all its children (cross-platform).
pub fn kill_process_tree(pid: u32) {
    #[cfg(unix)]
    {
        // Try process group kill first (negative PID)
        let rc = unsafe { libc::kill(-(pid as i32), libc::SIGKILL) };
        if rc != 0 {
            // Fallback to killing just the child
            unsafe { libc::kill(pid as i32, libc::SIGKILL) };
        }
    }

    #[cfg(windows)]
    {
        use std::process::Command;
        let _ = Command::new("taskkill")
            .args(["/F", "/T", "/PID", &pid.to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_binary_output() {
        // Control chars filtered
        assert_eq!(sanitize_binary_output("hello\x00world"), "helloworld");
        // Tab, newline, CR preserved
        assert_eq!(sanitize_binary_output("a\tb\nc\rd"), "a\tb\nc\rd");
        // Format chars filtered
        let format_chars: String = char::from_u32(0xfff9).into_iter().collect();
        assert_eq!(sanitize_binary_output(&format_chars), "");
        // Normal text passes through
        assert_eq!(sanitize_binary_output("hello world"), "hello world");
    }

    #[test]
    fn test_get_shell_config_unix() {
        #[cfg(unix)]
        {
            let config = get_shell_config(None).unwrap();
            // Should find some shell
            assert!(!config.shell.is_empty());
            assert_eq!(config.args, vec!["-c"]);
        }
    }

    #[test]
    fn test_get_shell_env_adds_bin_dir() {
        let env = get_shell_env("/custom/bin");
        let path = std::env::var("PATH").unwrap_or_default();
        if !path.contains("/custom/bin") {
            assert!(
                env.get("PATH")
                    .map_or(false, |p| p.starts_with("/custom/bin:")),
                "Expected PATH to start with /custom/bin: but got {:?}",
                env.get("PATH")
            );
        }
    }

    #[test]
    fn test_sanitize_binary_output_control_chars() {
        assert_eq!(
            sanitize_binary_output("hello\x00world\x01test"),
            "helloworldtest"
        );
    }

    #[test]
    fn test_sanitize_binary_output_preserves_whitespace() {
        assert_eq!(sanitize_binary_output("a\tb\nc\rd"), "a\tb\nc\rd");
    }

    #[test]
    fn test_sanitize_binary_output_format_chars() {
        let format_chars: String = (0xfff9u32..=0xfffbu32).filter_map(char::from_u32).collect();
        assert_eq!(sanitize_binary_output(&format_chars), "");
    }

    #[test]
    fn test_track_and_untrack_pid() {
        track_detached_child_pid(99999);
        untrack_detached_child_pid(99999);
        // Should not panic
    }

    #[test]
    fn test_custom_shell_path_not_found() {
        let result = get_shell_config(Some("/nonexistent/shell/path"));
        assert!(result.is_err());
    }
}
