use super::*;
use crate::extract::load;

const MANIFEST: &str = r#"{"format": 3, "game": "poe1", "steam_build_id": "1",
    "stat_descriptions": {"files": {
      "metadata/statdescriptions/stat_descriptions.txt": {"kind": "descriptions",
        "status": "ok", "output": "item.json", "descriptions": 3, "english_variants": 3},
      "metadata/statdescriptions/tincture_stat_descriptions.txt": {"kind": "descriptions",
        "status": "ok", "output": "tincture.json", "descriptions": 1, "english_variants": 1},
      "metadata/statdescriptions/graft_stat_descriptions.txt": {"kind": "descriptions",
        "status": "ok", "output": "graft.json", "descriptions": 0, "english_variants": 0}},
    "handlers": {}}}"#;

fn desc(line: u32, stat: &str, text: &str) -> String {
    format!(
        r#"{{"handed":false,"line":{line},"name":null,"off_hand_stats":null,"stats":["{stat}"],"variants":[
          {{"conditions":[{{"kind":"range","max":null,"min":null}}],"flags":[],"handlers":[],"line":{line},
           "placeholders":[{{"format":"","index":0}}],"text":"{text}"}}]}}"#
    )
}

fn file(source: &str, descriptions: &[String]) -> String {
    format!(
        r#"{{"descriptions": [{}], "file_flags": [], "includes": [], "languages": {{}},
        "no_description": [], "source": "metadata/statdescriptions/{source}"}}"#,
        descriptions.join(",")
    )
}

fn read(path: &str) -> Result<Vec<u8>, String> {
    let text = match path {
        "manifest.json" => MANIFEST.to_owned(),
        "item.json" => file(
            "stat_descriptions.txt",
            &[
                desc(1, "life", "+{0} to maximum Life"),
                desc(2, "speed_a", "{0}% increased Speed"),
                desc(3, "speed_b", "{0}% increased Speed"),
            ],
        ),
        "tincture.json" => file(
            "tincture_stat_descriptions.txt",
            &[desc(1, "tincture_effect", "{0}% increased Tincture Effect")],
        ),
        "graft.json" => file("graft_stat_descriptions.txt", &[]),
        _ => return Err(format!("{path}: not found")),
    };
    Ok(text.into_bytes())
}

fn bases() -> Bases {
    Bases::from_pairs(&[
        ("Coral Ring", 1),
        ("Ring", 1),
        ("Ironwood Tincture", 34),
        ("Fleshgraft", 38),
        ("Twin Base", 1),
        ("Twin Base", 34),
    ])
}

const BUILD: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<PathOfBuilding><Items>
<Item id="1">
    Rarity: RARE
    Secret Name
    Coral Ring
    Hunter Item
    Implicits: 1
    +10 to maximum Life
    {range:0.5}+(20-30) to maximum Life
    12% increased Speed
    No such text here
    {custom}My private note
    Corrupted
    <ModRange range="0.5" id="2"/>
</Item>
<Item id="2">
    Rarity: MAGIC
    Heated Ironwood Tincture of Skill
    Implicits: 0
    10% increased Tincture Effect
</Item>
<Item id="3">
    Rarity: UNIQUE
    Uniq
    Twin Base
    Variant: A
    Variant: B
    Selected Variant: 1
    Implicits: 0
    {variant:1}+5 to maximum Life
    {variant:2}+6 to maximum Life
</Item>
<Item id="4">
    Rarity: MAGIC
    Unknown Thing
    Implicits: 0
    +5 to maximum Life
</Item>
</Items></PathOfBuilding>"#;

fn run_one(builds: &[Build]) -> (Summary, Report) {
    let extract = load(&read).unwrap();
    let (lookup, lookup_report) = TableLookup::load(&read).unwrap();
    run(
        &extract,
        &lookup,
        lookup_report,
        &bases(),
        builds,
        Tool::default(),
    )
    .unwrap()
}

