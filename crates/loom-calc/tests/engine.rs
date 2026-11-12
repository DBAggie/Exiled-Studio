//! The engine core end to end (Phase 3 plan, step 4, "Accept"): the data
//! version check, cancellation, PoE1 and PoE2 results, `UnmappedStat`
//! instead of a silent zero, and the property tests (every output equals
//! its graph's evaluation, two runs give the same bytes, effect order does
//! not change results).
//!
//! The property tests draw their cases from a fixed-seed generator
//! (`SplitMix64`) instead of a property-testing crate, so the core gains no
//! dependency and every run checks the same cases.

use std::cell::Cell;

use loom_calc::{
    Calc, CalcData, CalcError, CalcInput, CalcOptions, CharacterBase, ConditionKey, Conditions,
    DamageType, Effect, EffectTemplate, Field, FieldState, Flags, IndeterminateReason, Ingest, Op,
    Phase, Quantity, QueryContext, Ruleset, Source, StatAmount, StatMap, StatMapping, StateKind,
    Term, UnsupportedReason, calculate, calculate_cancellable, calculate_with,
};
use loom_domain::{Build, DataVersion, Game, NodeId, StatId, StatLine, StatValue};

// ---------------------------------------------------------------- helpers

struct Data(DataVersion);

/// Data with a version and no table.
impl CalcData for Data {
    fn data_version(&self) -> DataVersion {
        self.0.clone()
    }
    fn item_base(&self, _: &str) -> Option<loom_calc::ItemBase> {
        None
    }
    fn character(&self, _: &str) -> Option<CharacterBase> {
        None
    }
    fn character_constant(&self, _: &str) -> Option<i64> {
        None
    }
    fn monster_constant(&self, _: &str) -> Option<i64> {
        None
    }
    fn passive_stats(&self, _: u32) -> Option<&[StatAmount]> {
        None
    }
    fn mastery_effect_stats(&self, _: u32) -> Option<&[StatAmount]> {
        None
    }
    fn passive_links(&self, _: u32) -> Option<loom_calc::PassiveLinks> {
        None
    }
    fn skill_gem(&self, _: &str) -> Option<&loom_calc::SkillGem> {
        None
    }
    fn gem_effect(&self, _: &str) -> Option<&loom_calc::GemEffect> {
        None
    }
    fn granted_effect(&self, _: &str) -> Option<&loom_calc::GrantedEffect> {
        None
    }
    fn active_skill(&self, _: &str) -> Option<&loom_calc::ActiveSkill> {
        None
    }
    fn stat_set(&self, _: &str) -> Option<&loom_calc::StatSet> {
        None
    }
    fn monster_level(&self, _: i64) -> Option<loom_calc::MonsterStats> {
        None
    }
    fn monster_type(&self, _: &str) -> Option<loom_calc::MonsterDefences> {
        None
    }
    fn game_constant(&self, _: &str) -> Option<(i64, i64)> {
        None
    }
}

fn version(game: Game) -> DataVersion {
    DataVersion {
        game,
        schema_version: 2,
        steam_build: Some(1),
    }
}

fn build(game: Game) -> Build {
    let mut b = Build::new(game);
    b.data = Some(version(game));
    b
}

fn stat(id: &str, value: i64) -> StatLine {
    StatLine {
        stat: StatId::new(id),
        value: StatValue::Exact {
            min: value,
            max: value,
        },
    }
}

/// Maps `base_life`, `inc_life`, `more_life`, `base_mana`; nothing else.
struct Mapping;

impl StatMapping for Mapping {
    fn map(&self, stat: &StatId) -> StatMap {
        let t = |quantity, op| {
            StatMap::Effects(vec![EffectTemplate {
                quantity,
                op,
                scale: 1.0,
                divisor: 1.0,
                conditions: Conditions::none(),
            }])
        };
        match stat.as_str() {
            "base_life" => t(Quantity::Life, Op::Base),
            "inc_life" => t(Quantity::Life, Op::Increased),
            "more_life" => t(Quantity::Life, Op::More),
            "base_mana" => t(Quantity::Mana, Op::Base),
            _ => StatMap::Unmapped,
        }
    }
}

/// The quantities the random phases read, with the field each writes.
const OUTPUTS: [(Quantity, Field); 4] = [
    (Quantity::Life, Field::Life),
    (Quantity::Mana, Field::Mana),
    (Quantity::Armour, Field::Armour),
    (
        Quantity::Resistance(DamageType::Fire),
        Field::Resistance(DamageType::Fire),
    ),
];

