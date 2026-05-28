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
            if map.contains_key(TAG) {
                // Already abstracted — idempotent passthrough.
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
