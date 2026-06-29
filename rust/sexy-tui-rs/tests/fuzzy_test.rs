//! Ported from packages/tui/test/fuzzy.test.ts
//!
//! Tests for fuzzy matching and filtering.

use sexy_tui_rs::{fuzzy_filter, fuzzy_match};

// =============================================================================
// fuzzyMatch
// =============================================================================

mod fuzzy_match_tests {
    use super::*;

    #[test]
    fn test_empty_query_matches_everything_with_score_0() {
        let result = fuzzy_match("", "anything");
        assert!(result.matches);
        assert_eq!(result.score, 0.0);
    }

    #[test]
    fn test_query_longer_than_text_does_not_match() {
        let result = fuzzy_match("longquery", "short");
        assert!(!result.matches);
    }

    #[test]
    fn test_exact_match_has_good_score() {
        let result = fuzzy_match("test", "test");
        assert!(result.matches);
        assert!(result.score < 0.0); // Should be negative due to consecutive bonuses
    }

    #[test]
    fn test_characters_must_appear_in_order() {
        let match_in_order = fuzzy_match("abc", "aXbXc");
        assert!(match_in_order.matches);

        let match_out_of_order = fuzzy_match("abc", "cba");
        assert!(!match_out_of_order.matches);
    }

    #[test]
    fn test_case_insensitive_matching() {
        let result = fuzzy_match("ABC", "abc");
        assert!(result.matches);

        let result2 = fuzzy_match("abc", "ABC");
        assert!(result2.matches);
    }

    #[test]
    fn test_consecutive_matches_score_better_than_scattered() {
        let consecutive = fuzzy_match("foo", "foobar");
        let scattered = fuzzy_match("foo", "f_o_o_bar");

        assert!(consecutive.matches);
        assert!(scattered.matches);
        assert!(consecutive.score < scattered.score);
    }

    #[test]
    fn test_word_boundary_matches_score_better() {
        let at_boundary = fuzzy_match("fb", "foo-bar");
        let not_at_boundary = fuzzy_match("fb", "afbx");

        assert!(at_boundary.matches);
        assert!(not_at_boundary.matches);
        assert!(at_boundary.score < not_at_boundary.score);
    }

    #[test]
    fn test_matches_swapped_alpha_numeric_tokens() {
        let result = fuzzy_match("codex52", "gpt-5.2-codex");
        assert!(result.matches);
    }
}

// =============================================================================
// fuzzyFilter
// =============================================================================

mod fuzzy_filter_tests {
    use super::*;

    #[test]
    fn test_empty_query_returns_all_items_unchanged() {
        let items = vec!["apple", "banana", "cherry"];
        let result = fuzzy_filter(&items, "", |x| x.to_string());
        assert_eq!(result, items);
    }

    #[test]
    fn test_filters_out_non_matching_items() {
        let items = vec!["apple", "banana", "cherry"];
        let result = fuzzy_filter(&items, "an", |x| x.to_string());
        assert!(result.contains(&"banana"));
        assert!(!result.contains(&"apple"));
        assert!(!result.contains(&"cherry"));
    }

    #[test]
    fn test_sorts_results_by_match_quality() {
        let items = vec!["a_p_p", "app", "application"];
        let result = fuzzy_filter(&items, "app", |x| x.to_string());

        // "app" should be first (exact consecutive match at start)
        assert_eq!(result[0], "app");
    }

    #[test]
    fn test_prioritizes_exact_matches_over_longer_prefix_matches() {
        let items = vec!["clone", "cl"];
        let result = fuzzy_filter(&items, "cl", |x| x.to_string());

        assert_eq!(result, vec!["cl", "clone"]);
    }

    #[test]
    fn test_works_with_custom_gettext_function() {
        #[derive(Clone, Debug, PartialEq)]
        struct NamedItem {
            name: &'static str,
            id: u8,
        }
        let items = vec![
            NamedItem { name: "foo", id: 1 },
            NamedItem { name: "bar", id: 2 },
            NamedItem {
                name: "foobar",
                id: 3,
            },
        ];
        let result = fuzzy_filter(&items, "foo", |item| item.name.to_string());

        assert_eq!(result.len(), 2);
        assert!(result.iter().any(|r| r.name == "foo"));
        assert!(result.iter().any(|r| r.name == "foobar"));
    }

    #[test]
    fn test_matches_slash_separated_provider_model_queries() {
        #[derive(Clone, Debug, PartialEq)]
        struct Model {
            id: &'static str,
            provider: &'static str,
        }
        let item = Model {
            id: "gpt-5.5",
            provider: "openai-codex",
        };
        let result = fuzzy_filter(&[item.clone()], "openai-codex/gpt-5.5", |m| {
            format!("{} {}", m.id, m.provider)
        });

        assert_eq!(result, vec![item]);
    }
}
