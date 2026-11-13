//! The gems other than the main skill's (Phase 3 plan, step 12; P3-12h):
//! what each enabled skill of the active skill set does to the character.
//!
//! What PoB v2.67.2 does (every loop over `env.player.activeSkillList` in
//! `Modules/Calc*.lua` was read for this):
//! - **which skills exist** (`CalcSetup.lua` 1631 to 1740): each socket
//!   group of the active set that is the main group, or is enabled with
//!   its slot in the weapon set in use (`slotEnabled`, 1504); each enabled
//!   gem's effects in order; an effect with a global effect only when the
//!   gem's `enableGlobal<n>` is on (1652); supports of the group applied as
//!   for the main skill ([`skill::group_skills`]);
//! - **reservation** (`CalcPerform.lua` 1928 to 2040): a skill whose types
//!   have `HasReservation` and not `ReservationBecomesCost` reserves its
//!   level's flat and percent mana or life, times the supports' multiplier
//!   ([`support_multiplier`]); per skill the flat amount is
//!   `round(floor(flat x mult) x (100 + inc) / 100 x more / (1 + eff / 100)
//!   / effMore, 0)` and the percent `round(percent x mult x ..., 2)`, both
//!   at least 0; `doActorLifeManaReservation` (526 to 560) reserves
//!   `flat + ceil(max x percent / 100)` of each pool (the unreserved phase);
//! - **buffs** (`CalcActiveSkill.lua` 794 to 859, `CalcPerform.lua` 2256 to
//!   2480): the skill's and its supports' mods with a `GlobalEffect` tag
//!   form buffs, named by the tag's `effectName` or the active skill's
//!   name; each buff is merged across skills by name, keeping each mod's
//!   largest value (`mergeBuff`), scaled by buff or aura effect
//!   (`ScaleAddList` adds the mods unchanged at a scale of 1), and applied
//!   to the character unless its `effectCond` fails or it is a totem's;
//! - **by type or name**: brands, minions, totems' life, warcries and their
//!   exerted attacks, pacts, triggers, mirages, curses and debuffs (the
//!   enemy), and every skill's `<Ailment>Max` (`CalcPerform.lua` 3534).
//!
//! What the engine does: a skill in [`SKILLS`] is modelled, any other is
//! `unmodelled-gem`. A skill there is checked against PoB's data (a
//! maintainer-run test reads PoB's export, format 4): its `GlobalEffect`
//! stats are exactly the table's, each slice row has PoB's mod name, type,
//! and flags without a `div` or `mult`, and its other mods name nothing
//! that scales buffs or reservation or sets an ailment's maximum.
//! - [`Kind::Inert`]: no global effect, no reservation, no minion, brand,
//!   warcry, or trigger type; nothing reaches the character.
//! - [`Kind::Buff`]: its reservation, and its buff rows ([`BuffStat`]).
//!
//! A support applied to such a skill must be in [`SUPPORTS`] (no global
//! effect but the rows it lists, nothing that scales buffs or
//! reservation); it adds its cost multiplier to the skill's reservation.
//! Any other support applied to it is `unmodelled-gem`; a support that
//! applies to no active skill of its group does nothing (as in PoB).
//!
//! Buff and aura effect, reservation modifiers and efficiency, gem quality
//! modifiers, and gem levels that depend on the socket are stats the
//! mapping does not know ([`DEPENDS_ON`] must stay unmapped: a test), so
//! they are reasons, and the effect of a buff here is its stat's value
//! unscaled. Global gem levels ("+1 to Level of all Cold Skill Gems") are
//! in each gem's level ([`super::gem_levels`], P3-8i), so a buff's stats
//! and a skill's reservation are those of its raised level, as in PoB.
//! A gem's quality from a socket of its colour is in its instance's
//! quality ([`sockets`], P3-12i); a gem of
//! a buff or of the main skill whose match is not known is
//! `unmodelled-skill` `socket-colour-quality` (an inert skill's quality
//! reaches nothing).

use std::collections::BTreeMap;

use loom_domain::Build;

