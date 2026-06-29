//! Port of `packages/coding-agent/src/utils/tools-manager.ts`.
//!
//! Manage external tool binaries (fd, ripgrep): detect, download, and install.

use std::path::{Path, PathBuf};
use std::process::Command;

const NETWORK_TIMEOUT_MS: u64 = 10_000;
const DOWNLOAD_TIMEOUT_MS: u64 = 120_000;

fn is_offline_mode_enabled() -> bool {
    let val = std::env::var("HAMR_OFFLINE")
        .or_else(|_| std::env::var("PI_OFFLINE"))
        .unwrap_or_default();
    val == "1" || val.eq_ignore_ascii_case("true") || val.eq_ignore_ascii_case("yes")
}

/// Configuration for an external tool.
struct ToolConfig {
    name: &'static str,
    repo: &'static str,
    binary_name: &'static str,
    system_binary_names: &'static [&'static str],
    tag_prefix: &'static str,
    get_asset_name: fn(version: &str, plat: &str, arch: &str) -> Option<String>,
}

static TOOLS: &[ToolConfig] = &[
    ToolConfig {
        name: "fd",
        repo: "sharkdp/fd",
        binary_name: "fd",
        system_binary_names: &["fd", "fdfind"],
        tag_prefix: "v",
        get_asset_name: |version, plat, arch| {
            let arch_str = if arch == "arm64" { "aarch64" } else { "x86_64" };
            match plat {
                "darwin" => Some(format!("fd-v{version}-{arch_str}-apple-darwin.tar.gz")),
                "linux" => Some(format!("fd-v{version}-{arch_str}-unknown-linux-gnu.tar.gz")),
                "windows" => Some(format!("fd-v{version}-{arch_str}-pc-windows-msvc.zip")),
                _ => None,
            }
        },
    },
    ToolConfig {
        name: "ripgrep",
        repo: "BurntSushi/ripgrep",
        binary_name: "rg",
        system_binary_names: &["rg"],
        tag_prefix: "",
        get_asset_name: |version, plat, arch| match plat {
            "darwin" => {
                let arch_str = if arch == "arm64" { "aarch64" } else { "x86_64" };
                Some(format!("ripgrep-{version}-{arch_str}-apple-darwin.tar.gz"))
            }
            "linux" => {
                if arch == "arm64" {
                    Some(format!(
                        "ripgrep-{version}-aarch64-unknown-linux-gnu.tar.gz"
                    ))
                } else {
                    Some(format!(
                        "ripgrep-{version}-x86_64-unknown-linux-musl.tar.gz"
                    ))
                }
            }
            "windows" => {
                let arch_str = if arch == "arm64" { "aarch64" } else { "x86_64" };
                Some(format!("ripgrep-{version}-{arch_str}-pc-windows-msvc.zip"))
            }
            _ => None,
        },
    },
];

/// Get the tools directory path from the bin directory.
pub fn get_tools_dir() -> PathBuf {
    // Use HAMR_BIN_DIR or default to ~/.hamr/bin
    if let Ok(dir) = std::env::var("HAMR_BIN_DIR") {
        PathBuf::from(dir)
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_string());
        PathBuf::from(home).join(".hamr").join("bin")
    }
}

fn command_exists(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Get the path to a tool (system-wide or in our tools dir).
pub fn get_tool_path(tool: &str) -> Option<String> {
    let config = TOOLS
        .iter()
        .find(|t| t.binary_name == tool || t.name == tool)?;
    let tools_dir = get_tools_dir();

    #[cfg(windows)]
    let binary_name = format!("{}.exe", config.binary_name);
    #[cfg(not(windows))]
    let binary_name = config.binary_name.to_string();

    // Check our tools directory first
    let local_path = tools_dir.join(&binary_name);
    if local_path.exists() {
        return Some(local_path.to_string_lossy().to_string());
    }

    // Check system PATH
    for name in config.system_binary_names {
        if command_exists(name) {
            return Some(name.to_string());
        }
    }

    None
}

async fn get_latest_version(repo: &str) -> Result<String, String> {
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(NETWORK_TIMEOUT_MS))
        .build()
        .map_err(|e| format!("HTTP client build error: {e}"))?;

    let response = client
        .get(&url)
        .header("User-Agent", "hamr-coding-agent")
        .send()
        .await
        .map_err(|e| format!("GitHub API request error: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("GitHub API error: {}", response.status()));
    }

    let data: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("GitHub API JSON error: {e}"))?;

    let tag_name = data["tag_name"]
        .as_str()
        .ok_or_else(|| "Missing tag_name in GitHub response".to_string())?;

    Ok(tag_name.trim_start_matches('v').to_string())
}

async fn download_file(url: &str, dest: &Path) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(DOWNLOAD_TIMEOUT_MS))
        .build()
        .map_err(|e| format!("HTTP client build error: {e}"))?;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Download request error: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("Download failed: {}", response.status()));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Download body error: {e}"))?;

    tokio::fs::write(dest, &bytes)
        .await
        .map_err(|e| format!("File write error: {e}"))?;

    Ok(())
}

fn find_binary_recursively(root_dir: &Path, binary_file_name: &str) -> Option<PathBuf> {
    let mut stack = vec![root_dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.file_name().map_or(false, |n| n == binary_file_name) {
                    return Some(path);
                }
                if path.is_dir() {
                    stack.push(path);
                }
            }
        }
    }
    None
}

fn extract_tar_gz_archive(archive_path: &Path, extract_dir: &Path) -> Result<(), String> {
    let file =
        std::fs::File::open(archive_path).map_err(|e| format!("Failed to open archive: {e}"))?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    archive
        .unpack(extract_dir)
        .map_err(|e| format!("Tar extraction error: {e}"))?;
    Ok(())
}

