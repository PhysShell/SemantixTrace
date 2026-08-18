//! Parity between the published JSON Schema files and the Rust serde
//! model (S1 acceptance criterion: "The JSON Schema validates a
//! hand-crafted seed corpus of representative v1 envelopes").
//!
//! Until this suite existed the `schema/*.schema.json` files were dead
//! documentation on the Rust side: nothing prevented the serde model
//! and the published schema from drifting apart. Three directions are
//! pinned here:
//!
//! 1. **Model → schema**: every serde-serialized envelope (property-
//!    generated, both versions) validates against its schema file.
//! 2. **Corpus → both**: the committed hand-crafted fixture corpus
//!    parses through [`trace_schema::read_event`] *and* validates
//!    against the schema files, and the two versions' schemas are
//!    mutually exclusive on the corpus (the `schema_version` const
//!    discriminates).
//! 3. **Rejection agreement**: a curated set of malformed envelopes is
//!    rejected by *both* the schema files and `read_event` — the two
//!    validators agree in the falsifying direction too.

mod common;

use proptest::prelude::*;
use trace_core::DomainEntityId;
use trace_schema::{read_event, v1, v2, write_event, SchemaError};

use common::{arb_current_event, arb_v1_event};

fn validator(schema_source: &str) -> jsonschema::Validator {
    let schema: serde_json::Value =
        serde_json::from_str(schema_source).expect("schema file is valid JSON");
    jsonschema::validator_for(&schema).expect("schema file compiles")
}

fn v1_validator() -> jsonschema::Validator {
    validator(include_str!("../schema/trace-event-v1.schema.json"))
}

fn v2_validator() -> jsonschema::Validator {
    validator(include_str!("../schema/trace-event-v2.schema.json"))
}

fn assert_valid(validator: &jsonschema::Validator, instance: &serde_json::Value) {
    let errors: Vec<String> = validator
        .iter_errors(instance)
        .map(|e| format!("{e} at {}", e.instance_path()))
        .collect();
    assert!(
        errors.is_empty(),
        "schema violations: {errors:?}\nfor: {instance}"
    );
}

const V1_CORPUS: &str = include_str!("fixtures/v1-all-kinds.jsonl");
const V2_CORPUS: &str = include_str!("fixtures/v2-all-kinds.jsonl");

#[test]
fn v1_seed_corpus_validates_and_parses() {
    let schema = v1_validator();
    let mut kinds_seen = std::collections::BTreeSet::new();
    for line in V1_CORPUS.lines() {
        let instance: serde_json::Value = serde_json::from_str(line).expect("fixture line JSON");
        assert_valid(&schema, &instance);

        let event = read_event(line).expect("fixture line parses");
        assert!(
            event.domain_entity_id.is_none(),
            "v1 corpus lines must upcast with domain_entity_id = None"
        );
        kinds_seen.insert(instance["kind"].as_str().expect("kind").to_owned());
    }
    assert_eq!(kinds_seen.len(), 7, "corpus must cover all 7 v1 kinds");
}

#[test]
fn v2_seed_corpus_validates_and_parses() {
    let schema = v2_validator();
    let mut kinds_seen = std::collections::BTreeSet::new();
    let mut entity_seen = false;
    for line in V2_CORPUS.lines() {
        let instance: serde_json::Value = serde_json::from_str(line).expect("fixture line JSON");
        assert_valid(&schema, &instance);

        let event = read_event(line).expect("fixture line parses");
        if instance.get("domain_entity_id").is_some() {
            entity_seen = true;
            assert_eq!(
                event.domain_entity_id,
                Some(DomainEntityId::new(
                    instance["domain_entity_id"].as_str().expect("string id")
                ))
            );
        }
        kinds_seen.insert(instance["kind"].as_str().expect("kind").to_owned());
    }
    assert_eq!(kinds_seen.len(), 7, "corpus must cover all 7 kinds");
    assert!(entity_seen, "corpus must exercise domain_entity_id");
}

/// The two published schemas are mutually exclusive on the corpus: the
/// `schema_version` const discriminates, so no line can validate under
/// both versions at once.
#[test]
fn version_consts_make_schemas_mutually_exclusive() {
    let v1_schema = v1_validator();
    let v2_schema = v2_validator();
    for line in V1_CORPUS.lines() {
        let instance: serde_json::Value = serde_json::from_str(line).expect("JSON");
        assert!(
            !v2_schema.is_valid(&instance),
            "v1 line valid under v2: {line}"
        );
    }
    for line in V2_CORPUS.lines() {
        let instance: serde_json::Value = serde_json::from_str(line).expect("JSON");
        assert!(
            !v1_schema.is_valid(&instance),
            "v2 line valid under v1: {line}"
        );
    }
}

