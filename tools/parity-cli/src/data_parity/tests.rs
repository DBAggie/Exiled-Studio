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
        "format": 3,
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
        "bosses": {
            "Armoured": {"armourMult": 50, "evasionMult": 0, "isUber": true},
            "Evasive": {"armourMult": 0, "evasionMult": 100, "isUber": false}
        },
        "pantheons": {
            "TestMajorGod": {"isMajorGod": true, "souls": [
                {"name": "Soul of Test", "mods": [{"line": "+10 to maximum Life", "value": [10]}]},
                {"name": "Test Upgrade", "mods": [{"line": "", "value": [5]}, {"line": "+2 to Strength", "value": [2]}]}
            ]},
            "TestMinorGod": {"isMajorGod": false, "souls": [
                {"name": "Soul of Minor Test", "mods": [{"line": "+4 to maximum Life", "value": [4]}]}
            ]}
        },
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

/// The synthetic bundle's bosses (`make_bundles.py`).
const BOSSES: [loom_calc::poe1::enemy::Boss; 2] = [
    loom_calc::poe1::enemy::Boss {
        name: "Armoured",
        monster_type: "TestBossArmoured",
        uber: true,
    },
    loom_calc::poe1::enemy::Boss {
        name: "Evasive",
        monster_type: "TestBossEvasive",
        uber: false,
    },
];