use super::gem_levels::GemLevels;
use super::mapping::HitEffect;
use super::skill::{self, GemEffectRef, GroupSkill, MainSkill};
use super::sockets;
use super::{Poe1Quantity, Pool, ReservationKind};
use crate::condition::Flags;
use crate::ingest::Ingest;
use crate::input::{CalcInput, Resource};
use crate::quantity::{Bound, DamageType, Op, Quantity};
use crate::source::Source;
use crate::state::UnsupportedReason;
use crate::store::{Conditions, Effect};

/// What a modelled skill does to the character.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    /// Nothing (module docs).
    Inert,
    /// Its reservation and a buff.
    Buff {
        /// PoB's buff name (the tag's `effectName`, else the skill's name):
        /// buffs of one name merge.
        name: &'static str,
        /// PoB's `effectType`.
        effect: EffectType,
        /// The buff's stats, sorted by stat ID.
        stats: &'static [(&'static str, BuffStat)],
    },
}

/// PoB's `effectType` of a buff.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectType {
    /// An aura (`buff.type == "Aura"`).
    Aura,
    /// A buff (`buff.type == "Buff"`).
    Buff,
}

impl EffectType {
    /// PoB's name.
    #[must_use]
    pub const fn pob(self) -> &'static str {
        match self {
            Self::Aura => "Aura",
            Self::Buff => "Buff",
        }
    }
}

