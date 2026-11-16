//! Minion modifiers that also affect you (P3-8j, `loom_calc::poe1::minions`)
//! against the real bundle's stat descriptions.

#[expect(dead_code, reason = "only the real bundle's reader is used here")]
mod common;

use loom_calc::poe1::Poe1Quantity;
use loom_calc::poe1::minions::{DEPENDS_ON, MinionValue, STATS};

/// The English text of each row's stat, as PoB's patterns read it
/// (`ModParser.lua` 3455 to 3462 and its "minions" words).
const TEXTS: [(&str, &str); 8] = [
    (
        "additive_minion_damage_modifiers_apply_to_you_at_150%_value",
        "Increases and Reductions to Minion Damage also affect you at 150% of their value",
    ),
    (
        "additive_modifiers_to_minion_attack_speed_also_affect_you",
        "Increases and Reductions to Minion Attack Speed also affect you",
    ),
    (
        "additive_modifiers_to_minion_cast_speed_also_affect_you",
        "Increases and Reductions to Minion Cast Speed also affect you",
    ),
    (
        "minion_attack_and_cast_speed_+%",
        "Minions have {0}% increased Attack and Cast Speed",
    ),
    (
        "minion_attack_speed_+%",
        "Minions have {0}% increased Attack Speed",
    ),
    (
        "minion_cast_speed_+%",
        "Minions have {0}% increased Cast Speed",
    ),
    ("minion_damage_+%", "Minions deal {0}% increased Damage"),
    (
        "minion_damage_increases_and_reductions_also_affects_you",
        "Increases and Reductions to Minion Damage also affect you",
    ),
];

/// The kind a text names, and its percent for a redirect.
fn kind_of(text: &str) -> Vec<MinionValue> {
    if text.contains("Attack and Cast Speed") {
        vec![MinionValue::AttackSpeed, MinionValue::CastSpeed]
    } else if text.contains("Attack Speed") {
        vec![MinionValue::AttackSpeed]
    } else if text.contains("Cast Speed") {
        vec![MinionValue::CastSpeed]
    } else {
        vec![MinionValue::Damage]
    }
}

#[test]
fn every_row_should_follow_its_text() {
    for ((stat, row), (text_stat, text)) in STATS.iter().zip(TEXTS) {
        assert_eq!(*stat, text_stat);
        let kinds: Vec<MinionValue> = row
            .iter()
            .map(|(q, _)| match q {
                Poe1Quantity::Minion(k) => *k,
                Poe1Quantity::MinionToYou(k, percent) => {
                    let want = if text.contains("at 150% of their value") {
                        150
                    } else {
                        100
                    };
                    assert_eq!(*percent, want, "{stat}");
                    *k
                }
                other => panic!("{stat}: {other:?}"),
            })
            .collect();
        assert_eq!(kinds, kind_of(text), "{stat}");
        let redirect = text.starts_with("Increases and Reductions");
        assert!(
            row.iter()
                .all(|(q, _)| matches!(q, Poe1Quantity::MinionToYou(..)) == redirect),
            "{stat}"
        );
    }
}

/// Each row's text is its stat's description in the real bundle, and the
/// redirects left ([`DEPENDS_ON`]) hold the bundle's other stats that name
/// minion damage or speed affecting you. Maintainer-run (the bundle is not
/// in the repository).
#[test]
#[ignore = "needs the real PoE1 bundle (LOOM_BUNDLE_DIR)"]
#[expect(
    clippy::disallowed_methods,
    reason = "a maintainer-run test reads the bundle named by the environment"
)]
fn real_minion_stats_should_have_these_texts() {
    let dir = std::env::var("LOOM_BUNDLE_DIR").expect("LOOM_BUNDLE_DIR");
    let data = common::bundle_at(std::path::Path::new(&dir));
    let texts: Vec<(&str, &str)> = data
        .stat_descriptions
        .files()
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
    // Every description of a redirect of minion modifiers to you is a row
    // or left as a reason.
    let mut redirects: Vec<&str> = texts
        .iter()
        .filter(|(_, t)| {
            ["Minion Damage", "Minion Attack Speed", "Minion Cast Speed"]
                .iter()
                .any(|k| t.contains(k))
                && t.contains("also affect")
        })
        .map(|&(s, _)| s)
        .collect();
    redirects.sort_unstable();
    redirects.dedup();
    for stat in redirects {
        assert!(
            TEXTS.iter().any(|&(s, _)| s == stat) || DEPENDS_ON.contains(&stat),
            "{stat}"
        );
    }
}
