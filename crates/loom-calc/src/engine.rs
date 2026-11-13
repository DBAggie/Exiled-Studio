//! The calculation (ADR-003, ADR-016; Phase 3 plan, decision 7): a pure
//! function over an immutable snapshot.
//!
//! [`calculate_with`] checks the data version, lets the game's
//! [`Ruleset`] collect the build's content into the modifier store, then
//! runs the ruleset's phases in order. Each phase reads the store and
//! writes fields with their graph nodes. Between steps it polls the host's
//! cancellation hook. No threads, clocks, or I/O; nothing iterates a hash
//! map, so two runs give the same result to the bit.

use std::collections::BTreeMap;

use loom_domain::Game;

use crate::condition::{ConditionKey, Flags, QueryContext};
use crate::error::CalcError;
use crate::graph::Graph;
use crate::ingest::{Ingest, StatMapping};
use crate::input::CalcInput;
use crate::poe1;
use crate::quantity::Quantity;
use crate::result::{CalcResult, Field, FieldResult};
use crate::source::Source;
use crate::state::{FieldState, Reasons, UnsupportedReason};
use crate::store::{ModStore, Query};

/// One step of a game's calculation.
#[derive(Clone, Copy)]
pub struct Phase {
    /// Its name.
    pub name: &'static str,
    /// What it does.
    pub run: fn(&mut Calc<'_>) -> Result<(), CalcError>,
}

impl core::fmt::Debug for Phase {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Phase").field("name", &self.name).finish()
    }
}

/// A game's calculation rules. Game-specific rules live in the game's
/// module (`loom_calc::poe1`); the core only runs them (ADR-001).
pub trait Ruleset {
    /// The game.
    fn game(&self) -> Game;

    /// The fields the game's slice has. Each one a phase does not write is
    /// reported `Unsupported` with `NotComputed`.
    fn slice(&self) -> &[Field];

    /// The stat-to-effect mapping.
    fn mapping(&self) -> &dyn StatMapping;

    /// Adds the build's content (items, passives, gems, the character's
    /// base values) to `ingest`.
    ///
    /// # Errors
    /// A [`CalcError`] that stops the calculation (for example a missing
    /// data row).
    fn collect(&self, input: &CalcInput<'_>, ingest: &mut Ingest<'_>) -> Result<(), CalcError>;

    /// The phases, in order.
    fn phases(&self) -> &[Phase];

    /// The named conditions the build settles (P3-8g), computed once per
    /// calculation, after `collect` and before the phases; every query
    /// context a phase builds with [`Calc::context`] holds a copy. They are
    /// build-wide: a condition that differs per skill or per hand needs
    /// another hook. The default settles none.
    fn states(&self, input: &CalcInput<'_>) -> BTreeMap<ConditionKey, bool> {
        let _ = input;
        BTreeMap::new()
    }
}

/// The state phases work on.
#[derive(Debug)]
pub struct Calc<'a> {
    input: &'a CalcInput<'a>,
    store: ModStore,
    graph: Graph,
    global: Reasons,
    fields: BTreeMap<Field, FieldResult>,
    states: BTreeMap<ConditionKey, bool>,
    skill_sources: std::collections::BTreeSet<Source>,
}

impl<'a> Calc<'a> {
    /// A query context with `flags` and the conditions the rules settled
    /// for this calculation ([`Ruleset::states`]).
    #[must_use]
    pub fn context(&self, flags: Flags) -> QueryContext {
        QueryContext {
            flags,
            states: self.states.clone(),
        }
    }

