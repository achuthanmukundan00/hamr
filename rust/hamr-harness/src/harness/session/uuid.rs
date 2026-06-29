//! Port of `packages/agent/src/harness/session/uuid.ts`.

pub fn uuidv7() -> String {
    uuid::Uuid::now_v7().hyphenated().to_string()
}

#[cfg(test)]
mod tests {
    use super::uuidv7;

    #[test]
    fn generates_hyphenated_uuid_v7() {
        let value = uuidv7();
        assert_eq!(value.len(), 36);
        assert_eq!(value.chars().nth(14), Some('7'));
    }
}
