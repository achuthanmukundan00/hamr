//! Port of `packages/coding-agent/src/utils/changelog.ts`.
//!
//! Parse and manipulate CHANGELOG.md files.

use std::path::Path;

/// A single changelog entry (version header + content).
#[derive(Debug, Clone)]
pub struct ChangelogEntry {
    pub major: i32,
    pub minor: i32,
    pub patch: i32,
    pub content: String,
}

const GITHUB_REPO: &str = "skaft-software/hamr";

/// Normalize a version string or entry to a tag (prefixed with `v`).
fn normalize_tag(version: &str) -> String {
    if version.starts_with('v') {
        version.to_string()
    } else {
        format!("v{version}")
    }
}

fn entry_version(entry: &ChangelogEntry) -> String {
    format!("{}.{}.{}", entry.major, entry.minor, entry.patch)
}

fn split_local_target(target: &str) -> (String, String, String) {
    let hash_index = target.find('#');
    let before_hash = match hash_index {
        Some(i) => &target[..i],
        None => target,
    };
    let fragment = match hash_index {
        Some(i) => target[i..].to_string(),
        None => String::new(),
    };

    let query_index = before_hash.find('?');
    let (path_part, query) = match query_index {
        Some(i) => (before_hash[..i].to_string(), before_hash[i..].to_string()),
        None => (before_hash.to_string(), String::new()),
    };

    (fragment, path_part, query)
}

fn normalize_path_part(value: &str) -> String {
    value.replace('\\', "/")
}

fn resolve_repository_path(target_path: &str) -> Option<String> {
    const CHANGELOG_LINK_BASE_PATH: &str = "packages/coding-agent";
    let normalized = normalize_path_part(target_path);

    let joined = if normalized.starts_with('/') {
        let trimmed = normalized.trim_start_matches('/');
        Path::new(trimmed).to_str().map(|s| s.to_string())
    } else {
        let combined = format!("{CHANGELOG_LINK_BASE_PATH}/{normalized}");
        Path::new(&combined).to_str().map(|s| s.to_string())
    };

    let joined = joined?;

    // Reject paths that escape the repo
    if joined == "." || joined.starts_with("../") || joined == ".." {
        return None;
    }

    Some(joined)
}

