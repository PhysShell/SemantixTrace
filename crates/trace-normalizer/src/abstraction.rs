//! Value abstraction: replace concrete values with stable, idempotent
//! buckets and format classes (`glossary.md` §4).
//!
//! Abstracted values are tagged JSON objects carrying the [`TAG`] key, so
//! re-abstracting an already-abstracted value is a no-op — this is what
//! makes [`abstract_value`] a fixed point.

use serde_json::{Map, Number, Value};

use crate::config::NormCfg;

/// Marker key present on every abstracted JSON object.
pub const TAG: &str = "_abstract";

/// Abstract a JSON value: numerics become bucket labels, strings become
/// `{class, len}` descriptors, containers recurse. Booleans and null pass
/// through (negligible re-identification risk).
///
/// Idempotent: `abstract_value(abstract_value(v), cfg) == abstract_value(v, cfg)`.
#[must_use]
pub fn abstract_value(value: &Value, cfg: &NormCfg) -> Value {
    match value {
        Value::Null | Value::Bool(_) => value.clone(),
        Value::Number(n) => numeric(n, cfg),
        Value::String(s) => string_class(s),
        Value::Array(items) => Value::Array(items.iter().map(|v| abstract_value(v, cfg)).collect()),
        Value::Object(map) => {
            if is_abstracted_tag(map) {
                // Already abstracted (validated tag shape) — idempotent passthrough.
                value.clone()
            } else {
                let mut out = Map::with_capacity(map.len());
                for (key, val) in map {
                    out.insert(key.clone(), abstract_value(val, cfg));
                }
                Value::Object(out)
            }
        }
    }
}

/// Returns true only for objects that were actually produced by this
/// abstractor. The kind alone is not enough: a forged
/// `{"_abstract":"numeric","bucket":"alice@example.com"}` would otherwise
/// pass through and leak the raw value, so the full shape is validated —
/// exact key set plus label-shaped values (bucket made of digits/`-`/`+`,
/// class/len from the fixed vocabularies). Anything else is recursively
/// abstracted, preserving the privacy-by-default policy (ADR-0007).
fn is_abstracted_tag(map: &Map<String, Value>) -> bool {
    let Some(Value::String(kind)) = map.get(TAG) else {
        return false;
    };
    match kind.as_str() {
        "numeric" => {
            map.len() == 2
                && matches!(map.get("bucket"), Some(Value::String(b)) if is_bucket_label(b))
        }
        "string" => {
            map.len() == 3
                && matches!(map.get("class"), Some(Value::String(c)) if is_known_class(c))
                && matches!(map.get("len"), Some(Value::String(l)) if is_known_len(l))
        }
        _ => false,
    }
}

fn is_bucket_label(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_digit() || b == b'-' || b == b'+')
}

fn is_known_class(s: &str) -> bool {
    matches!(s, "email" | "guid" | "numeric" | "free")
}

fn is_known_len(s: &str) -> bool {
    matches!(s, "0" | "1-8" | "9-32" | "33-128" | "128+")
}

fn tagged(kind: &str, entries: &[(&str, Value)]) -> Value {
    let mut map = Map::with_capacity(entries.len() + 1);
    map.insert(TAG.to_owned(), Value::String(kind.to_owned()));
    for (key, val) in entries {
        map.insert((*key).to_owned(), val.clone());
    }
    Value::Object(map)
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "float values are bucketed; truncation to the bucket's integer key is intentional"
)]
fn numeric(n: &Number, cfg: &NormCfg) -> Value {
    let as_int = n
        .as_i64()
        .or_else(|| n.as_u64().map(|u| i64::try_from(u).unwrap_or(i64::MAX)))
        .or_else(|| n.as_f64().map(|f| f.trunc() as i64))
        .unwrap_or(0);
    tagged(
        "numeric",
        &[("bucket", Value::String(cfg.bucket_label(as_int)))],
    )
}

