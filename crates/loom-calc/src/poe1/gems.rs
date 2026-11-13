//! The gems other than the main skill's (Phase 3 plan, step 12; P3-12h,
//! completed in the Phase 3 close-out): what each enabled skill of the
//! active skill set does to the character and the enemy.
//!
//! What PoB v2.67.2 does (line numbers of the pinned checkout,
//! `local/oracle/pob`):
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
//!   `flat + ceil(max x percent / 100)` of each pool (the unreserved phase).
//!   A support whose base mods set the reservation to 0 for the skill's
//!   type makes it reserve nothing ([`NO_RESERVATION_SUPPORTS`]: Guardian's
//!   Blessing on an aura, `Data/Skills/sup_str.lua` 2807);
//! - **global effects** (`CalcActiveSkill.lua` 794 to 859): the skill's
//!   and its supports' mods with a `GlobalEffect` tag form buffs, named by
//!   the tag's `effectName` or the active skill's name, of the tag's
//!   `effectType`. `CalcPerform.lua` then applies each kind (2250 to
//!   2780): a `Buff` (2266) or `Aura` (2392) to the character, scaled by
//!   its effect ([`skill_modifiers::Totals::buff_scale`]) and merged per
//!   name keeping each mod's largest value (`mergeBuff`, 44 to 66); a
//!   `Debuff` or `AuraDebuff` (2534) to the enemy, scaled by debuff (and
//!   aura) effect; a `Curse` (2601) into curse slots, scaled by curse
//!   effect ([`skill_modifiers::Totals::curse_scale`]), up to the curse
//!   limit (3155 to 3215); a `Warcry` (2311) to the character, scaled by
//!   warcry power and uptime. A buff whose `effectCond` fails does nothing
//!   (2260);
//! - **minions' skills** (2725 to 2890): a summoned minion's own skills
//!   that are enabled (the config enables Lightning Golem's Wrath) give
//!   their auras to the character, scaled by the minion skill's aura
//!   effect and the character's effect on self;
//! - **warcries' exerted attacks** (`CalcOffence.lua` 2523 to 2802): only
//!   Rallying Cry changes a hit (`OffensiveWarcryEffect`: `1 +
//!   min(nearby allies, 5) x its damage per ally x uptime`); Intimidating
//!   Cry's double damage enters only PoB's "theoretical" effect, which no
//!   slice field reads; exert modifiers (`ExertIncrease` and kin) scale
//!   damage by uptime;
//! - **by type or name**: brands, totems' life, pacts, triggers, mirages,
//!   and every skill's `<Ailment>Max` (`CalcPerform.lua` 3534).
//!
//! What the engine does: a skill in [`SKILLS`] is modelled, any other is
//! `unmodelled-gem`. A skill there is checked against PoB's data (a
//! maintainer-run test reads PoB's export, format 4): its `GlobalEffect`
//! stats are exactly the table's, each slice row has PoB's mod name, type,
//! and flags, and its other mods name nothing that scales buffs or
//! reservation or sets an ailment's maximum
//! (`tools/parity-cli/src/data_parity/gem_effects.rs`).
//! - [`Kind::Inert`]: no global effect; its supports' global effects
//!   still act (Arcane Surge on Freezing Pulse).
//! - [`Kind::Buff`]: its reservation and its buff rows ([`BuffStat`]),
//!   scaled by buff effect (the golems' buff effect for a golem, and the
//!   gems' own `BuffEffect` and `AuraEffect` stats: [`OWN_EFFECT_STATS`]),
//!   and, for an aura, the non-curse aura effect.
//! - [`Kind::Debuff`]: its rows on the enemy, merged per name (Frost Bomb's
//!   exposure, [`Poe1Quantity::SkillExposure`], which [`super::exposure`]
//!   compares with the others).
//! - [`Kind::Curse`]: its rows on the enemy, scaled by curse effect. With
//!   one curse limit, a second hex is a reason (`curse-limit`): PoB's slot
//!   priority is not modelled. A mark is not modelled.
//! - [`Kind::Warcry`]: its buff rows (none reaches a slice field), and
//!   Rallying Cry's exerted-attack effect only when no ally is configured
//!   (`multiplierNearbyAlly`): otherwise a reason.
//! - Lightning Golem's Wrath ([`WRATH_INPUT`]): when the config enables it
//!   and a Summon Lightning Golem is modelled, its aura rows at the golem's
//!   level ([`wrath`]).
//!
//! A support applied to a modelled skill must be in [`SUPPORTS`] (its
//! global stats only, nothing that scales buffs or reservation but the
//! listed ones); it adds its cost multiplier to the skill's reservation.
//! Any other support applied to it is `unmodelled-gem`; a support that
//! applies to no active skill of its group does nothing (as in PoB).
//!
//! Reservation efficiency, "Reservation of Skills", the effect of non-curse
//! auras (P3-8l), golem buff effect and curse effect (close-out) are read
//! from the character ([`skill_modifiers`]). Other buff and aura effect
//! and reservation modifiers, gem quality modifiers, and gem levels that
//! depend on the socket are stats the mapping does not know ([`DEPENDS_ON`]
//! must stay unmapped: a test), so they are reasons. Global gem levels
//! are in each gem's level ([`super::gem_levels`], P3-8i). A gem's
//! quality from a socket of its colour is in its instance's quality
//! ([`sockets`], P3-12i); a gem whose match is not known is
//! `unmodelled-skill` `socket-colour-quality` when its quality can reach a
//! field.

use std::collections::{BTreeMap, BTreeSet};

use loom_domain::{Build, ConfigInputKind, ConfigSet, ConfigValue};

use super::config::last_input;
use super::gem_levels::GemLevels;
use super::mapping::HitEffect;
use super::skill::{self, GemEffectRef, GroupSkill, MainSkill};
use super::skill_modifiers::{self, Totals};
use super::sockets;
use super::{Poe1Quantity, Pool, ReservationKind};
use crate::condition::{ConditionKey, Flags};
use crate::ingest::Ingest;
use crate::input::{CalcData, CalcInput, Resource};
use crate::quantity::{Bound, DamageType, Op, Quantity};
use crate::source::Source;
use crate::state::UnsupportedReason;
use crate::store::{Conditions, Effect};

/// What a modelled skill does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    /// Nothing of its own (module docs).
    Inert,
    /// Its reservation and a buff on the character.
    Buff {
        /// PoB's buff name (the tag's `effectName`, else the skill's name):
        /// buffs of one name merge.
        name: &'static str,
        /// PoB's `effectType`.
        effect: EffectType,
        /// The buff's stats, sorted by stat ID (`""`: a base mod).
        stats: &'static [(&'static str, BuffStat)],
    },
    /// A debuff on the enemy (PoB's `effectType = "Debuff"`).
    Debuff {
        /// PoB's buff name.
        name: &'static str,
        /// Its stats, sorted by stat ID.
        stats: &'static [(&'static str, BuffStat)],
    },
    /// A hex on the enemy (PoB's `effectType = "Curse"`).
    Curse {
        /// PoB's curse name.
        name: &'static str,
        /// Its stats, sorted by stat ID.
        stats: &'static [(&'static str, BuffStat)],
    },
    /// A warcry (PoB's `effectType = "Warcry"`).
    Warcry {
        /// PoB's buff name.
        name: &'static str,
        /// Its stats, sorted by stat ID.
        stats: &'static [(&'static str, BuffStat)],
    },
}

