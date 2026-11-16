//! The default main skill (Q-UX-1, `loom_app::main_skill`) on the public
//! community builds of `tools/oracle/fixtures/community/`: each file is
//! named after its build's main skill by the build's author (SOURCES.md),
//! a second reading of what the main skill is.
//!
//! The bundle is not in the repository (ADR-000); the test is ignored and
//! runs with `LOOM_B2_BUNDLE` naming an unpacked PoE1 bundle:
//! `LOOM_B2_BUNDLE=<dir> cargo test -p loom-app --test game_truth_main_skill
//! -- --ignored --nocapture`.

#![expect(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "a test over fixed fixtures that prints what it chose; a failure should panic"
)]

use std::path::{Path, PathBuf};

use loom_app::Session;
use loom_persistence::Store;
use loom_serialization::dto::app::v1 as api;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn bundle() -> PathBuf {
    let dir =
        PathBuf::from(std::env::var("LOOM_B2_BUNDLE").expect("LOOM_B2_BUNDLE names a bundle"));
    if dir.is_absolute() {
        dir
    } else {
        root().join(dir)
    }
}

fn idle() -> api::CalcStatus {
    api::CalcStatus {
        state: api::CalcState::Idle,
        version: 0,
        result_state: None,
        error: None,
    }
}

/// Each community build's expected main skill: the skill the file is named
/// after, or the build's own damaging pick, which rule 1 keeps (marked).
const EXPECTED: [(&str, &str); 28] = [
    // PoB's own pick: the group's `mainActiveSkill="2"`.
    ("animate-guardian-guardian", "Raise Spectre"),
    ("arakaalis-raise-spider-occultist", "Raise Spiders"),
    ("boneshatter-juggernaut", "Boneshatter of Complex Trauma"),
    ("cyclone-shockwave-slayer", "Cyclone"),
    (
        "elemental-hit-of-the-spectrum-deadeye",
        "Elemental Hit of the Spectrum",
    ),
    // PoB's own pick in the active set (set 9, group 1).
    ("essence-drain-contagion-occultist", "Vaal Blight"),
    ("ethereal-knives-mines-assassin", "Ethereal Knives"),
    // Facebreaker is the build's gloves.
    ("facebreaker-luminary", "Earthshatter"),
    ("flicker-strike-berserker", "Vaal Flicker Strike"),
    // PoB's own pick is Purity of Elements, an aura: replaced.
    ("forged-frostbearer-spectre-necromancer", "Raise Spectre"),
    ("holy-flame-totem-hierophant", "Holy Flame Totem"),
    ("ice-nova-elementalist-campaign-lab1", "Vaal Ice Nova"),
    ("ice-nova-elementalist-campaign", "Vaal Ice Nova"),
    ("ice-nova-elementalist-mapping", "Vaal Ice Nova"),
    (
        "kinetic-blast-of-clustering-necromancer",
        "Kinetic Blast of Clustering",
    ),
    ("kinetic-fusillade-ballista-hierophant", "Kinetic Fusillade"),
    ("lacerate-bleed-gladiator", "Lacerate of Haemorrhage"),
    (
        "lightning-trap-of-sparking-elementalist",
        "Lightning Trap of Sparking",
    ),
    ("penance-brand-ignite-elementalist", "Penance Brand"),
    (
        "poisonous-concoction-of-bouncing-pathfinder",
        "Poisonous Concoction of Bouncing",
    ),
    // PoB's own pick is an empty group: the first two-link slam (the
    // league start levels with slams before Righteous Fire).
    ("rf-chieftain-league-start", "Ground Slam"),
    ("rf-chieftain-life-stack", "Vaal Righteous Fire"),
    ("shock-nova-archmage-hierophant", "Shock Nova"),
    ("smite-inquisitor-act1", "Smite"),
    ("smite-inquisitor-act10", "Smite"),
    ("smite-inquisitor-act2", "Smite"),
    ("smite-inquisitor-act5", "Smite"),
    ("winter-orb-elementalist", "Winter Orb"),
];

#[test]
#[ignore = "needs a real PoE1 bundle (LOOM_B2_BUNDLE)"]
fn the_default_main_skill_should_be_the_one_each_build_is_named_after() {
    let mut session = Session::new(Store::new());
    session.set_data(loom_app::GameData::load(&bundle()));
    let dir = root().join("tools/oracle/fixtures/community");
    let mut wrong = Vec::new();
    for (name, expected) in EXPECTED {
        let text = std::fs::read_to_string(dir.join(format!("{name}.xml"))).unwrap();
        session.import_text(&text, None).unwrap();
        let build = session.snapshot(idle()).build.unwrap();
        let skill = build.main_skill.unwrap_or_default();
        println!("{name}: {skill}");
        if skill != expected {
            wrong.push(format!("{name}: {skill}, expected {expected}"));
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
    // Every community build is in the table.
    let count = std::fs::read_dir(&dir)
        .unwrap()
        .filter(|e| {
            e.as_ref()
                .unwrap()
                .path()
                .extension()
                .is_some_and(|x| x == "xml")
        })
        .count();
    assert_eq!(count, EXPECTED.len());
}

/// Q-UX-1's own case: PoB's one main socket group number, kept across a
/// skill set switch, pointed Smite's "Red Maps" set at Flesh and Stone.
#[test]
#[ignore = "needs a real PoE1 bundle (LOOM_B2_BUNDLE)"]
fn a_skill_set_switch_should_choose_the_sets_own_main_skill() {
    let mut session = Session::new(Store::new());
    session.set_data(loom_app::GameData::load(&bundle()));
    let text = std::fs::read_to_string(
        root().join("tools/oracle/fixtures/community/smite-inquisitor-act10.xml"),
    )
    .unwrap();
    session.import_text(&text, None).unwrap();
    // Red Maps and Aspirational: Smite again, in its transfigured form;
    // Early Maps has no Smite, its main skill is Static Strike's link.
    for (set, expected) in [
        (8, "Smite of Divine Judgement"),
        (9, "Smite of Divine Judgement"),
        (7, "Static Strike"),
    ] {
        session.use_skill_set(set).unwrap();
        let build = session.snapshot(idle()).build.unwrap();
        let skill = build.main_skill.unwrap_or_default();
        println!("set {set}: {skill}");
        assert_eq!(skill, expected, "set {set}");
    }
    // Undo goes back to the set and its main skill together.
    session.undo();
    let build = session.snapshot(idle()).build.unwrap();
    assert_eq!(
        build.main_skill.as_deref(),
        Some("Smite of Divine Judgement")
    );
}
