//! Session search query parsing and matching utilities.
//!
//! Port of `packages/coding-agent/src/modes/interactive/components/session-selector-search.ts`.

/// Sort mode for session listings.
#[derive(Clone, Copy, PartialEq)]
pub enum SortMode {
    Threaded,
    Recent,
    Relevance,
}

/// Filter for named vs all sessions.
#[derive(Clone, Copy, PartialEq)]
pub enum NameFilter {
    All,
    Named,
}

/// A parsed search query for session filtering.
pub struct ParsedSearchQuery {
    pub mode: ParseMode,
    pub tokens: Vec<SearchToken>,
    pub regex: Option<regex::Regex>,
    /// If set, parsing failed and the query should be treated as non-matching.
    pub error: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum ParseMode {
    Tokens,
    Regex,
}

/// A single search token.
#[derive(Clone)]
pub struct SearchToken {
    pub kind: TokenKind,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Fuzzy,
    Phrase,
}

/// Result of matching a session against a search query.
pub struct MatchResult {
    pub matches: bool,
    /// Lower is better; only meaningful when matches == true
    pub score: f64,
}

/// Minimal session info needed for search and display.
#[derive(Clone)]
pub struct SessionInfo {
    pub id: String,
    pub name: Option<String>,
    pub all_messages_text: String,
    pub cwd: Option<String>,
    pub modified: i64, // timestamp in milliseconds
    pub path: String,
    pub message_count: usize,
    pub first_message: String,
    pub parent_session_path: Option<String>,
}

fn normalize_whitespace_lower(text: &str) -> String {
    text.to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn get_session_search_text(session: &SessionInfo) -> String {
    format!(
        "{} {} {} {}",
        session.id,
        session.name.as_deref().unwrap_or(""),
        session.all_messages_text,
        session.cwd.as_deref().unwrap_or("")
    )
}

/// Check whether a session has a name.
pub fn has_session_name(session: &SessionInfo) -> bool {
    session
        .name
        .as_ref()
        .map_or(false, |n| !n.trim().is_empty())
}

fn matches_name_filter(session: &SessionInfo, filter: NameFilter) -> bool {
    match filter {
        NameFilter::All => true,
        NameFilter::Named => has_session_name(session),
    }
}

/// Parse a search query into a ParsedSearchQuery.
///
/// Supports two modes:
/// - Regex mode: `re:<pattern>`
/// - Token mode with quote support: `foo "exact phrase" bar`
pub fn parse_search_query(query: &str) -> ParsedSearchQuery {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return ParsedSearchQuery {
            mode: ParseMode::Tokens,
            tokens: vec![],
            regex: None,
            error: None,
        };
    }

    // Regex mode: re:<pattern>
    if trimmed.starts_with("re:") {
        let pattern = &trimmed[3..].trim();
        if pattern.is_empty() {
            return ParsedSearchQuery {
                mode: ParseMode::Regex,
                tokens: vec![],
                regex: None,
                error: Some("Empty regex".to_string()),
            };
        }
        match regex::Regex::new(&format!("(?i){}", pattern)) {
            Ok(re) => ParsedSearchQuery {
                mode: ParseMode::Regex,
                tokens: vec![],
                regex: Some(re),
                error: None,
            },
            Err(err) => ParsedSearchQuery {
                mode: ParseMode::Regex,
                tokens: vec![],
                regex: None,
                error: Some(err.to_string()),
            },
        }
    } else {
        // Token mode with quote support
        let mut tokens: Vec<SearchToken> = vec![];
        let mut buf = String::new();
        let mut in_quote = false;
        let mut had_unclosed_quote = false;

        for ch in trimmed.chars() {
            if ch == '"' {
                if in_quote {
                    flush_buffer(&mut buf, TokenKind::Phrase, &mut tokens);
                    in_quote = false;
                } else {
                    flush_buffer(&mut buf, TokenKind::Fuzzy, &mut tokens);
                    in_quote = true;
                }
                continue;
            }

            if !in_quote && ch.is_whitespace() {
                flush_buffer(&mut buf, TokenKind::Fuzzy, &mut tokens);
                continue;
            }

            buf.push(ch);
        }

        if in_quote {
            had_unclosed_quote = true;
        }

        // If quotes were unbalanced, fall back to plain whitespace tokenization
        if had_unclosed_quote {
            return ParsedSearchQuery {
                mode: ParseMode::Tokens,
                tokens: trimmed
                    .split_whitespace()
                    .map(|t| t.trim())
                    .filter(|t| !t.is_empty())
                    .map(|t| SearchToken {
                        kind: TokenKind::Fuzzy,
                        value: t.to_string(),
                    })
                    .collect(),
                regex: None,
                error: None,
            };
        }

        flush_buffer(
            &mut buf,
            if in_quote {
                TokenKind::Phrase
            } else {
                TokenKind::Fuzzy
            },
            &mut tokens,
        );

        ParsedSearchQuery {
            mode: ParseMode::Tokens,
            tokens,
            regex: None,
            error: None,
        }
    }
}

fn flush_buffer(buf: &mut String, kind: TokenKind, tokens: &mut Vec<SearchToken>) {
    let v = buf.trim().to_string();
    buf.clear();
    if v.is_empty() {
        return;
    }
    tokens.push(SearchToken { kind, value: v });
}

/// Match a session against a parsed search query.
pub fn match_session(session: &SessionInfo, parsed: &ParsedSearchQuery) -> MatchResult {
    let text = get_session_search_text(session);

    if parsed.mode == ParseMode::Regex {
        match &parsed.regex {
            Some(re) => {
                if let Some(m) = re.find(&text) {
                    MatchResult {
                        matches: true,
                        score: m.start() as f64 * 0.1,
                    }
                } else {
                    MatchResult {
                        matches: false,
                        score: 0.0,
                    }
                }
            }
            None => MatchResult {
                matches: false,
                score: 0.0,
            },
        }
    } else {
        if parsed.tokens.is_empty() {
            return MatchResult {
                matches: true,
                score: 0.0,
            };
        }

        let mut total_score = 0.0;
        let mut normalized_text: Option<String> = None;

        for token in &parsed.tokens {
            if token.kind == TokenKind::Phrase {
                if normalized_text.is_none() {
                    normalized_text = Some(normalize_whitespace_lower(&text));
                }
                let phrase = normalize_whitespace_lower(&token.value);
                if phrase.is_empty() {
                    continue;
                }
                let nt = normalized_text.as_ref().unwrap();
                match nt.find(&phrase) {
                    Some(idx) => {
                        total_score += idx as f64 * 0.1;
                    }
                    None => {
                        return MatchResult {
                            matches: false,
                            score: 0.0,
                        };
                    }
                }
            } else {
                // Fuzzy match approximation: case-insensitive substring search
                let lower_text = text.to_lowercase();
                let lower_query = token.value.to_lowercase();
                match lower_text.find(&lower_query) {
                    Some(idx) => {
                        total_score += idx as f64;
                    }
                    None => {
                        return MatchResult {
                            matches: false,
                            score: 0.0,
                        };
                    }
                }
            }
        }

        MatchResult {
            matches: true,
            score: total_score,
        }
    }
}

/// Filter and sort sessions based on query, sort mode, and name filter.
pub fn filter_and_sort_sessions(
    sessions: &[SessionInfo],
    query: &str,
    sort_mode: SortMode,
    name_filter: NameFilter,
) -> Vec<SessionInfo> {
    let name_filtered: Vec<SessionInfo> = sessions
        .iter()
        .filter(|s| matches_name_filter(s, name_filter))
        .cloned()
        .collect();

    let trimmed = query.trim();
    if trimmed.is_empty() {
        return name_filtered;
    }

    let parsed = parse_search_query(query);
    if parsed.error.is_some() {
        return vec![];
    }

    // Recent mode: filter only, keep incoming order
    if sort_mode == SortMode::Recent {
        let mut filtered = Vec::new();
        for s in &name_filtered {
            let res = match_session(s, &parsed);
            if res.matches {
                filtered.push(s.clone());
            }
        }
        return filtered;
    }

    // Relevance mode: sort by score, tie-break by modified desc
    let mut scored: Vec<(SessionInfo, f64)> = Vec::new();
    for s in &name_filtered {
        let res = match_session(s, &parsed);
        if !res.matches {
            continue;
        }
        scored.push((s.clone(), res.score));
    }

    scored.sort_by(|a, b| {
        let score_cmp = a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal);
        if score_cmp != std::cmp::Ordering::Equal {
            return score_cmp;
        }
        // Tie-break: more recent (higher modified timestamp) first
        b.0.modified.cmp(&a.0.modified)
    });