impl Kind {
    /// The kind's stats.
    #[must_use]
    pub const fn stats(&self) -> &'static [(&'static str, BuffStat)] {
        match self {
            Self::Inert => &[],
            Self::Buff { stats, .. }
            | Self::Debuff { stats, .. }
            | Self::Curse { stats, .. }
            | Self::Warcry { stats, .. } => stats,
        }
    }

    /// PoB's `effectType` and buff name, when the skill has a global
    /// effect.
    #[must_use]
    pub const fn buff(&self) -> Option<(&'static str, &'static str)> {
        match self {
            Self::Inert => None,
            Self::Buff { name, effect, .. } => Some((effect.pob(), name)),
            Self::Debuff { name, .. } => Some(("Debuff", name)),
            Self::Curse { name, .. } => Some(("Curse", name)),
            Self::Warcry { name, .. } => Some(("Warcry", name)),
        }
    }
}

/// PoB's `effectType` of a buff on the character.
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

/// What one stat of a skill's global effect does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BuffStat {
    /// Effects (on the character for a buff, on the enemy for a debuff or
    /// a curse), in the hit rows' form (quantity, op, divisor, all flags,
    /// any flags).
    Rows(&'static [HitEffect]),
    /// Reaches no slice field: a category, as `NO_EFFECT`'s.
    NoSlice(&'static str),
    /// Applies only under a condition only a modifier sets (PoB's
    /// `effectCond`), so it applies nowhere a modifier does not set it,
    /// and that modifier's stat is a reason ([`DEPENDS_ON`]).
    Gated(&'static str),
    /// Applies only when the config check `input` is on (it sets PoB's
    /// `effectCond` `condition`; Flame Wall's added damage).
    WhenInput {
        /// The config check.
        input: &'static str,
        /// PoB's `effectCond`.
        condition: &'static str,
        /// The rows.
        rows: &'static [HitEffect],
    },
    /// Applies in Blood Stance (PoB's `effectCond = "BloodStance"`, on
    /// unless the config's stance is Sand: `CalcSetup.lua` 538,
    /// `ConfigOptions.lua` 643).
    BloodStance(&'static [HitEffect]),
    /// Applies while unreserved life is at least 2 and mana at least 1
    /// (PoB's `StatThreshold` tags, Righteous Fire's): the rows carry the
    /// state [`STAT_THRESHOLD`], which the hit sets from those fields.
    Threshold(&'static [HitEffect]),
}

/// The state of [`BuffStat::Threshold`] rows.
pub const STAT_THRESHOLD: &str = "LifeUnreservedAtLeast2AndManaAtLeast1";

const NONE: Flags = Flags::NONE;
const fn added(t: DamageType, b: Bound) -> Quantity {
    Quantity::AddedDamage(t, b)
}
const fn enemy_res(t: DamageType) -> Quantity {
    Quantity::Poe1(Poe1Quantity::EnemyResistance(t))
}
const GOLEM_CONDITION: BuffStat = BuffStat::NoSlice("golem-condition");

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
    // Automation reserves and triggers the skill it supports
    // (`SupportAutomation`); nothing of its own reaches the character
    // (`Data/Skills/act_int.lua` 417).
    ("Automation", Kind::Inert),
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
    // Convocation's buff goes to minions only (`buffNotPlayer`,
    // `Data/Skills/other.lua` 907): nothing reaches the character.
    (
        "Convocation",
        Kind::Buff {
            name: "Convocation",
            effect: EffectType::Buff,
            stats: &[(
                "base_life_regeneration_rate_per_minute",
                BuffStat::NoSlice("minion-buff"),
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
    (
        "ElementalWeakness",
        Kind::Curse {
            name: "Elemental Weakness",
            stats: &[
                (
                    "base_resist_all_elements_%",
                    BuffStat::Rows(&[
                        (enemy_res(DamageType::Fire), Op::Base, 1.0, NONE, NONE),
                        (enemy_res(DamageType::Cold), Op::Base, 1.0, NONE, NONE),
                        (enemy_res(DamageType::Lightning), Op::Base, 1.0, NONE, NONE),
                    ]),
                ),
                (
                    "self_elemental_status_duration_-%",
                    BuffStat::NoSlice("ailment-duration"),
                ),
            ],
        },
    ),
    (
        "FlameWall",
        Kind::Buff {
            name: "Flame Wall",
            effect: EffectType::Buff,
            stats: &[
                (
                    "flame_wall_maximum_added_fire_damage",
                    BuffStat::WhenInput {
                        input: "flameWallAddedDamage",
                        condition: "FlameWallAddedDamage",
                        rows: &[(
                            added(DamageType::Fire, Bound::Max),
                            Op::Base,
                            1.0,
                            Flags::PROJECTILE,
                            NONE,
                        )],
                    },
                ),
                (
                    "flame_wall_minimum_added_fire_damage",
                    BuffStat::WhenInput {
                        input: "flameWallAddedDamage",
                        condition: "FlameWallAddedDamage",
                        rows: &[(
                            added(DamageType::Fire, Bound::Min),
                            Op::Base,
                            1.0,
                            Flags::PROJECTILE,
                            NONE,
                        )],
                    },
                ),
            ],
        },
    ),
    (
        "FleshAndStone",
        Kind::Buff {
            name: "Flesh and Stone",
            effect: EffectType::Aura,
            stats: &[
                (
                    "flesh_stone_blood_stance_enemies_physical_damage_taken_when_hit_+%_final_from_player_distance",
                    BuffStat::BloodStance(&[(
                        Quantity::Poe1(Poe1Quantity::EnemyTypeDamageTaken(DamageType::Physical)),
                        Op::More,
                        1.0,
                        Flags::HIT,
                        NONE,
                    )]),
                ),
                (
                    "flesh_stone_sand_stance_damage_taken_+%_final_from_distance_from_enemy_hits",
                    BuffStat::NoSlice("damage-taken"),
                ),
            ],
        },
    ),
    ("FreezingPulse", Kind::Inert),
    (
        "FrostBomb",
        Kind::Debuff {
            name: "Frost Bomb",
            stats: &[
                (
                    "base_cold_damage_resistance_%",
                    BuffStat::Rows(&[(
                        Quantity::Poe1(Poe1Quantity::SkillExposure(DamageType::Cold)),
                        Op::Base,
                        1.0,
                        NONE,
                        NONE,
                    )]),
                ),
                (
                    "life_regeneration_rate_+%",
                    BuffStat::NoSlice("regeneration"),
                ),
            ],
        },
    ),
    (
        "Frostbite",
        Kind::Curse {
            name: "Frostbite",
            stats: &[
                (
                    "base_cold_damage_resistance_%",
                    BuffStat::Rows(&[(enemy_res(DamageType::Cold), Op::Base, 1.0, NONE, NONE)]),
                ),
                (
                    "base_self_freeze_duration_-%",
                    BuffStat::NoSlice("ailment-duration"),
                ),
                ("chance_to_be_frozen_%", BuffStat::NoSlice("freeze")),
            ],
        },
    ),
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
        "HeraldOfAsh",
        Kind::Buff {
            name: "Herald of Ash",
            effect: EffectType::Buff,
            stats: &[(
                "physical_damage_%_to_add_as_fire",
                BuffStat::Rows(&[(
                    Quantity::Poe1(Poe1Quantity::GainAs(DamageType::Physical, DamageType::Fire)),
                    Op::Base,
                    1.0,
                    NONE,
                    NONE,
                )]),
            )],
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
    // Herald of Purity's sentinels attack on their own; the buff's more
    // physical damage reaches the character.
    (
        "HeraldOfPurity",
        Kind::Buff {
            name: "Herald of Purity",
            effect: EffectType::Buff,
            stats: &[(
                "herald_of_purity_physical_damage_+%_final",
                BuffStat::Rows(&[(Quantity::Damage, Op::More, 1.0, Flags::PHYSICAL, NONE)]),
            )],
        },
    ),
    ("HolyFlameTotem", Kind::Inert),
    ("IceNova", Kind::Inert),
    (
        "IntimidatingCry",
        Kind::Warcry {
            name: "Intimidating Cry",
            stats: &[
                (
                    "intimidating_cry_movement_speed_+%_per_5_power_up_to_cap",
                    BuffStat::NoSlice("movement"),
                ),
                (
                    "warcry_count_power_from_enemies",
                    BuffStat::NoSlice("warcry-power"),
                ),
            ],
        },
    ),
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
    (
        "RallyingCry",
        Kind::Warcry {
            name: "Rallying Cry",
            stats: &[
                (
                    "rallying_cry_weapon_damage_%_for_allies_per_5_monster_power",
                    BuffStat::NoSlice("allies"),
                ),
                (
                    "warcry_count_power_from_enemies",
                    BuffStat::NoSlice("warcry-power"),
                ),
            ],
        },
    ),
    (
        "RighteousFireAltX",
        Kind::Buff {
            name: "Righteous Fire of Arcane Devotion",
            effect: EffectType::Buff,
            stats: &[
                (
                    "base_nonlethal_fire_damage_%_of_maximum_mana_taken_per_minute",
                    BuffStat::NoSlice("degeneration"),
                ),
                (
                    "righteous_fire_cast_speed_+%_final",
                    BuffStat::Threshold(&[(Quantity::Speed, Op::More, 1.0, Flags::CAST, NONE)]),
                ),
            ],
        },
    ),
    ("ShieldCharge", Kind::Inert),
    ("StaticStrike", Kind::Inert),
    (
        "SummonFlameGolem",
        Kind::Buff {
            name: "Summon Flame Golem",
            effect: EffectType::Buff,
            stats: &[
                ("", GOLEM_CONDITION),
                (
                    "fire_golem_grants_area_of_effect_+%",
                    BuffStat::NoSlice("area"),
                ),
                (
                    "fire_golem_grants_damage_+%",
                    BuffStat::Rows(&[(Quantity::Damage, Op::Increased, 1.0, NONE, NONE)]),
                ),
            ],
        },
    ),
    (
        "SummonIceGolem",
        Kind::Buff {
            name: "Summon Ice Golem",
            effect: EffectType::Buff,
            stats: &[
                ("", GOLEM_CONDITION),
                (
                    "ice_golem_grants_accuracy_rating_+",
                    BuffStat::Rows(&[(Quantity::Accuracy, Op::Base, 1.0, NONE, NONE)]),
                ),
                (
                    "ice_golem_grants_critical_strike_chance_+%",
                    BuffStat::Rows(&[(Quantity::CritChance, Op::Increased, 1.0, NONE, NONE)]),
                ),
            ],
        },
    ),
    (
        "SummonLightningGolem",
        Kind::Buff {
            name: "Summon Lightning Golem",
            effect: EffectType::Buff,
            stats: &[
                ("", GOLEM_CONDITION),
                (
                    "lightning_golem_grants_attack_and_cast_speed_+%",
                    BuffStat::Rows(&[(Quantity::Speed, Op::Increased, 1.0, NONE, NONE)]),
                ),
                (
                    "lightning_golem_grants_base_mana_regeneration_rate_per_minute",
                    BuffStat::NoSlice("regeneration"),
                ),
            ],
        },
    ),
    (
        "SummonStoneGolem",
        Kind::Buff {
            name: "Summon Stone Golem",
            effect: EffectType::Buff,
            stats: &[
                ("", GOLEM_CONDITION),
                (
                    "stone_golem_grants_base_life_regeneration_rate_per_minute",
                    BuffStat::NoSlice("regeneration"),
                ),
                (
                    "stone_golem_grants_defences_+%",
                    BuffStat::Rows(&[
                        (Quantity::EnergyShield, Op::Increased, 1.0, NONE, NONE),
                        (Quantity::Armour, Op::Increased, 1.0, NONE, NONE),
                        (Quantity::Evasion, Op::Increased, 1.0, NONE, NONE),
                    ]),
                ),
            ],
        },
    ),
    // Summon Stone Golem of Safeguarding: its golem takes part of your
    // allies' melee damage; its only global mod is the golem condition.
    (
        "SummonStoneGolemAltY",
        Kind::Buff {
            name: "Summon Stone Golem of Safeguarding",
            effect: EffectType::Buff,
            stats: &[
                ("", GOLEM_CONDITION),
                (
                    "stone_golem_grants_melee_damage_removed_from_stone_golem_before_life_or_es_%",
                    BuffStat::NoSlice("damage-taken"),
                ),
            ],
        },
    ),
    // Shockwave Support's own attack (`SupportBluntWeaponShockwave`): it
    // hits on its own when triggered; nothing reaches the character.
    ("SupportBluntWeaponShockwave", Kind::Inert),
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
    // Arcane Surge Support's buff (`effectName = "Arcane Surge"`): its cast
    // speed is the largest any source gives (`ArcaneSurgeCastSpeed` MAX,
    // `poe1::buffs`), its mana regeneration reaches no slice field.
    (
        "SupportArcaneSurge",
        &[
            (
                "support_arcane_surge_cast_speed_+%",
                BuffStat::Rows(&[(
                    Quantity::Poe1(Poe1Quantity::ArcaneSurgeCastSpeed),
                    Op::Base,
                    1.0,
                    NONE,
                    NONE,
                )]),
            ),
            (
                "support_arcane_surge_mana_regeneration_rate_+%",
                BuffStat::NoSlice("regeneration"),
            ),
        ],
    ),
    ("SupportAutomation", &[]),
    ("SupportChanceToBleed", &[]),
    ("SupportElementalArmy", &[]),
    ("SupportElementalProliferation", &[]),
    ("SupportFasterAttacks", &[]),
    ("SupportFortify", &[]),
    ("SupportGuardiansBlessing", &[]),
    ("SupportGuardiansBlessingMinion", &[]),
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
    ("SupportMeatShield", &[]),
    ("SupportMeleePhysicalDamage", &[]),
    ("SupportMinionLife", &[]),
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

/// Supports whose base mods set the reservation of a skill of the type to
/// 0 (PoB's `SkillData` `manaReservationFlat` and kin, 0, tagged with the
/// type): Guardian's Blessing on an aura (`sup_str.lua` 2807).
pub const NO_RESERVATION_SUPPORTS: &[(&str, &str)] = &[("SupportGuardiansBlessing", "Aura")];

/// A gem's own stats that scale the buff of the skill it is part of
/// (PoB's `SkillStatMap.lua`: `golem_buff_effect_+%` 2089 is `BuffEffect`
/// INC, `aura_effect_+%` is `AuraEffect` INC), and whether only an aura
/// reads it.
pub const OWN_EFFECT_STATS: &[(&str, bool)] =
    &[("aura_effect_+%", true), ("golem_buff_effect_+%", false)];

/// Stats that must stay `Unmapped` while buffs act at
/// [`skill_modifiers`]' scale alone and reservation with its terms alone
/// (module docs), sorted: the generic ones PoB reads as `BuffEffect`,
/// `BuffEffectOnSelf`, `AuraEffect`, `<Pool>Reserved` (tagged `Aura`:
/// [`skill_modifiers`]' docs), `DebuffEffect`, `CurseEffect` MORE, the
/// curse limit, exerted attacks (close-out), or as no reservation, and the
/// only setter of Phantasmal Might (`ModParser.lua` 4473). Found by the
/// game's text in the bundle's stat descriptions (a maintainer-run test
/// checks each is in the bundle). A skill's own ("precision_aura_effect_+%",
/// "clarity_mana_reservation_+%") and gem level stats are guarded by name
/// ([`GUARDED_WORDS`]). The stats [`skill_modifiers::STATS`] maps left this
/// list in P3-8l and the close-out.
pub const DEPENDS_ON: &[&str] = &[
    "aura_effect_+%",
    "aura_skill_no_reservation",
    "base_life_reservation_+%",
    "base_mana_reservation_+%",
    "base_skill_no_reservation",
    "buff_effect_on_self_+%",
    "herald_effect_on_self_+%",
    "number_of_additional_curses_allowed",
    "skill_buff_effect_+%",
    "summoned_phantasms_grant_buff",
];

/// Words no mapped or `NoEffect` stat ID may hold while this module adds
/// buffs without the modifiers they name (a test): a skill's own effect
/// and reservation stats, gem levels and quality, and exerted attacks.
/// The global gem level stats are not rows: [`super::gem_levels`] maps
/// them, and every gem here takes their levels (P3-8i); a gem level that
/// depends on the socket stays unmapped.
pub const GUARDED_WORDS: &[&str] = &[
    "aura_effect",
    "buff_effect",
    "curse_effect",
    "debuff_effect",
    "exert",
    "extra_aura",
    "gem_level",
    "gem_quality",
    "phantasm",
    "reserve",
];

/// The config check that enables Lightning Golem's Wrath
/// (`ConfigOptions.lua` 681: `SkillData` `enable` for
/// `LightningGolemWrath`).
pub const WRATH_INPUT: &str = "summonLightningGolemEnableWrath";
/// The Lightning Golem's skill whose minion has Wrath.
pub const WRATH_GOLEM: &str = "SummonLightningGolem";
/// Wrath's granted effect (`Data/Skills/minion.lua` 449).
pub const WRATH_EFFECT: &str = "LightningGolemWrath";
/// PoB's buff name of Wrath.
pub const WRATH_NAME: &str = "Lightning Golem Wrath";
/// Wrath's stats as PoB maps them (each an aura `LightningMin` or
/// `LightningMax` BASE with the attack or spell keyword).
pub const WRATH_STATS: &[(&str, HitEffect)] = &[
    (
        "attack_maximum_added_lightning_damage",
        (
            added(DamageType::Lightning, Bound::Max),
            Op::Base,
            1.0,
            Flags::ATTACK,
            NONE,
        ),
    ),
    (
        "attack_minimum_added_lightning_damage",
        (
            added(DamageType::Lightning, Bound::Min),
            Op::Base,
            1.0,
            Flags::ATTACK,
            NONE,
        ),
    ),
    (
        "spell_maximum_added_lightning_damage",
        (
            added(DamageType::Lightning, Bound::Max),
            Op::Base,
            1.0,
            Flags::SPELL,
            NONE,
        ),
    ),
    (
        "spell_minimum_added_lightning_damage",
        (
            added(DamageType::Lightning, Bound::Min),
            Op::Base,
            1.0,
            Flags::SPELL,
            NONE,
        ),
    ),
];

/// Intimidating Cry's terms of its exerted attacks' uptime
/// (`CalcOffence.lua` 2592 to 2600), each a `Base` effect of
/// [`Poe1Quantity::IntimidatingCry`] the hit reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WarcryValue {
    /// Exerted attacks per cry: `floor((base + extra) x more)` (PoB's
    /// `NumIntimidatingExerts`, `CalcPerform.lua` 2316 to 2320), the
    /// base the gem's `skill_empowers_next_x_melee_attacks`; the extra
    /// exerts are stats the mapping does not know (reasons).
    Exerts,
    /// The cooldown, seconds (`calcSkillCooldown`, `CalcOffence.lua`
    /// 274 to 287): the level's over `1 + inc / 100`, `inc` the gems'
    /// (the warcry's and its supports') `base_cooldown_speed_+%` and
    /// `warcry_cooldown_speed_+%`; rounded up to server ticks unless the
    /// skill stores several uses.
    Cooldown,
    /// Casts per second before action speed (`calcWarcryCastTime`, 289 to
    /// 298): `1 / 0.8 x (1 + inc / 100)`, PoB's base `WarcryCastTime` 0.8
    /// (`CalcSetup.lua` 57) and the gems' `warcry_speed_+%` (Urgent
    /// Orders' among them).
    CastRate,
    /// Stored uses (the level's).
    StoredUses,
}

impl WarcryValue {
    /// The value's stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Exerts => "exerts",
            Self::Cooldown => "cooldown",
            Self::CastRate => "cast-rate",
            Self::StoredUses => "stored-uses",
        }
    }
}

/// PoB's base warcry cast time, seconds (`CalcSetup.lua` 57).
pub const WARCRY_CAST_TIME: f64 = 0.8;
/// PoB's server tick rate (`Data.lua` 231: `1 / 0.033`).
const SERVER_TICK_RATE: f64 = 1.0 / 0.033;

/// Intimidating Cry's uptime terms (module docs) from its first modelled
/// skill: `None` when the level is missing.
fn intimidating_terms(
    input: &CalcInput<'_>,
    s: &GroupSkill<'_>,
) -> Option<[(WarcryValue, f64); 4]> {
    let r = &s.active_gem;
    let level = skill::effect_level(r.effect, r.level)?;
    // The skill's own stats and its supports' (Urgent Orders' warcry
    // speed): PoB's `skillModList` holds both.
    let mut stats: Vec<(String, i64)> = Vec::new();
    for g in std::iter::once(r).chain(&s.supports) {
        let instance = skill::instance(g.effect, g.level, g.quality, input.data);
        if !instance.problems.is_empty() {
            return None;
        }
        stats.extend(instance.stats);
    }
    let stat = |id: &str| {
        #[expect(
            clippy::cast_precision_loss,
            reason = "stat values are small integers, exact as f64"
        )]
        let v = stats
            .iter()
            .filter(|(s, _)| s == id)
            .map(|(_, v)| *v as f64)
            .sum::<f64>();
        v
    };
    #[expect(
        clippy::cast_precision_loss,
        reason = "cooldowns and uses are small integers, exact as f64"
    )]
    let (cooldown_ms, stored) = (level.cooldown as f64, level.stored_uses as f64);
    // `CooldownRecovery` INC: `base_cooldown_speed_+%` and, with the
    // warcry keyword, `warcry_cooldown_speed_+%` (`SkillStatMap.lua` 534,
    // 547).
    let modifier = crate::math::max(
        0.0,
        1.0 + (stat("base_cooldown_speed_+%") + stat("warcry_cooldown_speed_+%")) / 100.0,
    );
    let mut cooldown = cooldown_ms / 1000.0 / modifier;
    if stored <= 1.0 {
        cooldown = crate::math::ceil(cooldown * SERVER_TICK_RATE) / SERVER_TICK_RATE;
    }
    let cast_rate = 1.0 / WARCRY_CAST_TIME * (1.0 + stat("warcry_speed_+%") / 100.0);
    Some([
        (
            WarcryValue::Exerts,
            crate::math::floor(stat("skill_empowers_next_x_melee_attacks")),
        ),
        (WarcryValue::Cooldown, cooldown),
        (WarcryValue::CastRate, cast_rate),
        (WarcryValue::StoredUses, stored),
    ])
}

/// The config input of nearby allies, which Rallying Cry's exerted
/// attacks read (`ConfigOptions.lua` 1110).
pub const NEARBY_ALLY_INPUT: &str = "multiplierNearbyAlly";
/// The config input of the stance (`ConfigOptions.lua` 643).
pub const STANCE_INPUT: &str = "bloodSandStance";

/// The config inputs this module reads.
#[must_use]
pub fn is_input(name: &str) -> bool {
    [WRATH_INPUT, "flameWallAddedDamage", STANCE_INPUT].contains(&name)
}

/// Whether an Arcane Surge Support applies to a skill of the active set
/// that PoB creates (close-out): its buff, named "Arcane Surge", then
/// sets `AffectedByArcaneSurge` (`CalcPerform.lua` 2272), and
/// `doActorMisc` (686 to 705, after the buffs) gives Arcane Surge's
/// effects as the config check does.
#[must_use]
pub fn arcane_surge_support(input: &CalcInput<'_>, levels: &GemLevels) -> bool {
    let build = input.build;
    let Ok(groups) = skill::active_groups(build) else {
        return false;
    };
    let main_group = skill::main_skill(build, input.data, levels)
        .ok()
        .map(|m| m.group - 1);
    let matches = sockets::socket_matches(build, &groups, input.data);
    groups.iter().enumerate().any(|(g, group)| {
        let slot = group.slot.as_ref().map(|s| s.as_str());
        if main_group != Some(g) && !(group.enabled && slot_enabled(build, slot)) {
            return false;
        }
        if skill::attribute(&group.attributes, "source").is_some() {
            return false;
        }
        let group_matches = matches.get(g).map_or(&[][..], Vec::as_slice);
        skill::group_skills(group, group_matches, input.data, levels).is_ok_and(|skills| {
            skills.iter().any(|s| {
                s.supports
                    .iter()
                    .any(|sup| sup.effect.id == "SupportArcaneSurge")
            })
        })
    })
}

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

/// The largest scaled value of each buff row, by buff name, then stat,
/// then row (`mergeBuff` compares each mod after its skill's scale), with
/// the gem it came from first, the row, and the row's state condition.
type Merged = BTreeMap<
    (&'static str, &'static str, usize),
    (f64, Source, HitEffect, Option<(&'static str, bool)>),
>;

/// One curse of the skill list: its name, its source, and its scaled rows.
struct CurseApplied {
    name: &'static str,
    source: Source,
    rows: Vec<(f64, HitEffect)>,
}

/// What the skills of the set collect before they are applied.
#[derive(Default)]
struct Collected {
    merged: Merged,
    curses: Vec<CurseApplied>,
    /// The levels of the Summon Lightning Golem skills, each with its gem
    /// (Wrath, [`wrath`]).
    lightning_golems: Vec<(Source, i64)>,
    /// Whether a modelled Rallying Cry is in the list.
    rallying_cry: Option<Source>,
    /// Whether an Intimidating Cry's terms are in.
    intimidating: bool,
}

/// Adds the other gems' reservation, buffs, debuffs, and curses, and their
/// reasons (module docs). `main` is the main skill's group (0-based) and
/// active effect when the engine computes its hit, which acts through its
/// own path. `totals` are the reservation and effect modifiers
/// ([`skill_modifiers::Totals::read`], once every source of them is in).
pub fn gems(
    input: &CalcInput<'_>,
    main: Option<(usize, &MainSkill<'_>)>,
    levels: &GemLevels,
    totals: &Totals,
    ingest: &mut Ingest<'_>,
) {
    let build = input.build;
    let Ok(groups) = skill::active_groups(build) else {
        // No typed set: nothing; an untyped group is a reason of `collect`.
        return;
    };
    let config = super::collect::active_config(build);
    let main_group = skill::main_skill(build, input.data, levels)
        .ok()
        .map(|m| m.group - 1);
    let matches = sockets::socket_matches(build, &groups, input.data);
    let mut collected = Collected::default();
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
                        source: source.clone(),
                        what: "reservation",
                    });
                }
                // PoB's skill list holds the main skill too.
                guaranteed_chill(&s.types, source, ingest);
                socket_reasons(g, s, ingest);
                // Its supports' global effects (Arcane Surge's buff) act
                // as any skill's.
                supports_global(input, g, s, totals, config, &mut collected, ingest);
                continue;
            }
            one_skill(input, g, s, totals, config, &mut collected, ingest);
        }
    }
    apply(input, config, totals, collected, ingest);
}

