//! Adversarial fixture matrix for the JSONL read path, plus the S2
//! storage law as a property.
//!
//! The S2 stage doc pins the happy path; real recordings die mid-write,
//! get transferred through tools that rewrite line endings, and grow
//! lines nobody predicted. Every behavior here is *pinned*, not
//! aspirational: the suite documents exactly what the reader does with
//! each corruption class so a refactor cannot silently change the
//! failure mode.
//!
//! Pinned posture (matches `docs/adr/0003` + `read_events` docs):
//!
//! - Corrupt *lines* — malformed JSON and undecodable (non-UTF-8)
//!   bytes alike — surface as typed per-line `Err` items and the
//!   stream continues past them, so callers *can* recover (the CLI
//!   chooses to abort — that policy lives there, not here).
//!   Non-progressing read failures (a dead reader, e.g. a corrupt
//!   zstd frame) are terminal: one error, then the stream ends.
//! - Blank / whitespace-only lines are skipped silently.
//! - CRLF is tolerated on read (LF-only on write).
//! - A BOM is corruption, not decoration.
//! - There is no line-length cap: a gigantic event is read whole.

use std::fs;
use std::io::Write as _;
use std::path::Path;

use chrono::{TimeZone, Utc};
use proptest::prelude::*;
use trace_core::ports::StorageBackend;
use trace_core::{
    CommandId, DomainEntityId, EventSeq, FieldId, Outcome, ScreenId, SessionId, ValuePolicy,
};
use trace_schema::v1::TraceEventKind;
use trace_schema::{write_event, Current, SchemaError};
use trace_storage::{JsonlBackend, JsonlError};

fn event(seq: u64, kind: TraceEventKind) -> Current {
    Current {
        seq: EventSeq::new(seq),
        session_id: SessionId::new(uuid::Uuid::from_u128(7)),
        ts: Utc.with_ymd_and_hms(2026, 5, 27, 12, 0, 0).unwrap() + chrono::Duration::seconds(1),
        correlation_id: None,
        domain_entity_id: None,
        kind,
    }
}

fn nav(seq: u64) -> Current {
    event(
        seq,
        TraceEventKind::NavigationOccurred {
            from: ScreenId::new("A"),
            to: ScreenId::new("B"),
        },
    )
}

fn line(event: &Current) -> String {
    write_event(event).expect("serialize")
}

fn read_all(path: &Path) -> Vec<Result<Current, JsonlError>> {
    JsonlBackend::new(path)
        .events()
        .expect("open")
        .collect::<Vec<_>>()
}

#[test]
fn torn_last_line_yields_typed_error_after_valid_prefix() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("torn.jsonl");
    let (a, b, c) = (nav(0), nav(1), nav(2));
    let torn = &line(&c)[..line(&c).len() / 2]; // cut mid-object, no LF
    fs::write(&path, format!("{}{}{torn}", line(&a), line(&b))).expect("write");

    let items = read_all(&path);
    assert_eq!(items.len(), 3);
    assert_eq!(items[0].as_ref().expect("first intact"), &a);
    assert_eq!(items[1].as_ref().expect("second intact"), &b);
    match &items[2] {
        Err(JsonlError::Schema(SchemaError::Parse(_))) => {}
        other => panic!("torn tail must be a typed Parse error, got {other:?}"),
    }
}

#[test]
fn malformed_middle_line_is_an_error_item_but_stream_recovers() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("middle.jsonl");
    let (a, b) = (nav(0), nav(1));
    fs::write(&path, format!("{}{{{{garbage\n{}", line(&a), line(&b))).expect("write");

    let items = read_all(&path);
    assert_eq!(items.len(), 3);
    assert!(items[0].is_ok());
    assert!(
        matches!(items[1], Err(JsonlError::Schema(_))),
        "garbage line must be a typed error item"
    );
    assert_eq!(
        items[2].as_ref().expect("stream must continue past damage"),
        &b
    );
}

#[test]
fn crlf_line_endings_are_tolerated_on_read() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("crlf.jsonl");
    let events = vec![nav(0), nav(1), nav(2)];
    let mut content = String::new();
    for e in &events {
        content.push_str(line(e).trim_end());
        content.push_str("\r\n");
    }
    fs::write(&path, content).expect("write");

    let read: Vec<Current> = read_all(&path)
        .into_iter()
        .collect::<Result<_, _>>()
        .expect("CRLF corpus must read cleanly");
    assert_eq!(read, events);
}

#[test]
fn missing_final_newline_still_reads_last_event() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("nolf.jsonl");
    let (a, b) = (nav(0), nav(1));
    fs::write(&path, format!("{}{}", line(&a), line(&b).trim_end())).expect("write");

    let read: Vec<Current> = read_all(&path)
        .into_iter()
        .collect::<Result<_, _>>()
        .expect("read");
    assert_eq!(read, vec![a, b]);
}

#[test]
fn utf8_bom_is_corruption_not_decoration() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("bom.jsonl");
    fs::write(&path, format!("\u{FEFF}{}", line(&nav(0)))).expect("write");

    let items = read_all(&path);
    assert!(
        matches!(items[0], Err(JsonlError::Schema(_))),
        "a BOM-prefixed first line must surface as a typed error (wire format: no BOM)"
    );
}

