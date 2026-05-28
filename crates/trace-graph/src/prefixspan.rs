//! `PrefixSpan` frequent-subsequence mining.
//!
//! A subsequence `[a, b, c]` is "contained in" sequence `s` if there
//! exist indices `i1 < i2 < i3` such that `s[i1]=a, s[i2]=b, s[i3]=c`
//! (non-contiguous matches are allowed).

use std::collections::HashMap;

use trace_core::CanonicalAction;

/// A frequent sequential pattern and its support count.
///
/// `support` is the number of input sequences that contain `sequence`
/// as a (possibly non-contiguous) subsequence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrequentPattern {
    /// The pattern sequence.
    pub sequence: Vec<CanonicalAction>,
    /// Number of input sequences that contain this pattern as a
    /// subsequence.
    pub support: usize,
}

/// Mine all frequent subsequences from `sequences` using the `PrefixSpan`
/// algorithm.
///
/// Returns every pattern whose `support >= min_support`.  An empty
/// `min_support` of `0` would return infinitely many patterns; callers
/// must pass `min_support >= 1`.
///
/// # Panics
///
/// Does not panic on well-formed inputs.
#[must_use]
pub fn prefixspan(sequences: &[Vec<CanonicalAction>], min_support: usize) -> Vec<FrequentPattern> {
    if min_support == 0 || sequences.is_empty() {
        return Vec::new();
    }
    let mut results = Vec::new();
    // Start with a projected DB that is the original sequences paired
    // with their respective indices (for support counting against the
    // original).
    let indexed: Vec<(usize, Vec<CanonicalAction>)> = sequences
        .iter()
        .enumerate()
        .map(|(i, s)| (i, s.clone()))
        .collect();
    prefixspan_inner(&[], &indexed, min_support, &mut results);
    results
}

/// Recursive `PrefixSpan` step.
///
/// `prefix` is the current global prefix.
/// `db` contains `(original_index, projected_suffix)` pairs.
///
/// The projected suffix is the portion of the original sequence that
/// comes **after** the point where `prefix` was matched.  On the first
/// call every projected suffix equals the full original sequence (because
/// the empty prefix has been trivially matched at position 0).
fn prefixspan_inner(
    prefix: &[CanonicalAction],
    db: &[(usize, Vec<CanonicalAction>)],
    min_support: usize,
    results: &mut Vec<FrequentPattern>,
) {
    // Count how many *distinct* original sequences contribute each item.
    let mut item_support: HashMap<CanonicalAction, usize> = HashMap::new();
    for (_, suffix) in db {
        let mut seen = std::collections::HashSet::new();
        for item in suffix {
            if seen.insert(item.clone()) {
                *item_support.entry(item.clone()).or_insert(0) += 1;
            }
        }
    }

    for (item, support) in item_support {
        if support < min_support {
            continue;
        }

        let mut new_prefix = prefix.to_vec();
        new_prefix.push(item.clone());

        // Build the projected database for `new_prefix`: for each entry
        // in `db`, find the *first* occurrence of `item` in its suffix and
        // take everything after that position.
        let projected: Vec<(usize, Vec<CanonicalAction>)> = db
            .iter()
            .filter_map(|(orig_idx, suffix)| {
                // Find the first position of `item` in this suffix.
                let pos = suffix.iter().position(|a| a == &item)?;
                Some((*orig_idx, suffix[pos + 1..].to_vec()))
            })
            .collect();

        // The actual support equals the number of entries we have in the
        // projected database (each entry comes from a distinct original
        // sequence by construction).
        let actual_support = projected.len();
        if actual_support < min_support {
            continue;
        }

        results.push(FrequentPattern {
            sequence: new_prefix.clone(),
            support: actual_support,
        });

        // Recurse only on non-empty projected suffixes.
        let non_empty: Vec<(usize, Vec<CanonicalAction>)> = projected
            .into_iter()
            .filter(|(_, s)| !s.is_empty())
            .collect();

        if !non_empty.is_empty() {
            prefixspan_inner(&new_prefix, &non_empty, min_support, results);
        }
    }
}

/// Return `true` if `subseq` is a (non-contiguous) subsequence of `seq`.
pub(crate) fn contains_as_subsequence(seq: &[CanonicalAction], subseq: &[CanonicalAction]) -> bool {
    if subseq.is_empty() {
        return true;
    }
    let mut j = 0;
    for item in seq {
        if item == &subseq[j] {
            j += 1;
            if j == subseq.len() {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use trace_core::{CanonicalAction, CommandId, ScreenId};

    use super::*;

    fn action(s: &str, c: &str) -> CanonicalAction {
        CanonicalAction {
            screen_id: ScreenId::new(s),
            command_id: CommandId::new(c),
            abstract_args: json!({}),
        }
    }

    #[test]
    fn empty_input_returns_empty() {
        let patterns = prefixspan(&[], 1);
        assert!(patterns.is_empty());
    }

    #[test]
    fn single_element_sequence() {
        let a = action("S", "A");
        let seqs = vec![vec![a.clone()]];
        let patterns = prefixspan(&seqs, 1);
        assert!(patterns.iter().any(|p| p.sequence == vec![a.clone()]));
    }

    #[test]
    fn support_is_accurate() {
        let a = action("S", "A");
        let b = action("S", "B");
        let seqs = vec![
            vec![a.clone(), b.clone()],
            vec![a.clone(), b.clone()],
            vec![b.clone()],
        ];
        let patterns = prefixspan(&seqs, 2);
        let ab = patterns
            .iter()
            .find(|p| p.sequence == vec![a.clone(), b.clone()]);
        assert!(ab.is_some(), "expected pattern [a, b] with support 2");
        assert_eq!(ab.unwrap().support, 2);
    }

    #[test]
    fn min_support_filters_rare_patterns() {
        let a = action("S", "A");
        let b = action("S", "B");
        let seqs = vec![vec![a, b.clone()], vec![b]];
        // With min_support=2, only b should appear (support=2).
        let patterns = prefixspan(&seqs, 2);
        assert!(
            patterns.iter().all(|p| p.support >= 2),
            "all patterns should have support >= 2"
        );
    }
}
