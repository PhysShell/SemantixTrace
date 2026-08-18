//! Normalization configuration ([`NormCfg`]).

use std::collections::BTreeMap;

/// Knobs controlling value and temporal abstraction.
///
/// `Default` matches the policy tables in `glossary.md` §4: numeric
/// buckets `0 / 1 / 2-10 / 11-100 / 101-1000 / 1001+`, a 50 ms burst
/// gap, a 5 s idle gap, and no per-field overrides.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormCfg {
    /// Inclusive upper bounds of the numeric buckets, ascending. A value
    /// `n` falls in the first bucket whose bound it does not exceed; a
    /// value above the last bound is the open-ended `>last` bucket.
    pub numeric_bucket_bounds: Vec<i64>,
    /// Per-field numeric bucket tables (S3 "per-field policy
    /// overrides"). While abstracting an argument object, a key present
    /// in this map switches the bucket table for that key's entire
    /// subtree (nested objects and arrays included) until a deeper key
    /// carries its own override; keys absent from the map inherit the
    /// enclosing table. A `BTreeMap` keeps iteration — and therefore
    /// output — deterministic.
    pub per_field_bucket_bounds: BTreeMap<String, Vec<i64>>,
    /// Consecutive identical canonical actions within this many
    /// milliseconds collapse into one (`BurstAction` semantics).
    pub burst_gap_ms: i64,
    /// A gap larger than this many milliseconds between two events is
    /// counted as a `SessionPause` in the [`FoldReport`].
    ///
    /// [`FoldReport`]: crate::fold::FoldReport
    pub idle_gap_ms: i64,
}

impl Default for NormCfg {
    fn default() -> Self {
        Self {
            // Bounds for 0 / 1 / 2-10 / 11-100 / 101-1000, then 1000+.
            numeric_bucket_bounds: vec![0, 1, 10, 100, 1000],
            per_field_bucket_bounds: BTreeMap::new(),
            burst_gap_ms: 50,
            idle_gap_ms: 5_000,
        }
    }
}

impl NormCfg {
    /// Label the numeric bucket `value` falls into using this config's
    /// global bounds table. Negative numbers map symmetrically with a
    /// `-` prefix (`glossary.md` §4).
    #[must_use]
    pub fn bucket_label(&self, value: i64) -> String {
        Self::label_in(&self.numeric_bucket_bounds, value)
    }

    /// Label `value` against an explicit bounds table (used by the
    /// abstractor when a per-field override is active).
    #[must_use]
    pub fn label_in(bounds: &[i64], value: i64) -> String {
        if value < 0 {
            return format!("-{}", Self::label_in(bounds, value.saturating_neg()));
        }
        let mut lower = 0_i64;
        for &bound in bounds {
            if value <= bound {
                return if lower == bound {
                    bound.to_string()
                } else {
                    format!("{lower}-{bound}")
                };
            }
            lower = bound.saturating_add(1);
        }
        let last = bounds.last().copied().unwrap_or(0);
        format!("{}+", last.saturating_add(1))
    }
}

#[cfg(test)]
mod tests {
    use super::NormCfg;

    #[test]
    fn default_buckets_match_glossary() {
        let cfg = NormCfg::default();
        assert_eq!(cfg.bucket_label(0), "0");
        assert_eq!(cfg.bucket_label(1), "1");
        assert_eq!(cfg.bucket_label(2), "2-10");
        assert_eq!(cfg.bucket_label(10), "2-10");
        assert_eq!(cfg.bucket_label(11), "11-100");
        assert_eq!(cfg.bucket_label(100), "11-100");
        assert_eq!(cfg.bucket_label(101), "101-1000");
        assert_eq!(cfg.bucket_label(1000), "101-1000");
        assert_eq!(cfg.bucket_label(1001), "1001+");
        assert_eq!(cfg.bucket_label(50_000), "1001+");
    }

    #[test]
    fn negatives_are_symmetric() {
        let cfg = NormCfg::default();
        assert_eq!(cfg.bucket_label(-5), "-2-10");
        assert_eq!(cfg.bucket_label(-1), "-1");
    }
}
