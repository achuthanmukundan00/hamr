//! Port of `packages/coding-agent/src/utils/clipboard.ts`.
//!
//! Cross-platform clipboard write support.

use std::io::Write;
use std::process::{Command, Stdio};

const MAX_OSC52_ENCODED_LENGTH: usize = 100_000;

fn is_remote_session() -> bool {
    std::env::var("SSH_CONNECTION").is_ok()
        || std::env::var("SSH_CLIENT").is_ok()
        || std::env::var("MOSH_CONNECTION").is_ok()
}

fn is_wayland_session() -> bool {
    // Check if WAYLAND_DISPLAY is set and DISPLAY is not set, or if the
    // session type is explicitly Wayland.
    let has_wayland = std::env::var("WAYLAND_DISPLAY").is_ok();
    let has_x11 = std::env::var("DISPLAY").is_ok();
    has_wayland && !has_x11
}

fn emit_osc52(text: &str) -> bool {
    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(text);
    if encoded.len() > MAX_OSC52_ENCODED_LENGTH {
        return false;
    }
    // Write OSC 52 sequence to stdout
    let seq = format!("\x1b]52;c;{encoded}\x07");
    let mut stdout = std::io::stdout().lock();
    let _ = stdout.write_all(seq.as_bytes());
    let _ = stdout.flush();
    true
}

/// Copy text to the system clipboard.
///
/// Tries, in order:
/// 1. Platform-native clipboard tools (`pbcopy` on macOS, `clip` on Windows,
///    `wl-copy`/`xclip`/`xsel` on Linux)
/// 2. OSC 52 escape sequence (for remote/SSH sessions)
/// 3. Returns `Err` if all methods fail
pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
    let p = std::env::consts::OS;

    let remote = is_remote_session();

    // 1. Try platform-native clipboard tools
    let mut copied = false;

    let run_cmd = |cmd: &str, args: &[&str]| -> Result<(), String> {
        let mut child = Command::new(cmd)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to spawn {cmd}: {e}"))?;

        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(text.as_bytes());
            // Drop stdin to close it
        }
        let status = child
            .wait()
            .map_err(|e| format!("Failed to wait for {cmd}: {e}"))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("{cmd} exited with {status}"))
        }
    };

    match p {
        "macos" => {
            if run_cmd("pbcopy", &[]).is_ok() {
                copied = true;
            }
        }
        "windows" => {
            if run_cmd("clip", &[]).is_ok() {
                copied = true;
            }
        }
        _ => {
            // Linux / others
            // Try Termux
            if std::env::var("TERMUX_VERSION").is_ok() {
                if run_cmd("termux-clipboard-set", &[]).is_ok() {
                    copied = true;
                }
            }

            if !copied {
                let is_wayland = is_wayland_session();
                let has_wayland_display = std::env::var("WAYLAND_DISPLAY").is_ok();
                let has_x11_display = std::env::var("DISPLAY").is_ok();

                if is_wayland && has_wayland_display {
                    // wl-copy: use spawn with pipe to avoid blocking
                    if let Ok(mut child) = Command::new("wl-copy")
                        .stdin(Stdio::piped())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .spawn()
                    {
                        if let Some(mut stdin) = child.stdin.take() {
                            let _ = stdin.write_all(text.as_bytes());
                        }
                        // Detach — don't wait; wl-copy daemonizes
                        let _ = child.wait();
                        copied = true;
                    } else if has_x11_display {
                        // Fallback to xclip
                        if run_cmd("xclip", &["-selection", "clipboard"]).is_ok() {
                            copied = true;
                        }
                    }
                } else if has_x11_display {
                    // Try xclip first, then xsel
                    if run_cmd("xclip", &["-selection", "clipboard"]).is_ok() {
                        copied = true;
                    } else if run_cmd("xsel", &["--clipboard", "--input"]).is_ok() {
                        copied = true;
                    }
                }
            }
        }
    }

    // 2. For remote sessions or if native failed, try OSC 52
    if remote || !copied {
        let osc52_copied = emit_osc52(text);
        copied = copied || osc52_copied;
    }

    if copied {
        Ok(())
    } else {
        Err("Failed to copy to clipboard".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_osc52_too_long() {
        // 100k+ bytes base64 → need >75k input chars
        let long = "x".repeat(80_000);
        assert!(!emit_osc52(&long));
    }

    #[test]
    fn test_osc52_short() {
        assert!(emit_osc52("hello"));
    }
}