/// Applies what the skills collected: buffs and debuffs in name order,
/// each stat in ID order, each row in order; then the curses; then Wrath.
fn apply(
    input: &CalcInput<'_>,
    config: Option<&ConfigSet>,
    totals: &Totals,
    collected: Collected,
    ingest: &mut Ingest<'_>,
) {
    let Collected {
        mut merged,
        curses,
        lightning_golems,
        rallying_cry,
        ..
    } = collected;
    // Lightning Golem's Wrath (module docs): merged as an aura of its name.
    wrath(
        input,
        config,
        totals,
        &lightning_golems,
        &mut merged,
        ingest,
    );
    for (value, source, (quantity, op, _, all, any), state) in merged.into_values() {
        let mut conditions = Conditions::none().with_all(all).with_any(any);
        if let Some((key, on)) = state {
            conditions = conditions.with_state(ConditionKey::new(key), on);
        }
        ingest.effect(Effect {
            quantity,
            op,
            value,
            conditions,
            source,
        });
    }
    // Curses: one limit, one hex (module docs).
    let names: BTreeSet<&str> = curses.iter().map(|c| c.name).collect();
    if names.len() > 1 || curses.len() > names.len() {
        for c in &curses {
            ingest.reason(UnsupportedReason::UnmodelledSkill {
                source: c.source.clone(),
                what: "curse-limit",
            });
        }
    } else {
        for c in curses {
            for (value, (quantity, op, _, all, any)) in c.rows {
                ingest.effect(Effect {
                    quantity,
                    op,
                    value,
                    conditions: Conditions::none().with_all(all).with_any(any),
                    source: c.source.clone(),
                });
            }
        }
    }
    // Rallying Cry's exerted attacks change a hit only with allies.
    if let Some(source) = rallying_cry {
        let allies =
            config.and_then(|set| last_input(set, NEARBY_ALLY_INPUT, ConfigInputKind::Input));
        let none = match allies {
            None => true,
            Some(ConfigValue::Number(n)) => n.value().0 <= 0.0,
            Some(_) => false,
        };
        if !none {
            ingest.reason(UnsupportedReason::UnmodelledSkill {
                source,
                what: "rallying-cry-allies",
            });
        }
    }
}

