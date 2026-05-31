//! [`JsonlBackend`] — append-only JSON Lines storage (ADR-0003).
//!
//! One JSON envelope per line, UTF-8, LF-terminated, no index. Writes
//! are buffered and raw (latency over size — ADR-0003 "No compression
//! for in-flight JSONL"). Reads stream the file through
//! [`trace_schema::read_event`] so callers see only
//! [`trace_schema::Current`]; a `.zst` path is transparently
//! decompressed when the crate's `zstd` feature is enabled.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use thiserror::Error;
use trace_core::ports::{EventStream, StorageBackend};
use trace_schema::{read_event, write_event, Current, SchemaError};

/// Errors produced by [`JsonlBackend`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum JsonlError {
    /// Underlying file I/O failed.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// A line could not be parsed / serialized through the schema.
    #[error("schema error: {0}")]
    Schema(#[from] SchemaError),

    /// A `.zst` path was opened for reading but the crate was built
    /// without the `zstd` feature.
    #[error("zstd support not compiled in (enable the `zstd` feature to read .zst archives)")]
    ZstdUnsupported,
}

/// Append-only JSON Lines backend over a single file.
///
/// The write handle is opened lazily on the first [`append`] so that a
/// read-only consumer (`trace analyze`) never creates an empty file.
///
/// [`append`]: JsonlBackend::append
#[derive(Debug)]
pub struct JsonlBackend {
    path: PathBuf,
    writer: Option<BufWriter<File>>,
}

impl JsonlBackend {
    /// Bind a backend to `path`. No file is opened until the first
    /// [`append`](JsonlBackend::append) (write) or
    /// [`events`](StorageBackend::events) (read).
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            writer: None,
        }
    }

    /// Flush any buffered writes to the OS.
    ///
    /// Required before reading back appended events through the same
    /// instance (the read path opens a separate handle).
    ///
    /// # Errors
    ///
    /// Returns [`JsonlError::Io`] if the flush fails.
    pub fn flush(&mut self) -> Result<(), JsonlError> {
        if let Some(writer) = self.writer.as_mut() {
            writer.flush()?;
        }
        Ok(())
    }

    fn writer(&mut self) -> Result<&mut BufWriter<File>, JsonlError> {
        if self.writer.is_none() {
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)?;
            self.writer = Some(BufWriter::new(file));
        }
        // Just ensured `Some` above.
        Ok(self
            .writer
            .as_mut()
            .expect("writer initialized in the branch above"))
    }
}

impl StorageBackend for JsonlBackend {
    type Event = Current;
    type Error = JsonlError;

    fn append(&mut self, event: &Current) -> Result<(), JsonlError> {
        let line = write_event(event)?; // already LF-terminated
        let writer = self.writer()?;
        writer.write_all(line.as_bytes())?;
        Ok(())
    }

    fn events(&self) -> Result<EventStream<'_, Current, JsonlError>, JsonlError> {
        let reader = open_reader(&self.path)?;
        Ok(read_events(reader))
    }
}

/// Stream events out of any line reader (a file, stdin, an in-memory
/// buffer), parsing each non-blank line through
/// [`trace_schema::read_event`].
///
/// Shared by [`JsonlBackend::events`] and the CLI's stdin path
/// (`trace analyze -`). Blank lines are skipped; per-line parse failures
/// are yielded as [`JsonlError::Schema`], read failures as
/// [`JsonlError::Io`].
pub fn read_events<R>(reader: R) -> EventStream<'static, Current, JsonlError>
where
    R: BufRead + 'static,
{
    let iter = reader.lines().filter_map(|line| match line {
        Ok(line) if line.trim().is_empty() => None,
        Ok(line) => Some(read_event(&line).map_err(JsonlError::Schema)),
        Err(io) => Some(Err(JsonlError::Io(io))),
    });
    Box::new(iter)
}

