/// TOML config deserialization and hot-reload support.
use std::collections::HashMap;

/// Load theme values from a TOML config file.
pub fn load_toml(path: &str, values: &mut HashMap<String, String>) {
    if let Ok(contents) = std::fs::read_to_string(path) {
        if let Ok(toml_val) = contents.parse::<toml::Value>() {
            flatten_toml(&toml_val, "", values);
        }
    }
}

/// Flatten a TOML value into dot-separated keys.
fn flatten_toml(value: &toml::Value, prefix: &str, values: &mut HashMap<String, String>) {
    match value {
        toml::Value::Table(table) => {
            for (key, val) in table {
                let new_prefix = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", prefix, key)
                };
                flatten_toml(val, &new_prefix, values);
            }
        }
        toml::Value::String(s) => {
            values.insert(prefix.to_string(), s.clone());
        }
        toml::Value::Integer(i) => {
            values.insert(prefix.to_string(), i.to_string());
        }
        toml::Value::Float(f) => {
            values.insert(prefix.to_string(), f.to_string());
        }
        toml::Value::Boolean(b) => {
            values.insert(prefix.to_string(), b.to_string());
        }
        _ => {}
    }
}
