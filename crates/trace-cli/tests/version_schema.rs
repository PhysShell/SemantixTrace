//! `trace version` must report the wire-schema version the binary
//! actually understands — i.e. `trace_schema::CURRENT_SCHEMA_VERSION`,
//! not a hand-maintained literal that goes stale at every bump.
//! External consumers use this value for capability detection; a wrong
//! answer makes them mis-handle files the binary can in fact read.

use std::process::Command;

#[test]
fn version_reports_the_wire_schema_the_binary_understands() {
    let out = Command::new(env!("CARGO_BIN_EXE_trace"))
        .args(["version", "--output", "json"])
        .output()
        .expect("run trace");
    assert!(out.status.success(), "trace version must exit 0");

    let payload: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("version output is JSON");
    assert_eq!(
        payload["schema"],
        serde_json::json!(trace_schema::CURRENT_SCHEMA_VERSION),
        "trace version must track trace_schema::CURRENT_SCHEMA_VERSION"
    );
}
