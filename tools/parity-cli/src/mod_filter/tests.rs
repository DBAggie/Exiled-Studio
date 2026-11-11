use loom_data::stat_desc::{Condition, DescRef, Hand, StatValue};

use std::sync::LazyLock;

use loom_parser::import::BaseRow;
use loom_parser::import::mods::{self, Decision, ModRules, numbered_stat, unnumbered};
use loom_parser::resolve::{Candidate, RawValue, ResolvedStat};
use loom_serialization::legacy::pob_item::domain_section;

use super::*;
use crate::poe1::domain;

/// The PoE1 rules the harness runs with.
static RULES: LazyLock<ModRules> = LazyLock::new(|| poe1::item_rules().mods);

const ITEM: i64 = domain::ITEM;
const PRE: i64 = generation::PREFIX;

fn exact(stat: &str, v: i64) -> ResolvedStat {
    ResolvedStat {
        stat: stat.to_owned(),
        value: RawValue::Exact(StatValue::exact(v)),
    }
}

fn hidden(stat: &str, condition: Condition) -> ResolvedStat {
    ResolvedStat {
        stat: stat.to_owned(),
        value: RawValue::Hidden {
            condition,
            assumed: None,
        },
    }
}

fn cand(stats: Vec<ResolvedStat>) -> Candidate {
    Candidate {
        at: DescRef {
            depth: 0,
            index: 0,
            hand: Hand::Main,
        },
        variant: 0,
        stats,
        alternatives: Vec::new(),
    }
}

fn row(domain: i64, generation: i64, stats: &[(&str, i64, i64)], spawn: &[(&str, i64)]) -> ModRow {
    ModRow {
        id: format!("Row{}", stats.first().map_or("", |s| s.0)),
        domain,
        generation,
        stats: stats
            .iter()
            .map(|(s, a, b)| ((*s).to_owned(), *a, *b))
            .collect(),
        spawn: spawn.iter().map(|(t, w)| ((*t).to_owned(), *w)).collect(),
        ..ModRow::default()
    }
}

fn base(tags: Option<&[&str]>) -> BaseRow {
    BaseRow {
        domain: ITEM,
        class: Some(7),
        tags: tags.map(|t| t.iter().map(|s| (*s).to_owned()).collect()),
        implicits: Vec::new(),
    }
}

/// A line as Spike B's filter took it; mapped onto the core context.
#[derive(Debug, Clone, Copy)]
struct LineContext {
    section: Section,
    crafted: bool,
    exarch: bool,
    eater: bool,
}

impl LineContext {
    fn core(self) -> mods::LineContext {
        let flags = [
            (self.crafted, "crafted"),
            (self.exarch, "exarch"),
            (self.eater, "eater"),
        ];
        mods::LineContext {
            section: domain_section(self.section),
            flags: flags
                .iter()
                .filter(|(on, _)| *on)
                .map(|(_, f)| (*f).to_owned())
                .collect(),
        }
    }
}

fn fits(row: &ModRow, base: &BaseRow, unique: bool, line: LineContext) -> bool {
    mods::fits(&RULES, row, base, unique, &line.core())
}

fn filter(ctx: &Context<'_>, line: LineContext, candidates: &[Candidate]) -> Decision {
    mods::filter(ctx, &line.core(), candidates)
}