/// The context every phase queries in: a fire spell hit, `a` settled true,
/// `b` settled false, `c` unknown.
fn context() -> QueryContext {
    QueryContext::with_flags(Flags::SPELL | Flags::HIT | Flags::FIRE)
        .with_state(ConditionKey::new("a"), true)
        .with_state(ConditionKey::new("b"), false)
}

fn chains(calc: &mut Calc<'_>) -> Result<(), CalcError> {
    for (quantity, field) in OUTPUTS {
        let q = calc.chain(quantity, &context())?;
        calc.set(field, q);
    }
    Ok(())
}

/// Derives a field from two earlier ones, so the graph has nodes built by
/// a phase as well as by the store.
fn derived(calc: &mut Calc<'_>) -> Result<(), CalcError> {
    let life = calc.field(Field::Life).and_then(|f| f.node);
    let mana = calc.field(Field::Mana).and_then(|f| f.node);
    let (Some(life), Some(mana)) = (life, mana) else {
        return Ok(());
    };
    let g = calc.graph_mut();
    let sum = g.derive(loom_calc::Node::Sum {
        term: Term::Step("life-plus-mana"),
        terms: vec![life, mana],
    })?;
    let node = g.derive(loom_calc::Node::RoundHalfUp {
        term: Term::Result,
        of: sum,
    })?;
    let value = g.value(node).unwrap_or(f64::NAN);
    calc.set_derived(
        Field::HitDps,
        loom_calc::Query {
            value,
            node,
            reasons: loom_calc::Reasons::new(),
        },
        &[Field::Life, Field::Mana],
    );
    // From a field no phase writes: not computed.
    calc.set_derived(
        Field::CritChance,
        loom_calc::Query {
            value,
            node,
            reasons: loom_calc::Reasons::new(),
        },
        &[Field::Evasion],
    );
    Ok(())
}

const PHASES: [Phase; 2] = [
    Phase {
        name: "chains",
        run: chains,
    },
    Phase {
        name: "derived",
        run: derived,
    },
];

/// A synthetic ruleset: `stats` from the character and `effects` as
/// given; the phases write [`OUTPUTS`] as chains and `HitDps` from two of
/// them; the slice adds `Evasion`, which no phase writes.
struct Rules {
    game: Game,
    stats: Vec<StatLine>,
    effects: Vec<Effect>,
}

impl Rules {
    fn new(stats: Vec<StatLine>, effects: Vec<Effect>) -> Self {
        Self {
            game: Game::Poe1,
            stats,
            effects,
        }
    }
}

impl Ruleset for Rules {
    fn game(&self) -> Game {
        self.game
    }

    fn slice(&self) -> &[Field] {
        &[
            Field::Life,
            Field::Mana,
            Field::Armour,
            Field::Resistance(DamageType::Fire),
            Field::HitDps,
            Field::Evasion,
        ]
    }

    fn mapping(&self) -> &dyn StatMapping {
        &Mapping
    }

    fn collect(&self, _input: &CalcInput<'_>, ingest: &mut Ingest<'_>) -> Result<(), CalcError> {
        ingest.stats(&Source::Character, &self.stats);
        for e in &self.effects {
            ingest.effect(e.clone());
        }
        Ok(())
    }

    fn phases(&self) -> &[Phase] {
        &PHASES
    }
}

fn run(b: &Build, rules: &Rules) -> Result<loom_calc::CalcResult, CalcError> {
    let data = Data(version(b.game()));
    let options = CalcOptions::default();
    let input = CalcInput {
        build: b,
        data: &data,
        options: &options,
    };
    calculate_with(&input, rules, &|| false)
}

// ------------------------------------------------------------ data version

#[test]
fn a_build_without_a_data_version_should_fail() {
    let mut b = build(Game::Poe1);
    b.data = None;
    let err = run(&b, &Rules::new(vec![], vec![])).unwrap_err();
    assert_eq!(err.code(), "CALC_NO_DATA_VERSION");
}

#[test]
fn a_build_of_other_data_should_fail() {
    let b = build(Game::Poe1);
    let options = CalcOptions::default();
    for other in [
        DataVersion {
            schema_version: 1,
            ..version(Game::Poe1)
        },
        DataVersion {
            steam_build: Some(2),
            ..version(Game::Poe1)
        },
        DataVersion {
            steam_build: None,
            ..version(Game::Poe1)
        },
        version(Game::Poe2),
    ] {
        let data = Data(other);
        let input = CalcInput {
            build: &b,
            data: &data,
            options: &options,
        };
        let err = calculate(&input).unwrap_err();
        assert_eq!(err.code(), "CALC_DATA_VERSION");
    }
}

