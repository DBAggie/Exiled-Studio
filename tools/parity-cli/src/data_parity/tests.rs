//! Tests of `data-parity` on the synthetic PoE1 bundle
//! (`crates/loom-data/tests/fixtures/bundle-poe1`) and a hand-written PoB
//! export equal to it: every value is written as PoB's exporter would have
//! written the bundle's row, so the equal export has no difference and
//! each test changes one value.

use std::path::Path;

use serde_json::{Value, json};

use super::compare::{self, Difference, lua, num};
use super::rules::{Category, Rules, glob};
use super::{Request, UNCATEGORIZED, pob, run};
use crate::resolve_corpus::Tool;

/// The synthetic bundle has no passive description file; its main file
/// stands in (it describes none of the synthetic passive stats, so they
/// render no line, as a stat without a description shows none in game).
const SCOPE: &str = "stat_descriptions";

fn bundle(name: &str) -> loom_data::bundle::Bundle {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/loom-data/tests/fixtures")
        .join(name);
    crate::load_bundle(&dir).unwrap()
}

/// The export PoB would write for the synthetic bundle.
fn equal_export() -> Value {
    json!({
        "format": 1,
        "pob": {"version": "2.67.2"},
        "tree_version": "3_29",
        "skill_types": {},
        "skills": {
            "TestBolt": {
                "castTime": 0.75,
                "baseEffectiveness": 1.5,
                // f32 0.03 widened, at 14 significant digits.
                "incrementalEffectiveness": 0.029_999_999_329_448,
                "stats": ["test_base_damage_min", "test_base_damage_max", "test_damage_per_level", "test_spell_flag"],
                "types": ["TestSpell"],
                "constant_stats": [{"stat": "test_damage_per_level", "value": 5}],
                "quality_stats": [{"stat": "test_quality_cast_speed", "value": 0.5}],
                "levels": {
                    "1": {"values": [4.5, 9, 1], "cost": {"Mana": 6}, "critChance": 6},
                    "2": {"values": [6.25, 12.5, 2], "cost": {"Mana": 7}, "critChance": 6}
                }
            },
            "TestStrike": {
                "castTime": 1,
                "stats": {},
                "types": ["TestAttack"],
                "constant_stats": {},
                "quality_stats": {},
                "levels": {}
            },
            "SupportTestDamage": {
                "support": true,
                "stats": ["test_support_more_damage"],
                "types": {},
                "constant_stats": {},
                "quality_stats": {},
                "levels": {
                    "1": {"values": [20], "cost": {}, "manaMultiplier": 30, "manaReservationPercent": 25}
                }
            }
        },
        "gems": {
            "Test/Gems/Bolt": {
                "gameId": "Test/Gems/Bolt", "variantId": "TestBolt", "grantedEffectId": "TestBolt",
                "reqStr": 0, "reqDex": 0, "reqInt": 100, "tags": ["test_spell"]
            },
            "Test/Gems/SupportDamage": {
                "gameId": "Test/Gems/SupportDamage", "variantId": "SupportTestDamage",
                "grantedEffectId": "SupportTestDamage", "reqStr": 50, "reqDex": 50, "reqInt": 0,
                "tags": ["test_support"]
            }
        },
        "bases": {
            "Plain Test Wand": {
                "type": "Wand",
                "weapon": {"PhysicalMin": 5, "PhysicalMax": 9, "CritChanceBase": 7, "AttackRateBase": 1.4, "Range": 120}
            },
            "Plain Test Shield": {
                "type": "Shield",
                "armour": {"ArmourBaseMin": 9, "ArmourBaseMax": 12, "MovementPenalty": 3, "BlockChance": 24}
            }
        },
        "monster_tables": {
            "monsterEvasionTable": [67, 134],
            "monsterAccuracyTable": [14, 28],
            "monsterLifeTable": [22, 28],
            "monsterLifeTable2": [10, 20],
            "monsterLifeTable3": [13, 26],
            "monsterAllyLifeTable": [15, 30],
            "monsterDamageTable": [4.989_999_771_118_2, 5.5],
            "monsterAllyDamageTable": [5.619_999_885_559_1, 11.239_999_771_118],
            "monsterArmourTable": [16, 32],
            "monsterAilmentThresholdTable": [22, 28],
            "monsterPhysConversionMultiTable": [0, 0]
        },
        "game_constants": {"MonsterDamageReductionImprovement": 6.7},
        "character_constants": {},
        "monster_constants": {},
        "tree": {
            "classes": [
                {"name": "Test Brute", "base_str": 32, "base_dex": 14, "base_int": 14, "ascendancies": ["TestChampion"]},
                {"name": "Test Sage", "base_str": 14, "base_dex": 14, "base_int": 32, "ascendancies": {}}
            ],
            "nodes": {
                "100": {"skill": 100, "name": "Test Start", "stats": {}, "expansionJewel": false, "masteryEffects": {}},
                "101": {"skill": 101, "name": "Test Life", "stats": {}, "expansionJewel": false, "masteryEffects": {}},
                "102": {"skill": 102, "name": "Test Keystone", "stats": {}, "isKeystone": true, "expansionJewel": false, "masteryEffects": {}},
                "103": {"skill": 103, "name": "Test Mastery", "stats": {}, "isMastery": true, "expansionJewel": false,
                        "masteryEffects": [{"effect": 4242, "stats": {}}]},
                "200": {"skill": 200, "name": "Test Champion", "stats": {}, "ascendancyName": "TestChampion",
                        "isAscendancyStart": true, "expansionJewel": false, "masteryEffects": {}}
            }
        }
    })
}

