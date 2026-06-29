//! Port of `packages/coding-agent/src/utils/open_browser.ts`
//!
//! Open a URL or file in the platform browser/default handler.
//! This intentionally never invokes a shell.

use std::process::Command;

/// Open a URL or file in the platform browser/default handler.
///
/// This intentionally never invokes a shell. On Windows, do not use
/// `cmd /c start`: cmd.exe re-parses metacharacters (&, |, ^, ...) before
/// `start` runs, which would make attacker-controlled URLs injectable.
///
/// Launcher failures (e.g. missing xdg-open) are silently ignored.
/// Browser launch is best-effort: callers still present the target to the user.
pub fn open_browser(target: &str) {
    let (cmd, args): (&str, &[&str]) = if cfg!(target_os = "macos") {
        ("open", &[target])
    } else if cfg!(target_os = "windows") {
        ("rundll32", &["url.dll,FileProtocolHandler", target])
    } else {
        ("xdg-open", &[target])
    };

    // Best-effort: spawn detached and ignore errors.
    let _ = Command::new(cmd)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "actually opens browser on macOS; run manually"]
    fn test_open_browser_does_not_panic() {
        // open_browser should never panic, even with unusual targets.
        open_browser("https://example.com");
        open_browser("file:///tmp/test.html");
    }
}
