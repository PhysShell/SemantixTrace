//! Semantic edit-distance alignment for `SemantxTrace` traces (S8).
//!
//! Aligns two trace event sequences using the Needleman-Wunsch algorithm
//! with a configurable cost function.  The built-in [`StructuralCost`]
//! derives costs from event kind and identifier without requiring any
//! domain knowledge — see `docs/` for the v1 cost table.
//!
//! # Quick start
//!
//! ```rust,no_run
//! use trace_align::{align, StructuralCost};
//! # let events_a: Vec<trace_schema::Current> = vec![];
//! # let events_b: Vec<trace_schema::Current> = vec![];
//! let alignment = align(&events_a, &events_b, &StructuralCost::default());
//! println!("cost: {}", alignment.total_cost);
//! ```

#![forbid(unsafe_code)]

pub mod align;
pub mod cost;
pub mod nearest;

pub use align::{align, Alignment, EditOp, Op};
pub use cost::{CostFn, StructuralCost};
pub use nearest::nearest;