fn differences(export: &Value) -> Vec<Difference> {
    let export = pob::parse(&serde_json::to_vec(export).unwrap()).unwrap();
    compare::compare(&bundle("bundle-poe1"), &export, SCOPE, &BOSSES)
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
    let comparison = compare::compare(&bundle("bundle-poe1"), &export, SCOPE, &BOSSES).unwrap();
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
fn rows_on_one_side_should_be_listed() {
    let mut export = equal_export();
    let skills = export["skills"].as_object_mut().unwrap();
    skills.insert(
        "PobOnly".to_owned(),
        json!({"stats": {}, "types": {}, "constant_stats": {}, "quality_stats": {}, "levels": {}}),
    );
    skills.remove("TestStrike");
    // A bundle level PoB does not have.
    export["skills"]["TestBolt"]["levels"]
        .as_object_mut()
        .unwrap()
        .remove("2");
    let export = pob::parse(&serde_json::to_vec(&export).unwrap()).unwrap();
    let comparison = compare::compare(&bundle("bundle-poe1"), &export, SCOPE, &BOSSES).unwrap();
    assert_eq!(
        fields(&comparison.differences),
        [("skills", "PobOnly", "row")]
    );
    let only: Vec<(&str, &str)> = comparison
        .bundle_only
        .iter()
        .map(|b| (b.table, b.key.as_str()))
        .collect();
    assert_eq!(
        only,
        [("skills", "TestStrike"), ("skills", "TestBolt:level.2")]
    );
    let skills = &comparison.tables["skills"];
    assert_eq!((skills.pob_only_rows, skills.bundle_only_rows), (1, 2));
}

#[test]
fn levels_should_be_found_as_pob_pairs_them() {
    // A level the bundle does not have.
    let mut export = equal_export();
    export["skills"]["TestBolt"]["levels"]["3"] =
        json!({"values": [4.5, 9, 1], "cost": {"Mana": 6}, "critChance": 6});
    assert_eq!(
        fields(&differences(&export)),
        [("skills", "TestBolt", "level.3")]
    );
    // A table with one row serves every level (skills.lua:263-264).
    let mut export = equal_export();
    export["skills"]["SupportTestDamage"]["levels"]["2"] =
        export["skills"]["SupportTestDamage"]["levels"]["1"].clone();
    assert_eq!(differences(&export), []);
    // A level key that is not a number.
    let mut export = equal_export();
    export["skills"]["TestBolt"]["levels"]["x"] = json!({"values": {}, "cost": {}});
    assert_eq!(
        fields(&differences(&export)),
        [("skills", "TestBolt", "level.x")]
    );
}

#[test]
fn crit_chance_should_prefer_the_second_column() {
    // PoB's OffhandCritChance (the bundle's attack_crit_chance) overrides
    // its AttackCritChance (spell_crit_chance) when not 0.
    let mut bundle = bundle("bundle-poe1");
    let skills = bundle.skills.as_mut().unwrap();
    let set = skills
        .stat_sets
        .iter_mut()
        .find(|s| s.id == "TestBolt")
        .unwrap();
    set.levels[0].attack_crit_chance = 750;
    let mut export = equal_export();
    export["skills"]["TestBolt"]["levels"]["1"]["critChance"] = json!(7.5);
    let export = pob::parse(&serde_json::to_vec(&export).unwrap()).unwrap();
    assert_eq!(
        compare::compare(&bundle, &export, SCOPE, &BOSSES)
            .unwrap()
            .differences,
        []
    );
}

#[test]
fn absent_should_equal_zero_only_where_pob_omits_zero() {
    // Absent cost on the bundle side against PoB's 0: a difference.
    let mut export = equal_export();
    export["skills"]["SupportTestDamage"]["levels"]["1"]["cost"] = json!({"Mana": 0});
    assert_eq!(
        fields(&differences(&export)),
        [("skills", "SupportTestDamage", "level.1.cost.Mana")]
    );
    // A stored-uses value PoB has and the bundle has not (0).
    let mut export = equal_export();
    export["skills"]["TestBolt"]["levels"]["1"]["storedUses"] = json!(1);
    assert_eq!(
        fields(&differences(&export)),
        [("skills", "TestBolt", "level.1.stored_uses")]
    );
    // Armour ranges and block: absent is 0.
    let mut export = equal_export();
    export["bases"]["Plain Test Shield"]["armour"]["EvasionBaseMax"] = json!(5);
    export["bases"]["Plain Test Shield"]["armour"]["BlockChance"] = json!(25);
    assert_eq!(
        fields(&differences(&export)),
        [
            ("bases", "Plain Test Shield", "evasion_max"),
            ("bases", "Plain Test Shield", "block")
        ]
    );
}

#[test]
fn gems_characters_and_masteries_should_compare() {
    let mut export = equal_export();
    export["gems"]["Test/Gems/Bolt"]["tags"] = json!(["test_spell", "fire"]);
    export["gems"]["Test/Gems/Bolt"]["vaalGem"] = json!(true);
    export["gems"]["Test/Gems/Bolt"]["secondaryGrantedEffectId"] = json!("TestStrike");
    export["tree"]["classes"][1]["ascendancies"] = json!(["TestSage"]);
    export["tree"]["nodes"]["103"]["masteryEffects"][0]["stats"] = json!(["5 Life"]);
    export["character_constants"] = json!({"base_speed": 37});
    assert_eq!(
        fields(&differences(&export)),
        [
            ("gems", "TestBolt", "secondary_granted_effect"),
            ("gems", "TestBolt", "tags"),
            ("gems", "TestBolt", "vaal"),
            ("characters", "Test Sage", "ascendancies"),
            ("character_constants", "base_speed", "value"),
            ("mastery_effects", "4242", "stats"),
        ]
    );
}

#[test]
fn bases_should_match_one_base_by_name() {
    // Two bundle bases with PoB's name: no single match.
    let mut bundle = bundle("bundle-poe1");
    let mut twin = bundle
        .base_items
        .iter()
        .find(|b| b.id == "Test/Items/Wand1")
        .unwrap()
        .clone();
    twin.id = "Test/Items/Wand2".to_owned();
    bundle.base_items.push(twin.clone());
    let properties = bundle.properties.as_mut().unwrap();
    let mut weapon = properties.weapon_types[0].clone();
    weapon.base_item = twin.id.clone();
    properties.weapon_types.push(weapon.clone());
    let export = pob::parse(&serde_json::to_vec(&equal_export()).unwrap()).unwrap();
    let found = compare::compare(&bundle, &export, SCOPE, &BOSSES)
        .unwrap()
        .differences;
    assert_eq!(fields(&found), [("bases", "Plain Test Wand", "row")]);
    // A Royale base is skipped, as PoB skips it (bases.lua:342).
    let royale = "Test/Items/Wand2Royale";
    bundle.base_items.last_mut().unwrap().id = royale.to_owned();
    bundle
        .properties
        .as_mut()
        .unwrap()
        .weapon_types
        .last_mut()
        .unwrap()
        .base_item = royale.to_owned();
    assert_eq!(
        compare::compare(&bundle, &export, SCOPE, &BOSSES)
            .unwrap()
            .differences,
        []
    );
}

#[test]
fn passive_text_should_render_as_the_game_shows_it() {
    // A described stat renders; a stat without a description shows no line.
    let mut bundle = bundle("bundle-poe1");
    let passive = bundle
        .passives
        .as_mut()
        .unwrap()
        .passives
        .iter_mut()
        .find(|p| p.node == 101)
        .unwrap();
    passive.stats.push(loom_data::bundle::passives::StatAmount {
        stat: "test_maximum_life".to_owned(),
        value: 10,
    });
    let mut export = equal_export();
    export["tree"]["nodes"]["101"]["stats"] = json!(["+10 to maximum Life"]);
    let export = pob::parse(&serde_json::to_vec(&export).unwrap()).unwrap();
    assert_eq!(
        compare::compare(&bundle, &export, SCOPE, &BOSSES)
            .unwrap()
            .differences,
        []
    );
}

#[test]
fn granted_passive_points_should_compare_as_a_number() {
    let mut export = equal_export();
    export["tree"]["nodes"]["101"]["grantedPassivePoints"] = json!(2);
    // The tree text's line for them is not node text.
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
    // A non-finite value is never equal to an absent one.
    assert_ne!(num(f64::NAN), Value::Null);
    assert_ne!(num(f64::INFINITY), Value::Null);
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

/// The synthetic bundle's Steam build, set by [`run_with`].
const BUILD: u64 = 1;

fn rules_json(rules: &Value) -> Vec<u8> {
    serde_json::to_vec(&json!({"format": 1, "game": "poe1", "steam_build": BUILD,
                               "pob_version": "2.67.2", "rules": rules}))
    .unwrap()
}

const THREAD: &str = "https://www.pathofexile.com/forum/view-thread/1";

/// A rule covering `count` differences; a `patch-delta` rule's change is
/// named in its notes.
fn rule(id: &str, category: &str, table: &str, field: &str, key: &str, source: &str) -> Value {
    let mut rule = json!({"id": id, "category": category, "table": table, "fields": [field],
                          "keys": [key], "count": 1, "evidence": "why", "sources": [source]});
    if category == "patch-delta" {
        rule["named_in_notes"] = json!(true);
    }
    rule
}

#[test]
fn rules_should_be_checked() {
    let ok = rule("a", "patch-delta", "skills", "*", "*", THREAD);
    assert!(Rules::parse(&rules_json(&json!([ok])), "poe1").is_ok());
    let mut without_named = ok.clone();
    without_named
        .as_object_mut()
        .unwrap()
        .remove("named_in_notes");
    let mut named_elsewhere = rule("a", "representation", "skills", "*", "*", "game:X.Y");
    named_elsewhere["named_in_notes"] = json!(false);
    let mut zero = ok.clone();
    zero["count"] = json!(0);
    let bad: [(Value, &str); 12] = [
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
            json!([rule("a", "pob-derivation", "skills", "*", "*", THREAD)]),
            "PoB v2.67.2 citation",
        ),
        // A later PoB only corroborates.
        (
            json!([rule(
                "a",
                "pob-derivation",
                "skills",
                "*",
                "*",
                "pob@abc:src/x.lua:1"
            )]),
            "PoB v2.67.2 citation",
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
        (json!([without_named]), "named_in_notes"),
        (json!([named_elsewhere]), "named_in_notes"),
        (json!([zero]), "count"),
        (
            json!([{"id": "a", "category": "representation", "table": "skills", "fields": [], "keys": ["*"],
                    "count": 1, "evidence": "why", "sources": ["game:X.Y"]}]),
            "no field",
        ),
        (
            json!([{"id": "a", "category": "representation", "table": "skills", "fields": ["*"], "keys": ["*"],
                    "count": 1, "evidence": " ", "sources": ["game:X.Y"]}]),
            "no evidence",
        ),
    ];
    for (rules, expected) in bad {
        let error = Rules::parse(&rules_json(&rules), "poe1").unwrap_err();
        assert!(error.contains(expected), "{expected}: {error}");
    }
    assert!(
        Rules::parse(&rules_json(&json!([])), "poe2")
            .unwrap_err()
            .contains("for poe1")
    );
}

/// The committed rules file reads and passes every check.
#[test]
fn committed_rules_should_read() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/phase-3/data-parity-rules-poe1.json");
    let rules = Rules::parse(&std::fs::read(path).unwrap(), "poe1").unwrap();
    assert_eq!(
        (rules.steam_build, rules.pob_version.as_str()),
        (26_530_219, "2.67.2")
    );
    assert!(!rules.rules.is_empty());
}

fn run_with(export: &Value, rules: &Value, bundle_name: &str) -> Result<super::Outputs, String> {
    let mut bundle = bundle(bundle_name);
    bundle.manifest.source.steam_build = Some(BUILD);
    let export = serde_json::to_vec(export).unwrap();
    let rules = rules_json(rules);
    run(&Request {
        bundle: &bundle,
        bundle_manifest: b"{}",
        export: &export,
        rules: &rules,
        passive_scope: SCOPE,
        bosses: &BOSSES,
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
    let rules = json!([
        rule(
            "cast",
            "patch-delta",
            "skills",
            "cast_time",
            "TestBolt",
            THREAD
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
    assert!(out.summary.pass, "{:?}", out.summary.failed_rules);
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
    assert_eq!(out.summary.rules[0].named_in_notes, Some(true));
}

#[test]
fn rules_should_fail_unless_exact() {
    let mut export = equal_export();
    export["skills"]["TestBolt"]["castTime"] = json!(0.8);
    // No rule: uncategorized.
    let out = run_with(&export, &json!([]), "bundle-poe1").unwrap();
    assert!(!out.summary.pass);
    assert_eq!(out.summary.categories[UNCATEGORIZED], 1);
    assert!(out.report.differences[0].category.is_none());
    assert!(out.overlay.rows.is_empty());

    // A rule that matches nothing.
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
    assert_eq!(out.summary.failed_rules, ["stale"]);

    // A new difference on a covered row: the count no longer holds.
    export["skills"]["TestBolt"]["levels"]["1"]["critChance"] = json!(5);
    let covered = json!([rule(
        "bolt",
        "representation",
        "skills",
        "*",
        "TestBolt",
        "game:X.Y"
    )]);
    let out = run_with(&export, &covered, "bundle-poe1").unwrap();
    assert_eq!(out.summary.categories[UNCATEGORIZED], 0);
    assert!(!out.summary.pass);
    assert_eq!(
        (out.summary.rules[0].matched, out.summary.rules[0].expected),
        (2, 1)
    );

    // A key pattern that matches nothing is stale even when the rule matched.
    let mut export = equal_export();
    export["skills"]["TestBolt"]["castTime"] = json!(0.8);
    let mut two_keys = rule(
        "bolt",
        "representation",
        "skills",
        "cast_time",
        "TestBolt",
        "game:X.Y",
    );
    two_keys["keys"] = json!(["TestBolt", "Gone"]);
    let out = run_with(&export, &json!([two_keys]), "bundle-poe1").unwrap();
    assert!(!out.summary.pass);
    assert_eq!(out.summary.rules[0].stale_keys, ["Gone"]);
}

#[test]
fn first_matching_rule_should_win() {
    let mut export = equal_export();
    export["skills"]["TestBolt"]["castTime"] = json!(0.8);
    let rules = json!([
        rule(
            "narrow",
            "patch-delta",
            "skills",
            "cast_time",
            "TestBolt",
            THREAD
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
    assert_eq!(out.summary.failed_rules, ["broad"]);
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
    export["format"] = json!(1);
    assert!(
        run_with(&export, &json!([]), "bundle-poe1")
            .unwrap_err()
            .contains("format 1")
    );
    let mut export = equal_export();
    export["extra"] = json!(1);
    assert!(run_with(&export, &json!([]), "bundle-poe1").is_err());
    // A non-empty object where a list belongs.
    let mut export = equal_export();
    export["skills"]["TestBolt"]["types"] = json!({"a": 1});
    assert!(run_with(&export, &json!([]), "bundle-poe1").is_err());
    // Rules for another PoB version.
    let mut export = equal_export();
    export["pob"]["version"] = json!("2.68.0");
    assert!(
        run_with(&export, &json!([]), "bundle-poe1")
            .unwrap_err()
            .contains("for PoB 2.67.2")
    );
}

#[test]
fn rules_should_be_for_the_bundles_build() {
    let bundle = bundle("bundle-poe1"); // no Steam build
    let export = serde_json::to_vec(&equal_export()).unwrap();
    let rules = rules_json(&json!([]));
    let error = run(&Request {
        bundle: &bundle,
        bundle_manifest: b"{}",
        export: &export,
        rules: &rules,
        passive_scope: SCOPE,
        bosses: &BOSSES,
        tool: Tool::default(),
    })
    .unwrap_err();
    assert!(error.contains("Steam build 1"), "{error}");
}

#[test]
fn bosses_should_compare_their_multipliers_and_uber_mark() {
    assert!(differences(&equal_export()).is_empty());
    let mut export = equal_export();
    export["bosses"]["Armoured"]["armourMult"] = json!(40);
    let found = differences(&export);
    assert_eq!(found.len(), 1);
    assert_eq!(
        (
            found[0].table,
            found[0].key.as_str(),
            found[0].field.as_str()
        ),
        ("bosses", "Armoured", "armour")
    );
    for (field, value) in [("evasionMult", json!(5)), ("isUber", json!(false))] {
        let mut export = equal_export();
        export["bosses"]["Armoured"][field] = value;
        assert_eq!(differences(&export).len(), 1, "{field}");
    }
    // A boss PoB lists and the engine does not, and the other way.
    let mut export = equal_export();
    export["bosses"]["Extra"] = json!({"armourMult": 0, "evasionMult": 0, "isUber": false});
    export["bosses"].as_object_mut().unwrap().remove("Evasive");
    let keys: Vec<_> = differences(&export).iter().map(|d| d.key.clone()).collect();
    assert!(keys.contains(&"Extra".to_owned()) && keys.contains(&"Evasive".to_owned()));
}

#[test]
fn pantheons_should_compare_gods_souls_lines_and_values() {
    assert!(differences(&equal_export()).is_empty());
    for (path, value, field) in [
        ("/pantheons/TestMajorGod/isMajorGod", json!(false), "major"),
        (
            "/pantheons/TestMajorGod/souls/1/name",
            json!("Other"),
            "souls",
        ),
        (
            "/pantheons/TestMajorGod/souls/1/mods/0/line",
            json!("Other line"),
            "soul.2.lines",
        ),
        (
            "/pantheons/TestMinorGod/souls/0/mods/0/value/0",
            json!(5),
            "soul.1.values",
        ),
    ] {
        let mut export = equal_export();
        *export.pointer_mut(path).unwrap() = value;
        let found = differences(&export);
        assert_eq!(
            fields(&found),
            [("pantheons", path.split('/').nth(2).unwrap(), field)],
            "{path}"
        );
    }
    // A god PoB has and the bundle lacks, and the other way.
    let mut export = equal_export();
    let minor = export["pantheons"]["TestMinorGod"].take();
    export["pantheons"]["Extra"] = minor;
    export["pantheons"]
        .as_object_mut()
        .unwrap()
        .remove("TestMinorGod");
    let found = differences(&export);
    assert_eq!(fields(&found), [("pantheons", "Extra", "row")], "PoB only");
    let export = pob::parse(&serde_json::to_vec(&export).unwrap()).unwrap();
    let comparison = compare::compare(&bundle("bundle-poe1"), &export, SCOPE, &BOSSES).unwrap();
    assert!(
        comparison
            .bundle_only
            .iter()
            .any(|b| b.table == "pantheons" && b.key == "TestMinorGod")
    );
}
