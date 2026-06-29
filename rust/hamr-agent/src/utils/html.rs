//! Port of `packages/coding-agent/src/utils/html.ts`.
//!
//! Decode HTML entities (named, hex, and decimal) found in strings.

/// Result of decoding an HTML entity at a position.
#[derive(Debug, Clone, PartialEq)]
pub struct DecodedHtmlEntity {
    pub text: String,
    pub length: usize,
}

fn decode_code_point(code_point: u32) -> Option<char> {
    char::from_u32(code_point)
}

/// Decode a known HTML entity name (without the `&` and `;` wrappers).
pub fn decode_html_entity(entity: &str) -> Option<&'static str> {
    match entity {
        "amp" => Some("&"),
        "lt" => Some("<"),
        "gt" => Some(">"),
        "quot" => Some("\""),
        "apos" => Some("'"),
        _ => None,
    }
}

/// Attempt to decode an HTML entity starting at `index` in `html` (the `&`
/// position). Returns the decoded text and the total length consumed (including
/// `&` and `;`) if successful.
pub fn decode_html_entity_at(html: &str, index: usize) -> Option<DecodedHtmlEntity> {
    let rest = html.get(index + 1..)?;
    let semicolon_index = rest.find(';')?;
    if semicolon_index > 16 {
        return None; // entity too long, likely not an entity
    }

    let entity = &rest[..semicolon_index];
    let decoded: Option<String> = if entity.starts_with("#x") || entity.starts_with("#X") {
        let code = u32::from_str_radix(&entity[2..], 16).ok()?;
        decode_code_point(code).map(String::from)
    } else if entity.starts_with('#') {
        let code = entity[1..].parse::<u32>().ok()?;
        decode_code_point(code).map(String::from)
    } else {
        decode_html_entity(entity).map(String::from)
    };

    decoded.map(|text| DecodedHtmlEntity {
        text,
        length: semicolon_index + 2, // & + entity + ;
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_named_entities() {
        assert_eq!(decode_html_entity("amp"), Some("&"));
        assert_eq!(decode_html_entity("lt"), Some("<"));
        assert_eq!(decode_html_entity("gt"), Some(">"));
        assert_eq!(decode_html_entity("quot"), Some("\""));
        assert_eq!(decode_html_entity("apos"), Some("'"));
        assert_eq!(decode_html_entity("unknown"), None);
    }

    #[test]
    fn test_hex_entity() {
        let result = decode_html_entity_at("&#x26;", 0);
        assert!(result.is_some());
        assert_eq!(result.unwrap().text, "&");
    }

    #[test]
    fn test_decimal_entity() {
        let result = decode_html_entity_at("&#60;", 0);
        assert!(result.is_some());
        assert_eq!(result.unwrap().text, "<");
    }

    #[test]
    fn test_no_semicolon() {
        assert!(decode_html_entity_at("&#x26 ", 0).is_none());
    }

    #[test]
    fn test_too_long() {
        assert!(decode_html_entity_at("&abcdefghijklmnopq;", 0).is_none());
    }
}
