//! Feature-topology contract for `.jsonl.zst` inputs.
//!
//! The help text unconditionally advertises `.jsonl.zst`, so the
//! *default* feature set includes `zstd` and the shipped binary keeps
//! the promise; a stripped `--no-default-features` build is a
//! supported opt-out that fails closed with `ZstdUnsupported`
//! (65 `EX_DATAERR`). Both sides are pinned here, gated on the
//! feature the binary was actually built with, and CI runs the real
//! default set and the stripped set as separate invocations — an
//! `--all-features` run alone could mask a topology regression
//! (removing `zstd` from `default` would otherwise keep every test
//! green).

use std::process::Command;

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_trace"))
        .args(args)
        .output()
        .expect("run trace")
}

#[cfg(feature = "zstd")]
mod with_zstd {
    use super::run;

    /// A zstd-enabled build (the default set) reads a compressed
    /// corpus and produces byte-identical output to the plain read.
    #[test]
    fn zst_input_reads_and_matches_plain() {
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
            "zstd-enabled build must read .jsonl.zst (stderr: {})",
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
}

#[cfg(not(feature = "zstd"))]
mod without_zstd {
    use super::run;

    /// A stripped build fails closed on a `.zst` path: 65 EX_DATAERR
    /// and a diagnostic naming zstd — never a silent misread of the
    /// compressed bytes as plain JSONL.
    #[test]
    fn zst_input_fails_closed_without_the_feature() {
        let dir = tempfile::tempdir().expect("tempdir");
        let zst_path = dir.path().join("multi_session.jsonl.zst");
        std::fs::write(&zst_path, b"content is irrelevant: the extension gates").expect("write");

        let out = run(&["analyze", zst_path.to_str().expect("utf-8 path")]);
        assert_eq!(
            out.status.code(),
            Some(65),
            "ZstdUnsupported maps to 65 EX_DATAERR"
        );
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("zstd"),
            "diagnostic must name the missing feature, got: {stderr}"
        );
        assert!(out.stdout.is_empty(), "no data may reach stdout");
    }
}
