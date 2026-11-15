//! The game copy converter on the maintainer's in-game copies
//! (`docs/ui/b2-data/game-copies/`, B-2 item evidence): every line ends in
//! one outcome, PoB's reader recognizes every converted line, each
//! conversion equals its recorded PoB text, and our reading equals PoB
//! v2.67.2's own reading of each copy (`pob-oracle/`) apart from the listed
//! differences (`docs/ui/b2-data/item-evidence.md`).

#![expect(
    clippy::disallowed_methods,
    reason = "a test reads the repository's evidence files; never part of a core build"
)]
#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a test reads fixed fixture files; a failure should panic"
)]

use std::path::PathBuf;

use loom_serialization::legacy::game_copy::poe1::{Copy, CopyError, convert, is_game_copy};
use loom_serialization::legacy::pob_item::{self, LineKind, Section};

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/ui/b2-data/game-copies")
}

/// The maintainer's in-game copies (`docs/ui/b2-data/item-evidence.md`).
fn fixtures() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = std::fs::read_dir(dir())
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "txt"))
        .map(|p| {
            let name = p.file_stem().unwrap().to_string_lossy().into_owned();
            (name, std::fs::read_to_string(&p).unwrap())
        })
        .collect();
    out.sort();
    assert_eq!(out.len(), 29, "the 29 copies");
    out
}

fn items() -> Vec<(String, String, Copy)> {
    fixtures()
        .into_iter()
        .filter(|(n, _)| n != "shield-crush")
        .map(|(n, t)| {
            let c = convert(&t).unwrap_or_else(|e| panic!("{n}: {e}"));
            (n, t, c)
        })
        .collect()
}

#[test]
fn every_line_of_every_copy_should_end_in_one_outcome() {
    for (name, text, copy) in items() {
        let non_blank: Vec<(usize, &str)> = text
            .split('\n')
            .enumerate()
            .map(|(i, l)| (i + 1, l.trim()))
            .filter(|(_, l)| !l.is_empty())
            .collect();
        let accounted: Vec<(usize, &str)> = copy
            .lines
            .iter()
            .map(|l| (l.number, l.raw.as_str()))
            .collect();
        assert_eq!(accounted, non_blank, "{name}");
    }
}

#[test]
fn the_pob_reader_should_recognize_every_converted_line() {
    for (name, _, copy) in items() {
        let item = pob_item::read(&copy.text);
        let unknown: Vec<&str> = item
            .lines
            .iter()
            .filter(|l| l.kind == LineKind::Unrecognized)
            .map(|l| l.raw.as_str())
            .collect();
        assert!(unknown.is_empty(), "{name}: {unknown:?}\n{}", copy.text);
        assert!(item.rarity.is_some(), "{name}");
        assert!(item.implicits.is_some(), "{name}");
        assert!(
            pob_item::unsupported_features(&item).is_empty(),
            "{name}: {:?}",
            pob_item::unsupported_features(&item)
        );
    }
}

/// Each conversion equals its committed PoB text in `game-copies/pob/`.
/// Set `LOOM_BLESS=1` to rewrite them after a reviewed change.
#[test]
fn each_conversion_should_equal_its_recorded_pob_text() {
    let out = dir().join("pob");
    let bless = std::env::var_os("LOOM_BLESS").is_some();
    for (name, _, copy) in items() {
        let path = out.join(format!("{name}.txt"));
        if bless {
            std::fs::create_dir_all(&out).unwrap();
            std::fs::write(&path, &copy.text).unwrap();
        }
        let recorded = std::fs::read_to_string(&path).unwrap_or_default();
        assert_eq!(copy.text, recorded, "{name}");
    }
}

#[test]
fn a_gem_should_be_refused() {
    let gem = fixtures()
        .into_iter()
        .find(|(n, _)| n == "shield-crush")
        .unwrap()
        .1;
    assert_eq!(convert(&gem), Err(CopyError::NotAnItem("Gem".to_owned())));
    assert_eq!(
        convert("Item Class: Stackable Currency\nRarity: Currency\nChaos Orb\n--------\n"),
        Err(CopyError::NotAnItem("Currency".to_owned()))
    );
    assert_eq!(convert("  \n\n"), Err(CopyError::Empty));
    assert_eq!(
        convert("Some text\n--------\nmore"),
        Err(CopyError::NoRarity)
    );
}

#[test]
fn a_game_copy_should_be_told_from_pob_text() {
    for (name, text) in fixtures() {
        assert!(is_game_copy(&text), "{name}");
    }
    assert!(!is_game_copy("Rarity: RARE\nName\nBase\nImplicits: 0\n"));
}

