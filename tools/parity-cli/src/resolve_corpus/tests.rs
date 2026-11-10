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
        ("Two-Toned Boots", 1),
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
    run_with(builds, &Mods::default())
}

fn run_with(builds: &[Build], mods: &Mods) -> (Summary, Report) {
    let extract = load(&read).unwrap();
    let (lookup, lookup_report) = TableLookup::load(&read, &extract).unwrap();
    run(
        &extract,
        &lookup,
        lookup_report,
        &bases(),
        mods,
        builds,
        Tool::default(),
    )
    .unwrap()
}

/// `speed_a` and `life` have `ITEM` prefix rows; `speed_b` has none.
fn speed_mods() -> Mods {
    let row = |stat: &str, lo, hi| mod_filter::ModRow {
        id: format!("{stat}_row"),
        domain: 1,
        generation: 1,
        stats: vec![(stat.to_owned(), lo, hi)],
        ..mod_filter::ModRow::default()
    };
    Mods::from_rows(vec![row("speed_a", 10, 15), row("life", 20, 30)])
}

#[test]
fn run_should_resolve_an_ambiguous_line_through_the_mods_filter() {
    let (s, r) = run_with(&[Build::new(BUILD.as_bytes().to_vec())], &speed_mods());
    assert_eq!(s.non_unique.outcomes.get("ambiguous"), None);
    assert_eq!(s.filter.non_unique.get("resolved/mods"), Some(&1));
    let line = r
        .builds
        .values()
        .flatten()
        .flat_map(|i| &i.lines)
        .find(|l| l.text == "12% increased Speed")
        .unwrap();
    assert_eq!(
        (line.outcome.as_str(), line.dropped.as_slice()),
        ("resolved", ["speed_b".to_owned()].as_slice())
    );
    assert_eq!(line.backed_by.as_deref(), Some("speed_a_row"));
}

#[test]
fn run_should_count_mod_backed_over_resolved_non_unique_lines() {
    let (s, _) = run_with(&[Build::new(BUILD.as_bytes().to_vec())], &speed_mods());
    // Resolved: +10 Life (an implicit: the prefix row does not fit),
    // +(20-30) Life, 12% Speed, and the tincture line (no row).
    let m = &s.mod_backed;
    assert_eq!((m.resolved, m.backed, m.equal), (4, 2, 2));
    assert_eq!(m.not_backed.get("no-fitting-row"), Some(&1));
    assert_eq!(m.not_backed.get("no-row-with-stats"), Some(&1));
}