/// What one stat of a buff does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BuffStat {
    /// Effects on the character, in the hit rows' form (quantity, op,
    /// divisor, all flags, any flags).
    Rows(&'static [HitEffect]),
    /// Reaches no slice field: a category, as `NO_EFFECT`'s.
    NoSlice(&'static str),
    /// Applies only under a condition only a modifier sets (PoB's
    /// `effectCond`), so it applies nowhere a modifier does not set it,
    /// and that modifier's stat is a reason ([`DEPENDS_ON`]).
    Gated(&'static str),
}

const NONE: Flags = Flags::NONE;
const fn added(t: DamageType, b: Bound) -> Quantity {
    Quantity::AddedDamage(t, b)
}

/// The modelled active skills, sorted by granted effect ID (a test checks
/// the order). Each was checked against PoB v2.67.2's data (module docs)
/// and no calc module names it.
pub const SKILLS: &[(&str, Kind)] = &[
    (
        "ArcticArmour",
        Kind::Buff {
            name: "Arctic Armour",
            effect: EffectType::Buff,
            stats: &[
                ("base_immune_to_freeze", BuffStat::NoSlice("freeze")),
                (
                    "new_arctic_armour_fire_damage_taken_when_hit_+%_final",
                    BuffStat::NoSlice("damage-taken"),
                ),
                (
                    "new_arctic_armour_physical_damage_taken_when_hit_+%_final",
                    BuffStat::NoSlice("damage-taken"),
                ),
            ],
        },
    ),
    (
        "BloodRage",
        Kind::Buff {
            name: "Blood Rage",
            effect: EffectType::Buff,
            stats: &[
                (
                    "attack_speed_+%_granted_from_skill",
                    BuffStat::Rows(&[(Quantity::Speed, Op::Increased, 1.0, Flags::ATTACK, NONE)]),
                ),
                (
                    "base_physical_damage_%_of_maximum_energy_shield_to_deal_per_minute",
                    BuffStat::NoSlice("degeneration"),
                ),
                (
                    "base_physical_damage_%_of_maximum_life_to_deal_per_minute",
                    BuffStat::NoSlice("degeneration"),
                ),
                (
                    "life_leech_from_physical_attack_damage_permyriad",
                    BuffStat::NoSlice("leech"),
                ),
            ],
        },
    ),
    (
        "Clarity",
        Kind::Buff {
            name: "Clarity",
            effect: EffectType::Aura,
            stats: &[(
                "base_mana_regeneration_rate_per_minute",
                BuffStat::NoSlice("regeneration"),
            )],
        },
    ),
    (
        "Discipline",
        Kind::Buff {
            name: "Discipline",
            effect: EffectType::Aura,
            stats: &[
                (
                    "base_maximum_energy_shield",
                    BuffStat::Rows(&[(Quantity::EnergyShield, Op::Base, 1.0, NONE, NONE)]),
                ),
                (
                    "energy_shield_recharge_rate_+%",
                    BuffStat::NoSlice("recharge"),
                ),
            ],
        },
    ),
    ("FreezingPulse", Kind::Inert),
    ("Frostblink", Kind::Inert),
    ("FrostblinkAltX", Kind::Inert),
    ("GroundSlam", Kind::Inert),
    (
        "Haste",
        Kind::Buff {
            name: "Haste",
            effect: EffectType::Aura,
            stats: &[
                (
                    "attack_speed_+%_granted_from_skill",
                    BuffStat::Rows(&[(Quantity::Speed, Op::Increased, 1.0, Flags::ATTACK, NONE)]),
                ),
                ("base_movement_velocity_+%", BuffStat::NoSlice("movement")),
                (
                    "cast_speed_+%_granted_from_skill",
                    BuffStat::Rows(&[(Quantity::Speed, Op::Increased, 1.0, Flags::CAST, NONE)]),
                ),
            ],
        },
    ),
    (
        "HeraldOfIce",
        Kind::Buff {
            name: "Herald of Ice",
            effect: EffectType::Buff,
            stats: &[
                (
                    "attack_maximum_added_cold_damage",
                    BuffStat::Rows(&[(
                        added(DamageType::Cold, Bound::Max),
                        Op::Base,
                        1.0,
                        Flags::ATTACK,
                        NONE,
                    )]),
                ),
                (
                    "attack_minimum_added_cold_damage",
                    BuffStat::Rows(&[(
                        added(DamageType::Cold, Bound::Min),
                        Op::Base,
                        1.0,
                        Flags::ATTACK,
                        NONE,
                    )]),
                ),
                (
                    "spell_maximum_added_cold_damage",
                    BuffStat::Rows(&[(
                        added(DamageType::Cold, Bound::Max),
                        Op::Base,
                        1.0,
                        Flags::SPELL,
                        NONE,
                    )]),
                ),
                (
                    "spell_minimum_added_cold_damage",
                    BuffStat::Rows(&[(
                        added(DamageType::Cold, Bound::Min),
                        Op::Base,
                        1.0,
                        Flags::SPELL,
                        NONE,
                    )]),
                ),
            ],
        },
    ),
    ("HolyFlameTotem", Kind::Inert),
    ("IceNova", Kind::Inert),
    ("LeapSlam", Kind::Inert),
    (
        "Precision",
        Kind::Buff {
            name: "Precision",
            effect: EffectType::Aura,
            stats: &[
                (
                    "accuracy_rating",
                    BuffStat::Rows(&[(Quantity::Accuracy, Op::Base, 1.0, NONE, NONE)]),
                ),
                (
                    "skill_buff_grants_critical_strike_chance_+%",
                    BuffStat::Rows(&[(Quantity::CritChance, Op::Increased, 1.0, NONE, NONE)]),
                ),
            ],
        },
    ),
    ("ShieldCharge", Kind::Inert),
    ("StaticStrike", Kind::Inert),
    ("VaalIceNova", Kind::Inert),
    (
        "Zealotry",
        Kind::Buff {
            name: "Zealotry",
            effect: EffectType::Aura,
            stats: &[
                (
                    "spell_critical_strike_chance_+%",
                    BuffStat::Rows(&[(
                        Quantity::CritChance,
                        Op::Increased,
                        1.0,
                        Flags::SPELL,
                        NONE,
                    )]),
                ),
                (
                    "spell_damage_aura_spell_damage_+%_final",
                    BuffStat::Rows(&[(Quantity::Damage, Op::More, 1.0, Flags::SPELL, NONE)]),
                ),
            ],
        },
    ),
];

/// The modelled supports, sorted by granted effect ID, each with its
/// global stats (a buff of the skill it supports, named by the tag's
/// `effectName` or that skill's name). Checked as [`SKILLS`].
pub const SUPPORTS: &[(&str, &[(&str, BuffStat)])] = &[
    ("SupportAddedColdDamage", &[]),
    ("SupportChanceToBleed", &[]),
    ("SupportElementalArmy", &[]),
    ("SupportElementalProliferation", &[]),
    ("SupportFasterAttacks", &[]),
    ("SupportFortify", &[]),
    ("SupportIncreasedCriticalDamage", &[]),
    ("SupportIncreasedCriticalStrikes", &[]),
    (
        "SupportInfusedChannelling",
        &[(
            "support_storm_barrier_damage_taken_when_hit_+%_final_while_channelling",
            BuffStat::NoSlice("damage-taken"),
        )],
    ),
    ("SupportInspiration", &[]),
    ("SupportIntensify", &[]),
    ("SupportMeleePhysicalDamage", &[]),
    (
        "SupportMomentum",
        &[(
            "support_momentum_movement_speed_+%_per_stack_removed",
            BuffStat::NoSlice("movement"),
        )],
    ),
    ("SupportPowerChargeOnCritical", &[]),
    ("SupportRuthless", &[]),
    ("SupportShockwave", &[]),
    (
        "SupportSummonPhantasm",
        &[
            (
                "phantasm_maximum_added_physical_damage_to_grant",
                BuffStat::Gated("PhantasmalMight"),
            ),
            (
                "phantasm_minimum_added_physical_damage_to_grant",
                BuffStat::Gated("PhantasmalMight"),
            ),
        ],
    ),
    ("SupportTrinity", &[]),
    ("SupportUrgentOrders", &[]),
];

/// Stats that must stay `Unmapped` while buffs act unscaled and
/// reservation without modifiers (module docs), sorted: the generic ones
/// PoB reads as `BuffEffect`, `BuffEffectOnSelf`, `AuraEffect`,
/// `Reserved`, `<Pool>Reserved`, `ReservationEfficiency`, or as no
/// reservation, and the only setter of Phantasmal Might
/// (`ModParser.lua` 4473). Found by the game's text in the bundle's stat
/// descriptions (a maintainer-run test checks each is in the bundle).
/// A skill's own ("precision_aura_effect_+%", "clarity_mana_reservation_+%")
/// and gem level stats are guarded by name ([`GUARDED_WORDS`]).
pub const DEPENDS_ON: &[&str] = &[
    "aura_effect_+%",
    "aura_skill_no_reservation",
    "base_life_reservation_+%",
    "base_life_reservation_efficiency_+%",
    "base_mana_reservation_+%",
    "base_mana_reservation_efficiency_+%",
    "base_reservation_+%",
    "base_reservation_efficiency_+%",
    "base_skill_no_reservation",
    "buff_effect_on_self_+%",
    "golem_buff_effect_+%",
    "herald_effect_on_self_+%",
    "mana_reservation_efficiency_-2%_per_1",
    "non_curse_aura_effect_+%",
    "reservation_efficiency_-2%_per_1",
    "skill_buff_effect_+%",
    "summoned_phantasms_grant_buff",
];

/// Words no mapped or `NoEffect` stat ID may hold while this module adds
/// buffs unscaled and reservation without modifiers (a test): a skill's
/// own effect and reservation stats, and gem levels and quality. The
/// global gem level stats are not rows: [`super::gem_levels`] maps them,
/// and every gem here takes their levels (P3-8i); a gem level that
/// depends on the socket stays unmapped.
pub const GUARDED_WORDS: &[&str] = &[
    "aura_effect",
    "buff_effect",
    "extra_aura",
    "gem_level",
    "gem_quality",
    "phantasm",
    "reserve",
];

/// What the table says about an active skill.
#[must_use]
pub fn kind(id: &str) -> Option<Kind> {
    SKILLS
        .binary_search_by(|(k, _)| (*k).cmp(id))
        .ok()
        .map(|i| SKILLS[i].1)
}

/// A modelled support's global stats.
#[must_use]
pub fn support(id: &str) -> Option<&'static [(&'static str, BuffStat)]> {
    SUPPORTS
        .binary_search_by(|(k, _)| (*k).cmp(id))
        .ok()
        .map(|i| SUPPORTS[i].1)
}

