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

/// Whose values a step computes (Phase 6 P6-7b2, plan decision 7): the
/// character, or the main skill's minion (a second actor in the shared
/// core; PoE2 has minions too).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Actor {
    /// The character.
    Player,
    /// The main skill's minion.
    Minion,
}

/// What each actor keeps apart (P6-7b2): its modifier store, the reasons
/// collecting it gave, and its settled conditions.
#[derive(Debug)]
pub struct ActorState {
    /// The modifier store.
    pub store: ModStore,
    /// The reasons collecting it gave.
    pub global: Reasons,
    /// Its settled conditions.
    pub states: BTreeMap<ConditionKey, bool>,
    /// The sources of its computed skill's own list
    /// ([`Calc::set_skill_sources`]; per actor, P6-7b2 review).
    pub skill_sources: std::collections::BTreeSet<Source>,
}

/// The state phases work on. Store, reasons, and states are the current
/// actor's ([`Calc::with_actor`]); the graph and the fields are shared, so
/// a minion's node can read the character's.
#[derive(Debug)]
pub struct Calc<'a> {
    input: &'a CalcInput<'a>,
    player: ActorState,
    minion: Option<ActorState>,
    actor: Actor,
    graph: Graph,
    fields: BTreeMap<Field, FieldResult>,
}

impl<'a> Calc<'a> {
    /// The current actor's state.
    const fn current(&self) -> &ActorState {
        match (self.actor, &self.minion) {
            (Actor::Minion, Some(m)) => m,
            _ => &self.player,
        }
    }

    /// The current actor's state, mutable.
    const fn current_mut(&mut self) -> &mut ActorState {
        match (self.actor, &mut self.minion) {
            (Actor::Minion, Some(m)) => m,
            _ => &mut self.player,
        }
    }

    /// The current actor.
    #[must_use]
    pub const fn actor(&self) -> Actor {
        self.actor
    }

    /// Adds the main skill's minion with its collected state (P6-7b2).
    pub fn add_minion(&mut self, minion: ActorState) {
        self.minion = Some(minion);
    }

    /// Whether the calculation has a minion actor.
    #[must_use]
    pub const fn has_minion(&self) -> bool {
        self.minion.is_some()
    }

    /// Runs `f` with `actor` current, then restores the actor before;
    /// `None` without running `f` for [`Actor::Minion`] when there is no
    /// minion (P6-7b2 review: a minion step never falls back to the
    /// character's store).
    pub fn with_actor<R>(&mut self, actor: Actor, f: impl FnOnce(&mut Self) -> R) -> Option<R> {
        if actor == Actor::Minion && self.minion.is_none() {
            return None;
        }
        let before = self.actor;
        self.actor = actor;
        let out = f(self);
        self.actor = before;
        Some(out)
    }

    /// A query context with `flags` and the conditions the rules settled
    /// for this calculation ([`Ruleset::states`]), the current actor's.
    #[must_use]
    pub fn context(&self, flags: Flags) -> QueryContext {
        QueryContext {
            flags,
            states: self.current().states.clone(),
        }
    }

