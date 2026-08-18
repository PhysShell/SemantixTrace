//! Durability primitive for [`JsonlBackend`] (S2 residual).
//!
//! The S2 stage doc promised "fsyncs on a configurable flush policy";
//! what shipped is a manual `flush()` that only hands bytes to the OS.
//! `sync` is the explicit durability checkpoint: flush the application
//! buffer *and* fsync the file, so an audit-grade recorder can commit
//! a session boundary to stable storage. (The 64-events/250 ms policy
//! engine stays deferred — see docs/decisions.log.md.)

use chrono::{TimeZone, Utc};
use trace_core::ports::StorageBackend;
use trace_core::{EventSeq, ScreenId, SessionId};
use trace_schema::v1::TraceEventKind;
use trace_schema::Current;
use trace_storage::JsonlBackend;

fn nav(seq: u64) -> Current {
    Current {
        seq: EventSeq::new(seq),
        session_id: SessionId::new(uuid::Uuid::from_u128(11)),
        ts: Utc.with_ymd_and_hms(2026, 5, 27, 12, 0, 0).unwrap(),
        correlation_id: None,
        domain_entity_id: None,
        kind: TraceEventKind::NavigationOccurred {
            from: ScreenId::new("A"),
            to: ScreenId::new("B"),
        },
    }
}

/// After `sync`, every appended event is visible to an independent
/// read handle — nothing may still sit in the application buffer.
#[test]
fn sync_makes_appends_visible_to_independent_readers() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("t.jsonl");
    let events = vec![nav(0), nav(1), nav(2)];

    let mut backend = JsonlBackend::new(&path);
    for e in &events {
        backend.append(e).expect("append");
    }
    backend.sync().expect("sync");

    let read: Vec<Current> = JsonlBackend::new(&path)
        .events()
        .expect("open")
        .collect::<Result<_, _>>()
        .expect("read");
    assert_eq!(read, events);

    // The original handle stays usable after a sync checkpoint.
    backend.append(&nav(3)).expect("append after sync");
    backend.sync().expect("second sync");
    let read: Vec<Current> = JsonlBackend::new(&path)
        .events()
        .expect("open")
        .collect::<Result<_, _>>()
        .expect("read");
    assert_eq!(read.len(), 4);
}

/// `sync` on a backend that never wrote is a no-op and must not create
/// the file (same lazy-writer contract as `flush`).
#[test]
fn sync_before_any_append_is_a_no_op() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("t.jsonl");

    let mut backend = JsonlBackend::new(&path);
    backend.sync().expect("sync with no writer must succeed");
    assert!(!path.exists(), "sync must not create the file");
}
