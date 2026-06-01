//! Needleman-Wunsch global sequence alignment for trace events.
//!
//! [`align`] returns a full [`Alignment`] including the edit script and
//! total cost.  The algorithm is O(n × m) in both time and space; for
//! very long traces (>10 000 events) consider a banded variant.

#![forbid(unsafe_code)]

use trace_schema::Current;

use crate::cost::CostFn;

/// A single edit operation produced by [`align`].
#[derive(Clone, Copy, Debug)]
pub struct EditOp {
    /// The kind of operation.
    pub op: Op,
    /// Cost charged for this operation.
    pub cost: f64,
    /// Index into sequence A (`None` for [`Op::Insert`]).
    pub a_idx: Option<usize>,
    /// Index into sequence B (`None` for [`Op::Delete`]).
    pub b_idx: Option<usize>,
}

/// Discriminant of an edit operation in a global alignment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    /// `a[i]` and `b[j]` are the same — zero cost.
    Match,
    /// `a[i]` was replaced by `b[j]` — non-zero cost.
    Substitute,
    /// `b[j]` was inserted (not present in A).
    Insert,
    /// `a[i]` was deleted (not present in B).
    Delete,
}

/// The result of aligning two trace-event sequences.
#[derive(Clone, Debug)]
pub struct Alignment {
    /// Sum of all edit-operation costs.
    pub total_cost: f64,
    /// The ordered list of edit operations from start to end.
    pub ops: Vec<EditOp>,
    /// Length of input sequence A.
    pub a_len: usize,
    /// Length of input sequence B.
    pub b_len: usize,
}

impl Alignment {
    /// Fraction of events that matched exactly (cost == 0.0), ranging
    /// from `0.0` (no matches) to `1.0` (perfect alignment).
    ///
    /// Returns `1.0` when both sequences are empty.
    #[must_use]
    pub fn match_rate(&self) -> f64 {
        let total = self.ops.len();
        if total == 0 {
            return 1.0;
        }
        let matches = self.ops.iter().filter(|op| op.op == Op::Match).count();
        #[allow(
            clippy::cast_precision_loss,
            reason = "sequence lengths are small enough for f64"
        )]
        let rate = matches as f64 / total as f64;
        rate
    }

    /// Similarity score: `1.0 − (total_cost / max_possible_cost)`.
    ///
    /// `max_possible_cost` is `max(a_len, b_len)` times the maximum
    /// indel cost used by the cost function (approximated as the sum of
    /// all per-op costs clamped to `[0, 1]`).  In practice the
    /// [`StructuralCost`] defaults give an indel of `0.6` and a
    /// `diff_kind` of `1.0`, so the scale is dominated by `diff_kind`.
    ///
    /// Returns `1.0` when both sequences are empty.
    ///
    /// [`StructuralCost`]: crate::cost::StructuralCost
    #[must_use]
    pub fn similarity(&self) -> f64 {
        let max_len = self.a_len.max(self.b_len);
        if max_len == 0 {
            return 1.0;
        }
        // Maximum possible cost: every position is a pure diff_kind (1.0).
        #[allow(
            clippy::cast_precision_loss,
            reason = "sequence lengths are small enough for f64"
        )]
        let max_cost = max_len as f64;
        (1.0_f64 - (self.total_cost / max_cost)).max(0.0)
    }
}

