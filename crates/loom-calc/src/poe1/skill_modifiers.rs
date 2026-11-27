//! Modifiers of the other gems' reservation and aura effect (Phase 3 plan,
//! step 8; P3-8l): "#% increased Mana Reservation Efficiency of Skills" and
//! kin, "#% increased Reservation of Skills", and "#% increased effect of
//! Non-Curse Auras from your Skills". Until P3-8l each was a reason
//! ([`super::gems::DEPENDS_ON`]), so [`super::gems`] reserved without
//! modifiers and added every buff unscaled.
//!
//! What PoB v2.67.2 does (line numbers of the pinned checkout):
//! - **Reading.** `Modules/ModParser.lua` reads "mana (life) reservation
//!   efficiency of skills" as `ManaReservationEfficiency`
//!   (`LifeReservationEfficiency`), "reservation efficiency of skills" as
//!   `ReservationEfficiency`, and "reservation of skills" as `Reserved`,
//!   each INC with no tag (222 to 232). It reads "effect of non-curse auras
//!   from your skills" as `AuraEffect` INC tagged with the skill types
//!   `Aura` and not `AppliesCurse` (457).
//! - **Reservation** (`Modules/CalcPerform.lua` 1965 to 1989), per
//!   reserving skill and pool: `inc` is the INC sum of `<Pool>Reserved` and
//!   `Reserved`, `more` their MORE product, `efficiency` the INC sum of
//!   `<Pool>ReservationEfficiency` and `ReservationEfficiency` at least
//!   -100, `efficiencyMore` their MORE product. With `more > 0`,
//!   `inc > -100`, and a base above 0, the flat amount is
//!   `round(floor(flat x mult) x (100 + inc) / 100 x more / (1 + efficiency
//!   / 100) / efficiencyMore, 0)` and the percent the same from
//!   `percent x mult`, rounded to 2 decimals; each at least 0. Otherwise
//!   the skill reserves nothing of that kind.
//! - **Aura effect** (2392 to 2424): an `Aura` buff's mods are scaled by
//!   `(1 + inc / 100) x more`, where `inc` sums `AuraEffect`, `BuffEffect`,
//!   `BuffEffectOnSelf`, `AuraEffectOnSelf`, `AuraBuffEffect`, and
//!   `SkillAuraEffectOnSelf` INC of the aura skill's list (its tags
//!   matched against the skill's types). A `Buff` buff (2266 to 2281) reads
//!   no `AuraEffect`. Each skill's buff is scaled **before** buffs of one
//!   name merge (`mergeBuff` keeps each mod's largest scaled value).
//! - **Scaling a mod** (`Classes/ModStore.lua` `ScaleAddMod`, 45 to 80): at
//!   a scale of 1 the mod is added unchanged. Otherwise a mod whose name
//!   and type are in `data.highPrecisionMods` (`Modules/Data.lua` 430 on;
//!   of the engine's buff rows only `CritChance` BASE, precision 2) or
//!   whose value is not whole (precision 1) becomes
//!   `floor(value x scale x 10^p) / 10^p`; any other becomes
//!   `modf(round(value x scale, 2))`, its integer part toward zero.
//!
//! **The engine.** [`STATS`] maps the seven stats to
//! [`Poe1Quantity::ReservationEfficiency`], [`Poe1Quantity::Reserved`],
//! and [`Poe1Quantity::NonCurseAuraEffect`] (`Increased`, unconditional:
//! a test). [`Totals::read`] sums them once every source of them is in
//! (passives, items, config; [`super::collect`] calls [`super::gems`]
//! after those); [`super::gems`] gives each reserving skill
//! [`Totals::reservation`] and scales each aura's rows by
//! [`Totals::buff_scale`] with [`scale`] before the merge. Only these
//! stats are read: every other stat PoB reads as one of these mods (a
//! skill's own, "Mana Reservation of Skills", buff effect, the MORE ones)
//! stays a reason ([`super::gems::DEPENDS_ON`]), so with one of them the
//! fields stay `Partial` and the sums here are PoB's.
//!
//! **Left as reasons.** "#% increased Mana (Life) Reservation of Skills"
//! (`base_mana_reservation_+%`, `base_life_reservation_+%`): PoB tags them
//! with the `Aura` skill type ("mana reservation of skills", 221), so it
//! would not apply them to a herald or Arctic Armour, while the game's
//! text names every skill. Applying either reading without evidence of
//! the game's would be a guess. "#% increased Effect of Buffs on you"
//! (`buff_effect_on_self_+%`): PoB also reads it for Onslaught, Arcane
//! Surge, Fortification, and others (`CalcPerform.lua` 676 to 826), which
//! [`super::buffs`] and [`super::charges`] do not scale yet.
//!
//! Provenance: a scaled buff row or a reservation carries its gem's source;
//! the modifiers that scaled it are no graph node (a documented gap, as
//! the charges' count and the shock's minimum sources).

use std::collections::BTreeSet;

use super::{Poe1Quantity, Pool};
use crate::ingest::Ingest;
use crate::quantity::{Bound, DamageType, Op, Quantity};
use crate::store::Conditions;

/// A modifier of a skill's duration or cooldown, or of a warcry's buff
/// or power (P6-11bs), each PoB's INC: warcries read them for their
/// uptime and power ([`super::gems`]); nothing else does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SkillTiming {
    /// `Duration` INC, untagged ("Skill Effect Duration").
    Duration,
    /// `Duration` INC of warcries (`KeywordFlag.Warcry`).
    WarcryDuration,
    /// `CooldownRecovery` INC, untagged ("Cooldown Recovery Rate").
    CooldownRecovery,
    /// `CooldownRecovery` INC of warcries.
    WarcryCooldownRecovery,
    /// `CooldownRecovery` INC of Intimidating Cry (its `SkillName`; the
    /// agent list).
    IntimidatingCryCooldownRecovery,
    /// `BuffEffect` INC of warcries ("Warcry Buff Effect").
    WarcryBuffEffect,
    /// `WarcryPower` INC ("increased total Power counted by Warcries").
    WarcryPower,
    /// `WarcrySpeed` INC of warcries ("Warcry Speed"; P6-11ci).
    WarcrySpeed,
}

impl SkillTiming {
    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Duration => "duration",
            Self::WarcryDuration => "warcry-duration",
            Self::CooldownRecovery => "cooldown-recovery",
            Self::WarcryCooldownRecovery => "warcry-cooldown-recovery",
            Self::IntimidatingCryCooldownRecovery => "intimidating-cry-cooldown-recovery",
            Self::WarcryBuffEffect => "warcry-buff-effect",
            Self::WarcryPower => "warcry-power",
            Self::WarcrySpeed => "warcry-speed",
        }
    }
}

/// A golem skill a buff effect names (P6-11gc; PoB's `ModParser.lua` 479
/// to 484, "effect of the buff granted by your <kind> golems", a
/// `SkillName` tag with `includeTransfigured`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Golem {
    /// Summon Chaos Golem.
    Chaos,
    /// Summon Flame Golem.
    Flame,
    /// Summon Ice Golem.
    Ice,
    /// Summon Lightning Golem.
    Lightning,
    /// Summon Stone Golem.
    Stone,
    /// Summon Carrion Golem (the agent list).
    Carrion,
}

impl Golem {
    /// Every golem, in [`Golem`]'s order (its index in
    /// [`Totals::named_golem_buff_effect`]).
    pub const ALL: [Self; 6] = [
        Self::Chaos,
        Self::Flame,
        Self::Ice,
        Self::Lightning,
        Self::Stone,
        Self::Carrion,
    ];

    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Chaos => "chaos",
            Self::Flame => "flame",
            Self::Ice => "ice",
            Self::Lightning => "lightning",
            Self::Stone => "stone",
            Self::Carrion => "carrion",
        }
    }

    /// The base skill's active skill ID; a transfigured one adds `_alt_x`
    /// or `_alt_y` (the game's `ActiveSkills`).
    #[must_use]
    pub const fn active_skill(self) -> &'static str {
        match self {
            Self::Chaos => "summon_chaos_golem",
            Self::Flame => "summon_flame_golem",
            Self::Ice => "summon_ice_golem",
            Self::Lightning => "summon_lightning_golem",
            Self::Stone => "summon_stone_golem",
            Self::Carrion => "summon_carrion_golem",
        }
    }

    /// Whether `active` (an active skill ID) is this golem's skill or a
    /// transfigured one (PoB's `includeTransfigured`: the gem's game ID,
    /// which a transfigured gem shares).
    #[must_use]
    pub fn is(self, active: &str) -> bool {
        let base = active
            .strip_suffix("_alt_x")
            .or_else(|| active.strip_suffix("_alt_y"))
            .unwrap_or(active);
        base == self.active_skill()
    }
}

