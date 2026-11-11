//! The item property and monster converter on made-up extracts.

use std::collections::BTreeMap;

use loom_data::bundle::properties::ValueRange;
use serde_json::{Value, json};

use super::{PropertySummary, convert};
use crate::bundle::Counts;

fn table(name: &str, columns: &[&str], rows: &Value) -> (String, Vec<u8>) {
    let columns: Vec<Value> = columns
        .iter()
        .map(|c| json!({"name": c, "references": null}))
        .collect();
    let body = json!({"table": name, "columns": columns, "rows": rows});
    (format!("tables/{name}.json"), body.to_string().into_bytes())
}

/// An extract in PoE1's shape (defence ranges, `Critical`, `MinionLife`,
/// `AltLife1`) or PoE2's (one value per defence, `CritChance`, `AllyLife`,
/// `ReloadTime`).
fn extract(poe1: bool) -> BTreeMap<String, Vec<u8>> {
    let key = if poe1 {
        "BaseItemTypesKey"
    } else {
        "BaseItemType"
    };
    let weapons = if poe1 {
        table(
            "WeaponTypes",
            &[
                key,
                "Critical",
                "Speed",
                "DamageMin",
                "DamageMax",
                "RangeMax",
            ],
            // The second row's base is past the table's end.
            &json!([[0, 700, 714, 5, 9, 120], [9, 500, 800, 1, 2, 11]]),
        )
    } else {
        table(
            "WeaponTypes",
            &[
                key,
                "CritChance",
                "WeaponClass",
                "Speed",
                "DamageMin",
                "DamageMax",
                "RangeMax",
                "ReloadTime",
            ],
            &json!([[0, 700, 3, 714, 5, 9, 120, 250]]),
        )
    };
    let armours = if poe1 {
        table(
            "ArmourTypes",
            &[
                key,
                "ArmourMin",
                "ArmourMax",
                "EvasionMin",
                "EvasionMax",
                "EnergyShieldMin",
                "EnergyShieldMax",
                "IncreasedMovementSpeed",
                "WardMin",
                "WardMax",
            ],
            &json!([[1, 9, 12, 0, 0, 3, 4, -3, 0, 0]]),
        )
    } else {
        table(
            "ArmourTypes",
            &[
                key,
                "Armour",
                "Evasion",
                "EnergyShield",
                "IncreasedMovementSpeed",
                "Ward",
            ],
            &json!([[1, 9, 0, 3, -3, 0]]),
        )
    };
    let mut monster_cols = vec![
        "DisplayLevel",
        "Damage",
        "Evasion",
        "Accuracy",
        "MonsterLife",
        "Armour",
        "AilmentThreshold",
        "EvasiveEvasion",
        if poe1 { "MinionLife" } else { "AllyLife" },
        "MinionDamage",
        "AltLife2",
    ];
    let mut level = vec![
        json!("1"),
        json!(4.99),
        json!(67),
        json!(14),
        json!(22),
        json!(16),
        json!(22),
        json!(76),
        json!(15),
        json!(5.62),
        json!(13),
    ];
    if poe1 {
        monster_cols.extend(["AltLife1", "MonsterPhysConversionMulti"]);
        level.extend([json!(10), json!(0)]);
    }
    [
        table(
            "BaseItemTypes",
            &["Id"],
            &json!([["Items/Wand1"], ["Items/Shield1"]]),
        ),
        weapons,
        armours,
        table("ShieldTypes", &[key, "Block"], &json!([[1, 24]])),
        table("DefaultMonsterStats", &monster_cols, &json!([level])),
        table(
            "GameConstants",
            &["Id", "Value", "Divisor"],
            &json!([["MonsterDamageReductionImprovement", 670, 100]]),
        ),
    ]
    .into_iter()
    .collect()
}

fn run(
    files: &BTreeMap<String, Vec<u8>>,
) -> Result<
    (
        loom_data::bundle::properties::PropertyTables,
        PropertySummary,
        usize,
    ),
    String,
> {
    let read = |p: &str| files.get(p).cloned().ok_or_else(|| format!("{p}: none"));
    let counts = Counts::default();
    let (t, s) = convert(&read, &counts)?;
    Ok((t, s, counts.dangling.get()))
}

#[test]
fn poe1_shape_should_convert_with_ranges_and_poe1_columns() {
    let (t, s, dangling) = run(&extract(true)).unwrap();
    let a = &t.armour_types[0];
    let m = &t.monster_levels[0];
    assert_eq!(
        (
            t.weapon_types.len(),
            (
                t.weapon_types[0].base_item.as_str(),
                t.weapon_types[0].crit_chance,
                t.weapon_types[0].attack_time,
                t.weapon_types[0].reload_time,
            ),
            (
                a.base_item.as_str(),
                a.armour,
                a.energy_shield,
                a.movement_speed
            ),
            t.shield_types[0].block,
            (
                m.level,
                m.damage,
                m.ally_life,
                m.alt_life_1,
                m.phys_conversion_multi
            ),
            (t.game_constants[0].value, t.game_constants[0].divisor),
            s,
            dangling,
        ),
        (
            1,
            ("Items/Wand1", 700, 714, None),
            (
                "Items/Shield1",
                ValueRange { min: 9, max: 12 },
                ValueRange { min: 3, max: 4 },
                -3
            ),
            24,
            (1, 4.99, 15, Some(10), Some(0)),
            (670, 100),
            PropertySummary { dropped_rows: 1 },
            1,
        )
    );
}