#[test]
fn a_build_whose_game_is_not_the_datas_should_fail() {
    // A PoE2 build that records PoE1 data, against PoE1 data.
    let mut b = build(Game::Poe2);
    b.data = Some(version(Game::Poe1));
    let data = Data(version(Game::Poe1));
    let options = CalcOptions::default();
    let input = CalcInput {
        build: &b,
        data: &data,
        options: &options,
    };
    assert_eq!(calculate(&input).unwrap_err().code(), "CALC_DATA_VERSION");
}

// ------------------------------------------------------------------ games

#[test]
fn a_poe2_build_should_be_unsupported_not_calculated() {
    let b = build(Game::Poe2);
    let data = Data(version(Game::Poe2));
    let options = CalcOptions::default();
    let input = CalcInput {
        build: &b,
        data: &data,
        options: &options,
    };
    let r = calculate(&input).unwrap();
    assert_eq!(r.game, Game::Poe2);
    assert_eq!(r.state, StateKind::Unsupported);
    assert!(r.fields.is_empty());
    assert!(r.graph.is_empty());
    // Other rules for the wrong game: also unsupported.
    let r = calculate_with(&input, &Rules::new(vec![], vec![]), &|| false).unwrap();
    assert_eq!(r.state, StateKind::Unsupported);
}

#[test]
fn a_poe1_build_without_its_class_in_the_data_should_be_indeterminate() {
    let b = build(Game::Poe1);
    let data = Data(version(Game::Poe1));
    let options = CalcOptions::default();
    let input = CalcInput {
        build: &b,
        data: &data,
        options: &options,
    };
    let r = calculate(&input).unwrap();
    assert_eq!(r.fields.len(), loom_calc::poe1::SLICE.len());
    assert_eq!(r.fields.len(), 24, "docs/phase-3/slice-fields.json");
    let computed: Vec<_> = r
        .fields
        .iter()
        .filter(|(_, f)| f.value.is_some())
        .map(|(k, f)| (*k, f.state.kind()))
        .collect();
    assert_eq!(computed.len(), 17, "{computed:?}");
    assert!(computed.iter().all(|(_, k)| *k == StateKind::Indeterminate));
    assert!(
        r.reasons
            .indeterminate
            .contains(&IndeterminateReason::MissingData {
                table: "characters",
                key: String::new()
            })
    );
    let not_computed = r
        .fields
        .values()
        .filter(|f| f.state == FieldState::Unsupported(UnsupportedReason::NotComputed))
        .count();
    assert_eq!(not_computed, 7);
    assert_eq!(r.state, StateKind::Indeterminate);
    assert!(format!("{input:?}").contains("Poe1"));
}

// ---------------------------------------------------------------- results

#[test]
fn mapped_stats_should_give_complete_fields_and_unwritten_ones_not_computed() {
    let rules = Rules::new(
        vec![
            stat("base_life", 100),
            stat("inc_life", 50),
            stat("more_life", 10),
            stat("base_mana", 40),
        ],
        vec![],
    );
    let r = run(&build(Game::Poe1), &rules).unwrap();
    let life = &r.fields[&Field::Life];
    assert_eq!(life.value, Some(100.0 * 1.5 * 1.1));
    assert_eq!(life.state, FieldState::Complete);
    assert_eq!(r.fields[&Field::Mana].value, Some(40.0));
    assert_eq!(r.fields[&Field::HitDps].value, Some(205.0));
    assert_eq!(
        r.fields[&Field::Evasion].state,
        FieldState::Unsupported(UnsupportedReason::NotComputed)
    );
    assert_eq!(r.state, StateKind::Unsupported);
}

