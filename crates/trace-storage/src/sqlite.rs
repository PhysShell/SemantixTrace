//! [`SqliteBackend`] — `SQLite` storage backend (S8, `sqlite` feature).
//!
//! Events are stored in a single `events` table keyed by
//! `(session_id, seq)`.  Four extracted index columns
//! (`command_id`, `screen_id`, `outcome`, `domain_entity_id`) accelerate
//! the most common filter queries without requiring a full `payload_json`
//! scan (S8 spec §"Schema").  The `payload_json` column remains the
//! authoritative record and is always read through
//! [`trace_schema::read_event`] so the upcaster chain applies to any
//! events stored in older schema versions.
//!
//! WAL journal mode is enabled on open so concurrent readers do not
//! block writers (S8 spec §"Approach").

use std::path::Path;

use rusqlite::{params, Connection, OpenFlags};
use thiserror::Error;
use trace_core::ports::{EventStream, StorageBackend};
use trace_schema::{read_event, write_event, Current, SchemaError};

use crate::extract::{event_kind_name, extract_command_id, extract_outcome_str, extract_screen_id};

/// Errors produced by [`SqliteBackend`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SqliteError {
    /// A rusqlite-level error (I/O, constraint violation, etc.).
    #[error("sqlite error: {0}")]
    Rusqlite(#[from] rusqlite::Error),

    /// A payload could not be serialized / deserialized through the schema.
    #[error("schema error: {0}")]
    Schema(#[from] SchemaError),
}

/// DDL executed once on every [`SqliteBackend::open`].
const SCHEMA_SQL: &str = "
    CREATE TABLE IF NOT EXISTS events (
        session_id       TEXT    NOT NULL,
        seq              INTEGER NOT NULL,
        schema_version   INTEGER NOT NULL DEFAULT 2,
        ts               TEXT    NOT NULL,
        kind             TEXT    NOT NULL,
        payload_json     TEXT    NOT NULL,
        command_id       TEXT,
        screen_id        TEXT,
        outcome          TEXT,
        domain_entity_id TEXT,
        PRIMARY KEY (session_id, seq)
    );
    CREATE INDEX IF NOT EXISTS idx_ev_schema  ON events(schema_version);
    CREATE INDEX IF NOT EXISTS idx_ev_cmd     ON events(command_id);
    CREATE INDEX IF NOT EXISTS idx_ev_screen  ON events(screen_id);
    CREATE INDEX IF NOT EXISTS idx_ev_outcome ON events(outcome);
    CREATE INDEX IF NOT EXISTS idx_ev_entity  ON events(domain_entity_id);
";

/// Append-and-query `SQLite` storage backend.
///
/// # Concurrency
///
/// The backend is **not** `Send` — each task or thread must open its own
/// connection.  WAL mode allows multiple read-only connections to run
/// concurrently with one write connection.
#[derive(Debug)]
pub struct SqliteBackend {
    conn: Connection,
}

impl SqliteBackend {
    /// Open (or create) a read-write `SQLite` database at `path`.
    ///
    /// Enables WAL journal mode and creates the `events` schema on first
    /// use.
    ///
    /// # Errors
    ///
    /// Returns [`SqliteError::Rusqlite`] if the file cannot be opened or the
    /// schema DDL fails.
    pub fn open(path: &Path) -> Result<Self, SqliteError> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        conn.execute_batch(SCHEMA_SQL)?;
        Ok(Self { conn })
    }

    /// Open an existing `SQLite` database at `path` in read-only mode.
    ///
    /// # Errors
    ///
    /// Returns [`SqliteError::Rusqlite`] if the file cannot be opened.
    pub fn open_readonly(path: &Path) -> Result<Self, SqliteError> {
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
        )?;
        Ok(Self { conn })
    }

    /// Iterate over every event that satisfies a column filter.
    ///
    /// The filter is expressed as a [`SliceBy`] value; the matching column
    /// is looked up via a hardcoded SQL statement (no string interpolation,
    /// no SQL injection risk).  Events are returned in `(session_id, seq)`
    /// order.
    ///
    /// # Errors
    ///
    /// Returns [`SqliteError`] if the query fails or a payload cannot be
    /// parsed.
    pub fn slice_events(&self, by: &SliceBy) -> Result<Vec<Current>, SqliteError> {
        let (sql, val): (&str, &str) = match by {
            SliceBy::SessionId(v) => (
                "SELECT payload_json FROM events \
                 WHERE session_id = ?1 ORDER BY seq",
                v,
            ),
            SliceBy::CommandId(v) => (
                "SELECT payload_json FROM events \
                 WHERE command_id = ?1 ORDER BY session_id, seq",
                v,
            ),
            SliceBy::ScreenId(v) => (
                "SELECT payload_json FROM events \
                 WHERE screen_id = ?1 ORDER BY session_id, seq",
                v,
            ),
            SliceBy::Outcome(v) => (
                "SELECT payload_json FROM events \
                 WHERE outcome = ?1 ORDER BY session_id, seq",
                v,
            ),
            SliceBy::DomainEntityId(v) => (
                "SELECT payload_json FROM events \
                 WHERE domain_entity_id = ?1 ORDER BY session_id, seq",
                v,
            ),
        };
        let mut stmt = self.conn.prepare(sql)?;
        let rows: Vec<String> = stmt
            .query_map([val], |row| row.get(0))?
            .collect::<Result<_, _>>()?;
        rows.into_iter()
            .map(|json| read_event(&json).map_err(SqliteError::Schema))
            .collect()
    }

    /// Return all distinct session-ids present in the corpus, sorted.
    ///
    /// # Errors
    ///
    /// Returns [`SqliteError::Rusqlite`] on query failure.
    pub fn session_ids(&self) -> Result<Vec<String>, SqliteError> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT session_id FROM events ORDER BY session_id")?;
        let ids: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<_, _>>()?;
        Ok(ids)
    }

    /// Candidate pre-filter for `trace report similar`: return session-ids
    /// that share at least one `command_id`, `screen_id`, or `outcome` value
    /// with `probe_values`.
    ///
    /// # Errors
    ///
    /// Returns [`SqliteError::Rusqlite`] on query failure.
    pub fn candidate_sessions(
        &self,
        probe_values: &DimensionSet,
        exclude_session: &str,
    ) -> Result<Vec<String>, SqliteError> {
        // Build a parameterised IN(...) clause for each non-empty dimension.
        // Safety: values come from parsed trace events, not user input.
        // The column names are hardcoded literals, not interpolated.
        let mut ids: std::collections::HashSet<String> = std::collections::HashSet::new();

        let mut collect = |sql: &str, values: &[String]| -> Result<(), SqliteError> {
            if values.is_empty() {
                return Ok(());
            }
            let placeholders: String = values
                .iter()
                .enumerate()
                .map(|(i, _)| format!("?{}", i + 1))
                .collect::<Vec<_>>()
                .join(", ");
            let full_sql = format!(
                "{sql} IN ({placeholders}) AND session_id != ?{}",
                values.len() + 1
            );
            let mut stmt = self.conn.prepare(&full_sql)?;
            let params: Vec<&dyn rusqlite::types::ToSql> = values
                .iter()
                .map(|v| v as &dyn rusqlite::types::ToSql)
                .chain(std::iter::once(
                    &exclude_session as &dyn rusqlite::types::ToSql,
                ))
                .collect();
            let rows: Vec<String> = stmt
                .query_map(params.as_slice(), |row| row.get(0))?
                .collect::<Result<_, _>>()?;
            ids.extend(rows);
            Ok(())
        };

        collect(
            "SELECT DISTINCT session_id FROM events WHERE command_id",
            &probe_values.command_ids,
        )?;
        collect(
            "SELECT DISTINCT session_id FROM events WHERE screen_id",
            &probe_values.screen_ids,
        )?;
        collect(
            "SELECT DISTINCT session_id FROM events WHERE outcome",
            &probe_values.outcomes,
        )?;

        let mut result: Vec<String> = ids.into_iter().collect();
        result.sort_unstable();
        Ok(result)
    }
}

