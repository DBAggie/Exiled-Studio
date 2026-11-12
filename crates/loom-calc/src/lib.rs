//! Calculation engine. Consumes immutable build snapshots, computes in `f64`
//! with typed canonicalization at the boundaries, and emits a structured
//! provenance graph (ADR-003).
//!
//! Core crate (ADR-016): compiles for `wasm32-unknown-unknown` and does no
//! filesystem, thread, clock, environment, or network access. Scheduling,
//! cancellation, and parallelism live in the host layer; the engine only
//! polls a cancellation hook the host passes in.
//!
//! The engine core (Phase 3 plan, step 4):
//! - [`CalcInput`]: the resolved build, a [`CalcData`] view, and
//!   [`CalcOptions`]; the build's data version must be the data's.
//! - [`ModStore`]: effects keyed by [`Quantity`], [`Op`], and
//!   [`Conditions`], with sum, product, flag, and override queries that
//!   each record their provenance.
//! - [`Graph`]: the provenance graph; every recorded value equals the
//!   graph's evaluation ([`Graph::check`]).
//! - [`FieldState`] and [`UnsupportedReason`]: nothing unmodelled is
//!   ignored silently (ADR-014, ADR-015); [`CalcError`] with `CALC_*`
//!   codes for failures that stop a calculation.
//! - [`calculate`]: runs the game's [`Ruleset`]. Game-specific rules live
//!   in [`poe1`] (ADR-001); a PoE2 build gets an `Unsupported` result.
//! - [`math`]: the audited math module.

pub mod condition;
pub mod engine;
pub mod error;
pub mod graph;
pub mod ingest;
pub mod input;
pub mod math;
pub mod poe1;
pub mod quantity;
pub mod result;
pub mod source;
pub mod state;
pub mod store;

pub use condition::{ConditionKey, Conditions, Flags, QueryContext};
pub use engine::{Calc, Phase, Ruleset, calculate, calculate_cancellable, calculate_with};
pub use error::CalcError;
pub use graph::{Graph, GraphError, Node, NodeIndex, Term};
pub use ingest::{EffectTemplate, Ingest, StatMap, StatMapping, TypedMap, settle};
pub use input::{
    ActiveSkill, ArmourBase, BaseName, CHARACTER_TEMPLATE, CalcData, CalcInput, CalcOptions,
    CharacterBase, GemEffect, GrantedEffect, ItemBase, MonsterDefences, MonsterStats, PassiveLinks,
    Range, SkillChoice, SkillGem, StatAmount, StatSet,
};
pub use quantity::{DamageType, Op, Quantity};
pub use result::{CalcResult, Field, FieldResult};
pub use source::Source;
pub use state::{FieldState, IndeterminateReason, Reasons, StateKind, UnsupportedReason};
pub use store::{Effect, ModStore, Query};