#[test]
fn an_unmapped_stat_should_make_fields_partial_never_a_silent_zero() {
    let rules = Rules::new(vec![stat("base_life", 100), stat("mystery", 7)], vec![]);
    let r = run(&build(Game::Poe1), &rules).unwrap();
    let expected = UnsupportedReason::UnmappedStat {
        stat: StatId::new("mystery"),
        source: Source::Character,
    };
    for field in [Field::Life, Field::Mana, Field::Armour] {
        let FieldState::Partial(reasons) = &r.fields[&field].state else {
            panic!("{field} is not partial: {:?}", r.fields[&field].state);
        };
        assert!(reasons.contains(&expected), "{field}");
    }
    assert_eq!(r.fields[&Field::Life].value, Some(100.0));
    assert_eq!(r.state, StateKind::Unsupported, "Evasion is not computed");
    assert!(
        r.reasons.partial.contains(&expected),
        "kept on the result as well"
    );
}

#[test]
fn reasons_should_show_on_the_result_when_no_field_is_computed() {
    struct Nothing;
    impl Ruleset for Nothing {
        fn game(&self) -> Game {
            Game::Poe1
        }
        fn slice(&self) -> &[Field] {
            &[Field::Life]
        }
        fn mapping(&self) -> &dyn StatMapping {
            &Mapping
        }
        fn collect(&self, _: &CalcInput<'_>, ingest: &mut Ingest<'_>) -> Result<(), CalcError> {
            ingest.stats(&Source::Character, &[stat("mystery", 1)]);
            Ok(())
        }
        fn phases(&self) -> &[Phase] {
            &[]
        }
    }
    let b = build(Game::Poe1);
    let data = Data(version(Game::Poe1));
    let options = CalcOptions::default();
    let input = CalcInput {
        build: &b,
        data: &data,
        options: &options,
    };
    let r = calculate_with(&input, &Nothing, &|| false).unwrap();
    assert_eq!(
        r.fields[&Field::Life].state,
        FieldState::Unsupported(UnsupportedReason::NotComputed)
    );
    assert_eq!(r.reasons.partial.len(), 1);
}

#[test]
fn a_derived_field_should_carry_its_dependencies_reasons() {
    let effects = vec![Effect {
        quantity: Quantity::Life,
        op: Op::Base,
        value: 5.0,
        conditions: Conditions::none().with_state(ConditionKey::new("c"), true),
        source: Source::Passive { node: NodeId(7) },
    }];
    let r = run(&build(Game::Poe1), &Rules::new(vec![], effects)).unwrap();
    assert_eq!(r.fields[&Field::Life].state.kind(), StateKind::Partial);
    assert_eq!(
        r.fields[&Field::HitDps].state,
        r.fields[&Field::Life].state,
        "HitDps reads Life"
    );
    let crit = &r.fields[&Field::CritChance];
    assert_eq!(crit.value, None);
    assert_eq!(
        crit.state,
        FieldState::Unsupported(UnsupportedReason::DependsOnNotComputed {
            field: Field::Evasion
        })
    );
}

#[test]
fn an_unsettled_condition_should_make_only_its_field_partial() {
    let effects = vec![Effect {
        quantity: Quantity::Mana,
        op: Op::Base,
        value: 5.0,
        conditions: Conditions::none().with_state(ConditionKey::new("c"), true),
        source: Source::Passive { node: NodeId(7) },
    }];
    let r = run(&build(Game::Poe1), &Rules::new(vec![], effects)).unwrap();
    assert_eq!(r.fields[&Field::Life].state, FieldState::Complete);
    assert_eq!(r.fields[&Field::Mana].state.kind(), StateKind::Partial);
    assert_eq!(r.fields[&Field::Mana].value, Some(0.0));
}

// ---------------------------------------------------------- cancellation

#[test]
fn cancellation_should_be_polled_between_steps() {
    let rules = Rules::new(vec![stat("base_life", 1)], vec![]);
    let b = build(Game::Poe1);
    let data = Data(version(Game::Poe1));
    let options = CalcOptions::default();
    let input = CalcInput {
        build: &b,
        data: &data,
        options: &options,
    };
    // Polls: before collecting, after collecting, after each of 2 phases.
    let total = 2 + PHASES.len();
    for stop_at in 0..total {
        let polls = Cell::new(0_usize);
        let cancel = || {
            let n = polls.get();
            polls.set(n + 1);
            n == stop_at
        };
        let err = calculate_with(&input, &rules, &cancel).unwrap_err();
        assert_eq!(err, CalcError::Cancelled);
        assert_eq!(err.code(), "CALC_CANCELLED");
        assert_eq!(polls.get(), stop_at + 1);
    }
    let polls = Cell::new(0_usize);
    let never = || {
        polls.set(polls.get() + 1);
        false
    };
    calculate_with(&input, &rules, &never).unwrap();
    assert_eq!(polls.get(), total);
    // The game entry point takes the same hook.
    let err = calculate_cancellable(&input, &|| true).unwrap_err();
    assert_eq!(err, CalcError::Cancelled);
}

