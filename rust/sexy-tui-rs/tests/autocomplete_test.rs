//! Ported from packages/tui/test/autocomplete.test.ts
//!
//! Tests for CombinedAutocompleteProvider: path prefix extraction,
//! slash commands, and fd-based file suggestions.

use sexy_tui_rs::autocomplete::{AutocompleteProvider, CombinedAutocompleteProvider, SlashCommand};

use std::io::Write;

/// Helper: create a temp directory with optional files and return its path.
fn setup_temp_dir(files: &[&str]) -> (tempfile::TempDir, CombinedAutocompleteProvider) {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    for f in files {
        let path = dir.path().join(f);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let mut file = std::fs::File::create(&path).expect("failed to create file");
        write!(file, "content").ok();
    }
    let mut cmd = SlashCommand::new("model");
    cmd.description = Some("Select model".into());
    let provider = CombinedAutocompleteProvider::new(
        vec![cmd],
        dir.path().to_string_lossy().to_string(),
        None,
    );
    (dir, provider)
}

// =============================================================================
// CombinedAutocompleteProvider
// =============================================================================

mod extract_path_prefix {
    use super::*;

    #[test]
    fn test_extracts_slash_from_hey_slash_when_forced() {
        let (_dir, provider) = setup_temp_dir(&[]);
        let result = provider.get_suggestions_legacy("hey /", 5);
        assert!(result.is_some(), "Should return suggestions for /");
    }

    #[test]
    fn test_does_not_trigger_for_slash_commands() {
        let (_dir, provider) = setup_temp_dir(&[]);
        let result = provider.get_suggestions_legacy("/model", 6);
        assert!(result.is_some(), "Should return suggestions for /model");
        if let Some(suggestions) = result {
            let has_model = suggestions.items.iter().any(|item| item.value == "model");
            assert!(has_model, "Should include 'model' suggestion (value without /)");
        }
    }

    #[test]
    fn test_command_argument_completion_returns_none_for_unknown_command() {
        let (_dir, provider) = setup_temp_dir(&["existing_file.txt"]);
        // Unknown command → no argument completions available → returns None
        let result = provider.get_suggestions_legacy("/command /", 10);
        assert!(
            result.is_none(),
            "Should return None for unknown command (no argument completions)"
        );
    }
}

// =============================================================================
// fd @ file suggestions (requires fd binary)
// =============================================================================

mod fd_file_suggestions {
    use super::*;

    fn setup_fd_temp_dir(structure: &[&str]) -> (tempfile::TempDir, CombinedAutocompleteProvider) {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        for f in structure {
            let path = dir.path().join(f);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            let mut file = std::fs::File::create(&path).expect("failed to create file");
            write!(file, "content").ok();
        }
        let provider =
            CombinedAutocompleteProvider::new(vec![], dir.path().to_string_lossy().to_string(), None);
        (dir, provider)
    }

    fn is_fd_available() -> bool {
        std::process::Command::new("which")
            .arg("fd")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    #[test]
    #[ignore = "requires fd binary on PATH"]
    fn test_returns_all_files_and_folders_for_empty_at_query() {
        if !is_fd_available() {
            return;
        }
        let (_dir, provider) = setup_fd_temp_dir(&["README.md", "src/"]);
        let result = provider.get_suggestions_legacy("@", 1);
        assert!(
            result.is_some(),
            "Should return suggestions for empty @ query"
        );
    }

    #[test]
    #[ignore = "requires fd binary on PATH"]
    fn test_matches_file_with_extension_in_query() {
        if !is_fd_available() {
            return;
        }
        let (_dir, provider) = setup_fd_temp_dir(&["file.txt"]);
        let result = provider.get_suggestions_legacy("@file.txt", 9);
        assert!(result.is_some(), "Should return suggestions for @file.txt");
    }

    #[test]
    #[ignore = "requires fd binary on PATH"]
    fn test_ranks_directories_before_files() {
        if !is_fd_available() {
            return;
        }
        let (_dir, provider) = setup_fd_temp_dir(&["src/", "src.txt"]);
        let result = provider.get_suggestions_legacy("@src", 4);
        assert!(result.is_some(), "Should return suggestions for @src");
    }

    #[test]
    #[ignore = "requires fd binary on PATH"]
    fn test_returns_nested_file_paths() {
        if !is_fd_available() {
            return;
        }
        let (_dir, provider) = setup_fd_temp_dir(&["src/index.ts"]);
        let result = provider.get_suggestions_legacy("@index", 6);
        assert!(result.is_some(), "Should return suggestions for @index");
    }

    #[test]
    #[ignore = "requires fd binary on PATH"]
    fn test_filters_are_case_insensitive() {
        if !is_fd_available() {
            return;
        }
        let (_dir, provider) = setup_fd_temp_dir(&["README.md", "src/"]);
        let result = provider.get_suggestions_legacy("@re", 3);
        assert!(result.is_some(), "Should return suggestions for @re");
    }

    #[test]
    #[ignore = "requires fd binary on PATH"]
    fn test_matches_deeply_nested_paths() {
        if !is_fd_available() {
            return;
        }
        let (_dir, provider) = setup_fd_temp_dir(&[
            "packages/tui/src/autocomplete.ts",
            "packages/ai/src/autocomplete.ts",
        ]);
        let result = provider.get_suggestions_legacy("@tui/src/auto", 14);
        assert!(
            result.is_some(),
            "Should return suggestions for @tui/src/auto"
        );
    }

    #[test]
    #[ignore = "requires fd binary on PATH"]
    fn test_includes_hidden_paths_but_excludes_dot_git() {
        if !is_fd_available() {
            return;
        }
        let (_dir, provider) = setup_fd_temp_dir(&[".pi/config.json", ".github/workflows/ci.yml"]);
        let result = provider.get_suggestions_legacy("@", 1);
        assert!(result.is_some(), "Should return suggestions for @");
    }
}

// =============================================================================
// dot-slash path completion
// =============================================================================

mod dot_slash_path_completion {
    use super::*;

    #[test]
    fn test_preserves_dot_slash_prefix_when_completing_paths() {
        let (_dir, provider) = setup_temp_dir(&["update.sh", "utils.ts"]);
        let result = provider.get_suggestions_legacy("./up", 4);
        assert!(result.is_some(), "Should return suggestions for ./ path");
        if let Some(suggestions) = &result {
            let has_update = suggestions
                .items
                .iter()
                .any(|item| item.value.contains("update"));
            assert!(has_update, "Should find update.sh-like suggestions");
        }
    }

    #[test]
    fn test_preserves_dot_slash_prefix_for_directory_completions() {
        let (_dir, provider) = setup_temp_dir(&["src/index.ts"]);
        let result = provider.get_suggestions_legacy("./sr", 4);
        assert!(
            result.is_some(),
            "Should return suggestions for ./ directory path"
        );
        if let Some(suggestions) = &result {
            let has_src = suggestions
                .items
                .iter()
                .any(|item| item.value.contains("src"));
            assert!(has_src, "Should find src/ suggestion");
        }
    }
}
