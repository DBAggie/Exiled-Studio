//! The enemy model (Phase 3 plan, step 11, `loom_calc::poe1::enemy`):
//! PoB v2.67.2's config rules over a synthetic bundle widened with the
//! bosses and monster levels the model reads, and (maintainer-run) the real
//! bundle against PoB's own values.

mod common;

use loom_calc::poe1::enemy::{BOSSES, BossKind, EnemyProblem, enemy, monster_armour};
use loom_data::bundle::Bundle;
use loom_data::bundle::properties::MonsterType;
use loom_domain::{
    Build, ConfigInput, ConfigSet, ConfigSetId, ConfigValue, Game, Number, Ordered, Sourced,
};

/// The synthetic bundle with 100 monster levels (evasion 10 x level) and
/// every boss PoB averages: armour 100 for the uber ones, 0 for the others;
/// evasion 50 for all.
#[expect(
    clippy::unwrap_used,
    reason = "a test helper: a failure fails the test"
)]
fn data() -> Bundle {
    let mut b = common::bundle();
    let p = b.properties.as_mut().unwrap();
    let template = p.monster_levels[0].clone();
    p.monster_levels = (1..=100)
        .map(|level| {
            let mut m = template.clone();
            m.level = level;
            m.evasion = 10 * level;
            m
        })
        .collect();
    p.monster_types = BOSSES
        .iter()
        .map(|boss| MonsterType {
            id: boss.monster_type.to_owned(),
            armour: if boss.uber { 100 } else { 0 },
            evasion: 50,
        })
        .collect();
    b
}

fn build(level: u32, inputs: &[(&str, ConfigValue)]) -> Build {
    let mut b = Build::new(Game::Poe1);
    b.character.level = Some(Sourced::new(level));
    let mut set = ConfigSet {
        id: ConfigSetId(1),
        title: None,
        inputs: Ordered::new(),
        attributes: Vec::new(),
    };
    for (name, value) in inputs {
        set.inputs.push(ConfigInput {
            name: (*name).to_owned(),
            value: value.clone(),
            attributes: Vec::new(),
        });
    }
    b.config.sets.push(set);
    b
}

fn number(n: f64) -> ConfigValue {
    ConfigValue::Number(Sourced::new(Number(n)))
}

fn text(s: &str) -> ConfigValue {
    ConfigValue::String(s.to_owned())
}

#[test]
fn the_default_enemy_should_be_a_pinnacle_boss() {
    let data = data();
    let e = enemy(&build(90, &[]), &data);
    assert_eq!(e.kind, BossKind::Pinnacle);
    assert_eq!(e.level, 84);
    assert_eq!(
        e.resistances,
        [Some(50.0), Some(50.0), Some(50.0), Some(30.0)]
    );
    // Mean armour multiplier 100 + 700 / 22; evasion 100 + 50.
    let armour = monster_armour(&data, 84).unwrap();
    let mean = 100.0 + 700.0 / 22.0;
    assert_eq!(e.armour, Some((armour * (mean / 100.0) + 0.5).floor()));
    assert_eq!(e.evasion, Some((840.0 * 1.5_f64 + 0.5).floor()));
    assert!(e.problems.is_empty(), "{:?}", e.problems);
}

