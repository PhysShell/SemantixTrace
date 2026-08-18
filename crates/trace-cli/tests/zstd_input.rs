//! The help text unconditionally advertises `.jsonl.zst` inputs on
//! analyze/normalize/graph/oracle/report, so the *default* build of
//! the binary must actually read them. Until the `zstd` feature
//! entered the default set, every default build answered
//! `ZstdUnsupported` (mapped to 65 EX_DATAERR) for an input its own
//! help promised to accept. Stripped builds may still opt out with
//! `--no-default-features`; this test runs under the default set and
//! pins the promise the shipped binary makes.

use std::process::Command;

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_trace"))
        .args(args)
        .output()
        .expect("run trace")
}

#[test]
fn default_build_reads_zst_input() {
    let dir = tempfile::tempdir().expect("tempdir");
    let zst_path = dir.path().join("multi_session.jsonl.zst");
    let raw = std::fs::read("tests/fixtures/multi_session.jsonl").expect("fixture");
    let compressed = zstd::stream::encode_all(raw.as_slice(), 3).expect("compress");
    std::fs::write(&zst_path, compressed).expect("write");

    let from_zst = run(&[
        "analyze",
        zst_path.to_str().expect("utf-8 path"),
        "--output",
        "json",
    ]);
    assert!(
        from_zst.status.success(),
        "default build must read .jsonl.zst (stderr: {})",
        String::from_utf8_lossy(&from_zst.stderr)
    );

    let from_plain = run(&[
        "analyze",
        "tests/fixtures/multi_session.jsonl",
        "--output",
        "json",
    ]);
    assert!(from_plain.status.success());
    assert_eq!(
        from_zst.stdout, from_plain.stdout,
        "compressed and plain reads of the same corpus must agree"
    );
}
