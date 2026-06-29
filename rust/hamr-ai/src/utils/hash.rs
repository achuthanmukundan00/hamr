//! Port of `packages/ai/src/utils/hash.ts`.
//!
//! Fast deterministic hash to shorten long strings. Mirrors the JS `shortHash`
//! bit-for-bit: same constants, `Math.imul` → `u32::wrapping_mul`, unsigned
//! right shifts, and base-36 encoding of `(h2 >>> 0)` followed by `(h1 >>> 0)`.

/// Encode a `u32` as a lowercase base-36 string, matching JS `(n >>> 0).toString(36)`.
fn to_base36(mut n: u32) -> String {
    if n == 0 {
        return "0".to_string();
    }
    const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut buf = Vec::new();
    while n > 0 {
        buf.push(DIGITS[(n % 36) as usize]);
        n /= 36;
    }
    buf.reverse();
    // Safe: DIGITS are all ASCII.
    String::from_utf8(buf).expect("base36 digits are ascii")
}

/// Fast deterministic hash to shorten long strings.
///
/// Iterates over UTF-16 code units to match JS `String.prototype.charCodeAt`.
pub fn short_hash(str: &str) -> String {
    let mut h1: u32 = 0xdead_beef;
    let mut h2: u32 = 0x41c6_ce57;
    for ch in str.encode_utf16() {
        let ch = ch as u32;
        h1 = (h1 ^ ch).wrapping_mul(2654435761);
        h2 = (h2 ^ ch).wrapping_mul(1597334677);
    }
    h1 = (h1 ^ (h1 >> 16)).wrapping_mul(2246822507) ^ (h2 ^ (h2 >> 13)).wrapping_mul(3266489909);
    h2 = (h2 ^ (h2 >> 16)).wrapping_mul(2246822507) ^ (h1 ^ (h1 >> 13)).wrapping_mul(3266489909);
    format!("{}{}", to_base36(h2), to_base36(h1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        assert_eq!(short_hash("hello"), short_hash("hello"));
    }

    #[test]
    fn distinct_inputs_distinct_outputs() {
        assert_ne!(short_hash("apple"), short_hash("banana"));
    }

    #[test]
    fn lowercase_base36() {
        let h = short_hash("the quick brown fox");
        assert!(
            h.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        );
    }

    #[test]
    fn empty_string_is_stable() {
        assert_eq!(short_hash(""), short_hash(""));
        assert!(!short_hash("").is_empty());
    }
}