/// One stat's quantity (`Increased`) and scale.
type ModifierRow = (&'static str, Poe1Quantity, f64);

/// The stats (module docs), sorted by stat ID. Each description is the
/// game's (`stat_descriptions.txt`), read by PoB's pattern:
/// - "#% increased Life (Mana) Reservation Efficiency of Skills";
/// - "#% increased Reservation of Skills";
/// - "#% increased Reservation Efficiency of Skills";
/// - "#% reduced Mana Reservation Efficiency of Skills" and "#% reduced
///   Reservation Efficiency of Skills" for the `-2%_per_1` stats, whose
///   descriptions double the value (`double`, `negate_and_double`): PoB
///   reads the line's number, so the stat acts at -2 per point;
/// - "#% increased effect of Non-Curse Auras from your Skills";
/// - "#% increased Effect of your Curses" (P3-close; PoB's `CurseEffect`
///   INC, `ModParser.lua` "effect of your curses");
/// - "#% increased Effect of Buffs granted by your Golems" (P3-close;
///   PoB's `BuffEffect` INC tagged `SkillType.Golem`, `ModParser.lua`
///   "effect of buffs granted by your golems");
/// - "#% increased Mana Reservation Efficiency of Herald Skills" (P6-11ag;
///   PoB's `ManaReservationEfficiency` INC tagged `SkillType.Herald`,
///   `ModParser.lua` 1027 "of herald skills").
/// - P6-11bs: "#% increased Cooldown Recovery Rate", "#% increased Skill
///   Effect Duration", and the warcries' duration, cooldown recovery, buff
///   effect, and power (PoB's `CooldownRecovery`, `Duration`, `BuffEffect`
///   INC, the warcry ones with `KeywordFlag.Warcry`; `WarcryPower` INC).
pub const STATS: [ModifierRow; 39] = [
    // Close-out M6: "Anger has #% increased Aura Effect".
    (
        "anger_aura_effect_+%",
        Poe1Quantity::NamedSkillEffect(2),
        1.0,
    ),
    // P6-11k: "Auras from your Skills have #% increased Effect on you"
    // (`SkillAuraEffectOnSelf` INC, `ModParser.lua`; no skill type tag).
    (
        "aura_effect_on_self_from_your_skills_+%",
        Poe1Quantity::SkillAuraEffectOnSelf,
        1.0,
    ),
    // The agent list (a game-truth line PoB reads): "#% increased Effect of
    // the Buff granted by your Carrion Golems" (the game's stat says bone).
    (
        "base_bone_golem_granted_buff_effect_+%",
        Poe1Quantity::NamedGolemBuffEffect(Golem::Carrion),
        1.0,
    ),
    // P6-11gc: "#% increased Effect of the Buff granted by your Chaos
    // Golems" (`BuffEffect` INC, the golem skill's name).
    (
        "base_chaos_golem_granted_buff_effect_+%",
        Poe1Quantity::NamedGolemBuffEffect(Golem::Chaos),
        1.0,
    ),
    (
        "base_cooldown_speed_+%",
        Poe1Quantity::SkillTiming(SkillTiming::CooldownRecovery),
        1.0,
    ),
    // P6-11gc: the Flame and Ice Golems' (the game's stat says fire).
    (
        "base_fire_golem_granted_buff_effect_+%",
        Poe1Quantity::NamedGolemBuffEffect(Golem::Flame),
        1.0,
    ),
    (
        "base_ice_golem_granted_buff_effect_+%",
        Poe1Quantity::NamedGolemBuffEffect(Golem::Ice),
        1.0,
    ),
    (
        "base_life_reservation_efficiency_+%",
        Poe1Quantity::ReservationEfficiency(Some(Pool::Life)),
        1.0,
    ),
    // P6-11gc: the Lightning Golems'.
    (
        "base_lightning_golem_granted_buff_effect_+%",
        Poe1Quantity::NamedGolemBuffEffect(Golem::Lightning),
        1.0,
    ),
    (
        "base_mana_reservation_efficiency_+%",
        Poe1Quantity::ReservationEfficiency(Some(Pool::Mana)),
        1.0,
    ),
    ("base_reservation_+%", Poe1Quantity::Reserved(None), 1.0),
    (
        "base_reservation_efficiency_+%",
        Poe1Quantity::ReservationEfficiency(None),
        1.0,
    ),
    // P6-11gc: "#% increased Effect of the Buff granted by your Stone
    // Golems" (`BuffEffect` INC, the golem skill's name).
    (
        "base_stone_golem_granted_buff_effect_+%",
        Poe1Quantity::NamedGolemBuffEffect(Golem::Stone),
        1.0,
    ),
    ("curse_effect_+%", Poe1Quantity::CurseEffect, 1.0),
    // Close-out M6: "Malevolence has #% increased Aura Effect".
    (
        "delirium_aura_effect_+%",
        Poe1Quantity::NamedSkillEffect(3),
        1.0,
    ),
    ("golem_buff_effect_+%", Poe1Quantity::GolemBuffEffect, 1.0),
    // P6-11at: "#% increased Effect of Herald Buffs on you" (`BuffEffect`
    // INC, `SkillType.Herald`, `ModParser.lua`).
    (
        "herald_effect_on_self_+%",
        Poe1Quantity::HeraldBuffEffect,
        1.0,
    ),
    (
        "herald_skills_mana_reservation_efficiency_+%",
        Poe1Quantity::HeraldManaReservationEfficiency,
        1.0,
    ),
    // The agent list (a game-truth line PoB reads): "Intimidating Cry has
    // #% increased Cooldown Recovery Rate" (`CooldownRecovery` INC, the
    // skill's name with `includeTransfigured`, `ModParser.lua`).
    (
        "intimidating_cry_cooldown_speed_+%",
        Poe1Quantity::SkillTiming(SkillTiming::IntimidatingCryCooldownRecovery),
        1.0,
    ),
    // Close-out M6: Malevolence's mana reservation efficiency.
    (
        "malevolence_mana_reservation_efficiency_+%",
        Poe1Quantity::NamedSkillEffect(4),
        1.0,
    ),
    // P6-11ey: "#% increased Mana Reservation Efficiency of Skills that
    // throw Mines" (`ManaReservationEfficiency` INC, `KeywordFlag.Mine`).
    (
        "mana_reservation_efficiency_+%_for_skills_that_throw_mines",
        Poe1Quantity::MineManaReservationEfficiency,
        1.0,
    ),
    (
        "mana_reservation_efficiency_-2%_per_1",
        Poe1Quantity::ReservationEfficiency(Some(Pool::Mana)),
        -2.0,
    ),
    // P6-11df: "#% increased Effect of your Marks" (`CurseEffect` INC with
    // `SkillType.Mark`).
    (
        "mark_skills_curse_effect_+%",
        Poe1Quantity::MarkCurseEffect,
        1.0,
    ),
    (
        "non_curse_aura_effect_+%",
        Poe1Quantity::NonCurseAuraEffect,
        1.0,
    ),
    // P6-11gg: "Your Offerings have #% increased Effect" (PoB's tree text
    // "increased effect of Offerings": `BuffEffect` INC with the offerings'
    // `SkillName` tag, `ModParser.lua` 486).
    (
        "offering_spells_effect_+%",
        Poe1Quantity::OfferingBuffEffect,
        1.0,
    ),
    // P6-11gn: "Pride has #% increased Aura Effect" (`AuraEffect` INC with
    // the `SkillName` Pride).
    (
        "pride_aura_effect_+%",
        Poe1Quantity::NamedSkillEffect(0),
        1.0,
    ),
    // The agent list: "#% increased Punishment Curse Effect" (`CurseEffect`
    // INC with the `SkillName` Punishment).
    (
        "punishment_curse_effect_+%",
        Poe1Quantity::NamedSkillEffect(6),
        1.0,
    ),
    (
        "reservation_efficiency_-2%_per_1",
        Poe1Quantity::ReservationEfficiency(None),
        -2.0,
    ),
    // P6-11gg: "Your Offerings have #% reduced Effect on you" (Mistress of
    // Sacrifice; `BuffEffectOnPlayer` INC on the offerings, `ModParser.lua`
    // 2924).
    (
        "self_offering_effect_+%",
        Poe1Quantity::OfferingEffectOnYou,
        1.0,
    ),
    (
        "skill_effect_duration_+%",
        Poe1Quantity::SkillTiming(SkillTiming::Duration),
        1.0,
    ),
    // Open items S3: "#% increased Mana Reservation Efficiency of Skills
    // Supported by Spellslinger" (`ManaReservationEfficiency` INC with
    // `Condition:SupportedBySpellslinger`, `ModParser.lua` 1596), and its
    // older "-2% per 1" form.
    (
        "spellslinger_mana_reservation_efficiency_+%",
        Poe1Quantity::SpellslingerManaReservationEfficiency,
        1.0,
    ),
    (
        "spellslinger_mana_reservation_efficiency_-2%_per_1",
        Poe1Quantity::SpellslingerManaReservationEfficiency,
        -2.0,
    ),
    // P6-11gn: "#% increased Vulnerability Curse Effect" (`CurseEffect` INC
    // with the `SkillName` Vulnerability).
    (
        "vulnerability_curse_effect_+%",
        Poe1Quantity::NamedSkillEffect(1),
        1.0,
    ),
    (
        "warcry_buff_effect_+%",
        Poe1Quantity::SkillTiming(SkillTiming::WarcryBuffEffect),
        1.0,
    ),
    (
        "warcry_cooldown_speed_+%",
        Poe1Quantity::SkillTiming(SkillTiming::WarcryCooldownRecovery),
        1.0,
    ),
    (
        "warcry_duration_+%",
        Poe1Quantity::SkillTiming(SkillTiming::WarcryDuration),
        1.0,
    ),
    (
        "warcry_monster_power_+%",
        Poe1Quantity::SkillTiming(SkillTiming::WarcryPower),
        1.0,
    ),
    // P6-11ci: "#% increased Warcry Speed" (`WarcrySpeed` INC,
    // `KeywordFlag.Warcry`).
    (
        "warcry_speed_+%",
        Poe1Quantity::SkillTiming(SkillTiming::WarcrySpeed),
        1.0,
    ),
    // Close-out M6: Zealotry's mana reservation efficiency.
    (
        "zealotry_mana_reservation_efficiency_+%",
        Poe1Quantity::NamedSkillEffect(5),
        1.0,
    ),
];

/// What a named skill effect modifies (P6-11gn).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamedEffect {
    /// The skill's aura effect (`AuraEffect` INC): its aura scale alone.
    Aura,
    /// The skill's curse effect (`CurseEffect` INC): its curse scale.
    Curse,
    /// The skill's mana reservation efficiency (`ManaReservationEfficiency`
    /// INC; close-out M6): added to its mana efficiency.
    ManaReservationEfficiency,
}

