//! Domain outcome of a command or async operation.

use serde::{Deserialize, Serialize};

/// Result of a [`CommandExecuted`] or [`AsyncOperationCompleted`] event.
///
/// Domain-meaningful, **not** HTTP-style. New variants land via the
/// [`Upcaster`] chain so the per-version event modules in `trace-schema`
/// stay frozen; this enum is marked `#[non_exhaustive]` so consumers
/// must handle future-version variants when reading the upcaster
/// [`Current`] alias (ADR-0012 `C-NON-EXHAUSTIVE`).
///
/// [`CommandExecuted`]: ../../../trace_schema/v1/enum.TraceEventKind.html#variant.CommandExecuted
/// [`AsyncOperationCompleted`]: ../../../trace_schema/v1/enum.TraceEventKind.html#variant.AsyncOperationCompleted
/// [`Upcaster`]: ../../../trace_schema/trait.Upcaster.html
/// [`Current`]: ../../../trace_schema/type.Current.html
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "outcome", content = "detail")]
#[non_exhaustive]
pub enum Outcome {
    /// The operation completed successfully.
    Success,
    /// The operation failed with the given human-readable message.
    Failure {
        /// Short human-readable description of the failure.
        message: String,
    },
    /// The operation was cancelled by the user or by a higher-level
    /// command before it completed.
    Cancelled,
    /// The operation did not complete within its configured deadline.
    TimedOut,
}

impl Outcome {
    /// True iff the operation succeeded.
    #[must_use]
    pub const fn is_success(&self) -> bool {
        matches!(self, Self::Success)
    }

    /// True iff the operation reached a non-success terminal state.
    #[must_use]
    pub const fn is_failure(&self) -> bool {
        matches!(
            self,
            Self::Failure { .. } | Self::Cancelled | Self::TimedOut
        )
    }
}

#[cfg(test)]
mod tests {
    use super::Outcome;

    #[test]
    fn success_is_success_and_not_failure() {
        let o = Outcome::Success;
        assert!(o.is_success());
        assert!(!o.is_failure());
    }

    #[test]
    fn timed_out_is_failure_not_success() {
        let o = Outcome::TimedOut;
        assert!(!o.is_success());
        assert!(o.is_failure());
    }

    #[test]
    fn failure_round_trip() {
        let o = Outcome::Failure {
            message: "boom".into(),
        };
        let s = serde_json::to_string(&o).expect("serialize");
        let back: Outcome = serde_json::from_str(&s).expect("parse");
        assert_eq!(o, back);
    }
}
