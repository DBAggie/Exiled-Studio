//! The other gems' reservation and aura effect modifiers (P3-8l,
//! `loom_calc::poe1::skill_modifiers`) against the real bundle's stat
//! descriptions. Maintainer-run (the bundle is not in the repository):
//! `LOOM_BUNDLE_DIR=<bundle> cargo test -p loom-calc --test
//! skill_modifiers -- --ignored`.

#[expect(dead_code, reason = "only the real bundle's reader is used here")]
mod common;

use loom_calc::StatMapping;
use loom_calc::poe1::mapping::{NO_EFFECT, Poe1Mapping};
use loom_calc::poe1::skill_modifiers::STATS;
use loom_domain::StatId;

/// The English text of each row's stat in `stat_descriptions.txt`, as PoB's
/// patterns read it (`ModParser.lua` 222 to 232, 457).
const TEXTS: [(&str, &str); 7] = [
    (
        "base_life_reservation_efficiency_+%",
        "{0}% increased Life Reservation Efficiency of Skills",
    ),
    (
        "base_mana_reservation_efficiency_+%",
        "{0}% increased Mana Reservation Efficiency of Skills",
    ),
    (
        "base_reservation_+%",
        "{0}% increased Reservation of Skills",
    ),
    (
        "base_reservation_efficiency_+%",
        "{0}% increased Reservation Efficiency of Skills",
    ),
    (
        "mana_reservation_efficiency_-2%_per_1",
        "{0}% reduced Mana Reservation Efficiency of Skills",
    ),
    (
        "non_curse_aura_effect_+%",
        "{0}% increased effect of Non-Curse Auras from your Skills",
    ),
    (
        "reservation_efficiency_-2%_per_1",
        "{0}% reduced Reservation Efficiency of Skills",
    ),
];

/// `NoEffect` rows whose text names reservation or the effect of auras or
/// buffs, each checked to reach no slice field (`poe1::mapping::NO_EFFECT`'s
/// comments): the aura skills' area of effect (P3-8e), which scales no
/// buff.
const NO_EFFECT_NAMING: &[&str] = &[
    "base_aura_area_of_effect_+%",
    // The golems' buff effect (P3-8n): only golem skills read it, and no
    // golem gem is modelled.
    "golem_buff_effect_+%",
];

#[test]
fn the_texts_should_name_the_rows() {
    let mut rows: Vec<&str> = STATS.iter().map(|(s, ..)| *s).collect();
    let mut texts: Vec<&str> = TEXTS.iter().map(|(s, _)| *s).collect();
    rows.sort_unstable();
    texts.sort_unstable();
    assert_eq!(texts, rows);
    for stat in NO_EFFECT_NAMING {
        assert!(NO_EFFECT.iter().any(|(s, _)| s == stat), "{stat}");
    }
}

/// Each row's text is its stat's description in the real bundle, and every
/// other stat whose text names reservation or the effect of auras or buffs
/// is a row, `Unmapped` (a reason), or a listed `NoEffect` row, so no such
/// stat changes the other gems' reservation or buffs silently.
#[test]
#[ignore = "needs the real PoE1 bundle (LOOM_BUNDLE_DIR)"]
#[expect(
    clippy::disallowed_methods,
    reason = "a maintainer-run test reads the bundle named by the environment"
)]
fn real_reservation_and_aura_stats_should_be_covered() {
    let dir = std::env::var("LOOM_BUNDLE_DIR").expect("LOOM_BUNDLE_DIR");
    let data = common::bundle_at(std::path::Path::new(&dir));
    // Every stat of every description with its texts: a description of
    // several stats names each of them, so each is checked below.
    let texts: Vec<(&str, &str)> = data
        .stat_descriptions
        .files()
        .flat_map(|(_, f)| &f.descriptions)
        .flat_map(|d| {
            d.stats.iter().flat_map(move |s| {
                d.variants
                    .iter()
                    .map(move |v| (s.as_str(), v.text.as_str()))
            })
        })
        .collect();
    for (stat, text) in TEXTS {
        assert!(
            texts.iter().any(|&(s, t)| s == stat && t == text),
            "{stat}: {text}"
        );
    }
    let names = |t: &str| {
        let t = t.to_ascii_lowercase();
        t.contains("reserv") || (t.contains("effect") && (t.contains("aura") || t.contains("buff")))
    };
    let mut stats: Vec<&str> = texts
        .iter()
        .filter(|(_, t)| names(t))
        .map(|&(s, _)| s)
        .collect();
    stats.sort_unstable();
    stats.dedup();
    let mut wrong = Vec::new();
    for stat in stats {
        if TEXTS.iter().any(|&(s, _)| s == stat) {
            continue;
        }
        match Poe1Mapping.map(&StatId::new(stat)) {
            loom_calc::StatMap::Unmapped => {}
            loom_calc::StatMap::NoEffect if NO_EFFECT_NAMING.contains(&stat) => {}
            other => wrong.push(format!("{stat}: {other:?}")),
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}