    /// The input.
    #[must_use]
    pub const fn input(&self) -> &CalcInput<'a> {
        self.input
    }

    /// The modifier store.
    #[must_use]
    pub const fn store(&self) -> &ModStore {
        &self.store
    }

    /// The graph, to combine query results.
    pub const fn graph_mut(&mut self) -> &mut Graph {
        &mut self.graph
    }

    /// The store and the graph together, to query the store into the
    /// graph.
    pub const fn parts(&mut self) -> (&ModStore, &mut Graph) {
        (&self.store, &mut self.graph)
    }

    /// A field written by an earlier phase.
    #[must_use]
    pub fn field(&self, field: Field) -> Option<&FieldResult> {
        self.fields.get(&field)
    }

    /// Names the sources whose effects form the computed skill's own list
    /// (PoB's `skillModList` without its parent): the rules' more
    /// multipliers round that list apart from the character's.
    pub fn set_skill_sources(&mut self, sources: std::collections::BTreeSet<Source>) {
        self.skill_sources = sources;
    }

    /// Whether `source` is one of [`Calc::set_skill_sources`]'.
    #[must_use]
    pub fn is_skill_source(&self, source: &Source) -> bool {
        self.skill_sources.contains(source)
    }

    /// Settles a condition from a value an earlier phase computed (a
    /// threshold on a field), for the later phases' contexts. A condition
    /// the rules settled before the phases keeps its value.
    pub fn settle_state(&mut self, key: ConditionKey, value: bool) {
        self.states.entry(key).or_insert(value);
    }

    /// `base * increased * more` on `quantity` (or its override); see
    /// [`ModStore::chain`].
    ///
    /// # Errors
    /// [`CalcError::Graph`].
    pub fn chain(&mut self, quantity: Quantity, cx: &QueryContext) -> Result<Query, CalcError> {
        Ok(self.store.chain(&mut self.graph, quantity, cx)?)
    }

    /// Writes a field from a query: its value and node, and its state from
    /// the query's reasons and the reasons that apply to every field. A
    /// later phase may replace the field.
    pub fn set(&mut self, field: Field, query: Query) {
        let mut reasons = query.reasons;
        reasons.extend(&self.global);
        self.fields.insert(
            field,
            FieldResult {
                value: Some(query.value),
                node: Some(query.node),
                state: reasons.into_state(),
            },
        );
    }

    /// Writes a field computed from other fields (`deps`): its state also
    /// carries every reason of each dependency, so a derived field is never
    /// `Complete` when what it reads is not. A dependency that is not
    /// computed (absent or `Unsupported`) makes the field `Unsupported`
    /// with `DependsOnNotComputed`, without a value.
    pub fn set_derived(&mut self, field: Field, query: Query, deps: &[Field]) {
        let mut reasons = query.reasons;
        reasons.extend(&self.global);
        for dep in deps {
            match self.fields.get(dep).map(|f| &f.state) {
                None | Some(FieldState::Unsupported(_)) => {
                    self.fields.insert(
                        field,
                        FieldResult::unsupported(UnsupportedReason::DependsOnNotComputed {
                            field: *dep,
                        }),
                    );
                    return;
                }
                Some(FieldState::Complete) => {}
                Some(FieldState::Partial(p)) => reasons.partial.extend(p.iter().cloned()),
                Some(FieldState::Indeterminate {
                    reasons: i,
                    partial: p,
                }) => {
                    reasons.indeterminate.extend(i.iter().cloned());
                    reasons.partial.extend(p.iter().cloned());
                }
            }
        }
        self.fields.insert(
            field,
            FieldResult {
                value: Some(query.value),
                node: Some(query.node),
                state: reasons.into_state(),
            },
        );
    }
}

/// Fails with [`CalcError::Cancelled`] when the hook says so.
fn poll(cancel: &dyn Fn() -> bool) -> Result<(), CalcError> {
    if cancel() {
        Err(CalcError::Cancelled)
    } else {
        Ok(())
    }
}

/// Calculates `input` with `rules`, polling `cancel` before collecting,
/// after collecting, and after each phase.
///
/// # Errors
/// [`CalcError::NoDataVersion`] or [`CalcError::DataVersionMismatch`]
/// first; [`CalcError::Cancelled`] when the hook returns `true`; any error
/// of the rules.
pub fn calculate_with(
    input: &CalcInput<'_>,
    rules: &dyn Ruleset,
    cancel: &dyn Fn() -> bool,
) -> Result<CalcResult, CalcError> {
    input.check_data_version()?;
    let game = input.build.game();
    if rules.game() != game {
        return Ok(CalcResult::game_unsupported(game));
    }
    poll(cancel)?;
    let mut ingest = Ingest::new(rules.mapping()).with_item_lines(&input.options.item_lines);
    rules.collect(input, &mut ingest)?;
    let (store, global) = ingest.finish();
    poll(cancel)?;
    let states = rules.states(input);
    let mut calc = Calc {
        input,
        store,
        graph: Graph::new(),
        global,
        fields: BTreeMap::new(),
        states,
        skill_sources: std::collections::BTreeSet::new(),
    };
    for phase in rules.phases() {
        (phase.run)(&mut calc)?;
        poll(cancel)?;
    }
    let Calc {
        graph,
        mut fields,
        global,
        ..
    } = calc;
    for field in rules.slice() {
        fields
            .entry(*field)
            .or_insert_with(|| FieldResult::unsupported(UnsupportedReason::NotComputed));
    }
    let state = CalcResult::worst(&fields);
    Ok(CalcResult {
        game,
        fields,
        graph,
        state,
        reasons: global,
    })
}

/// Calculates `input` with its game's rules. A game without rules gives
/// an `Unsupported` result, never another game's calculation (ADR-001).
///
/// # Errors
/// As [`calculate_with`].
pub fn calculate_cancellable(
    input: &CalcInput<'_>,
    cancel: &dyn Fn() -> bool,
) -> Result<CalcResult, CalcError> {
    match input.build.game() {
        Game::Poe1 => calculate_with(input, &poe1::Poe1Rules, cancel),
        Game::Poe2 => {
            input.check_data_version()?;
            poll(cancel)?;
            Ok(CalcResult::game_unsupported(Game::Poe2))
        }
    }
}

/// [`calculate_cancellable`] without cancellation.
///
/// # Errors
/// As [`calculate_with`].
pub fn calculate(input: &CalcInput<'_>) -> Result<CalcResult, CalcError> {
    calculate_cancellable(input, &|| false)
}
