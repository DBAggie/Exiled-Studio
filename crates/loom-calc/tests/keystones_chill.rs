//! The keystones that change the hit and the chill's rows (P3-8m,
//! `loom_calc::poe1::keystones`, `poe1::damage_taken`) against the real
//! bundle's stat descriptions. Maintainer-run (the bundle is not in the
//! repository): `LOOM_BUNDLE_DIR=<bundle> cargo test -p loom-calc --test
//! keystones_chill -- --ignored`.

#[expect(dead_code, reason = "only the real bundle's reader is used here")]
mod common;

use loom_calc::StatMapping;
use loom_calc::poe1::damage_taken::{CHILL_DEPENDS_ON, CHILL_STATS, SHOCK_STATS};
use loom_calc::poe1::keystones::KEYSTONES;
use loom_calc::poe1::mapping::{NO_EFFECT, Poe1Mapping};
use loom_domain::StatId;

/// The English text of each keystone's stat in
/// `passive_skill_stat_descriptions.txt` (the tree's text, which PoB
/// parses line by line; the descriptions file keeps the line breaks as
/// `\n`), and of each chill row's stat in `stat_descriptions.txt`, as
/// PoB's patterns read them (module docs of each).
const TEXTS: [(&str, &str); 7] = [
    ("chill_effect_+%", "{0}% increased Effect of Chill"),
    (
        "cold_ailment_effect_+%",
        "{0}% increased Effect of Cold Ailments",
    ),
    (
        "enemies_chilled_by_hits_take_damage_increased_by_chill_effect",
        "Enemies Chilled by your Hits have Damage taken increased by Chill Effect",
    ),
    (
        "keystone_cold_purist",
        "Enemies Chilled by your Hits have Cold Damage taken increased by Chill Effect\\nEnemies in your Chilling Areas have Cold Damage taken increased by Chill Effect\\nCannot deal non-Cold Damage",
    ),
    (
        "keystone_elemental_overload",
        "Skills that have dealt a Critical Strike in the past 8 seconds deal 40% more Elemental Damage with Hits and Ailments\\nYour Critical Strikes do not deal extra Damage\\nAilments never count as being from Critical Strikes",
    ),
    (
        "non_damaging_ailment_effect_+%",
        "{0}% increased Effect of Non-Damaging Ailments",
    ),
    (
        "resolute_technique",
        "Your hits can't be Evaded\\nNever deal Critical Strikes",
    ),
];

/// `NoEffect` rows whose text names chill or the effect of cold or
/// non-damaging ailments, each checked to reach no slice field
/// (`poe1::mapping::NO_EFFECT`'s comments): a skill's secondary area (a
/// skill file words it as its chilled ground's radius; P3-8e), durations,
/// and chill on you.
const NO_EFFECT_NAMING: &[&str] = &[
    "active_skill_base_secondary_area_of_effect_radius",
    "active_skill_secondary_area_of_effect_description_mode",
    "chill_effectiveness_on_self_+%",
    "cold_ailment_duration_+%",
];

#[test]
fn the_texts_should_name_the_rows() {
    let mut rows: Vec<&str> = KEYSTONES
        .iter()
        .map(|(s, _)| *s)
        .chain(CHILL_STATS.iter().map(|(s, ..)| *s))
        .collect();
    rows.sort_unstable();
    rows.dedup();
    let mut texts: Vec<&str> = TEXTS.iter().map(|(s, _)| *s).collect();
    texts.sort_unstable();
    assert_eq!(texts, rows);
    // The one stat of both ailment tables.
    assert!(
        SHOCK_STATS
            .iter()
            .any(|(s, ..)| *s == "non_damaging_ailment_effect_+%")
    );
    for stat in NO_EFFECT_NAMING {
        assert!(NO_EFFECT.iter().any(|(s, _)| s == stat), "{stat}");
    }
}

/// Each row's text is its stat's description in the real bundle; every
/// stat left is a stat of the bundle; and every other stat whose text
/// names chill or the effect of cold or non-damaging ailments is a row,
/// `Unmapped` (a reason), or a listed `NoEffect` row, so no such stat
/// changes the chill silently.
#[test]
#[ignore = "needs the real PoE1 bundle (LOOM_BUNDLE_DIR)"]
#[expect(
    clippy::disallowed_methods,
    reason = "a maintainer-run test reads the bundle named by the environment"
)]
fn real_keystone_and_chill_stats_should_be_covered() {
    let dir = std::env::var("LOOM_BUNDLE_DIR").expect("LOOM_BUNDLE_DIR");
    let data = common::bundle_at(std::path::Path::new(&dir));
    // Every description file: a skill's own file names its stats too.
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
    assert!(!texts.is_empty(), "no description file matched");
    for (stat, text) in TEXTS {
        assert!(
            texts.iter().any(|&(s, t)| s == stat && t == text),
            "{stat}: {text}"
        );
    }
    let missing: Vec<_> = CHILL_DEPENDS_ON
        .iter()
        .filter(|id| !data.stats.iter().any(|s| s.id == **id))
        .collect();
    assert!(missing.is_empty(), "{missing:?}");
    let names = |t: &str| {
        let t = t.to_ascii_lowercase();
        t.contains("chill")
            || t.contains("cold ailments")
            || t.contains("non-damaging ailments")
            || t.contains("can't be evaded")
            || t.contains("never deal critical strikes")
            || t.contains("do not deal extra damage")
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

/// Every modelled skill typed `NonHitChill` is also `ChillingArea`
/// (`poe1::gems`' guaranteed chill: PoB's `CannotChill` check applies to
/// the first alone, which the engine leaves as a reason), and the modelled
/// skills that chill are the ones the docs name. Maintainer-run, as above.
#[test]
#[ignore = "needs the real PoE1 bundle (LOOM_BUNDLE_DIR)"]
#[expect(
    clippy::disallowed_methods,
    reason = "a maintainer-run test reads the bundle named by the environment"
)]
fn modelled_chilling_skills_should_be_chilling_areas() {
    use loom_calc::CalcData;
    use loom_calc::poe1::gems::{CHILLING_AREA, NON_HIT_CHILL, SKILLS};
    let dir = std::env::var("LOOM_BUNDLE_DIR").expect("LOOM_BUNDLE_DIR");
    let data = common::bundle_at(std::path::Path::new(&dir));
    let mut chilling = Vec::new();
    for (id, _) in SKILLS {
        let effect = data.granted_effect(id).expect(id);
        let Some(active) = effect
            .active_skill
            .as_deref()
            .and_then(|a| data.active_skill(a))
        else {
            continue;
        };
        let has = |t: &str| active.types.iter().any(|x| x == t);
        assert!(!has(NON_HIT_CHILL) || has(CHILLING_AREA), "{id}");
        if has(CHILLING_AREA) {
            chilling.push(*id);
        }
    }
    assert_eq!(chilling, ["ArcticArmour", "Frostblink"]);
}
