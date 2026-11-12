//! Calc result document v1: golden bytes, determinism, the DTO read-back,
//! names equal to the engine's, and every error code.

use std::collections::BTreeMap;

use loom_calc::poe1::Poe1Field;
use loom_calc::{
    Calc, CalcData, CalcError, CalcInput, CalcOptions, CalcResult, CharacterBase, ConditionKey,
    Conditions, DamageType, EffectTemplate, Field, FieldResult, FieldState, Graph,
    IndeterminateReason, Ingest, Node, Op, Phase, Quantity, QueryContext, Reasons, Ruleset, Source,
    StatAmount, StatMap, StatMapping, StateKind, Term, UnsupportedReason, calculate,
    calculate_with,
};
use loom_domain::{Adapted, Build, DataVersion, Game, ItemId, NodeId, StatId, StatLine, StatValue};

use super::{Document, DtoError, from_json, to_json};

// ---------------------------------------------------------------- results

fn leaf(
    g: &mut Graph,
    source: Source,
    quantity: Quantity,
    op: Op,
    value: f64,
) -> loom_calc::NodeIndex {
    g.push(
        Node::Contribution {
            source,
            quantity,
            op,
            value,
        },
        value,
    )
    .unwrap()
}

fn item(line: usize, adapted: Option<Adapted>) -> Source {
    Source::Item {
        item: ItemId(7),
        line,
        adapted,
    }
}

/// Every reason the engine has, of both kinds.
fn every_reason() -> Reasons {
    let mut r = Reasons::new();
    r.partial.extend([
        UnsupportedReason::UnmappedStat {
            stat: StatId::new("made_up_stat"),
            source: Source::Passive { node: NodeId(42) },
        },
        UnsupportedReason::UnresolvedLine {
            source: item(3, None),
            reason: "no-match",
        },
        UnsupportedReason::ExcludedLine {
            source: item(4, None),
            exclusion: "custom",
        },
        UnsupportedReason::PendingLine {
            source: item(5, None),
        },
        UnsupportedReason::UnhandledKind {
            kind: "made-up-kind",
            source: Source::Gem { group: 1, gem: 2 },
        },
        UnsupportedReason::UnmodelledCondition {
            condition: ConditionKey::new("on-full-life"),
            quantity: Quantity::Damage,
            source: Source::Config {
                input: "made_up_input".to_owned(),
            },
        },
        UnsupportedReason::DependsOnNotComputed {
            field: Field::Evasion,
        },
        UnsupportedReason::NotComputed,
        UnsupportedReason::UnmodelledGem {
            source: Source::Gem { group: 0, gem: 1 },
        },
        UnsupportedReason::UnmodelledConfig {
            input: "made_up_input".to_owned(),
        },
        UnsupportedReason::UnmodelledChoice {
            choice: "made-up-choice",
            value: "Someone".to_owned(),
        },
        UnsupportedReason::UnmodelledItem {
            item: ItemId(8),
            slot: "Helmet".to_owned(),
            what: "base-properties",
        },
        UnsupportedReason::UnmodelledPassive {
            source: Source::Passive {
                node: NodeId(0x1_0010),
            },
            what: "cluster-jewel-node",
        },
        UnsupportedReason::UnreadLine {
            source: item(9, None),
        },
        UnsupportedReason::UncheckedTreeVersion,
        UnsupportedReason::UnmodelledSkill {
            source: Source::Gem { group: 0, gem: 0 },
            what: "physical-hit-armour",
        },
    ]);
    r.indeterminate.extend([
        IndeterminateReason::ConflictingOverride {
            quantity: Quantity::MaximumResistance(DamageType::Fire),
        },
        IndeterminateReason::UnsettledValue {
            stat: StatId::new("ranged_stat"),
            source: item(6, Some(Adapted::SplitSection)),
        },
        IndeterminateReason::MissingData {
            table: "passives",
            key: "999".to_owned(),
        },
        IndeterminateReason::TreeVersion {
            build: "3_28".to_owned(),
            data: "3_29".to_owned(),
        },
    ]);
    r
}

fn computed(g: &Graph, node: loom_calc::NodeIndex, state: FieldState) -> FieldResult {
    FieldResult {
        value: g.value(node),
        node: Some(node),
        state,
    }
}