/// The named skill effect modifiers (P6-11gn; PoB's `AuraEffect` or
/// `CurseEffect` INC with a `SkillName` tag, `ModParser.lua`'s "<skill> has
/// #% increased aura effect" and "#% increased <skill> curse effect"):
/// the stat, the skill's granted effect ID, and what it modifies, by
/// [`Poe1Quantity::NamedSkillEffect`]'s index.
pub const NAMED_SKILL_EFFECTS: [(&str, &str, NamedEffect); 7] = [
    ("pride_aura_effect_+%", "Pride", NamedEffect::Aura),
    (
        "vulnerability_curse_effect_+%",
        "Vulnerability",
        NamedEffect::Curse,
    ),
    // Close-out M6: "Anger has #% increased Aura Effect", "Malevolence has
    // #% increased Aura Effect" (PoB's `includeTransfigured` names: no
    // transfigured Anger or Malevolence exists).
    ("anger_aura_effect_+%", "Anger", NamedEffect::Aura),
    ("delirium_aura_effect_+%", "Malevolence", NamedEffect::Aura),
    // Close-out M6: "Malevolence has #% increased Mana Reservation
    // Efficiency", "Zealotry has ..." (`ManaReservationEfficiency` INC with
    // the `SkillName` tag).
    (
        "malevolence_mana_reservation_efficiency_+%",
        "Malevolence",
        NamedEffect::ManaReservationEfficiency,
    ),
    (
        "zealotry_mana_reservation_efficiency_+%",
        "Zealotry",
        NamedEffect::ManaReservationEfficiency,
    ),
    // The agent list: "#% increased Punishment Curse Effect", and the older
    // custom line "Punishment has #% reduced Curse Effect" (the custom
    // pattern `punishment-curse-effect`).
    (
        "punishment_curse_effect_+%",
        "Punishment",
        NamedEffect::Curse,
    ),
];

/// "Your Offering Skills also affect you" (P6-11gg; Mistress of
/// Sacrifice): PoB's `ExtraSkillMod` setting `buffNotPlayer` to false on
/// the four offerings (`ModParser.lua` 2923), so their buff reaches you as
/// a `Buff` does (`CalcPerform.lua` 2270 to 2280).
pub const OFFERINGS_AFFECT_YOU_STAT: &str = "offerings_also_buff_you";

/// The offering skills PoB's `SkillName` tags name ("Bone Offering", "Flesh
/// Offering", "Spirit Offering", "Blood Offering", `ModParser.lua` 486 and
/// 2923 to 2925), by granted effect ID.
pub const OFFERING_SKILLS: [&str; 4] = [
    "BloodOffering",
    "BoneOffering",
    "FleshOffering",
    "SpiritOffering",
];

/// The effects of a stat of [`STATS`] (or [`OFFERINGS_AFFECT_YOU_STAT`],
/// a `Flag`): `(quantity, op, scale)`.
#[must_use]
pub fn effects(stat: &str) -> Option<Vec<(Quantity, Op, f64)>> {
    if stat == OFFERINGS_AFFECT_YOU_STAT {
        return Some(vec![(
            Quantity::Poe1(Poe1Quantity::OfferingsAffectYou),
            Op::Flag,
            1.0,
        )]);
    }
    if let Ok(i) = NO_RESERVATION_STATS.binary_search_by(|(id, ..)| (*id).cmp(stat)) {
        let i = u8::try_from(i).ok()?;
        return Some(vec![(
            Quantity::Poe1(Poe1Quantity::SkillNoReservation(i)),
            Op::Flag,
            1.0,
        )]);
    }
    let i = STATS.binary_search_by(|(id, ..)| (*id).cmp(stat)).ok()?;
    let (_, q, scale) = STATS[i];
    Some(vec![(Quantity::Poe1(q), Op::Increased, scale)])
}

