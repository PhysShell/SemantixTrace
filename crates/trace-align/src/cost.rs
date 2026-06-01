//! Cost function trait and built-in structural implementation.
//!
//! [`CostFn`] is the extension point for domain-specific alignment costs.
//! [`StructuralCost`] derives costs purely from event kind and primary
//! identifier — no external configuration is required for the default
//! use case.

#![forbid(unsafe_code)]

use std::collections::HashMap;

use trace_schema::v1::TraceEventKind;
use trace_schema::Current;

/// Trait for computing edit costs between two [`Current`] trace events.
///
/// Implement this to plug in domain-specific alignment heuristics.
/// The built-in implementation is [`StructuralCost`].
pub trait CostFn: std::fmt::Debug + Send + Sync {
    /// Cost to substitute event `a` with event `b`.
    ///
    /// A cost of `0.0` means a perfect match; higher values mean greater
    /// semantic distance.
    fn substitute(&self, a: &Current, b: &Current) -> f64;

    /// Cost to insert event `b` (present in `B` but not `A`).
    fn insert(&self, b: &Current) -> f64;

    /// Cost to delete event `a` (present in `A` but not `B`).
    fn delete(&self, a: &Current) -> f64;
}

/// Structural cost function that derives costs from event kind and
/// primary identifier.
///
/// No domain knowledge is required: the cost table is purely structural.
/// Use [`StructuralCost::with_override`] to inject pair-specific costs
/// via TOML configuration.
///
/// # Default cost table
///
/// | Situation                        | Cost                    |
/// |----------------------------------|-------------------------|
/// | Same kind *and* same primary id  | `match_cost` (0.0)      |
/// | Same kind, different primary id  | `same_kind_diff_id` (0.4) |
/// | Different kind                   | `diff_kind` (1.0)       |
/// | Indel (insert or delete)         | `indel` (0.6)           |
#[derive(Clone, Debug)]
pub struct StructuralCost {
    /// Cost when two events are an exact kind+id match (default `0.0`).
    pub match_cost: f64,
    /// Cost when two events share a kind but have different primary ids
    /// (default `0.4`).
    pub same_kind_diff_id: f64,
    /// Cost when two events have different kinds (default `1.0`).
    pub diff_kind: f64,
    /// Cost for a gap (insert or delete) operation (default `0.6`).
    pub indel: f64,
    /// Per-pair overrides keyed by `(label_a, label_b)`.
    ///
    /// Looked up symmetrically: `(a, b)` and `(b, a)` are equivalent.
    pub overrides: HashMap<(String, String), f64>,
}

impl Default for StructuralCost {
    fn default() -> Self {
        Self {
            match_cost: 0.0,
            same_kind_diff_id: 0.4,
            diff_kind: 1.0,
            indel: 0.6,
            overrides: HashMap::new(),
        }
    }
}

impl StructuralCost {
    /// Register a pair-specific substitution cost.
    ///
    /// The lookup is symmetric: `(a_label, b_label)` and
    /// `(b_label, a_label)` both hit the same override.
    #[must_use]
    pub fn with_override(
        mut self,
        a_label: impl Into<String>,
        b_label: impl Into<String>,
        cost: f64,
    ) -> Self {
        self.overrides
            .insert((a_label.into(), b_label.into()), cost);
        self
    }
}

/// Compute a human-readable label for a [`Current`] event.
///
/// The label encodes event kind name and primary identifier so that
/// structurally identical events produce identical labels:
///
/// - `ScreenOpened:DeclarationEditor`
/// - `CommandExecuted:Graph47.Recalculate`
/// - `NavigationOccurred:Home→Detail`
fn event_label(event: &Current) -> String {
    match &event.kind {
        TraceEventKind::ScreenOpened { screen_id, .. } => {
            format!("ScreenOpened:{screen_id}")
        }
        TraceEventKind::CommandExecuted { command_id, .. } => {
            format!("CommandExecuted:{command_id}")
        }
        TraceEventKind::FieldChanged { field_id, .. } => {
            format!("FieldChanged:{field_id}")
        }
        TraceEventKind::ExceptionThrown { exception_type, .. } => {
            format!("ExceptionThrown:{exception_type}")
        }
        TraceEventKind::NavigationOccurred { from, to } => {
            format!("NavigationOccurred:{from}\u{2192}{to}")
        }
        TraceEventKind::ValidationFailed {
            validator,
            field_id,
            ..
        } => {
            format!("ValidationFailed:{validator}:{field_id}")
        }
        TraceEventKind::AsyncOperationCompleted { operation_id, .. } => {
            format!("AsyncOperationCompleted:{operation_id}")
        }
    }
}

/// Return the event-kind discriminant as a string (without the primary id).
///
/// Used to determine whether two events share the same kind category
/// before comparing their identifiers.
const fn kind_name(event: &Current) -> &'static str {
    match &event.kind {
        TraceEventKind::ScreenOpened { .. } => "ScreenOpened",
        TraceEventKind::CommandExecuted { .. } => "CommandExecuted",
        TraceEventKind::FieldChanged { .. } => "FieldChanged",
        TraceEventKind::ExceptionThrown { .. } => "ExceptionThrown",
        TraceEventKind::NavigationOccurred { .. } => "NavigationOccurred",
        TraceEventKind::ValidationFailed { .. } => "ValidationFailed",
        TraceEventKind::AsyncOperationCompleted { .. } => "AsyncOperationCompleted",
    }
}

impl CostFn for StructuralCost {
    fn substitute(&self, a: &Current, b: &Current) -> f64 {
        let label_a = event_label(a);
        let label_b = event_label(b);

        // Check symmetric override table first.
        if let Some(&c) = self
            .overrides
            .get(&(label_a.clone(), label_b.clone()))
            .or_else(|| self.overrides.get(&(label_b.clone(), label_a.clone())))
        {
            return c;
        }

        if label_a == label_b {
            return self.match_cost;
        }

        if kind_name(a) == kind_name(b) {
            self.same_kind_diff_id
        } else {
            self.diff_kind
        }
    }

    fn insert(&self, _b: &Current) -> f64 {
        self.indel
    }

    fn delete(&self, _a: &Current) -> f64 {
        self.indel
    }
}