#[test]
fn run_should_list_the_reason_of_a_line_that_stays_ambiguous() {
    let (s, _) = run_one(&[Build::new(BUILD.as_bytes().to_vec())]);
    let amb = s.ambiguous.get("12% increased Speed").unwrap();
    assert!(amb.reasons.contains("ambiguous/no-backed-candidate"));
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
fn run_should_account_for_every_line_as_an_outcome_or_structure() {
    let (s, _) = run_one(&[Build::new(BUILD.as_bytes().to_vec())]);
    let all: usize = s.line_kinds.values().sum();
    let structure: usize = s
        .line_kinds
        .iter()
        .filter(|(k, _)| !matches!(**k, "modifier" | "flag" | "influence" | "unrecognized"))
        .map(|(_, n)| n)
        .sum();
    assert_eq!(all, structure + s.non_unique.lines + s.unique.lines);
    assert!(
        s.non_unique
            .outcomes
            .keys()
            .all(|o| !o.starts_with("internal/"))
    );
}

#[test]
fn run_should_read_the_last_text_of_an_item_with_mod_range_children() {
    // `ItemsTab.lua` calls `ParseRaw` for each string child; the last wins.
    let xml = r#"<PathOfBuilding><Items><Item id="1">
        Rarity: RARE
        Old
        Coral Ring
        Implicits: 0
        +1 to maximum Life
        <ModRange range="0.5" id="1"/>
        Rarity: RARE
        New
        Coral Ring
        Implicits: 0
        +2 to maximum Life
        +3 to maximum Life
        <ModRange range="0.5" id="2"/>
    </Item></Items></PathOfBuilding>"#;
    let (s, r) = run_one(&[Build::new(xml.as_bytes().to_vec())]);
    assert_eq!(s.items_with_several_texts, 1);
    assert_eq!(s.mod_range_elements, 2);
    assert_eq!(s.non_unique.outcomes.get("resolved"), Some(&2));
    let texts: Vec<&str> = r
        .builds
        .values()
        .flatten()
        .flat_map(|i| &i.lines)
        .map(|l| l.text.as_str())
        .collect();
    assert_eq!(texts, ["+2 to maximum Life", "+3 to maximum Life"]);
}

#[test]
fn run_should_count_a_copied_build_once() {
    let one = Build::new(BUILD.as_bytes().to_vec());
    let (s, r) = run_one(&[one.clone_for_test(), one]);
    assert_eq!(
        (s.duplicate_builds, s.items, s.inputs.builds.len()),
        (1, 4, 1)
    );
    assert_eq!(r.builds.len(), 1);
}

#[test]
fn run_should_hash_unknown_spec_and_prefix_keys() {
    let xml = r#"<PathOfBuilding><Items><Item id="1">
        Rarity: RARE
        N
        Coral Ring
        My secret: words
        Implicits: 0
        {secretword}+1 to maximum Life
    </Item></Items></PathOfBuilding>"#;
    let (s, _) = run_one(&[Build::new(xml.as_bytes().to_vec())]);
    let json = serde_json::to_string(&s).unwrap();
    assert!(!json.contains("My secret") && !json.contains("secretword"));
    assert_eq!(s.unknown_spec_keys.len(), 1);
    assert_eq!(s.unknown_prefix_keys.len(), 1);
}

#[test]
fn run_should_never_write_item_names_or_custom_text() {
    let (s, r) = run_one(&[Build::new(BUILD.as_bytes().to_vec())]);
    let json = serde_json::to_string(&s).unwrap() + &serde_json::to_string(&r).unwrap();
    // A line no template matches may be the author's words: hashed in the
    // summary (the local report keeps it for B4b).
    let summary = serde_json::to_string(&s).unwrap();
    assert!(!summary.contains("No such text here"));
    assert!(
        r.builds
            .values()
            .flatten()
            .flat_map(|i| &i.lines)
            .any(|l| l.text == "No such text here")
    );
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
        Ok(("graft_stat_descriptions", false))
    );
    assert_eq!(
        b.scope(&item("Rarity: NORMAL\nIronwood Tincture\nImplicits: 0\n")),
        Ok(("tincture_stat_descriptions", false))
    );
    assert_eq!(
        b.scope(&item("Rarity: RARE\nN\nNope\nImplicits: 0\n")),
        Err("not-in-base-item-types".to_owned())
    );
    assert_eq!(
        b.scope(&item(
            "Rarity: RARE\nN\nTwo-Toned Boots (Armour/Evasion)\nImplicits: 0\n"
        )),
        Ok(("stat_descriptions", true))
    );
    assert_eq!(
        b.scope(&item("Rarity: RARE\nN\nNope (X)\nImplicits: 0\n")),
        Err("not-in-base-item-types".to_owned())
    );
    // Only PoB's listed suffixed base names lose their suffix.
    assert_eq!(
        b.scope(&item("Rarity: RARE\nN\nCoral Ring (X)\nImplicits: 0\n")),
        Err("not-in-base-item-types".to_owned())
    );
}

#[test]
fn marks_should_name_shapes_without_judging_them() {
    assert_eq!(marks("^1Note to self"), ["color-code"]);
    assert_eq!(
        marks("Quality (Attribute Modifiers): +20%"),
        ["spec-shaped"]
    );
    assert_eq!(marks("Memory Strands: 63"), ["spec-shaped"]);
    assert_eq!(
        marks("Added Small Passive Skills grant: 12% increased Fire Damage"),
        ["small-passive-grant"]
    );
    assert!(marks("+10 to maximum Life").is_empty());
    assert!(marks("^Z").is_empty());
}

impl Build {
    fn clone_for_test(&self) -> Self {
        Self {
            hash: self.hash.clone(),
            text: self.text.clone(),
        }
    }
}

#[test]
fn run_should_filter_unique_lines_without_counting_them_as_mod_backed() {
    let xml = r#"<PathOfBuilding><Items><Item id="1">
        Rarity: UNIQUE
        Uniq
        Coral Ring
        Implicits: 0
        12% increased Speed
    </Item></Items></PathOfBuilding>"#;
    let (s, r) = run_with(&[Build::new(xml.as_bytes().to_vec())], &speed_mods());
    // `speed_a` has only a prefix row: on a unique nothing fits.
    assert_eq!(
        s.filter.unique.get("ambiguous/no-backed-candidate"),
        Some(&1)
    );
    assert_eq!(s.mod_backed.resolved, 0);
    let line = r
        .builds
        .values()
        .flatten()
        .flat_map(|i| &i.lines)
        .next()
        .unwrap();
    assert!(line.backed.is_none());
}

#[test]
fn run_should_split_mod_backed_by_how_the_line_was_resolved() {
    let (s, _) = run_with(&[Build::new(BUILD.as_bytes().to_vec())], &speed_mods());
    let m = &s.mod_backed;
    // The filter resolved `12% increased Speed`; the other three are the
    // resolver's own, of which `+(20-30) to maximum Life` is backed.
    assert_eq!((m.template_resolved, m.template_backed), (3, 1));
}