/// A hand-built PoE1 result: every node kind, term, source, state, and
/// reason; special numbers (`0.1 + 0.2`, `-0.0`, infinity).
fn every_kind_result() -> CalcResult {
    let mut g = Graph::new();
    let base = leaf(&mut g, Source::Character, Quantity::Life, Op::Base, 38.0);
    let item_base = leaf(
        &mut g,
        item(0, Some(Adapted::CaseFolded)),
        Quantity::Life,
        Op::Base,
        0.1 + 0.2,
    );
    let inc = leaf(
        &mut g,
        Source::Passive { node: NodeId(42) },
        Quantity::Life,
        Op::Increased,
        10.0,
    );
    let more = leaf(
        &mut g,
        Source::Gem { group: 0, gem: 1 },
        Quantity::Life,
        Op::More,
        -0.0,
    );
    let flag = leaf(
        &mut g,
        Source::Config {
            input: "made_up_input".to_owned(),
        },
        Quantity::Poe1(loom_calc::poe1::Poe1Quantity::SpellBlockChance),
        Op::Flag,
        1.0,
    );
    let enemy = leaf(
        &mut g,
        Source::Enemy,
        Quantity::Resistance(DamageType::Fire),
        Op::Override,
        75.0,
    );
    let rule = leaf(
        &mut g,
        Source::Rule("strength-to-life"),
        Quantity::Life,
        Op::Base,
        5.0,
    );
    leaf(
        &mut g,
        Source::ItemBase {
            item: loom_domain::ItemId(4),
        },
        Quantity::Armour,
        Op::Base,
        12.0,
    );
    let sum = g
        .derive(Node::Sum {
            term: Term::Base,
            terms: vec![base, item_base, rule],
        })
        .unwrap();
    let pct = g
        .derive(Node::PercentFactor {
            term: Term::Increased,
            of: inc,
        })
        .unwrap();
    let more_f = g
        .derive(Node::PercentFactor {
            term: Term::More,
            of: more,
        })
        .unwrap();
    let product = g
        .derive(Node::Product {
            term: Term::Result,
            factors: vec![sum, pct, more_f],
        })
        .unwrap();
    let life = g
        .derive(Node::RoundHalfUp {
            term: Term::Result,
            of: product,
        })
        .unwrap();
    let floor = g
        .derive(Node::Floor {
            term: Term::Step("floored"),
            of: product,
        })
        .unwrap();
    let any = g
        .derive(Node::Any {
            term: Term::Result,
            of: vec![flag],
        })
        .unwrap();
    let cap = g
        .derive(Node::Constant {
            label: "resistance-cap",
            value: 75.0,
        })
        .unwrap();
    let infinite = g
        .derive(Node::Constant {
            label: "unbounded",
            value: f64::INFINITY,
        })
        .unwrap();
    let min = g
        .derive(Node::Min {
            term: Term::Result,
            a: enemy,
            b: cap,
        })
        .unwrap();
    let max = g
        .derive(Node::Max {
            term: Term::Step("at-least-floor"),
            a: floor,
            b: infinite,
        })
        .unwrap();
    let over = g
        .derive(Node::Override {
            by: enemy,
            replaced: min,
        })
        .unwrap();
    g.derive(Node::Trunc {
        term: Term::Step("total"),
        of: product,
    })
    .unwrap();
    let share = g
        .derive(Node::Quotient {
            term: Term::Step("crit-share"),
            a: sum,
            b: cap,
        })
        .unwrap();
    g.derive(Node::Difference {
        term: Term::Step("non-crit-share"),
        a: pct,
        b: share,
    })
    .unwrap();

    let reasons = every_reason();
    let mut fields = BTreeMap::new();
    fields.insert(Field::Life, computed(&g, life, FieldState::Complete));
    fields.insert(
        Field::Mana,
        computed(&g, floor, FieldState::Partial(reasons.partial.clone())),
    );
    fields.insert(
        Field::Resistance(DamageType::Fire),
        computed(
            &g,
            over,
            FieldState::Indeterminate {
                reasons: reasons.indeterminate.clone(),
                partial: [UnsupportedReason::NotComputed].into(),
            },
        ),
    );
    fields.insert(Field::Evasion, computed(&g, max, FieldState::Complete));
    fields.insert(
        Field::Poe1(Poe1Field::SpellBlockChance),
        computed(&g, any, FieldState::Complete),
    );
    fields.insert(
        Field::EnergyShield,
        FieldResult::unsupported(UnsupportedReason::NotComputed),
    );
    fields.insert(
        Field::HitDps,
        FieldResult::unsupported(UnsupportedReason::DependsOnNotComputed {
            field: Field::EnergyShield,
        }),
    );
    let state = CalcResult::worst(&fields);
    CalcResult {
        game: Game::Poe1,
        fields,
        graph: g,
        state,
        reasons,
    }
}