/// Align sequences `a` and `b` using the Needleman-Wunsch algorithm.
///
/// Returns the globally-optimal [`Alignment`] under the supplied cost
/// function.  The algorithm is correct for any non-negative cost table;
/// supplying negative costs will produce undefined results.
#[must_use]
#[allow(
    clippy::many_single_char_names,
    reason = "NW algorithm uses conventional i/j/n/m notation"
)]
pub fn align<C: CostFn>(a: &[Current], b: &[Current], cost: &C) -> Alignment {
    let n = a.len();
    let m = b.len();

    // -----------------------------------------------------------------------
    // Build the (n+1) × (m+1) DP table.
    // dp[i][j] = minimum cost to align a[0..i] with b[0..j].
    // -----------------------------------------------------------------------
    // Flat row-major layout: index = i * (m + 1) + j
    let size = (n + 1) * (m + 1);
    let mut dp = vec![0.0_f64; size];

    // Base cases: align a[0..i] with empty b → i deletions.
    for i in 1..=n {
        dp[i * (m + 1)] = dp[(i - 1) * (m + 1)] + cost.delete(&a[i - 1]);
    }
    // Base cases: align empty a with b[0..j] → j insertions.
    for j in 1..=m {
        dp[j] = dp[j - 1] + cost.insert(&b[j - 1]);
    }

    // Fill table.
    for i in 1..=n {
        for j in 1..=m {
            let sub_cost = cost.substitute(&a[i - 1], &b[j - 1]);
            let diag = dp[(i - 1) * (m + 1) + (j - 1)] + sub_cost;
            let del = dp[(i - 1) * (m + 1) + j] + cost.delete(&a[i - 1]);
            let ins = dp[i * (m + 1) + (j - 1)] + cost.insert(&b[j - 1]);
            dp[i * (m + 1) + j] = diag.min(del).min(ins);
        }
    }

    let total_cost = dp[n * (m + 1) + m];

    // -----------------------------------------------------------------------
    // Traceback.
    // -----------------------------------------------------------------------
    let mut ops: Vec<EditOp> = Vec::with_capacity(n + m);
    let mut i = n;
    let mut j = m;

    while i > 0 || j > 0 {
        if i > 0 && j > 0 {
            let sub_cost = cost.substitute(&a[i - 1], &b[j - 1]);
            let diag = dp[(i - 1) * (m + 1) + (j - 1)] + sub_cost;
            if (dp[i * (m + 1) + j] - diag).abs() < f64::EPSILON * 16.0 {
                // Diagonal move: match or substitute.
                let op = if sub_cost == 0.0 {
                    Op::Match
                } else {
                    Op::Substitute
                };
                ops.push(EditOp {
                    op,
                    cost: sub_cost,
                    a_idx: Some(i - 1),
                    b_idx: Some(j - 1),
                });
                i -= 1;
                j -= 1;
                continue;
            }
            let del = dp[(i - 1) * (m + 1) + j] + cost.delete(&a[i - 1]);
            if (dp[i * (m + 1) + j] - del).abs() < f64::EPSILON * 16.0 {
                ops.push(EditOp {
                    op: Op::Delete,
                    cost: cost.delete(&a[i - 1]),
                    a_idx: Some(i - 1),
                    b_idx: None,
                });
                i -= 1;
                continue;
            }
            // Must be insert.
            ops.push(EditOp {
                op: Op::Insert,
                cost: cost.insert(&b[j - 1]),
                a_idx: None,
                b_idx: Some(j - 1),
            });
            j -= 1;
        } else if i > 0 {
            ops.push(EditOp {
                op: Op::Delete,
                cost: cost.delete(&a[i - 1]),
                a_idx: Some(i - 1),
                b_idx: None,
            });
            i -= 1;
        } else {
            ops.push(EditOp {
                op: Op::Insert,
                cost: cost.insert(&b[j - 1]),
                a_idx: None,
                b_idx: Some(j - 1),
            });
            j -= 1;
        }
    }

    ops.reverse();

    Alignment {
        total_cost,
        ops,
        a_len: n,
        b_len: m,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use trace_core::{EventSeq, ScreenId, SessionId};
    use trace_schema::v1::{TraceEvent as V1Event, TraceEventKind};
    use trace_schema::v2::V1ToV2;
    use trace_schema::{Current, Upcaster};

    use crate::cost::StructuralCost;

    use super::align;

    fn make_screen(id: &str) -> Current {
        V1ToV2::upcast(V1Event {
            seq: EventSeq::new(0),
            session_id: SessionId::new(uuid::Uuid::nil()),
            ts: Utc::now(),
            correlation_id: None,
            kind: TraceEventKind::ScreenOpened {
                screen_id: ScreenId::new(id),
                params: serde_json::json!({}),
            },
        })
    }

    fn make_command(id: &str) -> Current {
        use trace_core::{CommandId, Outcome};
        V1ToV2::upcast(V1Event {
            seq: EventSeq::new(0),
            session_id: SessionId::new(uuid::Uuid::nil()),
            ts: Utc::now(),
            correlation_id: None,
            kind: TraceEventKind::CommandExecuted {
                command_id: CommandId::new(id),
                args: serde_json::json!({}),
                duration_ms: 0,
                outcome: Outcome::Success,
            },
        })
    }

    #[test]
    fn identical_sequences_cost_zero() {
        let cost = StructuralCost::default();
        let event = make_screen("Home");
        let a = vec![event.clone()];
        let b = vec![event];
        let alignment = align(&a, &b, &cost);
        assert!(
            alignment.total_cost.abs() < f64::EPSILON,
            "expected zero cost, got {}",
            alignment.total_cost
        );
        assert_eq!(alignment.a_len, 1);
        assert_eq!(alignment.b_len, 1);
    }

    #[test]
    fn insert_cost_single_event() {
        let cost = StructuralCost::default();
        let a: Vec<Current> = vec![];
        let b = vec![make_screen("Home")];
        let alignment = align(&a, &b, &cost);
        assert!(
            (alignment.total_cost - cost.indel).abs() < f64::EPSILON,
            "expected indel cost {}, got {}",
            cost.indel,
            alignment.total_cost
        );
    }

    #[test]
    fn substitution_diff_kind() {
        let cost = StructuralCost::default();
        let a = vec![make_screen("Home")];
        let b = vec![make_command("Save")];
        let alignment = align(&a, &b, &cost);
        assert!(
            (alignment.total_cost - cost.diff_kind).abs() < f64::EPSILON,
            "expected diff_kind cost {}, got {}",
            cost.diff_kind,
            alignment.total_cost
        );
    }

    #[test]
    fn substitution_same_kind_diff_id() {
        let cost = StructuralCost::default();
        let a = vec![make_screen("Home")];
        let b = vec![make_screen("Detail")];
        let alignment = align(&a, &b, &cost);
        assert!(
            (alignment.total_cost - cost.same_kind_diff_id).abs() < f64::EPSILON,
            "expected same_kind_diff_id cost {}, got {}",
            cost.same_kind_diff_id,
            alignment.total_cost
        );
    }
}