fn is_directory_target(original_path: &str, repository_path: &str) -> bool {
    if original_path.ends_with('/') {
        return true;
    }
    let basename = std::path::Path::new(repository_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    !basename.contains('.')
}

fn normalize_changelog_link_target(target: &str, tag: &str) -> String {
    let legacy_repo_re =
        regex::Regex::new(r"^https://github\.com/(?:badlogic|earendil-works)/pi-mono(?:/|$)")
            .unwrap();
    let url_scheme_re = regex::Regex::new(r"^[a-z][a-z0-9+.-]*:").unwrap();

    let repo_url = format!("https://github.com/{GITHUB_REPO}");

    let mut canonical = target.to_string();

    // Replace legacy repo URLs
    canonical = legacy_repo_re
        .replace_all(&canonical, format!("https://github.com/{GITHUB_REPO}/"))
        .into_owned();

    // Replace floating refs (main/master) with tag
    for route in ["blob", "tree"] {
        for branch in ["main", "master"] {
            let floating_prefix = format!("{repo_url}/{route}/{branch}/");
            if canonical.starts_with(&floating_prefix) {
                let rest = canonical[floating_prefix.len()..].to_string();
                canonical = format!("{repo_url}/{route}/{tag}/{rest}");
            }
        }
    }

    // If it's a fragment, protocol URL, or double-slash, return as-is
    if canonical.starts_with('#')
        || canonical.starts_with("//")
        || url_scheme_re.is_match(&canonical)
    {
        return canonical;
    }

    let (fragment, path_part, query) = split_local_target(&canonical);
    if path_part.is_empty() {
        return canonical;
    }

    if let Some(repository_path) = resolve_repository_path(&path_part) {
        let route = if is_directory_target(&path_part, &repository_path) {
            "tree"
        } else {
            "blob"
        };
        let encoded_path: String = repository_path
            .split('/')
            .map(|seg| urlencoding::encode(seg).into_owned())
            .collect::<Vec<_>>()
            .join("/");
        return format!("{repo_url}/{route}/{tag}/{encoded_path}{query}{fragment}");
    }

    canonical
}

/// Normalize markdown links in changelog entries so they point to the correct
/// tag on GitHub.
pub fn normalize_changelog_links(markdown: &str, version: &str) -> String {
    let tag = normalize_tag(version);
    let inline_link_re =
        regex::Regex::new(r"(!?\[[^\]\n]+\]\()([^\s)]+)((?:\s+[^)]*)?\))").unwrap();

    inline_link_re
        .replace_all(markdown, |caps: &regex::Captures| {
            let prefix = &caps[1];
            let target = &caps[2];
            let suffix = &caps[3];
            format!(
                "{}{}{}",
                prefix,
                normalize_changelog_link_target(target, &tag),
                suffix
            )
        })
        .into_owned()
}

/// Parse changelog entries from a CHANGELOG.md file.
///
/// Scans for `##` lines and collects content until the next `##` or EOF.
pub fn parse_changelog<P: AsRef<Path>>(changelog_path: P) -> Vec<ChangelogEntry> {
    let path = changelog_path.as_ref();
    if !path.exists() {
        return vec![];
    }

    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let lines: Vec<&str> = content.lines().collect();
    let version_header_re = regex::Regex::new(r"##\s+\[?(\d+)\.(\d+)\.(\d+)\]?").unwrap();
    let mut entries: Vec<ChangelogEntry> = vec![];

    let mut current_lines: Vec<String> = vec![];
    let mut current_version: Option<(i32, i32, i32)> = None;

    for line in &lines {
        if line.starts_with("## ") {
            // Save previous entry
            if let Some((maj, min, pat)) = current_version.take() {
                if !current_lines.is_empty() {
                    entries.push(ChangelogEntry {
                        major: maj,
                        minor: min,
                        patch: pat,
                        content: current_lines.join("\n").trim().to_string(),
                    });
                }
                current_lines = vec![];
            }

            // Try to parse version
            if let Some(caps) = version_header_re.captures(line) {
                let maj: i32 = caps[1].parse().unwrap_or(0);
                let min: i32 = caps[2].parse().unwrap_or(0);
                let pat: i32 = caps[3].parse().unwrap_or(0);
                current_version = Some((maj, min, pat));
                current_lines.push((*line).to_string());
            }
        } else if current_version.is_some() {
            current_lines.push((*line).to_string());
        }
    }

    // Save last entry
    if let Some((maj, min, pat)) = current_version {
        if !current_lines.is_empty() {
            entries.push(ChangelogEntry {
                major: maj,
                minor: min,
                patch: pat,
                content: current_lines.join("\n").trim().to_string(),
            });
        }
    }

    entries
}

/// Compare versions. Returns -1 if v1 < v2, 0 if equal, 1 if v1 > v2.
pub fn compare_versions(v1: &ChangelogEntry, v2: &ChangelogEntry) -> i32 {
    if v1.major != v2.major {
        return v1.major - v2.major;
    }
    if v1.minor != v2.minor {
        return v1.minor - v2.minor;
    }
    v1.patch - v2.patch
}

/// Get entries newer than `last_version` string (semver-like).
pub fn get_new_entries(entries: &[ChangelogEntry], last_version: &str) -> Vec<ChangelogEntry> {
    let parts: Vec<i32> = last_version
        .split('.')
        .map(|p| p.parse().unwrap_or(0))
        .collect();
    let last = ChangelogEntry {
        major: parts.first().copied().unwrap_or(0),
        minor: parts.get(1).copied().unwrap_or(0),
        patch: parts.get(2).copied().unwrap_or(0),
        content: String::new(),
    };

    entries
        .iter()
        .filter(|entry| compare_versions(entry, &last) > 0)
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let content = "## [1.0.0] - 2024-01-01\n\nFirst release\n\n## [1.1.0] - 2024-02-01\n\nSecond release\n";
        let path = std::env::temp_dir().join("CHANGELOG_TEST.md");
        std::fs::write(&path, content).unwrap();
        let entries = parse_changelog(&path);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].major, 1);
        assert_eq!(entries[0].minor, 0);
        assert_eq!(entries[0].patch, 0);
        assert_eq!(entries[1].minor, 1);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_no_changelog() {
        let entries = parse_changelog("/nonexistent/CHANGELOG.md");
        assert!(entries.is_empty());
    }

    #[test]
    fn test_compare_versions() {
        let v1 = ChangelogEntry {
            major: 1,
            minor: 0,
            patch: 0,
            content: String::new(),
        };
        let v2 = ChangelogEntry {
            major: 1,
            minor: 0,
            patch: 1,
            content: String::new(),
        };
        assert!(compare_versions(&v1, &v2) < 0);
        assert!(compare_versions(&v2, &v1) > 0);
        assert_eq!(compare_versions(&v1, &v1), 0);
    }

    #[test]
    fn test_get_new_entries() {
        let entries = vec![
            ChangelogEntry {
                major: 1,
                minor: 0,
                patch: 0,
                content: "v1.0".to_string(),
            },
            ChangelogEntry {
                major: 1,
                minor: 1,
                patch: 0,
                content: "v1.1".to_string(),
            },
            ChangelogEntry {
                major: 2,
                minor: 0,
                patch: 0,
                content: "v2.0".to_string(),
            },
        ];
        let new = get_new_entries(&entries, "1.0.0");
        assert_eq!(new.len(), 2);
        assert_eq!(new[0].minor, 1);
        assert_eq!(new[1].major, 2);
    }

    #[test]
    fn test_normalize_links() {
        let md = "See the [README](README.md) for details.";
        let result = normalize_changelog_links(md, "1.0.0");
        assert!(result.contains("skaft-software/hamr/blob/v1.0.0/"));
        assert!(result.contains("README.md"));
    }

    #[test]
    fn test_normalize_links_canonicalizes_old_repo_urls() {
        let md = "[#5167](https://github.com/earendil-works/pi-mono/pull/5167)\n\
                   [#4163](https://github.com/badlogic/pi-mono/issues/4163)\n\
                   [External](https://example.com/docs)\n\
                   [Local anchor](#settings)";
        let result = normalize_changelog_links(md, "0.79.0");
        assert!(result.contains("skaft-software/hamr/pull/5167"));
        assert!(result.contains("skaft-software/hamr/issues/4163"));
        assert!(result.contains("example.com/docs"));
        assert!(result.contains("#settings"));
    }

    #[test]
    fn test_normalize_links_preserves_external_links() {
        let md = "[External](https://example.com/docs)";
        let result = normalize_changelog_links(md, "0.79.0");
        assert!(result.contains("example.com/docs"));
    }

    #[test]
    fn test_normalize_links_handles_local_anchors() {
        let md = "[Local anchor](#settings)";
        let result = normalize_changelog_links(md, "0.79.0");
        assert!(result.contains("#settings"));
    }
}