#[test]
fn empty_file_yields_no_events() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("empty.jsonl");
    fs::write(&path, "").expect("write");
    assert!(read_all(&path).is_empty());
}

#[test]
fn whitespace_only_file_yields_no_events() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("blank.jsonl");
    fs::write(&path, "\n \n\t\n\r\n").expect("write");
    assert!(read_all(&path).is_empty());
}

/// An undecodable (non-UTF-8) line is a *per-line* failure:
/// `read_line` consumes the bad bytes before the UTF-8 validation
/// fails, so the stream progresses and later valid lines stay
/// reachable — same recovery semantics as a malformed JSON line. Only
/// non-progressing read failures (a dead reader) terminate the
/// stream.
#[test]
fn invalid_utf8_line_is_a_recoverable_per_line_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("bin.jsonl");
    let (a, b) = (nav(0), nav(1));
    let mut file = fs::File::create(&path).expect("create");
    file.write_all(line(&a).as_bytes()).expect("write");
    file.write_all(b"\xff\xfe not utf8 \xff\n").expect("write");
    file.write_all(line(&b).as_bytes()).expect("write");
    drop(file);

    let items = read_all(&path);
    assert_eq!(items.len(), 3);
    assert_eq!(items[0].as_ref().expect("first intact"), &a);
    match &items[1] {
        Err(JsonlError::Io(io)) => assert_eq!(io.kind(), std::io::ErrorKind::InvalidData),
        other => panic!("invalid UTF-8 must be an Io error, got {other:?}"),
    }
    assert_eq!(
        items[2]
            .as_ref()
            .expect("stream must continue past the bad line"),
        &b
    );
}

/// There is deliberately no line-length cap in the reader (documented
/// on `read_events`): a single event whose args run to megabytes is
/// read whole. This test pins that a 4 MiB line round-trips rather
/// than hitting an undocumented limit.
#[test]
fn gigantic_line_is_read_whole() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("huge.jsonl");
    let huge = event(
        0,
        TraceEventKind::CommandExecuted {
            command_id: CommandId::new("Bulk.Import"),
            args: serde_json::json!({ "blob": "a".repeat(4 * 1024 * 1024) }),
            duration_ms: 1,
            outcome: Outcome::Success,
        },
    );
    fs::write(&path, line(&huge)).expect("write");

    let read: Vec<Current> = read_all(&path)
        .into_iter()
        .collect::<Result<_, _>>()
        .expect("read");
    assert_eq!(read, vec![huge]);
}

// ---------------------------------------------------------------------------
// S2 storage law: iter(append(events)) == events, as a property.
// ---------------------------------------------------------------------------

fn arb_kind() -> impl Strategy<Value = TraceEventKind> {
    prop_oneof![
        "[A-Za-z]{1,10}".prop_map(|s| TraceEventKind::ScreenOpened {
            screen_id: ScreenId::new(s),
            params: serde_json::json!({}),
        }),
        ("[A-Za-z]{1,8}\\.[A-Za-z]{1,8}", 0_u64..100_000).prop_map(|(c, duration_ms)| {
            TraceEventKind::CommandExecuted {
                command_id: CommandId::new(c),
                args: serde_json::json!({}),
                duration_ms,
                outcome: Outcome::Success,
            }
        }),
        ("[A-Za-z]{1,8}", ".{0,24}").prop_map(|(t, m)| TraceEventKind::ExceptionThrown {
            exception_type: t,
            message: m,
            stack: None,
        }),
        "[A-Za-z]{1,10}".prop_map(|f| TraceEventKind::FieldChanged {
            field_id: FieldId::new(f),
            old: ValuePolicy::default_masked(),
            new: ValuePolicy::Removed,
        }),
        ("[A-Za-z]{1,8}", "[A-Za-z]{1,8}").prop_map(|(from, to)| {
            TraceEventKind::NavigationOccurred {
                from: ScreenId::new(from),
                to: ScreenId::new(to),
            }
        }),
        ("[A-Za-z]{1,8}", "[A-Za-z]{1,8}", ".{0,16}").prop_map(|(v, f, r)| {
            TraceEventKind::ValidationFailed {
                validator: v,
                field_id: FieldId::new(f),
                reason: r,
            }
        }),
        ("[a-z0-9-]{1,12}", 0_u64..100_000).prop_map(|(op, duration_ms)| {
            TraceEventKind::AsyncOperationCompleted {
                operation_id: op,
                duration_ms,
                outcome: Outcome::TimedOut,
            }
        }),
    ]
}

fn arb_event() -> impl Strategy<Value = Current> {
    (
        any::<u64>(),
        any::<u128>(),
        0_i64..3_000_000,
        prop::option::of("[A-Za-z]{1,8}:[a-z0-9]{1,8}"),
        arb_kind(),
    )
        .prop_map(|(seq, sid, offset_ms, entity, kind)| Current {
            seq: EventSeq::new(seq),
            session_id: SessionId::new(uuid::Uuid::from_u128(sid)),
            ts: Utc.with_ymd_and_hms(2026, 5, 27, 12, 0, 0).unwrap()
                + chrono::Duration::milliseconds(offset_ms),
            correlation_id: None,
            domain_entity_id: entity.map(DomainEntityId::new),
            kind,
        })
}

