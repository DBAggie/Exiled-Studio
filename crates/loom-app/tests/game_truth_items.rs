//! The maintainer's in-game item copies (`docs/ui/b2-data/game-copies/`,
//! B-2 item evidence) added to a build through the app on the real bundle:
//! each copy is added, its lines resolve or warn, the slot rules hold, and
//! each unique's flavour text in the copy equals the bundle's (#116's
//! `poe1/unique_flavour.json`).
//!
//! The bundle is not in the repository (ADR-000); the tests are ignored and
//! run with `LOOM_B2_BUNDLE` naming an unpacked PoE1 data schema 4 bundle
//! with unique flavour text:
//! `LOOM_B2_BUNDLE=<dir> cargo test -p loom-app --test game_truth_items --
//! --ignored --nocapture`.

#![expect(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "a test over fixed evidence files; a failure should panic"
)]

use std::path::{Path, PathBuf};

use loom_app::{Session, api};
use loom_persistence::Store;
use loom_serialization::legacy::game_copy::poe1 as game_copy;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn bundle() -> PathBuf {
    let dir = PathBuf::from(
        std::env::var("LOOM_B2_BUNDLE").expect("LOOM_B2_BUNDLE names a schema 4 bundle"),
    );
    if dir.is_absolute() {
        dir
    } else {
        root().join(dir)
    }
}

fn copy(name: &str) -> String {
    std::fs::read_to_string(root().join(format!("docs/ui/b2-data/game-copies/{name}.txt"))).unwrap()
}

fn copies() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> =
        std::fs::read_dir(root().join("docs/ui/b2-data/game-copies"))
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|e| e == "txt"))
            .map(|p| {
                let name = p.file_stem().unwrap().to_string_lossy().into_owned();
                (name, std::fs::read_to_string(&p).unwrap())
            })
            .filter(|(n, _)| n != "shield-crush")
            .collect();
    out.sort();
    out
}

/// A level 90 Marauder with one empty item set.
fn session() -> Session {
    let xml = "<PathOfBuilding>\n\
         <Build level=\"90\" targetVersion=\"3_0\" className=\"Marauder\" ascendClassName=\"None\"/>\n\
         <Tree activeSpec=\"1\"><Spec treeVersion=\"3_26\" classId=\"1\" ascendClassId=\"0\" nodes=\"\"/></Tree>\n\
         <Skills/>\n<Items activeItemSet=\"1\">\n<ItemSet id=\"1\"/>\n</Items>\n<Config/>\n\
         </PathOfBuilding>";
    let mut session = Session::new(Store::new());
    session.set_data(loom_app::GameData::load(&bundle()));
    session.import_text(xml, None).unwrap();
    session
}

fn idle() -> api::CalcStatus {
    api::CalcStatus {
        state: api::CalcState::Idle,
        version: 0,
        result_state: None,
        error: None,
    }
}

fn added(session: &Session) -> u32 {
    session.snapshot(idle()).history.undo.unwrap().item
}

#[test]
#[ignore = "needs the real bundle (LOOM_B2_BUNDLE)"]
fn every_copy_should_add_with_its_lines_resolved_or_warned() {
    let mut session = session();
    let mut report = Vec::new();
    for (name, text) in copies() {
        session
            .add_item(&text, None)
            .unwrap_or_else(|e| panic!("{name}: {}", e.message));
        let item = session.item(added(&session)).unwrap();
        let warnings: Vec<String> = item
            .lines
            .iter()
            .filter(|l| l.status == api::LineStatus::Warning)
            .map(|l| format!("{} ({})", l.text, l.code.clone().unwrap_or_default()))
            .collect();
        let modifiers = item.lines.iter().filter(|l| l.kind == "modifier").count();
        report.push(format!(
            "{name}: {modifiers} modifier lines, {} warnings {warnings:?}",
            warnings.len()
        ));
        // Every line the converter wrote is one the reader knows.
        assert!(
            item.lines.iter().all(|l| l.kind != "unrecognized"),
            "{name}: {:?}",
            item.lines
        );
    }
    println!("{}", report.join("\n"));
}

