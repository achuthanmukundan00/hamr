//! Port of `packages/coding-agent/src/utils/version_check.ts`.
//!
//! Check for new Hamr versions via npm registry and hamr.dev API.

use crate::utils::hamr_user_agent::get_hamr_user_agent;

/// Info about the latest Hamr release.
#[derive(Debug, Clone, Default)]
pub struct LatestHamrRelease {
    pub version: String,
    pub package_name: Option<String>,
    pub note: Option<String>,
}

const HAMR_NPM_PACKAGE_NAME: &str = "@skaft/hamr";
const NPM_LATEST_VERSION_URL: &str = "https://registry.npmjs.org/@skaft%2fhamr/latest";
const HAMR_LATEST_VERSION_URL: &str = "https://hamr.dev/api/latest-version";
const DEFAULT_VERSION_CHECK_TIMEOUT_MS: u64 = 10_000;

/// Compare two semver strings. Returns `std::cmp::Ordering` if both are valid,
/// or `None` if either is invalid.
pub fn compare_package_versions(left: &str, right: &str) -> Option<std::cmp::Ordering> {
    let left_parsed = semver::Version::parse(left.trim()).ok()?;
    let right_parsed = semver::Version::parse(right.trim()).ok()?;
    Some(left_parsed.cmp(&right_parsed))
}

/// Returns `true` if `candidate_version` is strictly newer than `current_version`.
pub fn is_newer_package_version(candidate: &str, current: &str) -> bool {
    match compare_package_versions(candidate, current) {
        Some(std::cmp::Ordering::Greater) => true,
        Some(_) => false,
        None => candidate.trim() != current.trim(),
    }
}

fn parse_latest_release(data: &serde_json::Value) -> Option<LatestHamrRelease> {
    let version = data.get("version")?.as_str()?;
    let version = version.trim();
    if version.is_empty() {
        return None;
    }

    let package_name = data
        .get("packageName")
        .or_else(|| data.get("name"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string());

    let note = data
        .get("note")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string());

    Some(LatestHamrRelease {
        version: version.to_string(),
        package_name,
        note,
    })
}

async fn fetch_latest_release(
    url: &str,
    current_version: &str,
    timeout_ms: u64,
) -> Result<Option<LatestHamrRelease>, reqwest::Error> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(timeout_ms))
        .build()?;

    let response = client
        .get(url)
        .header("accept", "application/json")
        .header("User-Agent", get_hamr_user_agent(current_version))
        .send()
        .await?;

    if !response.status().is_success() {
        return Ok(None);
    }

    let data: serde_json::Value = response.json().await?;
    Ok(parse_latest_release(&data))
}

/// Fetch the latest Hamr release info from npm registry or hamr.dev API.
pub async fn get_latest_hamr_release(
    current_version: &str,
    timeout_ms: Option<u64>,
) -> Option<LatestHamrRelease> {
    if std::env::var("HAMR_SKIP_VERSION_CHECK").is_ok()
        || std::env::var("PI_SKIP_VERSION_CHECK").is_ok()
        || std::env::var("HAMR_OFFLINE").is_ok()
        || std::env::var("PI_OFFLINE").is_ok()
    {
        return None;
    }

    let timeout = timeout_ms.unwrap_or(DEFAULT_VERSION_CHECK_TIMEOUT_MS);

    // Try npm registry first
    if let Ok(Some(mut release)) =
        fetch_latest_release(NPM_LATEST_VERSION_URL, current_version, timeout).await
    {
        release.package_name = Some(HAMR_NPM_PACKAGE_NAME.to_string());
        if !release.version.is_empty() {
            return Some(release);
        }
    }

    // Fallback to hamr.dev API
    fetch_latest_release(HAMR_LATEST_VERSION_URL, current_version, timeout)
        .await
        .unwrap_or(None)
}

/// Get the latest Hamr version string.
pub async fn get_latest_hamr_version(current_version: &str) -> Option<String> {
    get_latest_hamr_release(current_version, None)
        .await
        .map(|r| r.version)
}

/// Check if a newer version of Hamr is available.
pub async fn check_for_new_hamr_version(current_version: &str) -> Option<LatestHamrRelease> {
    get_latest_hamr_release(current_version, None)
        .await
        .filter(|release| is_newer_package_version(&release.version, current_version))
}

// Backward-compatible aliases
pub use check_for_new_hamr_version as check_for_new_pi_version;
pub use get_latest_hamr_release as get_latest_pi_release;
pub use get_latest_hamr_version as get_latest_pi_version;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compare_versions() {
        assert_eq!(
            compare_package_versions("1.0.0", "0.9.0"),
            Some(std::cmp::Ordering::Greater)
        );
        assert_eq!(
            compare_package_versions("1.0.0", "1.0.0"),
            Some(std::cmp::Ordering::Equal)
        );
        assert_eq!(
            compare_package_versions("0.9.0", "1.0.0"),
            Some(std::cmp::Ordering::Less)
        );
    }

    #[test]
    fn test_compare_invalid() {
        assert_eq!(compare_package_versions("not-a-version", "1.0.0"), None);
    }

    #[test]
    fn test_is_newer() {
        assert!(is_newer_package_version("1.1.0", "1.0.0"));
        assert!(!is_newer_package_version("0.9.0", "1.0.0"));
        assert!(!is_newer_package_version("1.0.0", "1.0.0"));
    }

    #[test]
    fn test_parse_latest_release_empty_data() {
        let data = serde_json::json!({});
        assert!(parse_latest_release(&data).is_none());
    }

    #[test]
    fn test_parse_latest_release_empty_version() {
        let data = serde_json::json!({"version": ""});
        assert!(parse_latest_release(&data).is_none());
    }

    #[test]
    fn test_parse_latest_release_with_note() {
        let data = serde_json::json!({
            "version": "1.2.3",
            "note": " **Read this** ",
        });
        let release = parse_latest_release(&data);
        assert!(release.is_some());
        let release = release.unwrap();
        assert_eq!(release.version, "1.2.3");
        assert_eq!(release.note.as_deref(), Some("**Read this**"));
        assert_eq!(release.package_name, None);
    }

    #[test]
    fn test_parse_latest_release_with_package_name() {
        let data = serde_json::json!({
            "version": "1.2.3",
            "name": "@skaft/hamr",
        });
        let release = parse_latest_release(&data);
        assert!(release.is_some());
        let release = release.unwrap();
        assert_eq!(release.package_name.as_deref(), Some("@skaft/hamr"));
    }

    #[test]
    fn test_parse_latest_release_with_package_name_field() {
        let data = serde_json::json!({
            "version": "1.2.3",
            "packageName": "@skaft/hamr",
        });
        let release = parse_latest_release(&data);
        assert!(release.is_some());
        let release = release.unwrap();
        assert_eq!(release.package_name.as_deref(), Some("@skaft/hamr"));
    }
}