/// `floor(x, 4)` as PoB's `MoreInternal` and `CalcPerform` take it for
/// `SupportManaMultiplier` (`data.highPrecisionMods` precision 4,
/// `ModList.lua` 139 to 141; `Common.lua` `floor` 659 to 666).
#[must_use]
pub fn support_multiplier(supports: &[GemEffectRef<'_>]) -> f64 {
    let mut product = 1.0;
    for s in supports {
        let Some(level) = skill::effect_level(s.effect, s.level) else {
            continue;
        };
        // PoB's `manaMultiplier` is the game's cost multiplier less 100,
        // a MORE mod; a support without one adds nothing.
        #[expect(
            clippy::cast_precision_loss,
            reason = "cost multipliers are small integers, exact as f64"
        )]
        let more = (level.cost_multiplier - 100) as f64;
        if more != 0.0 {
            product *= 1.0 + more / 100.0;
        }
    }
    floor4(product)
}

/// PoB's two floors of the multiplier's product (`MoreInternal`:
/// `floor(result x modResult x 10^4) / 10^4` with `result` 1; then
/// `CalcPerform`'s `floor(.., 4)`, which adds 0.0001 first), in doubles as
/// PoB takes them.
#[must_use]
pub fn floor4(product: f64) -> f64 {
    let inner = crate::math::floor(product * 10_000.0) / 10_000.0;
    crate::math::floor(inner * 10_000.0 + 0.0001) / 10_000.0
}

