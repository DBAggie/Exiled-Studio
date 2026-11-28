//! A passive's buffs (open items, the agent list: the 3_29 ascendancy
//! auras; data schema 6, `loom_data::bundle::PASSIVE_BUFF_SCHEMA`).
//!
//! In build 26530219's `PassiveSkills`, some nodes have no stats of their
//! own: the game grants their effect as a buff (`PassiveSkillBuffs`, a
//! `BuffTemplates` row whose `BuffDefinitions` stats apply to the buff's
//! targets). Ramako, Sun's Light, Void Beacon, Augury of Penitence, and
//! Unwavering Crusade are four of them. PoB v2.67.2 reads its own 3_29 tree
//! export, whose text has the lines (`TreeData/3_29/tree.lua`), and the tree
//! packs 3_25 to 3_28 carry them as the game's display stats
//! (`local_display_nearby_enemy_*`), so a 3_29 build read from the game data
//! lost them silently until now.
//!
//! The rule ([`stats`]): a node with stats of its own keeps them, and its
//! buffs are not read (the game's display stats are what PoB's text reads;
//! the buff implements them: Overwhelm's critical strike multiplier aura,
//! Gathering Winds' Tailwind, and kin). A node without stats reads each
//! buff through [`PASSIVE_BUFFS`]: each of the buff's stats as the stat the
//! tree's text resolves to. A buff not listed, or a stat a listed buff does
//! not name, is a reason (`unmodelled-passive`, `passive-buff`), never
//! dropped.

use loom_data::bundle::passives::{PassiveBuff, StatAmount};

/// How a buff's stat is read ([`PASSIVE_BUFFS`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuffStat {
    /// As this stat, the one the tree's text resolves to.
    As(&'static str),
    /// No slice field reads it (the entry says why).
    NoSliceField,
    /// A flag the buff carries at 0: it grants nothing (PoB's text has no
    /// line for it); any other value is a reason.
    NoneAtZero,
}

/// The buffs a stat-less node reads, by `BuffDefinitions` ID (sorted), each
/// stat with how it is read. PoB's 3_29 text and parse for each
/// (`TreeData/3_29/tree.lua`, `Data/ModCache.lua`):
/// - `augry_of_penitence` (Augury of Penitence; Energy Well): "Nearby
///   Enemies take 16% increased Elemental Damage" (enemy
///   `ElementalDamageTaken` INC) is the display stat the 3_28 pack reads;
///   "Nearby Enemies deal 8% less Elemental Damage" (enemy `ElementalDamage`
///   MORE) its own stat. Neither is mapped, so both are reasons.
/// - `nearby_enemies_have_no_fire_resistance_vs_dot_while_stationary`
///   (Ramako, Sun's Light): the same stat the 3_28 pack has (P6-11hc), and
///   the "no fire resistance" flag at 0.
/// - `occultist_nearby_enemies_chaos_cold_resist_life_regen` (Void
///   Beacon): "Nearby Enemies have -20% to Cold Resistance" and "-20% to
///   Chaos Resistance" (enemy `ColdResist`, `ChaosResist` BASE, 10367) are
///   the 3_28 pack's display stats; "Nearby Enemies have 100% reduced Life
///   Regeneration rate" (enemy `LifeRegen` INC, 10371) reaches no slice
///   field.
/// - `trigger_allied_guardian_relic_on_kill` (Unwavering Crusade;
///   Elemental Valiance): the 3_28 pack's stat, the trigger of Summon
///   Elemental Relic, not mapped (a reason).
pub const PASSIVE_BUFFS: &[(&str, &[(&str, BuffStat)])] = &[
    (
        "augry_of_penitence",
        &[
            (
                "elemental_damage_taken_+%",
                BuffStat::As("local_display_nearby_enemy_elemental_damage_taken_+%"),
            ),
            (
                "inquisitor_aura_elemental_damage_+%_final",
                BuffStat::As("inquisitor_aura_elemental_damage_+%_final"),
            ),
        ],
    ),
    (
        "nearby_enemies_have_no_fire_resistance_vs_dot_while_stationary",
        &[
            (
                "nearby_enemies_fire_dot_resistance_is_%_while_you_are_stationary",
                BuffStat::As("nearby_enemies_fire_dot_resistance_is_%_while_you_are_stationary"),
            ),
            (
                "nearby_enemies_no_fire_dot_resistance_while_you_are_stationary",
                BuffStat::NoneAtZero,
            ),
        ],
    ),
    (
        "occultist_nearby_enemies_chaos_cold_resist_life_regen",
        &[
            (
                "base_chaos_damage_resistance_%",
                BuffStat::As("local_display_nearby_enemy_chaos_damage_resistance_%"),
            ),
            (
                "base_cold_damage_resistance_%",
                BuffStat::As("local_display_nearby_enemy_cold_damage_resistance_%"),
            ),
            ("life_regeneration_rate_+%", BuffStat::NoSliceField),
        ],
    ),
    (
        "trigger_allied_guardian_relic_on_kill",
        &[(
            "can_trigger_summon_elemental_relic_on_nearby_ally_kill_or_on_hit_rare_or_unique",
            BuffStat::As(
                "can_trigger_summon_elemental_relic_on_nearby_ally_kill_or_on_hit_rare_or_unique",
            ),
        )],
    ),
];