/// "<Skill> has no Reservation" and "<Curse> has no Reservation if Cast
/// as an Aura" (P6-11fq; `ModParser.lua` 5317 to 5328): PoB sets the
/// named skill's reservation keys to 0 (a `SkillId` tag from its name,
/// not a Blessing), the second only while the skill is an `Aura`. Each
/// stat with the skill's name as the game shows it and whether it needs
/// the aura type, sorted by stat (a test; another test checks each
/// against the bundle's text).
pub const NO_RESERVATION_STATS: [(&str, &str, bool); 33] = [
    ("anger_reserves_no_mana", "Anger", false),
    ("arctic_armour_no_reservation", "Arctic Armour", false),
    ("bird_aspect_reserves_no_mana", "Aspect of the Avian", false),
    ("cat_aspect_reserves_no_mana", "Aspect of the Cat", false),
    ("chronomancer_reserves_no_mana", "Temporal Rift", false),
    ("clarity_reserves_no_mana", "Clarity", false),
    ("conductivity_no_reservation", "Conductivity", true),
    ("delirium_reserves_no_mana", "Malevolence", false),
    ("despair_no_reservation", "Despair", true),
    ("determination_reserves_no_mana", "Determination", false),
    ("discipline_reserves_no_mana", "Discipline", false),
    (
        "elemental_weakness_no_reservation",
        "Elemental Weakness",
        true,
    ),
    ("enfeeble_no_reservation", "Enfeeble", true),
    ("envy_reserves_no_mana", "Envy", false),
    ("flammability_no_reservation", "Flammability", true),
    ("flesh_stone_no_reservation", "Flesh and Stone", false),
    ("frostbite_no_reservation", "Frostbite", true),
    ("grace_reserves_no_mana", "Grace", false),
    ("haste_reserves_no_mana", "Haste", false),
    ("hatred_reserves_no_mana", "Hatred", false),
    ("precision_reserves_no_mana", "Precision", false),
    ("pride_reserves_no_mana", "Pride", false),
    ("punishment_no_reservation", "Punishment", true),
    (
        "purity_of_elements_reserves_no_mana",
        "Purity of Elements",
        false,
    ),
    ("purity_of_fire_reserves_no_mana", "Purity of Fire", false),
    ("purity_of_ice_reserves_no_mana", "Purity of Ice", false),
    (
        "purity_of_lightning_reserves_no_mana",
        "Purity of Lightning",
        false,
    ),
    (
        "spider_aspect_reserves_no_mana",
        "Aspect of the Spider",
        false,
    ),
    ("temporal_chains_no_reservation", "Temporal Chains", true),
    ("vitality_reserves_no_mana", "Vitality", false),
    ("vulnerability_no_reservation", "Vulnerability", true),
    ("wrath_reserves_no_mana", "Wrath", false),
    ("zealotry_reserves_no_mana", "Zealotry", false),
];

/// The sums of [`STATS`]' quantities.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Totals {
    /// `ReservationEfficiency` INC (no pool).
    pub efficiency: f64,
    /// `LifeReservationEfficiency` INC.
    pub life_efficiency: f64,
    /// `ManaReservationEfficiency` INC.
    pub mana_efficiency: f64,
    /// `ManaReservationEfficiency` INC of herald skills (P6-11ag).
    pub herald_mana_efficiency: f64,
    /// `ManaReservationEfficiency` INC of skills that throw mines (P6-11ey).
    pub mine_mana_efficiency: f64,
    /// `ManaReservationEfficiency` INC of skills Spellslinger supports
    /// (open items S3).
    pub spellslinger_mana_efficiency: f64,
    /// Skills reserve life instead of mana (`BloodMagicReserved`, an
    /// unconditional flag; P6-11an).
    pub reserve_life: bool,
    /// The offerings' buff reaches you too (P6-11gg,
    /// [`OFFERINGS_AFFECT_YOU_STAT`], an unconditional flag).
    pub offerings_affect_you: bool,
    /// Your warcries grant you no buff (P6-11hw; PoB's
    /// `CannotGainWarcryBuffs`, an unconditional flag).
    pub cannot_gain_warcry_buffs: bool,
    /// `Duration` MORE of warcries, each as its percent (P6-11hw: Call to
    /// Arms' 100% more); [`Self::warcry_duration_more`] multiplies them.
    pub warcry_duration_more_pct: [f64; 2],
    /// The skills with no reservation, a bit per
    /// [`NO_RESERVATION_STATS`] row (P6-11fq).
    pub no_reservation: u64,
    /// `Reserved` INC (no pool).
    pub reserved: f64,
    /// `AuraEffect` INC of non-curse aura skills.
    pub aura_effect: f64,
    /// `SkillAuraEffectOnSelf` INC: every aura buff on you (P6-11k).
    pub aura_on_self: f64,
    /// `BuffEffect` INC of golem skills (P3-close).
    pub golem_buff_effect: f64,
    /// `BuffEffect` INC of each named golem skill, by [`Golem::ALL`]
    /// (P6-11gc).
    pub named_golem_buff_effect: [f64; 6],
    /// `BuffEffect` INC of herald skills (P6-11at).
    pub herald_buff_effect: f64,
    /// `BuffEffect` INC of the offering skills (P6-11gg).
    pub offering_buff_effect: f64,
    /// Each named skill effect, by [`NAMED_SKILL_EFFECTS`]' index
    /// (P6-11gn).
    pub named_skill_effect: [f64; 7],
    /// `BuffEffectOnPlayer` INC of the offering skills (P6-11gg).
    pub offering_effect_on_you: f64,
    /// `CurseEffect` INC (P3-close).
    pub curse_effect: f64,
    /// `CurseEffect` INC of mark skills (P6-11df).
    pub mark_curse_effect: f64,
    /// `Duration` INC, untagged (P6-11bs).
    pub duration: f64,
    /// `Duration` INC of warcries.
    pub warcry_duration: f64,
    /// `CooldownRecovery` INC, untagged.
    pub cooldown_recovery: f64,
    /// `CooldownRecovery` INC of warcries.
    pub warcry_cooldown_recovery: f64,
    /// `CooldownRecovery` INC of Intimidating Cry (the agent list).
    pub intimidating_cry_cooldown_recovery: f64,
    /// `BuffEffect` INC of warcries.
    pub warcry_buff_effect: f64,
    /// `WarcryPower` INC.
    pub warcry_power: f64,
    /// `WarcrySpeed` INC of warcries (P6-11ci).
    pub warcry_speed: f64,
    /// An effect of these quantities that [`Self::read`] cannot sum (a
    /// condition, another op, a pool's `Reserved`): no row makes one (a
    /// test), so a later row that does is a reason, not a silent drop.
    pub unread: bool,
}

/// A skill's reservation terms for one pool (PoB's `values`, module docs).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReservationTerms {
    /// `values.inc`.
    pub inc: f64,
    /// `values.efficiency`, at least -100.
    pub efficiency: f64,
}

impl ReservationTerms {
    /// No modifier.
    pub const NONE: Self = Self {
        inc: 0.0,
        efficiency: 0.0,
    };
}

impl Totals {
    /// The sums of the unconditional `Increased` effects ingested so far.
    /// Every row is unconditional (a test); any other effect of these
    /// quantities sets [`Self::unread`].
    #[must_use]
    pub fn read(ingest: &Ingest<'_>) -> Self {
        Self::from_effects(ingest.effects())
    }

    /// [`Self::read`] from a finished store's effects (the minion step's
    /// view of the character, P6-7b4).
    #[must_use]
    pub fn from_effects(effects: &[crate::store::Effect]) -> Self {
        let mut t = Self::default();
        for e in effects {
            let Quantity::Poe1(q) = e.quantity else {
                continue;
            };
            if let Poe1Quantity::SkillNoReservation(i) = q {
                if e.op == Op::Flag && e.conditions == Conditions::none() {
                    t.no_reservation |= 1 << i;
                } else {
                    t.unread = true;
                }
                continue;
            }
            if q == Poe1Quantity::OfferingsAffectYou {
                if e.op == Op::Flag && e.conditions == Conditions::none() {
                    t.offerings_affect_you = true;
                } else {
                    t.unread = true;
                }
                continue;
            }
            if q == Poe1Quantity::CannotGainWarcryBuffs {
                if e.op == Op::Flag && e.conditions == Conditions::none() {
                    t.cannot_gain_warcry_buffs = true;
                } else {
                    t.unread = true;
                }
                continue;
            }
            // P6-11hw: warcries' more duration (Call to Arms), two at most.
            if q == Poe1Quantity::SkillTiming(SkillTiming::WarcryDuration) && e.op == Op::More {
                match t.warcry_duration_more_pct.iter_mut().find(|v| **v == 0.0) {
                    Some(slot) if e.conditions == Conditions::none() => *slot = e.value,
                    _ => t.unread = true,
                }
                continue;
            }
            if q == Poe1Quantity::ReserveLifeInsteadOfMana {
                if e.op == Op::Flag && e.conditions == Conditions::none() {
                    t.reserve_life = true;
                } else {
                    t.unread = true;
                }
                continue;
            }
            let sum = match q {
                Poe1Quantity::ReservationEfficiency(None) => &mut t.efficiency,
                Poe1Quantity::ReservationEfficiency(Some(Pool::Life)) => &mut t.life_efficiency,
                Poe1Quantity::ReservationEfficiency(Some(Pool::Mana)) => &mut t.mana_efficiency,
                Poe1Quantity::HeraldManaReservationEfficiency => &mut t.herald_mana_efficiency,
                Poe1Quantity::MineManaReservationEfficiency => &mut t.mine_mana_efficiency,
                Poe1Quantity::SpellslingerManaReservationEfficiency => {
                    &mut t.spellslinger_mana_efficiency
                }
                Poe1Quantity::Reserved(None) => &mut t.reserved,
                Poe1Quantity::NonCurseAuraEffect => &mut t.aura_effect,
                Poe1Quantity::SkillAuraEffectOnSelf => &mut t.aura_on_self,
                Poe1Quantity::GolemBuffEffect => &mut t.golem_buff_effect,
                Poe1Quantity::NamedGolemBuffEffect(g) => &mut t.named_golem_buff_effect[g as usize],
                Poe1Quantity::HeraldBuffEffect => &mut t.herald_buff_effect,
                Poe1Quantity::OfferingBuffEffect => &mut t.offering_buff_effect,
                Poe1Quantity::NamedSkillEffect(i) => &mut t.named_skill_effect[usize::from(i)],
                Poe1Quantity::OfferingEffectOnYou => &mut t.offering_effect_on_you,
                Poe1Quantity::CurseEffect => &mut t.curse_effect,
                Poe1Quantity::MarkCurseEffect => &mut t.mark_curse_effect,
                Poe1Quantity::SkillTiming(timing) => match timing {
                    SkillTiming::Duration => &mut t.duration,
                    SkillTiming::WarcryDuration => &mut t.warcry_duration,
                    SkillTiming::CooldownRecovery => &mut t.cooldown_recovery,
                    SkillTiming::WarcryCooldownRecovery => &mut t.warcry_cooldown_recovery,
                    SkillTiming::IntimidatingCryCooldownRecovery => {
                        &mut t.intimidating_cry_cooldown_recovery
                    }
                    SkillTiming::WarcryBuffEffect => &mut t.warcry_buff_effect,
                    SkillTiming::WarcryPower => &mut t.warcry_power,
                    SkillTiming::WarcrySpeed => &mut t.warcry_speed,
                },
                Poe1Quantity::Reserved(Some(_)) => {
                    t.unread = true;
                    continue;
                }
                _ => continue,
            };
            if e.op != Op::Increased || e.conditions != Conditions::none() {
                t.unread = true;
                continue;
            }
            *sum += e.value;
        }
        t
    }