fn open_reader(path: &Path) -> Result<Box<dyn BufRead>, JsonlError> {
    let file = File::open(path)?;
    if path.extension().and_then(|e| e.to_str()) == Some("zst") {
        #[cfg(feature = "zstd")]
        {
            let decoder = zstd::stream::read::Decoder::new(file)?;
            return Ok(Box::new(BufReader::new(decoder)));
        }
        #[cfg(not(feature = "zstd"))]
        {
            return Err(JsonlError::ZstdUnsupported);
        }
    }
    Ok(Box::new(BufReader::new(file)))
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use trace_core::ports::StorageBackend;
    use trace_core::{CommandId, EventSeq, FieldId, Outcome, ScreenId, SessionId, ValuePolicy};
    use trace_schema::v1::TraceEventKind;
    use trace_schema::Current;

    use super::JsonlBackend;

    fn event(seq: u64, kind: TraceEventKind) -> Current {
        Current {
            seq: EventSeq::new(seq),
            session_id: SessionId::new(uuid::Uuid::from_u128(u128::from(seq))),
            ts: Utc.with_ymd_and_hms(2026, 5, 27, 12, 0, 0).unwrap(),
            correlation_id: None,
            domain_entity_id: None,
            kind,
        }
    }

    /// One of every v1 kind, so the round-trip covers "all event-kind
    /// enumerations" (S2 acceptance criterion).
    fn all_kinds() -> Vec<Current> {
        vec![
            event(
                0,
                TraceEventKind::ScreenOpened {
                    screen_id: ScreenId::new("Editor"),
                    params: serde_json::json!({}),
                },
            ),
            event(
                1,
                TraceEventKind::CommandExecuted {
                    command_id: CommandId::new("Graph47.Recalculate"),
                    args: serde_json::json!({}),
                    duration_ms: 12,
                    outcome: Outcome::Success,
                },
            ),
            event(
                2,
                TraceEventKind::FieldChanged {
                    field_id: FieldId::new("Quantity"),
                    old: ValuePolicy::default_masked(),
                    new: ValuePolicy::Bucketed {
                        bucket: "11-100".into(),
                    },
                },
            ),
            event(
                3,
                TraceEventKind::ExceptionThrown {
                    exception_type: "InvalidOperation".into(),
                    message: "boom".into(),
                    stack: None,
                },
            ),
            event(
                4,
                TraceEventKind::NavigationOccurred {
                    from: ScreenId::new("A"),
                    to: ScreenId::new("B"),
                },
            ),
            event(
                5,
                TraceEventKind::ValidationFailed {
                    validator: "Required".into(),
                    field_id: FieldId::new("Iin"),
                    reason: "empty".into(),
                },
            ),
            event(
                6,
                TraceEventKind::AsyncOperationCompleted {
                    operation_id: "op-1".into(),
                    duration_ms: 99,
                    outcome: Outcome::TimedOut,
                },
            ),
        ]
    }

    #[test]
    fn append_then_read_round_trips_all_kinds() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("t.jsonl");
        let events = all_kinds();

        let mut backend = JsonlBackend::new(&path);
        for e in &events {
            backend.append(e).expect("append");
        }
        backend.flush().expect("flush");

        let read: Result<Vec<Current>, _> = backend.events().expect("open").collect();
        assert_eq!(read.expect("read"), events);
    }

    #[test]
    fn blank_lines_are_skipped() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("t.jsonl");
        let sample = all_kinds().swap_remove(1);

        let backend = JsonlBackend::new(&path);
        // Write one event followed by a stray blank line directly.
        std::fs::write(
            &path,
            format!(
                "{}\n\n",
                trace_schema::write_event(&sample)
                    .expect("serialize")
                    .trim_end()
            ),
        )
        .expect("write");

        let read: Result<Vec<Current>, _> = backend.events().expect("open").collect();
        assert_eq!(read.expect("read").len(), 1);
    }

    #[test]
    fn read_events_from_in_memory_reader() {
        // Exercises the shared line-parser used by both the file path and
        // the CLI's `trace analyze -` stdin path.
        let events = all_kinds();
        let mut buf = String::new();
        for e in &events {
            buf.push_str(&trace_schema::write_event(e).expect("serialize"));
        }
        buf.push('\n'); // trailing blank line must be skipped

        let cursor = std::io::Cursor::new(buf.into_bytes());
        let read: Result<Vec<Current>, _> = super::read_events(cursor).collect();
        assert_eq!(read.expect("read"), events);
    }

    #[test]
    fn missing_file_is_io_not_found() {
        let backend = JsonlBackend::new("/nonexistent/path/to/trace.jsonl");
        let kind = match backend.events() {
            Err(super::JsonlError::Io(io)) => Some(io.kind()),
            Ok(_) => None,
            Err(other) => panic!("unexpected: {other:?}"),
        };
        assert_eq!(kind, Some(std::io::ErrorKind::NotFound));
    }
}