impl StorageBackend for SqliteBackend {
    type Event = Current;
    type Error = SqliteError;

    fn append(&mut self, event: &Current) -> Result<(), SqliteError> {
        let payload_json = write_event(event)?;
        let payload_json = payload_json.trim_end_matches('\n');

        let kind_str = event_kind_name(&event.kind);
        let command_id = extract_command_id(&event.kind);
        let screen_id = extract_screen_id(&event.kind);
        let outcome_str = extract_outcome_str(&event.kind);
        let entity_id = event.domain_entity_id.as_ref().map(ToString::to_string);

        self.conn.execute(
            "INSERT OR REPLACE INTO events
             (session_id, seq, schema_version, ts, kind, payload_json,
              command_id, screen_id, outcome, domain_entity_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                event.session_id.to_string(),
                i64::try_from(event.seq.get()).unwrap_or(i64::MAX),
                i64::from(trace_schema::CURRENT_SCHEMA_VERSION),
                event.ts.to_rfc3339(),
                kind_str,
                payload_json,
                command_id,
                screen_id,
                outcome_str,
                entity_id,
            ],
        )?;
        Ok(())
    }

    fn events(&self) -> Result<EventStream<'_, Current, SqliteError>, SqliteError> {
        let mut stmt = self
            .conn
            .prepare("SELECT payload_json FROM events ORDER BY session_id, seq")?;
        let rows: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<_, _>>()?;
        Ok(Box::new(rows.into_iter().map(|json| {
            read_event(&json).map_err(SqliteError::Schema)
        })))
    }
}

// ---------------------------------------------------------------------------
// Supporting types
// ---------------------------------------------------------------------------

/// Filter selector for [`SqliteBackend::slice_events`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SliceBy {
    /// Filter by session UUID string.
    SessionId(String),
    /// Filter by extracted `command_id`.
    CommandId(String),
    /// Filter by extracted `screen_id`.
    ScreenId(String),
    /// Filter by extracted `outcome` label (`"success"`, `"failure"`, …).
    Outcome(String),
    /// Filter by extracted `domain_entity_id`.
    DomainEntityId(String),
}

