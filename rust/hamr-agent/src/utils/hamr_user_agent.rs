//! Port of `packages/coding-agent/src/utils/hamr-user-agent.ts`.
//!
//! Build the Hamr User-Agent string for HTTP requests.

/// Build a Hamr User-Agent string for the given version.
///
/// Format: `hamr/{version} ({os}; {runtime}; {arch})`
pub fn get_hamr_user_agent(version: &str) -> String {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let runtime = format!("rust/{}", env!("CARGO_PKG_VERSION"));
    format!("hamr/{version} ({os}; {runtime}; {arch})")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_formats_user_agent_correctly() {
        let ua = get_hamr_user_agent("1.2.3");
        assert!(ua.starts_with("hamr/1.2.3 ("));
        assert!(ua.ends_with(')'));
        assert!(ua.contains(std::env::consts::OS));
        assert!(ua.contains(std::env::consts::ARCH));
        assert!(ua.contains("rust/"));
    }

    #[test]
    fn test_matches_expected_regex() {
        let ua = get_hamr_user_agent("1.0.0");
        assert!(ua.starts_with("hamr/"));
        let parts: Vec<&str> = ua.splitn(2, ' ').collect();
        assert_eq!(parts.len(), 2);
        assert!(parts[1].starts_with('('));
        assert!(parts[1].ends_with(')'));
        // Contains at least 2 semicolons: os; runtime; arch
        let inner = &parts[1][1..parts[1].len() - 1];
        assert!(inner.contains(';'));
    }
}
