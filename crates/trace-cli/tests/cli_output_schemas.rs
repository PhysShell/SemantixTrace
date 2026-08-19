//! ADR-0014 §11: every `--output json` payload has a published,
//! versioned schema under `crates/trace-cli/schema/`. The S2
//! acceptance criterion requires the *actual* CLI output to validate
//! against those files — until now the schema files were referenced
//! only from a doc comment, so nothing stopped output and schema from
//! drifting apart.

use std::process::Command;

fn validator(schema_source: &str) -> jsonschema::Validator {
    let schema: serde_json::Value =
        serde_json::from_str(schema_source).expect("schema file is valid JSON");
    jsonschema::validator_for(&schema).expect("schema file compiles")
}

fn run_json(args: &[&str]) -> serde_json::Value {
    let out = Command::new(env!("CARGO_BIN_EXE_trace"))
        .args(args)
        .output()
        .expect("run trace");
    assert!(
        out.status.success(),
        "`trace {}` must exit 0, stderr: {}",
        args.join(" "),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.stderr.is_empty(),
        "data goes to stdout, diagnostics to stderr — a successful run must not write stderr"
    );
    serde_json::from_slice(&out.stdout).expect("stdout is a single JSON document")
}

fn assert_valid(schema: &jsonschema::Validator, instance: &serde_json::Value) {
    let errors: Vec<String> = schema
        .iter_errors(instance)
        .map(|e| e.to_string())
        .collect();
    assert!(
        errors.is_empty(),
        "schema violations: {errors:?}\nfor: {instance}"
    );
}

#[test]
fn version_json_validates_against_published_schema() {
    let schema = validator(include_str!("../schema/trace-version-v1.schema.json"));
    let payload = run_json(&["version", "--output", "json"]);
    assert_valid(&schema, &payload);
}

#[test]
fn analyze_json_validates_against_published_schema() {
    let schema = validator(include_str!("../schema/trace-analyze-v1.schema.json"));
    let payload = run_json(&[
        "analyze",
        "tests/fixtures/multi_session.jsonl",
        "--output",
        "json",
    ]);
    assert_valid(&schema, &payload);
}