    /// Warcries' `Duration` MORE as a factor (P6-11hw).
    #[must_use]
    pub fn warcry_duration_more(&self) -> f64 {
        self.warcry_duration_more_pct
            .iter()
            .fold(1.0, |m, v| m * (1.0 + v / 100.0))
    }

    /// The reservation terms of `pool` (module docs): the pool's and the
    /// generic efficiency summed, at least -100.
    #[must_use]
    pub fn reservation(&self, pool: Pool) -> ReservationTerms {
        let own = match pool {
            Pool::Life => self.life_efficiency,
            Pool::Mana => self.mana_efficiency,
            // No skill reserves energy shield.
            Pool::EnergyShield => 0.0,
        };
        ReservationTerms {
            inc: self.reserved,
            efficiency: crate::math::max(own + self.efficiency, -100.0),
        }
    }

    /// [`Self::reservation`] for a skill of `types` and granted effect `id`:
    /// a herald's mana efficiency adds its own (P6-11ag), a mine skill's its
    /// own (P6-11ey, the `RemoteMined` type: PoB's mine keyword), and a
    /// named skill's its own (close-out M6).
    #[must_use]
    pub fn reservation_for(
        &self,
        pool: Pool,
        types: &BTreeSet<String>,
        id: &str,
    ) -> ReservationTerms {
        self.reservation_for_skill(pool, types, id, false)
    }

    /// [`Self::reservation_for`], with whether Spellslinger supports the
    /// skill (open items S3: its mana efficiency adds its own, PoB's
    /// `Condition:SupportedBySpellslinger` that the support sets).
    #[must_use]
    pub fn reservation_for_skill(
        &self,
        pool: Pool,
        types: &BTreeSet<String>,
        id: &str,
        spellslinger: bool,
    ) -> ReservationTerms {
        let mut terms = self.reservation(pool);
        if pool != Pool::Mana {
            return terms;
        }
        let mut own = self.mana_efficiency;
        if spellslinger {
            own += self.spellslinger_mana_efficiency;
        }
        // Close-out M6: the skill's named mana reservation efficiency.
        own += self.named_skill(id, NamedEffect::ManaReservationEfficiency);
        if types.contains("Herald") {
            own += self.herald_mana_efficiency;
        }
        if types.contains("RemoteMined") {
            own += self.mine_mana_efficiency;
        }
        if own != self.mana_efficiency {
            terms.efficiency = crate::math::max(own + self.efficiency, -100.0);
        }
        terms
    }

    /// The `BuffEffect` INC the named golem modifiers give the skill whose
    /// active skill is `active` (P6-11gc): its golem's sum, else 0. Added
    /// to a buff's own effect ([`Self::buff_scale`]'s `own`).
    #[must_use]
    pub fn named_golem(&self, active: Option<&str>) -> f64 {
        let Some(active) = active else { return 0.0 };
        Golem::ALL
            .iter()
            .find(|g| g.is(active))
            .map_or(0.0, |&g| self.named_golem_buff_effect[g as usize])
    }

    /// The `CooldownRecovery` INC a skill-named modifier gives the skill
    /// whose active skill is `active` (the agent list): Intimidating
    /// Cry's, a transfigured one too (`includeTransfigured`), else 0.
    #[must_use]
    pub fn named_cooldown_recovery(&self, active: Option<&str>) -> f64 {
        let Some(active) = active else { return 0.0 };
        let base = active
            .strip_suffix("_alt_x")
            .or_else(|| active.strip_suffix("_alt_y"))
            .unwrap_or(active);
        if base == "intimidating_cry" {
            self.intimidating_cry_cooldown_recovery
        } else {
            0.0
        }
    }

    /// The `BuffEffect` INC the offering modifiers give the skill of the
    /// granted effect `id` (P6-11gg): [`Self::offering_buff_effect`] for
    /// an offering ([`OFFERING_SKILLS`]), else 0; with `on_you`, the effect
    /// on you ([`Self::offering_effect_on_you`], PoB's `BuffEffectOnPlayer`,
    /// which the minion's scale leaves out) too. Added to a buff's own
    /// effect.
    #[must_use]
    pub fn offering(&self, id: &str, on_you: bool) -> f64 {
        if !OFFERING_SKILLS.contains(&id) {
            return 0.0;
        }
        if on_you {
            self.offering_buff_effect + self.offering_effect_on_you
        } else {
            self.offering_buff_effect
        }
    }

    /// The named skill effect of `kind` for the skill of the granted
    /// effect `id` (P6-11gn, [`NAMED_SKILL_EFFECTS`]): its sum, else 0.
    #[must_use]
    pub fn named_skill(&self, id: &str, kind: NamedEffect) -> f64 {
        NAMED_SKILL_EFFECTS
            .iter()
            .enumerate()
            .filter(|(_, (_, skill, k))| *skill == id && *k == kind)
            .map(|(i, _)| self.named_skill_effect[i])
            .sum()
    }

    /// Whether the buff of the skill of the granted effect `id` reaches you
    /// although PoB's `buffNotPlayer` keeps it from you (P6-11gg): an
    /// offering while [`Self::offerings_affect_you`].
    #[must_use]
    pub fn offering_on_you(&self, id: &str) -> bool {
        self.offerings_affect_you && OFFERING_SKILLS.contains(&id)
    }

    /// The scale of a buff of `effect` from a skill of `types` (module
    /// docs): `1 + inc / 100`, `inc` the skill's own `BuffEffect`
    /// (`own`: its gem's stats, P3-close), plus the golems' buff effect
    /// for a skill typed `Golem` (P3-close), plus the non-curse aura
    /// effect for an aura of a skill typed `Aura` and not `AppliesCurse`.
    #[must_use]
    pub fn buff_scale(&self, aura: bool, types: &BTreeSet<String>, own: f64) -> f64 {
        let mut inc = own;
        if types.contains("Golem") {
            inc += self.golem_buff_effect;
        }
        // A herald's (P6-11at).
        if types.contains("Herald") {
            inc += self.herald_buff_effect;
        }
        if aura && types.contains("Aura") && !types.contains("AppliesCurse") {
            inc += self.aura_effect;
        }
        // Every aura buff on you, untagged (`CalcPerform.lua` 2413; P6-11k).
        if aura {
            inc += self.aura_on_self;
        }
        1.0 + inc / 100.0
    }