#[test]
fn poe2_shape_should_convert_with_single_values() {
    let (t, s, _) = run(&extract(false)).unwrap();
    let m = &t.monster_levels[0];
    assert_eq!(
        (
            t.weapon_types[0].reload_time,
            t.armour_types[0].armour,
            t.armour_types[0].energy_shield,
            (m.ally_life, m.alt_life_1, m.phys_conversion_multi),
            s,
        ),
        (
            Some(250),
            ValueRange { min: 9, max: 9 },
            ValueRange { min: 3, max: 3 },
            (15, None, None),
            PropertySummary::default(),
        )
    );
}

#[test]
fn missing_or_unreadable_inputs_should_fail_the_conversion() {
    // A required column missing (a renamed column must not read as 0).
    for (name, cols, rows) in [
        (
            "ArmourTypes",
            &[
                "BaseItemTypesKey",
                "ArmourMin",
                "ArmourMax",
                "IncreasedMovementSpeed",
            ][..],
            json!([[1, 9, 12, -3]]),
        ),
        (
            "ShieldTypes",
            &["BaseItemTypesKey", "BlockChance"][..],
            json!([[1, 24]]),
        ),
        (
            "GameConstants",
            &["Id", "Value"][..],
            json!([["MonsterDamageReductionImprovement", 670]]),
        ),
    ] {
        let mut files = extract(true);
        files.insert(format!("tables/{name}.json"), table(name, cols, &rows).1);
        assert!(run(&files).is_err(), "{name}");
    }
    // A missing value, a level that is no integer, a non-finite float.
    for (edit, what) in [
        (json!([[0, null, 714, 5, 9, 120]]), "weapon value"),
        (json!("x"), "level"),
        (json!("NaN"), "damage"),
    ] {
        let mut files = extract(true);
        if what == "weapon value" {
            files.insert(
                "tables/WeaponTypes.json".to_owned(),
                table(
                    "WeaponTypes",
                    &[
                        "BaseItemTypesKey",
                        "Critical",
                        "Speed",
                        "DamageMin",
                        "DamageMax",
                        "RangeMax",
                    ],
                    &edit,
                )
                .1,
            );
        } else {
            let path = "tables/DefaultMonsterStats.json";
            let mut v: Value = serde_json::from_slice(&files[path]).unwrap();
            let col = usize::from(what == "damage");
            v["rows"][0][col] = edit;
            files.insert(path.to_owned(), v.to_string().into_bytes());
        }
        assert!(run(&files).is_err(), "{what}");
    }
}

/// `table`'s file with the column `column` removed.
fn without(files: &mut BTreeMap<String, Vec<u8>>, name: &str, column: &str) {
    let path = format!("tables/{name}.json");
    let mut v: Value = serde_json::from_slice(&files[&path]).unwrap();
    let at = v["columns"]
        .as_array()
        .unwrap()
        .iter()
        .position(|c| c["name"] == column)
        .unwrap();
    v["columns"].as_array_mut().unwrap().remove(at);
    for row in v["rows"].as_array_mut().unwrap() {
        row.as_array_mut().unwrap().remove(at);
    }
    files.insert(path, v.to_string().into_bytes());
}

#[test]
fn each_table_should_drop_and_count_rows_without_a_base() {
    // A key past the table's end is dropped and dangling; a null key is
    // dropped only (it names nothing).
    for (name, key, dangling) in [
        ("ArmourTypes", json!(9), 1),
        ("ShieldTypes", json!(9), 1),
        ("ShieldTypes", Value::Null, 0),
    ] {
        let mut files = extract(true);
        let path = format!("tables/{name}.json");
        let mut v: Value = serde_json::from_slice(&files[&path]).unwrap();
        v["rows"][0][0] = key.clone();
        files.insert(path, v.to_string().into_bytes());
        let (_, s, d) = run(&files).unwrap();
        // The weapon table's own dangling row is always there.
        assert_eq!((s.dropped_rows, d), (2, 1 + dangling), "{name} {key}");
    }
}

#[test]
fn missing_columns_of_each_table_should_fail_the_conversion() {
    for (poe1, name, column) in [
        (true, "WeaponTypes", "Speed"),
        (true, "WeaponTypes", "Critical"),
        (false, "WeaponTypes", "CritChance"),
        (true, "DefaultMonsterStats", "MinionLife"),
        (false, "DefaultMonsterStats", "AllyLife"),
        (true, "DefaultMonsterStats", "AltLife2"),
        (true, "DefaultMonsterStats", "MinionDamage"),
        (true, "GameConstants", "Divisor"),
    ] {
        let mut files = extract(poe1);
        without(&mut files, name, column);
        assert!(run(&files).is_err(), "{name}.{column}");
    }
    // A partial PoE1 defence set names both sets in its message.
    let mut files = extract(true);
    without(&mut files, "ArmourTypes", "WardMax");
    let err = run(&files).unwrap_err();
    assert!(err.contains("WardMax") && err.contains("\"Ward\""), "{err}");
    // A constant without an ID.
    let mut files = extract(true);
    files.insert(
        "tables/GameConstants.json".to_owned(),
        table(
            "GameConstants",
            &["Id", "Value", "Divisor"],
            &json!([[null, 670, 100]]),
        )
        .1,
    );
    assert!(run(&files).is_err(), "constant without ID");
}
