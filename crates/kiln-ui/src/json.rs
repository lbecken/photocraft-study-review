//! Compact JSON with object keys sorted at every level (the control protocol's response form,
//! `docs/control-protocol.md` "Response"), independent of the map type `serde_json` was built
//! with, and the 200-character cap on error messages.

use serde_json::Value;

/// The longest error message, in characters (as the control protocol's cap).
pub(crate) const MAX_MESSAGE: usize = 200;

/// `value` as compact JSON, object keys sorted by code point at every level.
pub(crate) fn canonical(value: &Value) -> String {
    let mut out = String::new();
    write(value, &mut out);
    out
}

fn write(value: &Value, out: &mut String) {
    match value {
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write(item, out);
            }
            out.push(']');
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort_unstable();
            out.push('{');
            for (i, key) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&Value::String((*key).clone()).to_string());
                out.push(':');
                if let Some(v) = map.get(*key) {
                    write(v, out);
                }
            }
            out.push('}');
        }
        other => out.push_str(&other.to_string()),
    }
}

/// `s` cut to at most [`MAX_MESSAGE`] characters: the first 199 and `…` (never splits a char).
pub(crate) fn cap(s: &str) -> String {
    if s.chars().count() <= MAX_MESSAGE {
        return s.to_owned();
    }
    let mut out: String = s.chars().take(MAX_MESSAGE - 1).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn keys_are_sorted_at_every_level_and_output_is_compact() {
        let v = json!({"b": 1, "a": {"y": [1, {"d": 2, "c": 3}], "x": null}});
        assert_eq!(
            canonical(&v),
            r#"{"a":{"x":null,"y":[1,{"c":3,"d":2}]},"b":1}"#
        );
    }

    #[test]
    fn cap_cuts_long_messages_on_char_boundaries() {
        let long = "é".repeat(300);
        let capped = cap(&long);
        assert_eq!(capped.chars().count(), MAX_MESSAGE);
        assert!(capped.ends_with('…'));
        assert_eq!(cap("short"), "short");
    }
}
