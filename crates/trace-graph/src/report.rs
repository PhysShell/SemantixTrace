//! Workflow analytics report: top-N workflows, rare-but-failing, dead
//! features.
//!
//! This module provides the first concrete analytics-projection output
//! described in ADR-0011.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use trace_core::{CanonicalAction, CommandId, Scenario};
use uuid::Uuid;

use crate::builder::from_scenarios;
use crate::prefixspan::{prefixspan, FrequentPattern};

/// A top-N workflow entry.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TopWorkflow {
    /// The ordered sequence of canonical actions that form this workflow.
    pub actions: Vec<CanonicalAction>,
    /// Number of input sessions that contain this workflow as a
    /// subsequence.
    pub frequency: usize,
}

/// A workflow that is both rare (low frequency) and prone to failure
/// (high error rate).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RareFailingWorkflow {
    /// The canonical action sequence.
    pub actions: Vec<CanonicalAction>,
    /// Absolute number of occurrences.
    pub frequency: usize,
    /// `frequency / total_sessions`.
    pub frequency_fraction: f64,
    /// Fraction of sessions containing this workflow that also had an
    /// error event.
    pub error_rate: f64,
}

/// A command that appears rarely or not at all in the trace corpus —
/// a dead-feature candidate.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeadFeature {
    /// The command that appears to be unused or under-used.
    pub command_id: CommandId,
    /// Total observed visit count (may be zero for commands absent from
    /// the corpus).
    pub frequency: usize,
    /// `frequency / total_visits_across_all_nodes`.
    pub frequency_fraction: f64,
}

/// Full analytics report for a trace corpus.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WorkflowReport {
    /// Most-frequent workflows, sorted by descending frequency.
    pub top_workflows: Vec<TopWorkflow>,
    /// Rare-but-failing workflows.
    pub rare_failing: Vec<RareFailingWorkflow>,
    /// Dead-feature candidates.
    pub dead_features: Vec<DeadFeature>,
}

/// Thresholds and limits used when building a [`WorkflowReport`].
#[derive(Clone, Copy, Debug)]
pub struct WorkflowReportConfig {
    /// Maximum number of top-N workflows to include (default `20`).
    pub top_n: usize,
    /// Maximum relative frequency to be "rare" (default `0.01`).
    pub rare_freq_threshold: f64,
    /// Minimum error rate to be "failing" (default `0.05`).
    pub error_rate_threshold: f64,
    /// Floor fraction for dead-feature detection (default `0.01`).
    pub dead_floor_fraction: f64,
}

impl Default for WorkflowReportConfig {
    fn default() -> Self {
        Self {
            top_n: 20,
            rare_freq_threshold: 0.01,
            error_rate_threshold: 0.05,
            dead_floor_fraction: 0.01,
        }
    }
}

/// Build a [`WorkflowReport`] from a corpus of normalized scenarios.
///
/// # Parameters
///
/// * `scenarios` — `(session_id, scenario)` pairs.
/// * `error_sessions` — set of session IDs that contain an
///   `ExceptionThrown` event.
/// * `cfg` — thresholds and limits.
#[must_use]
pub fn build_report<S: ::std::hash::BuildHasher>(
    scenarios: &[(Uuid, Scenario)],
    error_sessions: &HashSet<Uuid, S>,
    cfg: &WorkflowReportConfig,
) -> WorkflowReport {
    if scenarios.is_empty() {
        return WorkflowReport::default();
    }

    let total_sessions = scenarios.len();
    let all_scenarios: Vec<Scenario> = scenarios.iter().map(|(_, s)| s.clone()).collect();
    let sequences: Vec<Vec<CanonicalAction>> =
        all_scenarios.iter().map(|s| s.actions.clone()).collect();

    // Mine patterns with support >= 1.
    let patterns = prefixspan(&sequences, 1);

    // Build action graph for dead-feature detection.
    let graph = from_scenarios(&all_scenarios);
    let total_visits: u64 = graph.graph().node_weights().map(|n| n.visit_count).sum();

    // ── Top-N workflows ────────────────────────────────────────────────
    let mut sorted_patterns: Vec<FrequentPattern> = patterns.clone();
    sorted_patterns.sort_by(|a, b| b.support.cmp(&a.support));
    let top_workflows: Vec<TopWorkflow> = sorted_patterns
        .iter()
        .take(cfg.top_n)
        .map(|p| TopWorkflow {
            actions: p.sequence.clone(),
            frequency: p.support,
        })
        .collect();

    // ── Rare-but-failing workflows ──────────────────────────────────────
    let mut rare_failing: Vec<RareFailingWorkflow> = Vec::new();

    #[allow(clippy::cast_precision_loss)]
    for pattern in &patterns {
        let freq_fraction = pattern.support as f64 / total_sessions as f64;
        if freq_fraction > cfg.rare_freq_threshold {
            continue;
        }

        // Count how many sessions containing this pattern also had errors.
        let matching_with_error = scenarios
            .iter()
            .filter(|(sid, scenario)| {
                error_sessions.contains(sid)
                    && crate::prefixspan::contains_as_subsequence(
                        &scenario.actions,
                        &pattern.sequence,
                    )
            })
            .count();

        #[allow(clippy::cast_precision_loss)]
        let error_rate = if pattern.support == 0 {
            0.0
        } else {
            matching_with_error as f64 / pattern.support as f64
        };

        if error_rate >= cfg.error_rate_threshold {
            rare_failing.push(RareFailingWorkflow {
                actions: pattern.sequence.clone(),
                frequency: pattern.support,
                frequency_fraction: freq_fraction,
                error_rate,
            });
        }
    }
    rare_failing.sort_by(|a, b| {
        b.error_rate
            .partial_cmp(&a.error_rate)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // ── Dead features ────────────────────────────────────────────────────
    let below = graph.commands_below_floor(total_visits, cfg.dead_floor_fraction);
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    let dead_features: Vec<DeadFeature> = below
        .into_iter()
        .map(|(cmd, count)| DeadFeature {
            command_id: cmd,
            frequency: count as usize,
            frequency_fraction: if total_visits == 0 {
                0.0
            } else {
                count as f64 / total_visits as f64
            },
        })
        .collect();

    WorkflowReport {
        top_workflows,
        rare_failing,
        dead_features,
    }
}
