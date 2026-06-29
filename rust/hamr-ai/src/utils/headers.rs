//! Port of `packages/ai/src/utils/headers.ts`.
//!
//! Flatten a [`reqwest::header::HeaderMap`] (the Rust analogue of the Web
//! `Headers` object) into a plain `HashMap<String, String>`.

use std::collections::HashMap;

use reqwest::header::HeaderMap;

/// Convert a [`HeaderMap`] into a `Record<string, string>`.
///
/// Header names are lowercased by `HeaderMap` (matching the Web `Headers`
/// iteration contract). Non-UTF-8 header values are skipped.
pub fn headers_to_record(headers: &HeaderMap) -> HashMap<String, String> {
    let mut result = HashMap::new();
    for (key, value) in headers.iter() {
        if let Ok(value) = value.to_str() {
            result.insert(key.as_str().to_string(), value.to_string());
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::{HeaderMap, HeaderValue};

    #[test]
    fn flattens_headers() {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", HeaderValue::from_static("application/json"));
        headers.insert("x-request-id", HeaderValue::from_static("abc123"));
        let record = headers_to_record(&headers);
        assert_eq!(
            record.get("content-type").map(String::as_str),
            Some("application/json")
        );
        assert_eq!(
            record.get("x-request-id").map(String::as_str),
            Some("abc123")
        );
    }

    #[test]
    fn empty_map_yields_empty_record() {
        assert!(headers_to_record(&HeaderMap::new()).is_empty());
    }
}