proptest! {
    /// S2 acceptance criterion: `iter(append(events)) == events` for
    /// arbitrary event sequences — same events, same order, nothing
    /// dropped, nothing invented.
    #[test]
    fn iter_of_append_round_trips_in_order(events in prop::collection::vec(arb_event(), 0..48)) {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("prop.jsonl");

        let mut backend = JsonlBackend::new(&path);
        for e in &events {
            backend.append(e).expect("append");
        }
        backend.flush().expect("flush");

        if events.is_empty() {
            // No append happened, so the lazy writer never created the
            // file; a read on the same path is NotFound by contract.
            let not_found = match backend.events() {
                Err(JsonlError::Io(ref io)) => io.kind() == std::io::ErrorKind::NotFound,
                _ => false,
            };
            prop_assert!(not_found, "lazy writer must not create an empty file");
        } else {
            let read: Vec<Current> = backend
                .events()
                .expect("open")
                .collect::<Result<_, _>>()
                .expect("read");
            prop_assert_eq!(read, events);
        }
    }
}

// ---------------------------------------------------------------------------
// zstd read path (feature-gated; CI runs --all-features).
// ---------------------------------------------------------------------------

#[cfg(feature = "zstd")]
mod zstd_read {
    use super::{line, nav, read_all, Current};
    use std::fs;
    use trace_core::ports::StorageBackend;

    /// A non-progressing read failure (a corrupt zstd frame keeps
    /// erroring on every `read` call) must yield ONE typed Io error and
    /// then terminate the stream. Before the fuse fix, `lines()` kept
    /// re-polling the dead reader and the stream became an infinite
    /// sequence of `Err(Io)` items — any collecting caller OOM'd.
    #[test]
    fn garbage_zst_frame_yields_one_io_error_then_terminates() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("bad.jsonl.zst");
        fs::write(&path, b"this is not a zstd frame").expect("write");

        match trace_storage::JsonlBackend::new(&path).events() {
            // Failing at open is also acceptable — still one typed error.
            Err(trace_storage::JsonlError::Io(_)) => {}
            Err(other) => panic!("expected Io, got {other:?}"),
            Ok(items) => {
                let items: Vec<_> = items.take(10_000).collect();
                assert!(
                    items.len() < 10_000,
                    "an Io-errored stream must terminate, not repeat the error forever"
                );
                assert!(
                    matches!(items.last(), Some(Err(trace_storage::JsonlError::Io(_)))),
                    "the failure must surface as a typed Io error"
                );
            }
        }
    }

    /// Same law for a stream that dies midway: everything decodable is
    /// yielded, then exactly one Io error, then termination.
    #[test]
    fn truncated_zst_stream_yields_io_error_then_terminates() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("cut.jsonl.zst");
        let raw: String = (0..64).map(|i| line(&nav(i))).collect::<Vec<_>>().concat();
        let mut compressed = zstd::stream::encode_all(raw.as_bytes(), 3).expect("compress");
        compressed.truncate(compressed.len() / 2);
        fs::write(&path, compressed).expect("write");

        let items: Vec<_> = trace_storage::JsonlBackend::new(&path)
            .events()
            .expect("open")
            .take(10_000)
            .collect();
        assert!(
            items.len() < 10_000,
            "an Io-errored stream must terminate, not repeat the error forever"
        );
        assert!(
            items
                .iter()
                .any(|i| matches!(i, Err(trace_storage::JsonlError::Io(_)))),
            "a truncated zstd stream must surface a typed Io error"
        );
        let first_err = items.iter().position(Result::is_err).expect("has error");
        assert_eq!(
            first_err,
            items.len() - 1,
            "nothing may follow the first Io error"
        );
    }

    #[test]
    fn zst_archive_round_trips() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("t.jsonl.zst");
        let events = vec![nav(0), nav(1), nav(2)];
        let raw: String = events.iter().map(line).collect();
        let compressed = zstd::stream::encode_all(raw.as_bytes(), 3).expect("compress");
        fs::write(&path, compressed).expect("write");

        let read: Vec<Current> = read_all(&path)
            .into_iter()
            .collect::<Result<_, _>>()
            .expect("read");
        assert_eq!(read, events);
    }
}

/// Without the `zstd` feature a `.zst` path must fail closed with the
/// dedicated variant, not be misread as plain JSONL.
#[cfg(not(feature = "zstd"))]
#[test]
fn zst_without_feature_is_unsupported() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("t.jsonl.zst");
    fs::write(&path, "irrelevant").expect("write");

    match JsonlBackend::new(&path).events() {
        Err(JsonlError::ZstdUnsupported) => {}
        Err(other) => panic!("expected ZstdUnsupported, got {other:?}"),
        Ok(_) => panic!("a .zst path must not open without the zstd feature"),
    }
}