fn string_class(s: &str) -> Value {
    let class = if is_email(s) {
        "email"
    } else if is_guid(s) {
        "guid"
    } else if !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) {
        "numeric"
    } else {
        "free"
    };
    tagged(
        "string",
        &[
            ("class", Value::String(class.to_owned())),
            ("len", Value::String(len_bucket(s.len()))),
        ],
    )
}

fn len_bucket(len: usize) -> String {
    let label = match len {
        0 => "0",
        1..=8 => "1-8",
        9..=32 => "9-32",
        33..=128 => "33-128",
        _ => "128+",
    };
    label.to_owned()
}

fn is_email(s: &str) -> bool {
    match s.split_once('@') {
        Some((local, domain)) => {
            !local.is_empty() && domain.contains('.') && !domain.starts_with('.')
        }
        None => false,
    }
}

fn is_guid(s: &str) -> bool {
    let groups = [8, 4, 4, 4, 12];
    let parts: Vec<&str> = s.split('-').collect();
    parts.len() == groups.len()
        && parts
            .iter()
            .zip(groups)
            .all(|(part, len)| part.len() == len && part.bytes().all(|b| b.is_ascii_hexdigit()))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{abstract_value, TAG};
    use crate::config::NormCfg;

    fn cfg() -> NormCfg {
        NormCfg::default()
    }

    #[test]
    fn numeric_becomes_bucket() {
        let v = abstract_value(&json!(42), &cfg());
        assert_eq!(v[TAG], json!("numeric"));
        assert_eq!(v["bucket"], json!("11-100"));
    }

    #[test]
    fn email_string_classified() {
        let v = abstract_value(&json!("alice@example.com"), &cfg());
        assert_eq!(v["class"], json!("email"));
    }

    #[test]
    fn guid_string_classified() {
        let v = abstract_value(&json!("00000000-0000-0000-0000-000000000000"), &cfg());
        assert_eq!(v["class"], json!("guid"));
    }

    #[test]
    fn nested_object_recurses() {
        let v = abstract_value(&json!({"qty": 7, "note": "hi"}), &cfg());
        assert_eq!(v["qty"]["bucket"], json!("2-10"));
        assert_eq!(v["note"]["class"], json!("free"));
    }

    #[test]
    fn external_object_with_abstract_key_is_not_bypassed() {
        // An externally supplied object carrying `_abstract` must NOT be
        // treated as already-abstracted; all its values must still be masked.
        let v = abstract_value(
            &json!({"_abstract": "x", "email": "alice@example.com"}),
            &cfg(),
        );
        // The raw email must never appear in the output.
        assert_ne!(v["email"], json!("alice@example.com"));
        assert_eq!(v["email"]["class"], json!("email"));
    }

    #[test]
    fn forged_numeric_tag_with_sensitive_bucket_is_recursed() {
        // A forged tag with the right kind but a non-label `bucket` value
        // must not pass through — otherwise it would leak the raw value
        // (ADR-0007). The strict shape check rejects it and recurses.
        let v = abstract_value(
            &json!({"_abstract": "numeric", "bucket": "alice@example.com"}),
            &cfg(),
        );
        assert!(!v.to_string().contains("alice@example.com"));
        assert_eq!(v["bucket"]["class"], json!("email"));
    }

    #[test]
    fn genuine_abstractor_output_passes_through() {
        // The strict check must still accept our own output unchanged,
        // so idempotency holds.
        let cfg = cfg();
        let original = abstract_value(&json!({"qty": 50, "name": "bob"}), &cfg);
        let again = abstract_value(&original, &cfg);
        assert_eq!(original, again);
    }

    #[test]
    fn idempotent_on_scalars_and_containers() {
        let cfg = cfg();
        for input in [
            json!(0),
            json!(123_456),
            json!(-5),
            json!("user@host.io"),
            json!([1, "two", {"three": 3}]),
            json!({"a": {"b": [4, 5]}}),
            json!(true),
            json!(null),
        ] {
            let once = abstract_value(&input, &cfg);
            let twice = abstract_value(&once, &cfg);
            assert_eq!(once, twice, "not idempotent for {input}");
        }
    }
}