/// The skill's own scale terms: the sum of its gems' [`OWN_EFFECT_STATS`]
/// that its buff's kind reads.
fn own_effect(input: &CalcInput<'_>, s: &GroupSkill<'_>, aura: bool) -> f64 {
    let mut sum = 0.0;
    for r in std::iter::once(&s.active_gem).chain(&s.supports) {
        let instance = skill::instance(r.effect, r.level, r.quality, input.data);
        for (stat, value) in &instance.stats {
            if let Some(&(_, aura_only)) = OWN_EFFECT_STATS.iter().find(|(id, _)| id == stat)
                && (aura || !aura_only)
            {
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "stat values are small integers, exact as f64"
                )]
                let v = *value as f64;
                sum += v;
            }
        }
    }
    sum
}

/// One active skill of group `g` (module docs).
fn one_skill(
    input: &CalcInput<'_>,
    g: usize,
    s: &GroupSkill<'_>,
    totals: &Totals,
    config: Option<&ConfigSet>,
    collected: &mut Collected,
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
    // `enableGlobal<n>` is on: a skill whose flag is off does not exist.
    if s.global_off && kind.buff().is_some() {
        return;
    }
    guaranteed_chill(&s.types, active_source.clone(), ingest);
    let mut supported = true;
    for sup in &s.supports {
        if support(&sup.effect.id).is_none() {
            ingest.reason(UnsupportedReason::UnmodelledGem {
                source: source(sup.gem),
            });
            supported = false;
        }
    }
    if !supported {
        return;
    }
    let supports_rows = s
        .supports
        .iter()
        .any(|sup| support(&sup.effect.id).is_some_and(|t| !t.is_empty()));
    if kind != Kind::Inert || supports_rows || reserves(&s.types) {
        socket_reasons(g, s, ingest);
    }
    if s.active_gem.effect.id == WRATH_GOLEM {
        collected
            .lightning_golems
            .push((active_source.clone(), s.active_gem.level));
    }
    if matches!(
        kind,
        Kind::Warcry {
            name: "Rallying Cry",
            ..
        }
    ) && collected.rallying_cry.is_none()
    {
        collected.rallying_cry = Some(active_source.clone());
    }
    // Intimidating Cry's exerted attacks: the first one's terms (PoB
    // computes it once, `IntimidatingCryCalculated`).
    if matches!(
        kind,
        Kind::Warcry {
            name: "Intimidating Cry",
            ..
        }
    ) && !collected.intimidating
    {
        collected.intimidating = true;
        match intimidating_terms(input, s) {
            Some(terms) => {
                for (v, value) in terms {
                    ingest.effect(Effect {
                        quantity: Quantity::Poe1(Poe1Quantity::IntimidatingCry(v)),
                        op: Op::Base,
                        value,
                        conditions: Conditions::none(),
                        source: active_source.clone(),
                    });
                }
            }
            None => ingest.reason(UnsupportedReason::UnmodelledSkill {
                source: active_source.clone(),
                what: "warcry-uptime",
            }),
        }
    }
    match kind {
        Kind::Inert => {}
        Kind::Buff {
            name,
            effect,
            stats,
        } => {
            let aura = effect == EffectType::Aura;
            let scale = totals.buff_scale(aura, &s.types, own_effect(input, s, aura));
            add_rows(
                input,
                g,
                &s.active_gem,
                name,
                stats,
                scale,
                config,
                &mut collected.merged,
                ingest,
            );
        }
        Kind::Debuff { name, stats } => {
            // `DebuffEffect` is a reason ([`DEPENDS_ON`]): scale 1.
            add_rows(
                input,
                g,
                &s.active_gem,
                name,
                stats,
                1.0,
                config,
                &mut collected.merged,
                ingest,
            );
        }
        Kind::Warcry { name, stats } => {
            // Warcry power and uptime scale only rows of its own; none
            // reaches a slice field (a test).
            add_rows(
                input,
                g,
                &s.active_gem,
                name,
                stats,
                1.0,
                config,
                &mut collected.merged,
                ingest,
            );
        }
        Kind::Curse { name, stats } => {
            let scale = totals.curse_scale();
            let instance = skill::instance(
                s.active_gem.effect,
                s.active_gem.level,
                s.active_gem.quality,
                input.data,
            );
            if !instance.problems.is_empty() {
                for p in &instance.problems {
                    ingest.reason(UnsupportedReason::UnmodelledSkill {
                        source: active_source.clone(),
                        what: p.name(),
                    });
                }
            } else {
                let mut rows = Vec::new();
                for (stat, value) in &instance.stats {
                    let Some(&(_, BuffStat::Rows(effects))) =
                        stats.iter().find(|(id, _)| id == stat)
                    else {
                        continue;
                    };
                    #[expect(
                        clippy::cast_precision_loss,
                        reason = "stat values are small integers, exact as f64"
                    )]
                    let value = *value as f64;
                    for &row in effects {
                        let (quantity, op, divisor, ..) = row;
                        rows.push((
                            skill_modifiers::scale(quantity, op, value / divisor, scale),
                            row,
                        ));
                    }
                }
                collected.curses.push(CurseApplied {
                    name,
                    source: active_source.clone(),
                    rows,
                });
            }
        }
    }
    supports_global(input, g, s, totals, config, collected, ingest);
    reservation(s, &active_source, totals, ingest);
}