fn differences(export: &Value) -> Vec<Difference> {
    let export = pob::parse(&serde_json::to_vec(export).unwrap()).unwrap();
    compare::compare(&bundle("bundle-poe1"), &export, SCOPE)
        .unwrap()
        .differences
}

fn fields(found: &[Difference]) -> Vec<(&str, &str, &str)> {
    found
        .iter()
        .map(|d| (d.table, d.key.as_str(), d.field.as_str()))
        .collect()
}

#[test]
fn equal_export_should_have_no_difference() {
    let export = pob::parse(&serde_json::to_vec(&equal_export()).unwrap()).unwrap();
    let comparison = compare::compare(&bundle("bundle-poe1"), &export, SCOPE).unwrap();
    assert_eq!(comparison.differences, []);
    // Every table compared something; bundle rows PoB lacks are counted.
    for table in super::TABLES {
        if !["character_constants", "monster_constants"].contains(&table) {
            assert!(comparison.tables[table].compared > 0, "{table}");
        }
    }
    assert_eq!(comparison.tables["monster_levels"].compared, 22);
}

#[test]
fn each_changed_value_should_be_one_difference() {
    // (pointer into the export, new value, expected (table, key, field)).
    let cases: [(&str, Value, (&str, &str, &str)); 14] = [
        (
            "/skills/TestBolt/castTime",
            json!(0.8),
            ("skills", "TestBolt", "cast_time"),
        ),
        (
            "/skills/TestBolt/levels/2/values/1",
            json!(12),
            ("skills", "TestBolt", "level.2.stat.test_base_damage_max"),
        ),
        (
            "/skills/TestBolt/levels/1/cost/Mana",
            json!(5),
            ("skills", "TestBolt", "level.1.cost.Mana"),
        ),
        (
            "/skills/TestBolt/levels/1/critChance",
            json!(5),
            ("skills", "TestBolt", "level.1.crit_chance"),
        ),
        (
            "/skills/TestBolt/quality_stats/0/value",
            json!(1),
            ("skills", "TestBolt", "quality_stats"),
        ),
        (
            "/skills/TestBolt/incrementalEffectiveness",
            json!(0.03),
            ("skills", "TestBolt", "incremental_effectiveness"),
        ),
        (
            "/skills/SupportTestDamage/levels/1/manaReservationPercent",
            json!(30),
            (
                "skills",
                "SupportTestDamage",
                "level.1.mana_reservation_percent",
            ),
        ),
        (
            "/gems/Test~1Gems~1Bolt/reqInt",
            json!(60),
            ("gems", "TestBolt", "requirements"),
        ),
        (
            "/bases/Plain Test Wand/weapon/AttackRateBase",
            json!(1.41),
            ("bases", "Plain Test Wand", "attack_rate"),
        ),
        (
            "/bases/Plain Test Shield/armour/MovementPenalty",
            json!(0),
            ("bases", "Plain Test Shield", "movement_penalty"),
        ),
        (
            "/monster_tables/monsterArmourTable/1",
            json!(30),
            ("monster_levels", "2", "armour"),
        ),
        (
            "/game_constants/MonsterDamageReductionImprovement",
            json!(6.8),
            (
                "game_constants",
                "MonsterDamageReductionImprovement",
                "value",
            ),
        ),
        (
            "/tree/classes/0/base_str",
            json!(20),
            ("characters", "Test Brute", "strength"),
        ),
        (
            "/tree/nodes/101/stats",
            json!(["+10 to maximum Life"]),
            ("passives", "101", "stats"),
        ),
    ];
    for (pointer, value, expected) in cases {
        let mut export = equal_export();
        *export
            .pointer_mut(pointer)
            .unwrap_or_else(|| panic!("{pointer}")) = value;
        let found = differences(&export);
        assert_eq!(fields(&found), [expected], "{pointer}");
    }
}