    /// The input.
    #[must_use]
    pub const fn input(&self) -> &CalcInput<'a> {
        self.input
    }

    /// The current actor's modifier store.
    #[must_use]
    pub const fn store(&self) -> &ModStore {
        &self.current().store
    }

    /// The graph, to combine query results.
    pub const fn graph_mut(&mut self) -> &mut Graph {
        &mut self.graph
    }

    /// The store and the graph together, to query the store into the
    /// graph.
    pub fn parts(&mut self) -> (&ModStore, &mut Graph) {
        let store = match (self.actor, &self.minion) {
            (Actor::Minion, Some(m)) => &m.store,
            _ => &self.player.store,
        };
        (store, &mut self.graph)
    }

    /// The reasons collecting the current actor gave (unmapped stats,
    /// unmodelled items, missing data): its own only. A minion's fields
    /// carry the character's too (`field_reasons`); the result's build-wide
    /// reasons are the character's (a minion's reach the result through its
    /// fields' states).
    #[must_use]
    pub const fn global(&self) -> &Reasons {
        &self.current().global
    }

    /// The reasons a field of the current actor carries: the character's;
    /// a minion's carry the character's too (an unmapped "Minions have ..."
    /// line lands in the character's).
    fn field_reasons(&self) -> Reasons {
        let mut r = self.player.global.clone();
        if self.actor == Actor::Minion
            && let Some(m) = &self.minion
        {
            r.extend(&m.global);
        }
        r
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
        self.current_mut().skill_sources = sources;
    }

    /// Whether `source` is one of [`Calc::set_skill_sources`]'.
    #[must_use]
    pub fn is_skill_source(&self, source: &Source) -> bool {
        self.current().skill_sources.contains(source)
    }

    /// Settles a condition from a value an earlier phase computed (a
    /// threshold on a field), for the later phases' contexts. A condition
    /// the rules settled before the phases keeps its value.
    pub fn settle_state(&mut self, key: ConditionKey, value: bool) {
        self.current_mut().states.entry(key).or_insert(value);
    }

    /// `base * increased * more` on `quantity` (or its override); see
    /// [`ModStore::chain`].
    ///
    /// # Errors
    /// [`CalcError::Graph`].
    pub fn chain(&mut self, quantity: Quantity, cx: &QueryContext) -> Result<Query, CalcError> {
        let (store, graph) = self.parts();
        Ok(store.chain(graph, quantity, cx)?)
    }

    /// Writes a field from a query: its value and node, and its state from
    /// the query's reasons and the reasons that apply to every field. A
    /// later phase may replace the field.
    pub fn set(&mut self, field: Field, query: Query) {
        let mut reasons = query.reasons;
        reasons.extend(&self.field_reasons());
        self.fields.insert(
            field,
            FieldResult {
                value: Some(query.value),
                node: Some(query.node),
                state: reasons.into_state(),
            },
        );
    }

    /// Writes a field the engine does not compute for this build, with
    /// `reason` (Phase 6: a field that applies but whose kind is not
    /// modelled).
    pub fn set_unsupported(&mut self, field: Field, reason: UnsupportedReason) {
        self.fields.insert(field, FieldResult::unsupported(reason));
    }

    /// Writes a field computed from other fields (`deps`): its state also
    /// carries every reason of each dependency, so a derived field is never
    /// `Complete` when what it reads is not. A dependency that is not
    /// computed (absent or `Unsupported`) makes the field `Unsupported`
    /// with `DependsOnNotComputed`, without a value.
    pub fn set_derived(&mut self, field: Field, query: Query, deps: &[Field]) {
        let mut reasons = query.reasons;
        reasons.extend(&self.field_reasons());
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

/// `input` collected with `rules`, with the states it settles: a
/// calculation before its phases (the data version is the caller's to
/// check).
///
/// # Errors
/// The rules' collect errors.
pub(crate) fn collected<'a>(
    input: &'a CalcInput<'a>,
    rules: &dyn Ruleset,
) -> Result<Calc<'a>, CalcError> {
    let mut ingest = Ingest::new(rules.mapping()).with_item_lines(&input.options.item_lines);
    rules.collect(input, &mut ingest)?;
    let (store, global) = ingest.finish();
    let states = rules.states(input);
    Ok(Calc {
        input,
        player: ActorState {
            store,
            global,
            states,
            skill_sources: std::collections::BTreeSet::new(),
        },
        minion: None,
        actor: Actor::Player,
        graph: Graph::new(),
        fields: BTreeMap::new(),
    })
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
    let mut calc = collected(input, rules)?;
    poll(cancel)?;
    for phase in rules.phases() {
        (phase.run)(&mut calc)?;
        poll(cancel)?;
    }
    let Calc {
        graph,
        mut fields,
        player,
        ..
    } = calc;
    let global = player.global;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::UnsupportedReason;

    /// Data with nothing (the tests read none).
    struct NoData;

    impl crate::input::CalcData for NoData {
        fn data_version(&self) -> loom_domain::DataVersion {
            loom_domain::DataVersion {
                game: Game::Poe1,
                schema_version: 0,
                steam_build: None,
            }
        }
        fn character(&self, _: &str) -> Option<crate::input::CharacterBase> {
            None
        }
        fn character_constant(&self, _: &str) -> Option<i64> {
            None
        }
        fn monster_constant(&self, _: &str) -> Option<i64> {
            None
        }
        fn passive_stats(&self, _: u32) -> Option<&[crate::input::StatAmount]> {
            None
        }
        fn mastery_effect_stats(&self, _: u32) -> Option<&[crate::input::StatAmount]> {
            None
        }
        fn item_base(&self, _: &str) -> Option<crate::input::ItemBase> {
            None
        }
        fn skill_gem(&self, _: &str) -> Option<&crate::input::SkillGem> {
            None
        }
        fn gem_effect(&self, _: &str) -> Option<&crate::input::GemEffect> {
            None
        }
        fn granted_effect(&self, _: &str) -> Option<&crate::input::GrantedEffect> {
            None
        }
        fn active_skill(&self, _: &str) -> Option<&crate::input::ActiveSkill> {
            None
        }
        fn stat_set(&self, _: &str) -> Option<&crate::input::StatSet> {
            None
        }
        fn monster_level(&self, _: i64) -> Option<crate::input::MonsterStats> {
            None
        }
        fn monster_type(&self, _: &str) -> Option<crate::input::MonsterDefences> {
            None
        }
        fn game_constant(&self, _: &str) -> Option<(i64, i64)> {
            None
        }
    }

    fn state(reason: Option<&'static str>) -> ActorState {
        let mut global = Reasons::new();
        if let Some(what) = reason {
            global.partial.insert(UnsupportedReason::UnmodelledSkill {
                source: Source::Rule(what),
                what,
            });
        }
        ActorState {
            store: ModStore::default(),
            global,
            states: BTreeMap::new(),
            skill_sources: std::collections::BTreeSet::new(),
        }
    }

    /// A calculation's state with the given player state, as `collected`
    /// builds it, without an input (the tests read none).
    fn calc(input: &'static CalcInput<'static>) -> Calc<'static> {
        Calc {
            input,
            player: state(Some("player")),
            minion: None,
            actor: Actor::Player,
            graph: Graph::new(),
            fields: BTreeMap::new(),
        }
    }

    /// The per-actor state (P6-7b2 review): `with_actor` refuses a missing
    /// minion and restores the actor; states, skill sources, and reasons
    /// stay apart; a minion's field carries both actors' reasons.
    #[test]
    fn actors_should_keep_their_state_apart() {
        let build: &'static loom_domain::Build =
            Box::leak(Box::new(loom_domain::Build::new(Game::Poe1)));
        let options: &'static crate::input::CalcOptions = Box::leak(Box::default());
        let input: &'static CalcInput<'static> = Box::leak(Box::new(CalcInput {
            build,
            data: &NoData,
            options,
        }));
        let mut c = calc(input);
        assert_eq!(c.with_actor(Actor::Minion, |_| ()), None);
        assert_eq!(c.actor(), Actor::Player);
        c.add_minion(state(Some("minion")));
        let key = ConditionKey::new("FullLife");
        c.with_actor(Actor::Minion, |c| {
            assert_eq!(c.actor(), Actor::Minion);
            c.settle_state(key.clone(), true);
            c.set_skill_sources([Source::Character].into_iter().collect());
            assert_eq!(c.global().partial.len(), 1);
            assert_eq!(c.field_reasons().partial.len(), 2);
        });
        assert_eq!(c.actor(), Actor::Player);
        assert!(!c.context(Flags::NONE).states.contains_key(&key));
        assert!(!c.is_skill_source(&Source::Character));
        assert_eq!(c.field_reasons().partial.len(), 1);
        assert!(c.with_actor(Actor::Minion, |c| c.context(Flags::NONE).states[&key]) == Some(true));
    }

    #[test]
    fn minion_fields_should_be_named_and_sorted_last() {
        use crate::quantity::DamageType;
        use crate::result::MinionField;
        assert_eq!(Field::Minion(MinionField::Life).to_string(), "minion.life");
        assert_eq!(
            Field::Minion(MinionField::Resistance(DamageType::Cold)).to_string(),
            "minion.resistance.cold"
        );
        assert!(Field::Minion(MinionField::Life) > Field::PoisonDps);
    }
}