/// The global stats of the supports applied to a skill: a buff named by
/// the tag's `effectName` (every modelled support's rows name one: Arcane
/// Surge), scaled as a buff of the skill's types.
fn supports_global(
    input: &CalcInput<'_>,
    g: usize,
    s: &GroupSkill<'_>,
    totals: &Totals,
    config: Option<&ConfigSet>,
    collected: &mut Collected,
    ingest: &mut Ingest<'_>,
) {
    for sup in &s.supports {
        let Some(table) = support(&sup.effect.id) else {
            continue;
        };
        if table.is_empty() {
            continue;
        }
        let name = support_buff_name(&sup.effect.id);
        // Arcane Surge's mods are `unscalable` (`sup_int.lua` 229 to 233):
        // scale 1.
        let scale = if sup.effect.id == "SupportArcaneSurge" {
            1.0
        } else {
            totals.buff_scale(false, &s.types, 0.0)
        };
        add_rows(
            input,
            g,
            sup,
            name,
            table,
            scale,
            config,
            &mut collected.merged,
            ingest,
        );
    }
}

/// The buff name of a support's global effect (its tag's `effectName`).
fn support_buff_name(id: &str) -> &'static str {
    match id {
        "SupportArcaneSurge" => "Arcane Surge",
        "SupportInfusedChannelling" => "Infusion",
        "SupportMomentum" => "Momentum",
        "SupportSummonPhantasm" => "Phantasmal Might",
        _ => "support",
    }
}

