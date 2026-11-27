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
//!   A guard (`EffectType::Guard`, P6-9e) is a buff that takes PoB's guard
//!   slots (`CalcPerform.lua` 3256 to 3275): Vaal Molten Shell alone, else
//!   every Vaal guard and one other; with two or more other guards PoB's
//!   pick follows its table order, so none applies and each is an
//!   `unmodelled-skill` `guard-slot` reason. A buff stat only minions get
//!   (`buffNotPlayer`, the offerings: [`BuffStat::MinionOnly`]) acts in
//!   [`minion_buffs`] alone, and on you too while "Your Offering Skills
//!   also affect you" (P6-11gg, [`skill_modifiers::Totals::offering_on_you`]).
//! - [`Kind::Debuff`]: its rows on the enemy, merged per name (Frost Bomb's
//!   exposure, [`Poe1Quantity::SkillExposure`], which [`super::exposure`]
//!   compares with the others).
//! - [`Kind::Curse`]: its rows on the enemy, scaled by curse effect (the
//!   character's and the gem's own `curse_effect_+%`, P6-9b). The curses
//!   take PoB's slots ([`curse_slots`], `CalcPerform.lua` 3155 to 3212):
//!   in the skill list's order, each takes the first free slot of the
//!   limit (`EnemyCurseLimit`: 1, plus [`Poe1Quantity::CurseLimit`]), or
//!   replaces a curse of its name or of a lower priority
//!   ([`curse_priority`], PoB's `determineCursePriority`). A mark (a curse
//!   skill typed `Mark`, P6-9e) takes a slot as a hex does, but once a
//!   mark holds a slot no other mark takes one (PoB: "Currently assume
//!   only 1 mark is possible"); a mark that loses its slot frees the rule.
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

use loom_domain::{Build, ConfigInputKind, ConfigSet, ConfigValue, SocketGroup};

use super::config::last_input;
use super::gem_levels::GemLevels;
use super::mapping::HitEffect;
use super::skill::{self, GemEffectRef, GroupSkill, MainSkill};
use super::skill_modifiers::{self, Totals};
use super::sockets;
use super::{DegenOf, LeechSource, Poe1Quantity, Pool, ReservationKind, TakenOf, TakenWhen};
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
    /// A guard (`buff.type == "Guard"`, P6-9e): a buff in PoB's guard slots
    /// (module docs).
    Guard,
}