// ------------------------------------------------------- property tests

/// `SplitMix64` (Steele, Lea, Flood 2014): a fixed-seed generator.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        let i = self.below(items.len() as u64) as usize;
        items[i]
    }

    /// A value with a fraction, sometimes large or negative, so summation
    /// order would show in the bits.
    fn value(&mut self) -> f64 {
        let whole = self.below(400) as i32 - 100;
        let frac = f64::from(self.below(1000) as u32) / 997.0;
        let scale = self.pick(&[1.0, 1.0, 1.0, 1e-3, 1e9]);
        if self.below(25) == 0 {
            return -0.0;
        }
        (f64::from(whole) + frac) * scale
    }

    fn effect(&mut self) -> Effect {
        let quantity = self.pick(&[
            Quantity::Life,
            Quantity::Mana,
            Quantity::Armour,
            Quantity::Resistance(DamageType::Fire),
            Quantity::Evasion,
        ]);
        let op = self.pick(&[
            Op::Base,
            Op::Base,
            Op::Increased,
            Op::Increased,
            Op::More,
            Op::Flag,
            Op::Override,
        ]);
        let mut conditions = Conditions::none();
        if self.below(4) == 0 {
            conditions =
                conditions.with_all(self.pick(&[Flags::SPELL, Flags::ATTACK, Flags::FIRE]));
        }
        if self.below(5) == 0 {
            conditions = conditions.with_any(Flags::ELEMENTAL);
        }
        if self.below(6) == 0 {
            let key = self.pick(&["a", "b", "c"]);
            conditions = conditions.with_state(ConditionKey::new(key), self.below(2) == 0);
        }
        let source = match self.below(4) {
            0 => Source::Character,
            1 => Source::Passive {
                node: NodeId(self.below(50) as u32),
            },
            2 => Source::Gem {
                group: 0,
                gem: self.below(6) as usize,
            },
            _ => Source::Rule(self.pick(&["r1", "r2"])),
        };
        Effect {
            quantity,
            op,
            value: self.value(),
            conditions,
            source,
        }
    }

    fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.below(i as u64 + 1) as usize;
            items.swap(i, j);
        }
    }
}

const CASES: u64 = 300;

fn random_rules(rng: &mut Rng) -> Rules {
    let n = rng.below(40);
    let effects = (0..n).map(|_| rng.effect()).collect();
    let stats = (0..rng.below(4))
        .map(|_| {
            let id = rng.pick(&["base_life", "inc_life", "more_life", "base_mana", "unknown"]);
            stat(id, rng.below(200) as i64)
        })
        .collect();
    Rules::new(stats, effects)
}

#[test]
fn every_output_should_equal_its_graphs_evaluation() {
    let mut rng = Rng(0x1001);
    let b = build(Game::Poe1);
    for case in 0..CASES {
        let rules = random_rules(&mut rng);
        let r = run(&b, &rules).unwrap();
        r.graph
            .check()
            .unwrap_or_else(|e| panic!("case {case}: {e}"));
        let evaluated = r.graph.evaluate_all().unwrap();
        for (field, f) in &r.fields {
            let (Some(value), Some(node)) = (f.value, f.node) else {
                continue;
            };
            assert_eq!(
                value.to_bits(),
                evaluated[node.get()].to_bits(),
                "case {case}: {field}"
            );
        }
    }
}

#[test]
fn two_runs_should_give_identical_results() {
    let mut rng = Rng(0x2002);
    let b = build(Game::Poe1);
    for case in 0..CASES {
        let rules = random_rules(&mut rng);
        let first = format!("{:?}", run(&b, &rules).unwrap());
        let second = format!("{:?}", run(&b, &rules).unwrap());
        assert_eq!(first, second, "case {case}");
    }
}

#[test]
fn effect_order_should_not_change_results() {
    let mut rng = Rng(0x3003);
    let b = build(Game::Poe1);
    for case in 0..CASES {
        let mut rules = random_rules(&mut rng);
        let before = format!("{:?}", run(&b, &rules).unwrap());
        rng.shuffle(&mut rules.effects);
        rng.shuffle(&mut rules.stats);
        let after = format!("{:?}", run(&b, &rules).unwrap());
        assert_eq!(before, after, "case {case}");
    }
}
