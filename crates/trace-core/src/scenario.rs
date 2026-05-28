//! [`Scenario`] and the canonical-action triple it composes from.

use serde::{Deserialize, Serialize};

use crate::ids::{CommandId, ScreenId};

/// Canonical (screen, command, abstract-args) triple. Two events that
/// fold to the same triple after value/temporal abstraction
/// (`trace-normalizer`, S3) are considered the same canonical action.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CanonicalAction {
    /// Screen the action happened on.
    pub screen_id: ScreenId,
    /// Semantic command id.
    pub command_id: CommandId,
    /// Abstract arguments — bucketed numerics, classed strings, dropped
    /// noise. JSON-shaped so the abstraction surface can grow without
    /// schema churn (ADR-0006).
    pub abstract_args: serde_json::Value,
}

/// A normalized scenario: an ordered list of [`CanonicalAction`]s
/// derived from a [`Session`] by [`trace-normalizer`] (S3).
///
/// Aggregate-root for the normalization bounded context. Construction
/// of a real scenario happens in S3; this S1 type is the contract every
/// downstream crate (graph, oracle, replay-planner) reads from.
///
/// [`Session`]: crate::session::Session
/// [`trace-normalizer`]: ../../../trace_normalizer/index.html
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scenario {
    /// The canonical action sequence in order.
    pub actions: Vec<CanonicalAction>,
}

impl Scenario {
    /// Build a new scenario from a vector of canonical actions.
    #[must_use]
    pub const fn new(actions: Vec<CanonicalAction>) -> Self {
        Self { actions }
    }

    /// Number of actions in the scenario.
    #[must_use]
    pub fn len(&self) -> usize {
        self.actions.len()
    }

    /// True iff the scenario has zero actions.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::{CanonicalAction, CommandId, Scenario, ScreenId};

    #[test]
    fn scenario_round_trip() {
        let s = Scenario::new(vec![CanonicalAction {
            screen_id: ScreenId::new("DeclarationEditor"),
            command_id: CommandId::new("Graph47.Recalculate"),
            abstract_args: serde_json::json!({"goodsCountBucket": "11-100"}),
        }]);
        let json = serde_json::to_string(&s).expect("serialize");
        let back: Scenario = serde_json::from_str(&json).expect("parse");
        assert_eq!(s, back);
    }
}