#[test]
fn run_should_put_every_item_line_under_one_outcome() {
    let (s, r) = run_one(&[Build::new(BUILD.as_bytes().to_vec())]);
    assert_eq!(s.items, 4);
    let n = &s.non_unique.outcomes;
    assert_eq!(n.get("resolved"), Some(&3), "{n:?}");
    assert_eq!(n.get("ambiguous"), Some(&1));
    assert_eq!(n.get("unresolved/no-template"), Some(&1));
    assert_eq!(n.get("excluded/pob-custom"), Some(&1));
    assert_eq!(n.get("excluded/not-a-modifier"), Some(&2));
    assert_eq!(n.get("unknown-base/no-base-in-name"), Some(&1));
    assert_eq!(s.non_unique.lines, n.values().sum::<usize>());
    // The unique item: base `Twin Base` has rows in two scopes.
    assert_eq!(
        s.unique.outcomes.get("unknown-base/scope-conflict"),
        Some(&1)
    );
    assert_eq!(
        s.unique.outcomes.get("excluded/not-selected-variant"),
        Some(&1)
    );
    assert_eq!(s.items_by_scope.get("tincture_stat_descriptions"), Some(&1));
    assert_eq!(s.mod_range_elements, 1);
    // Rate: 3 resolved of 6 (ambiguous, no-template, unknown-base count).
    assert_eq!(
        (s.rate.resolved, s.rate.denominator, s.rate.excluded),
        (3, 6, 3)
    );
    assert!(!s.rate.meets_rate);
    let items: usize = r.builds.values().map(Vec::len).sum();
    assert_eq!(items, 4);
    // Ambiguous candidates are listed.
    let amb = s.ambiguous.get("12% increased Speed").unwrap();
    assert_eq!(
        amb.detail.iter().cloned().collect::<Vec<_>>(),
        ["speed_a", "speed_b"]
    );
}

#[test]
fn run_should_never_write_item_names_or_custom_text() {
    let (s, r) = run_one(&[Build::new(BUILD.as_bytes().to_vec())]);
    let json = serde_json::to_string(&s).unwrap() + &serde_json::to_string(&r).unwrap();
    for secret in [
        "Secret Name",
        "My private note",
        "Uniq",
        "Heated",
        "Unknown Thing",
    ] {
        assert!(!json.contains(secret), "{secret} written");
    }
}

#[test]
fn run_should_be_deterministic_and_record_unreadable_builds() {
    let builds = [
        Build::new(BUILD.as_bytes().to_vec()),
        Build::new(b"<not xml".to_vec()),
        Build::new(vec![0xff, 0xfe]),
    ];
    let (s1, r1) = run_one(&builds);
    let (s2, r2) = run_one(&[
        builds[2].clone_for_test(),
        builds[0].clone_for_test(),
        builds[1].clone_for_test(),
    ]);
    assert_eq!(
        serde_json::to_string(&s1).unwrap(),
        serde_json::to_string(&s2).unwrap()
    );
    assert_eq!(
        serde_json::to_string(&r1).unwrap(),
        serde_json::to_string(&r2).unwrap()
    );
    assert_eq!(s1.unreadable_builds.len(), 2);
    assert_eq!(s1.inputs.builds.len(), 3);
}

#[test]
fn bases_scope_should_follow_the_mod_domain() {
    let b = bases();
    let item = |t: &str| pob_item::read(t);
    assert_eq!(
        b.scope(&item(
            "Rarity: RARE\nN\nSuperior Fleshgraft\nImplicits: 0\n"
        )),
        Ok("graft_stat_descriptions")
    );
    assert_eq!(
        b.scope(&item("Rarity: NORMAL\nIronwood Tincture\nImplicits: 0\n")),
        Ok("tincture_stat_descriptions")
    );
    assert_eq!(
        b.scope(&item("Rarity: RARE\nN\nNope\nImplicits: 0\n")),
        Err("not-in-base-item-types".to_owned())
    );
}

impl Build {
    fn clone_for_test(&self) -> Self {
        Self {
            hash: self.hash.clone(),
            text: self.text.clone(),
        }
    }
}