#[test]
fn the_shield_should_keep_its_specs_and_sections() {
    let copy = items()
        .into_iter()
        .find(|(n, _, _)| n == "tempest-refuge")
        .unwrap()
        .2;
    let item = pob_item::read(&copy.text);
    assert_eq!(item.base.as_deref(), Some("Ezomyte Tower Shield"));
    assert_eq!(item.implicits, Some(1));
    let sections: Vec<(Section, &str)> = item
        .modifiers()
        .map(|m| (m.section, m.text.as_str()))
        .collect();
    assert_eq!(
        sections.first(),
        Some(&(Section::Implicit, "+(30-40) to maximum Life"))
    );
    assert!(sections.contains(&(Section::Explicit, "+257 to Armour")));
    let crafted = item
        .modifiers()
        .find(|m| m.text == "+257 to Armour")
        .unwrap();
    assert!(crafted.has("crafted") && crafted.has("prefix"));
    assert_eq!(crafted.value("tags"), Some("defences,armour"));
    // The written total stays; the block chance is computed.
    assert!(copy.text.contains("Armour: 1198\n"));
    assert!(copy.text.contains("Quality: 20\n"));
    assert!(!copy.text.contains("Chance to Block:"));
    // Sockets: the copy's requirements include socketed gems'.
    assert!(!copy.text.contains("LevelReq"));
    assert!(copy.text.contains("Shaper Item\n"));
    let refuge = &copy.headers[2];
    assert_eq!(refuge.kind, "Prefix");
    assert_eq!(refuge.name.as_deref(), Some("Unrelenting"));
    assert_eq!(refuge.tier, Some(6));
    assert!(refuge.tags.is_empty());
    let upgraded = &copy.headers[3];
    assert_eq!(upgraded.kind, "Master Crafted Prefix");
    assert_eq!(upgraded.rank, Some(3));
    assert_eq!(upgraded.lines, ["+257 to Armour"]);
}

#[test]
fn flavour_and_the_usage_hint_should_be_told_apart() {
    let all = items();
    let flask = &all.iter().find(|(n, _, _)| n == "oriath-s-end").unwrap().2;
    assert_eq!(
        flask.flavour,
        [
            "Ennui prevailed, until inspiration sparked",
            "a mote of flame, kindled by fury",
            "and all in his path were obliterated."
        ]
    );
    assert_eq!(flask.dropped().get("usage-hint"), Some(&1));
    // A flask's effect and charges are property lines; its enchant is kept.
    assert!(
        flask
            .text
            .contains("Used when Charges reach full (enchant)\n")
    );
    assert!(!flask.text.contains("Lasts"));
    // No sockets: the copy's level is the requirement, as `ParseRaw` takes it.
    assert!(flask.text.contains("LevelReq: 56\n"));
    let jewel = &all.iter().find(|(n, _, _)| n == "lethal-pride").unwrap().2;
    assert_eq!(jewel.flavour.len(), 1);
    assert!(jewel.text.contains("under Akoya(...)\n"), "{}", jewel.text);
    assert!(jewel.text.contains("Limited to: 1\n"));
    assert!(jewel.text.contains("Historic - Unscalable Value\n"));
    assert_eq!(jewel.dropped().get("reminder"), Some(&1));
}

#[test]
fn a_cluster_jewels_enchants_should_count_as_implicits_without_their_reminders() {
    let copy = items()
        .into_iter()
        .find(|(n, _, _)| n == "soul-joy")
        .unwrap()
        .2;
    let item = pob_item::read(&copy.text);
    assert_eq!(item.implicits, Some(3));
    let enchants = item
        .modifiers()
        .filter(|m| m.section == Section::Enchant)
        .count();
    assert_eq!(enchants, 3);
    assert_eq!(copy.dropped().get("reminder"), Some(&3));
}

#[test]
fn influence_fracture_and_catalyst_should_reach_the_pob_text() {
    let all = items();
    let get = |n: &str| &all.iter().find(|(name, _, _)| name == n).unwrap().2;
    let pelt = get("ghoul-pelt");
    assert!(
        pelt.text
            .contains("Searing Exarch Item\nEater of Worlds Item\n")
    );
    assert!(
        pelt.text
            .contains("{exarch}{implicit}+2% to maximum Fire Resistance\n")
    );
    assert!(pelt.text.contains("Intangibility: 42%\n"));
    let ring = get("corpse-finger");
    assert!(ring.text.contains("Fractured Item\n"));
    assert!(
        ring.text
            .contains("{fractured}{suffix}{tags:mana}46% increased Mana Regeneration Rate\n")
    );
    let amulet = get("defiance-of-destiny");
    assert!(
        amulet
            .text
            .contains("Catalyst: Fertile\nCatalystQuality: 20\n")
    );
    let shield = get("foulborn-emperor-s-vigilance");
    assert!(
        shield
            .text
            .contains("{mutated}+20% to Maximum Quality - Unscalable Value\n")
    );
}