struct Data(DataVersion);

/// One class (`Witch`, the game's base values) and the per-level
/// constants; no passive.
impl CalcData for Data {
    fn data_version(&self) -> DataVersion {
        self.0.clone()
    }
    fn item_base(&self, _: &str) -> Option<loom_calc::ItemBase> {
        None
    }
    fn character(&self, class: &str) -> Option<CharacterBase> {
        (class == "Witch").then_some(CharacterBase {
            base_life: 38,
            base_mana: 34,
            strength: 14,
            dexterity: 14,
            intelligence: 32,
        })
    }
    fn character_constant(&self, stat: &str) -> Option<i64> {
        match stat {
            "life_per_level" => Some(12),
            "mana_per_level" => Some(6),
            _ => None,
        }
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

fn build(game: Game, steam_build: u64) -> Build {
    let mut b = Build::new(game);
    b.data = Some(DataVersion {
        game,
        schema_version: 2,
        steam_build: Some(steam_build),
    });
    b
}

/// Maps `base_life` and `inc_life`; nothing else.
struct Mapping;

impl StatMapping for Mapping {
    fn map(&self, stat: &StatId) -> StatMap {
        let op = match stat.as_str() {
            "base_life" => Op::Base,
            "inc_life" => Op::Increased,
            _ => return StatMap::Unmapped,
        };
        StatMap::Effects(vec![EffectTemplate {
            quantity: Quantity::Life,
            op,
            scale: 1.0,
            divisor: 1.0,
            conditions: Conditions::none(),
        }])
    }
}

fn life(calc: &mut Calc<'_>) -> Result<(), CalcError> {
    let q = calc.chain(Quantity::Life, &QueryContext::default())?;
    calc.set(Field::Life, q);
    Ok(())
}

const PHASES: [Phase; 1] = [Phase {
    name: "life",
    run: life,
}];

/// A synthetic ruleset that computes life from the character's stats.
struct Rules;

impl Ruleset for Rules {
    fn game(&self) -> Game {
        Game::Poe1
    }

    fn slice(&self) -> &[Field] {
        &[Field::Life, Field::Mana]
    }

    fn mapping(&self) -> &dyn StatMapping {
        &Mapping
    }

    fn collect(&self, _input: &CalcInput<'_>, ingest: &mut Ingest<'_>) -> Result<(), CalcError> {
        let stat = |id: &str, v| StatLine {
            stat: StatId::new(id),
            value: StatValue::Exact { min: v, max: v },
        };
        ingest.stats(
            &Source::Character,
            &[
                stat("base_life", 50),
                stat("inc_life", 12),
                stat("unknown_stat", 3),
            ],
        );
        Ok(())
    }

    fn phases(&self) -> &[Phase] {
        &PHASES
    }
}

/// A result the engine computed (the store's chain, a mapped and an
/// unmapped stat).
fn engine_result() -> CalcResult {
    let b = build(Game::Poe1, 1);
    let data = Data(b.data.clone().unwrap());
    let options = CalcOptions::default();
    let input = CalcInput {
        build: &b,
        data: &data,
        options: &options,
    };
    calculate_with(&input, &Rules, &|| false).unwrap()
}

/// The shipped rules on an empty level 1 Witch (`game` of each).
fn shipped_result(game: Game) -> CalcResult {
    let mut b = build(game, 1);
    b.character.class = Some(loom_domain::ClassId::new("Witch"));
    let data = Data(b.data.clone().unwrap());
    let options = CalcOptions::default();
    calculate(&CalcInput {
        build: &b,
        data: &data,
        options: &options,
    })
    .unwrap()
}

fn goldens() -> [(&'static str, CalcResult); 4] {
    [
        ("every-kind.json", every_kind_result()),
        ("engine.json", engine_result()),
        ("poe1-empty.json", shipped_result(Game::Poe1)),
        ("poe2-empty.json", shipped_result(Game::Poe2)),
    ]
}

// ------------------------------------------------------------------ bytes

/// The bytes are fixed: the committed files are what every platform
/// writes (CI runs this in the `rust` job: Windows and Linux gate, macOS
/// is best-effort). Regenerate with the
/// ignored `write_golden_files` test after a deliberate change.
#[test]
fn results_should_give_the_golden_bytes() {
    let committed = [
        include_str!("testdata/every-kind.json"),
        include_str!("testdata/engine.json"),
        include_str!("testdata/poe1-empty.json"),
        include_str!("testdata/poe2-empty.json"),
    ];
    for ((name, result), golden) in goldens().iter().zip(committed) {
        assert_eq!(to_json(result).unwrap(), golden, "{name}");
    }
}

#[test]
#[ignore = "writes the golden files"]
#[expect(
    clippy::disallowed_methods,
    reason = "a test-only helper writes the crate's golden files"
)]
fn write_golden_files() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src/dto/calc/v1/testdata");
    std::fs::create_dir_all(dir).unwrap();
    for (name, result) in goldens() {
        std::fs::write(format!("{dir}/{name}"), to_json(&result).unwrap()).unwrap();
    }
}