/// One skill's reservation of one pool, as PoB computes it with no
/// reservation modifier (module docs): the flat amount and the percent.
#[must_use]
pub fn reserved(flat: i64, percent_hundredths: i64, mult: f64) -> (f64, f64) {
    #[expect(
        clippy::cast_precision_loss,
        reason = "reservations are small integers, exact as f64"
    )]
    let (flat, percent) = (flat as f64, percent_hundredths as f64 / 100.0);
    // `values.inc` 0, `values.more` 1, efficiency 0, `efficiencyMore` 1.
    let (inc, more, efficiency, efficiency_more) = (0.0, 1.0, 0.0, 1.0);
    let flat_value = crate::math::floor(flat * mult);
    let reserved_flat = if flat_value == 0.0 {
        0.0
    } else {
        let x = flat_value * (100.0 + inc) / 100.0 * more
            / (1.0 + efficiency / 100.0)
            / efficiency_more;
        crate::math::max(crate::math::round_half_up(x), 0.0)
    };
    let percent_value = percent * mult;
    let reserved_percent = if percent_value == 0.0 {
        0.0
    } else {
        let x = percent_value * (100.0 + inc) / 100.0 * more
            / (1.0 + efficiency / 100.0)
            / efficiency_more;
        // PoB's `round(x, 2)`: `floor(x x 10^2 + 0.5) / 10^2`.
        crate::math::max(crate::math::floor(x * 100.0 + 0.5) / 100.0, 0.0)
    };
    (reserved_flat, reserved_percent)
}

/// Whether the item set's weapon set in use holds the group's slot (PoB's
/// `slotEnabled`); a group without a slot is always in use.
fn slot_enabled(build: &Build, slot: Option<&str>) -> bool {
    let Some(slot) = slot else { return true };
    let second = super::collect::active_item_set(build)
        .and_then(|set| {
            set.attributes
                .iter()
                .find(|a| a.name == "useSecondWeaponSet")
        })
        .is_some_and(|a| a.value == "true");
    super::collect::slot_in_use(slot, second)
}