    scored.into_iter().map(|(s, _)| s).collect()
}

impl SessionInfo {}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_session(name: Option<&str>, text: &str, modified: i64) -> SessionInfo {
        SessionInfo {
            id: format!("id-{}", text),
            name: name.map(|s| s.to_string()),
            all_messages_text: text.to_string(),
            cwd: None,
            modified,
            path: format!("/tmp/{}", text),
            message_count: 3,
            first_message: text.to_string(),
            parent_session_path: None,
        }
    }

    #[test]
    fn test_parse_empty_query() {
        let parsed = parse_search_query("");
        assert_eq!(parsed.mode, ParseMode::Tokens);
        assert!(parsed.tokens.is_empty());
    }

    #[test]
    fn test_parse_regex_mode() {
        let parsed = parse_search_query("re:hello");
        assert_eq!(parsed.mode, ParseMode::Regex);
        assert!(parsed.regex.is_some());
    }

    #[test]
    fn test_parse_regex_empty() {
        let parsed = parse_search_query("re:");
        assert!(parsed.error.is_some());
    }

    #[test]
    fn test_parse_token_mode() {
        let parsed = parse_search_query("foo bar");
        assert_eq!(parsed.mode, ParseMode::Tokens);
        assert_eq!(parsed.tokens.len(), 2);
    }

    #[test]
    fn test_parse_quoted_phrase() {
        let parsed = parse_search_query(r#"foo "exact phrase" bar"#);
        assert_eq!(parsed.tokens.len(), 3);
        assert_eq!(parsed.tokens[0].kind, TokenKind::Fuzzy);
        assert_eq!(parsed.tokens[1].kind, TokenKind::Phrase);
        assert_eq!(parsed.tokens[1].value, "exact phrase");
    }

    #[test]
    fn test_parse_unclosed_quote() {
        let parsed = parse_search_query(r#"foo "unclosed"#);
        // Falls back to whitespace tokenization
        assert_eq!(parsed.tokens.len(), 2);
        assert_eq!(parsed.tokens[0].kind, TokenKind::Fuzzy);
    }

    #[test]
    fn test_match_session_fuzzy() {
        let session = make_session(Some("test"), "hello world programming", 1000);
        let parsed = parse_search_query("hello");
        let result = match_session(&session, &parsed);
        assert!(result.matches);
    }

    #[test]
    fn test_match_session_no_match() {
        let session = make_session(Some("test"), "hello world", 1000);
        let parsed = parse_search_query("xyzzy");
        let result = match_session(&session, &parsed);
        assert!(!result.matches);
    }

    #[test]
    fn test_match_session_phrase() {
        let session = make_session(Some("test"), "hello world foo bar", 1000);
        let parsed = parse_search_query(r#""hello world""#);
        let result = match_session(&session, &parsed);
        assert!(result.matches);
    }

    #[test]
    fn test_has_session_name() {
        let s = make_session(Some("my session"), "text", 0);
        assert!(has_session_name(&s));
    }

    #[test]
    fn test_has_session_name_empty() {
        let s = make_session(None, "text", 0);
        assert!(!has_session_name(&s));
    }

    // --- name filter tests (port of session-selector-search.test.ts) ---

    #[test]
    fn test_name_filter_all_returns_all() {
        let sessions = vec![
            make_session(Some("My Project"), "blueberry", 1000),
            make_session(Some("Another"), "blueberry", 900),
            make_session(None, "blueberry", 800),
            make_session(None, "blueberry", 700),
        ];
        let result = filter_and_sort_sessions(&sessions, "", SortMode::Recent, NameFilter::All);
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn test_name_filter_named_returns_named_only() {
        let sessions = vec![
            make_session(Some("My Project"), "blueberry", 1000),
            make_session(Some("Another"), "blueberry", 900),
            make_session(None, "blueberry", 800),
        ];
        let result = filter_and_sort_sessions(&sessions, "", SortMode::Recent, NameFilter::Named);
        assert_eq!(result.len(), 2);
        for s in &result {
            assert!(s.name.is_some());
        }
    }

    #[test]
    fn test_name_filter_applied_before_search() {
        let sessions = vec![
            make_session(Some("My Project"), "blueberry", 1000),
            make_session(None, "blueberry", 900),
        ];
        let result =
            filter_and_sort_sessions(&sessions, "blueberry", SortMode::Recent, NameFilter::Named);
        assert_eq!(result.len(), 1);
        assert!(result[0].name.is_some());
    }

    #[test]
    fn test_name_filter_excludes_whitespace_only_names() {
        let sessions = vec![
            make_session(Some("   "), "test", 1000),
            make_session(Some(""), "test", 900),
            make_session(Some("Real Name"), "test", 800),
        ];
        let result = filter_and_sort_sessions(&sessions, "", SortMode::Recent, NameFilter::Named);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name.as_deref(), Some("Real Name"));
    }

    // --- regex mode tests ---

    #[test]
    fn test_regex_filter_case_insensitive() {
        let session = make_session(None, "Brave is great", 1000);
        let parsed = parse_search_query("re:\\bbrave\\b");
        let result = match_session(&session, &parsed);
        assert!(result.matches);
    }

    #[test]
    fn test_regex_no_match() {
        let session = make_session(None, "bravery is not the same", 1000);
        let parsed = parse_search_query("re:\\bbrave\\b");
        let result = match_session(&session, &parsed);
        assert!(!result.matches);
    }

    #[test]
    fn test_recent_sort_preserves_input_order() {
        let sessions = vec![
            make_session(None, "brave", 3000),
            make_session(None, "brave", 1000),
            make_session(None, "something else", 2000),
        ];
        let result =
            filter_and_sort_sessions(&sessions, "brave", SortMode::Recent, NameFilter::All);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].modified, 3000);
        assert_eq!(result[1].modified, 1000);
    }

    #[test]
    fn test_relevance_sort_by_score() {
        let sessions = vec![
            make_session(None, "xxxx brave", 3000),
            make_session(None, "brave xxxx", 1000),
        ];
        let result =
            filter_and_sort_sessions(&sessions, "brave", SortMode::Relevance, NameFilter::All);
        // "brave xxxx" should have lower score (matches at position 0)
        assert_eq!(result[0].modified, 1000);
    }

    #[test]
    fn test_relevance_tie_break_by_modified() {
        let sessions = vec![
            make_session(None, "brave", 3000),
            make_session(None, "brave", 1000),
        ];
        let result =
            filter_and_sort_sessions(&sessions, "brave", SortMode::Relevance, NameFilter::All);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].modified, 3000);
        assert_eq!(result[1].modified, 1000);
    }

    #[test]
    fn test_invalid_regex_returns_empty() {
        let sessions = vec![make_session(None, "brave", 1000)];
        let result = filter_and_sort_sessions(&sessions, "re:(", SortMode::Recent, NameFilter::All);
        assert!(result.is_empty());
    }

    #[test]
    fn test_empty_query_returns_all_filtered() {
        let sessions = vec![
            make_session(Some("a"), "hello", 1000),
            make_session(None, "world", 900),
        ];
        let result = filter_and_sort_sessions(&sessions, "", SortMode::Recent, NameFilter::Named);
        assert_eq!(result.len(), 1);
    }
}