#[test]
fn each_boss_kind_should_set_its_placeholders() {
    let data = data();
    // Not a boss: no resistance, the character's level (at most 85), the
    // table's armour and evasion.
    let e = enemy(&build(70, &[("enemyIsBoss", text("None"))]), &data);
    assert_eq!((e.kind, e.level), (BossKind::None, 70));
    assert_eq!(e.resistances, [None; 4]);
    assert_eq!(e.armour, monster_armour(&data, 70));
    assert_eq!(e.evasion, Some(700.0));
    assert_eq!(
        enemy(&build(95, &[("enemyIsBoss", text("None"))]), &data).level,
        85
    );
    // A standard boss: 40% elemental, 25% chaos; the table's armour and
    // evasion, unscaled.
    let e = enemy(&build(70, &[("enemyIsBoss", text("Boss"))]), &data);
    assert_eq!(
        e.resistances,
        [Some(40.0), Some(40.0), Some(40.0), Some(25.0)]
    );
    assert_eq!(e.level, 70);
    assert_eq!(e.armour, monster_armour(&data, 70));
    assert_eq!(e.evasion, Some(700.0));
    // Uber with a level input: the input (capped), the table at 85.
    let e = enemy(
        &build(
            90,
            &[("enemyIsBoss", text("Uber")), ("enemyLevel", number(90.0))],
        ),
        &data,
    );
    assert_eq!(e.level, 85);
    assert_eq!(e.evasion, Some(850.0 * 1.5));
    // A character level outside 1..100 is clamped, as PoB's.
    assert_eq!(
        enemy(&build(0, &[("enemyIsBoss", text("None"))]), &data).level,
        1
    );
    // Uber: level 85, the uber bosses' means (armour 200%, evasion 150%).
    let e = enemy(&build(10, &[("enemyIsBoss", text("Uber"))]), &data);
    assert_eq!(e.level, 85);
    assert_eq!(
        e.armour,
        Some((monster_armour(&data, 85).unwrap() * 2.0 + 0.5).floor())
    );
    assert_eq!(e.evasion, Some(850.0 * 1.5));
}

#[test]
fn inputs_should_win_over_placeholders() {
    let data = data();
    let e = enemy(
        &build(
            90,
            &[
                ("enemyLevel", number(70.0)),
                ("enemyFireResist", number(0.0)),
                ("enemyChaosResist", number(-20.0)),
                ("enemyArmour", number(1000.0)),
            ],
        ),
        &data,
    );
    // The level input; the armour and evasion placeholders read at least
    // the boss kind's level (84).
    assert_eq!(e.level, 70);
    assert_eq!(e.resistances[0], Some(0.0), "0 is a value (countAllowZero)");
    assert_eq!(e.resistances[3], Some(-20.0));
    assert_eq!(e.resistances[1], Some(50.0));
    assert_eq!(e.armour, Some(1000.0));
    assert_eq!(e.evasion, Some((840.0 * 1.5_f64 + 0.5).floor()));
    let e = enemy(&build(90, &[("enemyEvasion", number(0.0))]), &data);
    assert_eq!(e.evasion, Some(0.0), "an evasion input of 0 counts");
    assert_eq!(monster_armour(&data, 101), None, "past PoB's table");
    assert_eq!(monster_armour(&data, 0), None);
    // A level input above the cap; a level of 0 is no input.
    assert_eq!(
        enemy(&build(90, &[("enemyLevel", number(99.0))]), &data).level,
        85
    );
    assert_eq!(
        enemy(&build(90, &[("enemyLevel", number(0.0))]), &data).level,
        84
    );
}

#[test]
fn unknown_values_and_missing_data_should_be_problems() {
    let data = data();
    let e = enemy(&build(90, &[("enemyIsBoss", text("Huge"))]), &data);
    assert!(e.problems.contains(&EnemyProblem::UnknownBossKind {
        value: "Huge".to_owned()
    }));
    let e = enemy(&build(90, &[("enemyFireResist", text("x"))]), &data);
    assert!(e.problems.contains(&EnemyProblem::InputType {
        input: "enemyFireResist"
    }));
    let mut thin = data.clone();
    thin.properties.as_mut().unwrap().monster_types.clear();
    let e = enemy(&build(90, &[]), &thin);
    assert_eq!(e.armour, None);
    assert!(matches!(
        e.problems[0],
        EnemyProblem::MissingData {
            table: "monster_types",
            ..
        }
    ));
}

