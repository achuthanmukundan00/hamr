//! Port of `packages/coding-agent/src/utils/git.ts`.
//!
//! Parse git URLs into structured `GitSource` values.

/// Parsed git URL information.
#[derive(Debug, Clone, PartialEq)]
pub struct GitSource {
    /// Always `"git"` for git sources.
    pub r#type: String,
    /// Clone URL (always valid for git clone, without ref suffix).
    pub repo: String,
    /// Git host domain (e.g., `github.com`).
    pub host: String,
    /// Repository path (e.g., `user/repo`).
    pub path: String,
    /// Git ref (branch, tag, commit) if specified.
    pub ref_: Option<String>,
    /// True if ref was specified (package won't be auto-updated).
    pub pinned: bool,
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn split_ref(url: &str) -> (String, Option<String>) {
    // SCP-like: git@host:user/repo[@ref]
    if let Some(captures) = url.strip_prefix("git@") {
        if let Some(at_pos) = captures.find(':') {
            let host_part = &captures[..at_pos];
            let after_colon = &captures[at_pos + 1..];
            if let Some(ref_sep) = after_colon.rfind('@') {
                let repo_path = &after_colon[..ref_sep];
                let ref_str = &after_colon[ref_sep + 1..];
                if !repo_path.is_empty() && !ref_str.is_empty() {
                    return (
                        format!("git@{host_part}:{repo_path}"),
                        Some(ref_str.to_string()),
                    );
                }
            }
            return (url.to_string(), None);
        }
    }

    // Protocol URLs — extract #ref before parsing with url::Url (which strips fragments)
    if url.contains("://") {
        // Split on '#' to capture ref before URL parsing strips it
        let (url_without_ref, ref_from_hash) = if let Some(hash_pos) = url.find('#') {
            if hash_pos > 0 {
                let before = &url[..hash_pos];
                let after = &url[hash_pos + 1..];
                if !after.is_empty() {
                    (before, Some(after.to_string()))
                } else {
                    (url, None)
                }
            } else {
                (url, None)
            }
        } else {
            (url, None)
        };

        if let Ok(parsed) = url::Url::parse(url_without_ref) {
            let path_with_maybe_ref = parsed.path().trim_start_matches('/');
            if let Some(ref_sep) = path_with_maybe_ref.find('@') {
                let repo_path = &path_with_maybe_ref[..ref_sep];
                let ref_str = &path_with_maybe_ref[ref_sep + 1..];
                if !repo_path.is_empty() && !ref_str.is_empty() {
                    let clean_path = format!("/{repo_path}");
                    let clean_url = url::Url::parse_with_params(
                        &format!(
                            "{}://{}{}",
                            parsed.scheme(),
                            parsed.host_str().unwrap_or(""),
                            clean_path
                        ),
                        parsed
                            .query_pairs()
                            .map(|(k, v)| (k.to_string(), v.to_string())),
                    )
                    .ok()
                    .map(|u| u.as_str().trim_end_matches('/').to_string())
                    .unwrap_or_else(|| {
                        format!(
                            "{}://{}{}",
                            parsed.scheme(),
                            parsed.host_str().unwrap_or(""),
                            clean_path
                        )
                    });

                    return (clean_url, Some(ref_str.to_string()));
                }
            }
        }
        return (url_without_ref.to_string(), ref_from_hash);
    }

    // host/path[@ref] form (no protocol)
    if let Some(slash_index) = url.find('/') {
        if slash_index > 0 {
            let host = &url[..slash_index];
            let path_with_maybe_ref = &url[slash_index + 1..];
            if let Some(ref_sep) = path_with_maybe_ref.find('@') {
                let repo_path = &path_with_maybe_ref[..ref_sep];
                let ref_str = &path_with_maybe_ref[ref_sep + 1..];
                if !repo_path.is_empty() && !ref_str.is_empty() {
                    return (format!("{host}/{repo_path}"), Some(ref_str.to_string()));
                }
            }
        }
    }

    (url.to_string(), None)
}

fn decode_for_validation(value: &str) -> Option<String> {
    urlencoding::decode(value).ok().map(|s| s.to_string())
}

fn has_unsafe_git_install_part(value: &str, allow_slash: bool) -> bool {
    let decoded = decode_for_validation(value);
    if decoded.is_none() {
        return true;
    }
    let decoded = decoded.unwrap();

    let candidates = [value, &decoded];
    for candidate in candidates {
        if candidate.contains('\0') || candidate.contains('\\') || candidate.starts_with('/') {
            return true;
        }
        if !allow_slash && candidate.contains('/') {
            return true;
        }
        if candidate.split('/').any(|part| part == "..") {
            return true;
        }
    }
    false
}

fn build_git_source(args: GitSourceArgs) -> Option<GitSource> {
    if args.path.starts_with('/') {
        return None;
    }
    let normalized_path = args
        .path
        .trim_end_matches(".git")
        .trim_start_matches('/')
        .to_string();

    if args.host.is_empty() || normalized_path.is_empty() {
        return None;
    }

    let path_segments: Vec<&str> = normalized_path.split('/').collect();
    if path_segments.len() < 2 {
        return None;
    }

    if has_unsafe_git_install_part(&args.host, false)
        || has_unsafe_git_install_part(&normalized_path, true)
    {
        return None;
    }

    let is_pinned = args.ref_.is_some();
    Some(GitSource {
        r#type: "git".to_string(),
        repo: args.repo,
        host: args.host,
        path: normalized_path,
        ref_: args.ref_,
        pinned: is_pinned,
    })
}

struct GitSourceArgs {
    repo: String,
    host: String,
    path: String,
    ref_: Option<String>,
}

fn parse_generic_git_url(url: &str) -> Option<GitSource> {
    let (repo_without_ref, ref_) = split_ref(url);

    let (host, path, repo) = if let Some(captures) = repo_without_ref.strip_prefix("git@") {
        // SCP-like: git@host:path
        let at_colon = captures.find(':')?;
        let host_part = &captures[..at_colon];
        let path_part = &captures[at_colon + 1..];
        (
            host_part.to_string(),
            path_part.to_string(),
            repo_without_ref,
        )
    } else if repo_without_ref.starts_with("https://")
        || repo_without_ref.starts_with("http://")
        || repo_without_ref.starts_with("ssh://")
        || repo_without_ref.starts_with("git://")
    {
        let parsed = url::Url::parse(&repo_without_ref).ok()?;
        let host_str = parsed.host_str()?.to_string();
        let path_str = parsed.path().trim_start_matches('/').to_string();
        (host_str, path_str, repo_without_ref)
    } else {
        let slash_index = repo_without_ref.find('/')?;
        let host_str = repo_without_ref[..slash_index].to_string();
        let path_str = repo_without_ref[slash_index + 1..].to_string();
        // Validate it looks like a host
        if !host_str.contains('.') && host_str != "localhost" {
            return None;
        }
        let repo_url = format!("https://{repo_without_ref}");
        (host_str, path_str, repo_url)
    };

    build_git_source(GitSourceArgs {
        repo,
        host,
        path,
        ref_,
    })
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parse a git source string into a `GitSource`.
///
/// Rules:
/// - With `git:` prefix, accept all historical shorthand forms.
/// - Without `git:` prefix, only accept explicit protocol URLs.
pub fn parse_git_url(source: &str) -> Option<GitSource> {
    let trimmed = source.trim();
    let has_git_prefix = trimmed.starts_with("git:");
    let url = if has_git_prefix {
        trimmed[4..].trim()
    } else {
        trimmed
    };

    if !has_git_prefix {
        let lower = url.to_lowercase();
        let has_protocol = lower.starts_with("https://")
            || lower.starts_with("http://")
            || lower.starts_with("ssh://")
            || lower.starts_with("git://");
        if !has_protocol {
            return None;
        }
    }

    let (_repo, _ref_) = split_ref(url);

    // Try hosted-git-info-style parsing for known hosts (GitHub, GitLab, etc.)
    // We implement the core patterns directly:

    // For GitHub-style: github:user/repo or github:user/repo#ref
    if let Some(rest) = url
        .strip_prefix("github:")
        .or_else(|| url.strip_prefix("github.com/"))
    {
        let rest = rest.trim_start_matches('/');
        let (path, ref_) = if let Some(hash_pos) = rest.find('#') {
            (&rest[..hash_pos], Some(rest[hash_pos + 1..].to_string()))
        } else {
            (rest, None)
        };
        // Remove trailing .git
        let path = path.trim_end_matches(".git");
        if let Some(slash_pos) = path.find('/') {
            let user = &path[..slash_pos];
            let project = &path[slash_pos + 1..];
            if !user.is_empty() && !project.is_empty() {
                return build_git_source(GitSourceArgs {
                    repo: format!("https://github.com/{user}/{project}"),
                    host: "github.com".to_string(),
                    path: format!("{user}/{project}"),
                    ref_,
                });
            }
        }
    }

    // For GitLab-style
    if let Some(rest) = url
        .strip_prefix("gitlab:")
        .or_else(|| url.strip_prefix("gitlab.com/"))
    {
        let rest = rest.trim_start_matches('/');
        let (path, ref_) = if let Some(hash_pos) = rest.find('#') {
            (&rest[..hash_pos], Some(rest[hash_pos + 1..].to_string()))
        } else {
            (rest, None)
        };
        let path = path.trim_end_matches(".git");
        if let Some(slash_pos) = path.find('/') {
            let user = &path[..slash_pos];
            let project = &path[slash_pos + 1..];
            if !user.is_empty() && !project.is_empty() {
                return build_git_source(GitSourceArgs {
                    repo: format!("https://gitlab.com/{user}/{project}"),
                    host: "gitlab.com".to_string(),
                    path: format!("{user}/{project}"),
                    ref_,
                });
            }
        }
    }

    // For Bitbucket-style
    if let Some(rest) = url
        .strip_prefix("bitbucket:")
        .or_else(|| url.strip_prefix("bitbucket.org/"))
    {
        let rest = rest.trim_start_matches('/');
        let (path, ref_) = if let Some(hash_pos) = rest.find('#') {
            (&rest[..hash_pos], Some(rest[hash_pos + 1..].to_string()))
        } else {
            (rest, None)
        };
        let path = path.trim_end_matches(".git");
        if let Some(slash_pos) = path.find('/') {
            let user = &path[..slash_pos];
            let project = &path[slash_pos + 1..];
            if !user.is_empty() && !project.is_empty() {
                return build_git_source(GitSourceArgs {
                    repo: format!("https://bitbucket.org/{user}/{project}"),
                    host: "bitbucket.org".to_string(),
                    path: format!("{user}/{project}"),
                    ref_,
                });
            }
        }
    }

    // Generic parsing
    parse_generic_git_url(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_github_https() {
        let result = parse_git_url("https://github.com/user/repo.git");
        assert!(result.is_some());
        let gs = result.unwrap();
        assert_eq!(gs.host, "github.com");
        assert_eq!(gs.path, "user/repo");
        assert!(!gs.pinned);
    }

    #[test]
    fn test_github_with_ref() {
        let result = parse_git_url("https://github.com/user/repo.git#v1.0");
        assert!(result.is_some());
        let gs = result.unwrap();
        assert_eq!(gs.host, "github.com");
        assert_eq!(gs.path, "user/repo");
        assert_eq!(gs.ref_.as_deref(), Some("v1.0"));
        assert!(gs.pinned);
    }

    #[test]
    fn test_git_prefix() {
        let result = parse_git_url("git:github.com/user/repo");
        assert!(result.is_some());
        let gs = result.unwrap();
        assert_eq!(gs.host, "github.com");
    }

    #[test]
    fn test_no_protocol_without_prefix() {
        // Without git: prefix, no protocol means None
        assert!(parse_git_url("github.com/user/repo").is_none());
    }

    #[test]
    fn test_scp_like() {
        // SCP-like requires git: prefix or explicit protocol
        let result = parse_git_url("git:git@github.com:user/repo.git");
        assert!(result.is_some());
        let gs = result.unwrap();
        assert_eq!(gs.host, "github.com");
    }

    #[test]
    fn test_invalid_path() {
        assert!(parse_git_url("https://github.com/").is_none());
    }

    #[test]
    fn test_empty() {
        assert!(parse_git_url("").is_none());
    }

    #[test]
    fn test_ssh_url() {
        let result = parse_git_url("ssh://git@github.com/user/repo");
        assert!(result.is_some());
        let gs = result.unwrap();
        assert_eq!(gs.host, "github.com");
        assert_eq!(gs.path, "user/repo");
        assert_eq!(gs.repo, "ssh://git@github.com/user/repo");
    }

    #[test]
    fn test_https_url_with_ref_via_at() {
        let result = parse_git_url("https://github.com/user/repo@v1.0.0");
        assert!(result.is_some());
        let gs = result.unwrap();
        assert_eq!(gs.host, "github.com");
        assert_eq!(gs.path, "user/repo");
        assert_eq!(gs.ref_.as_deref(), Some("v1.0.0"));
        assert!(gs.pinned);
    }

    #[test]
    fn test_git_prefix_scp_with_ref() {
        let result = parse_git_url("git:git@github.com:user/repo@v1.0.0");
        assert!(result.is_some());
        let gs = result.unwrap();
        assert_eq!(gs.host, "github.com");
        assert_eq!(gs.path, "user/repo");
        assert_eq!(gs.ref_.as_deref(), Some("v1.0.0"));
        assert_eq!(gs.repo, "git@github.com:user/repo");
    }

    #[test]
    fn test_git_prefix_host_path() {
        let result = parse_git_url("git:github.com/user/repo");
        assert!(result.is_some());
        let gs = result.unwrap();
        assert_eq!(gs.host, "github.com");
        assert_eq!(gs.path, "user/repo");
        assert_eq!(gs.repo, "https://github.com/user/repo");
    }

    #[test]
    fn test_reject_unsafe_path_traversal() {
        assert!(parse_git_url("git:git@evil.example:../../victim/repo").is_none());
    }

    #[test]
    fn test_reject_unsafe_absolute_path() {
        assert!(parse_git_url("git:git@evil.example:/absolute/repo").is_none());
    }

    #[test]
    fn test_reject_unsafe_backslash() {
        assert!(parse_git_url("git:git@evil.example:user\\repo/name").is_none());
    }

    #[test]
    fn test_reject_git_scp_without_prefix() {
        assert!(parse_git_url("git@github.com:user/repo").is_none());
    }

    #[test]
    fn test_reject_bare_user_repo() {
        assert!(parse_git_url("user/repo").is_none());
    }

    #[test]
    fn test_bitbucket_url() {
        let result = parse_git_url("git:bitbucket.org/user/repo");
        assert!(result.is_some());
        let gs = result.unwrap();
        assert_eq!(gs.host, "bitbucket.org");
        assert_eq!(gs.path, "user/repo");
    }

    #[test]
    fn test_gitlab_url() {
        let result = parse_git_url("https://gitlab.com/user/repo.git");
        assert!(result.is_some());
        let gs = result.unwrap();
        assert_eq!(gs.host, "gitlab.com");
        assert_eq!(gs.path, "user/repo");
    }
}