#[test]
fn a_second_run_should_give_the_same_bytes() {
    for ((name, a), (_, b)) in goldens().iter().zip(goldens()) {
        assert_eq!(to_json(a).unwrap(), to_json(&b).unwrap(), "{name}");
    }
}

#[test]
fn a_document_should_read_back_to_the_same_bytes() {
    for (name, result) in goldens() {
        let json = to_json(&result).unwrap();
        let doc: Document = from_json(&json).unwrap();
        let mut again = serde_json::to_string_pretty(&doc).unwrap();
        again.push('\n');
        assert_eq!(again, json, "{name}");
    }
}

#[test]
fn special_numbers_should_keep_their_bits() {
    let json = to_json(&every_kind_result()).unwrap();
    assert!(json.contains("\"value\": 0.30000000000000004"), "{json}");
    assert!(json.contains("\"value\": -0.0"), "{json}");
    assert!(
        json.contains("\"f64_bits\": \"7ff0000000000000\""),
        "{json}"
    );
}

// ------------------------------------------------------------------ names

fn tag(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Object(m) => m.keys().next().unwrap().clone(),
        other => panic!("not a tagged value: {other}"),
    }
}

#[test]
fn reason_and_state_names_should_be_the_engines() {
    let r = every_reason();
    for reason in &r.partial {
        let v = serde_json::to_value(super::convert::unsupported_to(reason)).unwrap();
        assert_eq!(tag(&v), reason.name());
    }
    for reason in &r.indeterminate {
        let v = serde_json::to_value(super::convert::indeterminate_to(reason)).unwrap();
        assert_eq!(tag(&v), reason.name());
    }
    for kind in [
        StateKind::Complete,
        StateKind::Partial,
        StateKind::Unsupported,
        StateKind::Indeterminate,
    ] {
        let v = serde_json::to_value(super::convert::state_to(kind)).unwrap();
        assert_eq!(tag(&v), kind.name());
    }
}

#[test]
fn fields_and_quantities_should_be_named_by_display() {
    let doc = Document::from_result(&every_kind_result()).unwrap();
    let names: Vec<&str> = doc.result.fields.iter().map(|f| f.field.as_str()).collect();
    assert_eq!(
        names,
        [
            "life",
            "mana",
            "energy-shield",
            "evasion",
            "resistance.fire",
            "hit-dps",
            "poe1.spell-block-chance",
        ]
    );
    let json = to_json(&every_kind_result()).unwrap();
    for name in [
        "\"quantity\": \"poe1.spell-block-chance\"",
        "\"quantity\": \"maximum-resistance.fire\"",
        "\"op\": \"increased\"",
        "\"term\": \"step.floored\"",
        "\"rule\": \"strength-to-life\"",
        "\"adapted\": \"case-folded\"",
        "\"label\": \"resistance-cap\"",
    ] {
        assert!(json.contains(name), "{name} missing");
    }
}

