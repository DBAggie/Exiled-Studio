//! The other gems' reservation and aura effect modifiers (P3-8l,
//! `loom_calc::poe1::skill_modifiers`) against the real bundle's stat
//! descriptions. Maintainer-run (the bundle is not in the repository):
//! `LOOM_BUNDLE_DIR=<bundle> cargo test -p loom-calc --test
//! skill_modifiers -- --ignored`.

#[expect(dead_code, reason = "only the real bundle's reader is used here")]
mod common;

use loom_calc::StatMapping;
use loom_calc::poe1::mapping::no_effect_by_rule;
use loom_calc::poe1::mapping::{NO_EFFECT, Poe1Mapping};
use loom_calc::poe1::skill_modifiers::STATS;
use loom_domain::StatId;

/// The English text of each row's stat in `stat_descriptions.txt`, as PoB's
/// patterns read it (`ModParser.lua` 222 to 232, 457).
const TEXTS: [(&str, &str); 26] = [
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
    // Close-out.
    ("curse_effect_+%", "{0}% increased Effect of your Curses"),
    (
        "golem_buff_effect_+%",
        "{0}% increased Effect of Buffs granted by your Golems",
    ),
    // P6-11at: PoB's `BuffEffect` tagged `SkillType.Herald`.
    (
        "herald_effect_on_self_+%",
        "{0}% increased Effect of Herald Buffs on you",
    ),
    // P6-11ag: PoB's `ManaReservationEfficiency` tagged `SkillType.Herald`.
    (
        "herald_skills_mana_reservation_efficiency_+%",
        "{0}% increased Mana Reservation Efficiency of Herald Skills",
    ),
    // P6-11ey: PoB's `ManaReservationEfficiency` with `KeywordFlag.Mine`.
    (
        "mana_reservation_efficiency_+%_for_skills_that_throw_mines",
        "{0}% increased Mana Reservation Efficiency of Skills that throw Mines",
    ),
    // P6-11bs: PoB's `BuffEffect` with `KeywordFlag.Warcry`, read by the
    // warcries' scale (`skill_modifiers::Totals::warcry_buff_effect`).
    ("warcry_buff_effect_+%", "{0}% increased Warcry Buff Effect"),
    (
        "base_cooldown_speed_+%",
        "{0}% increased Cooldown Recovery Rate",
    ),
    (
        "skill_effect_duration_+%",
        "{0}% increased Skill Effect Duration",
    ),
    (
        "warcry_cooldown_speed_+%",
        "{0}% increased Warcry Cooldown Recovery Rate",
    ),
    ("warcry_duration_+%", "{0}% increased Warcry Duration"),
    (
        "warcry_monster_power_+%",
        "{0}% increased total Power counted by Warcries",
    ),
    ("warcry_speed_+%", "{0}% increased Warcry Speed"),
    // P6-11k: PoB's `SkillAuraEffectOnSelf`.
    (
        "aura_effect_on_self_from_your_skills_+%",
        "Auras from your Skills have {0}% increased Effect on you",
    ),
    // P6-11df: PoB's `CurseEffect` with `SkillType.Mark`.
    (
        "mark_skills_curse_effect_+%",
        "{0}% increased Effect of your Marks",
    ),
    // P6-11gc: PoB's `BuffEffect` tagged with the golem skill's name.
    (
        "base_chaos_golem_granted_buff_effect_+%",
        "{0}% increased Effect of the Buff granted by your Chaos Golems",
    ),
    (
        "base_fire_golem_granted_buff_effect_+%",
        "{0}% increased Effect of the Buff granted by your Flame Golems",
    ),
    (
        "base_ice_golem_granted_buff_effect_+%",
        "{0}% increased Effect of the Buff granted by your Ice Golems",
    ),
    (
        "base_lightning_golem_granted_buff_effect_+%",
        "{0}% increased Effect of the Buff granted by your Lightning Golems",
    ),
    (
        "base_stone_golem_granted_buff_effect_+%",
        "{0}% increased Effect of the Buff granted by your Stone Golems",
    ),
];

/// `NoEffect` rows whose text names reservation or the effect of auras or
/// buffs, each checked to reach no slice field (`poe1::mapping::NO_EFFECT`'s
/// comments): the aura skills' area of effect (P3-8e), which scales no
/// buff.
const NO_EFFECT_NAMING: &[&str] = &[
    "base_aura_area_of_effect_+%",
    // P6-11ey: the mine supports' cost as reservation, read by the
    // support's ID (`gems::COST_AS_RESERVATION_SUPPORTS`).
    "remote_mined_by_support",
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
    // P6-11fq: each no-reservation row's text names its skill, and the
    // aura rows say "if Cast as an Aura".
    for (stat, skill, aura) in loom_calc::poe1::skill_modifiers::NO_RESERVATION_STATS {
        let suffix = if aura {
            " has no Reservation if Cast as an Aura"
        } else {
            " has no Reservation"
        };
        assert!(
            texts
                .iter()
                .any(|&(s, t)| s == stat && t == format!("{skill}{suffix}")),
            "{stat}: {skill}{suffix}"
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
            // No effect by a rule of its own (Phase 6): a skill's own stat
            // with no output, a cluster jewel's or a gem display's prefix.
            loom_calc::StatMap::NoEffect if no_effect_by_rule(stat) => {}
            // A field's reason marker (P6-10b): still a reason, on that
            // field alone (Manaweave's cost from unreserved mana).
            loom_calc::StatMap::Effects(e)
                if e.iter().all(|t| {
                    matches!(
                        t.quantity,
                        loom_calc::Quantity::Poe1(
                            loom_calc::poe1::Poe1Quantity::CostReason(_)
                                | loom_calc::poe1::Poe1Quantity::RegenReason(..)
                                | loom_calc::poe1::Poe1Quantity::MovementReason(_)
                        )
                    )
                }) => {}
            // "<Skill> has no Reservation" (P6-11fq): the flag of its row
            // in `skill_modifiers::NO_RESERVATION_STATS`.
            loom_calc::StatMap::Effects(e)
                if e.iter().all(|t| {
                    matches!(
                        t.quantity,
                        loom_calc::Quantity::Poe1(
                            loom_calc::poe1::Poe1Quantity::SkillNoReservation(_)
                        )
                    )
                }) => {}
            // Blood Magic (P6-11an): its "Skills Reserve Life instead of
            // Mana" is the flag `skill_modifiers::Totals` reads.
            loom_calc::StatMap::Effects(e)
                if e.iter().any(|t| {
                    t.quantity
                        == loom_calc::Quantity::Poe1(
                            loom_calc::poe1::Poe1Quantity::ReserveLifeInsteadOfMana,
                        )
                }) => {}
            other => wrong.push(format!("{stat}: {other:?}")),
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}