#[test]
#[ignore = "needs the real bundle (LOOM_B2_BUNDLE)"]
fn slots_should_follow_the_games_hands() {
    let mut session = session();
    session
        .add_item(&copy("tempest-refuge"), Some("Weapon 2"))
        .unwrap();
    let shield = added(&session);
    assert_eq!(
        session.item(shield).unwrap().slot.as_deref(),
        Some("Weapon 2")
    );
    // A two-handed weapon empties the off hand.
    session
        .add_item(&copy("grim-mangler-q9"), Some("Weapon 1"))
        .unwrap();
    let maul = added(&session);
    assert_eq!(
        session.item(maul).unwrap().slot.as_deref(),
        Some("Weapon 1")
    );
    assert_eq!(session.item(shield).unwrap().slot, None);
    // An off-hand item is refused while the main hand holds it.
    let refused = session
        .add_item(&copy("tempest-refuge"), Some("Weapon 2"))
        .unwrap_err();
    assert_eq!(refused.code, "APP_ADD_ITEM");
    // A ring does not go on the head; a ring goes in Ring 2.
    assert_eq!(
        session
            .add_item(&copy("corpse-finger"), Some("Helmet"))
            .unwrap_err()
            .code,
        "APP_ADD_ITEM"
    );
    session
        .add_item(&copy("corpse-finger"), Some("Ring 2"))
        .unwrap();
    assert_eq!(
        session.item(added(&session)).unwrap().slot.as_deref(),
        Some("Ring 2")
    );
    session
        .add_item(&copy("oriath-s-end"), Some("Flask 3"))
        .unwrap();
    // The equipped items are in use, so the calculation reads them.
    let build = session.snapshot(idle()).build.unwrap();
    assert!(build.items.iter().find(|i| i.id == maul).unwrap().in_use);
}

/// Each unique copy's flavour section against the card's flavour, which
/// #116 reads from the bundle by the unique's name.
#[test]
#[ignore = "needs the real bundle (LOOM_B2_BUNDLE)"]
fn each_unique_copys_flavour_should_equal_the_bundles() {
    let mut session = session();
    let mut checked = 0;
    let mut differences = Vec::new();
    for (name, text) in copies() {
        let converted = game_copy::convert(&text).unwrap();
        if !text.contains("Rarity: Unique") {
            assert!(converted.flavour.is_empty(), "{name}");
            continue;
        }
        session.add_item(&text, None).unwrap();
        let card = session.item(added(&session)).unwrap().card;
        let card_lines: Vec<String> = card
            .flavour
            .iter()
            .map(|l| l.text.trim().to_owned())
            .filter(|t| !t.is_empty())
            .collect();
        let copy_lines: Vec<String> = converted
            .flavour
            .iter()
            .map(|l| l.trim().to_owned())
            .collect();
        checked += 1;
        if card_lines != copy_lines {
            println!("{name}:\n  copy {copy_lines:?}\n  card {card_lines:?}");
            differences.push(name);
        }
    }
    println!("{checked} uniques checked");
    // The game shows "To name a thing is to give it power." on The Light of
    // Meaning; PoB v2.67.2 pairs it with `UniqueJewel181` ("Faith given
    // under false pretenses ..."), and the bundle took PoB's pair. Fixed by
    // the main workstation's next data run (`item-evidence.md`, "Flavour
    // text"); this entry then goes.
    assert_eq!(differences, ["the-light-of-meaning"]);
}