#[test]
fn every_node_kind_should_have_its_children_and_term() {
    let doc = Document::from_result(&every_kind_result()).unwrap();
    let graph = &doc.result.graph;
    let kinds: Vec<String> = graph
        .iter()
        .map(|n| tag(&serde_json::to_value(&n.kind).unwrap()))
        .collect();
    for kind in [
        "contribution",
        "constant",
        "sum",
        "product",
        "percent-factor",
        "any",
        "override",
        "floor",
        "round-half-up",
        "trunc",
        "min",
        "max",
        "difference",
        "quotient",
    ] {
        assert!(kinds.iter().any(|k| k == kind), "{kind} missing");
    }
    for (i, n) in graph.iter().enumerate() {
        assert!(n.children.iter().all(|c| (*c as usize) < i), "node {i}");
        let leaf = matches!(
            n.kind,
            super::NodeKind::Contribution { .. } | super::NodeKind::Constant { .. }
        );
        assert_eq!(leaf, n.children.is_empty(), "node {i}");
        let termless = leaf || matches!(n.kind, super::NodeKind::Override);
        assert_eq!(termless, n.term.is_none(), "node {i}");
    }
    let over = graph
        .iter()
        .find(|n| matches!(n.kind, super::NodeKind::Override))
        .unwrap();
    assert_eq!(over.children.len(), 2);
}

#[test]
fn field_reasons_should_follow_the_state() {
    let doc = Document::from_result(&every_kind_result()).unwrap();
    let by = |name: &str| doc.result.fields.iter().find(|f| f.field == name).unwrap();
    let life = by("life");
    assert!(life.unsupported.is_empty() && life.indeterminate.is_empty());
    assert!(life.value.is_some() && life.node.is_some());
    assert_eq!(by("mana").unsupported.len(), 16);
    let es = by("energy-shield");
    assert!(es.unsupported.is_empty());
    assert!(matches!(
        es.reason,
        Some(super::UnsupportedReason::NotComputed)
    ));
    assert!(es.value.is_none() && es.node.is_none());
    assert!(by("mana").reason.is_none());
    let fire = by("resistance.fire");
    assert_eq!((fire.unsupported.len(), fire.indeterminate.len()), (1, 4));
    assert_eq!(doc.result.reasons.unsupported.len(), 16);
    assert_eq!(doc.result.reasons.indeterminate.len(), 4);
}

#[test]
fn the_shipped_rules_should_write_every_slice_field() {
    let doc = Document::from_result(&shipped_result(Game::Poe1)).unwrap();
    assert_eq!(doc.result.fields.len(), loom_calc::poe1::SLICE.len());
    let poe2 = to_json(&shipped_result(Game::Poe2)).unwrap();
    assert!(poe2.contains("\"state\": \"unsupported\""), "{poe2}");
    assert!(poe2.contains("\"fields\": []"), "{poe2}");
}

// ----------------------------------------------------------------- errors

#[test]
fn a_graph_that_does_not_hold_should_not_be_written() {
    let mut r = every_kind_result();
    let a = leaf(
        &mut r.graph,
        Source::Character,
        Quantity::Mana,
        Op::Base,
        1.0,
    );
    r.graph
        .push(
            Node::Sum {
                term: Term::Base,
                terms: vec![a],
            },
            2.0,
        )
        .unwrap();
    assert_eq!(to_json(&r).unwrap_err().code(), "DTO_CALC_GRAPH");
}

#[test]
fn a_field_whose_value_is_not_its_nodes_should_not_be_written() {
    let base = every_kind_result();
    let life = base.fields[&Field::Life].clone();
    // A node index past the end of the result's graph.
    let mut longer = base.graph.clone();
    let past = leaf(
        &mut longer,
        Source::Character,
        Quantity::Life,
        Op::Base,
        1.0,
    );
    for broken in [
        FieldResult {
            value: Some(1.5),
            ..life.clone()
        },
        FieldResult {
            node: None,
            ..life.clone()
        },
        FieldResult {
            value: None,
            ..life.clone()
        },
        FieldResult {
            node: Some(past),
            value: Some(1.0),
            ..life
        },
    ] {
        let mut r = base.clone();
        r.fields.insert(Field::Life, broken);
        let err = to_json(&r).unwrap_err();
        assert_eq!(err.code(), "DTO_CALC_FIELD");
        assert!(matches!(err, DtoError::Field(ref f) if f == "life"));
    }
}

