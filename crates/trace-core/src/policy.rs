//! Privacy-aware wrapper around a recorded value (ADR-0007).
//!
//! `mask-by-default` is the project's posture: strings are masked,
//! numerics bucketed, identifiers hashed. Raw values require an explicit
//! per-field opt-in on the adapter side plus an audit-log entry on
//! export. See [`docs/privacy.md`](../../../../docs/privacy.md) for the
//! operational rules.

use serde::{Deserialize, Serialize};

/// Privacy-aware wrapper around a recorded value.
///
/// New variants may be added in future schema versions through the
/// [`Upcaster`] chain (ADR-0006); the enum is `#[non_exhaustive]` to
/// keep that path open without a breaking change (ADR-0012
/// `C-NON-EXHAUSTIVE`).
///
/// [`Upcaster`]: ../../../trace_schema/trait.Upcaster.html
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "policy", rename_all = "snake_case")]
#[non_exhaustive]
pub enum ValuePolicy {
    /// The raw value. Recorded **only** if the adapter declared an
    /// explicit `[TraceField(policy = Raw)]` override.
    Raw {
        /// The raw JSON value the adapter captured.
        value: serde_json::Value,
    },
    /// The default for free-text strings (e.g. `"***"`).
    Masked {
        /// The displayed mask. Length intentionally does not encode
        /// information about the original (`privacy.md` §"Anti-patterns").
        display: String,
    },
    /// The default for numerics and dates: a stable categorical bucket
    /// (e.g. `"11-100"`, `"past_week"`).
    Bucketed {
        /// The bucket label, drawn from `glossary.md` §4.
        bucket: String,
    },
    /// A keyed hash for identifiers (email, phone, IBAN, IIN/BIN, card).
    /// Equivalence on identity is preserved without leaking the value.
    Hashed {
        /// Hex-encoded hash digest.
        hash: String,
        /// Hash algorithm name (e.g. `"blake3"`).
        algo: String,
    },
    /// The adapter dropped the field entirely (`[TraceField(policy =
    /// Removed)]`).
    Removed,
}

impl ValuePolicy {
    /// True iff the wrapped variant exposes the original value.
    #[must_use]
    pub const fn is_raw(&self) -> bool {
        matches!(self, Self::Raw { .. })
    }

    /// Construct the canonical "default masked string" form.
    #[must_use]
    pub fn default_masked() -> Self {
        Self::Masked {
            display: "***".to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ValuePolicy;

    #[test]
    fn default_masked_round_trip() {
        let v = ValuePolicy::default_masked();
        let s = serde_json::to_string(&v).expect("serialize");
        let back: ValuePolicy = serde_json::from_str(&s).expect("parse");
        assert_eq!(v, back);
        assert!(!back.is_raw());
    }

    #[test]
    fn raw_is_raw() {
        let v = ValuePolicy::Raw {
            value: serde_json::json!({"x": 1}),
        };
        assert!(v.is_raw());
    }

    #[test]
    fn bucketed_round_trip() {
        let v = ValuePolicy::Bucketed {
            bucket: "11-100".into(),
        };
        let s = serde_json::to_string(&v).expect("serialize");
        let back: ValuePolicy = serde_json::from_str(&s).expect("parse");
        assert_eq!(v, back);
    }
}
