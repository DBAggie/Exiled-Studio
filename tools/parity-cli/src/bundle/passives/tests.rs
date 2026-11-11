//! The character and passive converter on made-up extracts.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use super::{PassiveSummary, convert};
use crate::bundle::Counts;

fn table(name: &str, columns: &[&str], rows: &Value) -> (String, Vec<u8>) {
    let columns: Vec<Value> = columns
        .iter()
        .map(|c| json!({"name": c, "references": null}))
        .collect();
    let body = json!({"table": name, "columns": columns, "rows": rows});
    (format!("tables/{name}.json"), body.to_string().into_bytes())
}

/// An extract in PoE1's shape (`AscendancyKey`, five stat values, a class
/// template) or PoE2's (`Ascendancy`, seven stat values, none).
fn extract(poe1: bool) -> (BTreeMap<String, Vec<u8>>, Value) {
    let ascendancy_col = if poe1 { "AscendancyKey" } else { "Ascendancy" };
    let values: Vec<String> = (1..=if poe1 { 5 } else { 7 })
        .map(|n| format!("Stat{n}Value"))
        .collect();
    let mut passive_cols = vec![
        "Id",
        "PassiveSkillGraphId",
        "Name",
        "Stats",
        "IsKeystone",
        "IsNotable",
        "IsJewelSocket",
        "IsExpansion",
        "IsAnointmentOnly",
        "IsMultipleChoice",
        "IsMultipleChoiceOption",
        "SkillPointsGranted",
        "MasteryGroup",
        ascendancy_col,
        "IsAscendancyStartingNode",
        "Characters",
    ];
    passive_cols.extend(values.iter().map(String::as_str));
    let pad = |row: Vec<Value>| -> Value {
        let mut row = row;
        row.resize(passive_cols.len(), json!(0));
        Value::Array(row)
    };
    // Row 1: two stats with values; row 2: a stat past the table's end and
    // more stats than value columns (PoE1: six on five columns).
    let many: Vec<i64> = if poe1 { vec![0; 6] } else { vec![0] };
    let passives = json!([
        pad(vec![
            json!("start"),
            json!(10),
            json!("Start"),
            json!([]),
            json!(false),
            json!(false),
            json!(false),
            json!(false),
            json!(false),
            json!(false),
            json!(false),
            json!(0),
            json!(null),
            json!(null),
            json!(false),
            json!([0]),
        ]),
        pad(vec![
            json!("life"),
            json!(11),
            json!("Life"),
            json!([0, 1]),
            json!(false),
            json!(true),
            json!(false),
            json!(false),
            json!(false),
            json!(false),
            json!(false),
            json!(0),
            json!(0),
            json!(0),
            json!(true),
            json!([]),
            json!(7),
            json!(9),
        ]),
        pad(vec![
            json!("odd"),
            json!(12),
            json!("Odd"),
            json!([9]),
            json!(true),
        ]),
        pad(vec![json!("many"), json!(13), json!("Many"), json!(many),]),
    ]);
    let mut files: BTreeMap<String, Vec<u8>> = [
        table(
            "Stats",
            &["Id"],
            &json!([["s_a"], ["s_b"], ["life_per_level"]]),
        ),
        table(
            "Characters",
            &[
                "Id",
                "Name",
                "IntegerId",
                "BaseMaxLife",
                "BaseMaxMana",
                "BaseStrength",
                "BaseDexterity",
                "BaseIntelligence",
            ],
            &json!([[
                "Metadata/Characters/Str/Str",
                "Brute",
                1,
                38,
                34,
                32,
                14,
                14
            ]]),
        ),
        table(
            "Ascendancy",
            &["Id", "Name", "Character", "ClassNo"],
            &json!([["Champ", "Champion", 0, 1], ["Lost", "Lost", 5, 2]]),
        ),
        table("PassiveSkills", &passive_cols, &passives),
        table(
            "PassiveSkillMasteryGroups",
            &["Id", "MasteryEffects"],
            &json!([["LifeMastery", [0]]]),
        ),
        table(
            "PassiveSkillMasteryEffects",
            &[
                "Id",
                "HASH16",
                "Stats",
                "Stat1Value",
                "Stat2Value",
                "Stat3Value",
            ],
            &json!([["LifeMastery1", 4242, [1], 5, 0, 0]]),
        ),
        table(
            "PassiveSkillTrees",
            &["Id", "PassiveSkillGraph"],
            &json!([
                ["Atlas", "Metadata/Atlas"],
                ["Default", "Metadata/PassiveSkillGraph"]
            ]),
        ),
    ]
    .into_iter()
    .collect();
    files.insert(
        "passive_skill_graphs/metadata/passiveskillgraph.psg.json".to_owned(),
        json!({
            "groups": [{"flags": 0, "unknown1": 0, "unknown2": 0, "x": -14159.235, "y": 8413.885,
                "nodes": [{"id": 10, "orbit": 0, "orbit_index": 0,
                    "connections": [{"id": 11, "orbit": null}, {"id": 12, "orbit": -9}]}]}],
            "kind": 0, "orbit_slots": [1, 6], "roots": [10], "version": 3
        })
        .to_string()
        .into_bytes(),
    );
    let mut templates = serde_json::Map::new();
    let mut add = |path: &str, extends: &str, entries: Value| {
        let out = format!("object_templates/{path}.json");
        files.insert(
            out.clone(),
            json!({"abstract": false, "comments": 0, "extends": extends, "version": 3,
                "blocks": [{"name": "Stats", "blocks": [], "entries": entries},
                           {"name": "Pathfinding", "blocks": [],
                            "entries": [{"key": "base_speed", "quoted": false, "value": "37"}]}]})
            .to_string()
            .into_bytes(),
        );
        templates.insert(path.to_owned(), json!({"status": "ok", "output": out}));
    };
    add(
        "metadata/characters/character.ot",
        "nothing",
        json!([
            {"key": "life_per_level", "quoted": false, "value": "12"},
            {"key": "enable_weapon_sets", "quoted": false, "value": "true"},
            {"key": "s_a", "quoted": true, "value": "12"},
            {"key": "s_b", "quoted": false, "value": "1.5"},
            {"key": "s_b", "quoted": false, "value": "-3"}
        ]),
    );
    if poe1 {
        add(
            "metadata/characters/str/str.ot",
            "Metadata/Characters/Character",
            json!([]),
        );
    }
    let manifest = json!({"format": 5, "object_templates": {"files": templates}});
    (files, manifest)
}