/// One gem instance's rows of `table` into `merged` at `scale`.
#[expect(clippy::too_many_arguments, reason = "the merge's inputs")]
fn add_rows(
    input: &CalcInput<'_>,
    g: usize,
    r: &GemEffectRef<'_>,
    name: &'static str,
    table: &'static [(&'static str, BuffStat)],
    scale: f64,
    config: Option<&ConfigSet>,
    merged: &mut Merged,
    ingest: &mut Ingest<'_>,
) {
    let source = Source::Gem {
        group: g,
        gem: r.gem,
    };
    let instance = skill::instance(r.effect, r.level, r.quality, input.data);
    if !instance.problems.is_empty() {
        for p in &instance.problems {
            ingest.reason(UnsupportedReason::UnmodelledSkill {
                source: source.clone(),
                what: p.name(),
            });
        }
        return;
    }
    let stance_blood = !matches!(
        config.and_then(|set| last_input(set, STANCE_INPUT, ConfigInputKind::Input)),
        Some(ConfigValue::String(s)) if s == "SAND"
    );
    for (stat, value) in &instance.stats {
        let Some(&(id, b)) = table.iter().find(|(id, _)| id == stat) else {
            continue;
        };
        let (rows, state): (&[HitEffect], Option<(&'static str, bool)>) = match b {
            BuffStat::Rows(rows) => (rows, None),
            BuffStat::NoSlice(_) | BuffStat::Gated(_) => continue,
            BuffStat::WhenInput {
                input: check, rows, ..
            } => {
                let on = config.and_then(|set| last_input(set, check, ConfigInputKind::Input));
                match on {
                    None | Some(ConfigValue::Boolean(false)) => continue,
                    Some(ConfigValue::Boolean(true)) => (rows, None),
                    Some(_) => {
                        ingest.reason(UnsupportedReason::UnmodelledConfig {
                            input: check.to_owned(),
                        });
                        continue;
                    }
                }
            }
            BuffStat::BloodStance(rows) => {
                if !stance_blood {
                    continue;
                }
                (rows, None)
            }
            BuffStat::Threshold(rows) => (rows, Some((STAT_THRESHOLD, true))),
        };
        #[expect(
            clippy::cast_precision_loss,
            reason = "stat values are small integers, exact as f64"
        )]
        let value = *value as f64;
        for (i, &row) in rows.iter().enumerate() {
            let (quantity, op, divisor, ..) = row;
            let scaled = skill_modifiers::scale(quantity, op, value / divisor, scale);
            let entry = merged
                .entry((name, id, i))
                .or_insert((scaled, source.clone(), row, state));
            if scaled > entry.0 {
                *entry = (scaled, source.clone(), row, state);
            }
        }
    }
}

