//! Game-truth fixtures for the tooltip's Alt view headers (B-2, PR 2): the
//! maintainer's in-game screenshots (2026-11-14) of a Pig-Faced Bascinet
//! with Urchin's, Athlete's, and Thorny prefixes and of the Genius, of the
//! Lost, and of Abjuration suffixes. The screenshot shows `Prefix Modifier
//! "Urchin's" (Tier: 2)`; the other tiers are the rule's
//! (`loom_parser::import::affixes`, "Tier") and wait for the maintainer to
//! confirm them from the same screenshots. The rolled values are ours, in
//! each mod's range.
//!
//! The bundle is not in the repository (ADR-000); the tests are ignored and
//! run with `LOOM_B2_BUNDLE` naming an unpacked PoE1 data schema 4 bundle
//! whose item rules carry `affix_generations`:
//! `LOOM_B2_BUNDLE=<dir> cargo test -p loom-app --test game_truth_affixes --
//! --ignored`.

#![expect(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "a test over a fixed fixture; a failure should panic"
)]

use std::path::{Path, PathBuf};

use loom_app::Session;
use loom_persistence::Store;
use loom_serialization::dto::app::v1::{AffixKind, AffixView};

fn bundle() -> PathBuf {
    let dir = PathBuf::from(
        std::env::var("LOOM_B2_BUNDLE").expect("LOOM_B2_BUNDLE names a schema 4 bundle"),
    );
    if dir.is_absolute() {
        dir
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(dir)
    }
}

fn build(item: &str) -> String {
    format!(
        "<PathOfBuilding>\n\
         <Build level=\"90\" targetVersion=\"3_0\" className=\"Duelist\" ascendClassName=\"None\"/>\n\
         <Tree activeSpec=\"1\"><Spec treeVersion=\"3_26\" classId=\"4\" ascendClassId=\"0\" nodes=\"\"/></Tree>\n\
         <Skills/>\n\
         <Items activeItemSet=\"1\">\n<Item id=\"1\">\n{item}\n</Item>\n\
         <ItemSet id=\"1\"><Slot name=\"Helmet\" itemId=\"1\"/></ItemSet>\n</Items>\n<Config/>\n\
         </PathOfBuilding>"
    )
}

/// `(kind, name, tier, inferred, lines as text)` of each affix.
type Row = (AffixKind, String, Option<u32>, bool, Vec<String>);

fn affixes(item: &str) -> Vec<Row> {
    let mut session = Session::new(Store::new());
    session.set_data(loom_app::GameData::load(&bundle()));
    session.import_text(&build(item), None).unwrap();
    let view = session.item(1).unwrap();
    let text = |i: &u32| {
        view.lines
            .iter()
            .find(|l| l.index == *i)
            .map(|l| l.text.clone())
            .unwrap_or_default()
    };
    view.card
        .affixes
        .iter()
        .map(|a: &AffixView| {
            (
                a.kind.clone(),
                a.name.clone().unwrap_or_default(),
                a.tier,
                a.inferred,
                a.lines.iter().map(text).collect(),
            )
        })
        .collect()
}

/// The helmet as the game exports it: no mod named. The life line sums
/// Urchin's (24 to 28) and Athlete's (115 to 129).
const HELMET: &str = "Rarity: RARE\nDread Crown\nPig-Faced Bascinet\nItem Level: 84\n\
                      LevelReq: 65\nImplicits: 0\n\
                      +34 to Armour\n\
                      +148 to maximum Life\n\
                      Reflects 3 Physical Damage to Melee Attackers\n\
                      +53 to Intelligence\n\
                      +8% to Chaos Resistance\n\
                      +11% chance to Suppress Spell Damage";

/// The specs PoB writes for the same mods.
const SPECS: &str = "Prefix: {range:0.1}LocalBaseArmourAndLife2\n\
                     Prefix: {range:0.3}IncreasedLife8\n\
                     Prefix: {range:0.6}AttackerTakesDamage1\n\
                     Suffix: {range:0.5}Intelligence9\n\
                     Suffix: {range:0.6}ChaosResist1\n\
                     Suffix: {range:0}ChanceToSuppressSpells4\n\
                     Implicits: 0";

fn row(kind: AffixKind, name: &str, tier: u32, inferred: bool, lines: &[&str]) -> Row {
    (
        kind,
        name.to_owned(),
        Some(tier),
        inferred,
        lines.iter().map(|l| (*l).to_owned()).collect(),
    )
}

const ARMOUR: &str = "+34 to Armour";
const LIFE: &str = "+148 to maximum Life";

#[test]
#[ignore = "needs a real schema 4 bundle (LOOM_B2_BUNDLE)"]
fn the_named_helmet_should_head_every_mod_with_its_tier() {
    use AffixKind::{Prefix, Suffix};
    let item = HELMET.replace("Implicits: 0", SPECS);
    assert_eq!(
        affixes(&item),
        [
            // Tier 2 in the game (the screenshot): Nautilus's (62) is above
            // it; Crocodile's (78) does not spawn on helmets.
            row(Prefix, "Urchin's", 2, false, &[ARMOUR, LIFE]),
            // Fecund (64) above; Vigorous and up do not spawn on helmets.
            row(Prefix, "Athlete's", 2, false, &[LIFE]),
            // Spiny (10) above; Barbed and Jagged do not spawn on helmets.
            row(
                Prefix,
                "Thorny",
                2,
                false,
                &["Reflects 3 Physical Damage to Melee Attackers"]
            ),
            row(Suffix, "of the Genius", 2, false, &["+53 to Intelligence"]),
            // The lowest of six.
            row(
                Suffix,
                "of the Lost",
                6,
                false,
                &["+8% to Chaos Resistance"]
            ),
            // of Nullification (85) above; the "High" rows do not spawn on
            // helmets.
            row(
                Suffix,
                "of Abjuration",
                2,
                false,
                &["+11% chance to Suppress Spell Damage"]
            ),
        ]
    );
}

#[test]
#[ignore = "needs a real schema 4 bundle (LOOM_B2_BUNDLE)"]
fn the_exported_helmet_should_head_only_the_mods_its_lines_allow_one_reading_of() {
    use AffixKind::{Prefix, Suffix};
    assert_eq!(
        affixes(HELMET),
        [
            // Every reading of the armour and life lines has Urchin's.
            row(Prefix, "Urchin's", 2, true, &[ARMOUR, LIFE]),
            // No header for Athlete's: a legacy essence row whose classes
            // the data does not name (`IncreasedLifeEssenceChest1`, 120 to
            // 126) also makes the life line with Urchin's.
            row(
                Prefix,
                "Thorny",
                2,
                true,
                &["Reflects 3 Physical Damage to Melee Attackers"]
            ),
            // No header for of the Genius: the essence row
            // `IntelligenceEssence7` (51 to 58) also holds 53.
            row(Suffix, "of the Lost", 6, true, &["+8% to Chaos Resistance"]),
            row(
                Suffix,
                "of Abjuration",
                2,
                true,
                &["+11% chance to Suppress Spell Damage"]
            ),
        ]
    );
}
