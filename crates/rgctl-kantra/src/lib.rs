//! Native Konveyor Kantra rule evaluation against rgctl graphs and source cache.

pub mod catalog;
pub mod classify;
pub mod engine;
pub mod error;
pub mod eval;
pub mod findings;
pub mod index;
pub mod loader;
pub mod schema;

pub use catalog::{KantraCatalog, merge_ruleset_labels, rule_matches_target};
pub use engine::{EvalContext, EvalEdge, EvalGraph, EvalNode, EvalStageTimings, KantraEngine};
pub use error::KantraError;
pub use findings::KantraFindings;
pub use index::rewrite_snapshot_with_catalog;
pub use loader::KantraRuleset;
pub use schema::{RuleSupport, WhenClause};
