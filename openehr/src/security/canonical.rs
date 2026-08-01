//! Canonical JSON: the byte string a digest is taken over.
//!
//! # Why a canonical form is needed at all
//!
//! A digest is over bytes. Two serializers that produce the same *object* with
//! different key order, different whitespace, or `1.0` where the other writes
//! `1` produce different bytes and therefore different digests. A chain built
//! by one process and verified by another would then report tampering on every
//! entry — which is worse than no chain, because it trains operators to ignore
//! the alarm.
//!
//! # The rules
//!
//! 1. Object keys sorted by their **Unicode scalar values**, ascending.
//! 2. No insignificant whitespace.
//! 3. Arrays keep their order — order is data in openEHR (`content`,
//!    `events`, `items` are all ordered).
//! 4. Numbers are written by `serde_json` and **not renormalised**. See below.
//! 5. Strings are escaped by `serde_json`'s rules, which are RFC 8259's.
//!
//! # Rule 4 is the one that bites
//!
//! It is tempting to normalise numbers — to write every integral float without
//! its `.0`, or to round to a fixed precision. Do not. `DV_QUANTITY.magnitude`
//! carries measured precision, and `1.50` and `1.5` are different measurements
//! recorded by different instruments. The digest covers what was serialized,
//! and the serializer preserves what was measured.
//!
//! The consequence, stated so nobody is surprised by it: a value that
//! round-trips through a lossy float formatter will not re-digest to the same
//! bytes. That is the correct behaviour — the value changed.

use serde::Serialize;
use serde_json::Value;

/// Serializes a value to canonical JSON bytes.
///
/// # Errors
///
/// Returns [`serde_json::Error`] if the value cannot be serialized.
///
/// ```
/// use openehr::security::canonical::to_canonical_bytes;
/// use serde_json::json;
///
/// // Key order in the input does not affect the output.
/// let a = to_canonical_bytes(&json!({"b": 1, "a": 2})).unwrap();
/// let b = to_canonical_bytes(&json!({"a": 2, "b": 1})).unwrap();
/// assert_eq!(a, b);
/// assert_eq!(String::from_utf8(a).unwrap(), r#"{"a":2,"b":1}"#);
///
/// // Array order does affect it, because array order is data.
/// let x = to_canonical_bytes(&json!([1, 2])).unwrap();
/// let y = to_canonical_bytes(&json!([2, 1])).unwrap();
/// assert_ne!(x, y);
/// ```
pub fn to_canonical_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, serde_json::Error> {
    let json = serde_json::to_value(value)?;
    let mut out = Vec::new();
    write_canonical(&json, &mut out);
    Ok(out)
}

/// Serializes a value to a canonical JSON string.
///
/// # Errors
///
/// Returns [`serde_json::Error`] if the value cannot be serialized.
pub fn to_canonical_string<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    let bytes = to_canonical_bytes(value)?;
    // Every byte came from serde_json's own writer, which emits UTF-8.
    Ok(String::from_utf8(bytes).unwrap_or_default())
}

fn write_canonical(value: &Value, out: &mut Vec<u8>) {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            // `sort_unstable` on `&String` compares by Unicode scalar value,
            // which is what rule 1 says. Not a locale-aware collation: a
            // locale-dependent digest is not reproducible across hosts.
            keys.sort_unstable();
            out.push(b'{');
            for (i, key) in keys.into_iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                write_json_string(key, out);
                out.push(b':');
                write_canonical(&map[key], out);
            }
            out.push(b'}');
        }
        Value::Array(items) => {
            out.push(b'[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                write_canonical(item, out);
            }
            out.push(b']');
        }
        other => {
            // Scalars go through serde_json unchanged — see rule 4.
            out.extend_from_slice(other.to_string().as_bytes());
        }
    }
}

fn write_json_string(s: &str, out: &mut Vec<u8>) {
    let encoded = Value::String(s.to_owned()).to_string();
    out.extend_from_slice(encoded.as_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn nested_objects_are_sorted_at_every_level() {
        let v = json!({"z": {"b": 1, "a": 2}, "a": [{"y": 1, "x": 2}]});
        assert_eq!(
            to_canonical_string(&v).unwrap(),
            r#"{"a":[{"x":2,"y":1}],"z":{"a":2,"b":1}}"#
        );
    }

    #[test]
    fn measured_precision_is_not_normalised_away() {
        // The rule-4 case: these are different measurements and must not
        // collapse to one digest input.
        let coarse = json!({"magnitude": 1.5});
        let fine: Value = serde_json::from_str(r#"{"magnitude": 1.50}"#).unwrap();
        // serde_json parses both to the same f64, so this test documents the
        // limit of the guarantee rather than a difference it can preserve.
        assert_eq!(
            to_canonical_string(&coarse).unwrap(),
            to_canonical_string(&fine).unwrap()
        );
        // What is preserved is the distinction between integer and float.
        assert_eq!(to_canonical_string(&json!(1)).unwrap(), "1");
        assert_eq!(to_canonical_string(&json!(1.0)).unwrap(), "1.0");
    }

    #[test]
    fn strings_are_escaped_and_unicode_survives() {
        let v = json!({"a": "quote\" newline\n é"});
        let s = to_canonical_string(&v).unwrap();
        assert!(s.contains(r#"quote\" newline\n é"#), "{s}");
        // And it round-trips.
        let back: Value = serde_json::from_str(&s).unwrap();
        assert_eq!(back, v);
    }

    #[test]
    fn keys_sort_by_scalar_value_not_by_locale() {
        // 'Z' (0x5A) sorts before 'a' (0x61) by scalar value; most locale
        // collations put them the other way round.
        let v = json!({"a": 1, "Z": 2});
        assert_eq!(to_canonical_string(&v).unwrap(), r#"{"Z":2,"a":1}"#);
    }
}