/// The stats a stat-less node's `buffs` give, as the tree's text reads
/// them ([`PASSIVE_BUFFS`]), and whether something was left unread (a
/// reason for the caller).
#[must_use]
pub fn stats(buffs: &[PassiveBuff]) -> (Vec<StatAmount>, bool) {
    let mut out = Vec::new();
    let mut unread = false;
    for buff in buffs {
        let Ok(i) = PASSIVE_BUFFS.binary_search_by(|(d, _)| (*d).cmp(buff.definition.as_str()))
        else {
            // A buff without stats changes nothing (a visual, or one that
            // the node's own stats would carry).
            unread |= !buff.stats.is_empty();
            continue;
        };
        let rows = PASSIVE_BUFFS[i].1;
        for s in &buff.stats {
            match rows
                .iter()
                .find(|(stat, _)| *stat == s.stat)
                .map(|(_, r)| *r)
            {
                Some(BuffStat::As(stat)) => out.push(StatAmount {
                    stat: stat.to_owned(),
                    value: s.value,
                }),
                Some(BuffStat::NoSliceField) => {}
                Some(BuffStat::NoneAtZero) if s.value == 0 => {}
                Some(BuffStat::NoneAtZero) | None => unread = true,
            }
        }
    }
    (out, unread)
}

/// The nodes build 26530219 gives buffs and no stats (sorted): an older
/// bundle (data schema below 6) lists none of their buffs, so each is a
/// reason there instead of a silent loss (`collect`). Light of Divinity,
/// Energy Well, Unwavering Crusade, Void Beacon, Black Thumb, Augury of
/// Penitence, Elemental Valiance, and Ramako, Sun's Light. A real-bundle test
/// checks the list against the bundle (`tests/passive_buffs.rs`).
pub const BUFF_NODES: [u32; 8] = [13763, 17443, 19641, 27096, 34913, 40059, 40299, 61355];

#[cfg(test)]
mod tests {
    use super::*;

    fn buff(definition: &str, stats: &[(&str, i64)]) -> PassiveBuff {
        PassiveBuff {
            id: String::new(),
            definition: definition.to_owned(),
            radius: 60,
            stats: stats
                .iter()
                .map(|&(stat, value)| StatAmount {
                    stat: stat.to_owned(),
                    value,
                })
                .collect(),
        }
    }

    #[test]
    fn the_buffs_should_be_sorted() {
        assert!(PASSIVE_BUFFS.windows(2).all(|w| w[0].0 < w[1].0));
    }

    /// Void Beacon as build 26530219 has it: the two resistances as the
    /// tree packs' display stats, the life regeneration dropped as no
    /// field's.
    #[test]
    fn void_beacon_should_read_as_the_display_stats() {
        let (out, unread) = stats(&[buff(
            "occultist_nearby_enemies_chaos_cold_resist_life_regen",
            &[
                ("base_chaos_damage_resistance_%", -20),
                ("life_regeneration_rate_+%", -100),
                ("base_cold_damage_resistance_%", -20),
            ],
        )]);
        assert!(!unread);
        let names: Vec<&str> = out.iter().map(|s| s.stat.as_str()).collect();
        assert_eq!(
            names,
            [
                "local_display_nearby_enemy_chaos_damage_resistance_%",
                "local_display_nearby_enemy_cold_damage_resistance_%"
            ]
        );
        assert!(out.iter().all(|s| s.value == -20));
    }

    /// An unknown buff with stats, a stat a listed buff does not name, and
    /// a flag at another value than 0 are reasons; a buff without stats is
    /// not.
    #[test]
    fn what_is_not_listed_should_be_a_reason() {
        assert!(stats(&[buff("wither_nearby_enemies_every_second", &[("x", 1)])]).1);
        assert!(!stats(&[buff("generic_buff_aura", &[])]).1);
        assert!(
            stats(&[buff(
                "occultist_nearby_enemies_chaos_cold_resist_life_regen",
                &[("base_fire_damage_resistance_%", -20)],
            )])
            .1
        );
        let ramako = "nearby_enemies_have_no_fire_resistance_vs_dot_while_stationary";
        let flag = "nearby_enemies_no_fire_dot_resistance_while_you_are_stationary";
        assert!(!stats(&[buff(ramako, &[(flag, 0)])]).1);
        assert!(stats(&[buff(ramako, &[(flag, 1)])]).1);
    }
}