/// What both readings of a copy must agree on.
#[derive(Debug, PartialEq, Eq)]
struct Reading {
    rarity: Option<String>,
    name: Option<String>,
    base: Option<String>,
    specs: Vec<(String, String)>,
    flags: Vec<String>,
    mods: Vec<(String, String)>,
}

const COMPARED_SPECS: [&str; 12] = [
    "Quality",
    "Item Level",
    "Sockets",
    "Radius",
    "Limited to",
    "Armour",
    "Evasion Rating",
    "Energy Shield",
    "Ward",
    "Catalyst",
    "CatalystQuality",
    "Intangibility",
];

fn reading(text: &str) -> Reading {
    use loom_domain::LineKind as K;
    let body = pob_item::to_domain(&pob_item::read(text));
    let mut specs = Vec::new();
    let mut flags = Vec::new();
    let mut mods = Vec::new();
    for line in &body.lines {
        match &line.kind {
            K::Spec { key, value, .. }
                if COMPARED_SPECS.contains(&key.as_str()) || key == "Evasion" =>
            {
                // `BuildRaw` writes `Evasion Rating` as `Evasion`.
                let key = if key == "Evasion" {
                    "Evasion Rating"
                } else {
                    key.as_str()
                };
                // PoB writes `Quality: 0` for an item without quality.
                if !(key == "Quality" && value == "0") {
                    specs.push((key.to_owned(), value.clone()));
                }
            }
            K::Modifier(m) => {
                let shown = pob_item::shown_text(m).unwrap_or_else(|| m.text.clone());
                mods.push((format!("{:?}", m.section), shown));
            }
            K::Flag | K::Influence(_) => flags.push(line.source.clone()),
            _ => {}
        }
    }
    specs.sort();
    flags.sort();
    mods.sort();
    Reading {
        rarity: body.rarity.map(|r| format!("{r:?}")),
        name: body.name,
        base: body.base,
        specs,
        flags,
        mods,
    }
}

/// PoB v2.67.2's own reading of each copy (`tools/oracle/data/pob_items.lua`,
/// recorded in `game-copies/pob-oracle/`) and ours of the converted text
/// agree, line for line, apart from the differences listed in
/// `docs/ui/b2-data/item-evidence.md` ("PoB's reading").
#[test]
fn our_reading_should_equal_pobs_reading_of_each_copy() {
    let oracle = dir().join("pob-oracle");
    // (copy, modifier lines only we read, modifier lines only PoB reads).
    let known: [(&str, &[&str], &[&str]); 4] = [
        // PoB reads the item class word of a magic or unique weapon as a
        // modifier; the game shows it as a property.
        (
            "combatant-s-iron-staff-of-absorption",
            &[],
            &["Explicit Warstaff"],
        ),
        ("widowmaker", &[], &["Explicit Rune Dagger"]),
        // PoB keeps a magic flask's recovery property as a modifier.
        (
            "continuous-divine-life-flask-of-quashing",
            &[],
            &["Explicit Recovers 2400 Life over 3.50 Seconds"],
        ),
        // A line PoB cannot parse loses its roll there (`ParseRaw` sets
        // `main.defaultItemAffixQuality`, 0.5); the game rolled 15.
        (
            "replica-dreamfeather",
            &["Implicit Melee Hits have 15% chance to Fortify"],
            &["Implicit Melee Hits have 13% chance to Fortify"],
        ),
    ];
    let only = |a: &Reading, b: &Reading| -> Vec<String> {
        a.mods
            .iter()
            .filter(|m| !b.mods.contains(m))
            .map(|(s, t)| format!("{s} {t}"))
            .collect()
    };
    let mut differences = Vec::new();
    for (name, _, copy) in items() {
        let pob = std::fs::read_to_string(oracle.join(format!("{name}.txt"))).unwrap();
        let (mut ours, mut theirs) = (reading(&copy.text), reading(&pob));
        let (only_ours, only_pob) = (only(&ours, &theirs), only(&theirs, &ours));
        let (only_ours, only_pob): (Vec<&str>, Vec<&str>) = (
            only_ours.iter().map(String::as_str).collect(),
            only_pob.iter().map(String::as_str).collect(),
        );
        let expected = known
            .iter()
            .find(|(n, _, _)| *n == name)
            .map_or((&[][..], &[][..]), |(_, o, p)| (*o, *p));
        assert_eq!(
            (only_ours.as_slice(), only_pob.as_slice()),
            expected,
            "{name}"
        );
        ours.mods.clear();
        theirs.mods.clear();
        if ours != theirs {
            differences.push(format!("{name}:\n  ours   {ours:?}\n  PoB's  {theirs:?}"));
        }
    }
    assert!(differences.is_empty(), "{}", differences.join("\n"));
}