#[test]
fn rows_on_one_side_should_be_counted() {
    let mut export = equal_export();
    let skills = export["skills"].as_object_mut().unwrap();
    skills.insert(
        "PobOnly".to_owned(),
        json!({"stats": {}, "types": {}, "constant_stats": {}, "quality_stats": {}, "levels": {}}),
    );
    skills.remove("TestStrike");
    let export = pob::parse(&serde_json::to_vec(&export).unwrap()).unwrap();
    let comparison = compare::compare(&bundle("bundle-poe1"), &export, SCOPE).unwrap();
    assert_eq!(
        fields(&comparison.differences),
        [("skills", "PobOnly", "row")]
    );
    let skills = &comparison.tables["skills"];
    assert_eq!((skills.pob_only_rows, skills.bundle_only_rows), (1, 1));
}

#[test]
fn granted_passive_points_should_compare_as_a_number() {
    let mut export = equal_export();
    export["tree"]["nodes"]["101"]["stats"] = json!(["Grants 2 Passive Skill Points"]);
    assert_eq!(
        fields(&differences(&export)),
        [("passives", "101", "skill_points")]
    );
}

#[test]
fn numbers_should_compare_at_lua_precision() {
    assert_eq!(lua(f64::from(41.26_f32)), 41.259_998_321_533);
    assert_eq!(lua(1.0 / f64::from(0.06_f32)), 16.666_667_039_196);
    assert_eq!(lua(0.0), 0.0);
    assert_eq!(num(5.0), json!(5));
    assert_eq!(num(-3.0), json!(-3));
    assert_eq!(num(0.5), json!(0.5));
}

#[test]
fn glob_should_match_runs() {
    assert!(glob("*", ""));
    assert!(glob("level.*.stat.*", "level.20.stat.a_b"));
    assert!(glob(
        "TriggerSummonedAncestor*",
        "TriggerSummonedAncestorKaom"
    ));
    assert!(glob("a*b*c", "abc"));
    assert!(!glob("a*b*b", "ab"));
    assert!(!glob("level.31", "level.3"));
    assert!(!glob("row", "rows"));
}

fn rules_json(rules: &Value) -> Vec<u8> {
    serde_json::to_vec(&json!({"format": 1, "game": "poe1", "rules": rules})).unwrap()
}

fn rule(id: &str, category: &str, table: &str, field: &str, key: &str, source: &str) -> Value {
    json!({"id": id, "category": category, "table": table, "fields": [field], "keys": [key],
           "evidence": "why", "sources": [source]})
}

#[test]
fn rules_should_be_checked() {
    let thread = "https://www.pathofexile.com/forum/view-thread/1";
    let ok = rule("a", "patch-delta", "skills", "*", "*", thread);
    assert!(Rules::parse(&rules_json(&json!([ok])), "poe1").is_ok());
    let bad: [(Value, &str); 8] = [
        (
            json!([rule("", "representation", "skills", "*", "*", "game:X.Y")]),
            "empty id",
        ),
        (json!([ok, ok]), "duplicate id"),
        (
            json!([rule("a", "representation", "nope", "*", "*", "game:X.Y")]),
            "unknown table",
        ),
        (
            json!([rule(
                "a",
                "patch-delta",
                "skills",
                "*",
                "*",
                "v2.67.2:src/x.lua:1"
            )]),
            "patch-note thread",
        ),
        (
            json!([rule("a", "pob-derivation", "skills", "*", "*", thread)]),
            "PoB citation",
        ),
        (
            json!([rule(
                "a",
                "not-carried",
                "skills",
                "*",
                "*",
                "v2.67.2:src/x.lua:1"
            )]),
            "game data column",
        ),
        (
            json!([{"id": "a", "category": "representation", "table": "skills", "fields": [], "keys": ["*"],
                 "evidence": "why", "sources": ["game:X.Y"]}]),
            "no field",
        ),
        (
            json!([{"id": "a", "category": "representation", "table": "skills", "fields": ["*"], "keys": ["*"],
                 "evidence": " ", "sources": ["game:X.Y"]}]),
            "no evidence",
        ),
    ];
    for (rules, expected) in bad {
        let error = Rules::parse(&rules_json(&rules), "poe1").unwrap_err();
        assert!(error.contains(expected), "{error}");
    }
    assert!(
        Rules::parse(&rules_json(&json!([])), "poe2")
            .unwrap_err()
            .contains("for poe1")
    );
}

fn run_with(export: &Value, rules: &Value, bundle_name: &str) -> Result<super::Outputs, String> {
    let bundle = bundle(bundle_name);
    let export = serde_json::to_vec(export).unwrap();
    let rules = rules_json(rules);
    run(&Request {
        bundle: &bundle,
        bundle_manifest: b"{}",
        export: &export,
        rules: &rules,
        passive_scope: SCOPE,
        tool: Tool::default(),
    })
}