/// PoB v2.67.2's `data.monsterArmourTable` (`src/Data/Misc.lua`), which
/// its exporter derives from `MonsterDamageReductionImprovement`.
const POB_ARMOUR: [f64; 100] = [
    12.0, 15.0, 19.0, 23.0, 27.0, 32.0, 37.0, 43.0, 50.0, 57.0, 65.0, 74.0, 83.0, 94.0, 105.0,
    118.0, 132.0, 147.0, 164.0, 182.0, 202.0, 224.0, 248.0, 275.0, 303.0, 334.0, 368.0, 405.0,
    445.0, 489.0, 537.0, 589.0, 646.0, 707.0, 774.0, 846.0, 925.0, 1010.0, 1103.0, 1204.0, 1313.0,
    1432.0, 1560.0, 1700.0, 1850.0, 2014.0, 2191.0, 2383.0, 2591.0, 2815.0, 3059.0, 3322.0, 3607.0,
    3915.0, 4248.0, 4608.0, 4997.0, 5418.0, 5873.0, 6365.0, 6896.0, 7469.0, 8089.0, 8757.0, 9480.0,
    10259.0, 11101.0, 12009.0, 12989.0, 14047.0, 15188.0, 16419.0, 17747.0, 19178.0, 20722.0,
    22387.0, 24182.0, 26117.0, 28203.0, 30451.0, 32873.0, 35483.0, 38296.0, 41326.0, 44591.0,
    48107.0, 51894.0, 55973.0, 60365.0, 65095.0, 70188.0, 75670.0, 81573.0, 87926.0, 94765.0,
    102125.0, 110047.0, 118571.0, 127744.0, 137613.0,
];

#[test]
fn the_armour_table_should_equal_pobs_with_its_constant() {
    // The synthetic bundle's constant is the game's (670 / 100).
    let data = data();
    for (i, pob) in POB_ARMOUR.iter().enumerate() {
        let level = i64::try_from(i).unwrap() + 1;
        assert_eq!(monster_armour(&data, level), Some(*pob), "level {level}");
    }
}

/// The real PoE1 bundle's default enemy against the values PoB v2.67.2
/// wrote into the fixture builds it saved (`<Placeholder>`: enemy level
/// 84, armour 61989, evasion 10143, resistances 50/50/50/30; 71 fixtures).
/// Maintainer-run: `LOOM_BUNDLE_DIR=<bundle> cargo test -p loom-calc --test
/// enemy -- --ignored`.
#[test]
#[ignore = "needs the real PoE1 bundle (LOOM_BUNDLE_DIR)"]
#[expect(
    clippy::disallowed_methods,
    reason = "a maintainer-run test reads the bundle named by the environment"
)]
fn the_real_default_enemy_should_be_pobs() {
    let dir = std::env::var("LOOM_BUNDLE_DIR").expect("LOOM_BUNDLE_DIR");
    let data = common::bundle_at(std::path::Path::new(&dir));
    let e = enemy(&build(90, &[]), &data);
    assert!(e.problems.is_empty(), "{:?}", e.problems);
    assert_eq!(e.level, 84);
    assert_eq!(e.armour, Some(61989.0));
    assert_eq!(e.evasion, Some(10143.0));
    assert_eq!(
        e.resistances,
        [Some(50.0), Some(50.0), Some(50.0), Some(30.0)]
    );
    for (i, pob) in POB_ARMOUR.iter().enumerate() {
        let level = i64::try_from(i).unwrap() + 1;
        assert_eq!(monster_armour(&data, level), Some(*pob), "level {level}");
        // No value sits within 1e-9 of a whole number before the floor,
        // so C's `pow` (PoB) and repeated multiplication cannot floor apart.
        let base: f64 = 1.0 + 670.0 / 100.0 / 100.0;
        let raw = (10.0 + 2.0 * level as f64) * base.powi(i32::try_from(level).unwrap());
        let fraction = raw - raw.floor();
        assert!(
            fraction > 1e-9 && fraction < 1.0 - 1e-9,
            "level {level}: {raw}"
        );
    }
}