fn extract_zip_archive(archive_path: &Path, extract_dir: &Path) -> Result<(), String> {
    let file =
        std::fs::File::open(archive_path).map_err(|e| format!("Failed to open archive: {e}"))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Zip open error: {e}"))?;
    archive
        .extract(extract_dir)
        .map_err(|e| format!("Zip extraction error: {e}"))?;
    Ok(())
}

async fn download_tool(tool: &str) -> Result<String, String> {
    let config = TOOLS
        .iter()
        .find(|t| t.binary_name == tool || t.name == tool)
        .ok_or_else(|| format!("Unknown tool: {tool}"))?;

    let plat = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    // Get latest version
    let version = get_latest_version(config.repo).await?;

    // Special case: fd on macOS x64 needs v10.3.0 (last x86_64 build)
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    if tool == "fd" {
        version = "10.3.0".to_string();
    }

    // Get asset name
    let asset_name = (config.get_asset_name)(&version, plat, arch)
        .ok_or_else(|| format!("Unsupported platform: {plat}/{arch}"))?;

    let tools_dir = get_tools_dir();
    tokio::fs::create_dir_all(&tools_dir)
        .await
        .map_err(|e| format!("Failed to create tools dir: {e}"))?;

    let download_url = format!(
        "https://github.com/{repo}/releases/download/{prefix}{version}/{asset}",
        repo = config.repo,
        prefix = config.tag_prefix,
        version = version,
        asset = asset_name
    );
    let archive_path = tools_dir.join(&asset_name);

    #[cfg(windows)]
    let binary_ext = ".exe";
    #[cfg(not(windows))]
    let binary_ext = "";

    let binary_path = tools_dir.join(format!("{}{}", config.binary_name, binary_ext));

    // Download
    download_file(&download_url, &archive_path).await?;

    // Extract into temp directory
    let extract_dir = tools_dir.join(format!(
        "extract_tmp_{}_{}_{}",
        config.binary_name,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    tokio::fs::create_dir_all(&extract_dir)
        .await
        .map_err(|e| format!("Failed to create extract dir: {e}"))?;

    let result = (|| -> Result<(), String> {
        if asset_name.ends_with(".tar.gz") {
            extract_tar_gz_archive(&archive_path, &extract_dir)?;
        } else if asset_name.ends_with(".zip") {
            extract_zip_archive(&archive_path, &extract_dir)?;
        } else {
            return Err(format!("Unsupported archive format: {asset_name}"));
        }

        // Find the binary
        let binary_file_name = format!("{}{}", config.binary_name, binary_ext);

        // Try the expected extracted subdirectory first
        let stem = asset_name
            .strip_suffix(".tar.gz")
            .or_else(|| asset_name.strip_suffix(".zip"))
            .unwrap_or(&asset_name);
        let extracted_dir = extract_dir.join(stem);
        let mut extracted_binary = None;

        for candidate in [
            extracted_dir.join(&binary_file_name),
            extract_dir.join(&binary_file_name),
        ] {
            if candidate.exists() {
                extracted_binary = Some(candidate);
                break;
            }
        }

        if extracted_binary.is_none() {
            extracted_binary = find_binary_recursively(&extract_dir, &binary_file_name);
        }

        match extracted_binary {
            Some(src) => {
                std::fs::rename(&src, &binary_path)
                    .map_err(|e| format!("Failed to move binary: {e}"))?;
            }
            None => {
                return Err(format!(
                    "Binary not found in archive: expected {binary_file_name}"
                ));
            }
        }

        // Make executable (Unix only)
        #[cfg(not(windows))]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&binary_path, std::fs::Permissions::from_mode(0o755))
                .map_err(|e| format!("Failed to set permissions: {e}"))?;
        }

        Ok(())
    })();

    // Cleanup
    let _ = tokio::fs::remove_file(&archive_path).await;
    let _ = tokio::fs::remove_dir_all(&extract_dir).await;

    result?;

    Ok(binary_path.to_string_lossy().to_string())
}

/// Ensure a tool is available, downloading if necessary.
/// Returns the path to the tool, or `None` if unavailable.
pub async fn ensure_tool(tool: &str, silent: bool) -> Option<String> {
    if let Some(path) = get_tool_path(tool) {
        return Some(path);
    }

    let config = TOOLS
        .iter()
        .find(|t| t.binary_name == tool || t.name == tool)?;

    if is_offline_mode_enabled() {
        if !silent {
            eprintln!(
                "{} not found. Offline mode enabled, skipping download.",
                config.name
            );
        }
        return None;
    }

    // On Android/Termux, Linux binaries won't work
    #[cfg(target_os = "android")]
    {
        if !silent {
            eprintln!(
                "{} not found. Install with: pkg install {}",
                config.name, tool
            );
        }
        return None;
    }

    if !silent {
        eprintln!("{} not found. Downloading...", config.name);
    }

    match download_tool(tool).await {
        Ok(path) => {
            if !silent {
                eprintln!("{} installed to {}", config.name, path);
            }
            Some(path)
        }
        Err(e) => {
            if !silent {
                eprintln!("Failed to download {}: {}", config.name, e);
            }
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_tool_path_nonexistent() {
        // Should return None for nonexistent tools
        assert!(get_tool_path("nonexistent_tool_xyz").is_none());
    }

    #[test]
    fn test_tool_configs_valid() {
        // Verify tool configs produce asset names
        let plat = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        for config in TOOLS {
            let asset = (config.get_asset_name)("1.0.0", plat, arch);
            if ["darwin", "linux", "windows"].contains(&plat) {
                assert!(asset.is_some(), "No asset for {}/{}", config.name, plat);
            }
        }
    }
}