/// Lightning Golem's Wrath (module docs; `CalcPerform.lua` 2782 to 2812):
/// when the config enables it, the first Summon Lightning Golem's minion
/// casts it at the minion's level, the golem gem level's requirement,
/// which the gem's `base_display_minion_actor_level` stat equals in the
/// data (PoB: `activeEffect.grantedEffectLevel.levelRequirement`,
/// `CalcActiveSkill.lua` 707 to 711). Its values are effectiveness-scaled
/// at that level (`CalcTools.lua` 198 to 205: `round(((base + incremental
/// x (L - 1)) x baseEffectiveness x (1 + incrementalEffectiveness) ^ (L -
/// 1)) x value)`), and its aura applies at the minion skill's aura effect
/// (none modelled: 1).
fn wrath(
    input: &CalcInput<'_>,
    config: Option<&ConfigSet>,
    _totals: &Totals,
    golems: &[(Source, i64)],
    merged: &mut Merged,
    ingest: &mut Ingest<'_>,
) {
    let on = config.and_then(|set| last_input(set, WRATH_INPUT, ConfigInputKind::Input));
    match on {
        None | Some(ConfigValue::Boolean(false)) => return,
        Some(ConfigValue::Boolean(true)) => {}
        Some(_) => {
            ingest.reason(UnsupportedReason::UnmodelledConfig {
                input: WRATH_INPUT.to_owned(),
            });
            return;
        }
    }
    let Some((source, level)) = golems.first() else {
        return;
    };
    match wrath_rows(input.data, *level) {
        Some(rows) => {
            for (stat, value, row) in rows {
                let entry = merged.entry((WRATH_NAME, stat, 0)).or_insert((
                    value,
                    source.clone(),
                    row,
                    None,
                ));
                if value > entry.0 {
                    *entry = (value, source.clone(), row, None);
                }
            }
        }
        None => ingest.reason(UnsupportedReason::UnmodelledSkill {
            source: source.clone(),
            what: "wrath",
        }),
    }
}

/// Wrath's rows at a golem gem `level` (see [`wrath`]); `None` when the
/// data lacks what they need.
#[must_use]
pub fn wrath_rows(data: &dyn CalcData, level: i64) -> Option<Vec<(&'static str, f64, HitEffect)>> {
    let golem = data.granted_effect(WRATH_GOLEM)?;
    let golem_instance = skill::instance(golem, level, 0, data);
    let minion_level = golem_instance
        .stats
        .iter()
        .find(|(s, _)| s == "base_display_minion_actor_level")
        .map(|(_, v)| *v)?
        .clamp(1, 100);
    let effect = data.granted_effect(WRATH_EFFECT)?;
    let set = data.stat_set(effect.stat_sets.first()?)?;
    // One level (its requirement 1): PoB picks the last level whose
    // requirement the minion meets; several levels are not modelled.
    let [level_row] = set.levels.as_slice() else {
        return None;
    };
    let (base, base_div) = data.game_constant("SkillDamageBaseEffectiveness")?;
    let (inc, inc_div) = data.game_constant("SkillDamageIncrementalEffectiveness")?;
    #[expect(
        clippy::cast_precision_loss,
        reason = "constants and levels are small integers, exact as f64"
    )]
    let (base, inc, l) = (
        base as f64 / base_div as f64,
        inc as f64 / inc_div as f64,
        (minion_level - 1) as f64,
    );
    let base_effectiveness = f64::from(set.base_effectiveness);
    let incremental = f64::from(set.incremental_effectiveness);
    let effectiveness =
        (base + inc * l) * base_effectiveness * crate::math::pow(1.0 + incremental, l);
    let mut out = Vec::new();
    for (stat, row) in WRATH_STATS {
        let f = row_float(row_stat(level_row, stat)?)?;
        out.push((*stat, crate::math::round_half_up(effectiveness * f), *row));
    }
    Some(out)
}

/// The float stat `stat` of Wrath's level (effectiveness interpolation).
fn row_stat<'a>(
    row: &'a crate::input::StatSetLevel,
    stat: &str,
) -> Option<&'a crate::input::FloatStat> {
    row.float_stats
        .iter()
        .find(|f| f.stat == stat && f.interpolation == 3)
}