impl EffectType {
    /// PoB's name.
    #[must_use]
    pub const fn pob(self) -> &'static str {
        match self {
            Self::Aura => "Aura",
            Self::Buff => "Buff",
            Self::Guard => "Guard",
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
    /// Reaches only the enemy's hit on the character (P6-10b part 12), in a
    /// way the engine does not model: a marker
    /// ([`Poe1Quantity::EnemyHitReason`]) those fields read as an
    /// `unmapped-stat` reason when the stat is not 0. As `NoSlice` for
    /// everything else.
    Defence(&'static str),
    /// Rows applied only while the config count `input` (else its saved
    /// placeholder, else `default`, as [`Self::PerCount`]) is at least
    /// `at`, unscaled (the agent list, 2026-11-27: Sigil of Power's less
    /// enemy damage at its maximum stages; PoB's `MultiplierThreshold` tag
    /// with `actor = "enemy"`, which on the enemy reads the character's
    /// stage multiplier, against `thresholdVar`).
    AtCount {
        /// The config count.
        input: &'static str,
        /// The count without the input.
        default: u32,
        /// The count from which the rows apply.
        at: u32,
        /// PoB's multiplier name.
        var: &'static str,
        /// PoB's threshold multiplier name.
        threshold_var: &'static str,
        /// The rows.
        rows: &'static [HitEffect],
    },
    /// Reaches a slice field in a way the engine does not model: an
    /// `unmodelled-skill` reason with this name when the stat is not 0
    /// (P6-10b: Enduring Cry's regeneration per warcry power).
    Reason(&'static str),
    /// As [`Self::Reason`], only while the config check `input` is on: a
    /// value PoB scales by something only that check makes non-zero
    /// (P6-11dv: Immortal Call's per removable endurance charge, 0 without
    /// endurance charges in use).
    ReasonWhenInput {
        /// The config check.
        input: &'static str,
        /// The reason's name.
        what: &'static str,
    },
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
    /// Applies when the config check `input` is on, or with `neg` when it
    /// is off: it sets the enemy's condition `var` (PoB's `ActorCondition`
    /// tag on the enemy, negated with `neg`; Frost Shield's share with the
    /// enemy in or outside its area; P6-11ho).
    EnemyCheck {
        /// The config check.
        input: &'static str,
        /// The enemy's condition.
        var: &'static str,
        /// Whether the rows apply with the condition off.
        neg: bool,
        /// The rows.
        rows: &'static [HitEffect],
    },
    /// Applies only when the config check `input` is on: it sets the
    /// condition PoB's `modCond` names, which nothing else sets
    /// (`CalcActiveSkill.lua` 806 drops a mod whose `modCond` fails;
    /// Infused Channelling's `InfusionActive` from
    /// `infusedChannellingInfusion`, `ConfigOptions.lua` 495; P6-11aa).
    CheckModCond {
        /// The config check.
        input: &'static str,
        /// PoB's `modCond`.
        condition: &'static str,
        /// The rows.
        rows: &'static [HitEffect],
    },
    /// Applies in Blood Stance (PoB's `effectCond = "BloodStance"`, on
    /// unless the config's stance is Sand: `CalcSetup.lua` 538,
    /// `ConfigOptions.lua` 643).
    BloodStance(&'static [HitEffect]),
    /// Rows PoB marks `unscalable` on a skill whose other rows scale (a
    /// guard's absorption, Molten Shell's beside its armour): scale 1
    /// (P6-11ae).
    Unscalable(&'static [HitEffect]),
    /// A warcry's rows per warcry power (P6-11bs; PoB's `GlobalEffect` tag
    /// with `div` and `limit`, `CalcPerform.lua` 2327 to 2331): scaled by
    /// `floor(min(power, limit) / div)` with the warcry's buff effect and
    /// uptime ([`warcry_terms`]).
    PerPower {
        /// Power per step.
        div: f64,
        /// The power counted at most.
        limit: f64,
        /// The rows.
        rows: &'static [HitEffect],
    },
    /// Applies in Sand Stance (PoB's `effectCond = "SandStance"`, set when
    /// the config's stance is Sand: `ConfigOptions.lua` 643 to 646) as a
    /// `Buff` on the character, not on minions: scaled by the buff effect
    /// (`CalcPerform.lua` 2266 to 2276), not the aura effect, even on an
    /// aura skill (Flesh and Stone; P6-11x).
    SandStance(&'static [HitEffect]),
    /// Applies per the config count `input` (PoB's `Multiplier` tag on the
    /// mod, `var` a multiplier only that input sets): the scaled value
    /// times the count, nothing without it (Momentum's Swiftness per stack
    /// removed, `MomentumSwiftnessStacks`, `ConfigOptions.lua` 543; P6-11y).
    PerCount {
        /// The config count.
        input: &'static str,
        /// PoB's multiplier name.
        var: &'static str,
        /// The count's limit, if any (PoB's `limitVar`: Frost Shield's
        /// stages at most its `FrostShieldMaxStages`; P6-11ho).
        limit: Option<u32>,
        /// The count PoB applies without the input (the agent list,
        /// 2026-11-27: an option's `defaultPlaceholderState`, which
        /// `BuildModList` applies when the input is absent; the saved
        /// placeholder first, `ConfigTab.lua` 1083 to 1092). `None`: no
        /// input, no rows.
        default: Option<u32>,
        /// The rows.
        rows: &'static [HitEffect],
    },
    /// Applies per stack, at most `limit` (PoB's `effectStackVar` and
    /// `effectStackLimit` on the `GlobalEffect` tag, the stack count a
    /// multiplier only the config count `input` sets; P6-11fn, Ensnaring
    /// Arrow's `EnsnareStackCount`): the scaled value times `min(count,
    /// limit)`, nothing without stacks.
    PerStack {
        /// The config count.
        input: &'static str,
        /// PoB's stack variable.
        var: &'static str,
        /// PoB's stack limit.
        limit: u32,
        /// The rows.
        rows: &'static [HitEffect],
    },
    /// Applies at the skill's maximum stages (PoB's `effectCond` on a
    /// condition the skill's own `MultiplierThreshold` on its
    /// `StageAfterFirst` sets; P6-11gb, Scorching Ray): the gem's part
    /// `part` or above (PoB's `CalcPerform.lua` 2241 to 2245 set the
    /// maximum there), or a stage count (`skillStageCount`, PoB's default
    /// the maximum, `CalcActiveSkill.lua` 638) of at least `stages`.
    AtMaxStages {
        /// The part at the maximum stages (1-based).
        part: usize,
        /// The maximum stages.
        stages: u32,
        /// The skill's condition (PoB's `effectCond`).
        condition: &'static str,
        /// The rows.
        rows: &'static [HitEffect],
    },
    /// Applies while unreserved life is at least 2 and mana at least 1
    /// (PoB's `StatThreshold` tags, Righteous Fire's): the rows carry the
    /// state [`STAT_THRESHOLD`], which the hit sets from those fields.
    Threshold(&'static [HitEffect]),
    /// Applies while unreserved life is at least 2 (PoB's one
    /// `StatThreshold` tag on Righteous Fire's own buff, Phase 6): the rows
    /// carry the state [`LIFE_THRESHOLD`], which the hit sets.
    LifeThreshold(&'static [HitEffect]),
    /// Applies to minions alone (the skill's `buffNotPlayer` with
    /// `buffMinions`: the offerings, P6-9e): [`minion_buffs`] adds the
    /// rows, the character's buffs skip them.
    MinionOnly(&'static [HitEffect]),
    /// Applies while a named condition holds (PoB's `Condition` tag on the
    /// mod; Punishment's enemy `LowLife`, P6-9e): the rows carry the state
    /// `state`, which [`super::conditions`] settles from the config.
    WhenState {
        /// The state's key.
        state: &'static str,
        /// PoB's condition name (`var`).
        var: &'static str,
        /// The rows.
        rows: &'static [HitEffect],
    },
    /// Applies only when the config list `input` is `value` (`equal`) or
    /// is not (PoB's `modCond`, a condition the config alone sets: Pride's
    /// `PrideMaxEffect` from `prideEffect`, `ConfigOptions.lua` 566, and
    /// `PrideMinEffect` otherwise, `CalcSetup.lua` 539; `CalcActiveSkill.lua`
    /// 806 drops a mod whose `modCond` fails; P6-9e).
    ConfigIs {
        /// The config list.
        input: &'static str,
        /// The value it is compared with.
        value: &'static str,
        /// Whether the rows apply when the input equals `value`.
        equal: bool,
        /// PoB's `modCond`.
        condition: &'static str,
        /// The rows.
        rows: &'static [HitEffect],
    },
}

/// The state of [`BuffStat::Threshold`] rows.
pub const STAT_THRESHOLD: &str = "LifeUnreservedAtLeast2AndManaAtLeast1";

/// The state of [`BuffStat::LifeThreshold`] rows.
pub const LIFE_THRESHOLD: &str = "LifeUnreservedAtLeast2";

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
    ("Absolution", Kind::Inert),
    ("AbsolutionAltX", Kind::Inert),
    // P6-11fd: Ancestral Cry (`act_str.lua` 281): elemental resistances per
    // 5 warcry power up to 30 (`ElementalResist` BASE) and their maxima per
    // 10 power up to 30 (`ElementalResistMax` BASE), `Warcry` effects. Its
    // exerted attacks strike more enemies, which no slice field reads.
    (
        "AncestralCry",
        Kind::Warcry {
            name: "Ancestral Cry",
            stats: &[
                (
                    "ancestral_cry_elemental_resist_%_per_5_power_up_to_cap",
                    BuffStat::PerPower {
                        div: 5.0,
                        limit: 30.0,
                        rows: &[
                            (
                                Quantity::Resistance(DamageType::Fire),
                                Op::Base,
                                1.0,
                                NONE,
                                NONE,
                            ),
                            (
                                Quantity::Resistance(DamageType::Cold),
                                Op::Base,
                                1.0,
                                NONE,
                                NONE,
                            ),
                            (
                                Quantity::Resistance(DamageType::Lightning),
                                Op::Base,
                                1.0,
                                NONE,
                                NONE,
                            ),
                        ],
                    },
                ),
                (
                    "ancestral_cry_maximum_elemental_resist_%_per_10_power_up_to_cap",
                    BuffStat::PerPower {
                        div: 10.0,
                        limit: 30.0,
                        rows: &[
                            (
                                Quantity::MaximumResistance(DamageType::Fire),
                                Op::Base,
                                1.0,
                                NONE,
                                NONE,
                            ),
                            (
                                Quantity::MaximumResistance(DamageType::Cold),
                                Op::Base,
                                1.0,
                                NONE,
                                NONE,
                            ),
                            (
                                Quantity::MaximumResistance(DamageType::Lightning),
                                Op::Base,
                                1.0,
                                NONE,
                                NONE,
                            ),
                        ],
                    },
                ),
                (
                    "warcry_count_power_from_enemies",
                    BuffStat::NoSlice("warcry-power"),
                ),
            ],
        },
    ),
    (
        "Anger",
        Kind::Buff {
            name: "Anger",
            effect: EffectType::Aura,
            stats: &[
                (
                    "attack_maximum_added_fire_damage",
                    BuffStat::Rows(&[(
                        added(DamageType::Fire, Bound::Max),
                        Op::Base,
                        1.0,
                        Flags::ATTACK,
                        NONE,
                    )]),
                ),
                (
                    "attack_minimum_added_fire_damage",
                    BuffStat::Rows(&[(
                        added(DamageType::Fire, Bound::Min),
                        Op::Base,
                        1.0,
                        Flags::ATTACK,
                        NONE,
                    )]),
                ),
                (
                    "spell_maximum_added_fire_damage",
                    BuffStat::Rows(&[(
                        added(DamageType::Fire, Bound::Max),
                        Op::Base,
                        1.0,
                        Flags::SPELL,
                        NONE,
                    )]),
                ),
                (
                    "spell_minimum_added_fire_damage",
                    BuffStat::Rows(&[(
                        added(DamageType::Fire, Bound::Min),
                        Op::Base,
                        1.0,
                        Flags::SPELL,
                        NONE,
                    )]),
                ),
            ],
        },
    ),
    ("AnimateGuardian", Kind::Inert),
    ("AnimateGuardianAltX", Kind::Inert),
    ("AnimateWeapon", Kind::Inert),
    ("AnimateWeaponAltY", Kind::Inert),
    // P6-8a: Cast On Critical Strike's second spell
    ("Arc", Kind::Inert),
    (
        "ArcticArmour",
        Kind::Buff {
            name: "Arctic Armour",
            effect: EffectType::Buff,
            stats: &[
                ("base_immune_to_freeze", BuffStat::NoSlice("freeze")),
                // P6-11z: `<Type>DamageTakenWhenHit` MORE while
                // `Stationary` (`act_dex.lua` 588 to 593).
                (
                    "new_arctic_armour_fire_damage_taken_when_hit_+%_final",
                    BuffStat::WhenState {
                        state: super::conditions::STATIONARY,
                        var: "Stationary",
                        rows: &[(
                            Quantity::Poe1(Poe1Quantity::SelfDamageTaken(
                                TakenWhen::Hit,
                                TakenOf::Type(DamageType::Fire),
                            )),
                            Op::More,
                            1.0,
                            NONE,
                            NONE,
                        )],
                    },
                ),
                (
                    "new_arctic_armour_physical_damage_taken_when_hit_+%_final",
                    BuffStat::WhenState {
                        state: super::conditions::STATIONARY,
                        var: "Stationary",
                        rows: &[(
                            Quantity::Poe1(Poe1Quantity::SelfDamageTaken(
                                TakenWhen::Hit,
                                TakenOf::Type(DamageType::Physical),
                            )),
                            Op::More,
                            1.0,
                            NONE,
                            NONE,
                        )],
                    },
                ),
            ],
        },
    ),
    // P6-11fs: no global effect in PoB (`Data/Skills`).
    ("ArmageddonBrand", Kind::Inert),
    // Assassin's Mark (P6-9e): PoB's `SelfCritChance` and
    // `SelfCritMultiplier` BASE on the enemy, which a hit adds to its crit
    // chance's base and its extra damage (`CalcOffence.lua` 3015, 3125): the
    // character's sums, as Despair's added damage.
    (
        "AssassinsMark",
        Kind::Curse {
            name: "Assassin's Mark",
            stats: &[
                (
                    "enemy_additional_critical_strike_chance_against_self",
                    BuffStat::Rows(&[(Quantity::CritChance, Op::Base, 100.0, NONE, NONE)]),
                ),
                (
                    "enemy_additional_critical_strike_multiplier_against_self",
                    BuffStat::Rows(&[(Quantity::CritMultiplier, Op::Base, 1.0, NONE, NONE)]),
                ),
                ("life_granted_when_killed", BuffStat::NoSlice("on-kill")),
                ("mana_granted_when_killed", BuffStat::NoSlice("on-kill")),
            ],
        },
    ),
    ("Autoexertion", Kind::Inert),
    // Automation reserves and triggers the skill it supports
    // (`SupportAutomation`); nothing of its own reaches the character
    // (`Data/Skills/act_int.lua` 417).
    ("Automation", Kind::Inert),
    // P6-11dx: Battlemage's Cry (`act_int.lua`): base critical strike chance
    // per 5 warcry power up to 25 (`CritChance` BASE, `div = 100`, a
    // `Warcry` effect). Its exerted attacks (the skill's own stat, not
    // global) trigger the spells it supports, which no slice field reads.
    (
        "BattlemagesCry",
        Kind::Warcry {
            name: "Battlemage's Cry",
            stats: &[
                (
                    "divine_cry_additional_base_critical_strike_chance_per_5_power_up_to_cap",
                    BuffStat::PerPower {
                        div: 5.0,
                        limit: 25.0,
                        rows: &[(Quantity::CritChance, Op::Base, 100.0, NONE, NONE)],
                    },
                ),
                (
                    "warcry_count_power_from_enemies",
                    BuffStat::NoSlice("warcry-power"),
                ),
            ],
        },
    ),
    // P6-11fj: Bear Trap's debuff (`act_dex.lua`, `BearTrap`): the enemy's
    // `TrapMineDamageTaken` INC (`GlobalEffect`, `Debuff`), which only a
    // trap or mine skill's hit adds to its `takenInc` ([`super::hit`]).
    (
        "BearTrap",
        Kind::Debuff {
            name: "Bear Trap",
            stats: &[(
                "bear_trap_damage_taken_+%_from_traps_and_mines",
                BuffStat::Rows(&[(
                    Quantity::Poe1(Poe1Quantity::EnemyTrapMineDamageTaken),
                    Op::Increased,
                    1.0,
                    NONE,
                    NONE,
                )]),
            )],
        },
    ),
    // P6-11dt: Berserk's buff (`act_str.lua` 934): `RageEffect` INC, a
    // `GlobalEffect` buff marked unscalable, read by the rage
    // (`poe1::stacks`). Its minimum rage and rage loss PoB does not map.
    (
        "Berserk",
        Kind::Buff {
            name: "Berserk",
            effect: EffectType::Buff,
            stats: &[(
                "berserk_rage_effect_+%",
                BuffStat::Unscalable(&[(
                    Quantity::Poe1(Poe1Quantity::Stack(
                        super::stacks::StackKind::Rage,
                        super::stacks::StackValue::Effect,
                    )),
                    Op::Increased,
                    1.0,
                    NONE,
                    NONE,
                )]),
            )],
        },
    ),
    ("BlastRain", Kind::Inert),
    // P6-11fr: the base skills a Vaal gem also grants, with no global
    // effect in PoB (Blight, Earthquake); Flicker Strike's buff below.
    ("Blight", Kind::Inert),
    // P6-11dw: Blood and Sand (`act_str.lua` 1227): melee skills' more area
    // damage in each stance (`Damage` MORE with `ModFlag.Area` and the
    // `Melee` skill type, the stance PoB's `effectCond`); their area of
    // effect reaches no slice field. Its reservation is the skill's.
    (
        "BloodAndSand",
        Kind::Buff {
            name: "Blood and Sand",
            effect: EffectType::Buff,
            stats: &[
                (
                    "blood_sand_stance_melee_skills_area_damage_+%_final_in_blood_stance",
                    BuffStat::BloodStance(&[(
                        Quantity::Damage,
                        Op::More,
                        1.0,
                        Flags::AREA.union(Flags::SKILL_MELEE),
                        NONE,
                    )]),
                ),
                (
                    "blood_sand_stance_melee_skills_area_damage_+%_final_in_sand_stance",
                    BuffStat::SandStance(&[(
                        Quantity::Damage,
                        Op::More,
                        1.0,
                        Flags::AREA.union(Flags::SKILL_MELEE),
                        NONE,
                    )]),
                ),
                (
                    "blood_sand_stance_melee_skills_area_of_effect_+%_final_in_blood_stance",
                    BuffStat::NoSlice("area"),
                ),
                (
                    "blood_sand_stance_melee_skills_area_of_effect_+%_final_in_sand_stance",
                    BuffStat::NoSlice("area"),
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
                // Open items S7, part 8: `PhysicalDegen` BASE per point of
                // energy shield and of life, the stat over 6000
                // (`act_dex.lua` 3315 to 3322), read by `poe1::degen`.
                (
                    "base_physical_damage_%_of_maximum_energy_shield_to_deal_per_minute",
                    BuffStat::Rows(&[(
                        Quantity::Poe1(Poe1Quantity::Degen(
                            DamageType::Physical,
                            DegenOf::EnergyShield,
                        )),
                        Op::Base,
                        6000.0,
                        NONE,
                        NONE,
                    )]),
                ),
                (
                    "base_physical_damage_%_of_maximum_life_to_deal_per_minute",
                    BuffStat::Rows(&[(
                        Quantity::Poe1(Poe1Quantity::Degen(DamageType::Physical, DegenOf::Life)),
                        Op::Base,
                        6000.0,
                        NONE,
                        NONE,
                    )]),
                ),
                // P6-10b part 9: `PhysicalDamageLifeLeech` BASE with attacks,
                // the hit's leech (`poe1::leech`).
                (
                    "life_leech_from_physical_attack_damage_permyriad",
                    BuffStat::Rows(&[(
                        Quantity::Poe1(Poe1Quantity::DamageLeech(
                            Pool::Life,
                            LeechSource::Type(DamageType::Physical),
                        )),
                        Op::Base,
                        100.0,
                        Flags::ATTACK,
                        NONE,
                    )]),
                ),
            ],
        },
    ),
    // Open items S3, part 7: a skill an item grants and triggers (PoB's
    // export: no global effect).
    ("BoneNova", Kind::Inert),
    // Bone Offering (P6-9e): its buff goes to minions alone (`buffNotPlayer`).
    (
        "BoneOffering",
        Kind::Buff {
            name: "Bone Offering",
            effect: EffectType::Buff,
            stats: &[
                (
                    "base_spell_block_%",
                    BuffStat::MinionOnly(&[(
                        Quantity::Poe1(Poe1Quantity::SpellBlockChance),
                        Op::Base,
                        1.0,
                        NONE,
                        NONE,
                    )]),
                ),
                (
                    "monster_base_block_%",
                    BuffStat::MinionOnly(&[(Quantity::BlockChance, Op::Base, 1.0, NONE, NONE)]),
                ),
            ],
        },
    ),
    // Open items S3: Arcanist Brand: no global effect in PoB's data
    // (`act_int.lua` 545, its base mod the skill's own `debuff`); the spells
    // it triggers read its activation in its own context (`poe1::triggers`).
    ("BrandSupport", Kind::Inert),
    ("BurningArrow", Kind::Inert),
    (
        "Clarity",
        Kind::Buff {
            name: "Clarity",
            effect: EffectType::Aura,
            stats: &[(
                "base_mana_regeneration_rate_per_minute",
                BuffStat::Rows(&[(
                    Quantity::Poe1(Poe1Quantity::Regen(Pool::Mana)),
                    Op::Base,
                    60.0,
                    NONE,
                    NONE,
                )]),
            )],
        },
    ),
    ("Combust", Kind::Inert),
    (
        "Conductivity",
        Kind::Curse {
            name: "Conductivity",
            stats: &[
                (
                    "base_lightning_damage_resistance_%",
                    BuffStat::Rows(&[(
                        enemy_res(DamageType::Lightning),
                        Op::Base,
                        1.0,
                        NONE,
                        NONE,
                    )]),
                ),
                (
                    "base_self_shock_duration_-%",
                    BuffStat::NoSlice("shock-duration"),
                ),
                (
                    "chance_to_be_shocked_%",
                    BuffStat::NoSlice("ailment-chance"),
                ),
            ],
        },
    ),
    ("Contagion", Kind::Inert),
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
    ("Cremation", Kind::Inert),
    // Cyclone (P6-8a, Cast while Channelling's source): its movement speed
    // buff, `MovementSpeed` MORE with `Condition:ChannellingCyclone` (the
    // config's check) as an `unscalable` buff (`Data/Skills/act_dex.lua`
    // 4719 to 4721; [`UNSCALABLE`]).
    (
        "Cyclone",
        Kind::Buff {
            name: "Cyclone",
            effect: EffectType::Buff,
            stats: &[(
                "cyclone_movement_speed_+%_final",
                BuffStat::WhenState {
                    state: super::conditions::CHANNELLING_CYCLONE,
                    var: "ChannellingCyclone",
                    rows: &[(
                        Quantity::Poe1(Poe1Quantity::MovementSpeed),
                        Op::More,
                        1.0,
                        NONE,
                        NONE,
                    )],
                },
            )],
        },
    ),
    ("DarkPact", Kind::Inert),
    ("Dash", Kind::Inert),
    // P6-11fw: the banners (`act_str.lua`): an aura at `AuraEffect` MORE
    // -100 unless planted (the config's `bannerPlanted`), so nothing
    // without it; planted, scaled per Valour consumed
    // (`banner_buff_effect_+%_final_per_resource`, the config's
    // `bannerValour` up to `MaximumValour`, `CalcPerform.lua` 1299 to 1302),
    // which the engine does not model: a reason (`""`: its base mods).
    (
        "DefianceBanner",
        Kind::Buff {
            name: "Defiance Banner",
            effect: EffectType::Aura,
            stats: &[
                (
                    "",
                    BuffStat::ReasonWhenInput {
                        input: BANNER_PLANTED_INPUT,
                        what: "banner-valour",
                    },
                ),
                (
                    "base_movement_velocity_+%",
                    BuffStat::ReasonWhenInput {
                        input: BANNER_PLANTED_INPUT,
                        what: "banner-valour",
                    },
                ),
                (
                    "base_spell_suppression_chance_%",
                    BuffStat::ReasonWhenInput {
                        input: BANNER_PLANTED_INPUT,
                        what: "banner-valour",
                    },
                ),
            ],
        },
    ),
    ("Desecrate", Kind::Inert),
    (
        "Despair",
        Kind::Curse {
            name: "Despair",
            stats: &[
                (
                    "base_chaos_damage_resistance_%",
                    BuffStat::Rows(&[(enemy_res(DamageType::Chaos), Op::Base, 1.0, NONE, NONE)]),
                ),
                // PoB: the enemy's `SelfChaosMax` and `SelfChaosMin` BASE,
                // summed into every hit's added damage (`CalcOffence.lua`
                // 3263).
                (
                    "maximum_added_chaos_damage_taken",
                    BuffStat::Rows(&[(
                        added(DamageType::Chaos, Bound::Max),
                        Op::Base,
                        1.0,
                        NONE,
                        NONE,
                    )]),
                ),
                (
                    "minimum_added_chaos_damage_taken",
                    BuffStat::Rows(&[(
                        added(DamageType::Chaos, Bound::Min),
                        Op::Base,
                        1.0,
                        NONE,
                        NONE,
                    )]),
                ),
            ],
        },
    ),
    (
        "Determination",
        Kind::Buff {
            name: "Determination",
            effect: EffectType::Aura,
            stats: &[
                (
                    "base_physical_damage_reduction_rating",
                    BuffStat::Rows(&[(Quantity::Armour, Op::Base, 1.0, NONE, NONE)]),
                ),
                (
                    "determination_aura_armour_+%_final",
                    BuffStat::Rows(&[(Quantity::Armour, Op::More, 1.0, NONE, NONE)]),
                ),
            ],
        },
    ),
    ("DetonateDead", Kind::Inert),
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
                // P6-10b: a field now.
                (
                    "energy_shield_recharge_rate_+%",
                    BuffStat::Rows(&[(
                        Quantity::Poe1(Poe1Quantity::EnergyShieldRecharge),
                        Op::Increased,
                        1.0,
                        NONE,
                        NONE,
                    )]),
                ),
            ],
        },
    ),
    ("DivineIre", Kind::Inert),
    ("DominatingBlow", Kind::Inert),
    ("DominatingBlowAltX", Kind::Inert),
    ("DoubleStrike", Kind::Inert),
    ("Earthquake", Kind::Inert),
    ("Earthshatter", Kind::Inert),
    // P6-11au: no global effect in PoB's skill data (`gem_effects`).
    ("ElementalHitAltX", Kind::Inert),
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
                // PoB: the enemy's `SelfElementalAilmentDuration` INC, `mult =
                // -1`; ignite's duration reads it (P6-2b).
                (
                    "self_elemental_status_duration_-%",
                    BuffStat::Rows(&[(
                        Quantity::Poe1(Poe1Quantity::EnemyAilmentDuration),
                        Op::Increased,
                        -1.0,
                        NONE,
                        Flags::IGNITE,
                    )]),
                ),
            ],
        },
    ),
    ("EnchantmentOfFrostOnKill4", Kind::Inert),
    (
        "EnduringCry",
        Kind::Warcry {
            name: "Enduring Cry",
            stats: &[
                // Per 5 warcry power up to 25 (P6-11bs; PoB's
                // `LifeRegenPercent` BASE, the stat per minute: `div = 60`).
                (
                    "enduring_cry_life_regeneration_rate_per_minute_%_per_5_power_up_to_cap",
                    BuffStat::PerPower {
                        div: 5.0,
                        limit: 25.0,
                        rows: &[(
                            Quantity::Poe1(Poe1Quantity::RegenPercent(Pool::Life)),
                            Op::Base,
                            60.0,
                            NONE,
                            NONE,
                        )],
                    },
                ),
                (
                    "warcry_count_power_from_enemies",
                    BuffStat::NoSlice("warcry-power"),
                ),
            ],
        },
    ),
    (
        "Enfeeble",
        Kind::Curse {
            name: "Enfeeble",
            stats: &[
                // P6-10b parts 11 and 12: the enemy's accuracy (the
                // evade chance); its damage, which PoB conditions on the
                // enemy being rare or unique or not, a marker for the
                // enemy's hit.
                (
                    "accuracy_rating_+%",
                    BuffStat::Rows(&[(
                        Quantity::Poe1(Poe1Quantity::EnemyAccuracy),
                        Op::Increased,
                        1.0,
                        NONE,
                        NONE,
                    )]),
                ),
                (
                    "enfeeble_damage_+%_final",
                    BuffStat::Defence("enemy-offence"),
                ),
                (
                    "enfeeble_damage_+%_vs_rare_or_unique_final",
                    BuffStat::Defence("enemy-offence"),
                ),
            ],
        },
    ),
    // P6-11fn: Ensnaring Arrow's debuff ("Ensnared"): the enemy's
    // `ProjectileAttackDamageTaken` INC, per ensnare stack at most 1 (PoB's
    // `effectStackVar = "EnsnareStackCount"`, `effectStackLimit = 1`, the
    // stacks the config's `multiplierEnsnaredStackCount`), read only by a
    // projectile attack's hit ([`super::hit`]).
    (
        "EnsnaringArrow",
        Kind::Debuff {
            name: "Ensnared",
            stats: &[(
                "tethered_enemies_take_attack_projectile_damage_taken_+%",
                BuffStat::PerStack {
                    input: "multiplierEnsnaredStackCount",
                    var: "EnsnareStackCount",
                    limit: 1,
                    rows: &[(
                        Quantity::Poe1(Poe1Quantity::EnemyProjectileAttackDamageTaken),
                        Op::Increased,
                        1.0,
                        NONE,
                        NONE,
                    )],
                },
            )],
        },
    ),
    ("EssenceDrain", Kind::Inert),
    ("FireTrap", Kind::Inert),
    // Open items S3: no global effect in PoB's data (`act_int.lua` 5795).
    ("Fireball", Kind::Inert),
    ("FlameDash", Kind::Inert),
    // P6-11ga: Flame Surge's buff (`act_int.lua` 6620): the share of the
    // ignite's damage per second dealt as burning ground
    // (`IgniteDpsAsBurningGround` MAX, an unscalable `GlobalEffect` buff),
    // read by the main skill's ignite. Its more damage against burning
    // enemies is its own hit.
    (
        "FlameSurge",
        Kind::Buff {
            name: "Flame Surge",
            effect: EffectType::Buff,
            stats: &[(
                "flame_surge_ignite_damage_as_burning_ground_damage_%",
                BuffStat::Unscalable(&[(
                    Quantity::Poe1(Poe1Quantity::IgniteDpsAsBurningGround),
                    Op::Base,
                    1.0,
                    NONE,
                    NONE,
                )]),
            )],
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
        "Flammability",
        Kind::Curse {
            name: "Flammability",
            stats: &[
                (
                    "base_fire_damage_resistance_%",
                    BuffStat::Rows(&[(enemy_res(DamageType::Fire), Op::Base, 1.0, NONE, NONE)]),
                ),
                // PoB: the enemy's `SelfIgniteDuration` INC, `mult = -1`.
                (
                    "base_self_ignite_duration_-%",
                    BuffStat::Rows(&[(
                        Quantity::Poe1(Poe1Quantity::EnemyAilmentDuration),
                        Op::Increased,
                        -1.0,
                        Flags::IGNITE,
                        NONE,
                    )]),
                ),
                // PoB: the enemy's `SelfIgniteChance` BASE, added to every
                // hit's chance to ignite (`CalcOffence.lua` 4108).
                (
                    "chance_to_be_ignited_%",
                    BuffStat::Rows(&[(
                        Quantity::Poe1(Poe1Quantity::AilmentChance(
                            super::ailment::Ailment::Ignite,
                        )),
                        Op::Base,
                        1.0,
                        NONE,
                        NONE,
                    )]),
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
                    BuffStat::SandStance(&[(
                        Quantity::Poe1(Poe1Quantity::SelfDamageTaken(TakenWhen::Hit, TakenOf::All)),
                        Op::More,
                        1.0,
                        NONE,
                        NONE,
                    )]),
                ),
            ],
        },
    ),
    // Flesh Offering (P6-9e): its buff goes to minions alone (`buffNotPlayer`).
    (
        "FleshOffering",
        Kind::Buff {
            name: "Flesh Offering",
            effect: EffectType::Buff,
            stats: &[
                (
                    "attack_speed_+%_granted_from_skill",
                    BuffStat::MinionOnly(&[(
                        Quantity::Speed,
                        Op::Increased,
                        1.0,
                        Flags::ATTACK,
                        NONE,
                    )]),
                ),
                // P6-11gg: on you with "Your Offering Skills also affect
                // you" (`MovementSpeed` INC, a slice field since P6-10b).
                (
                    "base_movement_velocity_+%",
                    BuffStat::MinionOnly(&[(
                        Quantity::Poe1(Poe1Quantity::MovementSpeed),
                        Op::Increased,
                        1.0,
                        NONE,
                        NONE,
                    )]),
                ),
                (
                    "cast_speed_+%_granted_from_skill",
                    BuffStat::MinionOnly(&[(
                        Quantity::Speed,
                        Op::Increased,
                        1.0,
                        Flags::CAST,
                        NONE,
                    )]),
                ),
            ],
        },
    ),
    // P6-11fr: Flicker Strike beside the main skill (Vaal Flicker Strike's
    // gem grants it too): its buff's movement speed (`MovementSpeed` INC,
    // `effectName = "Flick Strike"`).
    (
        "FlickerStrike",
        Kind::Buff {
            name: "Flick Strike",
            effect: EffectType::Buff,
            stats: &[(
                "flicker_strike_buff_movement_speed_+%",
                BuffStat::Rows(&[(
                    Quantity::Poe1(Poe1Quantity::MovementSpeed),
                    Op::Increased,
                    1.0,
                    NONE,
                    NONE,
                )]),
            )],
        },
    ),
    // P6-9f: no global effect and no reservation in PoB's skill data (the
    // skill is inert when it is not the main skill).
    ("ForbiddenRite", Kind::Inert),
    // P6-9f: no global effect and no reservation in PoB's skill data (the
    // skill is inert when it is not the main skill).
    ("ForbiddenRiteAltX", Kind::Inert),
    ("FreezingPulse", Kind::Inert),
    ("Frenzy", Kind::Inert),
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
    // P6-11fk: Frost Shield (`act_int.lua`, `FrostShield`). PoB scales its
    // crit chance and life by `Multiplier:FrostShieldStage` (the config's
    // `frostShieldStages`, no default) at most `FrostShieldMaxStages` (the
    // gem's constant 4). P6-11ho: the stages apply; the shield's life and
    // share are a pool before yours (`poe1::enemy_hit`, `CalcDefence.lua`
    // 2444 to 2447, 47 to 52: none without life). The share is 20% with
    // the enemy in the area (the config's `conditionEnemyInFrostGlobe`),
    // else 60%. The energy shield drain (`EnergyShieldDegen` from one
    // stage) changes only the net regeneration (`CalcDefence.lua` 1288 to
    // 1292, 3397): a reason on those fields (open items S7, part 8).
    (
        "FrostShield",
        Kind::Buff {
            name: "Frost Shield",
            effect: EffectType::Buff,
            stats: &[
                (
                    "energy_shield_lost_per_minute",
                    BuffStat::Rows(&[(
                        Quantity::Poe1(Poe1Quantity::NetRegenReason(
                            "energy_shield_lost_per_minute",
                        )),
                        Op::Flag,
                        1.0,
                        NONE,
                        NONE,
                    )]),
                ),
                (
                    "frost_globe_absorb_damage_%_enemy_in_bubble",
                    BuffStat::EnemyCheck {
                        input: FROST_GLOBE_INPUT,
                        var: "EnemyInFrostGlobe",
                        neg: false,
                        rows: &[(
                            Quantity::Poe1(Poe1Quantity::FrostShieldShare),
                            Op::Base,
                            1.0,
                            NONE,
                            NONE,
                        )],
                    },
                ),
                (
                    "frost_globe_absorb_damage_%_enemy_outside_bubble",
                    BuffStat::EnemyCheck {
                        input: FROST_GLOBE_INPUT,
                        var: "EnemyInFrostGlobe",
                        neg: true,
                        rows: &[(
                            Quantity::Poe1(Poe1Quantity::FrostShieldShare),
                            Op::Base,
                            1.0,
                            NONE,
                            NONE,
                        )],
                    },
                ),
                (
                    "frost_globe_additional_spell_base_critical_strike_chance_per_stage",
                    BuffStat::PerCount {
                        input: FROST_SHIELD_STAGES_INPUT,
                        var: "FrostShieldStage",
                        limit: Some(FROST_SHIELD_MAX_STAGES),
                        default: None,
                        rows: &[(Quantity::CritChance, Op::Base, 100.0, Flags::SPELL, NONE)],
                    },
                ),
                (
                    "frost_globe_health_per_stage",
                    BuffStat::PerCount {
                        input: FROST_SHIELD_STAGES_INPUT,
                        var: "FrostShieldStage",
                        limit: Some(FROST_SHIELD_MAX_STAGES),
                        default: None,
                        rows: &[(
                            Quantity::Poe1(Poe1Quantity::FrostShieldLife),
                            Op::Base,
                            1.0,
                            NONE,
                            NONE,
                        )],
                    },
                ),
                (
                    "frost_globe_max_stages",
                    BuffStat::NoSlice("frost-shield-stage-limit"),
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
    ("GemDetonateMines", Kind::Inert),
    // General's Cry (P6-9e): its mirage warriors act only for a skill it
    // triggers (P6-8b); as another skill only its warcry power flag.
    (
        "GeneralsCry",
        Kind::Warcry {
            name: "General's Cry",
            stats: &[(
                "warcry_count_power_from_enemies",
                BuffStat::NoSlice("warcry-power"),
            )],
        },
    ),
    ("GoreShockwave", Kind::Inert),
    (
        "Grace",
        Kind::Buff {
            name: "Grace",
            effect: EffectType::Aura,
            stats: &[
                (
                    "base_evasion_rating",
                    BuffStat::Rows(&[(Quantity::Evasion, Op::Base, 1.0, NONE, NONE)]),
                ),
                (
                    "grace_aura_evasion_rating_+%_final",
                    BuffStat::Rows(&[(Quantity::Evasion, Op::More, 1.0, NONE, NONE)]),
                ),
            ],
        },
    ),
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
                // P6-10b: a field now.
                (
                    "base_movement_velocity_+%",
                    BuffStat::Rows(&[(
                        Quantity::Poe1(Poe1Quantity::MovementSpeed),
                        Op::Increased,
                        1.0,
                        NONE,
                        NONE,
                    )]),
                ),
                (
                    "cast_speed_+%_granted_from_skill",
                    BuffStat::Rows(&[(Quantity::Speed, Op::Increased, 1.0, Flags::CAST, NONE)]),
                ),
            ],
        },
    ),
    (
        "Hatred",
        Kind::Buff {
            name: "Hatred",
            effect: EffectType::Aura,
            stats: &[(
                "physical_damage_%_to_add_as_cold",
                BuffStat::Rows(&[(
                    Quantity::Poe1(Poe1Quantity::GainAs(DamageType::Physical, DamageType::Cold)),
                    Op::Base,
                    1.0,
                    NONE,
                    NONE,
                )]),
            )],
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
    // P6-11fz: Herald of the Breach's buff (`other.lua`, "Herald of the
    // Hive"): `Damage` MORE per herald skill affecting you (`Multiplier`
    // `Herald`, a `GlobalEffect` buff), the herald count of
    // `poe1::stacks::aura_or_herald`. Its pulses (Otherworldly Pressure,
    // the pulse delay) are its own hit, not a global effect.
    (
        "HeraldOfTheBreach",
        Kind::Buff {
            name: "Herald of the Hive",
            effect: EffectType::Buff,
            stats: &[(
                "skill_buff_grants_damage_+%_final_per_herald_skill_affecting_you",
                BuffStat::Rows(&[(
                    Quantity::Poe1(Poe1Quantity::Stack(
                        super::stacks::StackKind::Herald,
                        super::stacks::StackValue::Per(super::stacks::PerStack::DamageMore),
                    )),
                    Op::Base,
                    1.0,
                    NONE,
                    NONE,
                )]),
            )],
        },
    ),
    (
        "HeraldOfThunder",
        Kind::Buff {
            name: "Herald of Thunder",
            effect: EffectType::Buff,
            stats: &[
                (
                    "attack_maximum_added_lightning_damage",
                    BuffStat::Rows(&[(
                        added(DamageType::Lightning, Bound::Max),
                        Op::Base,
                        1.0,
                        Flags::ATTACK,
                        NONE,
                    )]),
                ),
                (
                    "attack_minimum_added_lightning_damage",
                    BuffStat::Rows(&[(
                        added(DamageType::Lightning, Bound::Min),
                        Op::Base,
                        1.0,
                        Flags::ATTACK,
                        NONE,
                    )]),
                ),
                (
                    "spell_maximum_added_lightning_damage",
                    BuffStat::Rows(&[(
                        added(DamageType::Lightning, Bound::Max),
                        Op::Base,
                        1.0,
                        Flags::SPELL,
                        NONE,
                    )]),
                ),
                (
                    "spell_minimum_added_lightning_damage",
                    BuffStat::Rows(&[(
                        added(DamageType::Lightning, Bound::Min),
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
    ("HolyStrike", Kind::Inert),
    ("IceNova", Kind::Inert),
    ("IceSpear", Kind::Inert),
    ("IcicleBurst", Kind::Inert),
    // P6-11dv: Immortal Call (`act_str.lua` 6700 to 6716), a guard: less
    // elemental and physical damage taken (`ElementalDamageTaken`,
    // `PhysicalDamageTaken` MORE). Its less physical damage taken per
    // removable endurance charge (`Multiplier:RemovableEnduranceCharge`, at
    // most 5, `div = 100`) needs the removable charges: a reason.
    (
        "ImmortalCall",
        Kind::Buff {
            name: "Immortal Call",
            effect: EffectType::Guard,
            stats: &[
                (
                    "mortal_call_elemental_damage_taken_+%_final",
                    BuffStat::Rows(&[(
                        Quantity::Poe1(Poe1Quantity::SelfDamageTaken(
                            TakenWhen::Always,
                            TakenOf::Elemental,
                        )),
                        Op::More,
                        1.0,
                        NONE,
                        NONE,
                    )]),
                ),
                (
                    "mortal_call_physical_damage_taken_+%_final",
                    BuffStat::Rows(&[(
                        Quantity::Poe1(Poe1Quantity::SelfDamageTaken(
                            TakenWhen::Always,
                            TakenOf::Type(DamageType::Physical),
                        )),
                        Op::More,
                        1.0,
                        NONE,
                        NONE,
                    )]),
                ),
                (
                    "mortal_call_physical_damage_taken_per_endurance_charge_consumed_final_permyriad",
                    BuffStat::ReasonWhenInput {
                        input: "useEnduranceCharges",
                        what: "removable-endurance-charges",
                    },
                ),
            ],
        },
    ),
    // P6-11fd: Infernal Cry (`act_str.lua` 6853): physical damage gained as
    // fire per 5 warcry power up to 25 (`PhysicalDamageGainAsFire` BASE, a
    // `Warcry` effect). Its exerted attacks' Combust and its explosion on
    // death reach no slice field.
    (
        "InfernalCry",
        Kind::Warcry {
            name: "Infernal Cry",
            stats: &[
                (
                    "infernal_cry_physical_damage_%_to_add_as_fire_per_5_power_up_to_cap",
                    BuffStat::PerPower {
                        div: 5.0,
                        limit: 25.0,
                        rows: &[(
                            Quantity::Poe1(Poe1Quantity::GainAs(
                                DamageType::Physical,
                                DamageType::Fire,
                            )),
                            Op::Base,
                            1.0,
                            NONE,
                            NONE,
                        )],
                    },
                ),
                (
                    "warcry_count_power_from_enemies",
                    BuffStat::NoSlice("warcry-power"),
                ),
            ],
        },
    ),
    (
        "IntimidatingCry",
        Kind::Warcry {
            name: "Intimidating Cry",
            stats: &[
                // Per 5 warcry power up to 30 (P6-11bs; PoB's
                // `MovementSpeed` INC).
                (
                    "intimidating_cry_movement_speed_+%_per_5_power_up_to_cap",
                    BuffStat::PerPower {
                        div: 5.0,
                        limit: 30.0,
                        rows: &[(
                            Quantity::Poe1(Poe1Quantity::MovementSpeed),
                            Op::Increased,
                            1.0,
                            NONE,
                            NONE,
                        )],
                    },
                ),
                (
                    "warcry_count_power_from_enemies",
                    BuffStat::NoSlice("warcry-power"),
                ),
            ],
        },
    ),
    // Open items S3: a wand attack with no global effect in PoB's data
    // (`act_int.lua`, `KineticBlast`).
    ("KineticBlast", Kind::Inert),
    ("LeapSlam", Kind::Inert),
    ("LightningWarp", Kind::Inert),
    (
        "Malevolence",
        Kind::Buff {
            name: "Malevolence",
            effect: EffectType::Aura,
            stats: &[
                (
                    "delirium_aura_damage_over_time_+%_final",
                    BuffStat::Rows(&[(Quantity::Damage, Op::More, 1.0, Flags::DOT, NONE)]),
                ),
                // Skill effect duration reaches no slice field.
                (
                    "delirium_skill_effect_duration_+%",
                    BuffStat::NoSlice("duration"),
                ),
            ],
        },
    ),
    // Molten Shell (P6-9e): a guard (module docs).
    (
        "MoltenShell",
        Kind::Buff {
            name: "Molten Shell",
            effect: EffectType::Guard,
            stats: &[
                (
                    "base_physical_damage_reduction_rating",
                    BuffStat::Rows(&[(Quantity::Armour, Op::Base, 1.0, NONE, NONE)]),
                ),
                // P6-11ae: the guard (`act_str.lua` 7545 to 7554).
                (
                    "molten_shell_damage_absorb_limit_%_of_armour",
                    BuffStat::Unscalable(&[(
                        Quantity::Poe1(Poe1Quantity::GuardAbsorbPerArmour),
                        Op::Base,
                        100.0,
                        NONE,
                        NONE,
                    )]),
                ),
                (
                    "molten_shell_damage_absorbed_%",
                    BuffStat::Unscalable(&[(
                        Quantity::Poe1(Poe1Quantity::GuardAbsorbRate),
                        Op::Base,
                        1.0,
                        NONE,
                        NONE,
                    )]),
                ),
                (
                    "molten_shell_max_damage_absorbed",
                    BuffStat::Unscalable(&[(
                        Quantity::Poe1(Poe1Quantity::GuardAbsorbMax),
                        Op::Base,
                        1.0,
                        NONE,
                        NONE,
                    )]),
                ),
            ],
        },
    ),
    ("OrbOfStorms", Kind::Inert),
    ("PenanceBrand", Kind::Inert),
    // Open items S2: Petrified Blood (`act_str.lua` 8519 to 8580), a buff that
    // reserves 35% of mana ([`reservation`]): its skills' mana cost also as
    // life while not on Low Life (`ManaCostAsLifeCost` BASE, `unscalable`,
    // [`UNSCALABLE_STATS`]; `costs` reads it), and the life loss below half
    // it prevents (`LifeLossBelowHalfPrevented` BASE, PoB's life hit pool,
    // `CalcDefence.lua` 177 to 181 and 2235 to 2245; `enemy_hit` and `ehp`
    // read it). The share lost over time (`LifeLossBelowHalfLost`) no PoB
    // module reads.
    (
        "PetrifiedBlood",
        Kind::Buff {
            name: "Petrified Blood",
            effect: EffectType::Buff,
            stats: &[
                (
                    "petrified_blood_%_life_loss_below_half_from_hit_to_prevent",
                    BuffStat::Rows(&[(
                        Quantity::Poe1(Poe1Quantity::LifeLossBelowHalfPrevented),
                        Op::Base,
                        1.0,
                        NONE,
                        NONE,
                    )]),
                ),
                (
                    "petrified_blood_%_prevented_life_loss_to_lose_over_time",
                    BuffStat::NoSlice("life-loss-over-time"),
                ),
                (
                    "skill_grants_life_cost_%_mana_cost_while_not_on_low_life",
                    BuffStat::WhenState {
                        state: super::conditions::NOT_LOW_LIFE,
                        var: "LowLife",
                        rows: &[(
                            Quantity::Poe1(Poe1Quantity::ManaCostAsLifeCost),
                            Op::Base,
                            1.0,
                            NONE,
                            NONE,
                        )],
                    },
                ),
            ],
        },
    ),
    // P6-11fo: Phase Run's buff (`act_dex.lua`, `PhaseRun`): movement
    // speed, and physical damage MORE with melee for skills that are not
    // totems' (PoB's negated `SkillType.SummonsTotem` tag,
    // `Flags::SKILL_NOT_TOTEM`). Its stealth and phasing reach no slice
    // field.
    (
        "PhaseRun",
        Kind::Buff {
            name: "Phase Run",
            effect: EffectType::Buff,
            stats: &[
                (
                    "base_movement_velocity_+%",
                    BuffStat::Rows(&[(
                        Quantity::Poe1(Poe1Quantity::MovementSpeed),
                        Op::Increased,
                        1.0,
                        NONE,
                        NONE,
                    )]),
                ),
                (
                    "phase_run_melee_physical_damage_+%_final",
                    BuffStat::Rows(&[(
                        Quantity::Damage,
                        Op::More,
                        1.0,
                        Flags::PHYSICAL
                            .union(Flags::MELEE)
                            .union(Flags::SKILL_NOT_TOTEM),
                        NONE,
                    )]),
                ),
            ],
        },
    ),
    // P6-11fy: Plague Bearer's buff (`act_int.lua`): poison damage MORE
    // while incubating (`modCond = "PlagueBearerIncubating"`), which the
    // config list `plagueBearerState` sets for "INC", its default
    // (`defaultIndex = 1`, `ConfigOptions.lua` 546): every value but "INF".
    (
        "PlagueBearer",
        Kind::Buff {
            name: "Plague Bearer",
            effect: EffectType::Buff,
            stats: &[(
                "corrosive_shroud_poison_damage_+%_final_while_accumulating_poison",
                BuffStat::ConfigIs {
                    input: PLAGUE_BEARER_INPUT,
                    value: "INF",
                    equal: false,
                    condition: "PlagueBearerIncubating",
                    rows: &[(Quantity::Damage, Op::More, 1.0, Flags::POISON, NONE)],
                },
            )],
        },
    ),
    // Poacher's Mark (P6-9e): the enemy's physical damage reduction and
    // added physical damage taken (PoB's `SelfPhysicalMin`/`Max` with
    // `ModFlag.Hit`, summed into the hit's added damage as Despair's).
    (
        "PoachersMark",
        Kind::Curse {
            name: "Poacher's Mark",
            stats: &[
                (
                    "base_additional_physical_damage_reduction_%",
                    BuffStat::Rows(&[(
                        Quantity::Poe1(Poe1Quantity::EnemyPhysicalDamageReduction),
                        Op::Base,
                        1.0,
                        NONE,
                        NONE,
                    )]),
                ),
                (
                    "life_granted_when_hit_by_attacks",
                    BuffStat::NoSlice("on-hit"),
                ),
                (
                    "mana_granted_when_hit_by_attacks",
                    BuffStat::NoSlice("on-hit"),
                ),
                (
                    "maximum_added_physical_damage_taken",
                    BuffStat::Rows(&[(
                        added(DamageType::Physical, Bound::Max),
                        Op::Base,
                        1.0,
                        Flags::HIT,
                        NONE,
                    )]),
                ),
                (
                    "minimum_added_physical_damage_taken",
                    BuffStat::Rows(&[(
                        added(DamageType::Physical, Bound::Min),
                        Op::Base,
                        1.0,
                        Flags::HIT,
                        NONE,
                    )]),
                ),
            ],
        },
    ),
    ("PoisonousConcoctionAltX", Kind::Inert),
    ("Portal", Kind::Inert),
    // Open items S3: Power Siphon's stats are its own (crit per power
    // charge); no global effect in PoB's data (`act_int.lua` 14102).
    ("PowerSiphon", Kind::Inert),
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
    // Pride (P6-9e): PoB's `AuraDebuff` on the enemy, its stat by the
    // config's `prideEffect` (`modCond`), scaled as an aura (its
    // `DebuffEffect` term is a reason, [`DEPENDS_ON`]), as Flesh and Stone.
    (
        "Pride",
        Kind::Buff {
            name: "Pride",
            effect: EffectType::Aura,
            stats: &[
                (
                    "physical_damage_aura_nearby_enemies_physical_damage_taken_+%",
                    BuffStat::ConfigIs {
                        input: PRIDE_INPUT,
                        value: "MAX",
                        equal: false,
                        condition: "PrideMinEffect",
                        rows: &[(
                            Quantity::Poe1(Poe1Quantity::EnemyTypeDamageTaken(
                                DamageType::Physical,
                            )),
                            Op::More,
                            1.0,
                            NONE,
                            NONE,
                        )],
                    },
                ),
                (
                    "physical_damage_aura_nearby_enemies_physical_damage_taken_+%_max",
                    BuffStat::ConfigIs {
                        input: PRIDE_INPUT,
                        value: "MAX",
                        equal: true,
                        condition: "PrideMaxEffect",
                        rows: &[(
                            Quantity::Poe1(Poe1Quantity::EnemyTypeDamageTaken(
                                DamageType::Physical,
                            )),
                            Op::More,
                            1.0,
                            NONE,
                            NONE,
                        )],
                    },
                ),
            ],
        },
    ),
    // P6-11fr: Primal Aegis (an item's triggered spell): no stat map in PoB
    // (its aegis is the granting modifier's).
    ("PrimalAegis", Kind::Inert),
    // Punishment (P6-9e): the enemy's damage taken while it is on low life
    // (PoB's `Condition` `LowLife` on the enemy, from the config).
    (
        "Punishment",
        Kind::Curse {
            name: "Punishment",
            stats: &[(
                "damage_taken_+%_on_low_life",
                BuffStat::WhenState {
                    state: super::conditions::ENEMY_LOW_LIFE,
                    var: "LowLife",
                    rows: &[(
                        Quantity::Poe1(Poe1Quantity::EnemyDamageTaken),
                        Op::Increased,
                        1.0,
                        NONE,
                        NONE,
                    )],
                },
            )],
        },
    ),
    (
        "PurityOfElements",
        Kind::Buff {
            name: "Purity of Elements",
            effect: EffectType::Aura,
            stats: &[
                (
                    "base_resist_all_elements_%",
                    BuffStat::Rows(&[
                        (
                            Quantity::Resistance(DamageType::Fire),
                            Op::Base,
                            1.0,
                            NONE,
                            NONE,
                        ),
                        (
                            Quantity::Resistance(DamageType::Cold),
                            Op::Base,
                            1.0,
                            NONE,
                            NONE,
                        ),
                        (
                            Quantity::Resistance(DamageType::Lightning),
                            Op::Base,
                            1.0,
                            NONE,
                            NONE,
                        ),
                    ]),
                ),
                (
                    "immune_to_status_ailments",
                    BuffStat::NoSlice("ailment-immunity"),
                ),
            ],
        },
    ),
    (
        "PurityOfFire",
        Kind::Buff {
            name: "Purity of Fire",
            effect: EffectType::Aura,
            stats: &[
                (
                    "base_fire_damage_resistance_%",
                    BuffStat::Rows(&[(
                        Quantity::Resistance(DamageType::Fire),
                        Op::Base,
                        1.0,
                        NONE,
                        NONE,
                    )]),
                ),
                (
                    "base_maximum_fire_damage_resistance_%",
                    BuffStat::Rows(&[(
                        Quantity::MaximumResistance(DamageType::Fire),
                        Op::Base,
                        1.0,
                        NONE,
                        NONE,
                    )]),
                ),
            ],
        },
    ),
    (
        "PurityOfIce",
        Kind::Buff {
            name: "Purity of Ice",
            effect: EffectType::Aura,
            stats: &[
                (
                    "base_cold_damage_resistance_%",
                    BuffStat::Rows(&[(
                        Quantity::Resistance(DamageType::Cold),
                        Op::Base,
                        1.0,
                        NONE,
                        NONE,
                    )]),
                ),
                (
                    "base_maximum_cold_damage_resistance_%",
                    BuffStat::Rows(&[(
                        Quantity::MaximumResistance(DamageType::Cold),
                        Op::Base,
                        1.0,
                        NONE,
                        NONE,
                    )]),
                ),
            ],
        },
    ),
    (
        "PurityOfLightning",
        Kind::Buff {
            name: "Purity of Lightning",
            effect: EffectType::Aura,
            stats: &[
                (
                    "base_lightning_damage_resistance_%",
                    BuffStat::Rows(&[(
                        Quantity::Resistance(DamageType::Lightning),
                        Op::Base,
                        1.0,
                        NONE,
                        NONE,
                    )]),
                ),
                (
                    "base_maximum_lightning_damage_resistance_%",
                    BuffStat::Rows(&[(
                        Quantity::MaximumResistance(DamageType::Lightning),
                        Op::Base,
                        1.0,
                        NONE,
                        NONE,
                    )]),
                ),
            ],
        },
    ),
    // P6-11fg: Pyroclast Mine of Sabotage (`PyroclastMineAltX`): its aura on
    // enemies near the mines applies fire exposure (PoB's `AuraDebuff`
    // `FireExposure` BASE, `effectName` "Pyroclast Mine Fire Exposure"),
    // the strongest exposure winning as every skill's (`poe1::exposure`);
    // its reservation is the mine's own. A debuff at scale 1: PoB scales an
    // `AuraDebuff` by the non-curse aura effect, so with one the debuff is
    // a reason (`one_skill`).
    (
        "PyroclastMineAltX",
        Kind::Debuff {
            name: "Pyroclast Mine Fire Exposure",
            stats: &[(
                "pyroclast_mine_aura_fire_exposure_%_to_apply",
                BuffStat::Rows(&[(
                    Quantity::Poe1(Poe1Quantity::SkillExposure(DamageType::Fire)),
                    Op::Base,
                    1.0,
                    NONE,
                    NONE,
                )]),
            )],
        },
    ),
    // Steelskin (P6-9e): a guard (module docs) that takes the non-Vaal
    // guard slot; its absorption reaches the defence estimate (P6-11ae).
    (
        "QuickGuard",
        Kind::Buff {
            name: "Steelskin",
            effect: EffectType::Guard,
            stats: &[
                ("", BuffStat::NoSlice("ailment-on-self")),
                // P6-11ae: the guard (`act_str.lua` 10952 to 10957).
                (
                    "quick_guard_damage_absorb_limit",
                    BuffStat::Unscalable(&[(
                        Quantity::Poe1(Poe1Quantity::GuardAbsorbLimit),
                        Op::Base,
                        1.0,
                        NONE,
                        NONE,
                    )]),
                ),
                (
                    "quick_guard_damage_absorbed_%",
                    BuffStat::Unscalable(&[(
                        Quantity::Poe1(Poe1Quantity::GuardAbsorbRate),
                        Op::Base,
                        1.0,
                        NONE,
                        NONE,
                    )]),
                ),
            ],
        },
    ),
    ("RainOfArrows", Kind::Inert),
    ("RaiseZombie", Kind::Inert),
    ("RaiseZombieAltX", Kind::Inert),
    ("RaiseZombieAltY", Kind::Inert),
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
    // PoB `Data/Skills/act_int.lua` `skills["RighteousFire"]`: its spell
    // damage more is a buff while unreserved life is at least 2; so is its
    // degeneration, `FireDegen` BASE per point of energy shield and of
    // life, the stat over 6000 (open items S7, part 8; `act_int.lua` 15439
    // to 15446), read by `poe1::degen`.
    (
        "RighteousFire",
        Kind::Buff {
            name: "Righteous Fire",
            effect: EffectType::Buff,
            stats: &[
                (
                    "base_nonlethal_fire_damage_%_of_maximum_energy_shield_taken_per_minute",
                    BuffStat::LifeThreshold(&[(
                        Quantity::Poe1(Poe1Quantity::Degen(
                            DamageType::Fire,
                            DegenOf::EnergyShield,
                        )),
                        Op::Base,
                        6000.0,
                        NONE,
                        NONE,
                    )]),
                ),
                (
                    "base_nonlethal_fire_damage_%_of_maximum_life_taken_per_minute",
                    BuffStat::LifeThreshold(&[(
                        Quantity::Poe1(Poe1Quantity::Degen(DamageType::Fire, DegenOf::Life)),
                        Op::Base,
                        6000.0,
                        NONE,
                        NONE,
                    )]),
                ),
                (
                    "righteous_fire_spell_damage_+%_final",
                    BuffStat::LifeThreshold(&[(
                        Quantity::Damage,
                        Op::More,
                        1.0,
                        Flags::SPELL,
                        NONE,
                    )]),
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
                // Open items S7, part 8: `FireDegen` BASE per point of mana,
                // the stat over 6000, while unreserved life is at least 2
                // (`act_int.lua` 15539 to 15542; its threshold of 1 mana
                // changes nothing: below it the amount per point is 0).
                (
                    "base_nonlethal_fire_damage_%_of_maximum_mana_taken_per_minute",
                    BuffStat::LifeThreshold(&[(
                        Quantity::Poe1(Poe1Quantity::Degen(DamageType::Fire, DegenOf::Mana)),
                        Op::Base,
                        6000.0,
                        NONE,
                        NONE,
                    )]),
                ),
                (
                    "righteous_fire_cast_speed_+%_final",
                    BuffStat::Threshold(&[(Quantity::Speed, Op::More, 1.0, Flags::CAST, NONE)]),
                ),
            ],
        },
    ),
    ("RollingMagma", Kind::Inert),
    // P6-11gb: Scorching Ray's fire exposure (`act_int.lua` 15732): a
    // debuff (`effectName` "Fire Exposure") whose `effectCond`
    // `ScorchingRayMaxStages` the skill sets at 8 stages (its
    // `MultiplierThreshold` 7 on `StageAfterFirst`), the default stage
    // count, or in its part 2 ("Maximum Stages"). Its burning is its own
    // damage over time.
    (
        "ScorchingRay",
        Kind::Debuff {
            name: "Fire Exposure",
            stats: &[(
                "base_fire_damage_resistance_%",
                BuffStat::AtMaxStages {
                    part: 2,
                    stages: 8,
                    condition: "ScorchingRayMaxStages",
                    rows: &[(
                        Quantity::Poe1(Poe1Quantity::SkillExposure(DamageType::Fire)),
                        Op::Base,
                        1.0,
                        NONE,
                        NONE,
                    )],
                },
            )],
        },
    ),
    ("SearingBond", Kind::Inert),
    // P6-11fd: Seismic Cry (`act_str.lua` 9667): more armour per 5 warcry
    // power up to 25 (`Armour` MORE, a `Warcry` effect). Its stun threshold
    // and its exerted slams' area reach no slice field.
    (
        "SeismicCry",
        Kind::Warcry {
            name: "Seismic Cry",
            stats: &[
                (
                    "seismic_cry_armour_+%_final_per_5_power_up_to_cap",
                    BuffStat::PerPower {
                        div: 5.0,
                        limit: 25.0,
                        rows: &[(Quantity::Armour, Op::More, 1.0, NONE, NONE)],
                    },
                ),
                (
                    "seismic_cry_stun_threshold_+%_per_5_power_up_to_cap",
                    BuffStat::NoSlice("stun"),
                ),
                (
                    "warcry_count_power_from_enemies",
                    BuffStat::NoSlice("warcry-power"),
                ),
            ],
        },
    ),
    ("ShieldCharge", Kind::Inert),
    ("ShieldCrush", Kind::Inert),
    // P6-11fx: Sigil of Power (`act_int.lua`): added lightning damage per
    // stage up to its maximum, and the enemy's less damage at the maximum.
    // The agent list (2026-11-27): the stages are the config's
    // `sigilOfPowerStages`, else its saved placeholder, else 1 (PoB's
    // `defaultPlaceholderState`), at most the gem's constant
    // `circle_of_power_max_stages` (4; PoB's `limitVar`). The less damage
    // reads, on the enemy, the character's stage multiplier
    // (`actor = "enemy"`): it applies at the maximum, unscaled.
    (
        "SigilOfPower",
        Kind::Buff {
            name: "Sigil of Power",
            effect: EffectType::Buff,
            stats: &[
                (
                    "circle_of_power_enemy_damage_+%_final_at_max_stages",
                    BuffStat::AtCount {
                        input: SIGIL_OF_POWER_STAGES_INPUT,
                        default: 1,
                        at: SIGIL_OF_POWER_MAX_STAGES,
                        var: "SigilOfPowerStage",
                        threshold_var: "SigilOfPowerMaxStages",
                        rows: &[(
                            Quantity::Poe1(Poe1Quantity::EnemyDamage),
                            Op::More,
                            1.0,
                            NONE,
                            NONE,
                        )],
                    },
                ),
                (
                    "circle_of_power_max_added_lightning_per_stage",
                    BuffStat::PerCount {
                        input: SIGIL_OF_POWER_STAGES_INPUT,
                        var: "SigilOfPowerStage",
                        limit: Some(SIGIL_OF_POWER_MAX_STAGES),
                        default: Some(1),
                        rows: &[(
                            added(DamageType::Lightning, Bound::Max),
                            Op::Base,
                            1.0,
                            NONE,
                            NONE,
                        )],
                    },
                ),
                (
                    "circle_of_power_max_stages",
                    BuffStat::NoSlice("sigil-of-power-stage-limit"),
                ),
                (
                    "circle_of_power_min_added_lightning_per_stage",
                    BuffStat::PerCount {
                        input: SIGIL_OF_POWER_STAGES_INPUT,
                        var: "SigilOfPowerStage",
                        limit: Some(SIGIL_OF_POWER_MAX_STAGES),
                        default: Some(1),
                        rows: &[(
                            added(DamageType::Lightning, Bound::Min),
                            Op::Base,
                            1.0,
                            NONE,
                            NONE,
                        )],
                    },
                ),
            ],
        },
    ),
    // Smite's aura (P6-8a: Smite beside the main skill, as Cast On Critical
    // Strike's source): `LightningMin` and `LightningMax` BASE as a
    // `GlobalEffect` aura without flags, and `EnemyShockChance`
    // (`Data/Skills/act_str.lua` 10325 to 10334; their `AffectedByVaalSmite`
    // condition is off without Vaal Smite). As the main skill its aura acts
    // through the hit's mapping (P3-8l). Its other stats act on Smite alone.
    (
        "Smite",
        Kind::Buff {
            name: "Smite",
            effect: EffectType::Aura,
            stats: &[
                (
                    "base_chance_to_shock_%_from_skill",
                    BuffStat::NoSlice("ailment-chance"),
                ),
                (
                    "maximum_added_lightning_damage_from_skill",
                    BuffStat::Rows(&[(
                        added(DamageType::Lightning, Bound::Max),
                        Op::Base,
                        1.0,
                        NONE,
                        NONE,
                    )]),
                ),
                (
                    "minimum_added_lightning_damage_from_skill",
                    BuffStat::Rows(&[(
                        added(DamageType::Lightning, Bound::Min),
                        Op::Base,
                        1.0,
                        NONE,
                        NONE,
                    )]),
                ),
            ],
        },
    ),
    // Sniper's Mark (P6-9e): projectile hits only ([`super::hit`]).
    (
        "SnipersMark",
        Kind::Curse {
            name: "Sniper's Mark",
            stats: &[
                (
                    "projectile_damage_taken_+%",
                    BuffStat::Rows(&[(
                        Quantity::Poe1(Poe1Quantity::EnemyProjectileDamageTaken),
                        Op::Increased,
                        1.0,
                        NONE,
                        NONE,
                    )]),
                ),
                (
                    "projectiles_hitting_self_split_into_x",
                    BuffStat::NoSlice("projectile-split"),
                ),
            ],
        },
    ),
    // Open items S3, part 5: the lightning spell of Mjolner's fixture
    // (PoB's export: no global effect).
    ("Spark", Kind::Inert),
    ("SpectralShieldThrow", Kind::Inert),
    ("Spellslinger", Kind::Inert),
    ("StaticStrike", Kind::Inert),
    ("StormRain", Kind::Inert),
    // Summon Carrion Golem (P6-9e): its buff's added damage goes to your
    // other minions (`MinionModifier`); no minion hit is computed yet
    // (P6-7c).
    (
        "SummonCarrionGolem",
        Kind::Buff {
            name: "Summon Carrion Golem",
            effect: EffectType::Buff,
            stats: &[
                ("", GOLEM_CONDITION),
                (
                    "bone_golem_grants_minion_maximum_added_physical_damage",
                    BuffStat::NoSlice("minion-buff"),
                ),
                (
                    "bone_golem_grants_minion_minimum_added_physical_damage",
                    BuffStat::NoSlice("minion-buff"),
                ),
            ],
        },
    ),
    (
        "SummonChaosGolem",
        Kind::Buff {
            name: "Summon Chaos Golem",
            effect: EffectType::Buff,
            stats: &[
                ("", GOLEM_CONDITION),
                (
                    "chaos_golem_grants_chaos_resistance_%",
                    BuffStat::Rows(&[(
                        Quantity::Resistance(DamageType::Chaos),
                        Op::Base,
                        1.0,
                        NONE,
                        NONE,
                    )]),
                ),
                (
                    "chaos_golem_grants_dot_multiplier_+",
                    BuffStat::Rows(&[(Quantity::DotMultiplier, Op::Base, 1.0, NONE, NONE)]),
                ),
            ],
        },
    ),
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
    ("SummonHolyRelicAltX", Kind::Inert),
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
                    BuffStat::Rows(&[(
                        Quantity::Poe1(Poe1Quantity::Regen(Pool::Mana)),
                        Op::Base,
                        60.0,
                        NONE,
                        NONE,
                    )]),
                ),
            ],
        },
    ),
    ("SummonRagingSpirit", Kind::Inert),
    ("SummonRagingSpiritAltX", Kind::Inert),
    // P6-9f: no global effect and no reservation in PoB's skill data (the
    // skill is inert when it is not the main skill).
    ("SummonSentinelOfRadiance", Kind::Inert),
    ("SummonSkeletons", Kind::Inert),
    ("SummonSkeletonsAltX", Kind::Inert),
    ("SummonSkeletonsAltY", Kind::Inert),
    // Open items (2026-11-24): Summon Skitterbots (`act_int.lua` 20085):
    // its one global stat, 10% more trap and mine damage (`Damage` MORE,
    // `KeywordFlag.Mine` or `Trap`, a `Buff`), and its reservation. Its
    // shock and chill of the enemy are [`skitterbots`]'s; its minions are no
    // hit of the character's.
    (
        "SummonSkitterbots",
        Kind::Buff {
            name: "Summon Skitterbots",
            effect: EffectType::Buff,
            stats: &[(
                "skitterbots_trap_mine_damage_+%_final",
                BuffStat::Rows(&[(
                    Quantity::Damage,
                    Op::More,
                    1.0,
                    NONE,
                    Flags::KEYWORD_MINE.union(Flags::KEYWORD_TRAP),
                )]),
            )],
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
                    BuffStat::Rows(&[(
                        Quantity::Poe1(Poe1Quantity::Regen(Pool::Life)),
                        Op::Base,
                        60.0,
                        NONE,
                        NONE,
                    )]),
                ),
                (
                    "stone_golem_grants_defences_+%",
                    BuffStat::Rows(&[
                        (Quantity::EnergyShield, Op::Increased, 1.0, NONE, NONE),
                        (Quantity::Armour, Op::Increased, 1.0, NONE, NONE),
                        (Quantity::Evasion, Op::Increased, 1.0, NONE, NONE),
                        // P6-11ek: Iron Reflexes counts it once.
                        (
                            Quantity::Poe1(Poe1Quantity::ArmourAndEvasionShared),
                            Op::Increased,
                            1.0,
                            NONE,
                            NONE,
                        ),
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
                // P6-11ar: `takenFromStoneGolemBeforeYou` BASE
                // (`SkillStatMap.lua` 2087), the golem's life a pool before
                // yours (`poe1::enemy_hit`).
                (
                    "stone_golem_grants_melee_damage_removed_from_stone_golem_before_life_or_es_%",
                    BuffStat::Rows(&[(
                        Quantity::Poe1(Poe1Quantity::TakenFromStoneGolem),
                        Op::Base,
                        1.0,
                        NONE,
                        NONE,
                    )]),
                ),
            ],
        },
    ),
    ("SummonVoidSpawn", Kind::Inert),
    ("Sunder", Kind::Inert),
    // Shockwave Support's own attack (`SupportBluntWeaponShockwave`): it
    // hits on its own when triggered; nothing reaches the character.
    ("SupportBluntWeaponShockwave", Kind::Inert),
    // Tempest Shield (P6-9e): disabled without a shield ([`one_skill`]).
    (
        "TempestShield",
        Kind::Buff {
            name: "Tempest Shield",
            effect: EffectType::Buff,
            stats: &[
                (
                    "shield_spell_block_%",
                    BuffStat::Rows(&[(
                        Quantity::Poe1(Poe1Quantity::SpellBlockChance),
                        Op::Base,
                        1.0,
                        NONE,
                        NONE,
                    )]),
                ),
                (
                    "skill_display_buff_grants_shock_immunity",
                    BuffStat::NoSlice("ailment-on-self"),
                ),
            ],
        },
    ),
    (
        "TemporalChains",
        Kind::Curse {
            name: "Temporal Chains",
            stats: &[
                // P6-11cv: the enemy's debuffs expire slower (PoB's
                // `BuffExpireFaster` MORE, `act_dex.lua` 15947), which
                // lengthens the ailments (`poe1::ailment`). The action speed:
                // no slice field.
                (
                    "buff_time_passed_+%_other_than_temporal_chains",
                    BuffStat::Rows(&[(
                        Quantity::Poe1(Poe1Quantity::EnemyBuffExpireFaster),
                        Op::More,
                        1.0,
                        NONE,
                        NONE,
                    )]),
                ),
                (
                    "temporal_chains_action_speed_+%_final",
                    BuffStat::NoSlice("enemy-speed"),
                ),
                (
                    "temporal_chains_action_speed_+%_vs_rare_or_unique_final",
                    BuffStat::NoSlice("enemy-speed"),
                ),
            ],
        },
    ),
    ("TentacleSmash", Kind::Inert),
    ("Tornado", Kind::Inert),
    ("ToxicRain", Kind::Inert),
    // P6-11fr: Bloodrend (an item's triggered spell): no stat map in PoB.
    ("TriggeredBloodrend", Kind::Inert),
    ("TriggeredMoltenStrike", Kind::Inert),
    ("TriggeredStarfall", Kind::Inert),
    // P6-9f: no global effect and no reservation in PoB's skill data (the
    // skill is inert when it is not the main skill).
    ("UniqueMirageWarriors", Kind::Inert),
    // Open items S3, part 8: The Hidden Blade's skill (PoB's export: no global
    // effect).
    ("UnseenStrike", Kind::Inert),
    // P6-11fr: Vaal Blight's Hinder (`act_int.lua`, `VaalBlight`): the
    // enemy's `ChaosDamageTaken` INC, a `Debuff` named "Hinder". Its damage
    // over time acts on Vaal Blight alone.
    (
        "VaalBlight",
        Kind::Debuff {
            name: "Hinder",
            stats: &[(
                "hinder_enemy_chaos_damage_taken_+%",
                BuffStat::Rows(&[(
                    Quantity::Poe1(Poe1Quantity::EnemyTypeDamageTaken(DamageType::Chaos)),
                    Op::Increased,
                    1.0,
                    NONE,
                    NONE,
                )]),
            )],
        },
    ),
    ("VaalDetonateDead", Kind::Inert),
    // P6-11du: Vaal Discipline's buff (`act_int.lua`): energy shield
    // recharge not delayed by damage, a `DUMMY` mod PoB's calculation never
    // reads.
    (
        "VaalDiscipline",
        Kind::Buff {
            name: "Vaal Discipline",
            effect: EffectType::Aura,
            stats: &[(
                "energy_shield_recharge_not_delayed_by_damage",
                BuffStat::NoSlice("recharge-delay"),
            )],
        },
    ),
    // P6-11fr: Vaal attacks with no global effect in PoB.
    ("VaalEarthquake", Kind::Inert),
    ("VaalFlickerStrike", Kind::Inert),
    // Open items S2: Vaal Grace's aura (`act_dex.lua` 8998 to 9040): its one
    // global stat, "+15% chance to Evade Attack Hits" (`EvadeChance` BASE,
    // [`super::evade`]). Its "Damage taken from Suppressed Hits is Unlucky"
    // has no row in PoB's stat map, so PoB applies nothing; the game makes
    // the enemy's suppressed hits unlucky, which the engine does not model:
    // a reason on the fields the enemy's hit reaches (review of S2).
    (
        "VaalGrace",
        Kind::Buff {
            name: "Vaal Grace",
            effect: EffectType::Aura,
            stats: &[
                (
                    "chance_to_evade_attacks_%",
                    BuffStat::Rows(&[(
                        Quantity::Poe1(Poe1Quantity::EvadeChance),
                        Op::Base,
                        1.0,
                        NONE,
                        NONE,
                    )]),
                ),
                (
                    "damage_taken_from_suppressed_hits_is_unlucky",
                    BuffStat::Defence("spell-suppression"),
                ),
            ],
        },
    ),
    // P6-11du: Vaal Haste's aura (`act_dex.lua` 9171), Haste's stats; PoB
    // merges buffs by name, so it stacks with Haste.
    (
        "VaalHaste",
        Kind::Buff {
            name: "Vaal Haste",
            effect: EffectType::Aura,
            stats: &[
                (
                    "attack_speed_+%_granted_from_skill",
                    BuffStat::Rows(&[(Quantity::Speed, Op::Increased, 1.0, Flags::ATTACK, NONE)]),
                ),
                (
                    "base_movement_velocity_+%",
                    BuffStat::Rows(&[(
                        Quantity::Poe1(Poe1Quantity::MovementSpeed),
                        Op::Increased,
                        1.0,
                        NONE,
                        NONE,
                    )]),
                ),
                (
                    "cast_speed_+%_granted_from_skill",
                    BuffStat::Rows(&[(Quantity::Speed, Op::Increased, 1.0, Flags::CAST, NONE)]),
                ),
            ],
        },
    ),
    ("VaalIceNova", Kind::Inert),
    // Vaal Molten Shell (P6-9e): a guard that takes every guard slot.
    (
        "VaalMoltenShell",
        Kind::Buff {
            name: "Vaal Molten Shell",
            effect: EffectType::Guard,
            stats: &[
                (
                    "base_physical_damage_reduction_rating",
                    BuffStat::Rows(&[(Quantity::Armour, Op::Base, 1.0, NONE, NONE)]),
                ),
                // P6-11ae: the guard (`act_str.lua` 7545 to 7554).
                (
                    "molten_shell_damage_absorb_limit_%_of_armour",
                    BuffStat::Unscalable(&[(
                        Quantity::Poe1(Poe1Quantity::GuardAbsorbPerArmour),
                        Op::Base,
                        100.0,
                        NONE,
                        NONE,
                    )]),
                ),
                (
                    "molten_shell_damage_absorbed_%",
                    BuffStat::Unscalable(&[(
                        Quantity::Poe1(Poe1Quantity::GuardAbsorbRate),
                        Op::Base,
                        1.0,
                        NONE,
                        NONE,
                    )]),
                ),
                (
                    "molten_shell_max_damage_absorbed",
                    BuffStat::Unscalable(&[(
                        Quantity::Poe1(Poe1Quantity::GuardAbsorbMax),
                        Op::Base,
                        1.0,
                        NONE,
                        NONE,
                    )]),
                ),
            ],
        },
    ),
    // P6-11fs: Vaal Righteous Fire's buff (`act_str.lua`): spell damage MORE.
    // Its burning and the life it takes act on the skill alone.
    (
        "VaalRighteousFire",
        Kind::Buff {
            name: "Vaal Righteous Fire",
            effect: EffectType::Buff,
            stats: &[(
                "vaal_righteous_fire_spell_damage_+%_final",
                BuffStat::Rows(&[(Quantity::Damage, Op::More, 1.0, Flags::SPELL, NONE)]),
            )],
        },
    ),
    // Open items S2: Vaal Smite's aura (`act_str.lua` 10517 to 10575): Smite's
    // aura's stats without its condition, `LightningMin` and `LightningMax`
    // BASE and `EnemyShockChance`; it replaces Smite's aura ([`apply`]).
    (
        "VaalSmite",
        Kind::Buff {
            name: VAAL_SMITE_AURA,
            effect: EffectType::Aura,
            stats: &[
                (
                    "base_chance_to_shock_%_from_skill",
                    BuffStat::NoSlice("ailment-chance"),
                ),
                (
                    "maximum_added_lightning_damage_from_skill",
                    BuffStat::Rows(&[(
                        added(DamageType::Lightning, Bound::Max),
                        Op::Base,
                        1.0,
                        NONE,
                        NONE,
                    )]),
                ),
                (
                    "minimum_added_lightning_damage_from_skill",
                    BuffStat::Rows(&[(
                        added(DamageType::Lightning, Bound::Min),
                        Op::Base,
                        1.0,
                        NONE,
                        NONE,
                    )]),
                ),
            ],
        },
    ),
    ("VaalSummonSkeletons", Kind::Inert),
    (
        "Vitality",
        Kind::Buff {
            name: "Vitality",
            effect: EffectType::Aura,
            stats: &[(
                "base_life_regeneration_rate_per_minute",
                BuffStat::Rows(&[(
                    Quantity::Poe1(Poe1Quantity::Regen(Pool::Life)),
                    Op::Base,
                    60.0,
                    NONE,
                    NONE,
                )]),
            )],
        },
    ),
    ("VolatileDead", Kind::Inert),
    (
        "Vulnerability",
        Kind::Curse {
            name: "Vulnerability",
            stats: &[
                (
                    "physical_damage_taken_+%",
                    BuffStat::Rows(&[(
                        Quantity::Poe1(Poe1Quantity::EnemyTypeDamageTaken(DamageType::Physical)),
                        Op::Increased,
                        1.0,
                        NONE,
                        NONE,
                    )]),
                ),
                // PoB: the enemy's `SelfBleedChance` BASE (4097) and
                // `SelfBleedDuration` INC.
                (
                    "receive_bleeding_chance_%_when_hit_by_attack",
                    BuffStat::Rows(&[(
                        Quantity::Poe1(Poe1Quantity::AilmentChance(super::ailment::Ailment::Bleed)),
                        Op::Base,
                        1.0,
                        NONE,
                        NONE,
                    )]),
                ),
                (
                    "self_bleed_duration_+%",
                    BuffStat::Rows(&[(
                        Quantity::Poe1(Poe1Quantity::EnemyAilmentDuration),
                        Op::Increased,
                        1.0,
                        Flags::BLEED,
                        NONE,
                    )]),
                ),
            ],
        },
    ),
    // P6-11fw: as Defiance Banner.
    (
        "WarBanner",
        Kind::Buff {
            name: "War Banner",
            effect: EffectType::Aura,
            stats: &[
                (
                    "",
                    BuffStat::ReasonWhenInput {
                        input: BANNER_PLANTED_INPUT,
                        what: "banner-valour",
                    },
                ),
                (
                    "accuracy_rating_+%",
                    BuffStat::ReasonWhenInput {
                        input: BANNER_PLANTED_INPUT,
                        what: "banner-valour",
                    },
                ),
                (
                    "war_banner_physical_damage_+%_final_with_melee_skills",
                    BuffStat::ReasonWhenInput {
                        input: BANNER_PLANTED_INPUT,
                        what: "banner-valour",
                    },
                ),
            ],
        },
    ),
    // P6-11fx: Wave of Conviction's exposure (`act_str.lua`): one of fire,
    // cold, or lightning exposure, the config's `waveOfConvictionExposureType`
    // (no default: nothing without it) as each mod's `effectCond`. With one
    // chosen, a reason (its matching rule not modelled here).
    (
        "WaveOfConviction",
        Kind::Debuff {
            name: "Wave of Conviction",
            stats: &[(
                "purge_expose_resist_%_matching_highest_element_damage",
                BuffStat::ReasonWhenInput {
                    input: WAVE_OF_CONVICTION_INPUT,
                    what: "wave-of-conviction-exposure",
                },
            )],
        },
    ),
    // P6-9f: no global effect and no reservation in PoB's skill data (the
    // skill is inert when it is not the main skill).
    ("WhirlingBlades", Kind::Inert),
    // P6-11fs: Wither's debuff slows the enemy ("Withered", reaching no
    // slice field); its withered stacks are the config's (P6-11aq).
    (
        "Wither",
        Kind::Debuff {
            name: "Withered",
            stats: &[(
                "base_movement_velocity_+%",
                BuffStat::NoSlice("enemy-movement"),
            )],
        },
    ),
    // Withering Step (P6-11aq): a buff with no reservation whose global
    // effects are Elusive's, `Condition:CanBeElusive` (the stat-less base
    // mod, `act_dex.lua` 17333) and `elusive_effect_+%` (its quality's
    // `ElusiveEffect`): Elusive itself is the `buffElusive` input, a named
    // condition (`config::NAMED_ONLY`), so neither reaches a slice field.
    // Its withered stacks are `damage_taken`'s (`can_wither`).
    (
        "WitheringStep",
        Kind::Buff {
            name: "Withering Step",
            effect: EffectType::Buff,
            stats: &[
                ("", BuffStat::NoSlice("elusive")),
                ("elusive_effect_+%", BuffStat::NoSlice("elusive")),
            ],
        },
    ),
    (
        "Wrath",
        Kind::Buff {
            name: "Wrath",
            effect: EffectType::Aura,
            stats: &[
                (
                    "attack_maximum_added_lightning_damage",
                    BuffStat::Rows(&[(
                        added(DamageType::Lightning, Bound::Max),
                        Op::Base,
                        1.0,
                        Flags::ATTACK,
                        NONE,
                    )]),
                ),
                (
                    "attack_minimum_added_lightning_damage",
                    BuffStat::Rows(&[(
                        added(DamageType::Lightning, Bound::Min),
                        Op::Base,
                        1.0,
                        Flags::ATTACK,
                        NONE,
                    )]),
                ),
                (
                    "wrath_aura_spell_lightning_damage_+%_final",
                    BuffStat::Rows(&[(
                        Quantity::Damage,
                        Op::More,
                        1.0,
                        Flags::SPELL.union(Flags::LIGHTNING),
                        NONE,
                    )]),
                ),
            ],
        },
    ),
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

/// Skills whose buff PoB marks `unscalable` (its `GlobalEffect` tag): no
/// buff effect modifier scales it (P6-8a).
const UNSCALABLE: &[&str] = &["Cyclone"];

/// Stats of a gate ([`BuffStat::WhenState`]) whose mod PoB marks
/// `unscalable` while the skill's other buff mods are not (open items S2:
/// Petrified Blood's life cost, `act_str.lua` 8527).
const UNSCALABLE_STATS: &[&str] = &["skill_grants_life_cost_%_mana_cost_while_not_on_low_life"];

/// Supports whose stats act through the hex they support (PoB's
/// `GlobalEffect` `effectType = "Curse"`; P6-11fl), sorted by ID: the
/// rows join the curse's, so they apply only while it holds a curse slot.
/// Each is also in [`SUPPORTS`] (its global stats none); its other stats
/// (Hexpass's hexproof and curse area) reach no slice field.
pub const CURSE_SUPPORTS: &[(&str, &[(&str, BuffStat)])] = &[(
    "SupportHexpass",
    &[(
        "hexed_enemies_cannot_deal_critical_strikes",
        BuffStat::Rows(&[(
            Quantity::Poe1(Poe1Quantity::EnemyNeverCrit),
            Op::Flag,
            1.0,
            NONE,
            NONE,
        )]),
    )],
)];

/// The modelled supports, sorted by granted effect ID, each with its
/// global stats (a buff of the skill it supports, named by the tag's
/// `effectName` or that skill's name). Checked as [`SKILLS`].
pub const SUPPORTS: &[(&str, &[(&str, BuffStat)])] = &[
    // P6-11fm: Battlemage's Cry's second effect (its gem's
    // `secondaryGrantedEffect`): it triggers the spells it supports with
    // less damage, a local stat; no global stat, and a triggered curse
    // applies as any.
    ("BattlemagesCrySupport", &[]),
    // Open items S2: added chaos damage, the supported skill's own; no `GlobalEffect` mod in PoB's export.
    ("SupportAddedChaosDamage", &[]),
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
                BuffStat::Rows(&[(
                    Quantity::Poe1(Poe1Quantity::ArcaneSurgeManaRegen),
                    Op::Base,
                    1.0,
                    NONE,
                    NONE,
                )]),
            ),
        ],
    ),
    // P6-9f: its aura effect is the supported aura's own scale term
    // (`OWN_EFFECT_STATS`) and its life reservation
    // `LIFE_RESERVATION_SUPPORTS`.
    ("SupportArrogance", &[]),
    // Autoexertion's warcry part (P6-11dy; `sup_str.lua`): its warcry shares
    // no buff ([`AUTOEXERTION_SUPPORT`]), reserves Autoexertion's share
    // (its level rows), and costs a fixed amount of mana no slice field
    // reads; no global stat.
    ("SupportAutoexertion", &[]),
    ("SupportAutomation", &[]),
    ("SupportAwakenedBrutality", &[]),
    ("SupportAwakenedCastOnCriticalStrike", &[]),
    ("SupportAwakenedCastWhileChannelling", &[]),
    ("SupportAwakenedControlledDestruction", &[]),
    // P6-9e
    ("SupportAwakenedEmpower", &[]),
    ("SupportAwakenedGreaterMultipleProjectiles", &[]),
    ("SupportAwakenedMeleePhysicalDamage", &[]),
    ("SupportAwakenedMinionDamage", &[]),
    ("SupportAwakenedViciousProjectiles", &[]),
    ("SupportAwakenedVoidManipulation", &[]),
    // P6-11ft: no global stat in PoB (the supported skill's own damage and
    // trigger; Mirage Archer beside the main skill, its main-skill part
    // P6-8b's).
    ("SupportBarrage", &[]),
    // Blasphemy (P6-11r): the hex it supports becomes an aura (its types,
    // so its curse priority and reservation), at its `CurseEffect` MORE
    // ([`BLASPHEMY_CURSE_EFFECT_STAT`]); no global stat.
    ("SupportBlasphemy", &[]),
    // P6-11fv: Bonechill, the chill as cold damage taken
    // ([`BONECHILL_SUPPORT`], `damage_taken`); its cold damage MORE is the
    // supported skill's own; no global stat.
    ("SupportBonechill", &[]),
    // Open items S3: Arcanist Brand's support part: no global stat; the
    // spell it supports is triggered by the brand (`poe1::triggers`).
    ("SupportBrandSupport", &[]),
    ("SupportBrutality", &[]),
    ("SupportBurningDamage", &[]),
    // Cast On Critical Strike's two parts (P6-8a; the awakened gem's attack
    // part is `SupportAwakenedCastOnCriticalStrike`) and Cast when Damage
    // Taken: no global stat; the trigger reads theirs (`poe1::triggers`).
    ("SupportCastOnCritTriggered", &[]),
    ("SupportCastOnCritTriggeredPlus", &[]),
    ("SupportCastOnCriticalStrike", &[]),
    // Open items S3: Cast on Death: no global stat; the spell it supports
    // takes PoB's stand-in time (`poe1::triggers`).
    ("SupportCastOnDeath", &[]),
    // Open items S3, part 8: Kitava's Thirst's support.
    ("SupportCastOnManaSpent", &[]),
    ("SupportCastOnMeleeKill", &[]),
    ("SupportCastOnMeleeKillTriggered", &[]),
    ("SupportCastOnWardBreak", &[]),
    ("SupportCastWhenDamageTaken", &[]),
    ("SupportCastWhileChannelling", &[]),
    ("SupportCastWhileChannellingTriggered", &[]),
    ("SupportCastWhileChannellingTriggeredPlus", &[]),
    ("SupportChanceToBleed", &[]),
    // P6-11fr: no stat map in PoB.
    ("SupportChargedMines", &[]),
    // P6-11hr: Combustion's "Ignites from Supported Skills apply -#% to
    // Fire Resistance" (`CombustionFireResist` BASE, -10 at every level;
    // `CalcPerform.lua` 2709 to 2716): the enemy's fire resistance while the
    // config ignites it, from a skill that can ignite ([`supports_global`]).
    // PoB applies the first such skill's value, every one -10; not scaled
    // by debuff effect (PoB adds it to the enemy directly). Its more fire
    // damage is the supported skill's own (`HIT_ROWS`).
    (
        "SupportCombustion",
        &[(
            "ignites_apply_fire_resistance_+",
            BuffStat::EnemyCheck {
                input: super::damage_taken::IGNITED_INPUT,
                var: "Ignited",
                neg: false,
                rows: &[(enemy_res(DamageType::Fire), Op::Base, 1.0, NONE, NONE)],
            },
        )],
    ),
    ("SupportConcentratedEffect", &[]),
    ("SupportControlledDestruction", &[]),
    ("SupportCooldownRecovery", &[]),
    ("SupportCruelty", &[]),
    ("SupportCullingStrike", &[]),
    // Hextouch's part that supports the hex (P6-11ds; `sup_int.lua` 1484):
    // the hex triggered on hit, no cost, at its `CurseEffect` MORE
    // ([`HEXTOUCH_CURSE_EFFECT_STAT`]) and a curse duration no slice field
    // reads; no global stat.
    ("SupportCurseOnHitCurse", &[]),
    ("SupportDamageOnFullLife", &[]),
    ("SupportDeadlyAilments", &[]),
    // Open items S2: Eclipse; no `GlobalEffect` mod in PoB's export.
    ("SupportEclipse", &[]),
    ("SupportEfficacy", &[]),
    // P6-11fq: Eldritch Blasphemy, Blasphemy's plus version: the same
    // aura, reservation, and `CurseEffect` MORE. Its Malignant Madness
    // minion damage needs a condition of yours nothing sets (PoB's
    // `enemyMalignantMadness`; the enemy's own is P6-11fp's); no global
    // stat.
    ("SupportEldritchBlasphemy", &[]),
    ("SupportElementalArmy", &[]),
    ("SupportElementalDamageWithAttacks", &[]),
    ("SupportElementalFocus", &[]),
    ("SupportElementalProliferation", &[]),
    ("SupportEmpower", &[]),
    ("SupportEnduranceChargeOnMeleeStun", &[]),
    ("SupportEnhance", &[]),
    ("SupportEnlighten", &[]),
    // Eternal Blessing (P6-9g): its aura effect is the supported aura's own
    // scale term (`OWN_EFFECT_STATS`), and the aura reserves nothing
    // (`NO_RESERVATION_SUPPORTS`). PoB reads none of its other stats (the
    // other skills it disables are not disabled there).
    ("SupportEternalBlessing", &[]),
    ("SupportFasterAttacks", &[]),
    ("SupportFasterCasting", &[]),
    ("SupportFeedingFrenzy", &[]),
    ("SupportFirePenetration", &[]),
    ("SupportFistofWar", &[]),
    ("SupportFocusedBallista", &[]),
    ("SupportFortify", &[]),
    // P6-11ft: Generosity, its aura not on you
    // ([`AURA_NOT_ON_SELF_SUPPORTS`]); no global stat.
    ("SupportGenerosity", &[]),
    ("SupportGreaterChain", &[]),
    ("SupportGreaterMultipleProjectiles", &[]),
    ("SupportGreaterMultistrike", &[]),
    ("SupportGreaterVolley", &[]),
    ("SupportGuardiansBlessing", &[]),
    ("SupportGuardiansBlessingMinion", &[]),
    // P6-11fl: its rows act through the hex ([`CURSE_SUPPORTS`]).
    ("SupportHexpass", &[]),
    ("SupportHextouch", &[]),
    ("SupportHypothermia", &[]),
    ("SupportIceBite", &[]),
    ("SupportIgniteProliferation", &[]),
    ("SupportImmolate", &[]),
    ("SupportImpale", &[]),
    ("SupportIncreasedAreaOfEffect", &[]),
    ("SupportIncreasedCriticalDamage", &[]),
    ("SupportIncreasedCriticalStrikes", &[]),
    ("SupportInfernalLegion", &[]),
    (
        "SupportInfusedChannelling",
        // P6-11aa: `DamageTakenWhenHit` MORE while Infusion is active.
        &[(
            "support_storm_barrier_damage_taken_when_hit_+%_final_while_channelling",
            BuffStat::CheckModCond {
                input: INFUSION_INPUT,
                condition: "InfusionActive",
                rows: &[(
                    Quantity::Poe1(Poe1Quantity::SelfDamageTaken(TakenWhen::Hit, TakenOf::All)),
                    Op::More,
                    1.0,
                    NONE,
                    NONE,
                )],
            },
        )],
    ),
    ("SupportInspiration", &[]),
    ("SupportIntensify", &[]),
    ("SupportInvertTheRules", &[]),
    ("SupportLifetap", &[]),
    // P6-11fu: Maim's debuff (`sup_str.lua` 3589): the enemy's
    // `PhysicalDamageTaken` INC while it is maimed (its `Maimed` condition,
    // the config's `conditionEnemyMaimed`), a `Debuff` named "Maim" at scale
    // 1 ([`DEBUFF_SUPPORTS`]). Its physical damage MORE is the supported
    // skill's own.
    (
        "SupportMaim",
        &[(
            "support_maimed_enemies_physical_damage_taken_+%",
            BuffStat::WhenState {
                state: super::conditions::ENEMY_MAIMED,
                var: "Maimed",
                rows: &[(
                    Quantity::Poe1(Poe1Quantity::EnemyTypeDamageTaken(DamageType::Physical)),
                    Op::Increased,
                    1.0,
                    NONE,
                    NONE,
                )],
            },
        )],
    ),
    ("SupportManaforgedArrows", &[]),
    // Mark On Hit (P6-9g): the mark's curse effect
    // ([`MARK_CURSE_EFFECT_STAT`]); PoB reads its trigger stat
    // (`triggerMarkOnRareOrUnique`) nowhere, so the mark applies as cast.
    ("SupportMarkOnHit", &[]),
    ("SupportMeatShield", &[]),
    ("SupportMeleePhysicalDamage", &[]),
    ("SupportMeleeSplash", &[]),
    ("SupportMinionDamage", &[]),
    ("SupportMinionLife", &[]),
    ("SupportMinionPact", &[]),
    ("SupportMinionSpeed", &[]),
    ("SupportMirageArcher", &[]),
    (
        "SupportMomentum",
        // Swiftness per the config's stacks removed (P6-11y).
        &[(
            "support_momentum_movement_speed_+%_per_stack_removed",
            BuffStat::PerCount {
                input: MOMENTUM_REMOVED_INPUT,
                var: "MomentumStacksRemoved",
                limit: None,
                default: None,
                rows: &[(
                    Quantity::Poe1(Poe1Quantity::MovementSpeed),
                    Op::Increased,
                    1.0,
                    NONE,
                    NONE,
                )],
            },
        )],
    ),
    // P6-11au: no global effect in PoB's skill data (`gem_effects`).
    ("SupportMoreDuration", &[]),
    ("SupportMultipleTotems", &[]),
    ("SupportMultistrike", &[]),
    ("SupportOverexertion", &[]),
    ("SupportPierce", &[]),
    ("SupportPowerChargeOnCritical", &[]),
    ("SupportPredator", &[]),
    ("SupportPulverise", &[]),
    ("SupportRuthless", &[]),
    ("SupportSecondWind", &[]),
    ("SupportShockwave", &[]),
    ("SupportSpellCascade", &[]),
    ("SupportSpellEcho", &[]),
    // Open items S3: Spellslinger's support part: no global stat; the
    // spell it supports reserves its level's share of mana (the support's
    // level row, as Blasphemy's) and is triggered (`poe1::triggers`).
    ("SupportSpellslinger", &[]),
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
    ("SupportSwiftAffliction", &[]),
    ("SupportSwiftAssembly", &[]),
    ("SupportTrapAndMineDamage", &[]),
    // Open items S3, part 5: the supports an item's trigger line gives its
    // socketed spells (`item_supports::TRIGGER_ROWS`): no global stat; the
    // spell is triggered (`poe1::triggers`).
    // Open items S3, part 8: Maloney's Mechanism's and the crafted Focus
    // line's supports.
    ("SupportTriggerBowSkillOnBowAttack", &[]),
    ("SupportTriggerElementalSpellOnBlock", &[]),
    ("SupportTriggerFireSpellOnHit", &[]),
    ("SupportTriggerSpellFromHelmet", &[]),
    ("SupportTriggerSpellOnAttack", &[]),
    ("SupportTriggerSpellOnBowAttack", &[]),
    ("SupportTriggerSpellOnBowAttackFreezeHit", &[]),
    // The agent list: the crafted trigger (`poe1::triggers`, `SkillUse`).
    ("SupportTriggerSpellOnSkillUse", &[]),
    ("SupportTriggerSpellOnUnarmedMeleeCriticalHit", &[]),
    ("SupportTrinity", &[]),
    // Open items S2: Unbound Ailments has no global stat; its ailment
    // effect is the supported skill's own, read for Summon Skitterbots'
    // auras ([`skitterbots`]) and a reason for a chilling area
    // ([`guaranteed_chill`]).
    ("SupportUnboundAilments", &[]),
    (
        "SupportUniqueCosprisMaliceColdSpellsCastOnMeleeCriticalStrike",
        &[],
    ),
    ("SupportUniqueMjolnerLightningSpellsCastOnHit", &[]),
    ("SupportUniqueVirulenceSpellsCastOnBlock", &[]),
    ("SupportUnleash", &[]),
    ("SupportUrgentOrders", &[]),
    ("SupportViciousProjectiles", &[]),
    ("SupportVoidManipulation", &[]),
    ("SupportVolatility", &[]),
    ("SupportWitheringTouch", &[]),
];

/// Supports whose base mods set the reservation of a skill of the type to
/// 0 (PoB's `SkillData` `manaReservationFlat` and kin, 0, tagged with the
/// type): Guardian's Blessing on an aura (`sup_str.lua` 2807), Eternal
/// Blessing (2062).
pub const NO_RESERVATION_SUPPORTS: &[(&str, &str)] = &[
    // P6-9g: Eternal Blessing (`sup_str.lua` 2075 to 2080; it supports
    // auras only).
    ("SupportEternalBlessing", "Aura"),
    ("SupportGuardiansBlessing", "Aura"),
];

/// Supports whose skill reserves life instead of mana (PoB's
/// `BloodMagicReserved`, `base_skill_reserve_life_instead_of_mana`;
/// `CalcPerform.lua` 1955 to 1964: life's base reservation gains mana's,
/// then each pool takes its own terms; P6-9f): Arrogance.
pub const LIFE_RESERVATION_SUPPORTS: &[&str] = &["SupportArrogance"];

/// Skills only PoB has (P6-11eu): no game granted effect has the ID, so
/// a gem of one is `unmodelled-gem` before any table is read. PoB adds
/// "On Kill Monster Explosion" (`EnemyExplode`, `act_*.lua`) as a granted
/// group from the explode-on-kill modifiers; it has no buff, reservation,
/// or cost, so it reaches a field only as the main skill or in full DPS. A
/// granted group of these alone is no reason otherwise.
pub const POB_ONLY_INERT: [&str; 1] = ["EnemyExplode"];

/// The supports whose `remote_mined_by_support` stat flags
/// `ManaCostGainAsReservation` and `LifeCostGainAsReservation` (P6-11et;
/// `SkillStatMap.lua` 1838; PoB's skill data: `sup_dex.lua` 2672,
/// `sup_int.lua` 3192 and 5695): the supported skill's level cost is its
/// flat reservation (`CalcPerform.lua` 1946 to 1953).
pub const COST_AS_RESERVATION_SUPPORTS: [&str; 3] = [
    "SupportBlastchainMine",
    "SupportHighImpactMine",
    "SupportLocusMine",
];

/// A curse gem's own curse effect stat (Despair's, a hex's quality):
/// PoB's `CurseEffect` INC on the skill's own list (P6-9b).
pub const OWN_CURSE_EFFECT_STAT: &str = "curse_effect_+%";

/// Mark On Hit's `CurseEffect` INC on the mark it supports (P6-9g; PoB's
/// `SkillStatMap.lua` 2107).
pub const MARK_CURSE_EFFECT_STAT: &str = "mark_skills_curse_effect_+%";

/// Blasphemy's `CurseEffect` MORE on the curse it supports (P6-11r; PoB's
/// `statMap`, a constant -25).
pub const BLASPHEMY_CURSE_EFFECT_STAT: &str = "support_blasphemy_curse_effect_+%_final";

/// Autoexertion's part that supports a warcry (P6-11dy): the warcry is used
/// automatically and shares no buff (PoB's `CannotShareWarcryBuffs`).
pub const AUTOEXERTION_SUPPORT: &str = "SupportAutoexertion";

/// Hextouch's less curse effect on the hex it supports (P6-11ds; PoB's
/// `CurseEffect` MORE of `SupportCurseOnHitCurse`, `sup_int.lua` 1500).
pub const HEXTOUCH_CURSE_EFFECT_STAT: &str = "support_hextouch_curse_effect_+%_final";

/// A gem's own stats that scale the buff of the skill it is part of
/// (PoB's `SkillStatMap.lua`: `golem_buff_effect_+%` 2089 is `BuffEffect`
/// INC, `aura_effect_+%` is `AuraEffect` INC, `offering_spells_effect_+%`
/// is `BuffEffect` INC, the offerings', P6-9e), and whether only an aura
/// reads it.
pub const OWN_EFFECT_STATS: &[(&str, bool)] = &[
    ("aura_effect_+%", true),
    ("golem_buff_effect_+%", false),
    // P6-11ft: Generosity's increase of the aura it supports (PoB's
    // `AuraEffect` INC on the skill), which only allies and minions get.
    ("non_curse_aura_effect_+%", true),
    ("offering_spells_effect_+%", false),
];

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

/// The config's warcry power (`ConfigOptions.lua` 732: `WarcryPower`
/// OVERRIDE; a count, applied when not 0, else its placeholder when not
/// 0, `ConfigTab.lua` 1088 to 1093).
pub const WARCRY_POWER_INPUT: &str = "multiplierWarcryPower";

/// The warcry power a boss enemy gives (`ConfigOptions.lua` 2097, 2132,
/// 2166: `WarcryPower` BASE 20 for the Boss, Pinnacle, and Uber kinds).
const BOSS_WARCRY_POWER: f64 = 20.0;

/// A warcry's terms (P6-11bs; `CalcPerform.lua` 2311 to 2350).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WarcryTerms {
    /// `(1 + inc / 100) x more`: `inc` the gem's own buff effect and the
    /// character's warcry buff effect; `more` 1.
    pub scale: f64,
    /// The power: the config's override, else `max(base x (1 + inc / 100),
    /// minimum)`, `base` 20 against a boss, `inc` the character's.
    pub power: f64,
    /// `min(duration / cooldown, 1)` (2336): `duration` PoB's
    /// `calcSkillDuration` (`CalcOffence.lua` 300 to 313: `(base + BASE) x
    /// max(calcLib.mod(Duration), 0)`), `cooldown` the level's over
    /// `calcLib.mod(CooldownRecovery)` (no server tick rounding here).
    pub uptime: f64,
}

/// The stats of a warcry's gems PoB reads as its `Duration` MORE
/// (`SkillStatMap.lua` 694 to 707).
const DURATION_MORE_STATS: &[&str] = &[
    "active_skill_quality_duration_+%_final",
    // Close-out M3: More and Less Duration.
    "support_more_duration_skill_effect_duration_+%_final",
    "support_reduced_duration_skill_effect_duration_+%_final",
    "support_swift_affliction_skill_effect_and_damaging_ailment_duration_+%_final",
];

/// The warcry power (P6-11bs, P6-11hz; `CalcPerform.lua` 1311 and 2327):
/// the config's override (its input, else its placeholder), else
/// `max(base x (1 + inc / 100), 0)`, `base` 20 against a boss. With `inc`
/// 0 it is PoB's `Multiplier:WarcryPower` (`output.WarcryPower`, the base
/// alone).
///
/// # Errors
/// `warcry-power` for a config power that is not a number.
pub fn warcry_power(
    input: &CalcInput<'_>,
    config: Option<&ConfigSet>,
    inc: f64,
) -> Result<f64, &'static str> {
    let number = |kind| match config.and_then(|set| last_input(set, WARCRY_POWER_INPUT, kind)) {
        None => Ok(None),
        Some(ConfigValue::Number(n)) if n.value().0.is_finite() => {
            Ok((n.value().0 != 0.0).then_some(n.value().0))
        }
        Some(_) => Err("warcry-power"),
    };
    let power = match number(ConfigInputKind::Input)? {
        Some(p) => p,
        None => match number(ConfigInputKind::Placeholder)? {
            Some(p) => p,
            None => {
                let base = if super::enemy::enemy(input.build, input.data).kind
                    == super::enemy::BossKind::None
                {
                    0.0
                } else {
                    BOSS_WARCRY_POWER
                };
                crate::math::max(base * (1.0 + inc / 100.0), 0.0)
            }
        },
    };
    Ok(power)
}

/// A warcry's terms ([`WarcryTerms`]), or the reason the engine cannot
/// read them.
///
/// # Errors
/// The `unmodelled-skill` name: `warcry-power` for a config power that is
/// not a number, `warcry-uptime` for a missing level or a stat the terms
/// do not read.
pub fn warcry_terms(
    input: &CalcInput<'_>,
    s: &GroupSkill<'_>,
    totals: &Totals,
    config: Option<&ConfigSet>,
) -> Result<WarcryTerms, &'static str> {
    let r = &s.active_gem;
    let level = skill::effect_level(r.effect, r.level).ok_or("warcry-uptime")?;
    let mut stats: Vec<(String, i64)> = Vec::new();
    for g in std::iter::once(r).chain(&s.supports) {
        let instance = skill::instance(g.effect, g.level, g.quality, input.data);
        if !instance.problems.is_empty() {
            return Err("warcry-uptime");
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
    // A flat cooldown change (`CooldownRecovery` BASE) is not read.
    if stat("base_cooldown_modifier_ms") != 0.0 {
        return Err("warcry-uptime");
    }
    // The power.
    let power = warcry_power(input, config, totals.warcry_power)?;
    // The duration.
    let base = stat("base_skill_effect_duration") / 1000.0;
    let inc = stat("skill_effect_duration_+%") + totals.duration + totals.warcry_duration;
    // P6-11hw: the character's warcry duration MORE (Call to Arms).
    let more = DURATION_MORE_STATS
        .iter()
        .fold(totals.warcry_duration_more(), |m, id| {
            m * (1.0 + stat(id) / 100.0)
        });
    let duration = base * crate::math::max((1.0 + inc / 100.0) * more, 0.0);
    // The cooldown.
    #[expect(
        clippy::cast_precision_loss,
        reason = "a cooldown in milliseconds, exact as f64"
    )]
    let cooldown_ms = level.cooldown as f64;
    let recovery = 1.0
        + (stat("base_cooldown_speed_+%")
            + stat("warcry_cooldown_speed_+%")
            + totals.cooldown_recovery
            + totals.warcry_cooldown_recovery
            + totals.named_cooldown_recovery(s.active.map(|a| a.id.as_str())))
            / 100.0;
    let cooldown = cooldown_ms / 1000.0 / recovery;
    let uptime = crate::math::min(duration / cooldown, 1.0);
    let scale = 1.0 + (own_effect(input, s, false) + totals.warcry_buff_effect) / 100.0;
    Ok(WarcryTerms {
        scale,
        power,
        uptime,
    })
}

/// PoB's base warcry cast time, seconds (`CalcSetup.lua` 57).
pub const WARCRY_CAST_TIME: f64 = 0.8;
/// PoB's server tick rate (`Modules/Data.lua` 231: `1 / 0.033`).
const SERVER_TICK_RATE: f64 = 1.0 / 0.033;

/// Intimidating Cry's uptime terms (module docs) from its first modelled
/// skill: `None` when the level is missing.
fn intimidating_terms(
    input: &CalcInput<'_>,
    s: &GroupSkill<'_>,
    totals: &Totals,
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
    // 547), and the character's (PoB's `skillModList` holds them: the
    // untagged, the warcries', and Intimidating Cry's by name; the agent
    // list).
    let modifier = crate::math::max(
        0.0,
        1.0 + (stat("base_cooldown_speed_+%")
            + stat("warcry_cooldown_speed_+%")
            + totals.cooldown_recovery
            + totals.warcry_cooldown_recovery
            + totals.named_cooldown_recovery(s.active.map(|a| a.id.as_str())))
            / 100.0,
    );
    let mut cooldown = cooldown_ms / 1000.0 / modifier;
    if stored <= 1.0 {
        cooldown = crate::math::ceil(cooldown * SERVER_TICK_RATE) / SERVER_TICK_RATE;
    }
    // The gems' warcry speed and the character's (P6-11ci; PoB's
    // `calcLib.mod(skillModList, "WarcrySpeed")`).
    let cast_rate =
        1.0 / WARCRY_CAST_TIME * (1.0 + (stat("warcry_speed_+%") + totals.warcry_speed) / 100.0);
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
/// attacks read (`ConfigOptions.lua` 1122).
pub const NEARBY_ALLY_INPUT: &str = "multiplierNearbyAlly";
/// The config input of the stance (`ConfigOptions.lua` 643).
pub const STANCE_INPUT: &str = "bloodSandStance";
/// The config check of Infused Channelling's Infusion
/// (`ConfigOptions.lua` 495; P6-11aa).
pub const INFUSION_INPUT: &str = "infusedChannellingInfusion";
/// The config count of Momentum stacks removed (Swiftness,
/// `ConfigOptions.lua` 543; P6-11y).
pub const MOMENTUM_REMOVED_INPUT: &str = "MomentumSwiftnessStacks";
/// The config list of Wave of Conviction's exposure type
/// (`ConfigOptions.lua` 736; P6-11fx).
pub const WAVE_OF_CONVICTION_INPUT: &str = "waveOfConvictionExposureType";
/// The config list of Plague Bearer's state (`ConfigOptions.lua` 546;
/// P6-11fy).
pub const PLAGUE_BEARER_INPUT: &str = "plagueBearerState";
/// The config check of a planted banner (`ConfigOptions.lua` 287;
/// P6-11fw).
pub const BANNER_PLANTED_INPUT: &str = "bannerPlanted";
/// The config list of Pride's effect (`ConfigOptions.lua` 566: `MIN` or
/// `MAX`; P6-9e).
pub const PRIDE_INPUT: &str = "prideEffect";

/// Frost Shield's stages (P6-11ho; PoB's `frostShieldStages`,
/// `ConfigOptions.lua` 455).
const FROST_SHIELD_STAGES_INPUT: &str = "frostShieldStages";

/// The enemy in Frost Shield's area (P6-11ho; `ConfigOptions.lua` 2016).
const FROST_GLOBE_INPUT: &str = "conditionEnemyInFrostGlobe";

/// Frost Shield's maximum stages (`frost_globe_max_stages`, the gem's
/// constant).
const FROST_SHIELD_MAX_STAGES: u32 = 4;

/// The config count of Sigil of Power's stages (`ConfigOptions.lua` 620).
pub const SIGIL_OF_POWER_STAGES_INPUT: &str = "sigilOfPowerStages";
/// Sigil of Power's maximum stages (`circle_of_power_max_stages`, the
/// gem's constant).
const SIGIL_OF_POWER_MAX_STAGES: u32 = 4;

/// The config inputs this module reads.
#[must_use]
pub fn is_input(name: &str) -> bool {
    [
        WRATH_INPUT,
        "flameWallAddedDamage",
        STANCE_INPUT,
        PRIDE_INPUT,
        MOMENTUM_REMOVED_INPUT,
        INFUSION_INPUT,
        WARCRY_POWER_INPUT,
        // P6-11fk, P6-11fn: Frost Shield's stages, Ensnaring Arrow's stacks.
        FROST_SHIELD_STAGES_INPUT,
        // P6-11ho: the enemy in Frost Shield's area.
        FROST_GLOBE_INPUT,
        "multiplierEnsnaredStackCount",
        // P6-11fw: a planted banner (its Valour count goes with it).
        BANNER_PLANTED_INPUT,
        WAVE_OF_CONVICTION_INPUT,
        PLAGUE_BEARER_INPUT,
        // The agent list (2026-11-27): Sigil of Power's stages.
        SIGIL_OF_POWER_STAGES_INPUT,
    ]
    .contains(&name)
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
        skill::group_skills(group, group_matches, input.data, levels)
            .iter()
            .any(|s| {
                s.supports
                    .iter()
                    .any(|sup| sup.effect.id == "SupportArcaneSurge")
            })
    })
}

/// Whether the gem table applies a curse to the enemy (P6-11cu): a skill of
/// [`Kind::Curse`] (curses and marks) in [`affected_by`]'s groups. PoB's
/// curse limit starts at 1 (`EnemyCurseLimit`), so such a skill fills a
/// slot and the enemy is `Cursed` (`CalcPerform.lua` 3301 to 3303). A
/// curse is never a main skill whose hit the engine computes, so
/// [`gems`] applies each.
#[must_use]
pub fn curses_enemy(input: &CalcInput<'_>, levels: &GemLevels) -> bool {
    !applied_curses(input, levels).is_empty()
}

/// Whether a mark holds a curse slot on the enemy (PoB's enemy `Marked`,
/// `CalcPerform.lua` 3304; P6-11de), where the skill list settles it:
/// `Some(false)` with no mark among [`curses_enemy`]'s curses, `Some(true)`
/// when every one is a mark (the first takes a slot: PoB's limit starts at
/// 1, and a mark never loses its slot to another mark). With hexes and a
/// mark, the slots' priorities decide ([`curse_slots`], read only while
/// the gems are applied): `None`, a reason.
#[must_use]
pub fn marks_enemy(input: &CalcInput<'_>, levels: &GemLevels) -> Option<bool> {
    let curses = applied_curses(input, levels);
    if !curses.contains(&true) {
        Some(false)
    } else if curses.iter().all(|mark| *mark) {
        Some(true)
    } else {
        None
    }
}

/// The curse skills the gem table applies to the enemy ([`curses_enemy`]),
/// each as whether it is a mark (typed `Mark`, PoB's `isMark`).
fn applied_curses(input: &CalcInput<'_>, levels: &GemLevels) -> Vec<bool> {
    let build = input.build;
    let Ok(groups) = skill::active_groups(build) else {
        return Vec::new();
    };
    let main_group = skill::main_skill(build, input.data, levels)
        .ok()
        .map(|m| m.group - 1);
    let matches = sockets::socket_matches(build, &groups, input.data);
    let mut out = Vec::new();
    for (g, group) in groups.iter().enumerate() {
        let slot = group.slot.as_ref().map(|s| s.as_str());
        if (main_group != Some(g) && !(group.enabled && slot_enabled(build, slot)))
            || skill::attribute(&group.attributes, "source").is_some()
        {
            continue;
        }
        let group_matches = matches.get(g).map_or(&[][..], Vec::as_slice);
        out.extend(
            skill::group_skills(group, group_matches, input.data, levels)
                .iter()
                .filter(|s| matches!(kind(&s.active_gem.effect.id), Some(Kind::Curse { .. })))
                .map(|s| s.types.contains("Mark")),
        );
    }
    out
}

/// The curses cast by you (P6-11gz; PoB's `SelfCast<Curse>`,
/// `CalcPerform.lua` 2603): every curse skill of the gem table among the
/// enabled groups ([`applied_curses`]'), applied or not, whose types (with
/// its supports') are neither `Triggered` nor `Aura`. `None` when the skill
/// groups are not certain.
#[must_use]
pub fn self_cast_curses(input: &CalcInput<'_>) -> Option<BTreeSet<&'static str>> {
    let build = input.build;
    let levels = GemLevels::none();
    let groups = skill::active_groups(build).ok()?;
    let main_group = skill::main_skill(build, input.data, &levels)
        .ok()
        .map(|m| m.group - 1);
    let matches = sockets::socket_matches(build, &groups, input.data);
    let mut out = BTreeSet::new();
    for (g, group) in groups.iter().enumerate() {
        let slot = group.slot.as_ref().map(|s| s.as_str());
        if (main_group != Some(g) && !(group.enabled && slot_enabled(build, slot)))
            || skill::attribute(&group.attributes, "source").is_some()
        {
            continue;
        }
        let group_matches = matches.get(g).map_or(&[][..], Vec::as_slice);
        for s in skill::group_skills(group, group_matches, input.data, &levels) {
            if let Some(Kind::Curse { name, .. }) = kind(&s.active_gem.effect.id)
                && !s.types.contains("Triggered")
                && !s.types.contains("Aura")
            {
                out.insert(name);
            }
        }
    }
    Some(out)
}

/// The curse names of the gem table (P6-11gz), for the self-cast states.
#[must_use]
pub fn curse_names() -> Vec<&'static str> {
    SKILLS
        .iter()
        .filter_map(|(_, k)| match k {
            Kind::Curse { name, .. } => Some(*name),
            _ => None,
        })
        .collect()
}

/// The auras the gem table applies to the character, by PoB's buff name
/// (a Vaal aura's without "Vaal "; P6-11o): what PoB's `AffectedBy<Name>`
/// conditions read (`CalcPerform.lua` 2417 to 2419). The groups are
/// [`arcane_surge_support`]'s.
#[must_use]
pub fn affected_by(input: &CalcInput<'_>, levels: &GemLevels) -> BTreeSet<&'static str> {
    let build = input.build;
    let mut out = BTreeSet::new();
    let Ok(groups) = skill::active_groups(build) else {
        return out;
    };
    let main_group = skill::main_skill(build, input.data, levels)
        .ok()
        .map(|m| m.group - 1);
    let matches = sockets::socket_matches(build, &groups, input.data);
    for (g, group) in groups.iter().enumerate() {
        let slot = group.slot.as_ref().map(|s| s.as_str());
        if main_group != Some(g) && !(group.enabled && slot_enabled(build, slot)) {
            continue;
        }
        if skill::attribute(&group.attributes, "source").is_some() {
            continue;
        }
        let group_matches = matches.get(g).map_or(&[][..], Vec::as_slice);
        for s in skill::group_skills(group, group_matches, input.data, levels) {
            if let Some(Kind::Buff {
                name,
                effect: EffectType::Aura,
                ..
            }) = kind(&s.active_gem.effect.id)
            {
                out.insert(name.strip_prefix("Vaal ").unwrap_or(name));
            }
        }
    }
    out
}

/// PoB's `Multiplier:Herald` plus `Multiplier:AuraAffectingSelf`
/// (P6-11s; `CalcPerform.lua` 2112 to 2126): over the skills of the main
/// group and the enabled groups in an enabled slot, the distinct skills
/// whose types (the supports' added ones included) hold `Herald`, and the
/// distinct others that hold `Aura` but not `AuraAffectsEnemies` and that
/// no Generosity supports (`auraCannotAffectSelf`, `sup_str.lua` 2706 and
/// 2773). Distinct by granted effect, as PoB by skill name.
#[must_use]
pub fn aura_or_herald_count(input: &CalcInput<'_>) -> usize {
    let (heralds, auras) = herald_and_aura_counts(input);
    heralds + auras
}

/// PoB's `Multiplier:Herald` and `Multiplier:AuraAffectingSelf` apart
/// (P6-11af; [`aura_or_herald_count`]'s count).
#[must_use]
pub fn herald_and_aura_counts(input: &CalcInput<'_>) -> (usize, usize) {
    const GENEROSITY: [&str; 2] = ["SupportGenerosity", "SupportAwakenedGenerosity"];
    let build = input.build;
    let levels = GemLevels::none();
    let Ok(groups) = skill::active_groups(build) else {
        return (0, 0);
    };
    let main_group = skill::main_skill(build, input.data, &levels)
        .ok()
        .map(|m| m.group - 1);
    let matches = sockets::socket_matches(build, &groups, input.data);
    let (mut heralds, mut auras) = (BTreeSet::new(), BTreeSet::new());
    for (g, group) in groups.iter().enumerate() {
        let slot = group.slot.as_ref().map(|s| s.as_str());
        if main_group != Some(g) && !(group.enabled && slot_enabled(build, slot)) {
            continue;
        }
        let group_matches = matches.get(g).map_or(&[][..], Vec::as_slice);
        for s in skill::group_skills(group, group_matches, input.data, &levels) {
            let id = s.active_gem.effect.id.as_str();
            if s.types.contains("Herald") {
                heralds.insert(id);
            } else if s.types.contains("Aura")
                && !s.types.contains("AuraAffectsEnemies")
                && !s
                    .supports
                    .iter()
                    .any(|sup| GENEROSITY.contains(&sup.effect.id.as_str()))
            {
                auras.insert(id);
            }
        }
    }
    (heralds.len(), auras.len())
}

/// Whether any active skill is a totem (P6-11ep; PoB's `skillFlags.totem`
/// over `activeSkillList`, `CalcPerform.lua` 1334): over the skills of the
/// main group and the enabled groups in an enabled slot (as
/// [`herald_and_aura_counts`]), one whose types (the supports' added ones
/// included) hold `SummonsTotem`.
#[must_use]
pub fn any_totem_skill(input: &CalcInput<'_>) -> bool {
    let build = input.build;
    let levels = GemLevels::none();
    let Ok(groups) = skill::active_groups(build) else {
        return false;
    };
    let main_group = skill::main_skill(build, input.data, &levels)
        .ok()
        .map(|m| m.group - 1);
    let matches = sockets::socket_matches(build, &groups, input.data);
    groups.iter().enumerate().any(|(g, group)| {
        let slot = group.slot.as_ref().map(|s| s.as_str());
        if main_group != Some(g) && !(group.enabled && slot_enabled(build, slot)) {
            return false;
        }
        let group_matches = matches.get(g).map_or(&[][..], Vec::as_slice);
        skill::group_skills(group, group_matches, input.data, &levels)
            .iter()
            .any(|s| s.types.contains(skill::TOTEM_TYPE))
    })
}

/// The largest attach limit of the active brand skills (P6-11ew; PoB's
/// `BrandsAttachedLimit` on each skill's list, `CalcPerform.lua` 1322):
/// over the skills of the main group and the enabled groups in an enabled
/// slot, those of the `Brand` type, each the sum of its gems'
/// `base_number_of_sigils_allowed_per_target` (`SkillStatMap.lua` 2155).
/// `None` without a brand skill.
#[must_use]
pub fn brand_attach_limit(input: &CalcInput<'_>) -> Option<f64> {
    const STAT: &str = "base_number_of_sigils_allowed_per_target";
    let build = input.build;
    let levels = GemLevels::none();
    let groups = skill::active_groups(build).ok()?;
    let main_group = skill::main_skill(build, input.data, &levels)
        .ok()
        .map(|m| m.group - 1);
    let matches = sockets::socket_matches(build, &groups, input.data);
    let mut best: Option<f64> = None;
    for (g, group) in groups.iter().enumerate() {
        let slot = group.slot.as_ref().map(|s| s.as_str());
        if main_group != Some(g) && !(group.enabled && slot_enabled(build, slot)) {
            continue;
        }
        let group_matches = matches.get(g).map_or(&[][..], Vec::as_slice);
        for s in skill::group_skills(group, group_matches, input.data, &levels) {
            if !s.types.contains("Brand") {
                continue;
            }
            let mut limit = 0.0;
            for gem in std::iter::once(&s.active_gem).chain(&s.supports) {
                let instance = skill::instance(gem.effect, gem.level, gem.quality, input.data);
                #[expect(clippy::cast_precision_loss, reason = "a small count, exact as f64")]
                if let Some((_, v)) = instance.stats.iter().find(|(id, _)| id == STAT) {
                    limit += *v as f64;
                }
            }
            best = Some(best.map_or(limit, |b: f64| b.max(limit)));
        }
    }
    best
}

/// The number of summoned golems (P6-11gc): [`minion_limit`] for
/// `ActiveGolemLimit`.
#[must_use]
pub fn golem_limit(input: &CalcInput<'_>, extra: f64) -> f64 {
    minion_limit(
        input,
        "ActiveGolemLimit",
        "base_number_of_golems_allowed",
        extra,
    )
}

/// The number of summoned minions of a limit (P6-11gc, P6-11ie; PoB's
/// `output[limit]`, `CalcPerform.lua` 1401 to 1413): over the skills of the
/// main group and the enabled groups in an enabled slot whose minion list
/// (PoB's `CalcActiveSkill.lua` 659 to 679: the active skill's
/// [`super::pob_skill::PobSkill::minions`], then its supports'
/// [`super::pob_skill::PobSkill::add_minions`]) has a minion limited by
/// `limit` (not hostile), the largest `floor(val x more)`: `val` the
/// skill's `limit` BASE (its gems' `stat`, plus the character's `extra`; 0
/// when the sum is 0, `calcLib.val`), `more` its supports'
/// `ActiveMinionLimit` MORE (`support_undead_army_minion_maximum_count_+%_final`,
/// `sup_int.lua` 1063). 0 without one, as PoB's unset output.
#[must_use]
pub fn minion_limit(input: &CalcInput<'_>, limit: &str, stat: &str, extra: f64) -> f64 {
    const MORE: &str = "support_undead_army_minion_maximum_count_+%_final";
    let build = input.build;
    let levels = GemLevels::none();
    let Ok(groups) = skill::active_groups(build) else {
        return 0.0;
    };
    let main_group = skill::main_skill(build, input.data, &levels)
        .ok()
        .map(|m| m.group - 1);
    let matches = sockets::socket_matches(build, &groups, input.data);
    let mut best: f64 = 0.0;
    for (g, group) in groups.iter().enumerate() {
        let slot = group.slot.as_ref().map(|s| s.as_str());
        if main_group != Some(g) && !(group.enabled && slot_enabled(build, slot)) {
            continue;
        }
        let group_matches = matches.get(g).map_or(&[][..], Vec::as_slice);
        for s in skill::group_skills(group, group_matches, input.data, &levels) {
            if s.global_off {
                continue;
            }
            let limited = |m: &&str| {
                super::pob_minion::minion(m)
                    .is_some_and(|m| m.limit == Some(limit) && m.hostile.is_none())
            };
            let own = super::pob_skill::skill(&s.active_gem.effect.id)
                .is_some_and(|p| p.minions.iter().any(limited));
            let added = s.supports.iter().any(|gem| {
                super::pob_skill::skill(&gem.effect.id)
                    .is_some_and(|p| p.add_minions.iter().any(limited))
            });
            if !own && !added {
                continue;
            }
            let mut base = extra;
            let mut more = 1.0;
            for gem in std::iter::once(&s.active_gem).chain(&s.supports) {
                let instance = skill::instance(gem.effect, gem.level, gem.quality, input.data);
                for (id, v) in &instance.stats {
                    #[expect(clippy::cast_precision_loss, reason = "a small count, exact as f64")]
                    let v = *v as f64;
                    if id == stat {
                        base += v;
                    } else if id == MORE {
                        more *= 1.0 + v / 100.0;
                    }
                }
            }
            if base != 0.0 {
                best = best.max(crate::math::floor(base * more));
            }
        }
    }
    best
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
pub fn support_multiplier(supports: &[GemEffectRef<'_>], cost_mores: &[f64]) -> f64 {
    let mut product = 1.0;
    // An item's multiplier for its socketed skills (P6-11bi): a MORE of the
    // value less 100, as a support's.
    for m in cost_mores {
        product *= 1.0 + (m - 100.0) / 100.0;
    }
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
/// Whether an active skill can wither (P6-11aq; PoB's
/// `Condition:CanWither` on a skill's mod list, `CalcPerform.lua` 2704):
/// `(a skill that is not a minion skill can, a minion skill can)`, `None`
/// when none can. The skills are [`affected_by`]'s; each can when
/// `everyone` (the character's own flag, which every skill's mod list
/// holds) or when its gem's or a support's stats at their levels hold a
/// stat of [`super::damage_taken::CAN_WITHER_STATS`] or its own of
/// [`super::damage_taken::CAN_WITHER_SKILL_STATS`] above 0. A minion skill
/// is one PoB gives a minion (`minionList` or `addMinionList`).
#[must_use]
pub fn can_wither(input: &CalcInput<'_>, everyone: bool) -> Option<(bool, bool)> {
    use super::damage_taken::{CAN_WITHER_SKILL_STATS, CAN_WITHER_STATS};
    let build = input.build;
    let levels = GemLevels::none();
    let groups = skill::active_groups(build).ok()?;
    let main_group = skill::main_skill(build, input.data, &levels)
        .ok()
        .map(|m| m.group - 1);
    let matches = sockets::socket_matches(build, &groups, input.data);
    let (mut player, mut minion) = (false, false);
    for (g, group) in groups.iter().enumerate() {
        let slot = group.slot.as_ref().map(|s| s.as_str());
        if main_group != Some(g) && !(group.enabled && slot_enabled(build, slot)) {
            continue;
        }
        if skill::attribute(&group.attributes, "source").is_some() {
            continue;
        }
        let group_matches = matches.get(g).map_or(&[][..], Vec::as_slice);
        for s in skill::group_skills(group, group_matches, input.data, &levels) {
            let id = s.active_gem.effect.id.as_str();
            let can = everyone
                || std::iter::once(s.active_gem)
                    .chain(s.supports.iter().copied())
                    .any(|gem| {
                        let instance =
                            skill::instance(gem.effect, gem.level, gem.quality, input.data);
                        instance.stats.iter().any(|(stat, value)| {
                            *value > 0
                                && (CAN_WITHER_STATS.contains(&stat.as_str())
                                    || CAN_WITHER_SKILL_STATS
                                        .iter()
                                        .any(|(e, st)| *e == gem.effect.id && st == stat))
                        })
                    });
            if !can {
                continue;
            }
            let is_minion = super::pob_skill::skill(id)
                .is_some_and(|p| !p.minions.is_empty() || !p.add_minions.is_empty());
            if is_minion {
                minion = true;
            } else {
                player = true;
            }
        }
    }
    (player || minion).then_some((player, minion))
}

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
/// One merged row: its value, source, row, and the state it needs.
type MergedRow = (f64, Source, HitEffect, Option<(&'static str, bool)>);

type Merged = BTreeMap<(&'static str, &'static str, usize), MergedRow>;

/// One curse of the skill list: its name, its source, and its scaled rows.
struct CurseApplied {
    name: &'static str,
    source: Source,
    /// Each row's value, row, and the state it needs (P6-9e).
    rows: Vec<(f64, HitEffect, Option<&'static str>)>,
    /// PoB's `determineCursePriority` ([`curse_priority`]).
    priority: i64,
    /// PoB's `isMark`: the skill is typed `Mark` (P6-9e).
    mark: bool,
}

/// PoB's base curse priorities (`Modules/Data.lua` 322 to 335,
/// `data.cursePriority`), by curse name: a later hex replaces an earlier
/// one of lower priority in a full slot.
const CURSE_PRIORITY: &[(&str, i64)] = &[
    ("Assassin's Mark", 11),
    ("Conductivity", 7),
    ("Despair", 8),
    ("Elemental Weakness", 4),
    ("Enfeeble", 2),
    ("Flammability", 5),
    ("Frostbite", 6),
    ("Poacher's Mark", 13),
    ("Punishment", 9),
    ("Sniper's Mark", 12),
    ("Temporal Chains", 1),
    ("Vulnerability", 3),
    ("Warlord's Mark", 10),
];

/// PoB's slot priorities (`data.cursePriority`): a gem socketed in that
/// item slot's group.
const CURSE_SLOT_PRIORITY: &[(&str, i64)] = &[
    ("Amulet", 2000),
    ("Body Armour", 5000),
    ("Boots", 7000),
    ("Gloves", 6000),
    ("Helmet", 3000),
    ("Ring 1", 8000),
    ("Ring 2", 9000),
    ("Ring 3", 10000),
    ("Weapon 1", 1000),
    ("Weapon 2", 4000),
];

/// PoB's `determineCursePriority` (`CalcPerform.lua` 563 to 593) for a
/// curse of `name` from a gem at `gem` (0-based) of a group in `slot`:
/// the curse's base priority, plus 100 per socket (the gem's position,
/// at most 8), plus the slot's (a swap slot counts as its slot), plus
/// 20000 from an aura. A group granted by an item (PoB's 11000) is not
/// modelled here ([`gems`] skips it).
fn curse_priority(name: &str, gem: usize, slot: Option<&str>, aura: bool) -> i64 {
    let base = CURSE_PRIORITY
        .iter()
        .find(|(n, _)| *n == name)
        .map_or(0, |(_, p)| *p);
    let socket = i64::try_from(gem.saturating_add(1).min(8)).unwrap_or(8) * 100;
    let slot = slot.map_or(0, |s| {
        let s = s.trim_end_matches(" Swap");
        CURSE_SLOT_PRIORITY
            .iter()
            .find(|(n, _)| *n == s)
            .map_or(0, |(_, p)| *p)
    });
    base + socket + slot + if aura { 20000 } else { 0 }
}

/// PoB's curse slots (`CalcPerform.lua` 3155 to 3212): the indexes of
/// `curses` (in the skill list's order) that hold the `limit` slots. While
/// a mark holds a slot, another mark takes none (`markSlotted`).
fn curse_slots(curses: &[CurseApplied], limit: usize) -> Vec<usize> {
    let mut slots: Vec<Option<usize>> = vec![None; limit];
    let mut mark_slotted = false;
    for (c, curse) in curses.iter().enumerate() {
        let mut slot = None;
        for (i, held) in slots.iter().enumerate() {
            if curse.mark && mark_slotted {
                slot = None;
                break;
            }
            match held {
                None => {
                    slot = Some(i);
                    break;
                }
                Some(h) if curses[*h].name == curse.name => {
                    slot = (curses[*h].priority < curse.priority).then_some(i);
                    break;
                }
                Some(h) if curses[*h].priority < curse.priority => slot = Some(i),
                Some(_) => {}
            }
        }
        if let Some(i) = slot {
            if slots[i].is_some_and(|h| curses[h].mark) {
                mark_slotted = false;
            }
            slots[i] = Some(c);
            if curse.mark {
                mark_slotted = true;
            }
        }
    }
    slots.into_iter().flatten().collect()
}

/// What the skills of the set collect before they are applied.
#[derive(Default)]
struct Collected {
    merged: Merged,
    curses: Vec<CurseApplied>,
    /// The guard buffs by name, each with its first skill (P6-9e).
    guards: BTreeMap<&'static str, Source>,
    /// The levels of the Summon Lightning Golem skills, each with its gem
    /// (Wrath, [`wrath`]).
    lightning_golems: Vec<(Source, i64)>,
    /// Whether a modelled Rallying Cry is in the list.
    rallying_cry: Option<Source>,
    /// Whether an Intimidating Cry's terms are in.
    intimidating: bool,
    /// The aura buffs that affect allies, by name, each with its first
    /// skill (P6-11er: `AurasAffectEnemies` puts them on the enemy too).
    ally_auras: BTreeMap<&'static str, Source>,
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
            // A group of PoB's own skills alone (P6-11eu): nothing unless
            // it is the main skill or in full DPS (module docs,
            // [`POB_ONLY_INERT`]).
            let pob_only = group.gems.typed().all(|gem| {
                !gem.enabled
                    || gem
                        .skill
                        .as_ref()
                        .is_some_and(|id| POB_ONLY_INERT.contains(&id.as_str()))
            });
            let full_dps = skill::attribute(&group.attributes, "includeInFullDPS") == Some("true");
            if pob_only && main_group != Some(g) && !full_dps {
                continue;
            }
            // An item's skill (P6-11fe) when its slot socket no supports
            // PoB would add to it: PoB's rebuilt group (module docs,
            // [`granted_view`]).
            let from_item = skill::attribute(&group.attributes, "source")
                .is_some_and(|s| s.starts_with("Item"));
            let grant = if from_item {
                if slot_supports(&groups, g, input.data) {
                    grant_supports(build, group)
                } else {
                    Some(false)
                }
            } else {
                None
            };
            if let Some(with_supports) = grant {
                let view = if with_supports {
                    granted_view_with_supports(group, &groups, g)
                } else {
                    granted_view(group)
                };
                let skills = skill::group_skills(&view, &[], input.data, levels);
                for s in &skills {
                    let is_main = main.is_some_and(|(mg, m)| {
                        mg == g
                            && m.active_gem.gem == s.active_gem.gem
                            && m.active_gem.effect.id == s.active_gem.effect.id
                    });
                    if is_main {
                        continue;
                    }
                    let place = GroupPlace { index: g, slot };
                    one_skill(input, place, s, totals, config, &mut collected, ingest);
                }
                continue;
            }
            for (i, gem) in group.gems.typed().enumerate() {
                if gem.enabled {
                    ingest.reason(UnsupportedReason::UnmodelledGem {
                        source: Source::Gem { group: g, gem: i },
                    });
                }
            }
            continue;
        }
        // A gem the data does not have grants nothing, as in PoB; it is a
        // reason (Phase 6).
        for i in skill::unknown_gems(group, input.data) {
            ingest.reason(UnsupportedReason::UnmodelledGem {
                source: Source::Gem { group: g, gem: i },
            });
        }
        let group_matches = matches.get(g).map_or(&[][..], Vec::as_slice);
        let skills = skill::group_skills(group, group_matches, input.data, levels);
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
                // Its reservation as any skill's (P6-11et; PoB's loop over
                // `activeSkillList`, the main skill included).
                reservation(s, &source, totals, ingest);
                // High-Impact Mine's double damage near mines (P6-11ez):
                // PoB stacks its debuff per active mine, the gem's
                // `skillMineCount` (`CalcActiveSkill.lua` 612 to 617); without
                // a count it has none, so only a count is a reason.
                if s.supports
                    .iter()
                    .any(|sup| sup.effect.id == "SupportHighImpactMine")
                    && std::iter::once(&s.active_gem)
                        .chain(&s.supports)
                        .filter_map(|r| group.gems.typed().nth(r.gem))
                        .any(|gem| {
                            skill::attribute(&gem.attributes, "skillMineCount")
                                .and_then(|v| v.parse::<f64>().ok())
                                .is_some_and(|n| n > 0.0)
                        })
                {
                    ingest.reason(UnsupportedReason::UnmodelledSkill {
                        source: source.clone(),
                        what: "active-mine-count",
                    });
                }
                // PoB's skill list holds the main skill too.
                ailment_overrides(input, s, source, ingest);
                socket_reasons(g, s, ingest);
                // Its supports' global effects (Arcane Surge's buff) act
                // as any skill's.
                supports_global(input, g, s, totals, config, &mut collected, ingest);
                // Its buff's degeneration acts on you as any buff's (open
                // items S7, part 8); the rest of the buff the hit reads.
                main_skill_degen(input, g, s, totals, config, &mut collected, ingest);
                continue;
            }
            let place = GroupPlace { index: g, slot };
            one_skill(input, place, s, totals, config, &mut collected, ingest);
        }
    }
    apply(input, config, totals, collected, ingest);
}

/// The group PoB makes of an item's skill (P6-11fe; `CalcSetup.lua` 1384
/// to 1402): its first gem alone, the active skill, at the granted level
/// the build stores, quality 0, with its first global effect on
/// (`enableGlobal1 = true`; PoB writes "nil" in the file).
fn granted_view(group: &SocketGroup) -> SocketGroup {
    let mut view = group.clone();
    let first = view.gems.typed().next().cloned();
    view.gems = loom_domain::Ordered::default();
    if let Some(mut gem) = first {
        gem.quality = loom_domain::Sourced::new(0);
        gem.attributes.retain(|a| a.name != "enableGlobal1");
        gem.attributes.push(loom_domain::Attribute {
            name: "enableGlobal1".to_owned(),
            value: "true".to_owned(),
        });
        gem.enabled = true;
        view.gems.push(gem);
    }
    view
}

/// [`granted_view`] with every gem of the slot's other groups (not of the
/// group's source) after the active gem (P6-11ff; `CalcSetup.lua` 1690 to
/// 1710): PoB adds each socketed support of the slot; the skill's matching
/// then keeps the supports that fit, the best of each kind
/// ([`skill::group_skills`]).
fn granted_view_with_supports(
    group: &SocketGroup,
    groups: &[&SocketGroup],
    g: usize,
) -> SocketGroup {
    let mut view = granted_view(group);
    let source = skill::attribute(&group.attributes, "source");
    for (o, other) in groups.iter().enumerate() {
        if o == g
            || other.slot != group.slot
            || skill::attribute(&other.attributes, "source") == source
        {
            continue;
        }
        for gem in other.gems.typed().filter(|gem| gem.enabled) {
            view.gems.push(gem.clone());
        }
    }
    view
}

/// Whether PoB adds the slot's supports to an item's granted skill
/// (P6-11ff; `ModParser.lua` 2046 to 2062, 3275 to 3367): `Some(true)` when
/// the item's grant lines all take supports ("Grants ...", "Trigger(s) /
/// Cast(s) Level # ...", "Use Level # ..."), `Some(false)` when they are
/// all `noSupports` ("Curse Enemies with ...", and the triggered grants
/// without a level: "Cast / Trigger ... on <event>", "... when Hit", "...
/// when your Skills or Minions Kill", "... when you take a Critical
/// Strike"), `None` when the item is not found, has no grant line, or has
/// both kinds (the build file does not say which line made the group).
fn grant_supports(build: &Build, group: &SocketGroup) -> Option<bool> {
    let source = skill::attribute(&group.attributes, "source")?;
    let id: u32 = source
        .strip_prefix("Item:")?
        .split(':')
        .next()?
        .parse()
        .ok()?;
    let item = build.items.item(loom_domain::ItemId(id))?;
    let mut kinds = std::collections::BTreeSet::new();
    for (_, m) in item.body.modifiers() {
        if m.selected
            && let Some(kind) = grant_kind(&m.text)
        {
            kinds.insert(kind);
        }
    }
    match kinds.len() {
        1 => kinds.into_iter().next(),
        _ => None,
    }
}

/// A grant line's kind ([`grant_supports`]): `Some(true)` when PoB adds
/// the slot's supports to the skill it grants, `Some(false)` for
/// `noSupports`, `None` for another line.
fn grant_kind(text: &str) -> Option<bool> {
    let t = text.to_lowercase();
    let triggered = (t.starts_with("cast") || t.starts_with("trigger") || t.starts_with("use"))
        && (t.contains(" on ") || t.contains(" when "));
    let levelled = t.contains(" level ");
    if t.starts_with("curse enemies with") || (triggered && !levelled) {
        Some(false)
    } else if t.starts_with("grants ") || (triggered && levelled) {
        Some(true)
    } else {
        None
    }
}

/// Whether another group in group `g`'s slot, not of its source, holds an
/// enabled support gem (P6-11fe): PoB adds every such support to an item's
/// skill unless the grant says `noSupports`, which the build file does not
/// keep (`CalcSetup.lua` 1690 to 1710). Such a group stays a reason.
fn slot_supports(groups: &[&SocketGroup], g: usize, data: &dyn crate::input::CalcData) -> bool {
    let group = groups[g];
    let Some(slot) = group.slot.as_ref() else {
        return false;
    };
    let source = skill::attribute(&group.attributes, "source");
    groups.iter().enumerate().any(|(o, other)| {
        o != g
            && other.slot.as_ref() == Some(slot)
            && skill::attribute(&other.attributes, "source") != source
            && other.gems.typed().any(|gem| {
                gem.enabled
                    && skill::gem_effects(gem, data)
                        .and_then(|e| e.first().copied())
                        .is_some_and(|e| e.support)
            })
    })
}

/// PoB's guard slots (`CalcPerform.lua` 3256 to 3275; module docs): the
/// guard names that apply. Two or more non-Vaal guards: PoB keeps the
/// first of its table order, which the engine does not know, so none of
/// them applies and each is a `guard-slot` reason.
fn guard_slots(
    guards: &BTreeMap<&'static str, Source>,
    ingest: &mut Ingest<'_>,
) -> BTreeSet<&'static str> {
    let (kept, unknown) = guard_pick(guards.keys().copied());
    for name in unknown {
        ingest.reason(UnsupportedReason::UnmodelledSkill {
            source: guards[name].clone(),
            what: "guard-slot",
        });
    }
    kept
}

/// [`guard_slots`]' choice: the guards that apply, and those whose slot
/// PoB's table order decides.
fn guard_pick(
    names: impl Iterator<Item = &'static str>,
) -> (BTreeSet<&'static str>, Vec<&'static str>) {
    const VAAL_MOLTEN_SHELL: &str = "Vaal Molten Shell";
    let names: BTreeSet<&'static str> = names.collect();
    if names.contains(VAAL_MOLTEN_SHELL) {
        return (BTreeSet::from([VAAL_MOLTEN_SHELL]), Vec::new());
    }
    let mut kept = BTreeSet::new();
    let mut other = Vec::new();
    for name in names {
        if name.starts_with("Vaal") {
            kept.insert(name);
        } else {
            other.push(name);
        }
    }
    if let [one] = other.as_slice() {
        kept.insert(*one);
        other.clear();
    }
    (kept, other)
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
        guards,
        lightning_golems,
        rallying_cry,
        ally_auras,
        ..
    } = collected;
    // Guards: PoB's slots (module docs).
    let kept = guard_slots(&guards, ingest);
    merged.retain(|(name, ..), _| !guards.contains_key(name) || kept.contains(name));
    // Vaal Smite's aura replaces Smite's (open items S2): every row of
    // Smite's aura has `Condition` `AffectedByVaalSmite` negated
    // (`act_str.lua` 10325 to 10334), which PoB sets while Vaal Smite's
    // aura applies (`CalcPerform.lua` 2417 to 2419).
    if let Some((_, row)) = merged
        .iter()
        .find(|((name, ..), _)| *name == VAAL_SMITE_AURA)
    {
        let vaal = row.1.clone();
        merged.retain(|(name, ..), _| *name != SMITE_AURA);
        // Smite as the main skill gives its aura through the hit's mapping
        // (P3-8l), which that condition would turn off: not modelled.
        let main_is_smite = skill::main_skill(input.build, input.data, &GemLevels::none())
            .is_ok_and(|m| m.effect().id == "Smite");
        if main_is_smite {
            ingest.reason(UnsupportedReason::UnmodelledSkill {
                source: vaal,
                what: "vaal-smite-over-smite",
            });
        }
    }
    // Lightning Golem's Wrath (module docs): merged as an aura of its name.
    wrath(
        input,
        config,
        totals,
        &lightning_golems,
        &mut merged,
        ingest,
    );
    let on_enemy = auras_on_enemy(totals, &ally_auras, &merged, ingest);
    for (value, source, (quantity, op, _, all, any), state) in merged.into_values().chain(on_enemy)
    {
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
    // Curses: PoB's slots (module docs). The limit is 1 and the character's
    // unconditional `CurseLimit` effects; a conditional one is a reason.
    let mut limit = 1.0;
    // Close-out M1: "while affected by Malevolence", settled from the
    // auras affecting the character.
    let malevolence = Conditions::none().with_state(
        crate::condition::ConditionKey::new(super::conditions::AFFECTED_BY_MALEVOLENCE),
        true,
    );
    let mut affected: Option<bool> = None;
    for e in ingest.effects() {
        if e.quantity == Quantity::Poe1(Poe1Quantity::CurseLimit) {
            if e.conditions == Conditions::none() {
                limit += e.value;
            } else if e.conditions == malevolence {
                let on = *affected.get_or_insert_with(|| {
                    affected_by(input, &super::gem_levels::GemLevels::none())
                        .contains("Malevolence")
                });
                if on {
                    limit += e.value;
                }
            } else {
                limit = f64::NAN;
            }
        }
    }
    // "Your Curse Limit is equal to your maximum Power Charges" (P6-11gr;
    // `CalcPerform.lua` 3156): the maximum power charges instead.
    let power_limit = Quantity::Poe1(Poe1Quantity::CurseLimitIsMaximumPowerCharges);
    let flagged: Vec<bool> = ingest
        .effects()
        .iter()
        .filter(|e| e.quantity == power_limit && e.op == Op::Flag && e.value != 0.0)
        .map(|e| e.conditions == Conditions::none())
        .collect();
    if flagged.contains(&false) {
        limit = f64::NAN;
    } else if !flagged.is_empty() {
        limit = super::charges::power_maximum(ingest, input.data).unwrap_or(f64::NAN);
    }
    if limit.is_nan() {
        for c in &curses {
            ingest.reason(UnsupportedReason::UnmodelledSkill {
                source: c.source.clone(),
                what: "curse-limit",
            });
        }
    } else {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a small count of curses, at least 0"
        )]
        let limit = limit.max(0.0) as usize;
        let held = curse_slots(&curses, limit);
        for (i, c) in curses.into_iter().enumerate() {
            if !held.contains(&i) {
                continue;
            }
            for (value, (quantity, op, _, all, any), state) in c.rows {
                let mut conditions = Conditions::none().with_all(all).with_any(any);
                if let Some(key) = state {
                    conditions = conditions.with_state(ConditionKey::new(key), true);
                }
                ingest.effect(Effect {
                    quantity,
                    op,
                    value,
                    conditions,
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

/// The aura rows on the enemy (P6-11er; `CalcPerform.lua` 2451 to 2464):
/// with `AurasAffectEnemies`, each aura that affects allies is also a
/// debuff on the enemy, its mods scaled by the skill's `AuraEffect` and
/// `BuffEffect` alone. The engine's merged rows carry the character's
/// scale, which adds the aura effect on you; when that is 0 the two are
/// equal, otherwise each aura is an `aura-on-enemy-scale` reason. A
/// resistance row is the enemy's resistance (`EnemyResistance`, which the
/// hit adds to the enemy's base); a maximum resistance row is nothing on
/// the enemy, as PoB's enemy maximum reads only the config and its
/// constant (`calcResistForType`); any other row is an `aura-on-enemy`
/// reason.
fn auras_on_enemy(
    totals: &Totals,
    ally_auras: &BTreeMap<&'static str, Source>,
    merged: &Merged,
    ingest: &mut Ingest<'_>,
) -> Vec<MergedRow> {
    let flag = Quantity::Poe1(Poe1Quantity::AurasAffectEnemies);
    let on = ingest
        .effects()
        .iter()
        .any(|e| e.quantity == flag && e.conditions == Conditions::none() && e.value != 0.0);
    if !on || ally_auras.is_empty() {
        return Vec::new();
    }
    if totals.aura_on_self != 0.0 {
        for source in ally_auras.values() {
            ingest.reason(UnsupportedReason::UnmodelledSkill {
                source: source.clone(),
                what: "aura-on-enemy-scale",
            });
        }
        return Vec::new();
    }
    let mut out = Vec::new();
    for ((name, _, _), (value, source, (quantity, op, scale, all, any), state)) in merged {
        let Some(aura) = ally_auras.get(name) else {
            continue;
        };
        match (quantity, op) {
            (Quantity::Resistance(t), Op::Base) => out.push((
                *value,
                source.clone(),
                (enemy_res(*t), Op::Base, *scale, *all, *any),
                *state,
            )),
            (Quantity::MaximumResistance(_), _) => {}
            _ => ingest.reason(UnsupportedReason::UnmodelledSkill {
                source: aura.clone(),
                what: "aura-on-enemy",
            }),
        }
    }
    out
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

/// The named golem buff effect for `s`'s buff (P6-11gc,
/// [`Totals::named_golem`]).
fn named_golem(totals: &Totals, s: &GroupSkill<'_>) -> f64 {
    totals.named_golem(s.active.map(|a| a.id.as_str()))
}

/// The named aura effect for `s`'s aura (P6-11gn, Pride's): 0 for a buff
/// that is not an aura.
fn named_aura(totals: &Totals, s: &GroupSkill<'_>, aura: bool) -> f64 {
    if aura {
        totals.named_skill(&s.active_gem.effect.id, skill_modifiers::NamedEffect::Aura)
    } else {
        0.0
    }
}

/// Whether PoB applies the skill's buff to the main skill's minion
/// (`CalcPerform.lua` 2282 and 2427): an aura (every self aura reaches
/// allies), or a buff with `skillData.buffMinions` or `buffAllies`.
fn reaches_minion(s: &GroupSkill<'_>) -> bool {
    s.types.contains("Aura")
        || super::pob_skill::skill(&s.active_gem.effect.id).is_some_and(|p| {
            ["buffMinions", "buffAllies"]
                .iter()
                .any(|k| p.skill_data(k).is_some_and(super::pob_skill::Value::truthy))
        })
}

/// The character's auras and buffs on the main skill's minion (P6-7b4;
/// `CalcPerform.lua` 2282 to 2290 for a buff, 2425 to 2438 for an aura):
/// each enabled skill (as [`gems`] walks them) whose buff reaches the
/// minion ([`reaches_minion`]) adds its rows, scaled as on the character
/// and merged per name keeping each row's largest value (`mergeBuff`
/// into `minionBuffs`). The scale is the character's ([`Totals`]): PoB's
/// minion scale leaves out the effect "on self" terms, and the engine
/// reads none of them. One the engine does not model, or with a support
/// it does not model, is a `minion-buffs` reason. Lightning Golem's Wrath
/// (a minion skill's aura, which reaches the minion too, 2775 to 2780)
/// is a reason when the config enables it.
pub fn minion_buffs(
    input: &CalcInput<'_>,
    levels: &GemLevels,
    totals: &Totals,
    ingest: &mut Ingest<'_>,
) {
    let build = input.build;
    let Ok(groups) = skill::active_groups(build) else {
        return;
    };
    let config = super::collect::active_config(build);
    let main_group = skill::main_skill(build, input.data, levels)
        .ok()
        .map(|m| m.group - 1);
    let matches = sockets::socket_matches(build, &groups, input.data);
    let mut merged = Merged::new();
    let reason = |source: Source| UnsupportedReason::UnmodelledSkill {
        source,
        what: "minion-buffs",
    };
    for (g, group) in groups.iter().enumerate() {
        let slot = group.slot.as_ref().map(|s| s.as_str());
        if main_group != Some(g) && !(group.enabled && slot_enabled(build, slot)) {
            continue;
        }
        // A granted group is the character's `unmodelled-gem` ([`gems`]),
        // which the minion's fields carry.
        if skill::attribute(&group.attributes, "source").is_some() {
            continue;
        }
        let group_matches = matches.get(g).map_or(&[][..], Vec::as_slice);
        for s in &skill::group_skills(group, group_matches, input.data, levels) {
            if !reaches_minion(s) {
                continue;
            }
            let source = Source::Gem {
                group: g,
                gem: s.active_gem.gem,
            };
            if s.active_gem.effect.id == WRATH_GOLEM {
                let on =
                    config.and_then(|set| last_input(set, WRATH_INPUT, ConfigInputKind::Input));
                if !matches!(on, None | Some(ConfigValue::Boolean(false))) {
                    ingest.reason(reason(source.clone()));
                }
            }
            match kind(&s.active_gem.effect.id) {
                Some(Kind::Buff {
                    name,
                    effect,
                    stats,
                }) => {
                    if s.global_off {
                        continue;
                    }
                    if s.supports
                        .iter()
                        .any(|sup| support(&sup.effect.id).is_none())
                    {
                        ingest.reason(reason(source));
                        continue;
                    }
                    let aura = effect == EffectType::Aura;
                    let aura_buff = aura && !s.types.contains("AuraAffectsEnemies");
                    // The offerings' effect (P6-11gg), not the effect on you.
                    let own = own_effect(input, s, aura)
                        + named_golem(totals, s)
                        + named_aura(totals, s, aura)
                        + totals.offering(&s.active_gem.effect.id, false);
                    let scale = totals.buff_scale(aura, &s.types, own);
                    // The extra aura modifiers (P6-11gu), on the minion too.
                    if aura_buff {
                        extra_aura_rows(name, &source, scale, &mut merged, ingest);
                    }
                    add_rows(
                        input,
                        g,
                        &s.active_gem,
                        name,
                        stats,
                        (scale, scale),
                        config,
                        true,
                        false,
                        None,
                        &mut merged,
                        ingest,
                    );
                }
                _ => ingest.reason(reason(source)),
            }
        }
    }
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
}

/// Where a skill's group is: its index among the active set's groups
/// (0-based) and its item slot (the curse priority reads it).
#[derive(Debug, Clone, Copy)]
struct GroupPlace<'a> {
    index: usize,
    slot: Option<&'a str>,
}

/// One active skill of the group at `place` (module docs).
/// The main skill's buff rows of [`Poe1Quantity::Degen`] (open items S7,
/// part 8): PoB applies the main skill's `GlobalEffect` buff to you as any
/// skill's (Righteous Fire's degeneration as the main skill), scaled as
/// [`one_skill`] scales a buff; the buff's other stats the hit reads.
fn main_skill_degen(
    input: &CalcInput<'_>,
    g: usize,
    s: &GroupSkill<'_>,
    totals: &Totals,
    config: Option<&ConfigSet>,
    collected: &mut Collected,
    ingest: &mut Ingest<'_>,
) {
    let Some(Kind::Buff {
        name,
        effect: EffectType::Buff,
        stats,
    }) = kind(&s.active_gem.effect.id)
    else {
        return;
    };
    if s.global_off {
        return;
    }
    let on_you = totals.offering_on_you(&s.active_gem.effect.id);
    let own = own_effect(input, s, false)
        + named_golem(totals, s)
        + named_aura(totals, s, false)
        + totals.offering(&s.active_gem.effect.id, on_you);
    let scale = if UNSCALABLE.contains(&s.active_gem.effect.id.as_str()) {
        1.0
    } else {
        totals.buff_scale(false, &s.types, own)
    };
    let mut rows = Merged::new();
    add_rows(
        input,
        g,
        &s.active_gem,
        name,
        stats,
        (scale, scale),
        config,
        false,
        on_you,
        None,
        &mut rows,
        ingest,
    );
    for (key, row) in rows {
        if matches!(row.2.0, Quantity::Poe1(Poe1Quantity::Degen(..))) {
            collected.merged.entry(key).or_insert(row);
        }
    }
}

fn one_skill(
    input: &CalcInput<'_>,
    place: GroupPlace<'_>,
    s: &GroupSkill<'_>,
    totals: &Totals,
    config: Option<&ConfigSet>,
    collected: &mut Collected,
    ingest: &mut Ingest<'_>,
) {
    let GroupPlace { index: g, slot } = place;
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
    ailment_overrides(input, s, active_source.clone(), ingest);
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
    // A skill that requires a shield is disabled without one in the off
    // hand (`CalcActiveSkill.lua` 269 to 273): no buff, its supports' none
    // (`CalcPerform.lua` 2227 skips a disabled skill's buffs), but it still
    // reserves (the reservation loop, 1939, does not check; P6-9e).
    if s.types.contains("RequiresShield") {
        let disabled = match super::collect::hands(input).off_hand {
            super::collect::Held::Shield => false,
            super::collect::Held::Unknown => {
                ingest.reason(UnsupportedReason::UnmodelledSkill {
                    source: active_source.clone(),
                    what: "requires-shield",
                });
                true
            }
            super::collect::Held::Empty
            | super::collect::Held::Weapon(_)
            | super::collect::Held::Other => true,
        };
        if disabled {
            reservation(s, &active_source, totals, ingest);
            return;
        }
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
        match intimidating_terms(input, s, totals) {
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
            // An offering's buff on you (P6-11gg, Mistress of Sacrifice):
            // its `MinionOnly` rows apply, with the effect on you.
            let on_you = totals.offering_on_you(&s.active_gem.effect.id);
            // PoB's guard scale reads the character's modifiers alone
            // (`modStore` the `modDB`): no gem's own effect stat.
            let own = if effect == EffectType::Guard {
                0.0
            } else {
                own_effect(input, s, aura)
                    + named_golem(totals, s)
                    + named_aura(totals, s, aura)
                    + totals.offering(&s.active_gem.effect.id, on_you)
            };
            // An `unscalable` buff (Cyclone's movement speed, P6-8a): scale 1.
            let scale = if UNSCALABLE.contains(&s.active_gem.effect.id.as_str()) {
                1.0
            } else {
                totals.buff_scale(aura, &s.types, own)
            };
            // An aura skill's `Buff` rows (Sand Stance) take the buff effect.
            let buff_scale = if aura && !UNSCALABLE.contains(&s.active_gem.effect.id.as_str()) {
                totals.buff_scale(
                    false,
                    &s.types,
                    own_effect(input, s, false) + named_golem(totals, s),
                )
            } else {
                scale
            };
            if effect == EffectType::Guard {
                collected
                    .guards
                    .entry(name)
                    .or_insert_with(|| active_source.clone());
            }
            // An aura on you and your allies (PoB's `buff.type == "Aura"`,
            // `CalcPerform.lua` 2451): not one that affects enemies itself.
            if aura && !s.types.contains("AuraAffectsEnemies") {
                collected
                    .ally_auras
                    .entry(name)
                    .or_insert_with(|| active_source.clone());
            }
            // Generosity (P6-11ft; PoB's `auraCannotAffectSelf`,
            // `CalcPerform.lua` 2409): the aura affects your allies and
            // minions, not you.
            let not_on_self = aura
                && s.supports
                    .iter()
                    .any(|sup| AURA_NOT_ON_SELF_SUPPORTS.contains(&sup.effect.id.as_str()));
            // The extra aura modifiers (P6-11gu; `CalcPerform.lua` 2394 to
            // 2420): each aura buff on you adds them at its own scale.
            if aura && !not_on_self && !s.types.contains("AuraAffectsEnemies") {
                extra_aura_rows(name, &active_source, scale, &mut collected.merged, ingest);
            }
            if !not_on_self {
                add_rows(
                    input,
                    g,
                    &s.active_gem,
                    name,
                    stats,
                    (scale, buff_scale),
                    config,
                    false,
                    on_you,
                    None,
                    &mut collected.merged,
                    ingest,
                );
            }
        }
        Kind::Debuff { name, stats } => {
            // An aura's debuff (P6-11fg, PoB's `AuraDebuff`) takes the non-curse
            // aura effect, which the engine reads: with one, a reason (its
            // reservation below still applies).
            if s.types.contains("Aura") && totals.aura_effect != 0.0 {
                ingest.reason(UnsupportedReason::UnmodelledSkill {
                    source: active_source.clone(),
                    what: "aura-debuff-effect",
                });
            } else {
                // `DebuffEffect` is a reason ([`DEPENDS_ON`]): scale 1.
                add_rows(
                    input,
                    g,
                    &s.active_gem,
                    name,
                    stats,
                    (1.0, 1.0),
                    config,
                    false,
                    false,
                    None,
                    &mut collected.merged,
                    ingest,
                );
            }
        }
        Kind::Warcry { name, stats } => {
            // Autoexertion's warcries share no buff (P6-11dy; PoB's
            // `CannotShareWarcryBuffs` flag, `CalcPerform.lua` 2326): their
            // rows do not apply to you. Their exerts and reservation are read
            // elsewhere, as for any warcry.
            // P6-11hw: Call to Arms' "Your Warcries do not grant Buffs or
            // Charges to You" (PoB's `CannotGainWarcryBuffs`, 2338): no
            // buff on you, its exerts and reservation as any warcry's.
            if totals.cannot_gain_warcry_buffs
                || s.supports
                    .iter()
                    .any(|sup| sup.effect.id == AUTOEXERTION_SUPPORT)
            {
                supports_global(input, g, s, totals, config, collected, ingest);
                reservation(s, &active_source, totals, ingest);
                return;
            }
            // Its buff effect, power, and uptime (P6-11bs): without them
            // (a level missing, a value not read) its power rows are a
            // reason and its other rows act at scale 1, which no slice
            // field reads (a test).
            let terms = warcry_terms(input, s, totals, config);
            let scale = match terms {
                Ok(t) => t.scale,
                Err(what) => {
                    ingest.reason(UnsupportedReason::UnmodelledSkill {
                        source: active_source.clone(),
                        what,
                    });
                    1.0
                }
            };
            add_rows(
                input,
                g,
                &s.active_gem,
                name,
                stats,
                (scale, 1.0),
                config,
                false,
                false,
                terms.ok(),
                &mut collected.merged,
                ingest,
            );
        }
        Kind::Curse { name, stats } => {
            let instance = skill::instance(
                s.active_gem.effect,
                s.active_gem.level,
                s.active_gem.quality,
                input.data,
            );
            // The gem's own curse effect (its `skillModList`'s `CurseEffect`
            // INC, P6-9b).
            #[expect(
                clippy::cast_precision_loss,
                reason = "stat values are small integers, exact as f64"
            )]
            let own: f64 = instance
                .stats
                .iter()
                .filter(|(stat, _)| stat == OWN_CURSE_EFFECT_STAT)
                .map(|(_, v)| *v as f64)
                .sum::<f64>()
                // Mark On Hit's `CurseEffect` INC on the mark it supports
                // (P6-9g; PoB's `SkillStatMap.lua` 2107, `SkillType.Mark`:
                // the support needs a mark). PoB reads no other stat of it.
                + s.supports
                    .iter()
                    .flat_map(|sup| {
                        skill::instance(sup.effect, sup.level, sup.quality, input.data).stats
                    })
                    .filter(|(stat, _)| stat == MARK_CURSE_EFFECT_STAT)
                    .map(|(_, v)| v as f64)
                    .sum::<f64>();
            // The supports' `CurseEffect` MORE (Blasphemy's, P6-11r): PoB's
            // `calcLib.mod` multiplies the increase by each more.
            #[expect(
                clippy::cast_precision_loss,
                reason = "stat values are small integers, exact as f64"
            )]
            let more: f64 = s
                .supports
                .iter()
                .flat_map(|sup| {
                    skill::instance(sup.effect, sup.level, sup.quality, input.data).stats
                })
                .filter(|(stat, _)| {
                    stat == BLASPHEMY_CURSE_EFFECT_STAT || stat == HEXTOUCH_CURSE_EFFECT_STAT
                })
                .map(|(_, v)| 1.0 + v as f64 / 100.0)
                .product();
            // A mark's own increase (P6-11df: "Effect of your Marks").
            let own = if s.types.contains("Mark") {
                own + totals.mark_curse_effect
            } else {
                own
            };
            // A named curse's own increase (P6-11gn: Vulnerability's).
            let own = own
                + totals.named_skill(&s.active_gem.effect.id, skill_modifiers::NamedEffect::Curse);
            let scale = totals.curse_scale_with(own) * more;
            if !instance.problems.is_empty() {
                for p in &instance.problems {
                    ingest.reason(UnsupportedReason::UnmodelledSkill {
                        source: active_source.clone(),
                        what: p.name(),
                    });
                }
            } else {
                let mut rows = Vec::new();
                // The supports' rows on the hex (P6-11fl, Hexpass: PoB's
                // `GlobalEffect` `effectType = "Curse"` on the support's
                // stat joins the curse it supports).
                for sup in &s.supports {
                    let Some((_, sup_stats)) =
                        CURSE_SUPPORTS.iter().find(|(id, _)| *id == sup.effect.id)
                    else {
                        continue;
                    };
                    for (stat, value) in
                        skill::instance(sup.effect, sup.level, sup.quality, input.data).stats
                    {
                        let Some(&(_, BuffStat::Rows(effects))) =
                            sup_stats.iter().find(|(id, _)| *id == stat)
                        else {
                            continue;
                        };
                        #[expect(
                            clippy::cast_precision_loss,
                            reason = "stat values are small integers, exact as f64"
                        )]
                        let value = value as f64;
                        for &row in effects {
                            let (quantity, op, divisor, ..) = row;
                            rows.push((
                                skill_modifiers::scale(quantity, op, value / divisor, scale),
                                row,
                                None,
                            ));
                        }
                    }
                }
                for (stat, value) in &instance.stats {
                    let (effects, state) = match stats.iter().find(|(id, _)| id == stat) {
                        Some(&(_, BuffStat::Rows(effects))) => (effects, None),
                        Some(&(_, BuffStat::WhenState { state, rows, .. })) => (rows, Some(state)),
                        _ => continue,
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
                            state,
                        ));
                    }
                }
                collected.curses.push(CurseApplied {
                    name,
                    source: active_source.clone(),
                    rows,
                    priority: curse_priority(
                        name,
                        s.active_gem.gem,
                        slot,
                        s.types.contains("Aura"),
                    ),
                    mark: s.types.contains("Mark"),
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
        // Combustion only from a skill that can ignite (PoB's `CannotIgnite`
        // check, `CalcPerform.lua` 2712; [`cannot_ignite`]).
        if sup.effect.id == "SupportCombustion" && cannot_ignite(input, s) {
            continue;
        }
        let name = support_buff_name(&sup.effect.id);
        // Arcane Surge's mods are `unscalable` (`sup_int.lua` 229 to 233):
        // scale 1. A support's debuff takes `DebuffEffect` alone, a reason
        // ([`DEPENDS_ON`]): scale 1 (P6-11fu).
        let scale = if sup.effect.id == "SupportArcaneSurge"
            || DEBUFF_SUPPORTS.contains(&sup.effect.id.as_str())
        {
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
            (scale, scale),
            config,
            false,
            false,
            None,
            &mut collected.merged,
            ingest,
        );
    }
}

/// Supports whose global stats are a debuff on the enemy (P6-11fu, Maim;
/// P6-11hr, Combustion): not scaled by buff effect.
const DEBUFF_SUPPORTS: &[&str] = &["SupportCombustion", "SupportMaim"];

/// Whether `stat` is one of a modelled support's global stats
/// ([`SUPPORTS`]): the main skill's supports apply them as any skill's do
/// (`supports_global`), so on the skill itself they have no effect
/// (`mapping::SkillMapping`; P6-11y).
#[must_use]
pub fn is_support_global_stat(stat: &str) -> bool {
    SUPPORTS
        .iter()
        .any(|(_, table)| table.iter().any(|(id, _)| *id == stat))
}

/// The buff name of a support's global effect (its tag's `effectName`).
fn support_buff_name(id: &str) -> &'static str {
    match id {
        "SupportArcaneSurge" => "Arcane Surge",
        "SupportInfusedChannelling" => "Infusion",
        "SupportMomentum" => "Momentum",
        "SupportCombustion" => "Combustion",
        "SupportMaim" => "Maim",
        "SupportSummonPhantasm" => "Phantasmal Might",
        _ => "support",
    }
}

/// One gem instance's rows of `table` into `merged` at `scales`: the
/// skill's own effect scale, and the buff effect scale its
/// [`BuffStat::SandStance`] rows take.
#[expect(clippy::too_many_arguments, reason = "the merge's inputs")]
fn add_rows(
    input: &CalcInput<'_>,
    g: usize,
    r: &GemEffectRef<'_>,
    name: &'static str,
    table: &'static [(&'static str, BuffStat)],
    (scale, buff_scale): (f64, f64),
    config: Option<&ConfigSet>,
    for_minion: bool,
    minion_only_on_you: bool,
    warcry: Option<WarcryTerms>,
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
        let mut row_scale = scale;
        let mut count = 1.0;
        let (rows, state): (&[HitEffect], Option<(&'static str, bool)>) = match b {
            BuffStat::Rows(rows) => (rows, None),
            // Only a warcry with its terms (P6-11bs): `(1 + inc / 100) x
            // more x bonus x uptime` (`CalcPerform.lua` 2347).
            BuffStat::PerPower { div, limit, rows } => {
                let Some(w) = warcry else {
                    continue;
                };
                let bonus = crate::math::floor(crate::math::min(w.power, limit) / div);
                row_scale = scale * bonus * w.uptime;
                (rows, None)
            }
            BuffStat::Unscalable(rows) => {
                row_scale = 1.0;
                (rows, None)
            }
            BuffStat::NoSlice(_) | BuffStat::Gated(_) => continue,
            BuffStat::Defence(category) => {
                if *value != 0 {
                    // A guard is a pool; the rest change the hit.
                    let quantity = if category == "guard" {
                        Poe1Quantity::PoolReason(id)
                    } else {
                        Poe1Quantity::EnemyHitReason(id)
                    };
                    ingest.effect(Effect {
                        quantity: Quantity::Poe1(quantity),
                        op: Op::Flag,
                        value: 1.0,
                        conditions: Conditions::none(),
                        source: source.clone(),
                    });
                }
                continue;
            }
            BuffStat::AtCount {
                input: counted,
                default,
                at,
                rows,
                ..
            } => {
                let read = config
                    .and_then(|set| last_input(set, counted, ConfigInputKind::Input))
                    .or_else(|| {
                        config
                            .and_then(|set| last_input(set, counted, ConfigInputKind::Placeholder))
                    });
                let n = match read {
                    Some(ConfigValue::Number(n)) => n.value().0,
                    None => f64::from(default),
                    Some(_) => {
                        ingest.reason(UnsupportedReason::UnmodelledConfig {
                            input: counted.to_owned(),
                        });
                        continue;
                    }
                };
                if n < f64::from(at) {
                    continue;
                }
                row_scale = 1.0;
                (rows, None)
            }
            BuffStat::Reason(what) => {
                if *value != 0 {
                    ingest.reason(UnsupportedReason::UnmodelledSkill {
                        source: source.clone(),
                        what,
                    });
                }
                continue;
            }
            BuffStat::ReasonWhenInput { input: check, what } => {
                let on = config.and_then(|set| last_input(set, check, ConfigInputKind::Input));
                // A count at 0 is off too (P6-11fk: Frost Shield's stages).
                let off = match on {
                    None | Some(ConfigValue::Boolean(false)) => true,
                    Some(ConfigValue::Number(n)) => n.value().0 == 0.0,
                    _ => false,
                };
                if *value != 0 && !off {
                    ingest.reason(UnsupportedReason::UnmodelledSkill {
                        source: source.clone(),
                        what,
                    });
                }
                continue;
            }
            BuffStat::WhenInput {
                input: check, rows, ..
            }
            | BuffStat::CheckModCond {
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
            BuffStat::EnemyCheck {
                input: check,
                neg,
                rows,
                ..
            } => match config.and_then(|set| last_input(set, check, ConfigInputKind::Input)) {
                None | Some(ConfigValue::Boolean(false)) if neg => (rows, None),
                Some(ConfigValue::Boolean(true)) if !neg => (rows, None),
                None | Some(ConfigValue::Boolean(_)) => continue,
                Some(_) => {
                    ingest.reason(UnsupportedReason::UnmodelledConfig {
                        input: check.to_owned(),
                    });
                    continue;
                }
            },
            BuffStat::BloodStance(rows) => {
                if !stance_blood {
                    continue;
                }
                (rows, None)
            }
            BuffStat::SandStance(rows) => {
                if stance_blood || for_minion {
                    continue;
                }
                row_scale = buff_scale;
                (rows, None)
            }
            BuffStat::PerCount {
                input: counted,
                limit,
                default,
                rows,
                ..
            } => {
                // An absent input with a default: the saved placeholder,
                // else the default (PoB's `BuildModList`).
                let mut read =
                    config.and_then(|set| last_input(set, counted, ConfigInputKind::Input));
                if read.is_none() && default.is_some() {
                    read = config
                        .and_then(|set| last_input(set, counted, ConfigInputKind::Placeholder));
                }
                let raw = match read {
                    Some(ConfigValue::Number(n)) if n.value().0.is_finite() => n.value().0,
                    None => match default {
                        Some(d) => f64::from(d),
                        None => continue,
                    },
                    Some(_) => {
                        ingest.reason(UnsupportedReason::UnmodelledConfig {
                            input: counted.to_owned(),
                        });
                        continue;
                    }
                };
                if raw == 0.0 {
                    continue;
                }
                count = raw;
                if let Some(limit) = limit {
                    count = crate::math::min(count, f64::from(limit));
                }
                (rows, None)
            }
            BuffStat::PerStack {
                input: counted,
                limit,
                rows,
                ..
            } => {
                match config.and_then(|set| last_input(set, counted, ConfigInputKind::Input)) {
                    Some(ConfigValue::Number(n))
                        if n.value().0.is_finite() && n.value().0 > 0.0 =>
                    {
                        count = crate::math::min(n.value().0, f64::from(limit));
                    }
                    Some(ConfigValue::Number(n)) if n.value().0 <= 0.0 => continue,
                    None => continue,
                    Some(_) => {
                        ingest.reason(UnsupportedReason::UnmodelledConfig {
                            input: counted.to_owned(),
                        });
                        continue;
                    }
                }
                (rows, None)
            }
            BuffStat::AtMaxStages {
                part, stages, rows, ..
            } => {
                if !at_max_stages(input, g, r.gem, part, stages) {
                    continue;
                }
                (rows, None)
            }
            BuffStat::Threshold(rows) => (rows, Some((STAT_THRESHOLD, true))),
            BuffStat::LifeThreshold(rows) => (rows, Some((LIFE_THRESHOLD, true))),
            BuffStat::WhenState { state, rows, .. } => {
                if UNSCALABLE_STATS.contains(&id) {
                    row_scale = 1.0;
                }
                (rows, Some((state, true)))
            }
            BuffStat::MinionOnly(rows) => {
                if !for_minion && !minion_only_on_you {
                    continue;
                }
                (rows, None)
            }
            BuffStat::ConfigIs {
                input: list,
                value: wanted,
                equal,
                rows,
                ..
            } => {
                let is = match config.and_then(|set| last_input(set, list, ConfigInputKind::Input))
                {
                    None => false,
                    Some(ConfigValue::String(s)) => s == wanted,
                    Some(_) => {
                        ingest.reason(UnsupportedReason::UnmodelledConfig {
                            input: list.to_owned(),
                        });
                        continue;
                    }
                };
                if is != equal {
                    continue;
                }
                (rows, None)
            }
        };
        if let Some(w) = warcry
            && !matches!(b, BuffStat::PerPower { .. })
        {
            row_scale = row_scale * 1.0 * w.uptime;
        }
        #[expect(
            clippy::cast_precision_loss,
            reason = "stat values are small integers, exact as f64"
        )]
        let value = *value as f64;
        for (i, &row) in rows.iter().enumerate() {
            let (quantity, op, divisor, ..) = row;
            let scaled = skill_modifiers::scale(quantity, op, value / divisor, row_scale) * count;
            let entry = merged
                .entry((name, id, i))
                .or_insert((scaled, source.clone(), row, state));
            if scaled > entry.0 {
                *entry = (scaled, source.clone(), row, state);
            }
        }
    }
}

/// The speed every aura buff adds ("Auras from your Skills grant #%
/// increased Attack and Cast Speed to you and Allies", P6-11gu): PoB adds
/// each `ExtraAuraEffect` mod to the aura's buff, scaled by its multiplier
/// and merged per aura name. The unconditional sum is the row's value; a
/// conditional one is a reason.
pub const EXTRA_AURA_SPEED: &str = "auras_grant_attack_and_cast_speed_+%_to_you_and_your_allies";

fn extra_aura_rows(
    name: &'static str,
    source: &Source,
    scale: f64,
    merged: &mut Merged,
    ingest: &mut Ingest<'_>,
) {
    let quantity = Quantity::Poe1(Poe1Quantity::ExtraAuraSpeed);
    let mut sum = 0.0;
    let mut unread = false;
    for e in ingest.effects().iter().filter(|e| e.quantity == quantity) {
        if e.op == Op::Base && e.conditions == Conditions::none() {
            sum += e.value;
        } else {
            unread = true;
        }
    }
    if unread {
        ingest.reason(UnsupportedReason::UnmodelledSkill {
            source: source.clone(),
            what: "extra-aura-effect",
        });
        return;
    }
    if sum == 0.0 {
        return;
    }
    let row: HitEffect = (Quantity::Speed, Op::Increased, 1.0, NONE, NONE);
    let scaled = skill_modifiers::scale(Quantity::Speed, Op::Increased, sum, scale);
    let entry =
        merged
            .entry((name, EXTRA_AURA_SPEED, 0))
            .or_insert((scaled, source.clone(), row, None));
    if scaled > entry.0 {
        *entry = (scaled, source.clone(), row, None);
    }
}

/// Whether the gem at `gem` of group `g` is at its maximum stages
/// ([`BuffStat::AtMaxStages`]): its `skillPart` (PoB's `m_min(#parts,
/// skillPart or 1)`, the parts at most `part`) is `part`, or its
/// `skillStageCount` (missing: the maximum; PoB's `m_max(count, 1)`) is
/// at least `stages`.
fn at_max_stages(input: &CalcInput<'_>, g: usize, gem: usize, part: usize, stages: u32) -> bool {
    let Ok(groups) = skill::active_groups(input.build) else {
        return true;
    };
    let Some(x) = groups.get(g).and_then(|group| group.gems.typed().nth(gem)) else {
        return true;
    };
    let number = |name| skill::attribute(&x.attributes, name).and_then(|v| v.parse::<f64>().ok());
    if number("skillPart").is_some_and(|p| p >= part as f64) {
        return true;
    }
    number("skillStageCount").is_none_or(|c| c.max(1.0) >= f64::from(stages))
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
/// Spellslinger's support part: a skill it supports takes "of Skills
/// Supported by Spellslinger" reservation efficiency (open items S3).
const SPELLSLINGER_SUPPORT: &str = "SupportSpellslinger";

fn reserves(types: &BTreeSet<String>) -> bool {
    types.contains("HasReservation") && !types.contains("ReservationBecomesCost")
}

/// Summon Skitterbots' granted effect.
pub const SKITTERBOTS: &str = "SummonSkitterbots";

/// PoB's choice between a skill's ailment overrides (`CalcPerform.lua`
/// 1360 to 1399): Summon Skitterbots' own branch ([`skitterbots`]), else a
/// chilling area's or a non-hit chill's guaranteed chill
/// ([`guaranteed_chill`]).
fn ailment_overrides(
    input: &CalcInput<'_>,
    s: &GroupSkill<'_>,
    source: Source,
    ingest: &mut Ingest<'_>,
) {
    if s.active_gem.effect.id == SKITTERBOTS {
        skitterbots(input, s, source, ingest);
    } else {
        guaranteed_chill(input, s, source, ingest);
    }
}

/// Summon Skitterbots' shock and chill of the enemy (open items; PoB's
/// `CalcPerform.lua` 1360 to 1383): `ShockOverride` BASE `15 x (1 + (inc +
/// SkitterbotAilmentEffect) / 100)` and `ChillOverride` BASE `10 x (...)`,
/// `inc` the skill's own `EnemyShockEffect` or `EnemyChillEffect` INC (its
/// mods of source `Skill`: the gem's `shock_effect_+%` and
/// `chill_effect_+%` at its level), each marking the enemy shocked or
/// chilled. No effect modifier scales an override afterwards (3509 to
/// 3519). The `inc` is every `Skill` mod of the skill: the gem's and its
/// supports' stats ([`SHOCK_EFFECT_STATS`], [`CHILL_EFFECT_STATS`]; open
/// items S2: Unbound Ailments' `non_damaging_ailment_effect_+%`, Bonechill's
/// `chill_effect_+%`). Bonechill on it (`supportBonechill`) gives the
/// guaranteed Bonechill (1357 to 1358, 1379 to 1380; [`BONECHILL_SUPPORT`]):
/// the chill raises the enemy's cold damage taken (3444); its
/// `SkitterbotBonechill` flag only reads the chill on you (844), which the
/// modifiers that make the auras affect you start. Those modifiers and the
/// ones that stop or change the auras (`SkitterbotsCannotShock`,
/// `SkitterbotAilmentEffect`, `ScorchingSkitterbot`,
/// `SkitterbotAffectPlayer`: unmapped stats) are reasons.
fn skitterbots(input: &CalcInput<'_>, s: &GroupSkill<'_>, source: Source, ingest: &mut Ingest<'_>) {
    // The gem's and its supports' `Skill` mods (PoB's `{ source = "Skill" }`).
    let instances: Vec<_> = std::iter::once(&s.active_gem)
        .chain(&s.supports)
        .map(|r| skill::instance(r.effect, r.level, r.quality, input.data))
        .collect();
    let sum = |ids: &[&str]| -> f64 {
        instances
            .iter()
            .flat_map(|i| &i.stats)
            .filter(|(id, _)| ids.contains(&id.as_str()))
            .map(|(_, v)| {
                #[expect(clippy::cast_precision_loss, reason = "gem stat values, exact as f64")]
                let v = *v as f64;
                v
            })
            .sum()
    };
    if s.supports
        .iter()
        .any(|sup| sup.effect.id == BONECHILL_SUPPORT)
    {
        ingest.effect(Effect {
            quantity: Quantity::Poe1(Poe1Quantity::Bonechill(true)),
            op: Op::Flag,
            value: 1.0,
            conditions: Conditions::none(),
            source: source.clone(),
        });
    }
    for (quantity, default, effect) in [
        (
            Poe1Quantity::SkillShockOverride,
            super::damage_taken::SHOCK_DEFAULT,
            sum(&SHOCK_EFFECT_STATS),
        ),
        (
            Poe1Quantity::SkillChillOverride,
            super::damage_taken::CHILL_DEFAULT,
            sum(&CHILL_EFFECT_STATS),
        ),
    ] {
        ingest.effect(Effect {
            quantity: Quantity::Poe1(quantity),
            op: Op::Base,
            value: default * (1.0 + effect / 100.0),
            conditions: Conditions::none(),
            source: source.clone(),
        });
    }
}

/// Whether an active Summon Skitterbots shocks the enemy (open items;
/// `CalcPerform.lua` 1365: `Condition:Shocked` on the enemy):
/// `Some(true)` when the active skill set has one, `None` when the
/// skills are not certain.
#[must_use]
pub fn skitterbots_shock(input: &CalcInput<'_>) -> Option<bool> {
    let levels = super::gem_levels::GemLevels::none();
    let groups = skill::active_groups(input.build).ok()?;
    let main_group = skill::main_skill(input.build, input.data, &levels)
        .ok()
        .map(|m| m.group - 1);
    Some(groups.iter().enumerate().any(|(g, group)| {
        let slot = group.slot.as_ref().map(|s| s.as_str());
        (main_group == Some(g) || (group.enabled && slot_enabled(input.build, slot)))
            && group.gems.typed().any(|gem| {
                gem.enabled
                    && gem
                        .skill
                        .as_ref()
                        .is_some_and(|id| id.as_str() == SKITTERBOTS)
            })
    }))
}

/// The skill type that gives a guaranteed chill whatever the skill's
/// modifiers (PoB's `ChillingArea`, `CalcPerform.lua` 1393).
pub const CHILLING_AREA: &str = "ChillingArea";

/// Supports whose aura does not affect you (PoB's `auraCannotAffectSelf`
/// from `aura_cannot_affect_self`; P6-11ft): Generosity.
const AURA_NOT_ON_SELF_SUPPORTS: &[&str] = &["SupportGenerosity"];

/// Bonechill Support's granted effect (P6-11fv).
const BONECHILL_SUPPORT: &str = "SupportBonechill";

/// Smite's aura's buff name.
const SMITE_AURA: &str = "Smite";
/// Vaal Smite's aura's buff name, which replaces Smite's ([`apply`]).
const VAAL_SMITE_AURA: &str = "Vaal Smite";

/// The stats that give a skill's own `EnemyShockEffect` INC
/// (`SkillStatMap.lua` 1091 to 1107; open items S2).
const SHOCK_EFFECT_STATS: [&str; 3] = [
    "lightning_ailment_effect_+%",
    "non_damaging_ailment_effect_+%",
    "shock_effect_+%",
];

/// The stats that give a skill's own `EnemyChillEffect` INC
/// (`SkillStatMap.lua` 1085 to 1119; open items S2).
const CHILL_EFFECT_STATS: [&str; 3] = [
    "chill_effect_+%",
    "cold_ailment_effect_+%",
    "non_damaging_ailment_effect_+%",
];

/// Quality stats that give a skill's own chill effect (PoB's
/// `EnemyChillEffect` INC; [`guaranteed_chill`]).
const SKILL_CHILL_QUALITY: &[&str] = &["chill_effect_+%", "cold_ailment_effect_+%"];

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
///
/// PoB scales the chill by the skill's own `EnemyChillEffect` too
/// (`calcLib.mod(activeSkill.skillModList, ..., "EnemyChillEffect")`): a
/// skill whose quality gives it (Frost Shield's `cold_ailment_effect_+%`,
/// P6-11fk) at a quality above 0 is a `skill-chill-effect` reason, the
/// chill kept at the character's effect.
fn guaranteed_chill(
    input: &CalcInput<'_>,
    s: &GroupSkill<'_>,
    source: Source,
    ingest: &mut Ingest<'_>,
) {
    let types = &s.types;
    // Bonechill (P6-11fv; `CalcPerform.lua` 1357 to 1358, 1379, 1397):
    // any skill it supports gives `HasBonechill` (PoB's `CannotChill`
    // check comes from modifiers the engine does not read), a chilling area
    // also the guarantee.
    // PoB gives it only to a chilling area, a non-hit chill, or a skill
    // that can chill (`CalcPerform.lua` 1357): a skill whose gems cannot
    // (Elemental Focus, P6-11ge) does not get it.
    if s.supports
        .iter()
        .any(|sup| sup.effect.id == BONECHILL_SUPPORT)
        && (types.contains(CHILLING_AREA)
            || types.contains(NON_HIT_CHILL)
            || !cannot_chill(input, s))
    {
        ingest.effect(Effect {
            quantity: Quantity::Poe1(Poe1Quantity::Bonechill(types.contains(CHILLING_AREA))),
            op: Op::Flag,
            value: 1.0,
            conditions: Conditions::none(),
            source: source.clone(),
        });
    }
    // Bonechill's own `chill_effect_+%` on every level (P6-11fv) is the
    // supported skill's `EnemyChillEffect` too.
    let own_chill = (s.active_gem.quality > 0
        && s.active_gem
            .effect
            .quality
            .iter()
            .any(|q| q.per_mille != 0 && SKILL_CHILL_QUALITY.contains(&q.stat.as_str())))
        || s.supports.iter().any(|sup| {
            sup.effect.id == BONECHILL_SUPPORT
                // A support's chill effect (open items S2: Unbound
                // Ailments' `non_damaging_ailment_effect_+%`).
                || skill::instance(sup.effect, sup.level, sup.quality, input.data)
                    .stats
                    .iter()
                    .any(|(id, v)| *v != 0 && CHILL_EFFECT_STATS.contains(&id.as_str()))
        });
    if types.contains(CHILLING_AREA) && own_chill {
        ingest.reason(UnsupportedReason::UnmodelledSkill {
            source: source.clone(),
            what: "skill-chill-effect",
        });
    }
    if types.contains(CHILLING_AREA) {
        ingest.effect(Effect {
            quantity: Quantity::Poe1(Poe1Quantity::SkillChill),
            op: Op::Base,
            value: super::damage_taken::CHILL_DEFAULT,
            conditions: Conditions::none(),
            source,
        });
    } else if types.contains(NON_HIT_CHILL) && !cannot_chill(input, s) {
        ingest.reason(UnsupportedReason::UnmodelledSkill {
            source,
            what: "guaranteed-chill",
        });
    }
}

/// The gem stats that make a skill unable to chill (PoB's `CannotChill`
/// from its stat map; P6-11ge): Elemental Focus's.
pub const CANNOT_CHILL_STATS: &[&str] = &["cannot_inflict_status_ailments"];

/// Whether a gem of `s` (its active gem or a support) has a stat of
/// [`CANNOT_CHILL_STATS`] (P6-11ge). Modifiers that give `CannotChill`
/// otherwise are unmapped stats, so reasons.
fn cannot_chill(input: &CalcInput<'_>, s: &GroupSkill<'_>) -> bool {
    std::iter::once(&s.active_gem).chain(&s.supports).any(|g| {
        skill::instance(g.effect, g.level, g.quality, input.data)
            .stats
            .iter()
            .any(|(id, _)| CANNOT_CHILL_STATS.contains(&id.as_str()))
    })
}

/// The gem stats that make a skill unable to ignite (PoB's `CannotIgnite`
/// flags, `SkillStatMap.lua` 1062, 1071, 1211; P6-11hr): Elemental Focus's,
/// and a skill's own `never_ignite` (Flame Surge's).
pub const CANNOT_IGNITE_STATS: &[&str] = &[
    "cannot_inflict_status_ailments",
    "never_any_ailment",
    "never_ignite",
];

/// Whether a gem of `s` has a stat of [`CANNOT_IGNITE_STATS`] (P6-11hr,
/// Combustion). Modifiers that give `CannotIgnite` otherwise are unmapped
/// stats, so reasons.
fn cannot_ignite(input: &CalcInput<'_>, s: &GroupSkill<'_>) -> bool {
    std::iter::once(&s.active_gem).chain(&s.supports).any(|g| {
        skill::instance(g.effect, g.level, g.quality, input.data)
            .stats
            .iter()
            .any(|(id, _)| CANNOT_IGNITE_STATS.contains(&id.as_str()))
    })
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
    // "<Skill> has no Reservation" (P6-11fq): by the skill's name as PoB's
    // `gemIdLookup` finds it, the aura ones only for an aura, never a
    // Blessing.
    let name = s.active.map(|a| a.name.as_str());
    let named_free = skill_modifiers::NO_RESERVATION_STATS
        .iter()
        .enumerate()
        .any(|(i, (_, skill, aura))| {
            totals.no_reservation & (1 << i) != 0
                && name == Some(*skill)
                && (!*aura || s.types.contains("Aura"))
                && !s.types.contains("Blessing")
        });
    if named_free {
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
    let mult = support_multiplier(&s.supports, &s.cost_mores);
    let life_instead = totals.reserve_life
        || s.supports
            .iter()
            .any(|sup| LIFE_RESERVATION_SUPPORTS.contains(&sup.effect.id.as_str()));
    // A support's own mana reservation share replaces the skill's (P6-11r:
    // Blasphemy's; PoB sets `skillData.manaReservationPercent` from each
    // support's level row, `CalcActiveSkill.lua` 516 and 517, the last
    // one kept).
    let from_support = s
        .supports
        .iter()
        .filter_map(|sup| skill::effect_level(sup.effect, sup.level))
        .filter_map(|l| {
            l.reservations
                .iter()
                .find(|r| r.percent && r.resource == Resource::Mana)
        })
        .next_back();
    let mut reservations: Vec<loom_data::bundle::skills::Reservation> = level
        .reservations
        .iter()
        .filter(|r| from_support.is_none() || !(r.percent && r.resource == Resource::Mana))
        .cloned()
        .collect();
    reservations.extend(from_support.cloned());
    // A mine support's cost as reservation (P6-11et): the level's mana and
    // life costs replace the flat reservation of each (PoB's
    // `ManaCostBase` and `LifeCostBase` sums, which no row the engine maps
    // gives, plus the level cost).
    if s.supports
        .iter()
        .any(|sup| COST_AS_RESERVATION_SUPPORTS.contains(&sup.effect.id.as_str()))
    {
        for (cost, resource) in [("Mana", Resource::Mana), ("Life", Resource::Life)] {
            let Some(c) = level.costs.iter().find(|c| c.resource == cost) else {
                continue;
            };
            reservations.retain(|r| r.percent || r.resource != resource);
            reservations.push(loom_data::bundle::skills::Reservation {
                resource,
                percent: false,
                amount: c.amount,
            });
        }
    }
    for r in reservations {
        let pool = match r.resource {
            Resource::Mana if life_instead => Pool::Life,
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
        let spellslinger = s
            .supports
            .iter()
            .any(|sup| sup.effect.id == SPELLSLINGER_SUPPORT);
        let terms =
            totals.reservation_for_skill(pool, &s.types, &s.active_gem.effect.id, spellslinger);
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

    /// A grant line's kind (P6-11ff), as PoB's patterns.
    #[test]
    fn grant_lines_should_say_whether_supports_join() {
        assert_eq!(grant_kind("Grants Level 20 Precision Skill"), Some(true));
        assert_eq!(grant_kind("Grants Summon Harbinger Skill"), Some(true));
        assert_eq!(
            grant_kind("Trigger Level 20 Raise Spiders on Kill"),
            Some(true)
        );
        assert_eq!(
            grant_kind("Curse Enemies with Assassin's Mark on Hit"),
            Some(false)
        );
        assert_eq!(
            grant_kind("Curse Enemies with Level 12 Vulnerability on Hit"),
            Some(false)
        );
        assert_eq!(
            grant_kind("Trigger Commandment of Frost on Kill"),
            Some(false)
        );
        assert_eq!(grant_kind("+20 to maximum Life"), None);
    }

    /// `AurasAffectEnemies` (P6-11er): an ally aura's resistance row goes
    /// on the enemy, its maximum resistance does nothing there, another row
    /// is a reason; nothing without the flag, or with the aura effect on you.
    #[test]
    fn ally_auras_should_go_on_the_enemy_with_the_flag() {
        let source = Source::Gem { group: 0, gem: 0 };
        let mut auras = BTreeMap::new();
        auras.insert("Purity of Ice", source.clone());
        let mut merged: Merged = BTreeMap::new();
        let res = (
            Quantity::Resistance(DamageType::Cold),
            Op::Base,
            1.0,
            NONE,
            NONE,
        );
        let max = (
            Quantity::MaximumResistance(DamageType::Cold),
            Op::Base,
            1.0,
            NONE,
            NONE,
        );
        let acc = (Quantity::Accuracy, Op::Base, 1.0, NONE, NONE);
        merged.insert(
            ("Purity of Ice", "res", 0),
            (20.0, source.clone(), res, None),
        );
        merged.insert(
            ("Purity of Ice", "max", 0),
            (2.0, source.clone(), max, None),
        );
        merged.insert(("Other", "res", 0), (30.0, source.clone(), res, None));
        let flag = |i: &mut Ingest<'_>| {
            i.effect(Effect {
                quantity: Quantity::Poe1(Poe1Quantity::AurasAffectEnemies),
                op: Op::Flag,
                value: 1.0,
                conditions: Conditions::none(),
                source: source.clone(),
            });
        };
        let totals = Totals::default();
        // Without the flag: nothing.
        let mut i = Ingest::new(&Poe1Mapping);
        assert!(auras_on_enemy(&totals, &auras, &merged, &mut i).is_empty());
        // With it: the resistance alone, as the enemy's.
        let mut i = Ingest::new(&Poe1Mapping);
        flag(&mut i);
        let out = auras_on_enemy(&totals, &auras, &merged, &mut i);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].0, 20.0);
        assert_eq!(out[0].2.0, enemy_res(DamageType::Cold));
        assert!(i.finish().1.partial.is_empty());
        // Another row is a reason.
        merged.insert(
            ("Purity of Ice", "acc", 0),
            (5.0, source.clone(), acc, None),
        );
        let mut i = Ingest::new(&Poe1Mapping);
        flag(&mut i);
        let _ = auras_on_enemy(&totals, &auras, &merged, &mut i);
        assert_eq!(i.finish().1.partial.len(), 1);
        // The aura effect on you: a reason, nothing on the enemy.
        let totals = Totals {
            aura_on_self: 10.0,
            ..Totals::default()
        };
        let mut i = Ingest::new(&Poe1Mapping);
        flag(&mut i);
        assert!(auras_on_enemy(&totals, &auras, &merged, &mut i).is_empty());
        assert_eq!(i.finish().1.partial.len(), 1);
    }

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
        assert_eq!(kind("LoomNoSuchSkill"), None);
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
                | BuffStat::CheckModCond { rows: e, .. }
                | BuffStat::BloodStance(e)
                | BuffStat::SandStance(e)
                | BuffStat::Unscalable(e)
                | BuffStat::PerCount { rows: e, .. }
                | BuffStat::PerStack { rows: e, .. }
                | BuffStat::AtMaxStages { rows: e, .. }
                | BuffStat::AtCount { rows: e, .. }
                | BuffStat::Threshold(e)
                | BuffStat::LifeThreshold(e)
                | BuffStat::WhenState { rows: e, .. }
                | BuffStat::MinionOnly(e)
                | BuffStat::PerPower { rows: e, .. }
                | BuffStat::EnemyCheck { rows: e, .. }
                | BuffStat::ConfigIs { rows: e, .. } => e,
                BuffStat::NoSlice(_)
                | BuffStat::Defence(_)
                | BuffStat::Gated(_)
                | BuffStat::Reason(_)
                | BuffStat::ReasonWhenInput { .. } => &[],
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
                        | Poe1Quantity::EnemyAilmentDuration
                        // P6-11cv: its debuffs' expiry.
                        | Poe1Quantity::EnemyBuffExpireFaster
                        | Poe1Quantity::EnemyProjectileDamageTaken
                        | Poe1Quantity::EnemyTrapMineDamageTaken
                        | Poe1Quantity::EnemyProjectileAttackDamageTaken
                        | Poe1Quantity::EnemyNeverCrit
                        | Poe1Quantity::EnemyPhysicalDamageReduction
                        // P6-10b parts 11 and 12: its accuracy and damage.
                        | Poe1Quantity::EnemyAccuracy
                        | Poe1Quantity::EnemyDamage
                )
            )
        };
        for (id, k) in SKILLS {
            for (stat, b) in k.stats() {
                let BuffStat::Rows(rows) = b else { continue };
                for row in *rows {
                    match k {
                        Kind::Debuff { .. } | Kind::Curse { .. } => {
                            // PoB sums the enemy's `SelfChaosMin` and kin
                            // and its `Self<Ailment>Chance` into the hit's
                            // added damage and ailment chance (P6-9b):
                            // the character's quantities, the same sum;
                            // and its `SelfCritChance` and
                            // `SelfCritMultiplier` into the hit's crit
                            // (P6-9e, Assassin's Mark).
                            let summed = matches!(
                                row.0,
                                Quantity::AddedDamage(..)
                                    | Quantity::CritChance
                                    | Quantity::CritMultiplier
                                    | Quantity::Poe1(Poe1Quantity::AilmentChance(_))
                            );
                            assert!(enemy(row.0) || summed, "{id} {stat}");
                        }
                        Kind::Buff { .. } | Kind::Warcry { .. } => {
                            assert!(!enemy(row.0), "{id} {stat}");
                        }
                        Kind::Inert => {}
                    }
                }
            }
            // Warcries: no row reaches a slice field (module docs); one that
            // would is a reason, or a field's reason marker (P6-10b: Enduring
            // Cry's regeneration, Intimidating Cry's movement speed).
            let marker = |rows: &[HitEffect]| {
                rows.iter().all(|r| {
                    matches!(
                        r.0,
                        Quantity::Poe1(
                            Poe1Quantity::RegenReason(..) | Poe1Quantity::MovementReason(_)
                        )
                    )
                })
            };
            if let Kind::Warcry { stats, .. } = k {
                assert!(
                    stats.iter().all(|(_, b)| match b {
                        BuffStat::NoSlice(_)
                        | BuffStat::Defence(_)
                        | BuffStat::Reason(_)
                        | BuffStat::ReasonWhenInput { .. } => true,
                        BuffStat::Rows(rows) => marker(rows),
                        // Scaled by power and uptime (P6-11bs).
                        BuffStat::PerPower { .. } => true,
                        _ => false,
                    }),
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
        // P6-2d: impale's effect is the impale's, not a buff's or an aura's.
        // P6-11cj: Empower's supported level, which `skill::supported_level`
        // reads (a `NoEffect` row so the support's own mapping is quiet).
        let checked = [
            "curse_effect_on_self_+%",
            "impale_debuff_effect_+%",
            // P6-11en: the two-handed weapons' impale effect, the same.
            "impale_inflicted_by_two_handed_weapons_debuff_effect_+%",
            "supported_active_skill_gem_level_+",
            // P6-11ck: Enhance's supported quality, the same way.
            "supported_active_skill_gem_quality_%",
            // P6-11hm: missing unreserved life gained before a hit, read by
            // the EHP estimate, not a reservation.
            "missing_unreserved_life_%_gained_as_life_before_hit",
            // P6-11hs: the totem's level (PoB's `totemLevel`, the totem's
            // life), not a gem's level.
            "totem_support_gem_level",
            // P6-11ib: maximum resistances that read the reservations
            // (`collect::reserved_max_resist`), not one.
            "additional_maximum_all_elemental_resistances_%_with_reserved_life_and_mana",
            // Close-out M2: area of effect per unreserved mana, area only.
            "hierophant_area_of_effect_+%_per_50_unreserved_mana_up_to_100%",
            // Open items S8: "Warcries cannot Exert Travel Skills", which
            // PoB leaves unparsed; no exerted attack of this module.
            "travel_skills_cannot_be_exerted",
            // Open items S8: Adrenaline when leech is removed by filling
            // unreserved life (`NoEffect`), not a reservation.
            "chance_to_gain_adrenaline_for_2_seconds_on_leech_removed_by_filling_unreserved_life_%",
        ];
        for id in ids {
            for word in GUARDED_WORDS {
                // P6-11cl: the typed supported levels, read the same way.
                let typed = super::super::skill::SUPPORTED_TYPED_LEVEL_STATS
                    .iter()
                    .any(|(stat, _)| *stat == id);
                assert!(
                    !id.contains(word) || checked.contains(&id) || typed,
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
        assert_eq!(support_multiplier(&[], &[]), 1.0);
        assert_eq!(support_multiplier(&[], &[80.0]), 0.8);
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
        assert!(is_input(PRIDE_INPUT));
        assert!(!is_input("buffOnslaught"));
    }

    fn curse(name: &'static str, priority: i64, mark: bool) -> CurseApplied {
        CurseApplied {
            name,
            source: Source::Character,
            rows: Vec::new(),
            priority,
            mark,
        }
    }

    /// PoB's mark rule (P6-9e): a second mark takes no slot while one is
    /// held, even a free one.
    #[test]
    fn aura_not_on_self_supports_should_be_modelled() {
        // P6-11fu: and the debuff supports, with global rows.
        for id in DEBUFF_SUPPORTS {
            assert!(support(id).is_some_and(|t| !t.is_empty()), "{id}");
        }
        for id in AURA_NOT_ON_SELF_SUPPORTS {
            assert!(support(id).is_some(), "{id}");
        }
        assert!(
            OWN_EFFECT_STATS
                .iter()
                .any(|(s, aura)| *s == "non_curse_aura_effect_+%" && *aura)
        );
    }

    /// The skill and support tables are binary-searched by ID (open items
    /// S3: an entry out of order hid Cast On Critical Strike).
    #[test]
    fn the_gem_tables_should_be_sorted_by_id() {
        for w in SKILLS.windows(2) {
            assert!(w[0].0 < w[1].0, "{} before {}", w[0].0, w[1].0);
        }
        for w in SUPPORTS.windows(2) {
            assert!(w[0].0 < w[1].0, "{} before {}", w[0].0, w[1].0);
        }
    }

    #[test]
    fn curse_supports_should_be_sorted_modelled_supports() {
        assert!(CURSE_SUPPORTS.windows(2).all(|w| w[0].0 < w[1].0));
        for (id, stats) in CURSE_SUPPORTS {
            assert!(SUPPORTS.iter().any(|(s, _)| s == id), "{id}");
            assert!(stats.windows(2).all(|w| w[0].0 < w[1].0), "{id}");
        }
    }

    #[test]
    fn curse_slots_should_hold_one_mark() {
        let curses = [
            curse("Despair", 8, false),
            curse("Assassin's Mark", 11, true),
            curse("Sniper's Mark", 12, true),
        ];
        assert_eq!(curse_slots(&curses, 3), vec![0, 1]);
    }

    /// A mark that loses its slot to a curse of higher priority frees the
    /// rule for a later mark, which still needs a slot of its own.
    #[test]
    fn curse_slots_should_free_the_mark_rule_when_a_mark_is_replaced() {
        let curses = [
            curse("Assassin's Mark", 11, true),
            curse("Elemental Weakness", 20004, false),
            curse("Sniper's Mark", 12, true),
        ];
        assert_eq!(curse_slots(&curses, 1), vec![1]);
        assert_eq!(curse_slots(&curses, 2), vec![0, 1]);
        // A later mark of a higher priority replaces the curse that took
        // the first mark's slot.
        let curses = [
            curse("Assassin's Mark", 11, true),
            curse("Elemental Weakness", 20004, false),
            curse("Sniper's Mark", 30012, true),
        ];
        assert_eq!(curse_slots(&curses, 1), vec![2]);
    }

    /// PoB's guard slots (P6-9e): Vaal Molten Shell alone; else the Vaal
    /// guards and one other; two others are PoB's table order's choice.
    #[test]
    fn guard_pick_should_follow_pobs_slots() {
        let pick = |n: &[&'static str]| guard_pick(n.iter().copied());
        assert_eq!(
            pick(&["Molten Shell", "Vaal Molten Shell", "Steelskin"]),
            (BTreeSet::from(["Vaal Molten Shell"]), Vec::new())
        );
        assert_eq!(
            pick(&["Molten Shell"]),
            (BTreeSet::from(["Molten Shell"]), Vec::new())
        );
        assert_eq!(
            pick(&["Molten Shell", "Steelskin"]),
            (BTreeSet::new(), vec!["Molten Shell", "Steelskin"])
        );
    }
}
