//! Minion modifiers (P3-8j and P6-7b3b, `loom_calc::poe1::minions`)
//! against the real bundle's stat descriptions.

#[expect(dead_code, reason = "only the real bundle's reader is used here")]
mod common;

use loom_calc::poe1::Poe1Quantity;
use loom_calc::poe1::ailment::Ailment;
use loom_calc::poe1::minions::{DEPENDS_ON, MinionValue, STATS};
use loom_calc::quantity::{Bound, DamageType, Op};

/// The English text of each row's stat, as PoB's patterns read it
/// (`ModParser.lua` 3455 to 3462 and its "minions" and "golems" words;
/// P6-7b3b's rows checked against PoB's parse of each text).
const TEXTS: [(&str, &str); 51] = [
    (
        "active_skill_minion_energy_shield_+%_final",
        "Minions have {0}% more Energy Shield",
    ),
    (
        "active_skill_minion_life_+%_final",
        "Minions have {0}% more Maximum Life",
    ),
    (
        "active_skill_minion_movement_velocity_+%_final",
        "{0}% more Minion Movement Speed",
    ),
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
        "golem_maximum_energy_shield_+%",
        "Golems have {0}% increased Maximum Energy Shield",
    ),
    (
        "golem_maximum_life_+%",
        "Golems have {0}% increased Maximum Life",
    ),
    (
        "golem_physical_damage_reduction_rating",
        "Golems have {0:+d} to Armour",
    ),
    (
        "minion_additional_spell_block_%",
        "Minions have {0:+d}% Chance to Block Spell Damage",
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
        "minion_bleed_on_hit_with_attacks_%",
        "Minions have {0}% chance to cause Bleeding with Attacks",
    ),
    (
        "minion_block_%",
        "Minions have {0:+d}% Chance to Block Attack Damage",
    ),
    (
        "minion_cast_speed_+%",
        "Minions have {0}% increased Cast Speed",
    ),
    (
        "minion_chance_to_deal_double_damage_%",
        "Minions have {0}% chance to deal Double Damage",
    ),
    (
        "minion_chance_to_freeze_shock_ignite_%",
        "Minions have {0}% chance to Freeze, Shock and Ignite",
    ),
    (
        "minion_chance_to_ignite_%",
        "Minions have {0}% chance to Ignite",
    ),
    (
        "minion_chaos_resistance_%",
        "Minions have {0:+d}% to Chaos Resistance",
    ),
    (
        "minion_cold_damage_resistance_%",
        "Minions have {0:+d}% to Cold Resistance",
    ),
    (
        "minion_cooldown_recovery_+%",
        "Minions have {0}% increased Cooldown Recovery Rate",
    ),
    (
        "minion_critical_strike_chance_+%",
        "Minions have {0}% increased Critical Strike Chance",
    ),
    (
        "minion_critical_strike_multiplier_+",
        "Minions have {0:+d}% to Critical Strike Multiplier",
    ),
    ("minion_damage_+%", "Minions deal {0}% increased Damage"),
    (
        "minion_damage_increases_and_reductions_also_affects_you",
        "Increases and Reductions to Minion Damage also affect you",
    ),
    (
        "minion_elemental_resistance_%",
        "Minions have {0:+d}% to all Elemental Resistances",
    ),
    (
        "minion_evasion_rating_+%",
        "Minions have {0}% increased Evasion Rating",
    ),
    (
        "minion_fire_damage_resistance_%",
        "Minions have {0:+d}% to Fire Resistance",
    ),
    (
        "minion_global_maximum_added_chaos_damage",
        "Minions deal {0} to {1} additional Chaos Damage",
    ),
    (
        "minion_global_maximum_added_cold_damage",
        "Minions deal {0} to {1} additional Cold Damage",
    ),
    (
        "minion_global_maximum_added_fire_damage",
        "Minions deal {0} to {1} additional Fire Damage",
    ),
    (
        "minion_global_maximum_added_lightning_damage",
        "Minions deal {0} to {1} additional Lightning Damage",
    ),
    (
        "minion_global_maximum_added_physical_damage",
        "Minions deal {0} to {1} additional Physical Damage",
    ),
    (
        "minion_global_minimum_added_chaos_damage",
        "Minions deal {0} to {1} additional Chaos Damage",
    ),
    (
        "minion_global_minimum_added_cold_damage",
        "Minions deal {0} to {1} additional Cold Damage",
    ),
    (
        "minion_global_minimum_added_fire_damage",
        "Minions deal {0} to {1} additional Fire Damage",
    ),
    (
        "minion_global_minimum_added_lightning_damage",
        "Minions deal {0} to {1} additional Lightning Damage",
    ),
    (
        "minion_global_minimum_added_physical_damage",
        "Minions deal {0} to {1} additional Physical Damage",
    ),
    (
        "minion_life_regeneration_rate_per_minute_%",
        "Minions Regenerate {0}% of Life per second",
    ),
    (
        "minion_maximum_all_elemental_resistances_%",
        "Minions have {0:+d}% to all maximum Elemental Resistances",
    ),
    (
        "minion_maximum_energy_shield_+%",
        "Minions have {0}% increased maximum Energy Shield",
    ),
    (
        "minion_maximum_life_%_to_add_as_maximum_energy_shield",
        "Minions gain {0}% of Maximum Life as Extra Maximum Energy Shield",
    ),
    (
        "minion_maximum_life_+%",
        "Minions have {0}% increased maximum Life",
    ),
    (
        "minion_movement_speed_+%",
        "Minions have {0}% increased Movement Speed",
    ),
    (
        "minion_physical_damage_reduction_rating",
        "Minions have {0:+d} to Armour",
    ),
    (
        "minions_chance_to_poison_on_hit_%",
        "Minions have {0}% chance to Poison Enemies on Hit",
    ),
    (
        "necromancer_defensive_notable_minion_maximum_life_+%_final",
        "Minions have {0}% more Maximum Life",
    ),
    (
        "necromancer_minion_damage_+%_final",
        "Minions deal {0}% more Damage",
    ),
    (
        "support_minion_damage_+%_final",
        "Minions from Supported Skills deal {0}% more Damage",
    ),
    (
        "support_minion_damage_minion_life_+%_final",
        "Minions from Supported Skills have {0}% more Maximum Life",
    ),
    (
        "support_minion_maximum_life_+%_final",
        "Minions have {0}% more maximum Life",
    ),
];