/// A float stat's stored value as PoB reads it (the game's f32, widened).
fn row_float(f: &crate::input::FloatStat) -> Option<f64> {
    Some(f64::from(f.value))
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
fn reserves(types: &BTreeSet<String>) -> bool {
    types.contains("HasReservation") && !types.contains("ReservationBecomesCost")
}

/// The skill type that gives a guaranteed chill whatever the skill's
/// modifiers (PoB's `ChillingArea`, `CalcPerform.lua` 1393).
pub const CHILLING_AREA: &str = "ChillingArea";

/// The skill type that gives one unless the skill cannot chill (PoB's
/// `NonHitChill` without the `CannotChill` flag).
pub const NON_HIT_CHILL: &str = "NonHitChill";

/// A skill's guaranteed chill (P3-8m; PoB's `ChillOverride`,
/// `CalcPerform.lua` 1393 to 1399): a `ChillingArea` skill adds
/// [`Poe1Quantity::SkillChill`] at the default chill, which
/// [`super::damage_taken::chill`] scales by the character's chill effect
/// (the skill's own gems carry none: the `data-parity` table
/// `gem_effects` refuses `EnemyChillEffect` in a modelled skill or
/// support). A `NonHitChill` skill without `ChillingArea` chills unless
/// it cannot (`CannotChill`, from modifiers the engine does not read): a
/// reason. Every modelled `NonHitChill` skill is also `ChillingArea` (a
/// test).
fn guaranteed_chill(types: &BTreeSet<String>, source: Source, ingest: &mut Ingest<'_>) {
    if types.contains(CHILLING_AREA) {
        ingest.effect(Effect {
            quantity: Quantity::Poe1(Poe1Quantity::SkillChill),
            op: Op::Base,
            value: super::damage_taken::CHILL_DEFAULT,
            conditions: Conditions::none(),
            source,
        });
    } else if types.contains(NON_HIT_CHILL) {
        ingest.reason(UnsupportedReason::UnmodelledSkill {
            source,
            what: "guaranteed-chill",
        });
    }
}

/// The skill's reservation (module docs), one effect per pool and kind.
fn reservation(s: &GroupSkill<'_>, source: &Source, totals: &Totals, ingest: &mut Ingest<'_>) {
    if !reserves(&s.types) {
        return;
    }
    // A support that removes the reservation of the skill's type.
    let removed = s.supports.iter().any(|sup| {
        NO_RESERVATION_SUPPORTS
            .iter()
            .any(|(id, ty)| *id == sup.effect.id && s.types.contains(*ty))
    });
    if removed {
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
        let terms = totals.reservation(pool);
        let amounts = if r.percent {
            skill_modifiers::reserved(0, r.amount, mult, terms)
        } else {
            skill_modifiers::reserved(r.amount, 0, mult, terms)
        };
        let Some((flat, percent)) = amounts else {
            // An efficiency of -100: PoB divides by 0.
            ingest.reason(UnsupportedReason::UnmodelledSkill {
                source: source.clone(),
                what: "reservation-efficiency",
            });
            continue;
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
        assert!(sorted_unique(GUARDED_WORDS));
        for (id, kind) in SKILLS {
            let ids: Vec<_> = kind.stats().iter().map(|(s, _)| *s).collect();
            assert!(sorted_unique(&ids), "{id}");
        }
        for (id, stats) in SUPPORTS {
            let ids: Vec<_> = stats.iter().map(|(s, _)| *s).collect();
            assert!(sorted_unique(&ids), "{id}");
        }
        assert_eq!(
            kind("Precision").map(|k| matches!(k, Kind::Buff { .. })),
            Some(true)
        );
        assert_eq!(kind("GroundSlam"), Some(Kind::Inert));
        assert_eq!(kind("Fireball"), None);
        assert!(support("SupportFasterAttacks").is_some_and(<[_]>::is_empty));
        assert!(support("SupportArcaneSurge").is_some_and(|t| !t.is_empty()));
        assert_eq!(support("SupportFireDamage"), None);
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
            .flat_map(|(_, k)| k.stats().iter())
            .chain(SUPPORTS.iter().flat_map(|(_, s)| s.iter()));
        for (stat, b) in rows {
            let effects: &[HitEffect] = match b {
                BuffStat::Rows(e)
                | BuffStat::WhenInput { rows: e, .. }
                | BuffStat::BloodStance(e)
                | BuffStat::Threshold(e) => e,
                BuffStat::NoSlice(_) | BuffStat::Gated(_) => &[],
            };
            for e in effects {
                assert!(!early.contains(&e.0), "{stat}");
            }
        }
    }

    /// A debuff's or a curse's rows act on the enemy, a buff's and a
    /// warcry's on the character.
    #[test]
    fn debuff_and_curse_rows_should_act_on_the_enemy_alone() {
        let enemy = |q: Quantity| {
            matches!(
                q,
                Quantity::Poe1(
                    Poe1Quantity::EnemyResistance(_)
                        | Poe1Quantity::SkillExposure(_)
                        | Poe1Quantity::EnemyTypeDamageTaken(_)
                        | Poe1Quantity::EnemyDamageTaken
                )
            )
        };
        for (id, k) in SKILLS {
            for (stat, b) in k.stats() {
                let BuffStat::Rows(rows) = b else { continue };
                for row in *rows {
                    match k {
                        Kind::Debuff { .. } | Kind::Curse { .. } => {
                            assert!(enemy(row.0), "{id} {stat}");
                        }
                        Kind::Buff { .. } | Kind::Warcry { .. } => {
                            assert!(!enemy(row.0), "{id} {stat}");
                        }
                        Kind::Inert => {}
                    }
                }
            }
            // Warcries: no row reaches a slice field (module docs).
            if let Kind::Warcry { stats, .. } = k {
                assert!(
                    stats.iter().all(|(_, b)| matches!(b, BuffStat::NoSlice(_))),
                    "{id}"
                );
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
        // Checked rows: curses on you (`curse_effect_on_self_+%`, P3-8n) are
        // not the curses this module casts.
        let checked = ["curse_effect_on_self_+%"];
        for id in ids {
            for word in GUARDED_WORDS {
                assert!(
                    !id.contains(word) || checked.contains(&id),
                    "{id}: revisit poe1::gems"
                );
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
        // The two mapped effect stats (`skill_modifiers`) are no word of
        // the guard but for their own.
        for (stat, ..) in skill_modifiers::STATS {
            assert!(!DEPENDS_ON.contains(&stat), "{stat}");
        }
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
        assert_eq!(
            Quantity::Poe1(Poe1Quantity::SkillExposure(DamageType::Cold)).to_string(),
            "poe1.skill-exposure.cold"
        );
        assert_eq!(
            Quantity::Poe1(Poe1Quantity::GainAs(DamageType::Physical, DamageType::Fire))
                .to_string(),
            "poe1.gain-as.physical.fire"
        );
        assert_eq!(Kind::Inert.buff(), None);
        assert_eq!(
            kind("Frostbite").and_then(|k| k.buff()),
            Some(("Curse", "Frostbite"))
        );
    }

    #[test]
    fn this_module_should_read_its_inputs() {
        assert!(is_input(WRATH_INPUT));
        assert!(is_input("flameWallAddedDamage"));
        assert!(is_input(STANCE_INPUT));
        assert!(!is_input("buffOnslaught"));
    }
}
