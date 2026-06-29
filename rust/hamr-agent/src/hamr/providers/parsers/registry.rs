//! Tool-call parser registry.
//!
//! Maps parser IDs to parser implementations. Supports registration,
//! lookup, listing, and parse dispatch. Designed to mirror vLLM's
//! ToolParserManager but for Hamr's Rust runtime.

use std::collections::HashMap;

use super::types::{
    ParserInfo, ToolCallParseResult, ToolCallParser, ToolCallParserFactory, ToolCallParserRegistry,
};
use super::utils::sanitize_reasoning_tags;

// ─── Default registry implementation ─────────────────────

/// A simple registry backed by a `HashMap<String, ToolCallParserFactory>`.
#[derive(Default)]
pub struct DefaultToolCallParserRegistry {
    parsers: HashMap<String, ToolCallParserFactory>,
}

impl DefaultToolCallParserRegistry {
    pub fn new() -> Self {
        Self {
            parsers: HashMap::new(),
        }
    }
}

impl ToolCallParserRegistry for DefaultToolCallParserRegistry {
    fn register(&mut self, id: &str, factory: ToolCallParserFactory) {
        let normalized = id.trim().to_lowercase();
        if normalized.is_empty() {
            panic!("parser id must not be empty");
        }
        self.parsers.insert(normalized, factory);
    }

    fn get(&self, id: &str) -> Option<Box<dyn ToolCallParser>> {
        let factory = self.parsers.get(id.trim().to_lowercase().as_str())?;
        Some(factory())
    }

    fn list_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.parsers.keys().cloned().collect();
        ids.sort();
        ids
    }

    fn list_parsers(&self) -> Vec<ParserInfo> {
        let mut infos: Vec<ParserInfo> = self
            .parsers
            .iter()
            .map(|(id, factory)| {
                let parser = factory();
                ParserInfo {
                    id: id.clone(),
                    description: parser.description().to_string(),
                    model_families: parser
                        .model_families()
                        .iter()
                        .map(|s| s.to_string())
                        .collect(),
                }
            })
            .collect();
        infos.sort_by(|a, b| a.id.cmp(&b.id));
        infos
    }

    fn parse(&self, id: &str, content: &str) -> ToolCallParseResult {
        let parser = match self.get(id) {
            Some(p) => p,
            None => {
                return ToolCallParseResult::err(
                    id,
                    content,
                    format!(
                        "unknown parser: \"{}\". Available: {}",
                        id,
                        self.list_ids().join(", ")
                    ),
                );
            }
        };
        let sanitized = sanitize_reasoning_tags(content);
        parser.parse(&sanitized)
    }
}

/// Global parser map, shared by all registry functions.
static GLOBAL_PARSERS: std::sync::LazyLock<
    std::sync::Mutex<HashMap<String, ToolCallParserFactory>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(HashMap::new()));

/// Register a parser factory globally.
pub fn register_tool_call_parser(id: &str, factory: ToolCallParserFactory) {
    let normalized = id.trim().to_lowercase();
    if normalized.is_empty() {
        panic!("parser id must not be empty");
    }
    GLOBAL_PARSERS.lock().unwrap().insert(normalized, factory);
}

/// Get a parser from the global registry.
pub fn get_tool_call_parser(id: &str) -> Option<Box<dyn ToolCallParser>> {
    let factory = GLOBAL_PARSERS
        .lock()
        .unwrap()
        .get(id.trim().to_lowercase().as_str())
        .copied()?;
    Some(factory())
}

/// List all registered parser ids from the global registry.
pub fn list_tool_call_parser_ids() -> Vec<String> {
    let mut ids: Vec<String> = GLOBAL_PARSERS.lock().unwrap().keys().cloned().collect();
    ids.sort();
    ids
}

/// Parse content using the globally registered parser.
pub fn parse_with_tool_call_parser(id: &str, content: &str) -> ToolCallParseResult {
    let factory = {
        let map = GLOBAL_PARSERS.lock().unwrap();
        map.get(id.trim().to_lowercase().as_str()).copied()
    };

    let parser = match factory {
        Some(f) => f(),
        None => {
            let ids = {
                let map = GLOBAL_PARSERS.lock().unwrap();
                let mut ids: Vec<String> = map.keys().cloned().collect();
                ids.sort();
                ids
            };
            return ToolCallParseResult::err(
                id,
                content,
                format!("unknown parser: \"{}\". Available: {}", id, ids.join(", ")),
            );
        }
    };

    let sanitized = sanitize_reasoning_tags(content);
    parser.parse(&sanitized)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestParser;

    impl ToolCallParser for TestParser {
        fn id(&self) -> &str {
            "test"
        }
        fn description(&self) -> &str {
            "Test parser"
        }
        fn model_families(&self) -> &[&str] {
            &["test-family"]
        }
        fn parse(&self, _content: &str) -> ToolCallParseResult {
            ToolCallParseResult::ok("test", vec![], "")
        }
    }

    fn test_parser_factory() -> Box<dyn ToolCallParser> {
        Box::new(TestParser)
    }

    #[test]
    fn test_register_and_get() {
        let mut registry = DefaultToolCallParserRegistry::new();
        registry.register("test", test_parser_factory);
        let parser = registry.get("test");
        assert!(parser.is_some());
        assert_eq!(parser.unwrap().id(), "test");
    }

    #[test]
    fn test_get_unknown_returns_none() {
        let registry = DefaultToolCallParserRegistry::new();
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_list_ids() {
        let mut registry = DefaultToolCallParserRegistry::new();
        registry.register("b", test_parser_factory);
        registry.register("a", test_parser_factory);
        let ids = registry.list_ids();
        assert_eq!(ids, vec!["a", "b"]);
    }

    #[test]
    fn test_parse_unknown() {
        let registry = DefaultToolCallParserRegistry::new();
        let result = registry.parse("nonexistent", "hello");
        assert!(!result.ok);
        assert!(result.error.unwrap().contains("unknown parser"));
    }

    #[test]
    fn test_parse_sanitizes_reasoning() {
        let mut registry = DefaultToolCallParserRegistry::new();
        registry.register("test", test_parser_factory);
        let result = registry.parse("test", "hello <think>thinking</think> world");
        assert!(result.ok);
        assert_eq!(result.content, ""); // parser returns empty content
    }

    #[test]
    fn test_list_parsers() {
        let mut registry = DefaultToolCallParserRegistry::new();
        registry.register("test", test_parser_factory);
        let parsers = registry.list_parsers();
        assert_eq!(parsers.len(), 1);
        assert_eq!(parsers[0].id, "test");
        assert_eq!(parsers[0].description, "Test parser");
        assert_eq!(parsers[0].model_families, vec!["test-family"]);
    }
}