fn backing<'m>(
    ctx: &Context<'m>,
    line: LineContext,
    stats: &[ResolvedStat],
) -> Result<(bool, &'m str), &'static str> {
    mods::backing(ctx, &line.core(), stats)
}

fn line(section: Section) -> LineContext {
    LineContext {
        section,
        crafted: false,
        exarch: false,
        eater: false,
    }
}

const EXPLICIT: LineContext = LineContext {
    section: Section::Explicit,
    crafted: false,
    exarch: false,
    eater: false,
};

fn ctx<'a>(mods: &'a Mods, bases: &'a [BaseRow]) -> Context<'a> {
    Context {
        rules: &RULES,
        mods,
        bases,
        unique: false,
        influences: Vec::new(),
    }
}

/// `local_es` spawns on `helmet`, `global_es` on `ring`; both `ITEM`
/// prefixes, the usual local/global ambiguity.
fn es_mods() -> Mods {
    Mods::from_rows(vec![
        row(
            ITEM,
            PRE,
            &[("local_es", 10, 20)],
            &[("helmet", 1000), ("default", 0)],
        ),
        row(
            ITEM,
            PRE,
            &[("global_es", 30, 40)],
            &[("ring", 1000), ("default", 0)],
        ),
    ])
}

fn es_candidates(v: i64) -> Vec<Candidate> {
    vec![
        cand(vec![exact("local_es", v)]),
        cand(vec![exact("global_es", v)]),
    ]
}

#[test]
fn backs_should_tell_equal_from_hybrid_rows() {
    let single = row(ITEM, PRE, &[("a", 1, 5)], &[]);
    let hybrid = row(ITEM, PRE, &[("a", 1, 5), ("b", 1, 5)], &[]);
    let c = [exact("a", 3)];
    assert_eq!(
        (single.backs(&c), hybrid.backs(&c)),
        (Some(true), Some(false))
    );
}

#[test]
fn backs_should_allow_a_missing_stat_only_when_its_value_admits_zero() {
    let r = row(ITEM, PRE, &[("min", 1, 5)], &[]);
    let zero = [
        exact("min", 2),
        hidden(
            "max",
            Condition::Range {
                min: None,
                max: None,
            },
        ),
    ];
    let non_zero = [exact("min", 2), exact("max", 4)];
    assert_eq!((r.backs(&zero), r.backs(&non_zero)), (Some(true), None));
}

#[test]
fn backs_should_need_at_least_one_candidate_stat_on_the_row() {
    let r = row(ITEM, PRE, &[("other", 1, 5)], &[]);
    assert_eq!(r.backs(&[hidden("a", Condition::Not(1))]), None);
}

#[test]
fn values_overlap_should_test_each_kind_of_value() {
    let r = row(ITEM, PRE, &[("a", 10, 20)], &[]);
    let ranged = ResolvedStat {
        stat: "a".to_owned(),
        value: RawValue::Ranged(StatValue { min: 18, max: 25 }),
    };
    let lookup = ResolvedStat {
        stat: "a".to_owned(),
        value: RawValue::Lookup(vec![3, 30]),
    };
    let not_15 = hidden("a", Condition::Not(15));
    assert!(r.values_overlap(&[ranged]));
    assert!(!r.values_overlap(&[lookup]));
    assert!(r.values_overlap(&[not_15]));
    assert!(!r.values_overlap(&[exact("a", 21)]));
    let one = row(ITEM, PRE, &[("a", 15, 15)], &[]);
    assert!(!one.values_overlap(&[hidden("a", Condition::Not(15))]));
}

#[test]
fn fits_should_need_a_crafted_row_of_the_base_class_for_crafted_lines() {
    let mut r = row(domain::CRAFTED, PRE, &[("a", 1, 1)], &[]);
    let crafted = LineContext {
        crafted: true,
        ..EXPLICIT
    };
    assert!(fits(&r, &base(None), false, crafted));
    r.craft_classes = vec![8];
    assert!(!fits(&r, &base(None), false, crafted));
    r.craft_classes = vec![7, 8];
    assert!(fits(&r, &base(None), false, crafted));
    assert!(!fits(&r, &base(None), false, EXPLICIT));
}

#[test]
fn fits_should_take_unique_rows_for_unique_explicits_only() {
    let unique_row = row(ITEM, generation::UNIQUE, &[("a", 1, 1)], &[]);
    let prefix = row(ITEM, PRE, &[("a", 1, 1)], &[]);
    let b = base(None);
    assert!(fits(&unique_row, &b, true, EXPLICIT) && !fits(&prefix, &b, true, EXPLICIT));
    assert!(!fits(&unique_row, &b, false, EXPLICIT) && fits(&prefix, &b, false, EXPLICIT));
}

#[test]
fn fits_should_take_delve_rows_on_jewels_and_unveiled_rows_on_items() {
    let delve = row(domain::DELVE, PRE, &[("a", 1, 1)], &[]);
    let unveiled = row(domain::UNVEILED, PRE, &[("a", 1, 1)], &[]);
    let jewel = BaseRow {
        domain: domain::BASE_JEWEL,
        ..base(None)
    };
    assert!(fits(&delve, &jewel, false, EXPLICIT));
    assert!(!fits(&unveiled, &jewel, false, EXPLICIT));
    assert!(fits(&unveiled, &base(None), false, EXPLICIT));
}

#[test]
fn fits_should_follow_the_section() {
    let b = BaseRow {
        domain: 2,
        ..base(None)
    };
    let flask_enchant = row(ITEM, 21, &[("a", 1, 1)], &[]);
    let implicit = row(2, generation::UNIQUE, &[("a", 1, 1)], &[]);
    let exarch = row(2, generation::EXARCH, &[("a", 1, 1)], &[]);
    assert!(fits(&flask_enchant, &b, false, line(Section::Enchant)));
    assert!(!fits(&flask_enchant, &b, false, line(Section::Implicit)));
    assert!(fits(&implicit, &b, false, line(Section::Implicit)));
    // Harvest, Heist, cluster, and anoint enchants are `UNIQUE` rows.
    assert!(fits(&implicit, &b, false, line(Section::Enchant)));
    // An implicit without the eldritch flag still takes eldritch rows.
    assert!(fits(&exarch, &b, false, line(Section::Implicit)));
    let eldritch = LineContext {
        exarch: true,
        ..line(Section::Implicit)
    };
    assert!(fits(&exarch, &b, false, eldritch) && !fits(&implicit, &b, false, eldritch));
    assert!(!fits(&implicit, &b, false, line(Section::ClassRequirement)));
}

#[test]
fn filter_should_resolve_by_mods_when_one_candidate_has_no_fitting_row() {
    let mods = Mods::from_rows(vec![row(ITEM, PRE, &[("real", 1, 9)], &[])]);
    let bases = [base(None)];
    let cs = [
        cand(vec![exact("virtual", 5)]),
        cand(vec![exact("real", 5)]),
    ];
    let d = filter(&ctx(&mods, &bases), EXPLICIT, &cs);
    assert_eq!(
        (d.kept.clone(), d.name()),
        (vec![1], "resolved/mods".to_owned())
    );
}

#[test]
fn filter_should_keep_every_candidate_when_none_is_backed() {
    let mods = Mods::from_rows(vec![row(ITEM, generation::SUFFIX, &[("a", 1, 9)], &[])]);
    let bases = [base(None)];
    let cs = [cand(vec![exact("a", 5)]), cand(vec![exact("b", 5)])];
    let d = filter(&ctx(&mods, &bases), line(Section::Enchant), &cs);
    assert_eq!(
        (d.kept.clone(), d.name()),
        (vec![0, 1], "ambiguous/no-backed-candidate".to_owned())
    );
}

#[test]
fn filter_should_break_a_tie_by_the_base_spawn_tags() {
    let mods = es_mods();
    let bases = [base(Some(&["helmet", "default"]))];
    let d = filter(&ctx(&mods, &bases), EXPLICIT, &es_candidates(50));
    assert_eq!(
        (d.kept.clone(), d.name()),
        (vec![0], "resolved/spawn-tags".to_owned())
    );
}

#[test]
fn filter_should_skip_spawn_tags_when_the_base_tags_are_unknown() {
    let mods = es_mods();
    let bases = [base(None)];
    let d = filter(&ctx(&mods, &bases), EXPLICIT, &es_candidates(50));
    assert_eq!(d.name(), "ambiguous/several-backed");
    assert!(d.notes.contains("base-tags-unknown"));
}

#[test]
fn filter_should_skip_spawn_tags_for_crafted_lines() {
    let mods = Mods::from_rows(vec![
        row(
            domain::CRAFTED,
            PRE,
            &[("local_es", 10, 20)],
            &[("default", 0)],
        ),
        row(
            domain::CRAFTED,
            PRE,
            &[("global_es", 10, 20)],
            &[("default", 0)],
        ),
    ]);
    let bases = [base(Some(&["helmet"]))];
    let crafted = LineContext {
        crafted: true,
        ..EXPLICIT
    };
    let d = filter(&ctx(&mods, &bases), crafted, &es_candidates(15));
    assert_eq!(d.kept, vec![0, 1]);
    assert!(d.notes.contains("crafted-no-spawn-tags"));
}

#[test]
fn filter_should_skip_spawn_tags_when_an_influence_row_backs_a_candidate() {
    let mut influenced = row(ITEM, PRE, &[("global_es", 10, 20)], &[("ring_shaper", 500)]);
    influenced.influence = Some(0);
    let mut rows = vec![row(ITEM, PRE, &[("local_es", 10, 20)], &[("helmet", 1000)])];
    rows.push(influenced);
    let mods = Mods::from_rows(rows);
    let bases = [base(Some(&["helmet"]))];
    let mut c = ctx(&mods, &bases);
    c.influences = vec![0];
    let d = filter(&c, EXPLICIT, &es_candidates(15));
    assert_eq!(d.kept, vec![0, 1]);
    assert!(d.notes.contains("influence-tags-unknown"));
    // Without the influence on the item, spawn tags decide.
    c.influences.clear();
    assert_eq!(filter(&c, EXPLICIT, &es_candidates(15)).kept, vec![0]);
}

#[test]
fn filter_should_prefer_the_base_implicit_for_implicit_lines() {
    let mods = Mods::from_rows(vec![
        row(ITEM, generation::UNIQUE, &[("local_es", 10, 20)], &[]),
        row(ITEM, generation::UNIQUE, &[("global_es", 10, 20)], &[]),
    ]);
    let bases = [BaseRow {
        implicits: vec![1],
        ..base(None)
    }];
    let d = filter(
        &ctx(&mods, &bases),
        line(Section::Implicit),
        &es_candidates(15),
    );
    assert_eq!(
        (d.kept.clone(), d.name()),
        (vec![1], "resolved/base-implicit".to_owned())
    );
}

#[test]
fn filter_should_break_a_last_tie_by_values() {
    let mods = Mods::from_rows(vec![
        row(ITEM, PRE, &[("local_es", 10, 20)], &[("default", 1)]),
        row(ITEM, PRE, &[("global_es", 30, 40)], &[("default", 1)]),
    ]);
    let bases = [base(Some(&["default"]))];
    let d = filter(&ctx(&mods, &bases), EXPLICIT, &es_candidates(35));
    assert_eq!(
        (d.kept.clone(), d.name()),
        (vec![1], "resolved/values".to_owned())
    );
}

#[test]
fn filter_should_skip_spawn_tags_when_an_essence_row_with_unknown_classes_backs_a_candidate() {
    let mut essence = row(ITEM, PRE, &[("global_es", 10, 20)], &[("default", 0)]);
    essence.essence_only = true;
    let mods = Mods::from_rows(vec![
        row(ITEM, PRE, &[("local_es", 10, 20)], &[("helmet", 1)]),
        essence,
    ]);
    let bases = [base(Some(&["helmet"]))];
    let d = filter(&ctx(&mods, &bases), EXPLICIT, &es_candidates(15));
    assert_eq!(d.kept, vec![0, 1]);
    assert!(d.notes.contains("essence-tags-unknown"));
}

#[test]
fn filter_should_not_count_eldritch_rows_as_spawning_on_an_unflagged_implicit() {
    let mods = Mods::from_rows(vec![
        row(
            ITEM,
            generation::EXARCH,
            &[("local_es", 10, 20)],
            &[("helmet", 1)],
        ),
        row(ITEM, generation::CORRUPTED, &[("global_es", 10, 20)], &[]),
    ]);
    let bases = [base(Some(&["helmet"]))];
    let d = filter(
        &ctx(&mods, &bases),
        line(Section::Implicit),
        &es_candidates(15),
    );
    assert_eq!(d.name(), "ambiguous/several-backed");
}

#[test]
fn weight_should_come_from_the_first_row_tag_the_base_has() {
    let r = row(ITEM, PRE, &[("a", 1, 1)], &[("b", 0), ("a", 100)]);
    let tags = ["a".to_owned(), "b".to_owned()];
    assert_eq!(r.weight(&tags), 0);
    assert_eq!(r.weight(&tags[..1]), 100);
}

#[test]
fn backing_should_return_a_hybrid_row_when_no_equal_row_backs() {
    let mods = Mods::from_rows(vec![row(ITEM, PRE, &[("a", 1, 9), ("b", 1, 9)], &[])]);
    let bases = [base(None)];
    assert_eq!(
        backing(&ctx(&mods, &bases), EXPLICIT, &[exact("a", 5)]),
        Ok((false, "Rowa"))
    );
}

#[test]
fn influence_types_should_match_the_vendored_schema_enum() {
    let schema: Value =
        serde_json::from_str(include_str!("../../../extractor/schema/schema.min.json")).unwrap();
    let e = schema["enumerations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["name"] == "InfluenceTypes")
        .unwrap();
    assert_eq!(e["indexing"], 0);
    let names = e["enumerators"].as_array().unwrap();
    let at = |n: i64| names[usize::try_from(n).unwrap()].as_str().unwrap();
    assert_eq!(at(poe1::NO_INFLUENCE), "NONE");
    let expected = [
        ("Shaper", "SHAPER"),
        ("Elder", "ELDER"),
        ("Crusader", "CRUSADER"),
        ("Redeemer", "EYRIE"),
        ("Hunter", "BASILISK"),
        ("Warlord", "ADJUDICATOR"),
    ];
    for (display, name) in expected {
        assert_eq!(
            at(influence_type(&format!("{display} Item")).unwrap()),
            name
        );
    }
}

#[test]
fn load_should_pair_spawn_tags_and_weights_by_position() {
    let row = serde_json::json!([[5, 1], [7, 9]]);
    let cells = row.as_array().unwrap();
    let ids = ["t0".to_owned(), "t1".to_owned()];
    // Tag index 5 is unknown: its pair goes, `t1` keeps weight 9.
    assert_eq!(
        spawn_pairs(cells, 0, 1, &ids),
        Some(vec![("t1".to_owned(), 9)])
    );
    let uneven = serde_json::json!([[1], [7, 9]]);
    assert_eq!(spawn_pairs(uneven.as_array().unwrap(), 0, 1, &ids), None);
}

#[test]
fn backing_should_name_why_a_resolved_line_is_not_backed() {
    let mods = Mods::from_rows(vec![
        row(ITEM, PRE, &[("a", 10, 20), ("b", 1, 1)], &[]),
        row(domain::CRAFTED, PRE, &[("c", 1, 1)], &[]),
    ]);
    let bases = [base(None)];
    let c = ctx(&mods, &bases);
    assert_eq!(
        backing(&c, EXPLICIT, &[exact("a", 15)]),
        Ok((false, "Rowa"))
    );
    assert_eq!(
        backing(&c, EXPLICIT, &[exact("a", 25)]),
        Err("values-outside")
    );
    assert_eq!(
        backing(&c, EXPLICIT, &[exact("c", 1)]),
        Err("no-fitting-row")
    );
    assert_eq!(
        backing(&c, EXPLICIT, &[exact("d", 1)]),
        Err("no-row-with-stats")
    );
}

#[test]
fn influence_type_should_follow_pob_influence_names() {
    assert_eq!(influence_type("Redeemer Item"), Some(3));
    assert_eq!(influence_type("Warlord Item"), Some(5));
    assert_eq!(influence_type("Searing Exarch Item"), None);
}

fn mods_table() -> String {
    let mut columns = vec![
        "Id",
        "Domain",
        "GenerationType",
        "SpawnWeight_TagsKeys",
        "SpawnWeight_Values",
        "InfluenceTypes",
        "CraftingItemClassRestrictions",
        "IsEssenceOnlyModifier",
        "ModTypeKey",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect::<Vec<_>>();
    for n in 1..=STAT_SLOTS {
        columns.extend([
            format!("StatsKey{n}"),
            format!("Stat{n}Min"),
            format!("Stat{n}Max"),
        ]);
    }
    let mut row = vec![
        r#""M""#.to_owned(),
        "1".to_owned(),
        "2".to_owned(),
        "[1, 0]".to_owned(),
        "[100, 0]".to_owned(),
        "6".to_owned(),
        "[]".to_owned(),
        "true".to_owned(),
        "0".to_owned(),
    ];
    // Slot 1: stat 0 from 5 to 3 (min and max swapped); slot 2: stat 1 at
    // (0, 0); slot 8: stat 2; the rest empty.
    for n in 1..=STAT_SLOTS {
        row.extend(
            match n {
                1 => ["0", "5", "3"],
                2 => ["1", "0", "0"],
                8 => ["2", "7", "9"],
                _ => ["null", "0", "0"],
            }
            .map(str::to_owned),
        );
    }
    let columns: Vec<String> = columns
        .iter()
        .map(|c| format!(r#"{{"name": "{c}", "references": null}}"#))
        .collect();
    format!(
        r#"{{"table": "Mods", "columns": [{}], "rows": [[{}]]}}"#,
        columns.join(", "),
        row.join(", ")
    )
}

/// `Stats` with `s0` (`IsLocal`), `s1` (`IsWeaponLocal`), and `s2`.
fn stats_table() -> String {
    r#"{"table": "Stats", "columns": [{"name": "Id", "references": null}, {"name": "IsLocal", "references": null}, {"name": "IsWeaponLocal", "references": null}], "rows": [["s0", true, false], ["s1", true, true], ["s2", false, false]]}"#.to_owned()
}

fn id_table(name: &str, ids: &[&str]) -> String {
    let rows: Vec<String> = ids.iter().map(|i| format!(r#"["{i}"]"#)).collect();
    format!(
        r#"{{"table": "{name}", "columns": [{{"name": "Id", "references": null}}], "rows": [{}]}}"#,
        rows.join(", ")
    )
}

#[test]
fn load_should_read_eight_stat_slots_and_drop_zero_stats() {
    let read = |p: &str| -> Result<Vec<u8>, String> {
        Ok(match p {
            "tables/Mods.json" => mods_table(),
            "tables/Stats.json" => stats_table(),
            "tables/Tags.json" => id_table("Tags", &["default", "ring"]),
            _ => return Err(format!("{p}: not found")),
        }
        .into_bytes())
    };
    let mods = load(&read).unwrap();
    let r = mods.row(0).unwrap();
    assert_eq!(r.stats, [("s0".to_owned(), 3, 5), ("s2".to_owned(), 7, 9)]);
    assert_eq!(r.unset, ["s1".to_owned()]);
    assert_eq!(
        r.spawn,
        [("ring".to_owned(), 100), ("default".to_owned(), 0)]
    );
    assert_eq!(
        (r.influence, r.essence_only, r.domain, r.generation),
        (None, true, 1, 2)
    );
    assert_eq!(mods.rows_with_stat("s2").count(), 1);
    assert_eq!(r.mod_type, Some(0));
    assert_eq!(
        mods.locality
            .iter()
            .map(|(k, v)| (k.as_str(), *v))
            .collect::<Vec<_>>(),
        [("s0", Locality::Item), ("s1", Locality::Weapon)]
    );
}

#[test]
fn backing_should_name_a_fitting_row_whose_value_is_set_outside_the_row() {
    let mut granted = row(ITEM, generation::UNIQUE, &[], &[]);
    granted.unset = vec!["passive".to_owned()];
    let mods = Mods::from_rows(vec![granted]);
    let bases = [base(None)];
    let c = ctx(&mods, &bases);
    let keys = ResolvedStat {
        stat: "passive".to_owned(),
        value: RawValue::Lookup(vec![1234]),
    };
    let enchant = line(Section::Enchant);
    assert_eq!(
        backing(&c, enchant, std::slice::from_ref(&keys)),
        Err("row-value-unset")
    );
    // No candidate is backed; the one with a fitting unset row is kept.
    let d = filter(
        &c,
        enchant,
        &[cand(vec![keys]), cand(vec![exact("other", 1)])],
    );
    assert_eq!(d.name(), "resolved/unset-fit");
    assert!(d.notes.contains("row-value-unset"));
}

fn lookup(stat: &str) -> ResolvedStat {
    ResolvedStat {
        stat: stat.to_owned(),
        value: RawValue::Lookup(vec![1234]),
    }
}

/// The anoint shape: five `GrantedPassive*`-like `UNIQUE` rows carry
/// `hash`, `hash_2` .. `hash_5` at `(0, 0)`; a weapon-tree row carries
/// `tree_hash`, which does not fit the enchant section.
fn anoint_mods() -> Mods {
    let mut rows: Vec<ModRow> = ["hash", "hash_2", "hash_3", "hash_4", "hash_5"]
        .iter()
        .map(|s| {
            let mut r = row(ITEM, generation::UNIQUE, &[], &[]);
            r.unset = vec![(*s).to_owned()];
            r
        })
        .collect();
    let mut tree = row(ITEM, generation::WEAPON_TREE[0], &[], &[]);
    tree.unset = vec!["tree_hash".to_owned()];
    rows.push(tree);
    Mods::from_rows(rows)
}

#[test]
fn filter_should_resolve_numbered_stat_equivalents_to_the_unnumbered_stat() {
    let mods = anoint_mods();
    let bases = [base(None)];
    let c = ctx(&mods, &bases);
    let cands: Vec<Candidate> = ["tree_hash", "hash_3", "hash", "hash_2", "hash_4", "hash_5"]
        .iter()
        .map(|s| cand(vec![lookup(s)]))
        .collect();
    let d = filter(&c, line(Section::Enchant), &cands);
    assert_eq!(d.name(), "resolved/unset-fit+numbered-stat");
    assert_eq!(d.kept, [2]);
    assert!(d.notes.contains("numbered-stat-equivalents"));
}

#[test]
fn filter_should_not_merge_candidates_that_differ_by_more_than_a_number() {
    let mods = anoint_mods();
    let bases = [base(None)];
    let c = ctx(&mods, &bases);
    // Both have a fitting unset row, but their stats differ by more than a
    // number (`x`): they stay ambiguous.
    let d = filter(
        &c,
        line(Section::Enchant),
        &[
            cand(vec![lookup("hash")]),
            cand(vec![lookup("hash_2"), exact("x", 1)]),
        ],
    );
    assert_eq!(d.name(), "ambiguous/no-backed-candidate");
    assert_eq!(d.kept, [0, 1]);
    let mut d = Decision {
        kept: vec![0, 1],
        ..Decision::default()
    };
    numbered_stat(
        &mut d,
        &[cand(vec![lookup("a")]), cand(vec![lookup("b_2")])],
    );
    assert_eq!(d.kept, [0, 1]);
    // Same stats and same numbers (two descriptions of one stat): not
    // numbered equivalents.
    numbered_stat(&mut d, &[cand(vec![lookup("a")]), cand(vec![lookup("a")])]);
    assert_eq!(d.kept, [0, 1]);
    assert!(d.rules.is_empty());
}

#[test]
fn unnumbered_should_strip_only_a_single_digit_suffix_from_two_to_nine() {
    assert_eq!(
        unnumbered("mod_granted_passive_hash_5"),
        ("mod_granted_passive_hash", 5)
    );
    assert_eq!(
        unnumbered("mod_granted_passive_hash"),
        ("mod_granted_passive_hash", 1)
    );
    assert_eq!(unnumbered("x_1"), ("x_1", 1));
    assert_eq!(unnumbered("x_10"), ("x_10", 1));
    assert_eq!(unnumbered("x_"), ("x_", 1));
}

/// `base(..)` has item class 7.
fn essence_row(stat: &str, classes: Option<Vec<i64>>) -> ModRow {
    ModRow {
        essence_only: true,
        essence_classes: classes,
        ..row(ITEM, PRE, &[(stat, 10, 20)], &[("default", 0)])
    }
}

#[test]
fn fits_should_need_the_base_class_for_an_essence_row_with_known_classes() {
    let b = base(None);
    assert!(fits(&essence_row("a", Some(vec![7])), &b, false, EXPLICIT));
    assert!(!fits(&essence_row("a", Some(vec![8])), &b, false, EXPLICIT));
    assert!(fits(&essence_row("a", None), &b, false, EXPLICIT));
}

#[test]
fn filter_should_drop_a_candidate_whose_essence_row_is_for_other_classes() {
    let mods = Mods::from_rows(vec![
        row(ITEM, PRE, &[("local_es", 10, 20)], &[("helmet", 1)]),
        essence_row("global_es", Some(vec![8])),
    ]);
    let bases = [base(Some(&["helmet"]))];
    let d = filter(&ctx(&mods, &bases), EXPLICIT, &es_candidates(15));
    assert_eq!(
        (d.kept.clone(), d.name()),
        (vec![0], "resolved/mods".to_owned())
    );
}

#[test]
fn filter_should_count_an_essence_row_for_the_base_class_as_spawning() {
    let mods = Mods::from_rows(vec![
        row(ITEM, PRE, &[("local_es", 10, 20)], &[("ring", 1)]),
        essence_row("global_es", Some(vec![7])),
    ]);
    let bases = [base(Some(&["helmet"]))];
    let d = filter(&ctx(&mods, &bases), EXPLICIT, &es_candidates(15));
    assert_eq!(
        (d.kept.clone(), d.name()),
        (vec![1], "resolved/spawn-tags".to_owned())
    );
}

/// An `Essences` table with every column of [`ESSENCE_COLUMNS`]: one row,
/// `Staff_ModsKey` = Mods row 0, `Display_Quiver_ModsKey` = row 1,
/// `Helmet_ModsKey` = row 0, `Display_Helmet_ModsKey` = row 2 (not read),
/// the rest null.
fn essences_table() -> String {
    // A summary column the map does not read, set to Mods row 2.
    let columns: Vec<String> = ESSENCE_COLUMNS
        .iter()
        .map(|(c, _)| *c)
        .chain(["Display_Helmet_ModsKey"])
        .map(|c| format!(r#"{{"name": "{c}", "references": null}}"#))
        .collect();
    let row: Vec<&str> = ESSENCE_COLUMNS
        .iter()
        .map(|(c, _)| match *c {
            "Staff_ModsKey" | "Helmet_ModsKey" => "0",
            "Display_Quiver_ModsKey" => "1",
            _ => "null",
        })
        .chain(["2"])
        .collect();
    format!(
        r#"{{"table": "Essences", "columns": [{}], "rows": [[{}]]}}"#,
        columns.join(", "),
        row.join(", ")
    )
}

fn class_ids() -> Vec<String> {
    let mut ids: Vec<String> = ESSENCE_COLUMNS
        .iter()
        .flat_map(|(_, c)| c.iter())
        .map(|c| (*c).to_owned())
        .collect();
    ids.sort();
    ids.dedup();
    ids
}

#[test]
fn essence_classes_should_map_columns_to_item_classes() {
    let ids = class_ids();
    let ids_ref: Vec<&str> = ids.iter().map(String::as_str).collect();
    let read = |p: &str| -> Result<Vec<u8>, String> {
        Ok(match p {
            "tables/Essences.json" => essences_table(),
            "tables/ItemClasses.json" => id_table("ItemClasses", &ids_ref),
            _ => return Err(format!("{p}: not found")),
        }
        .into_bytes())
    };
    let at = |id: &str| i64::try_from(ids.iter().position(|c| c == id).unwrap()).unwrap();
    let map = essence_classes(&read).unwrap().unwrap();
    assert_eq!(
        map.get(&0),
        Some(&BTreeSet::from([at("Helmet"), at("Staff"), at("Warstaff")]))
    );
    assert_eq!(map.get(&1), Some(&BTreeSet::from([at("Quiver")])));
    assert_eq!(
        map.get(&2),
        None,
        "Display_* summaries other than quiver are not read"
    );
}

fn essence_read(p: &str, ids: &[&str], mods_rows: usize) -> Result<Vec<u8>, String> {
    Ok(match p {
        "tables/Mods.json" => {
            let one = mods_table();
            let (head, row) = one.split_once("\"rows\": [").unwrap();
            let row = row.trim_end_matches("]}");
            format!("{head}\"rows\": [{}]}}", vec![row; mods_rows].join(", "))
        }
        "tables/Stats.json" => stats_table(),
        "tables/Tags.json" => id_table("Tags", &["default", "ring"]),
        "tables/Essences.json" => essences_table(),
        "tables/ItemClasses.json" => id_table("ItemClasses", ids),
        _ => return Err(format!("{p}: not found")),
    }
    .into_bytes())
}

#[test]
fn load_should_apply_essence_classes_to_the_mods_rows() {
    let ids = class_ids();
    let ids_ref: Vec<&str> = ids.iter().map(String::as_str).collect();
    let mods = load(&|p: &str| essence_read(p, &ids_ref, 3)).unwrap();
    let warstaff = i64::try_from(ids.iter().position(|c| c == "Warstaff").unwrap()).unwrap();
    assert!(
        mods.row(0)
            .unwrap()
            .essence_classes
            .as_ref()
            .unwrap()
            .contains(&warstaff)
    );
    assert!(mods.row(2).unwrap().essence_classes.is_none());
    // The fixture rows are essence-only: rows 0 and 1 known, row 2 not.
    assert_eq!(mods.essence_rows(), (2, 1));
}

#[test]
fn load_should_fail_when_essences_names_a_row_outside_mods() {
    let ids = class_ids();
    let ids_ref: Vec<&str> = ids.iter().map(String::as_str).collect();
    assert!(load(&|p: &str| essence_read(p, &ids_ref, 1)).is_err());
}

#[test]
fn fits_should_keep_an_essence_row_when_the_base_class_is_unknown() {
    let b = BaseRow {
        class: None,
        ..base(None)
    };
    assert!(fits(&essence_row("a", Some(vec![8])), &b, false, EXPLICIT));
}

#[test]
fn filter_should_count_an_ordinary_row_an_essence_puts_on_the_base_class_as_spawning() {
    let mut ordinary = row(ITEM, PRE, &[("global_es", 10, 20)], &[("ring", 1)]);
    ordinary.essence_classes = Some(vec![7]);
    let mods = Mods::from_rows(vec![
        row(ITEM, PRE, &[("local_es", 10, 20)], &[("ring", 1)]),
        ordinary,
    ]);
    let bases = [base(Some(&["helmet"]))];
    let d = filter(&ctx(&mods, &bases), EXPLICIT, &es_candidates(15));
    assert_eq!(
        (d.kept.clone(), d.name()),
        (vec![1], "resolved/spawn-tags".to_owned())
    );
}

#[test]
fn essence_classes_should_be_none_without_the_table_and_fail_on_a_missing_class() {
    let none = |p: &str| -> Result<Vec<u8>, String> { Err(format!("{p}: not found")) };
    assert!(essence_classes(&none).unwrap().is_none());
    let read = |p: &str| -> Result<Vec<u8>, String> {
        Ok(match p {
            "tables/Essences.json" => essences_table(),
            "tables/ItemClasses.json" => id_table("ItemClasses", &["Helmet"]),
            _ => return Err(format!("{p}: not found")),
        }
        .into_bytes())
    };
    assert!(essence_classes(&read).is_err());
}

#[test]
fn filter_should_keep_backed_numbered_candidates_ambiguous() {
    // Both `x` and `x_2` are backed by fitting rows: a real ambiguity, not
    // the anoint's unknown slot, so `numbered-stat` does not run.
    let mods = Mods::from_rows(vec![
        row(ITEM, PRE, &[("x", 1, 5)], &[]),
        row(ITEM, PRE, &[("x_2", 1, 5)], &[]),
    ]);
    let bases = [base(None)];
    let c = ctx(&mods, &bases);
    let d = filter(
        &c,
        EXPLICIT,
        &[cand(vec![exact("x", 3)]), cand(vec![exact("x_2", 3)])],
    );
    assert_eq!(d.name(), "ambiguous/several-backed");
    assert!(!d.notes.contains("numbered-stat-equivalents"));
}

/// Crafted `local_es` and `global_es` rows (no spawn tags for crafted
/// lines), `local_es` an `IsLocal` stat.
fn local_global_mods() -> Mods {
    let mut mods = Mods::from_rows(vec![
        row(domain::CRAFTED, PRE, &[("local_es", 10, 20)], &[]),
        row(domain::CRAFTED, PRE, &[("global_es", 10, 20)], &[]),
    ]);
    mods.locality.insert("local_es".to_owned(), Locality::Item);
    mods
}

const CRAFTED: LineContext = LineContext {
    crafted: true,
    ..EXPLICIT
};

#[test]
fn filter_should_keep_the_local_reading_on_a_base_that_carries_the_property() {
    let mods = local_global_mods();
    let bases = [base(Some(&["armour", "helmet"]))];
    let d = filter(&ctx(&mods, &bases), CRAFTED, &es_candidates(15));
    assert_eq!(
        (d.kept.clone(), d.name()),
        (vec![0], "resolved/local-stat".to_owned())
    );
}

#[test]
fn filter_should_keep_the_global_reading_on_a_base_without_the_property() {
    let mods = local_global_mods();
    let bases = [base(Some(&["ring"]))];
    let d = filter(&ctx(&mods, &bases), CRAFTED, &es_candidates(15));
    assert_eq!(
        (d.kept.clone(), d.name()),
        (vec![1], "resolved/local-stat".to_owned())
    );
}

#[test]
fn filter_should_need_the_weapon_tag_for_a_weapon_local_stat() {
    let mut mods = local_global_mods();
    mods.locality
        .insert("local_es".to_owned(), Locality::Weapon);
    let bases = [base(Some(&["armour", "gloves"]))];
    let d = filter(&ctx(&mods, &bases), CRAFTED, &es_candidates(15));
    assert_eq!(d.kept, vec![1]);
}

#[test]
fn filter_should_not_apply_local_stat_to_unique_or_implicit_lines() {
    let mods = local_global_mods();
    let bases = [base(Some(&["ring"]))];
    let implicit = line(Section::Implicit);
    let d = filter(&ctx(&mods, &bases), implicit, &es_candidates(15));
    assert!(!d.rules.contains(&"local-stat"));
    let unique = Context {
        unique: true,
        ..ctx(&mods, &bases)
    };
    assert!(
        !filter(&unique, CRAFTED, &es_candidates(15))
            .rules
            .contains(&"local-stat")
    );
}

#[test]
fn filter_should_not_drop_a_local_reading_that_spawns_on_the_base() {
    let mut mods = Mods::from_rows(vec![
        row(ITEM, PRE, &[("local_es", 10, 20)], &[("flask", 1)]),
        row(ITEM, PRE, &[("global_es", 10, 20)], &[("flask", 1)]),
    ]);
    mods.locality.insert("local_es".to_owned(), Locality::Item);
    let bases = [base(Some(&["flask"]))];
    let d = filter(&ctx(&mods, &bases), EXPLICIT, &es_candidates(15));
    assert_eq!(d.kept, vec![0, 1]);
    assert!(d.notes.contains("local-stat-spawns"));
}

#[test]
fn filter_should_read_a_legacy_essence_row_through_its_mod_type() {
    // The essence row's `ModType` (1) has a row that is not essence-only and
    // spawns nowhere: the essence row spawns nowhere, and `spawn-tags` runs.
    let mut essence = row(ITEM, PRE, &[("old_es", 10, 20)], &[]);
    essence.essence_only = true;
    essence.mod_type = Some(1);
    let mut sibling = row(ITEM, PRE, &[("old_es", 1, 2)], &[("default", 0)]);
    sibling.mod_type = Some(1);
    let mods = Mods::from_rows(vec![
        row(ITEM, PRE, &[("new_es", 10, 20)], &[("helmet", 1)]),
        essence,
        sibling,
    ]);
    assert_eq!(mods.row(1).unwrap().type_spawn, Some(Vec::new()));
    let bases = [base(Some(&["helmet"]))];
    let cs = vec![
        cand(vec![exact("new_es", 15)]),
        cand(vec![exact("old_es", 15)]),
    ];
    let d = filter(&ctx(&mods, &bases), EXPLICIT, &cs);
    assert_eq!(
        (d.kept.clone(), d.name()),
        (vec![0], "resolved/spawn-tags".to_owned())
    );
}

#[test]
fn filter_should_count_a_legacy_essence_row_where_its_mod_type_spawns() {
    let mut essence = row(ITEM, PRE, &[("old_es", 10, 20)], &[]);
    essence.essence_only = true;
    essence.mod_type = Some(1);
    let mut sibling = row(ITEM, PRE, &[("old_es", 1, 2)], &[("helmet", 5)]);
    sibling.mod_type = Some(1);
    let mods = Mods::from_rows(vec![essence, sibling]);
    let r = mods.row(0).unwrap();
    assert!(r.type_spawns_on(&["helmet".to_owned()]) && !r.type_spawns_on(&["ring".to_owned()]));
    assert!(!r.essence_unknown());
}

#[test]
fn filter_should_break_a_tie_by_a_spawning_row_with_the_value() {
    // Both readings spawn; only `new_es` through a row whose range holds 15.
    let mods = Mods::from_rows(vec![
        row(ITEM, PRE, &[("new_es", 10, 20)], &[("helmet", 1)]),
        row(ITEM, PRE, &[("old_es", 1, 2)], &[("helmet", 1)]),
        row(ITEM, PRE, &[("old_es", 10, 20)], &[("helmet", 0)]),
    ]);
    let bases = [base(Some(&["helmet"]))];
    let cs = vec![
        cand(vec![exact("new_es", 15)]),
        cand(vec![exact("old_es", 15)]),
    ];
    let d = filter(&ctx(&mods, &bases), EXPLICIT, &cs);
    assert_eq!(
        (d.kept.clone(), d.name()),
        (vec![0], "resolved/spawn-values".to_owned())
    );
}

#[test]
fn filter_should_not_break_a_tie_by_spawn_values_when_spawn_tags_was_skipped() {
    let mods = Mods::from_rows(vec![
        row(
            domain::CRAFTED,
            PRE,
            &[("new_es", 10, 20)],
            &[("helmet", 1)],
        ),
        row(domain::CRAFTED, PRE, &[("old_es", 10, 20)], &[]),
    ]);
    let bases = [base(Some(&["helmet"]))];
    let cs = vec![
        cand(vec![exact("new_es", 15)]),
        cand(vec![exact("old_es", 15)]),
    ];
    let d = filter(&ctx(&mods, &bases), CRAFTED, &cs);
    assert_eq!(d.kept, vec![0, 1]);
}

#[test]
fn filter_should_skip_local_stat_when_base_rows_differ_or_tags_are_unknown() {
    let mods = local_global_mods();
    let differ = [base(Some(&["armour"])), base(Some(&["ring"]))];
    let d = filter(&ctx(&mods, &differ), CRAFTED, &es_candidates(15));
    assert_eq!(d.kept, vec![0, 1]);
    assert!(d.notes.contains("local-stat-bases-differ"));
    let partly_unknown = [base(Some(&["ring"])), base(None)];
    let d = filter(&ctx(&mods, &partly_unknown), CRAFTED, &es_candidates(15));
    assert!(d.notes.contains("local-stat-bases-differ"));
    let unknown = [base(None)];
    let d = filter(&ctx(&mods, &unknown), CRAFTED, &es_candidates(15));
    assert!(d.notes.contains("local-stat-tags-unknown"));
}

#[test]
fn filter_should_note_a_candidate_with_weapon_and_item_local_stats() {
    let mut mods = Mods::from_rows(vec![
        row(
            domain::CRAFTED,
            PRE,
            &[("local_es", 10, 20), ("local_acc", 10, 20)],
            &[],
        ),
        row(domain::CRAFTED, PRE, &[("global_es", 10, 20)], &[]),
    ]);
    mods.locality.insert("local_es".to_owned(), Locality::Item);
    mods.locality
        .insert("local_acc".to_owned(), Locality::Weapon);
    let cs = vec![
        cand(vec![exact("local_es", 15), exact("local_acc", 15)]),
        cand(vec![exact("global_es", 15)]),
    ];
    let bases = [base(Some(&["ring"]))];
    let d = filter(&ctx(&mods, &bases), CRAFTED, &cs);
    assert!(d.notes.contains("local-stat-mixed") && !d.rules.contains(&"local-stat"));
}

#[test]
fn filter_should_read_global_on_a_belt_even_when_only_the_local_row_holds_the_value() {
    // The B5a Crystal Belt line: `values` alone would keep the local reading
    // (52 inside the local row only); belts carry no defence property.
    let mut mods = Mods::from_rows(vec![
        row(ITEM, PRE, &[("local_es", 50, 60)], &[("armour", 1)]),
        row(ITEM, PRE, &[("global_es", 10, 40)], &[("belt", 1)]),
    ]);
    mods.locality.insert("local_es".to_owned(), Locality::Item);
    let bases = [base(Some(&["belt"]))];
    let d = filter(&ctx(&mods, &bases), EXPLICIT, &es_candidates(52));
    assert_eq!(
        (d.kept.clone(), d.name()),
        (vec![1], "resolved/local-stat".to_owned())
    );
}

#[test]
fn type_spawn_should_take_rows_of_the_same_domain_and_generation_only() {
    let mut essence = row(ITEM, PRE, &[("old_es", 10, 20)], &[]);
    essence.essence_only = true;
    essence.mod_type = Some(1);
    let mut suffix = row(
        ITEM,
        generation::SUFFIX,
        &[("old_es", 1, 2)],
        &[("helmet", 5)],
    );
    suffix.mod_type = Some(1);
    let mods = Mods::from_rows(vec![essence, suffix]);
    let r = mods.row(0).unwrap();
    assert_eq!(r.type_spawn, None);
    assert!(r.essence_unknown());
}