/// The largest value of each buff stat, by buff name, then stat
/// (`mergeBuff`), with the gem it came from first and its rows.
type Merged = BTreeMap<(&'static str, &'static str), (i64, Source, &'static [HitEffect])>;

/// Adds the other gems' reservation and buffs, and their reasons (module
/// docs). `main` is the main skill's group (0-based) and active effect
/// when the engine computes its hit, which acts through its own path.
pub fn gems(
    input: &CalcInput<'_>,
    main: Option<(usize, &MainSkill<'_>)>,
    levels: &GemLevels,
    ingest: &mut Ingest<'_>,
) {
    let build = input.build;
    let Ok(groups) = skill::active_groups(build) else {
        // No typed set: nothing; an untyped group is a reason of `collect`.
        return;
    };
    let main_group = skill::main_skill(build, input.data, levels)
        .ok()
        .map(|m| m.group - 1);
    let matches = sockets::socket_matches(build, &groups, input.data);
    let mut merged: Merged = BTreeMap::new();
    for (g, group) in groups.iter().enumerate() {
        let slot = group.slot.as_ref().map(|s| s.as_str());
        if main_group != Some(g) && !(group.enabled && slot_enabled(build, slot)) {
            continue;
        }
        // A group an item, the tree, or another source grants is rebuilt by
        // PoB from its source (level, `enableGlobal1` forced on, no
        // supports or the slot's: `CalcSetup.lua` 1340 to 1402): not
        // modelled.
        if skill::attribute(&group.attributes, "source").is_some() {
            for (i, gem) in group.gems.typed().enumerate() {
                if gem.enabled {
                    ingest.reason(UnsupportedReason::UnmodelledGem {
                        source: Source::Gem { group: g, gem: i },
                    });
                }
            }
            continue;
        }
        let group_matches = matches.get(g).map_or(&[][..], Vec::as_slice);
        let skills = match skill::group_skills(group, group_matches, input.data, levels) {
            Ok(s) => s,
            Err(_) => {
                // A gem the data does not have: every enabled gem of the
                // group is a reason, as before P3-12h.
                for (i, gem) in group.gems.typed().enumerate() {
                    if gem.enabled {
                        ingest.reason(UnsupportedReason::UnmodelledGem {
                            source: Source::Gem { group: g, gem: i },
                        });
                    }
                }
                continue;
            }
        };
        for s in &skills {
            let is_main = main.is_some_and(|(mg, m)| {
                mg == g
                    && m.active_gem.gem == s.active_gem.gem
                    && m.active_gem.effect.id == s.active_gem.effect.id
            });
            if is_main {
                // Its stats act through the hit; what the hit does not read
                // is a reason.
                let source = Source::Gem {
                    group: g,
                    gem: s.active_gem.gem,
                };
                if reserves(&s.types) {
                    ingest.reason(UnsupportedReason::UnmodelledSkill {
                        source,
                        what: "reservation",
                    });
                }
                socket_reasons(g, s, ingest);
                continue;
            }
            one_skill(input, g, s, &mut merged, ingest);
        }
    }
    // Buffs in name order, each stat in ID order: every effect is a sum
    // term of integers, so the order changes no value.
    for (value, source, rows) in merged.into_values() {
        #[expect(
            clippy::cast_precision_loss,
            reason = "stat values are small integers, exact as f64"
        )]
        let value = value as f64;
        for &(quantity, op, divisor, all, any) in rows {
            ingest.effect(Effect {
                quantity,
                op,
                value: value / divisor,
                conditions: Conditions::none().with_all(all).with_any(any),
                source: source.clone(),
            });
        }
    }
}

/// One active skill of group `g` (module docs).
fn one_skill(
    input: &CalcInput<'_>,
    g: usize,
    s: &GroupSkill<'_>,
    merged: &mut Merged,
    ingest: &mut Ingest<'_>,
) {
    let source = |gem: usize| Source::Gem { group: g, gem };
    let active_source = source(s.active_gem.gem);
    let Some(kind) = kind(&s.active_gem.effect.id) else {
        ingest.reason(UnsupportedReason::UnmodelledGem {
            source: active_source,
        });
        return;
    };
    // PoB creates an effect with a global effect only when its
    // `enableGlobal<n>` is on: a buff whose flag is off does not exist.
    if s.global_off && matches!(kind, Kind::Buff { .. }) {
        return;
    }
    let mut supported = true;
    for sup in &s.supports {
        if support(&sup.effect.id).is_none() {
            ingest.reason(UnsupportedReason::UnmodelledGem {
                source: source(sup.gem),
            });
            supported = false;
        }
    }
    let Kind::Buff {
        name, stats: own, ..
    } = kind
    else {
        // Inert: its supports act on it alone.
        return;
    };
    if !supported {
        return;
    }
    socket_reasons(g, s, ingest);
    // The skill's instance and its supports' instances: each global stat
    // of the table joins the buff.
    let instances = std::iter::once((&s.active_gem, own)).chain(
        s.supports
            .iter()
            .map(|sup| (sup, support(&sup.effect.id).unwrap_or(&[]))),
    );
    for (r, table) in instances {
        let instance = skill::instance(r.effect, r.level, r.quality, input.data);
        if !instance.problems.is_empty() {
            for p in &instance.problems {
                ingest.reason(UnsupportedReason::UnmodelledSkill {
                    source: source(r.gem),
                    what: p.name(),
                });
            }
            continue;
        }
        for (stat, value) in &instance.stats {
            let Some(&(id, b)) = table.iter().find(|(id, _)| id == stat) else {
                continue;
            };
            match b {
                BuffStat::Rows(rows) => {
                    let entry = merged
                        .entry((name, id))
                        .or_insert((*value, source(r.gem), rows));
                    if *value > entry.0 {
                        *entry = (*value, source(r.gem), rows);
                    }
                }
                BuffStat::NoSlice(_) | BuffStat::Gated(_) => {}
            }
        }
    }
    reservation(s, &active_source, ingest);
}