    /// A curse's scale (P3-close; `CalcPerform.lua` 2619 to 2632): `(1 +
    /// inc / 100) x more`, `inc` the `CurseEffect` INC (and the enemy's
    /// `CurseEffectOnSelf`, which nothing the engine reads gives), `more` 1
    /// (every MORE stat is a reason).
    #[must_use]
    pub fn curse_scale(&self) -> f64 {
        self.curse_scale_with(0.0)
    }

    /// [`Self::curse_scale`] with the curse gem's own `CurseEffect` INC
    /// (`own`, P6-9b).
    #[must_use]
    pub fn curse_scale_with(&self, own: f64) -> f64 {
        1.0 + (self.curse_effect + own) / 100.0
    }
}

/// PoB's precision for scaling a mod of this quantity and op
/// (`data.highPrecisionMods`), among the quantities buff rows reach.
fn high_precision(quantity: Quantity, op: Op) -> Option<u8> {
    match (quantity, op) {
        (Quantity::CritChance, Op::Base) => Some(2),
        // P6-11bs: `<Pool>RegenPercent` BASE at 2, `<Pool>Regen` BASE at 1
        // (`Modules/Data.lua` 437 to 454).
        (Quantity::Poe1(Poe1Quantity::RegenPercent(_)), Op::Base) => Some(2),
        (Quantity::Poe1(Poe1Quantity::Regen(_)), Op::Base) => Some(1),
        _ => None,
    }
}

/// A mod's value scaled as PoB's `ScaleAddMod` (module docs).
#[must_use]
pub fn scale(quantity: Quantity, op: Op, value: f64, scale: f64) -> f64 {
    // A flag is not scaled (`ScaleAddMod` scales numbers only; a flag's
    // value is `true`).
    if scale == 1.0 || op == Op::Flag {
        return value;
    }
    let precision = high_precision(quantity, op).or_else(|| {
        // `data.defaultHighPrecision` for a value that is not whole.
        (crate::math::floor(value) != value).then_some(1)
    });
    if let Some(p) = precision {
        // Exact powers, as literals (no platform `powi`).
        let power = if p == 1 { 10.0 } else { 100.0 };
        return crate::math::floor(value * scale * power) / power;
    }
    // `m_modf(round(value x scale, 2))`: the integer part, toward zero.
    let rounded = crate::math::floor(value * scale * 100.0 + 0.5) / 100.0;
    crate::math::trunc(rounded)
}

/// The main skill's own aura stats, by granted effect ID: PoB's stat map
/// gives them a `GlobalEffect` aura tag, so they are a buff on the
/// character, scaled as any aura (module docs), though the engine reads
/// them through the hit ([`super::mapping::SkillMapping`]). Smite's
/// (`Data/Skills/act_str.lua`, `skills["Smite"]`: `LightningMin` and
/// `LightningMax` BASE, `effectType = "Aura"`), each with the quantity it
/// maps to (for [`scale`]'s precision).
pub const MAIN_SKILL_AURA_STATS: &[(&str, &str, Quantity)] = &[
    (
        "Smite",
        "maximum_added_lightning_damage_from_skill",
        Quantity::AddedDamage(DamageType::Lightning, Bound::Max),
    ),
    (
        "Smite",
        "minimum_added_lightning_damage_from_skill",
        Quantity::AddedDamage(DamageType::Lightning, Bound::Min),
    ),
];

/// A main skill instance's stat value as its buff applies it: scaled by
/// `buff_scale` when it is one of [`MAIN_SKILL_AURA_STATS`], else as is.
/// The scaled value is whole ([`scale`] takes the integer part of a whole
/// value's product), so it stays a stat value.
#[must_use]
pub fn main_skill_stat(effect: &str, stat: &str, value: i64, buff_scale: f64) -> i64 {
    let Some(&(_, _, quantity)) = MAIN_SKILL_AURA_STATS
        .iter()
        .find(|(e, s, _)| *e == effect && *s == stat)
    else {
        return value;
    };
    #[expect(
        clippy::cast_precision_loss,
        reason = "stat values are small integers, exact as f64"
    )]
    let scaled = scale(quantity, Op::Base, value as f64, buff_scale);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a whole value's scale is whole and small"
    )]
    let scaled = scaled as i64;
    scaled
}

/// One skill's reservation of one pool, as PoB computes it (module docs):
/// the flat amount and the percent. `None` when PoB would divide by 0: an
/// efficiency of -100 on a base it reserves (with `inc` above -100).
#[must_use]
pub fn reserved(
    flat: i64,
    percent_hundredths: i64,
    mult: f64,
    terms: ReservationTerms,
) -> Option<(f64, f64)> {
    #[expect(
        clippy::cast_precision_loss,
        reason = "reservations are small integers, exact as f64"
    )]
    let (flat, percent) = (flat as f64, percent_hundredths as f64 / 100.0);
    let ReservationTerms { inc, efficiency } = terms;
    // `values.more` and `efficiencyMore` are 1: every MORE stat is a
    // reason.
    let (more, efficiency_more) = (1.0, 1.0);
    let applies = more > 0.0 && inc > -100.0;
    let flat_value = crate::math::floor(flat * mult);
    let percent_value = percent * mult;
    if applies && efficiency <= -100.0 && (flat_value != 0.0 || percent_value != 0.0) {
        return None;
    }
    let reserved_flat = if !applies || flat_value == 0.0 {
        0.0
    } else {
        let x = flat_value * (100.0 + inc) / 100.0 * more
            / (1.0 + efficiency / 100.0)
            / efficiency_more;
        crate::math::max(crate::math::round_half_up(x), 0.0)
    };
    let reserved_percent = if !applies || percent_value == 0.0 {
        0.0
    } else {
        let x = percent_value * (100.0 + inc) / 100.0 * more
            / (1.0 + efficiency / 100.0)
            / efficiency_more;
        // PoB's `round(x, 2)`: `floor(x x 10^2 + 0.5) / 10^2`.
        crate::math::max(crate::math::floor(x * 100.0 + 0.5) / 100.0, 0.0)
    };
    Some((reserved_flat, reserved_percent))
}

#[cfg(test)]
mod tests {
    /// P6-11fq: sorted, one flag per row, at most 64 rows (the bit set).
    #[test]
    fn no_reservation_stats_should_be_sorted_flags() {
        assert!(NO_RESERVATION_STATS.windows(2).all(|w| w[0].0 < w[1].0));
        assert!(NO_RESERVATION_STATS.len() <= 64);
        assert_eq!(
            effects("despair_no_reservation"),
            Some(vec![(
                Quantity::Poe1(Poe1Quantity::SkillNoReservation(8)),
                Op::Flag,
                1.0
            )])
        );
    }

    /// A flag is not scaled (P6-4b review; `ScaleAddMod` scales numbers).
    #[test]
    fn a_flag_should_not_be_scaled() {
        assert_eq!(scale(Quantity::Damage, Op::Flag, 1.0, 0.5), 1.0);
        assert_eq!(scale(Quantity::Damage, Op::Increased, 40.0, 0.5), 20.0);
    }

    use super::*;
    use crate::ingest::{StatMap, StatMapping};
    use crate::poe1::ArmourSlot;
    use crate::poe1::mapping::{PieceMapping, Poe1Mapping, SkillMapping, WeaponMapping};
    use loom_domain::StatId;

    #[test]
    fn stats_should_be_sorted_and_not_guarded() {
        assert!(STATS.windows(2).all(|w| w[0].0 < w[1].0));
        for (stat, ..) in STATS {
            assert!(
                !super::super::gems::DEPENDS_ON.contains(&stat),
                "{stat}: mapped here, so not a reason"
            );
        }
    }