/// The kinds and ops a text names (PoB's reading of it); a description of
/// two stats (added damage's bounds) names the bound by its stat.
fn kinds_of(stat: &str, text: &str) -> Vec<(MinionValue, Op)> {
    let op = if text.starts_with("Increases and Reductions") {
        Op::Flag
    } else if text.contains("% more ") {
        Op::More
    } else if text.contains("% increased ") {
        Op::Increased
    } else {
        Op::Base
    };
    let elemental = |f: fn(DamageType) -> MinionValue| {
        [DamageType::Fire, DamageType::Cold, DamageType::Lightning]
            .into_iter()
            .map(|t| (f(t), op))
            .collect()
    };
    let added = [
        ("additional Physical Damage", DamageType::Physical),
        ("additional Fire Damage", DamageType::Fire),
        ("additional Cold Damage", DamageType::Cold),
        ("additional Lightning Damage", DamageType::Lightning),
        ("additional Chaos Damage", DamageType::Chaos),
    ]
    .into_iter()
    .find(|(words, _)| text.contains(words));
    if let Some((_, t)) = added {
        let bound = if stat.contains("minimum") {
            Bound::Min
        } else {
            Bound::Max
        };
        vec![(MinionValue::AddedDamage(t, bound), op)]
    } else if text.contains("chance to cause Bleeding") {
        vec![(MinionValue::AilmentChance(Ailment::Bleed), op)]
    } else if text.contains("chance to Ignite") || text.contains("Shock and Ignite") {
        vec![(MinionValue::AilmentChance(Ailment::Ignite), op)]
    } else if text.contains("chance to Poison") {
        vec![(MinionValue::AilmentChance(Ailment::Poison), op)]
    } else if text.contains("Double Damage") {
        vec![(MinionValue::DoubleDamageChance, op)]
    } else if text.contains("Critical Strike Multiplier") {
        vec![(MinionValue::CritMultiplier, op)]
    } else if text.contains("Critical Strike Chance") {
        vec![(MinionValue::CritChance, op)]
    } else if text.contains("Attack and Cast Speed") {
        vec![(MinionValue::AttackSpeed, op), (MinionValue::CastSpeed, op)]
    } else if text.contains("Attack Speed") {
        vec![(MinionValue::AttackSpeed, op)]
    } else if text.contains("Cast Speed") {
        vec![(MinionValue::CastSpeed, op)]
    } else if text.contains("Damage also affect")
        || text.contains("increased Damage")
        || text.contains("more Damage")
    {
        vec![(MinionValue::Damage, op)]
    } else if text.contains("Movement Speed") {
        vec![(MinionValue::MovementSpeed, op)]
    } else if text.contains("Cooldown Recovery") {
        vec![(MinionValue::CooldownRecovery, op)]
    } else if text.contains("of Life per second") {
        vec![(MinionValue::LifeRegen, op)]
    } else if text.contains("as Extra Maximum Energy Shield") {
        vec![(MinionValue::LifeAsEnergyShield, op)]
    } else if text.contains("Energy Shield") {
        vec![(MinionValue::EnergyShield, op)]
    } else if text.contains("aximum Life") {
        vec![(MinionValue::Life, op)]
    } else if text.contains("to Armour") {
        vec![(MinionValue::Armour, op)]
    } else if text.contains("Evasion Rating") {
        vec![(MinionValue::Evasion, op)]
    } else if text.contains("Block Spell Damage") {
        vec![(MinionValue::SpellBlockChance, op)]
    } else if text.contains("Block Attack Damage") {
        vec![(MinionValue::BlockChance, op)]
    } else if text.contains("all maximum Elemental Resistances") {
        elemental(MinionValue::MaximumResistance)
    } else if text.contains("all Elemental Resistances") {
        elemental(MinionValue::Resistance)
    } else if text.contains("Chaos Resistance") {
        vec![(MinionValue::Resistance(DamageType::Chaos), op)]
    } else if text.contains("Cold Resistance") {
        vec![(MinionValue::Resistance(DamageType::Cold), op)]
    } else if text.contains("Fire Resistance") {
        vec![(MinionValue::Resistance(DamageType::Fire), op)]
    } else {
        panic!("no kind for {text}")
    }
}

#[test]
fn every_row_should_follow_its_text() {
    assert_eq!(STATS.len(), TEXTS.len());
    for ((stat, row), (text_stat, text)) in STATS.iter().zip(TEXTS) {
        assert_eq!(*stat, text_stat);
        let kinds: Vec<(MinionValue, Op)> = row
            .iter()
            .map(|&(q, op, _)| match q {
                Poe1Quantity::Minion(k) => {
                    assert!(!text.starts_with("Golems"), "{stat}");
                    (k, op)
                }
                Poe1Quantity::GolemMinion(k) => {
                    assert!(text.starts_with("Golems"), "{stat}");
                    (k, op)
                }
                Poe1Quantity::MinionToYou(k, percent) => {
                    let want = if text.contains("at 150% of their value") {
                        150
                    } else {
                        100
                    };
                    assert_eq!(percent, want, "{stat}");
                    (k, op)
                }
                other => panic!("{stat}: {other:?}"),
            })
            .collect();
        assert_eq!(kinds, kinds_of(stat, text), "{stat}");
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