fn run(
    files: &BTreeMap<String, Vec<u8>>,
    manifest: &Value,
) -> Result<
    (
        loom_data::bundle::passives::PassiveTables,
        PassiveSummary,
        usize,
    ),
    String,
> {
    let read = |p: &str| files.get(p).cloned().ok_or_else(|| format!("{p}: none"));
    let counts = Counts::default();
    let (t, s) = convert(&read, manifest, &counts)?;
    Ok((t, s, counts.dangling.get()))
}

#[test]
fn poe1_shape_should_convert_with_class_templates() {
    let (files, manifest) = extract(true);
    let (t, s, dangling) = run(&files, &manifest).unwrap();
    let life = &t.passives[1];
    assert_eq!(
        (
            t.characters[0].template.as_deref(),
            t.object_templates
                .iter()
                .map(|x| (
                    x.id.as_str(),
                    x.extends.as_deref(),
                    x.stats.len(),
                    x.values.len()
                ))
                .collect::<Vec<_>>(),
            t.ascendancies[0].character.as_deref(),
            t.ascendancies[1].character.as_deref(),
            t.passives[0].characters.clone(),
            (
                life.stats
                    .iter()
                    .map(|x| (x.stat.as_str(), x.value))
                    .collect::<Vec<_>>(),
                life.notable,
                life.mastery_group.as_deref(),
                life.ascendancy.as_deref(),
                life.ascendancy_start,
            ),
            t.passives[2].stats.len(),
            t.passives[3].stats.len(),
        ),
        (
            Some("metadata/characters/str/str"),
            vec![
                ("metadata/characters/character", None, 2, 3),
                (
                    "metadata/characters/str/str",
                    Some("metadata/characters/character"),
                    0,
                    0
                ),
            ],
            Some("Metadata/Characters/Str/Str"),
            None,
            vec!["Metadata/Characters/Str/Str".to_owned()],
            (
                vec![("s_a", 7), ("s_b", 9)],
                true,
                Some("LifeMastery"),
                Some("Champ"),
                true
            ),
            0,
            5,
        )
    );
    let tree = &t.tree;
    assert_eq!(
        (
            tree.graph.as_str(),
            (tree.groups[0].x, tree.groups[0].y),
            tree.groups[0].nodes[0]
                .connections
                .iter()
                .map(|c| (c.node, c.orbit))
                .collect::<Vec<_>>(),
            tree.roots.clone(),
            t.mastery_effects[0].hash,
            t.mastery_groups[0].effects.clone(),
        ),
        (
            "Metadata/PassiveSkillGraph",
            (-14159.235, 8413.885),
            vec![(11, None), (12, Some(-9))],
            vec![10],
            4242,
            vec!["LifeMastery1".to_owned()],
        )
    );
    // Dangling: the lost ascendancy's class, the odd passive's stat; dropped:
    // that stat and the sixth stat of `many`; kept as written: a non-stat
    // and a quoted value.
    assert_eq!(
        (s, dangling),
        (
            PassiveSummary {
                templates: 2,
                template_values: 3,
                dropped_stats: 2,
            },
            2
        )
    );
}

