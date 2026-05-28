//! Normalization configuration ([`NormCfg`]).

/// Knobs controlling value and temporal abstraction.
///
/// `Default` matches the policy tables in `glossary.md` §4: numeric
/// buckets `0 / 1 / 2-10 / 11-100 / 101-1000 / 1000+`, a 50 ms burst
/// gap, and a 5 s idle gap.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormCfg {
    /// Inclusive upper bounds of the numeric buckets, ascending. A value
    /// `n` falls in the first bucket whose bound it does not exceed; a
    /// value above the last bound is the open-ended `>last` bucket.
    pub numeric_bucket_bounds: Vec<i64>,
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
            burst_gap_ms: 50,
            idle_gap_ms: 5_000,
        }
    }
}

impl NormCfg {
    /// Label the numeric bucket `value` falls into. Negative numbers map
    /// symmetrically with a `-` prefix (`glossary.md` §4).
    #[must_use]
    pub fn bucket_label(&self, value: i64) -> String {
        if value < 0 {
            return format!("-{}", self.bucket_label(value.saturating_neg()));
        }
        let mut lower = 0_i64;
        for &bound in &self.numeric_bucket_bounds {
            if value <= bound {
                return if lower == bound {
                    bound.to_string()
                } else {
                    format!("{lower}-{bound}")
                };
            }
            lower = bound.saturating_add(1);
        }
        let last = self.numeric_bucket_bounds.last().copied().unwrap_or(0);
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