/// Rejection agreement: envelopes malformed in representative ways are
/// rejected by the schema file *and* by `read_event`.
#[test]
fn schema_and_serde_agree_on_rejection() {
    let cases: &[(&str, &str)] = &[
        (
            "missing ts",
            r#"{"schema_version":1,"seq":0,"session_id":"00000000-0000-0000-0000-00000000000a","kind":"NavigationOccurred","from":"A","to":"B"}"#,
        ),
        (
            "unknown kind",
            r#"{"schema_version":1,"seq":0,"session_id":"00000000-0000-0000-0000-00000000000a","ts":"2026-05-27T12:00:00Z","kind":"ButtonClicked","button":"Ok"}"#,
        ),
        (
            "outcome outside enum",
            r#"{"schema_version":1,"seq":0,"session_id":"00000000-0000-0000-0000-00000000000a","ts":"2026-05-27T12:00:00Z","kind":"CommandExecuted","command_id":"X.Do","args":{},"duration_ms":1,"outcome":"exploded"}"#,
        ),
        (
            "string seq",
            r#"{"schema_version":1,"seq":"first","session_id":"00000000-0000-0000-0000-00000000000a","ts":"2026-05-27T12:00:00Z","kind":"NavigationOccurred","from":"A","to":"B"}"#,
        ),
        (
            "missing schema_version",
            r#"{"seq":0,"session_id":"00000000-0000-0000-0000-00000000000a","ts":"2026-05-27T12:00:00Z","kind":"NavigationOccurred","from":"A","to":"B"}"#,
        ),
        (
            "bad ValuePolicy discriminator",
            r#"{"schema_version":1,"seq":0,"session_id":"00000000-0000-0000-0000-00000000000a","ts":"2026-05-27T12:00:00Z","kind":"FieldChanged","field_id":"F","old":{"policy":"plaintext","value":"x"},"new":{"policy":"removed"}}"#,
        ),
    ];
    let schema = v1_validator();
    for (name, line) in cases {
        let instance: serde_json::Value = serde_json::from_str(line).expect("JSON");
        assert!(
            !schema.is_valid(&instance),
            "schema accepted `{name}`: {line}"
        );
        assert!(
            matches!(
                read_event(line),
                Err(SchemaError::InvalidShape(_) | SchemaError::Parse(_))
            ),
            "read_event accepted `{name}`: {line}"
        );
    }
}

proptest! {
    /// Model → schema, v1: every serde-serialized v1 envelope validates
    /// against the published v1 schema file.
    #[test]
    fn generated_v1_envelopes_validate_against_v1_schema(event in arb_v1_event()) {
        let schema = v1_validator();
        let instance = serde_json::to_value(v1::TraceEnvelope::from_event(event))
            .expect("serialize");
        let errors: Vec<String> = schema.iter_errors(&instance).map(|e| e.to_string()).collect();
        prop_assert!(errors.is_empty(), "schema violations: {:?} for {}", errors, instance);
    }

    /// Model → schema, v2: every line `write_event` emits (the actual
    /// on-disk writer path) validates against the published v2 schema.
    #[test]
    fn write_event_output_validates_against_v2_schema(event in arb_current_event()) {
        let schema = v2_validator();
        let raw = write_event(&event).expect("serialize");
        let instance: serde_json::Value =
            serde_json::from_str(raw.trim_end_matches('\n')).expect("own output is JSON");
        let errors: Vec<String> = schema.iter_errors(&instance).map(|e| e.to_string()).collect();
        prop_assert!(errors.is_empty(), "schema violations: {:?} for {}", errors, instance);
    }

    /// Model → schema, v2 envelope type directly (not only through
    /// `write_event`), so hand-built envelopes stay in parity too.
    #[test]
    fn generated_v2_envelopes_validate_against_v2_schema(event in arb_current_event()) {
        let schema = v2_validator();
        let instance = serde_json::to_value(v2::TraceEnvelope::from_event(event))
            .expect("serialize");
        let errors: Vec<String> = schema.iter_errors(&instance).map(|e| e.to_string()).collect();
        prop_assert!(errors.is_empty(), "schema violations: {:?} for {}", errors, instance);
    }
}