/// The rounded value of each armour piece's defence in a calculation
/// (`loom_calc::poe1::phases::piece`: a `RoundHalfUp` of the
/// `piece-unrounded` product) the piece has, in graph order; a defence
/// the base lacks is 0 and left out.
fn piece_values(session: &Session) -> Vec<f64> {
    use loom_calc::{Node, Term};
    let (state, data) = session.calc_input().unwrap();
    let result = loom_app::calculate(&state, &data, &|| false).unwrap();
    let graph = &result.graph;
    graph
        .iter()
        .filter_map(|(_, node, value)| match node {
            Node::RoundHalfUp { of, .. } => match graph.node(*of) {
                Some(Node::Product {
                    term: Term::Step("piece-unrounded"),
                    ..
                }) if value != 0.0 => Some(value),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

/// A pasted armour piece without a percentile line calculates the defences
/// its copy shows: the engine derives the base roll from the shown total
/// (E4, PoB's `Item.lua:2323-2337`), so each piece's own value is the
/// game's. Values from the copies (`item-evidence.md`, "Values").
#[test]
#[ignore = "needs the real bundle (LOOM_B2_BUNDLE)"]
fn a_pasted_armour_piece_should_calculate_the_defences_its_copy_shows() {
    let cases: [(&str, &str, &[f64]); 6] = [
        ("ghoul-pelt", "Body Armour", &[4818.0]),
        ("tempest-refuge", "Weapon 2", &[1198.0]),
        ("loath-crown", "Helmet", &[313.0, 64.0]),
        ("soul-ascension", "Gloves", &[260.0, 55.0]),
        ("foulborn-emperor-s-vigilance", "Weapon 2", &[2818.0, 598.0]),
        ("storm-keep-q9", "Body Armour", &[703.0]),
    ];
    for (name, slot, shown) in cases {
        let mut session = session();
        session.add_item(&copy(name), Some(slot)).unwrap();
        let mut got = piece_values(&session);
        let mut want = shown.to_vec();
        got.sort_by(f64::total_cmp);
        want.sort_by(f64::total_cmp);
        assert_eq!(got, want, "{name}");
    }
}

/// What a copy's property lines print: `(name, values)`, the
/// `(augmented)` marks removed; `Elemental Damage` lists one range per
/// element.
fn printed(text: &str) -> Vec<(String, Vec<String>)> {
    const NAMES: [&str; 8] = [
        "Physical Damage",
        "Elemental Damage",
        "Chaos Damage",
        "Critical Strike Chance",
        "Attacks per Second",
        "Armour",
        "Evasion Rating",
        "Energy Shield",
    ];
    text.lines()
        .filter_map(|l| {
            let (name, value) = l.trim().split_once(": ")?;
            (NAMES.contains(&name) || name == "Chance to Block").then(|| {
                let values = value
                    .split(", ")
                    .map(|v| v.replace(" (augmented)", "").trim().to_owned())
                    .collect();
                (name.to_owned(), values)
            })
        })
        .collect()
}

/// The same lines from the card's values, as the game prints them:
/// ranges whole, rate and crit to two places, block a whole percent.
fn shown(v: &api::CardValues) -> Vec<(String, Vec<String>)> {
    let range = |d: &api::CardDamage| format!("{}-{}", d.min, d.max);
    let mut out = Vec::new();
    let of = |t: &'static str| v.damage.iter().filter(move |d| d.damage_type == t);
    if let Some(d) = of("physical").next() {
        out.push(("Physical Damage".to_owned(), vec![range(d)]));
    }
    let elemental: Vec<String> = ["fire", "cold", "lightning"]
        .iter()
        .flat_map(|t| of(t).map(range))
        .collect();
    if !elemental.is_empty() {
        out.push(("Elemental Damage".to_owned(), elemental));
    }
    if let Some(d) = of("chaos").next() {
        out.push(("Chaos Damage".to_owned(), vec![range(d)]));
    }
    if let Some(c) = v.crit_chance {
        out.push((
            "Critical Strike Chance".to_owned(),
            vec![format!("{c:.2}%")],
        ));
    }
    if let Some(r) = v.attack_rate {
        out.push(("Attacks per Second".to_owned(), vec![format!("{r:.2}")]));
    }
    if let Some(b) = v.block {
        out.push(("Chance to Block".to_owned(), vec![format!("{b}%")]));
    }
    for d in &v.defences {
        let name = match d.defence.as_str() {
            "armour" => "Armour",
            "evasion" => "Evasion Rating",
            _ => "Energy Shield",
        };
        out.push((name.to_owned(), vec![format!("{}", d.value)]));
    }
    out
}

/// The card's values equal what each copy's property lines print: weapon
/// damage per type, critical strike chance, attacks per second, a
/// shield's block, and the defences (E5). The elements' order on the
/// card's side is fire, cold, lightning, as the copies print them.
#[test]
#[ignore = "needs the real bundle (LOOM_B2_BUNDLE)"]
fn each_cards_values_should_equal_what_its_copy_prints() {
    let mut session = session();
    let mut checked = 0;
    let (mut partial, mut differences) = (Vec::new(), Vec::new());
    for (name, text) in copies() {
        session.add_item(&text, None).unwrap();
        let card = session.item(added(&session)).unwrap().card;
        let mut want = printed(&text);
        let Some(values) = card.values else {
            assert!(
                want.is_empty(),
                "{name}: no values, the copy prints {want:?}"
            );
            continue;
        };
        if values.partial {
            partial.push(name.clone());
        }
        let mut got = shown(&values);
        for v in [&mut want, &mut got] {
            for (_, values) in v.iter_mut() {
                values.sort();
            }
            v.sort();
        }
        if got != want {
            println!("{name}:\n  card {got:?}\n  copy {want:?}");
            differences.push(name);
        }
        checked += 1;
    }
    println!("{checked} cards checked; partial {partial:?}");
    assert!(checked >= 15, "{checked}");
    // Replica Dreamfeather's "(5-10)% increased Attack Speed" is one of its
    // three lines the resolver finds ambiguous (`LINE_AMBIGUOUS`, local or
    // global), so the card leaves it out (1.50 for the game's 1.65) and
    // says so; Loath Crown's unresolved lines are not local ones.
    assert_eq!(partial, ["loath-crown", "replica-dreamfeather"]);
    assert_eq!(differences, ["replica-dreamfeather"]);
}