/// Set of dimension values extracted from a probe session for
/// candidate pre-filtering in `trace report similar`.
#[derive(Debug, Default, Clone)]
pub struct DimensionSet {
    /// Distinct `command_id` values observed in the probe session.
    pub command_ids: Vec<String>,
    /// Distinct `screen_id` values.
    pub screen_ids: Vec<String>,
    /// Distinct `outcome` values.
    pub outcomes: Vec<String>,
}

impl DimensionSet {
    /// Build a [`DimensionSet`] from a slice of events.
    #[must_use]
    pub fn from_events(events: &[Current]) -> Self {
        use std::collections::BTreeSet;
        use trace_schema::v1::TraceEventKind;

        let mut cmd: BTreeSet<String> = BTreeSet::new();
        let mut scr: BTreeSet<String> = BTreeSet::new();
        let mut out: BTreeSet<String> = BTreeSet::new();

        for e in events {
            match &e.kind {
                TraceEventKind::CommandExecuted {
                    command_id,
                    outcome,
                    ..
                } => {
                    cmd.insert(command_id.to_string());
                    out.insert(crate::extract::outcome_label(outcome).to_owned());
                }
                TraceEventKind::ScreenOpened { screen_id, .. } => {
                    scr.insert(screen_id.to_string());
                }
                TraceEventKind::AsyncOperationCompleted { outcome, .. } => {
                    out.insert(crate::extract::outcome_label(outcome).to_owned());
                }
                _ => {}
            }
        }
        Self {
            command_ids: cmd.into_iter().collect(),
            screen_ids: scr.into_iter().collect(),
            outcomes: out.into_iter().collect(),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use trace_core::ports::StorageBackend;
    use trace_core::{CommandId, EventSeq, Outcome, ScreenId, SessionId};
    use trace_schema::v1::TraceEventKind;

    use super::{SliceBy, SqliteBackend};

    fn make_event(seq: u64, session: uuid::Uuid, kind: TraceEventKind) -> trace_schema::Current {
        trace_schema::Current {
            seq: EventSeq::new(seq),
            session_id: SessionId::new(session),
            ts: Utc.with_ymd_and_hms(2026, 6, 1, 0, 0, 0).unwrap(),
            correlation_id: None,
            domain_entity_id: None,
            kind,
        }
    }

    fn sample_events() -> Vec<trace_schema::Current> {
        let sid = uuid::Uuid::from_u128(1);
        vec![
            make_event(
                0,
                sid,
                TraceEventKind::ScreenOpened {
                    screen_id: ScreenId::new("Editor"),
                    params: serde_json::json!({}),
                },
            ),
            make_event(
                1,
                sid,
                TraceEventKind::CommandExecuted {
                    command_id: CommandId::new("Graph47.Recalculate"),
                    args: serde_json::json!({}),
                    duration_ms: 10,
                    outcome: Outcome::Success,
                },
            ),
            make_event(
                2,
                sid,
                TraceEventKind::CommandExecuted {
                    command_id: CommandId::new("Declaration.Save"),
                    args: serde_json::json!({}),
                    duration_ms: 5,
                    outcome: Outcome::Failure {
                        message: "disk full".into(),
                    },
                },
            ),
        ]
    }

    #[test]
    fn round_trip_all_events() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.sqlite");
        let events = sample_events();

        let mut backend = SqliteBackend::open(&path).unwrap();
        for e in &events {
            backend.append(e).unwrap();
        }
        let read: Vec<_> = backend.events().unwrap().collect::<Result<_, _>>().unwrap();
        assert_eq!(read, events);
    }

    #[test]
    fn slice_by_command_id() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("slice.sqlite");
        let events = sample_events();

        let mut backend = SqliteBackend::open(&path).unwrap();
        for e in &events {
            backend.append(e).unwrap();
        }

        let result = backend
            .slice_events(&SliceBy::CommandId("Graph47.Recalculate".into()))
            .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], events[1]);
    }

    #[test]
    fn slice_by_outcome() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("outcome.sqlite");
        let events = sample_events();

        let mut backend = SqliteBackend::open(&path).unwrap();
        for e in &events {
            backend.append(e).unwrap();
        }

        let result = backend
            .slice_events(&SliceBy::Outcome("failure".into()))
            .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], events[2]);
    }

    #[test]
    fn duplicate_append_replaces_row() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("dup.sqlite");
        let events = sample_events();

        let mut backend = SqliteBackend::open(&path).unwrap();
        backend.append(&events[0]).unwrap();
        // Append same (session_id, seq) again — INSERT OR REPLACE.
        backend.append(&events[0]).unwrap();
        let read: Vec<_> = backend.events().unwrap().collect::<Result<_, _>>().unwrap();
        assert_eq!(read.len(), 1);

        drop(events);
        drop(backend);
    }
}
