use loom_data::stat_desc::{DescRef, Hand, StatValue};

use super::*;

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
    assert_eq!(at(NO_INFLUENCE), "NONE");
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
            "tables/Stats.json" => id_table("Stats", &["s0", "s1", "s2"]),
            "tables/Tags.json" => id_table("Tags", &["default", "ring"]),
            _ => return Err(format!("{p}: not found")),
        }
        .into_bytes())
    };
    let mods = Mods::load(&read).unwrap();
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
    assert_eq!(mods.rows_for(&[exact("s2", 8)]).len(), 1);
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
    let d = filter(
        &c,
        enchant,
        &[cand(vec![keys]), cand(vec![exact("other", 1)])],
    );
    assert_eq!(d.name(), "ambiguous/no-backed-candidate");
    assert!(d.notes.contains("row-value-unset"));
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
/// `Helmet_ModsKey` = row 0, the rest null.
fn essences_table() -> String {
    let columns: Vec<String> = ESSENCE_COLUMNS
        .iter()
        .map(|(c, _)| format!(r#"{{"name": "{c}", "references": null}}"#))
        .collect();
    let row: Vec<&str> = ESSENCE_COLUMNS
        .iter()
        .map(|(c, _)| match *c {
            "Staff_ModsKey" | "Helmet_ModsKey" => "0",
            "Display_Quiver_ModsKey" => "1",
            _ => "null",
        })
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