#[test]
fn poe2_shape_should_convert_without_class_templates() {
    let (files, manifest) = extract(false);
    let (t, s, _) = run(&files, &manifest).unwrap();
    assert_eq!(
        (
            t.characters[0].template.as_deref(),
            t.passives[1].ascendancy.as_deref(),
            t.passives[3].stats.len(),
            s.templates,
        ),
        (None, Some("Champ"), 1, 1)
    );
}

#[test]
fn missing_inputs_should_fail_the_conversion() {
    let (files, _) = extract(true);
    assert!(run(&files, &json!({"format": 4})).is_err(), "format 4");
    let (mut files, manifest) = extract(true);
    files.remove("passive_skill_graphs/metadata/passiveskillgraph.psg.json");
    assert!(run(&files, &manifest).is_err(), "graph file");
    let (mut files, manifest) = extract(true);
    files.insert(
        "tables/PassiveSkillTrees.json".to_owned(),
        table(
            "PassiveSkillTrees",
            &["Id", "PassiveSkillGraph"],
            &json!([["Atlas", "Metadata/Atlas"]]),
        )
        .1,
    );
    assert!(run(&files, &manifest).is_err(), "no Default tree");
    let (files, mut manifest) = extract(true);
    manifest["object_templates"]["files"]["metadata/characters/character.ot"]["status"] =
        "failed".into();
    assert!(run(&files, &manifest).is_err(), "failed template");
    // A missing integer that identifies a row or feeds the calc.
    for (name, cols, rows) in [
        (
            "Characters",
            &[
                "Id",
                "Name",
                "IntegerId",
                "BaseMaxLife",
                "BaseMaxMana",
                "BaseStrength",
                "BaseDexterity",
                "BaseIntelligence",
            ][..],
            json!([[
                "Metadata/Characters/Str/Str",
                "Brute",
                null,
                38,
                34,
                32,
                14,
                14
            ]]),
        ),
        (
            "PassiveSkillMasteryEffects",
            &[
                "Id",
                "HASH16",
                "Stats",
                "Stat1Value",
                "Stat2Value",
                "Stat3Value",
            ][..],
            json!([["LifeMastery1", null, [1], 5, 0, 0]]),
        ),
    ] {
        let (mut files, manifest) = extract(true);
        files.insert(format!("tables/{name}.json"), table(name, cols, &rows).1);
        assert!(run(&files, &manifest).is_err(), "{name}");
    }
    // A graph without one of its lists, or with a non-integer node.
    for edit in [
        |g: &mut Value| g["orbit_slots"] = Value::Null,
        |g: &mut Value| g["groups"] = Value::Null,
        |g: &mut Value| g["groups"][0]["nodes"][0]["connections"][0]["id"] = "x".into(),
    ] {
        let (mut files, manifest) = extract(true);
        let path = "passive_skill_graphs/metadata/passiveskillgraph.psg.json";
        let mut g: Value = serde_json::from_slice(&files[path]).unwrap();
        edit(&mut g);
        files.insert(path.to_owned(), g.to_string().into_bytes());
        assert!(run(&files, &manifest).is_err(), "{g}");
    }
    // A template file without `extends`.
    let (mut files, manifest) = extract(true);
    let path = "object_templates/metadata/characters/character.ot.json";
    let mut t: Value = serde_json::from_slice(&files[path]).unwrap();
    t.as_object_mut().unwrap().remove("extends");
    files.insert(path.to_owned(), t.to_string().into_bytes());
    assert!(run(&files, &manifest).is_err(), "no extends");
}