/// A reason for each gem of the skill (its active gem and the supports
/// that apply) whose socket colour bonus is not known
/// ([`sockets::SocketMatch::Unknown`]): its quality may differ by
/// [`sockets::BONUS`].
fn socket_reasons(g: usize, s: &GroupSkill<'_>, ingest: &mut Ingest<'_>) {
    for r in std::iter::once(&s.active_gem).chain(&s.supports) {
        if matches!(r.socket, sockets::SocketMatch::Unknown(_)) {
            ingest.reason(UnsupportedReason::UnmodelledSkill {
                source: Source::Gem {
                    group: g,
                    gem: r.gem,
                },
                what: "socket-colour-quality",
            });
        }
    }
}

/// Whether a skill with these types reserves (PoB: `HasReservation` and
/// not `ReservationBecomesCost`, `CalcPerform.lua` 1940).
fn reserves(types: &std::collections::BTreeSet<String>) -> bool {
    types.contains("HasReservation") && !types.contains("ReservationBecomesCost")
}

/// The skill's reservation (module docs), one effect per pool and kind.
fn reservation(s: &GroupSkill<'_>, source: &Source, ingest: &mut Ingest<'_>) {
    if !reserves(&s.types) {
        return;
    }
    let effect = s.active_gem.effect;
    let Some(level) = skill::effect_level(effect, s.active_gem.level) else {
        ingest.reason(UnsupportedReason::UnmodelledSkill {
            source: source.clone(),
            what: "no-level",
        });
        return;
    };
    let mult = support_multiplier(&s.supports);
    for r in &level.reservations {
        let pool = match r.resource {
            Resource::Mana => Pool::Mana,
            Resource::Life => Pool::Life,
            Resource::Spirit => {
                ingest.reason(UnsupportedReason::UnmodelledSkill {
                    source: source.clone(),
                    what: "spirit-reservation",
                });
                continue;
            }
        };
        let (flat, percent) = if r.percent {
            reserved(0, r.amount, mult)
        } else {
            reserved(r.amount, 0, mult)
        };
        for (kind, value) in [
            (ReservationKind::Flat, flat),
            (ReservationKind::Percent, percent),
        ] {
            if value != 0.0 {
                ingest.effect(Effect {
                    quantity: Quantity::Poe1(Poe1Quantity::Reservation(pool, kind)),
                    op: Op::Base,
                    value,
                    conditions: Conditions::none(),
                    source: source.clone(),
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest::{StatMap, StatMapping};
    use crate::poe1::ArmourSlot;
    use crate::poe1::mapping::{
        HIT_ROWS, LOCAL_ROWS, NO_EFFECT, PieceMapping, Poe1Mapping, ROWS, SkillMapping,
        WEAPON_ROWS, WeaponMapping,
    };
    use loom_domain::StatId;

    fn sorted_unique<T: Ord>(ids: &[T]) -> bool {
        ids.windows(2).all(|w| w[0] < w[1])
    }

    #[test]
    fn the_tables_should_be_sorted_and_unique() {
        let skills: Vec<_> = SKILLS.iter().map(|(id, _)| *id).collect();
        let supports: Vec<_> = SUPPORTS.iter().map(|(id, _)| *id).collect();
        assert!(sorted_unique(&skills));
        assert!(sorted_unique(&supports));
        assert!(sorted_unique(DEPENDS_ON));
        for (id, kind) in SKILLS {
            if let Kind::Buff { stats, .. } = kind {
                let ids: Vec<_> = stats.iter().map(|(s, _)| *s).collect();
                assert!(sorted_unique(&ids), "{id}");
            }
        }
        assert_eq!(
            kind("Precision").map(|k| matches!(k, Kind::Buff { .. })),
            Some(true)
        );
        assert_eq!(kind("GroundSlam"), Some(Kind::Inert));
        assert_eq!(kind("Fireball"), None);
        assert!(support("SupportFasterAttacks").is_some_and(<[_]>::is_empty));
        assert!(support("SupportArcaneSurge").is_none());
    }

    /// PoB applies buffs after attributes, life, and mana are computed
    /// (`CalcPerform.lua` 1905 versus 3284): no buff row may change them.
    #[test]
    fn no_buff_row_should_reach_attributes_life_or_mana() {
        let early = [
            Quantity::Strength,
            Quantity::Dexterity,
            Quantity::Intelligence,
            Quantity::Life,
            Quantity::Mana,
        ];
        let rows = SKILLS
            .iter()
            .filter_map(|(_, k)| match k {
                Kind::Buff { stats, .. } => Some(*stats),
                Kind::Inert => None,
            })
            .chain(SUPPORTS.iter().map(|(_, s)| *s))
            .flatten();
        for (stat, b) in rows {
            if let BuffStat::Rows(effects) = b {
                for e in *effects {
                    assert!(!early.contains(&e.0), "{stat}");
                }
            }
        }
    }

    /// The stats that scale buffs or reservation, or gem levels, stay
    /// reasons through every mapping (module docs).
    #[test]
    fn the_stats_buffs_and_reservation_depend_on_should_stay_unmapped() {
        let ids = HIT_ROWS
            .iter()
            .map(|r| r.0)
            .chain(ROWS.iter().map(|r| r.0))
            .chain(LOCAL_ROWS.iter().map(|r| r.0))
            .chain(WEAPON_ROWS.iter().map(|r| r.0))
            .chain(NO_EFFECT.iter().map(|r| r.0));
        for id in ids {
            for word in GUARDED_WORDS {
                assert!(!id.contains(word), "{id}: revisit poe1::gems");
            }
        }
        let piece = PieceMapping {
            slot: ArmourSlot::Helmet,
        };
        let mappings: [&dyn StatMapping; 4] = [&Poe1Mapping, &SkillMapping, &piece, &WeaponMapping];
        for stat in DEPENDS_ON {
            for m in mappings {
                assert_eq!(m.map(&StatId::new(*stat)), StatMap::Unmapped, "{stat}");
            }
        }
    }

    #[test]
    fn reservation_should_round_as_pob() {
        // Flat: floored before the round (15 x 1.3 = 19.5 gives 19).
        assert_eq!(reserved(32, 0, 1.0), (32.0, 0.0));
        assert_eq!(reserved(15, 0, 1.3), (19.0, 0.0));
        // Percent: hundredths of a percent, rounded to 2 decimals.
        assert_eq!(reserved(0, 2500, 1.2), (0.0, 30.0));
        assert_eq!(reserved(0, 3500, 1.0), (0.0, 35.0));
        assert_eq!(reserved(0, 2250, 1.1), (0.0, 24.75));
        assert_eq!(reserved(0, 0, 1.5), (0.0, 0.0));
    }

    #[test]
    fn the_multiplier_should_floor_twice_in_doubles() {
        assert_eq!(floor4(1.0), 1.0);
        assert_eq!(floor4(1.2), 1.2);
        // 1.15 x 10^4 rounds to 11500 exactly in doubles, so the floor
        // keeps it; a product a hair above a step stays on it.
        assert_eq!(floor4(1.15), 1.15);
        assert_eq!(floor4(1.123_456_7), 1.1234);
        assert_eq!(floor4(1.2 * 1.1), 1.32);
        assert_eq!(support_multiplier(&[]), 1.0);
    }

    #[test]
    fn names_should_be_stable() {
        assert_eq!(
            Quantity::Poe1(Poe1Quantity::Reservation(
                Pool::Mana,
                ReservationKind::Percent
            ))
            .to_string(),
            "poe1.reservation.mana.percent"
        );
        assert_eq!(EffectType::Aura.pob(), "Aura");
    }
}
