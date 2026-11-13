//! The enemy's exposure and the guaranteed shock (P3-8k,
//! `loom_calc::poe1::exposure`, `poe1::damage_taken`) against the real
//! bundle's stat descriptions. Maintainer-run (the bundle is not in the
//! repository): `LOOM_BUNDLE_DIR=<bundle> cargo test -p loom-calc --test
//! exposure_shock -- --ignored`.

#[expect(dead_code, reason = "only the real bundle's reader is used here")]
mod common;

use loom_calc::StatMapping;
use loom_calc::poe1::damage_taken::{SHOCK_DEPENDS_ON, SHOCK_STATS};
use loom_calc::poe1::exposure::{DEPENDS_ON, STATS};
use loom_calc::poe1::mapping::{NO_EFFECT, Poe1Mapping};
use loom_domain::StatId;

/// The English text of each row's stat in `stat_descriptions.txt`, as PoB's
/// patterns read it (module docs of each).
const TEXTS: [(&str, &str); 15] = [
    (
        "chance_to_inflict_cold_exposure_on_hit_with_cold_damage_%",
        "{0}% Chance to Inflict Cold Exposure on Hit with Cold Damage",
    ),
    (
        "cold_exposure_you_inflict_applies_extra_cold_resistance_+%",
        "Cold Exposure you inflict applies an extra {0:+d}% to Cold Resistance",
    ),
    (
        "cold_skill_chance_to_inflict_cold_exposure_%",
        "Cold Skills have a {0}% chance to apply Cold Exposure on Hit",
    ),
    (
        "cold_exposure_on_hit_magnitude",
        "Inflict Cold Exposure on Hit, applying {0}% to Cold Resistance",
    ),
    (
        "fire_exposure_on_hit_magnitude",
        "Inflict Fire Exposure on Hit, applying {0}% to Fire Resistance",
    ),
    (
        "lightning_exposure_on_hit_magnitude",
        "Inflict Lightning Exposure on Hit, applying {0}% to Lightning Resistance",
    ),
    (
        "exposure_you_inflict_applies_extra_%_to_affected_resistance",
        "Exposure you inflict applies an extra {:+d}% to the affected Resistance",
    ),
    (
        "fire_exposure_you_inflict_applies_extra_fire_resistance_+%",
        "Fire Exposure you inflict applies an extra {0:+d}% to Fire Resistance",
    ),
    (
        "fire_skill_chance_to_inflict_fire_exposure_%",
        "Fire Skills have a {0}% chance to apply Fire Exposure on Hit",
    ),
    (
        "inflict_all_exposure_on_hit",
        "Inflict Fire, Cold, and Lightning Exposure on Hit",
    ),
    (
        "lightning_skill_chance_to_inflict_lightning_exposure_%",
        "Lightning Skills have a {0}% chance to apply Lightning Exposure on Hit",
    ),
    (
        "lightning_ailment_effect_+%",
        "{0}% increased Effect of Lightning Ailments",
    ),
    (
        "non_damaging_ailment_effect_+%",
        "{0}% increased Effect of Non-Damaging Ailments",
    ),
    ("shock_effect_+%", "{0}% increased Effect of Shock"),
    (
        "shock_minimum_damage_taken_increase_%",
        "Shocks from your Hits always increase Damage taken by at least {0}%",
    ),
];

/// `NoEffect` rows whose text names exposure, shock, or the effect of
/// lightning or non-damaging ailments, each checked to reach no slice
/// field (`poe1::mapping::NO_EFFECT`'s comments).
const NO_EFFECT_NAMING: &[&str] = &[
    "all_damage_can_shock",
    "always_shock",
    "base_chance_to_shock_%",
    "base_chance_to_shock_%_from_skill",
    "cannot_be_shocked_while_at_maximum_power_charges",
    "chance_to_freeze_shock_ignite_%",
    "lightning_ailment_duration_+%",
    "mana_regeneration_rate_per_minute_%_if_inflicted_exposure_recently",
    "shocked_effect_on_self_+%",
];

#[test]
fn the_texts_should_name_the_rows() {
    let rows: Vec<&str> = STATS
        .iter()
        .map(|(s, _)| *s)
        .chain(SHOCK_STATS.iter().map(|(s, ..)| *s))
        .collect();
    let mut texts: Vec<&str> = TEXTS.iter().map(|(s, _)| *s).collect();
    let mut sorted = rows.clone();
    sorted.sort_unstable();
    texts.sort_unstable();
    assert_eq!(texts, sorted);
    for stat in NO_EFFECT_NAMING {
        assert!(NO_EFFECT.iter().any(|(s, _)| s == stat), "{stat}");
    }
}

/// Each row's text is its stat's description in the real bundle; every
/// stat left is a stat of the bundle; and every other stat whose text
/// names exposure, shock, or the effect of lightning or non-damaging
/// ailments is a row, `Unmapped` (a reason), or a listed `NoEffect` row,
/// so no such stat changes the exposure or the shock silently.
#[test]
#[ignore = "needs the real PoE1 bundle (LOOM_BUNDLE_DIR)"]
#[expect(
    clippy::disallowed_methods,
    reason = "a maintainer-run test reads the bundle named by the environment"
)]
fn real_exposure_and_shock_stats_should_be_covered() {
    let dir = std::env::var("LOOM_BUNDLE_DIR").expect("LOOM_BUNDLE_DIR");
    let data = common::bundle_at(std::path::Path::new(&dir));
    // The exposure debuff's own file words its resistance stats as
    // exposure ("Cold Exposure applies #% to Cold Resistance"); on the
    // character they are resistances.
    let texts: Vec<(&str, &str)> = data
        .stat_descriptions
        .files()
        .filter(|(name, _)| !name.contains("debuff_skill_stat_descriptions"))
        .flat_map(|(_, f)| &f.descriptions)
        .filter(|d| d.stats.len() == 1)
        .flat_map(|d| {
            d.variants
                .iter()
                .map(move |v| (d.stats[0].as_str(), v.text.as_str()))
        })
        .collect();
    for (stat, text) in TEXTS {
        assert!(
            texts.iter().any(|&(s, t)| s == stat && t == text),
            "{stat}: {text}"
        );
    }
    let missing: Vec<_> = DEPENDS_ON
        .iter()
        .chain(SHOCK_DEPENDS_ON)
        .filter(|id| !data.stats.iter().any(|s| s.id == **id))
        .collect();
    assert!(missing.is_empty(), "{missing:?}");
    let names = |t: &str| {
        let t = t.to_ascii_lowercase();
        t.contains("exposure")
            || t.contains("shock")
            || t.contains("lightning ailments")
            || t.contains("non-damaging ailments")
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