    #[test]
    fn the_character_mapping_should_read_every_row_unconditionally() {
        for (stat, q, scale) in STATS {
            let StatMap::Effects(effects) = Poe1Mapping.map(&StatId::new(stat)) else {
                panic!("{stat} is not mapped");
            };
            assert_eq!(effects.len(), 1, "{stat}");
            let t = &effects[0];
            assert_eq!(t.quantity, Quantity::Poe1(q), "{stat}");
            assert_eq!(t.op, Op::Increased, "{stat}");
            assert!((t.scale - scale).abs() < f64::EPSILON, "{stat}");
            assert!((t.divisor - 1.0).abs() < f64::EPSILON, "{stat}");
            assert_eq!(t.conditions, Conditions::none(), "{stat}");
        }
    }

    /// A gem's own modifiers act on its skill alone (PoB's `skillModList`):
    /// through the main skill's mapping they stay reasons, and other
    /// groups' gems never reach a mapping.
    #[test]
    fn a_gems_modifiers_should_be_reasons() {
        for (stat, q, _) in STATS {
            // A skill's own duration reaches no slice field through its
            // hit (P6-11bs; it was a `NoEffect` row before). Its own
            // cooldown recovery is its cooldown's and its trigger's (open
            // items S3: `TypedCooldownRecovery` without a type).
            let want = if q == Poe1Quantity::SkillTiming(SkillTiming::Duration) {
                StatMap::NoEffect
            } else if q == Poe1Quantity::SkillTiming(SkillTiming::CooldownRecovery) {
                StatMap::Effects(vec![crate::ingest::EffectTemplate {
                    quantity: crate::quantity::Quantity::Poe1(Poe1Quantity::TypedCooldownRecovery),
                    op: crate::quantity::Op::Increased,
                    scale: 1.0,
                    divisor: 1.0,
                    conditions: Conditions::none(),
                }])
            } else {
                StatMap::Unmapped
            };
            assert_eq!(SkillMapping.map(&StatId::new(stat)), want, "{stat}");
        }
    }

    /// Each stat left (module docs) stays a reason through every mapping.
    #[test]
    fn the_stats_left_should_stay_unmapped() {
        let piece = PieceMapping {
            slot: ArmourSlot::Helmet,
        };
        let mappings: [&dyn StatMapping; 4] = [&Poe1Mapping, &SkillMapping, &piece, &WeaponMapping];
        for stat in [
            "aura_effect_+%",
            "base_life_reservation_+%",
            "base_mana_reservation_+%",
            "buff_effect_on_self_+%",
        ] {
            assert!(super::super::gems::DEPENDS_ON.contains(&stat), "{stat}");
            for m in mappings {
                assert_eq!(m.map(&StatId::new(stat)), StatMap::Unmapped, "{stat}");
            }
        }
    }

    #[test]
    fn reservation_should_round_as_pob() {
        let none = ReservationTerms::NONE;
        // Without modifiers, as P3-12h.
        assert_eq!(reserved(32, 0, 1.0, none), Some((32.0, 0.0)));
        assert_eq!(reserved(15, 0, 1.3, none), Some((19.0, 0.0)));
        assert_eq!(reserved(0, 2500, 1.2, none), Some((0.0, 30.0)));
        assert_eq!(reserved(0, 2250, 1.1, none), Some((0.0, 24.75)));
        assert_eq!(reserved(0, 0, 1.5, none), Some((0.0, 0.0)));
        // Efficiency divides: 50% / 1.12 = 44.642..., rounded to 44.64.
        let eff = |efficiency| ReservationTerms {
            inc: 0.0,
            efficiency,
        };
        assert_eq!(reserved(0, 5000, 1.0, eff(12.0)), Some((0.0, 44.64)));
        // 25 / 1.3 = 19.23..., rounded half up to 19.
        assert_eq!(reserved(25, 0, 1.0, eff(30.0)), Some((19.0, 0.0)));
        // A negative efficiency raises it: 35% / 0.9 = 38.888... to 38.89.
        assert_eq!(reserved(0, 3500, 1.0, eff(-10.0)), Some((0.0, 38.89)));
        // At -100 PoB divides by 0, unless it reserves nothing anyway.
        assert_eq!(reserved(0, 3500, 1.0, eff(-100.0)), None);
        assert_eq!(reserved(0, 0, 1.0, eff(-100.0)), Some((0.0, 0.0)));
        let gone = ReservationTerms {
            inc: -100.0,
            efficiency: -100.0,
        };
        assert_eq!(reserved(30, 3500, 1.0, gone), Some((0.0, 0.0)));
        // `inc` multiplies, and at -100 or below the skill reserves nothing.
        let inc = |inc| ReservationTerms {
            inc,
            efficiency: 0.0,
        };
        assert_eq!(reserved(0, 5000, 1.0, inc(-20.0)), Some((0.0, 40.0)));
        assert_eq!(reserved(40, 5000, 1.0, inc(-100.0)), Some((0.0, 0.0)));
        // Both: 50 x 0.9 / 1.1 = 40.909... to 40.91.
        let both = ReservationTerms {
            inc: -10.0,
            efficiency: 10.0,
        };
        assert_eq!(reserved(0, 5000, 1.0, both), Some((0.0, 40.91)));
    }

    #[test]
    fn the_efficiency_should_sum_the_pool_and_generic_and_stop_at_minus_100() {
        let t = Totals {
            efficiency: 5.0,
            life_efficiency: 3.0,
            mana_efficiency: 10.0,
            reserved: -4.0,
            aura_effect: 0.0,
            unread: false,
            ..Totals::default()
        };
        assert_eq!(
            t.reservation(Pool::Mana),
            ReservationTerms {
                inc: -4.0,
                efficiency: 15.0
            }
        );
        assert_eq!(t.reservation(Pool::Life).efficiency, 8.0);
        let low = Totals {
            mana_efficiency: -150.0,
            ..Totals::default()
        };
        assert_eq!(low.reservation(Pool::Mana).efficiency, -100.0);
    }

    /// A herald's own mana reservation efficiency adds to the mana and
    /// generic ones for a skill typed `Herald` alone (P6-11ag).
    #[test]
    fn a_heralds_efficiency_should_apply_to_heralds_only() {
        let t = Totals {
            efficiency: 5.0,
            mana_efficiency: 10.0,
            herald_mana_efficiency: 20.0,
            ..Totals::default()
        };
        let herald: BTreeSet<String> = ["Herald".to_owned()].into();
        let aura: BTreeSet<String> = ["Aura".to_owned()].into();
        assert_eq!(
            t.reservation_for(Pool::Mana, &herald, "HeraldOfIce")
                .efficiency,
            35.0
        );
        assert_eq!(
            t.reservation_for(Pool::Mana, &aura, "Hatred").efficiency,
            15.0
        );
        assert_eq!(
            t.reservation_for(Pool::Life, &herald, "HeraldOfIce")
                .efficiency,
            5.0
        );
        // Close-out M6: a named skill's own efficiency.
        let z = Totals {
            mana_efficiency: 10.0,
            named_skill_effect: [0.0, 0.0, 0.0, 0.0, 0.0, 50.0, 0.0],
            ..Totals::default()
        };
        assert_eq!(
            z.reservation_for(Pool::Mana, &aura, "Zealotry").efficiency,
            60.0
        );
        assert_eq!(
            z.reservation_for(Pool::Mana, &aura, "Hatred").efficiency,
            10.0
        );
    }