#[test]
fn another_format_version_or_shape_should_not_read() {
    let json = to_json(&every_kind_result()).unwrap();
    for (from, to, code) in [
        ("\"loom-calc-result\"", "\"loom-build\"", "DTO_FORMAT"),
        ("\"version\": 1,", "\"version\": 2,", "DTO_VERSION"),
        (
            "\"game\": \"poe1\",",
            "\"game\": \"poe1\", \"x\": 1,",
            "DTO_JSON",
        ),
        ("\"op\": \"base\"", "\"op\": 1", "DTO_JSON"),
        (
            "\"state\": \"complete\"",
            "\"state\": \"made-up\"",
            "DTO_JSON",
        ),
    ] {
        let changed = json.replacen(from, to, 1);
        assert_ne!(changed, json, "{from} not found");
        assert_eq!(from_json(&changed).unwrap_err().code(), code, "{from}");
    }
    assert_eq!(from_json("not json").unwrap_err().code(), "DTO_JSON");
}

/// A change to a parsed document.
type Change = Box<dyn Fn(&mut serde_json::Value)>;

#[test]
fn a_document_with_a_broken_structure_should_not_read() {
    let json = to_json(&every_kind_result()).unwrap();
    let doc: serde_json::Value = serde_json::from_str(&json).unwrap();
    let node_of = |kind: &str| {
        doc["result"]["graph"]
            .as_array()
            .unwrap()
            .iter()
            .position(|n| n["kind"] == kind)
            .unwrap()
    };
    let (sum, over, floor) = (node_of("sum"), node_of("override"), node_of("floor"));
    let field_of = |name: &str| {
        doc["result"]["fields"]
            .as_array()
            .unwrap()
            .iter()
            .position(|f| f["field"] == name)
            .unwrap()
    };
    let (life, es, mana) = (
        field_of("life"),
        field_of("energy-shield"),
        field_of("mana"),
    );
    let breaks: Vec<(&str, Change)> = vec![
        (
            "a later child",
            Box::new(move |d| d["result"]["graph"][sum]["children"][0] = 9999.into()),
        ),
        (
            "a leaf with children",
            Box::new(|d| d["result"]["graph"][0]["children"] = serde_json::json!([0])),
        ),
        (
            "override with one child",
            Box::new(move |d| d["result"]["graph"][over]["children"] = serde_json::json!([0])),
        ),
        (
            "floor without a term",
            Box::new(move |d| {
                d["result"]["graph"][floor]
                    .as_object_mut()
                    .unwrap()
                    .remove("term");
            }),
        ),
        (
            "a leaf with a term",
            Box::new(|d| d["result"]["graph"][0]["term"] = "base".into()),
        ),
        (
            "a node past the end",
            Box::new(move |d| d["result"]["fields"][life]["node"] = 9999.into()),
        ),
        (
            "a value not the node's",
            Box::new(move |d| d["result"]["fields"][life]["value"] = 1.5.into()),
        ),
        (
            "a value without a node",
            Box::new(move |d| {
                d["result"]["fields"][life]
                    .as_object_mut()
                    .unwrap()
                    .remove("node");
            }),
        ),
        (
            "unsupported without its reason",
            Box::new(move |d| {
                d["result"]["fields"][es]
                    .as_object_mut()
                    .unwrap()
                    .remove("reason");
            }),
        ),
        (
            "complete with a reason",
            Box::new(move |d| d["result"]["fields"][life]["reason"] = "not-computed".into()),
        ),
        (
            "partial without reasons",
            Box::new(move |d| {
                d["result"]["fields"][mana]
                    .as_object_mut()
                    .unwrap()
                    .remove("unsupported");
            }),
        ),
    ];
    for (what, change) in breaks {
        let mut d = doc.clone();
        change(&mut d);
        let err = from_json(&serde_json::to_string(&d).unwrap()).unwrap_err();
        assert_eq!(err.code(), "DTO_CALC_SHAPE", "{what}: {err}");
    }
}