/// One difference per category, and the overlay takes PoB's value only for
/// the two that change a value.
#[test]
fn categories_should_decide_the_overlay() {
    let mut export = equal_export();
    export["skills"]["TestBolt"]["castTime"] = json!(0.8); // patch-delta
    export["monster_tables"]["monsterArmourTable"][0] = json!(12); // pob-derivation
    export["tree"]["nodes"]["102"]["isNotable"] = json!(true); // representation
    export["tree"]["nodes"]["101"]["stats"] = json!(["Aura line"]); // not-carried
    let thread = "https://www.pathofexile.com/forum/view-thread/1";
    let rules = json!([
        rule(
            "cast",
            "patch-delta",
            "skills",
            "cast_time",
            "TestBolt",
            thread
        ),
        rule(
            "armour",
            "pob-derivation",
            "monster_levels",
            "armour",
            "*",
            "v2.67.2:src/x.lua:1"
        ),
        rule(
            "notable",
            "representation",
            "passives",
            "notable",
            "102",
            "game:PassiveSkills.IsNotable"
        ),
        rule(
            "buff",
            "not-carried",
            "passives",
            "stats",
            "101",
            "game:PassiveSkills.PassiveSkillBuffs"
        ),
    ]);
    let out = run_with(&export, &rules, "bundle-poe1").unwrap();
    assert!(out.summary.pass);
    for (category, count) in [
        ("patch-delta", 1),
        ("pob-derivation", 1),
        ("representation", 1),
        ("not-carried", 1),
        (UNCATEGORIZED, 0),
    ] {
        assert_eq!(out.summary.categories[category], count, "{category}");
    }
    let rows: Vec<(&str, &str, &Value, Category)> = out
        .overlay
        .rows
        .iter()
        .map(|r| (r.table, r.field.as_str(), &r.pob, r.category))
        .collect();
    assert_eq!(
        rows,
        [
            ("skills", "cast_time", &json!(0.8), Category::PatchDelta),
            (
                "monster_levels",
                "armour",
                &json!(12),
                Category::PobDerivation
            ),
        ]
    );
    assert_eq!(out.summary.overlay_rows, 2);
    assert!(out.report.differences.iter().all(|d| d.rule.is_some()));
}

#[test]
fn uncategorized_or_unused_should_fail() {
    let mut export = equal_export();
    export["skills"]["TestBolt"]["castTime"] = json!(0.8);
    let out = run_with(&export, &json!([]), "bundle-poe1").unwrap();
    assert!(!out.summary.pass);
    assert_eq!(out.summary.categories[UNCATEGORIZED], 1);
    assert!(out.report.differences[0].category.is_none());
    assert!(out.overlay.rows.is_empty());

    let stale = json!([rule(
        "stale",
        "representation",
        "gems",
        "tags",
        "*",
        "game:GemEffects.Tags"
    )]);
    let out = run_with(&equal_export(), &stale, "bundle-poe1").unwrap();
    assert!(!out.summary.pass);
    assert_eq!(out.summary.unused_rules, ["stale"]);
}

#[test]
fn first_matching_rule_should_win() {
    let mut export = equal_export();
    export["skills"]["TestBolt"]["castTime"] = json!(0.8);
    let thread = "https://www.pathofexile.com/forum/view-thread/1";
    let rules = json!([
        rule(
            "narrow",
            "patch-delta",
            "skills",
            "cast_time",
            "TestBolt",
            thread
        ),
        rule(
            "broad",
            "representation",
            "skills",
            "*",
            "*",
            "game:GrantedEffects.CastTime"
        ),
    ]);
    let out = run_with(&export, &rules, "bundle-poe1").unwrap();
    assert_eq!(out.report.differences[0].rule.as_deref(), Some("narrow"));
    assert_eq!(out.summary.unused_rules, ["broad"]);
}

#[test]
fn bad_inputs_should_not_start() {
    // A PoE2 bundle: PoB v2.67.2 is the PoE1 oracle.
    assert!(
        run_with(&equal_export(), &json!([]), "bundle-poe2")
            .unwrap_err()
            .contains("PoE1")
    );
    // Another export format, and an unknown field.
    let mut export = equal_export();
    export["format"] = json!(2);
    assert!(
        run_with(&export, &json!([]), "bundle-poe1")
            .unwrap_err()
            .contains("format 2")
    );
    let mut export = equal_export();
    export["extra"] = json!(1);
    assert!(run_with(&export, &json!([]), "bundle-poe1").is_err());
    // A non-empty object where a list belongs.
    let mut export = equal_export();
    export["skills"]["TestBolt"]["types"] = json!({"a": 1});
    assert!(run_with(&export, &json!([]), "bundle-poe1").is_err());
}