    #[test]
    fn aura_effect_should_scale_non_curse_auras_only() {
        let t = Totals {
            aura_effect: 25.0,
            ..Totals::default()
        };
        let types = |names: &[&str]| {
            names
                .iter()
                .map(|s| (*s).to_owned())
                .collect::<BTreeSet<_>>()
        };
        assert_eq!(t.buff_scale(true, &types(&["Aura", "Buff"]), 0.0), 1.25);
        assert_eq!(
            t.buff_scale(true, &types(&["Aura", "AppliesCurse"]), 0.0),
            1.0
        );
        // A buff (a herald) reads no aura effect, nor an aura without the
        // type.
        assert_eq!(t.buff_scale(false, &types(&["Aura"]), 0.0), 1.0);
        assert_eq!(t.buff_scale(true, &types(&["Buff"]), 0.0), 1.0);
        // P6-11k: the effect of your skills' auras on you, every aura buff,
        // untagged; never a buff.
        let s = Totals {
            aura_on_self: 10.0,
            ..t
        };
        assert_eq!(s.buff_scale(true, &types(&["Aura", "Buff"]), 0.0), 1.35);
        assert_eq!(s.buff_scale(true, &types(&["Buff"]), 0.0), 1.1);
        assert_eq!(s.buff_scale(false, &types(&["Aura"]), 0.0), 1.0);
        // P3-close: the golems' buff effect for a golem skill's buff, the
        // skill's own on top; neither for another skill.
        let g = Totals {
            golem_buff_effect: 50.0,
            ..t
        };
        assert_eq!(g.buff_scale(false, &types(&["Buff", "Golem"]), 20.0), 1.7);
        assert_eq!(g.buff_scale(false, &types(&["Buff"]), 20.0), 1.2);
        assert_eq!(g.buff_scale(true, &types(&["Aura", "Buff"]), 0.0), 1.25);
        // P6-11at: a herald's buff effect, for a herald skill alone.
        let h = Totals {
            herald_buff_effect: 40.0,
            ..t
        };
        assert_eq!(h.buff_scale(false, &types(&["Buff", "Herald"]), 0.0), 1.4);
        assert_eq!(h.buff_scale(false, &types(&["Buff", "Golem"]), 0.0), 1.0);
        // P6-11gc: a named golem's buff effect, its transfigured skills'
        // too; no other golem's.
        let mut n = Totals::default();
        n.named_golem_buff_effect[Golem::Lightning as usize] = 31.0;
        assert_eq!(n.named_golem(Some("summon_lightning_golem")), 31.0);
        assert_eq!(n.named_golem(Some("summon_lightning_golem_alt_x")), 31.0);
        assert_eq!(n.named_golem(Some("summon_flame_golem")), 0.0);
        assert_eq!(n.named_golem(Some("lightning_golem_wrath")), 0.0);
        assert_eq!(n.named_golem(None), 0.0);
        // P6-11gn: a named skill's effect, for that skill and kind alone.
        let p = Totals {
            named_skill_effect: [20.0, 15.0, 0.0, 0.0, 0.0, 0.0, -50.0],
            ..Totals::default()
        };
        assert_eq!(p.named_skill("Pride", NamedEffect::Aura), 20.0);
        assert_eq!(p.named_skill("Pride", NamedEffect::Curse), 0.0);
        assert_eq!(p.named_skill("Vulnerability", NamedEffect::Curse), 15.0);
        assert_eq!(p.named_skill("Hatred", NamedEffect::Aura), 0.0);
        assert_eq!(p.named_skill("Punishment", NamedEffect::Curse), -50.0);
        // P6-11gg: the offerings' effect, on you with the effect on you.
        let o = Totals {
            offering_buff_effect: 20.0,
            offering_effect_on_you: -50.0,
            ..Totals::default()
        };
        assert_eq!(o.offering("BoneOffering", false), 20.0);
        assert_eq!(o.offering("FleshOffering", true), -30.0);
        assert_eq!(o.offering("Clarity", true), 0.0);
        assert!(!o.offering_on_you("BoneOffering"));
        let o = Totals {
            offerings_affect_you: true,
            ..o
        };
        assert!(o.offering_on_you("SpiritOffering"));
        assert!(!o.offering_on_you("Clarity"));
        let c = Totals {
            curse_effect: 30.0,
            ..Totals::default()
        };
        assert_eq!(c.curse_scale(), 1.3);
    }

    #[test]
    fn a_mod_should_scale_as_pob() {
        let acc = Quantity::Accuracy;
        // Scale 1: unchanged.
        assert_eq!(scale(acc, Op::Base, 117.0, 1.0), 117.0);
        // Whole values: rounded to 2 decimals, then toward zero.
        assert_eq!(scale(acc, Op::Base, 117.0, 1.25), 146.0);
        assert_eq!(scale(acc, Op::Base, 15.0, 1.1), 16.0);
        assert_eq!(scale(acc, Op::Base, 30.0, 1.33), 39.0);
        assert_eq!(scale(Quantity::Damage, Op::More, -9.0, 1.5), -13.0);
        // 1 x 2.996 rounds to 3.0 at 2 decimals before the integer part.
        assert_eq!(scale(acc, Op::Base, 1.0, 2.996), 3.0);
        assert_eq!(scale(acc, Op::Base, 1.0, 2.99), 2.0);
        // Not whole: floored at 1 decimal.
        assert_eq!(scale(acc, Op::Base, 2.5, 1.3), 3.2);
        // High precision: floored at 2 decimals.
        assert_eq!(scale(Quantity::CritChance, Op::Base, 1.0, 1.255), 1.25);
        assert_eq!(
            scale(Quantity::CritChance, Op::Increased, 50.0, 1.255),
            62.0
        );
    }

    #[test]
    fn the_main_skills_aura_stats_should_scale_alone() {
        let min = "minimum_added_lightning_damage_from_skill";
        // Smite's aura: 1 to 19 at 1.1 gives 1 (1.1) and 20 (20.9).
        assert_eq!(main_skill_stat("Smite", min, 1, 1.1), 1);
        assert_eq!(
            main_skill_stat(
                "Smite",
                "maximum_added_lightning_damage_from_skill",
                19,
                1.1
            ),
            20
        );
        assert_eq!(main_skill_stat("Smite", min, 19, 1.0), 19);
        // Another skill's stat, or another stat of Smite: as is.
        assert_eq!(main_skill_stat("IceNova", min, 19, 1.1), 19);
        assert_eq!(main_skill_stat("Smite", "attack_speed_+%", 10, 1.1), 10);
        for (_, stat, quantity) in MAIN_SKILL_AURA_STATS {
            let StatMap::Effects(e) = SkillMapping.map(&StatId::new(*stat)) else {
                panic!("{stat}");
            };
            assert_eq!(e.len(), 1, "{stat}");
            assert_eq!(e[0].quantity, *quantity, "{stat}");
            assert_eq!(e[0].op, Op::Base, "{stat}");
        }
    }

    #[test]
    fn totals_should_read_unconditional_increases_only() {
        use crate::source::Source;
        use crate::store::Effect;
        let mut ingest = Ingest::new(&Poe1Mapping);
        let effect = |q, op, value, conditions| Effect {
            quantity: Quantity::Poe1(q),
            op,
            value,
            conditions,
            source: Source::Rule("test"),
        };
        ingest.effect(effect(
            Poe1Quantity::ReservationEfficiency(Some(Pool::Mana)),
            Op::Increased,
            12.0,
            Conditions::none(),
        ));
        ingest.effect(effect(
            Poe1Quantity::ReservationEfficiency(Some(Pool::Mana)),
            Op::Increased,
            8.0,
            Conditions::none(),
        ));
        ingest.effect(effect(
            Poe1Quantity::NonCurseAuraEffect,
            Op::Increased,
            10.0,
            Conditions::none(),
        ));
        ingest.effect(effect(
            Poe1Quantity::NonCurseAuraEffect,
            Op::More,
            50.0,
            Conditions::none(),
        ));
        let t = Totals::read(&ingest);
        assert_eq!(t.mana_efficiency, 20.0);
        assert_eq!(t.aura_effect, 10.0);
        assert_eq!(t.efficiency, 0.0);
        // The MORE effect is not summed: it is unread.
        assert!(t.unread);
        let mut plain = Ingest::new(&Poe1Mapping);
        plain.effect(effect(
            Poe1Quantity::Reserved(None),
            Op::Increased,
            -8.0,
            Conditions::none(),
        ));
        assert!(!Totals::read(&plain).unread);
        plain.effect(effect(
            Poe1Quantity::Reserved(Some(Pool::Mana)),
            Op::Increased,
            5.0,
            Conditions::none(),
        ));
        assert!(Totals::read(&plain).unread);
    }

    #[test]
    fn names_should_be_stable() {
        for (q, name) in [
            (
                Poe1Quantity::ReservationEfficiency(None),
                "poe1.reservation-efficiency",
            ),
            (
                Poe1Quantity::ReservationEfficiency(Some(Pool::Mana)),
                "poe1.reservation-efficiency.mana",
            ),
            (Poe1Quantity::Reserved(None), "poe1.reserved"),
            (
                Poe1Quantity::Reserved(Some(Pool::Life)),
                "poe1.reserved.life",
            ),
            (
                Poe1Quantity::NonCurseAuraEffect,
                "poe1.non-curse-aura-effect",
            ),
        ] {
            assert_eq!(Quantity::Poe1(q).to_string(), name);
        }
    }
}
