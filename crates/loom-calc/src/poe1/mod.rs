//! Path of Exile rules (ADR-001; Phase 3 plan, decision 13). The shared
//! engine (store, graph, states, math) is game-neutral; everything only
//! Path of Exile has lives here: its quantities and fields, its
//! stat-to-effect mapping (P3-8), and its phases (P3-6 on).
//!
//! P3-6 computes the character fields (attributes, life, mana, energy
//! shield from global sources, unreserved life and mana): [`collect`]
//! gathers the build, [`mapping`] maps the stats those fields read,
//! [`phases`] computes them. P3-7 adds the core defences: armour,
//! evasion, energy shield with armour pieces, resistances, block, spell
//! block, spell suppression. P3-9b computes the main skill's hit when it is
//! a self-cast spell ([`hit`]): average damage, cast rate, crit chance and
//! multiplier, hit DPS. Its other hit fields, and every hit field of
//! another kind of skill, are `Unsupported` with `NotComputed`.

pub mod ailment;
pub mod buffs;
pub mod charges;
pub mod collect;
pub mod combined;
pub mod conditions;
pub mod config;
pub mod costs;
pub mod damage_taken;
pub mod devotion;
pub mod dot;
pub mod ehp;
pub mod enemy;
pub mod enemy_hit;
pub mod evade;
pub mod exposure;
pub mod flasks;
pub mod full_dps;
pub mod gem_levels;
pub mod gems;
pub mod hit;
pub mod item_supports;
pub mod item_values;
pub mod keystones;
pub mod leech;
pub mod mapping;
pub mod minion_actor;
pub mod minion_hit;
pub mod minions;
pub mod movement;
pub mod pantheon;
pub mod pob_minion;
pub mod reduction;
pub mod regen;
pub mod repeats;
pub mod resist_conversion;
#[rustfmt::skip]
pub mod pob_minions;
pub mod pob_skill;
#[rustfmt::skip]
pub mod pob_skills;
pub mod phases;
pub mod skill;
pub mod skill_modifiers;
pub mod skill_no_output;
pub mod sockets;
pub mod stacks;
pub mod states;
pub mod taken;
pub mod triggers;

pub use mapping::Poe1Mapping;
pub use pantheon::Poe1Data;

use loom_domain::Game;

use crate::engine::{Phase, Ruleset};
use crate::error::CalcError;
use crate::ingest::{Ingest, StatMapping};
use crate::input::CalcInput;
use crate::quantity::{Bound, DamageType};
use crate::result::Field;

/// A quantity only Path of Exile has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Poe1Quantity {
    /// Chance to suppress spell damage.
    SpellSuppressionChance,
    /// Chance to block spell damage.
    SpellBlockChance,
    /// Maximum chance to block spell damage.
    MaximumSpellBlockChance,
    /// A value of the armour piece in a slot (its local properties).
    Piece(ArmourSlot, PieceValue),
    /// The main skill's own base damage of a type (its level's
    /// "spell_minimum_base_cold_damage", ...).
    SkillBaseDamage(DamageType, Bound),
    /// The main skill's base critical strike chance, percent (its level's).
    SkillCritChance,
    /// The main skill's damage effectiveness (its level's), which scales
    /// added damage.
    DamageEffectiveness,
    /// The main skill's base damage multiplier (its level's).
    BaseDamageMultiplier,
    /// The main skill's cast time, seconds.
    CastTime,
    /// A value of the weapon in the main hand (its local properties).
    Weapon(WeaponValue),
    /// A value of the weapon in the off hand (its local properties; Phase
    /// 6, dual wielding).
    OffHandWeapon(WeaponValue),
    /// The skill's own conversion of one damage type to another, percent
    /// (PoB's `Skill<From>DamageConvertTo<To>`, which takes precedence over
    /// other conversion).
    SkillConversion(DamageType, DamageType),
    /// The main skill's attack speed multiplier, percent (its level's: PoB's
    /// `attackSpeedMultiplier`).
    AttackSpeedMultiplier,
    /// The enemy's largest physical damage reduction, percent.
    MaximumPhysicalDamageReduction,
    /// The enemy's damage taken, every damage type (PoB's `DamageTaken` on
    /// the enemy): its `Increased` and `More` effects scale each hit
    /// ([`damage_taken`]).
    EnemyDamageTaken,
    /// The enemy's evasion modifiers (PoB's `Evasion` INC and MORE on the
    /// enemy, `calcLib.val`): they scale its base evasion before an
    /// attack's chance to hit (Blind, [`damage_taken`]; P3-12g).
    EnemyEvasion,
    /// The enemy's resistance to an elemental damage type from exposure
    /// (PoB's `<Type>Resist` BASE on the enemy, P3-8k): `Base`, only
    /// [`exposure`] adds them; a hit adds their sum to the enemy's base
    /// resistance before its maximum ([`hit`]).
    EnemyResistance(DamageType),
    /// A chance to inflict exposure of an elemental damage type (PoB's
    /// `<Type>ExposureChance`, P3-8k): `Base`; [`exposure`] reads only
    /// whether the sum is above 0 (PoB's `CanApply<Type>Exposure`).
    ExposureChance(DamageType),
    /// "Exposure you inflict applies an extra #%" (PoB's `ExtraExposure`
    /// with no type, `Extra<Type>Exposure` with one; P3-8k): `Base`, read
    /// by [`exposure`].
    ExtraExposure(Option<DamageType>),
    /// "Shocks from your Hits always increase Damage taken by at least #%"
    /// (PoB's `ShockMinimum`, P3-8k): `Base`, read by [`damage_taken`].
    ShockMinimum,
    /// The effect of the shocks you inflict (PoB's `EnemyShockEffect`,
    /// P3-8k): `Increased`, read by [`damage_taken`] to scale a guaranteed
    /// shock.
    ShockEffect,
    /// Onslaught (PoB's `Condition:Onslaught`, turned into the `Onslaught`
    /// flag by `CalcSetup.lua` 97): a `Flag` effect from a stat that grants
    /// it (P3-12j); [`buffs`] gives its speed once, whatever the number of
    /// sources, as PoB's flag does.
    Onslaught,
    /// Flasks applied to you have increased effect (PoB's `FlaskEffect`;
    /// P6-4b): `Increased`, read by [`flasks`].
    FlaskEffect,
    /// Magic utility flasks applied to you have increased effect (PoB's
    /// `MagicUtilityFlaskEffect`; P6-4b): `Increased`, read by [`flasks`].
    MagicUtilityFlaskEffect,
    /// A flask's own increased effect (PoB's local `FlaskEffect`; P6-4b):
    /// `Increased`, read by [`flasks`] from the flask's lines alone.
    LocalFlaskEffect,
    /// A modifier of a charge kind (P3-8d): its maximum, its minimum, or an
    /// effect per charge. Only [`charges`] reads them: it counts the
    /// charges as PoB's `doActorCharges` and turns each per-charge effect
    /// into its target times the count.
    Charge(charges::ChargeKind, charges::ChargeValue),
    /// A skill's reservation of a pool (P3-12h, [`gems`]): one `Base`
    /// effect per reserving skill, its value PoB's per-skill amount
    /// (`CalcPerform.lua` 1975 to 1989), summed by the unreserved phase.
    Reservation(Pool, ReservationKind),
    /// A global gem level stat of [`gem_levels::LEVEL_STATS`], by stat ID
    /// (P3-8i): `Base`, read once by [`gem_levels::GemLevels`] when the
    /// skills are set up, as PoB reads its `GemProperty` modifiers.
    GemLevel(&'static str),
    /// A socketed gem level stat of [`gem_levels::SOCKETED_STATS`] on the
    /// item in a slot (P6-11q): `Base`, read by [`gem_levels::GemLevels`]
    /// for the gems of the groups in that slot.
    SocketedGemLevel(&'static str, gem_levels::GemSlot),
    /// "Socketed Skill Gems get a #% Cost & Reservation Multiplier" on the
    /// item in a slot (P6-11bi; PoB's `ExtraSkillMod`
    /// `SupportManaMultiplier` MORE of the value less 100): `Base`, the
    /// multiplier in percent, read by [`gem_levels::GemLevels`] for the
    /// skills of the groups in that slot.
    SocketedCostMultiplier(gem_levels::GemSlot),
    /// A support an item grants the gems in its slot, by the stat of
    /// [`item_supports::ROWS`] (P6-11r): `Base`, the stat's value (the
    /// support's level unless the row fixes it), read by
    /// [`gem_levels::GemLevels`] for the groups in that slot.
    ItemSupport(&'static str, gem_levels::GemSlot),
    /// A minion modifier (P3-8j): "Minions deal #% increased Damage" and
    /// kin, `Increased`. No minion is computed; only [`minions`] reads
    /// them, to give the character those a redirect names.
    Minion(minions::MinionValue),
    /// A redirect of a minion modifier kind to the character at a percent
    /// of their value (P3-8j): a `Flag` ("... also affect you", 100; "... at
    /// 150% of their value", 150); [`minions`] takes the largest.
    MinionToYou(minions::MinionValue, u16),
    /// Block chance per 5% of another block chance (PoB's `PerStat` tag
    /// with `div = 5`; P6-11f): `Base`, read by [`phases`]. `true`: spell
    /// block per 5% chance to block attack damage (`BlockChance`); `false`:
    /// attack block per 5% chance to block on the equipped shield
    /// (`ShieldBlockChance`).
    BlockPer5(bool),
    /// The enemy's projectile damage taken (PoB's `ProjectileDamageTaken`
    /// INC on the enemy; Sniper's Mark, P6-9e): `Increased`, read only by a
    /// projectile hit ([`hit`]).
    EnemyProjectileDamageTaken,
    /// A pool's regeneration (P6-10b): `Base` the flat amount per second
    /// (PoB's `<Pool>Regen` BASE), `Increased` and `More` its rate, read by
    /// [`regen`].
    Regen(Pool),
    /// A pool's regeneration as a percent of the pool per second (PoB's
    /// `<Pool>RegenPercent` BASE; P6-10b): `Base`, read by [`regen`].
    RegenPercent(Pool),
    /// A pool's regeneration as a percent of the pool per minute (the
    /// game's per-minute stats on a state row, which has no scale; P6-10b):
    /// `Base`, read by [`regen`] divided by 60.
    RegenPercentPerMinute(Pool),
    /// A pool's flat regeneration per minute (a state row's per-minute
    /// stat, which has no scale; P6-10b): `Base`, read by [`regen`] divided
    /// by 60.
    RegenPerMinute(Pool),
    /// A pool's increased regeneration rate per 1% chance to block spell
    /// damage (PoB's `<Pool>Regen` INC with `PerStat` `SpellBlockChance`,
    /// P6-11t): `Base`, read by [`regen`] times the spell block chance,
    /// floored.
    RegenIncPerSpellBlock(Pool),
    /// A pool's flat regeneration per second per 1% uncapped fire
    /// resistance (PoB's `<Pool>Regen` BASE with `PerStat`
    /// `FireResistTotal`, P6-11t): `Base`, read by [`regen`] times the
    /// resistance's total, floored.
    RegenPerFireResist(Pool),
    /// A stat that changes a pool's regeneration in a way the engine does
    /// not model (P6-10b; [`mapping::REGEN_REASONS`]): a `Flag` only
    /// [`regen`] reads, as that field's `unmapped-stat` reason, so the
    /// character's other fields keep their state.
    RegenReason(Pool, &'static str),
    /// A pool's recovery rate (PoB's `<Pool>RecoveryRate`; P6-10b):
    /// `Increased` and `More`, read by [`regen`].
    RecoveryRate(Pool),
    /// Energy shield's recharge rate (PoB's `EnergyShieldRecharge`; P6-10b):
    /// `Increased` and `More`, read by [`regen`].
    EnergyShieldRecharge,
    /// The faster start of energy shield's recharge (PoB's
    /// `EnergyShieldRechargeFaster`; P6-10b): `Increased`, read by [`regen`].
    EnergyShieldRechargeFaster,
    /// Zealot's Oath (PoB's `ZealotsOath`; P6-10b): a `Flag`, read by
    /// [`regen`].
    ZealotsOath,
    /// "You have Consecrated Ground around you while stationary" (PoB's
    /// `Condition:OnConsecratedGround` with `Condition:Stationary`; P6-10b):
    /// a `Flag`, read by [`buffs`].
    ConsecratedWhileStationary,
    /// Arcane Surge's increased mana regeneration from its sources (PoB's
    /// `ArcaneSurgeManaRegen`, read as its largest; P6-10b): `Base`, read
    /// by [`buffs`].
    ArcaneSurgeManaRegen,
    /// A hit's chance to ignore the enemy's physical damage reduction (PoB's
    /// `ChanceToIgnoreEnemyPhysicalDamageReduction`; P6-11j): `Base`, read
    /// by a hit's armour ([`hit`]).
    IgnorePdrChance,
    /// The enemy's physical damage reduction (PoB's `PhysicalDamageReduction`
    /// BASE on the enemy; Poacher's Mark, P6-9e): `Base`, added to the
    /// armour reduction of a physical hit, and alone for physical damage
    /// over time ([`hit`]).
    EnemyPhysicalDamageReduction,
    /// Additional curses you can apply (PoB's `EnemyCurseLimit` BASE beyond
    /// the base 1; P6-9b): `Base`, read by [`gems`] for the curse slots.
    CurseLimit,
    /// A minion modifier for golems only ("Golems have #% increased
    /// Maximum Life"; PoB's `MinionModifier` tagged `SkillType.Golem`,
    /// `ModParser.lua` 1137; P6-7b3b): no character field reads it; the
    /// minion step applies it when the main skill is a golem skill.
    GolemMinion(minions::MinionValue),
    /// Reservation efficiency of a pool, or of both (PoB's
    /// `<Pool>ReservationEfficiency` and `ReservationEfficiency`, P3-8l):
    /// `Increased`, read by [`skill_modifiers::Totals`] for the other
    /// gems' reservation.
    ReservationEfficiency(Option<Pool>),
    /// Mana reservation efficiency of herald skills (PoB's
    /// `ManaReservationEfficiency` INC tagged `SkillType.Herald`; P6-11ag):
    /// `Increased`, read by [`skill_modifiers::Totals`] for a skill typed
    /// `Herald`.
    HeraldManaReservationEfficiency,
    /// The skill's additional repeats (PoB's `RepeatCount` BASE; P6-11aj),
    /// read by [`repeats`].
    RepeatCount,
    /// The damage of the `n`th repeat (PoB's `RepeatOneDamage` to
    /// `RepeatThreeDamage` MORE; P6-11aj).
    RepeatDamage(u8),
    /// Damage per repeat (PoB's `RepeatPerRepeatDamage` MORE; P6-11aj).
    RepeatPerRepeatDamage,
    /// The final repeat's chance to deal double damage (PoB's
    /// `RepeatFinalDoubleDamageChance` BASE; P6-11aj).
    RepeatFinalDoubleDamageChance,
    /// The `Damage` MORE [`repeats`] derives (P6-11aj): read with the
    /// hit's `Damage` MORE, apart from it (PoB multiplies it in without
    /// rounding it with the skill's other `Damage` MORE: a PoB oracle
    /// probe, `docs/phase-6/stat-mop-up.md`, "P6-11aj").
    RepeatAverageDamage,
    /// What the character recovers when it blocks (PoB's `LifeOnBlock`,
    /// `EnergyShieldOnBlock`, `EnergyShieldOnSpellBlock` BASE; P6-11ak),
    /// read by [`enemy_hit`] for the effective health pool.
    BlockGain(BlockGain),
    /// The chance to suppress spell damage is lucky (PoB's
    /// `SpellSuppressionChanceIsLucky`; P6-11al): a `Flag`, read by
    /// [`enemy_hit`] for the effective chance.
    SuppressionLucky,
    /// Energy shield cannot be recovered above armour (PoB's
    /// `ArmourESRecoveryCap`; P6-11am, Divine Shield): a `Flag`, read by
    /// [`enemy_hit`] for `EnergyShieldRecoveryCap`.
    ArmourEsRecoveryCap,
    /// Energy shield protects mana instead of life (PoB's
    /// `EnergyShieldProtectsMana`; P6-11ap, Eldritch Battery): a `Flag`,
    /// read by [`enemy_hit`] and [`ehp`] for the pools and by [`costs`] for
    /// the mana cost per second, paid with energy shield.
    EsProtectsMana,
    /// A skill can wither (PoB's `Condition:CanWither`; P6-11aq): a `Flag`
    /// from the character's stats, which every active skill then has;
    /// read by [`damage_taken`] for the enemy's withered stacks.
    CanWither,
    /// The share of melee damage taken from Summon Stone Golem of
    /// Safeguarding's life before yours, percent (PoB's
    /// `takenFromStoneGolemBeforeYou`; P6-11ar): `Base`, read by
    /// [`enemy_hit`].
    TakenFromStoneGolem,
    /// The effect of your Withered (PoB's `WitherEffect`; P6-11aq):
    /// `Increased`, read by [`damage_taken`].
    WitherEffect,
    /// Physical damage prevented regenerated as energy shield, percent
    /// (PoB's `PhysicalDamageMitigatedEnergyShieldPseudoRecoup`; P6-11am,
    /// Divine Shield): `Base`. It reaches no field itself; any recoup makes
    /// PoB's estimate track recoupable damage, which limits its speed-up
    /// (`ehp`).
    PseudoRecoup,
    /// Damage taken recouped as mana, percent (PoB's `ManaRecoup`;
    /// P6-11as): `Base`, read by [`enemy_hit`] for `anyRecoup`.
    ManaRecoup,
    /// Damage taken recouped as life, percent (PoB's `LifeRecoup`; P6-10a,
    /// a Glorious Vanity conquest): `Base`, read by [`enemy_hit`] for
    /// `anyRecoup` as [`Self::ManaRecoup`].
    LifeRecoup,
    /// Devotion (PoB's `Devotion`; P6-11az): `Base`, summed by
    /// [`devotion`].
    Devotion,
    /// A line that reads devotion ([`devotion::READERS`], by index): `Base`,
    /// its value, which [`devotion`] scales and ingests as the stat it
    /// means.
    DevotionReader(u8),
    /// Skills reserve life instead of mana (PoB's `BloodMagicReserved`;
    /// P6-11an, Blood Magic): a `Flag`, read by
    /// [`skill_modifiers::Totals`] for the other gems' reservation.
    ReserveLifeInsteadOfMana,
    /// Energy shield recharge applies to life instead (PoB's
    /// `EnergyShieldRechargeAppliesToLife`; P6-11ao, Eternal Youth): a
    /// `Flag`; [`regen`] then sets no energy shield recharge.
    EsRechargeAppliesToLife,
    /// The hit's repeat count as [`repeats`] settled it (PoB's
    /// `output.Repeats`; P6-11aj): `Base`, for the rate and the costs.
    Repeats,
    /// The share of fire resistance modifiers that also apply to another
    /// type, percent (PoB's `FireMaxResConvertTo<To>` when the flag is
    /// `true`, else `FireResConvertTo<To>`, BASE; P6-11ah), read by
    /// [`resist_conversion`].
    ResistConvert(bool, crate::quantity::DamageType),
    /// Reservation of a pool, or of both (PoB's `<Pool>Reserved` and
    /// `Reserved`, P3-8l): `Increased`, read as
    /// [`Self::ReservationEfficiency`].
    Reserved(Option<Pool>),
    /// The effect of non-curse auras from your skills (PoB's `AuraEffect`
    /// tagged `Aura` and not `AppliesCurse`, P3-8l): `Increased`, read by
    /// [`skill_modifiers::Totals`] to scale the other gems' auras.
    NonCurseAuraEffect,
    /// "Auras from your Skills have #% increased Effect on you" (PoB's
    /// `SkillAuraEffectOnSelf`, P6-11k): `Increased`, read by
    /// [`skill_modifiers::Totals`] for every aura buff on you.
    SkillAuraEffectOnSelf,
    /// Culling Strike (PoB's `CullPercent` MAX 10; P6-12): a `Flag`, read
    /// by [`combined`].
    CullingStrike,
    /// A defence's increase from one equipped piece only (PoB's
    /// `<Defence>` or `Defences` INC tagged `SlotName`, P6-11k: "#%
    /// increased Energy Shield from Equipped Shield"): `Increased`, added to
    /// that piece's modifier ([`phases`]).
    SlotIncreased(ArmourSlot, Defence),
    /// "Your hits can't be Evaded" (PoB's `CannotBeEvaded`, P3-8m): a
    /// `Flag`; an attack's chance to hit is then 100 ([`hit`]).
    CannotBeEvaded,
    /// "Never deal Critical Strikes" (PoB's `NeverCrit`, P3-8m): a `Flag`;
    /// the hit's crit chance and crit multiplier are then 0 ([`hit`]).
    NeverCrit,
    /// "Your Critical Strikes do not deal extra Damage" (PoB's
    /// `NoCritMultiplier`, P3-8m): a `Flag`; the crit multiplier is then 1
    /// ([`hit`]).
    NoCritMultiplier,
    /// "Cannot deal non-Cold Damage", per type the hit cannot deal (PoB's
    /// `DealNo<Type>`, P3-8m): a `Flag`; the hit deals none of that type,
    /// though its damage still converts to others ([`hit`]).
    DealNo(DamageType),
    /// The enemy's damage taken of one damage type (PoB's
    /// `<Type>DamageTaken` on the enemy, P3-8m): `Increased`, only
    /// [`damage_taken`] adds them; a hit of that type adds them to
    /// [`Self::EnemyDamageTaken`]'s sum ([`hit`]).
    EnemyTypeDamageTaken(DamageType),
    /// Chill raises the enemy's damage taken by its effect (P3-8m): a
    /// `Flag`, `None` for every type (PoB's `ChillEffectIncDamageTaken`),
    /// `Some(Cold)` for cold (`ChillEffectIncColdDamageTaken`), read by
    /// [`damage_taken`].
    ChillDamageTaken(Option<DamageType>),
    /// The effect of the chills you inflict (PoB's `EnemyChillEffect`,
    /// P3-8m): `Increased`, read by [`damage_taken`] to scale a skill's
    /// guaranteed chill.
    ChillEffect,
    /// A skill's guaranteed chill (PoB's `ChillOverride` from an active
    /// skill typed `ChillingArea` or `NonHitChill`, P3-8m): `Base`, the
    /// default chill (10) per skill, which [`gems`] adds and
    /// [`damage_taken`] scales by [`Self::ChillEffect`].
    SkillChill,
    /// Action speed (PoB's `ActionSpeed`, P3-8n): `Increased`; a self-cast
    /// skill's rate is scaled by `max(1 + inc / 100, 0)` before the server
    /// tick cap ([`hit`]).
    ActionSpeed,
    /// Movement speed (PoB's `MovementSpeed`; P6-10b): `Increased` and
    /// `More`, read by [`movement`].
    MovementSpeed,
    /// A stat that changes movement speed in a way the engine does not model
    /// (P6-10b; [`mapping::MOVEMENT_REASONS`]): a `Flag` only [`movement`]
    /// reads, as those fields' `unmapped-stat` reason.
    MovementReason(&'static str),
    /// A skill's mana cost (PoB's `ManaCost`; P6-10b): `Increased`, `More`,
    /// and `Base` (the total), read by [`costs`].
    ManaCost,
    /// A skill's cost of any resource (PoB's `Cost`; P6-10b): `Increased`,
    /// `More`, and `Base`, read by [`costs`].
    Cost,
    /// Mana cost added after the support multiplier (PoB's
    /// `ManaCostNoMult`; P6-10b): `Base`, read by [`costs`].
    ManaCostNoMult,
    /// The total mana cost of channelling (`true`) or non-channelling
    /// skills (PoB's `ManaCost` BASE with a `SkillType` `Channel` tag,
    /// negated for `false`; P6-11t): `Base`, read by [`costs`] for a main
    /// skill of that kind.
    ManaCostChannelling(bool),
    /// Mana cost efficiency (PoB's `ManaCostEfficiency`; P6-10b):
    /// `Increased`, read by [`costs`].
    ManaCostEfficiency,
    /// Cost efficiency (PoB's `CostEfficiency`; P6-10b): `Increased`, read by
    /// [`costs`].
    CostEfficiency,
    /// A stat that changes the cost in a way the engine does not model
    /// (P6-10b; [`mapping::COST_REASONS`]): a `Flag` only [`costs`] reads.
    CostReason(&'static str),
    /// A skill's life cost (PoB's `LifeCost`; P6-10b part 8): `Increased`,
    /// `More`, and `Base` (the total), read by [`costs`].
    LifeCost,
    /// A skill's energy shield cost (PoB's `ESCost`; P6-10b part 8).
    EnergyShieldCost,
    /// Life cost efficiency (PoB's `LifeCostEfficiency`; P6-10b part 8).
    LifeCostEfficiency,
    /// Energy shield cost efficiency (PoB's `ESCostEfficiency`; P6-10b
    /// part 8).
    EnergyShieldCostEfficiency,
    /// Skills cost life instead of mana (PoB's `CostLifeInsteadOfMana`;
    /// P6-10b part 8): a `Flag` [`costs`] reads.
    CostLifeInsteadOfMana,
    /// Skills cost energy shield instead of mana or life (PoB's
    /// `CostESInsteadOfManaOrLife`; P6-10b part 8): a `Flag`.
    CostEnergyShieldInsteadOfManaOrLife,
    /// The percent of the mana cost paid in life instead (PoB's
    /// `HybridManaAndLifeCost_Life`; P6-10b part 8): `Base`.
    HybridLifeCost,
    /// A base life cost as a percent of the mana cost (PoB's
    /// `ManaCostAsLifeCost`; P6-10b part 8): `Base`.
    ManaCostAsLifeCost,
    /// A base energy shield cost as a percent of the mana cost (PoB's
    /// `ManaCostAsEnergyShieldCost`; P6-10b part 8): `Base`.
    ManaCostAsEnergyShieldCost,
    /// A damage type's base damage reduction, percent (PoB's
    /// `<Type>DamageReduction` BASE; P6-10b part 5): read by [`reduction`].
    DamageReduction(DamageType),
    /// The largest total recovery per second from a pool's leech, percent
    /// of the pool (PoB's `Max<Pool>LeechRate`; P6-10b part 7): `Base`,
    /// `Increased`, and `More`, read by [`leech`].
    LeechRateMax(Pool),
    /// The largest recovery of one leech of a pool, percent of the pool
    /// (PoB's `Max<Pool>LeechInstance`; P6-10b part 7): read by [`leech`].
    LeechInstanceMax(Pool),
    /// Damage leeched as a pool, percent (PoB's `Damage<Pool>Leech`,
    /// `<Type>Damage<Pool>Leech`, `ElementalDamage<Pool>Leech`, and
    /// `DamageLeech` for life and mana; P6-10b part 9): `Base`, read by
    /// [`leech`] in the hit's context.
    DamageLeech(Pool, LeechSource),
    /// A pool's leech rate (PoB's `<Pool>LeechRate`; P6-10b part 9):
    /// `Increased` and `More`.
    LeechRate(Pool),
    /// The percent of a pool's leech that is instant (PoB's
    /// `Instant<Pool>Leech`; P6-10b part 9): `Base`.
    InstantLeech(Pool),
    /// The skill cannot leech the pool (PoB's `CannotLeech<Pool>`; P6-10b
    /// part 9): a `Flag`.
    CannotLeech(Pool),
    /// Damage the character takes (PoB's `DamageTaken`, `<Type>DamageTaken`,
    /// `ElementalDamageTaken`, and their `WhenHit` and `OverTime` forms;
    /// P6-10b part 10): `Increased` and `More`, read by [`taken`].
    SelfDamageTaken(TakenWhen, TakenOf),
    /// The enemy's accuracy rating (PoB's enemy `Accuracy`; P6-10b part
    /// 11): `Increased` and `More` (Blind's), read by [`evade`].
    EnemyAccuracy,
    /// The character cannot evade (PoB's `CannotEvade`; P6-10b part 11): a
    /// `Flag` [`evade`] reads.
    CannotEvade,
    /// A stat that changes the enemy's hit in a way the engine does not
    /// model (P6-10b part 12; a gem's [`gems::BuffStat::Defence`]): a
    /// `Flag` only [`enemy_hit`] reads, as its fields' reason.
    EnemyHitReason(&'static str),
    /// A stat that changes the pools maximum hit reads in a way the engine
    /// does not model (P6-10b part 13: guard skills, Mind over Matter,
    /// Divine Shield): a `Flag` only those fields read.
    PoolReason(&'static str),
    /// A stat that changes damage taken over time in a way the engine does
    /// not model (P6-10b part 13): a `Flag` only [`taken`] reads.
    DotTakenReason(&'static str),
    /// The enemy's crit chance against the character (PoB's
    /// `EnemyCritChance` INC; P6-10b part 12).
    EnemyCritChance,
    /// Less extra damage taken from crits (PoB's `ReduceCritExtraDamage`
    /// BASE; P6-10b part 12).
    CritExtraDamageReduction,
    /// The enemy's damage (PoB's enemy `Damage`; P6-10b part 12):
    /// `Increased` (delirium's) and `More`.
    EnemyDamage,
    /// A share of a type's damage taken as another (PoB's
    /// `<Type>DamageTakenAs<Other>`, `ElementalDamageTakenAs<Other>`, and
    /// their `FromHits` forms when the flag is set; P6-10b part 12): `Base`.
    DamageTakenAs(TakenOf, DamageType, bool),
    /// The percent of armour that applies to a type's damage taken (PoB's
    /// `ArmourAppliesTo<Type>DamageTaken`; P6-10b part 12): `Base`.
    ArmourAppliesTo(DamageType),
    /// The suppressed spell damage prevented (PoB's `SpellSuppressionEffect`
    /// BASE; P6-10b part 12).
    SpellSuppressionEffect,
    /// The maximum fortification (PoB's `MaximumFortification` BASE; P6-10b
    /// part 12), read by [`enemy_hit`] while Fortified.
    MaximumFortification,
    /// The share of maximum life at or below which you count as on low
    /// life, a fraction (PoB's `LowLifePercentage` BASE; P6-11ad), read by
    /// [`enemy_hit`] for `LifeRecoverable` with the config's
    /// `conditionLowLife`.
    LowLifePercentage,
    /// The share of a hit a guard skill takes, percent (PoB's
    /// `GuardAbsorbRate` BASE; P6-11ae), read by [`enemy_hit`].
    GuardAbsorbRate,
    /// A guard's absorb limit (PoB's `GuardAbsorbLimit` BASE; P6-11ae).
    GuardAbsorbLimit,
    /// A guard's absorb limit per point of armour, a fraction (PoB's
    /// `GuardAbsorbLimit` BASE `PerStat` `Armour`, Molten Shell's; P6-11ae),
    /// at most [`Self::GuardAbsorbMax`] in total.
    GuardAbsorbPerArmour,
    /// The total cap of [`Self::GuardAbsorbPerArmour`] (PoB's
    /// `Multiplier:GuardAbsorbMax` BASE; P6-11ae).
    GuardAbsorbMax,
    /// The share of damage taken from mana before life, percent (PoB's
    /// `DamageTakenFromManaBeforeLife` and `<Type>DamageTakenFromManaBeforeLife`
    /// BASE; P6-10b part 15: mind over matter), read by [`enemy_hit`] and
    /// [`ehp`].
    ManaBeforeLife(TakenOf),
    /// The share of a blocked hit's damage taken, percent (PoB's
    /// `BlockEffect` BASE; P6-10b part 14), read by [`ehp`].
    BlockEffect,
    /// The chance to avoid projectiles (PoB's `AvoidProjectilesChance` BASE;
    /// P6-10b part 14), read by [`ehp`].
    AvoidProjectiles,
    /// A stat that changes the effective health pool alone in a way the
    /// engine does not model (P6-10b part 14): a `Flag` only [`ehp`]'s
    /// fields read.
    EhpReason(&'static str),
    /// The elemental types' base damage reduction, percent (PoB's
    /// `ElementalDamageReduction` BASE; P6-10b part 5): read by
    /// [`reduction`] for fire, cold, and lightning.
    ElementalDamageReduction,
    /// A stat that changes the damage reduction in a way the engine does
    /// not model (P6-10b part 5; [`mapping::REDUCTION_REASONS`]): a `Flag`
    /// only [`reduction`] reads.
    ReductionReason(&'static str),
    /// A stat that changes a hit's rate or DPS in a way the engine does not
    /// model unless a trigger sets the rate (P6-8a; [`mapping::HIT_REASONS`]):
    /// a `Flag` only [`hit`] reads.
    HitReason(&'static str),
    /// A chance to deal double damage, percent (PoB's `DoubleDamageChance`,
    /// P3-8n): `Base`; a hit's damage is scaled by `1 + min(chance, 100) /
    /// 100` ([`hit`]).
    DoubleDamageChance,
    /// Conversion of one damage type to another that is not the skill's
    /// own (PoB's `<From>DamageConvertTo<To>`, P3-8n): `Base`; scaled down
    /// to what the skill's own conversion leaves ([`hit`]).
    Conversion(DamageType, DamageType),
    /// A config multiplier or a count of stacks, and the modifiers per
    /// stack ([`stacks`], P3-8n).
    Stack(stacks::StackKind, stacks::StackValue),
    /// Arcane Surge's cast speed (PoB's `ArcaneSurgeCastSpeed` MAX,
    /// P3-8n): `Base`, the largest read by [`buffs`] (20 without one).
    ArcaneSurgeCastSpeed,
    /// "Gain Convergence when you Hit a Unique Enemy" (PoB's
    /// `Condition:CanGainConvergence`, P3-8n): a `Flag` [`buffs`] reads.
    CanGainConvergence,
    /// The effect of consecrated ground you create (PoB's
    /// `ConsecratedGroundEffect`, P3-8n): `Increased`, read by
    /// [`damage_taken`].
    ConsecratedGroundEffect,
    /// The damage taken consecrated ground applies to enemies (PoB's
    /// `DamageTakenConsecratedGround` on the enemy, P3-8n): `Increased`,
    /// read by [`damage_taken`].
    ConsecratedGroundDamageTaken,
    /// "Enemies Ignited or Chilled by you have #% to Elemental
    /// Resistances" (P3-8n): `Base`, read by [`damage_taken`].
    AilmentElementalResistance,
    /// "#% increased Damage with Hits against Chilled Enemies" (P6-11bf):
    /// `Increased`, which [`damage_taken`] turns into hit `Damage` INC
    /// while the enemy is chilled (PoB's `ActorCondition` `Chilled` on the
    /// enemy).
    DamageVsChilled,
    /// "Enemies permanently take #% increased Damage for each second
    /// they've ever been Chilled by you" (P3-8n): `Base`, read by
    /// [`damage_taken`].
    ChillSecondsDamageTaken,
    /// "Inflict <Type> Exposure on Hit, applying #% to <Type> Resistance"
    /// (PoB's `<Type>ExposureChance` 100 and the enemy's `<Type>Exposure`,
    /// P3-8n): `Base`, the exposure, read by [`exposure`].
    ExposureOnHit(DamageType),
    /// A pool's regeneration, percent of the pool per minute, while you
    /// have applied an exposure recently (PoB's `<Pool>RegenPercent` BASE
    /// with `AppliedExposureRecently`; P6-11au): `Base`, turned into
    /// [`Poe1Quantity::RegenPercent`] by [`exposure`] when an exposure is on
    /// the enemy.
    RegenIfExposureApplied(Pool),
    /// "Allocates <notable>" (PoB's `GrantedPassive`, P3-8n): `Base`, the
    /// node's hash; `collect` adds the node's stats.
    GrantedPassive,
    /// "#% increased Effect of Buffs granted by your Golems" (PoB's
    /// `BuffEffect` INC tagged with the `Golem` skill type; P3-close):
    /// `Increased`, read by [`skill_modifiers::Totals`] for golem skills'
    /// buffs.
    GolemBuffEffect,
    /// The effect of herald skills' buffs (PoB's `BuffEffect` INC tagged
    /// `SkillType.Herald`; P6-11at), read by [`skill_modifiers`].
    HeraldBuffEffect,
    /// "#% increased Effect of your Curses" (PoB's `CurseEffect` INC;
    /// P3-close): `Increased`, read by [`skill_modifiers::Totals`] for
    /// curse gems' effect on the enemy.
    CurseEffect,
    /// A skill's exposure on the enemy (PoB's enemy `<Type>Exposure` BASE
    /// from a debuff, for example Frost Bomb's; P3-close): `Base`, read by
    /// [`exposure`], which keeps the strongest.
    SkillExposure(DamageType),
    /// Damage of the first type gained as extra damage of the second (PoB's
    /// `<From>DamageGainAs<To>` BASE; P3-close): `Base`, percent, read by
    /// [`hit`]'s conversion table.
    GainAs(DamageType, DamageType),
    /// A chance to inflict a damaging ailment on hit, percent (PoB's
    /// `EnemyIgniteChance`, `BleedChance`, `PoisonChance`; Phase 6):
    /// `Base`, read by [`ailment`].
    AilmentChance(ailment::Ailment),
    /// The duration of the ailments you inflict (PoB's `EnemyIgniteDuration`,
    /// `EnemyAilmentDuration`, `EnemyElementalAilmentDuration`,
    /// `DamagingAilmentDuration`, one quantity whose conditions say which
    /// ailments; Phase 6): `Increased` and `More`.
    AilmentDuration,
    /// Ailments deal their damage faster (PoB's `IgniteBurnFaster`,
    /// `BleedFaster`, `PoisonFaster`, by conditions; Phase 6): `Increased`.
    AilmentFaster,
    /// Ailments deal their damage slower (PoB's `IgniteBurnSlower` and kin;
    /// Phase 6): `Increased`.
    AilmentSlower,
    /// A chance to impale on hit, percent (PoB's `ImpaleChance`; Phase 6):
    /// `Base`, read by [`ailment::impale`].
    ImpaleChance,
    /// The effect of impales (PoB's `ImpaleEffect`; Phase 6): `Increased`
    /// and `More`.
    ImpaleEffect,
    /// Impales on the enemy at most, added to the character template's
    /// (PoB's `ImpaleStacksMax` BASE; Phase 6).
    ImpaleStacksMax,
    /// "Cannot cause Bleeding" and kin (PoB's `CannotBleed`, `CannotPoison`,
    /// `CannotIgnite`; Phase 6): a `Flag` read by [`ailment`].
    CannotInflict(ailment::Ailment),
    /// The duration of ailments on the enemy (PoB's enemy
    /// `SelfIgniteDuration`, `SelfAilmentDuration`,
    /// `SelfElementalAilmentDuration`, by conditions; curses such as
    /// Elemental Weakness; Phase 6): `Increased`, read by [`ailment`].
    EnemyAilmentDuration,
    /// "Ailments never count as being from Critical Strikes" (PoB's
    /// `AilmentsAreNeverFromCrit`, Elemental Overload; Phase 6): a `Flag`
    /// read by [`ailment`].
    AilmentsNeverFromCrit,
    /// The effect of the ailments you inflict (PoB's `AilmentEffect`, by
    /// conditions; Phase 6): `Increased` and `More`.
    AilmentEffect,
    /// Intimidating Cry's terms for its exerted attacks' uptime
    /// ([`gems::WarcryValue`]; close-out): `Base`, read by [`hit`].
    IntimidatingCry(gems::WarcryValue),
}

/// A pool: one a skill reserves (life or mana), or one that regenerates
/// (P6-10b: energy shield too).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Pool {
    /// Life.
    Life,
    /// Mana.
    Mana,
    /// Energy shield (regeneration only; P6-10b).
    EnergyShield,
}

/// A recovery on block (P6-11ak; PoB's text, `ModParser.lua` 382 to 4717).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BlockGain {
    /// Life, flat (`LifeOnBlock` BASE).
    Life,
    /// Life, a percent of maximum life (`LifeOnBlock` BASE 1 with
    /// `PercentStat` `Life`: `ceil(Life x percent / 100)` per mod).
    LifePercent,
    /// Energy shield on blocking spell damage, flat
    /// (`EnergyShieldOnSpellBlock` BASE).
    EnergyShieldSpell,
    /// Energy shield, a percent of armour (`EnergyShieldOnBlock` BASE 1
    /// with `PercentStat` `Armour`).
    EnergyShieldPercentOfArmour,
}

impl BlockGain {
    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Life => "life",
            Self::LifePercent => "life-percent",
            Self::EnergyShieldSpell => "energy-shield-spell",
            Self::EnergyShieldPercentOfArmour => "energy-shield-percent-of-armour",
        }
    }
}

/// When a damage taken modifier applies (P6-10b part 10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TakenWhen {
    /// Always (PoB's `DamageTaken`).
    Always,
    /// From hits (PoB's `DamageTakenWhenHit`).
    Hit,
    /// Over time (PoB's `DamageTakenOverTime`).
    OverTime,
    /// From attacks (PoB's `AttackDamageTaken` INC and MORE,
    /// `DamageTakenFromAttacks` BASE; P6-10b part 12).
    Attack,
    /// From projectile attacks (PoB's `<Type>DamageTakenFromProjectileAttacks`
    /// BASE).
    ProjectileAttack,
    /// From spells (PoB's `SpellDamageTaken` INC and MORE,
    /// `DamageTakenFromSpells` BASE).
    Spell,
    /// From spell projectiles (PoB's `DamageTakenFromSpellProjectiles` BASE).
    SpellProjectile,
}

impl TakenWhen {
    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Always => "always",
            Self::Hit => "hit",
            Self::OverTime => "over-time",
            Self::Attack => "attack",
            Self::ProjectileAttack => "projectile-attack",
            Self::Spell => "spell",
            Self::SpellProjectile => "spell-projectile",
        }
    }
}

/// What damage a damage taken modifier reads (P6-10b part 10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TakenOf {
    /// All damage.
    All,
    /// One type's.
    Type(DamageType),
    /// Fire, cold, and lightning.
    Elemental,
}

impl TakenOf {
    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Type(t) => t.name(),
            Self::Elemental => "elemental",
        }
    }
}

/// What damage a leech line reads (P6-10b part 9): all of it, one type's,
/// or the elemental types'.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LeechSource {
    /// Any damage (PoB's `Damage<Pool>Leech`, and `DamageLeech`).
    Any,
    /// One type's damage (PoB's `<Type>Damage<Pool>Leech`).
    Type(DamageType),
    /// Fire, cold, and lightning damage (PoB's `ElementalDamage<Pool>Leech`).
    Elemental,
}

impl LeechSource {
    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Any => "any",
            Self::Type(t) => t.name(),
            Self::Elemental => "elemental",
        }
    }
}

impl Pool {
    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Life => "life",
            Self::Mana => "mana",
            Self::EnergyShield => "energy-shield",
        }
    }
}

/// How a skill reserves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReservationKind {
    /// An amount.
    Flat,
    /// A percentage of the pool.
    Percent,
}

impl ReservationKind {
    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Flat => "flat",
            Self::Percent => "percent",
        }
    }
}

/// A hand that holds a weapon (Phase 6, dual wielding).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Hand {
    /// The main hand (`Weapon 1`).
    Main,
    /// The off hand (`Weapon 2`).
    Off,
}

impl Hand {
    /// The quantity of a value of the weapon in this hand.
    #[must_use]
    pub const fn weapon(self, value: WeaponValue) -> crate::quantity::Quantity {
        crate::quantity::Quantity::Poe1(match self {
            Self::Main => Poe1Quantity::Weapon(value),
            Self::Off => Poe1Quantity::OffHandWeapon(value),
        })
    }

    /// PoB's slot name.
    #[must_use]
    pub const fn slot(self) -> &'static str {
        match self {
            Self::Main => "Weapon 1",
            Self::Off => "Weapon 2",
        }
    }
}

/// A value of the weapon in the main hand (`Classes/Item.lua`, the weapon
/// branch of `BuildModListForSlotNum`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WeaponValue {
    /// Damage of a type: `Base` is the base's (physical) and the local
    /// flat lines ("Adds # to # Physical Damage"), `Increased` the local
    /// increases.
    Damage(DamageType, Bound),
    /// Attack time, milliseconds (the base's).
    AttackTime,
    /// A weapon's base attack rate, uses per second, when it has no attack
    /// time in milliseconds (a minion's weapon from its monster data: PoB's
    /// `AttackRate = 1 / attackTime`, `CalcActiveSkill.lua` 747; P6-7c):
    /// `Base`, read by [`hit`] instead of [`Self::AttackTime`].
    AttackRate,
    /// Local attack speed (`Increased`).
    AttackSpeed,
    /// Critical strike chance, percent: `Base` is the base's and the local
    /// flat lines, `Increased` the local increases.
    CritChance,
    /// The weapon's quality, percent.
    Quality,
    /// The weapon deals no physical damage when not 0 (the game's
    /// `local_weapon_no_physical_damage`, which the canonical "#% increased
    /// Physical Damage" line carries as 0).
    NoPhysicalDamage,
    /// "Quality does not increase Physical Damage" (PoB's
    /// `AlternateQualityWeapon`, P3-8n): a `Flag`; the weapon's physical
    /// damage then ignores its quality.
    AlternateQuality,
    /// The weapon's local "+# to Accuracy Rating" (`Base`; Phase 6): PoB's
    /// `Accuracy` BASE with the hand's `MainHandAttack` or `OffHandAttack`
    /// condition (`Classes/Item.lua` 2282 to 2288), so each hand's pass
    /// reads only its own weapon's ([`hit`]).
    Accuracy,
    /// "Grants #% increased Elemental Damage per 2% Quality" (P3-8n):
    /// `Base`; `collect` turns it into the character's elemental damage
    /// times `floor(quality / 2)`.
    ElementalDamagePerQuality,
}

impl core::fmt::Display for WeaponValue {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Damage(t, b) => write!(f, "damage.{}.{}", t.name(), b.name()),
            Self::AttackTime => f.write_str("attack-time"),
            Self::AttackRate => f.write_str("attack-rate"),
            Self::AttackSpeed => f.write_str("attack-speed"),
            Self::CritChance => f.write_str("crit-chance"),
            Self::Quality => f.write_str("quality"),
            Self::NoPhysicalDamage => f.write_str("no-physical-damage"),
            Self::AlternateQuality => f.write_str("alternate-quality"),
            Self::ElementalDamagePerQuality => f.write_str("elemental-damage-per-quality"),
            Self::Accuracy => f.write_str("accuracy"),
        }
    }
}

impl core::fmt::Display for Poe1Quantity {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::SpellSuppressionChance => f.write_str("spell-suppression-chance"),
            Self::SpellBlockChance => f.write_str("spell-block-chance"),
            Self::MaximumSpellBlockChance => f.write_str("maximum-spell-block-chance"),
            Self::Piece(slot, value) => write!(f, "piece.{}.{value}", slot.name()),
            Self::SkillBaseDamage(t, b) => {
                write!(f, "skill-base-damage.{}.{}", t.name(), b.name())
            }
            Self::SkillCritChance => f.write_str("skill-crit-chance"),
            Self::DamageEffectiveness => f.write_str("damage-effectiveness"),
            Self::BaseDamageMultiplier => f.write_str("base-damage-multiplier"),
            Self::CastTime => f.write_str("cast-time"),
            Self::Weapon(value) => write!(f, "weapon.{value}"),
            Self::OffHandWeapon(value) => write!(f, "off-hand-weapon.{value}"),
            Self::SkillConversion(from, to) => {
                write!(f, "skill-conversion.{}.{}", from.name(), to.name())
            }
            Self::AttackSpeedMultiplier => f.write_str("attack-speed-multiplier"),
            Self::MaximumPhysicalDamageReduction => {
                f.write_str("maximum-physical-damage-reduction")
            }
            Self::EnemyDamageTaken => f.write_str("enemy-damage-taken"),
            Self::EnemyEvasion => f.write_str("enemy-evasion"),
            Self::EnemyResistance(t) => write!(f, "enemy-resistance.{}", t.name()),
            Self::ExposureChance(t) => write!(f, "exposure-chance.{}", t.name()),
            Self::ExtraExposure(None) => f.write_str("extra-exposure"),
            Self::ExtraExposure(Some(t)) => write!(f, "extra-exposure.{}", t.name()),
            Self::ShockMinimum => f.write_str("shock-minimum"),
            Self::ShockEffect => f.write_str("shock-effect"),
            Self::Onslaught => f.write_str("onslaught"),
            Self::FlaskEffect => f.write_str("flask-effect"),
            Self::MagicUtilityFlaskEffect => f.write_str("magic-utility-flask-effect"),
            Self::LocalFlaskEffect => f.write_str("local-flask-effect"),
            Self::Charge(kind, value) => write!(f, "charge.{}.{}", kind.name(), value.name()),
            Self::GemLevel(stat) => write!(f, "gem-level.{stat}"),
            Self::SocketedCostMultiplier(slot) => write!(f, "socketed-cost-multiplier.{slot:?}"),
            Self::SocketedGemLevel(stat, slot) => {
                write!(f, "socketed-gem-level.{}.{stat}", slot.name())
            }
            Self::ItemSupport(stat, slot) => {
                write!(f, "item-support.{}.{stat}", slot.name())
            }
            Self::Minion(value) => write!(f, "minion.{}", value.name()),
            Self::GolemMinion(value) => write!(f, "golem-minion.{}", value.name()),
            Self::CurseLimit => f.write_str("curse-limit"),
            Self::EnemyProjectileDamageTaken => f.write_str("enemy-projectile-damage-taken"),
            Self::IgnorePdrChance => f.write_str("ignore-pdr-chance"),
            Self::Regen(p) => write!(f, "regen.{}", p.name()),
            Self::RegenPercent(p) => write!(f, "regen-percent.{}", p.name()),
            Self::RecoveryRate(p) => write!(f, "recovery-rate.{}", p.name()),
            Self::LeechRateMax(p) => write!(f, "leech-rate-max.{}", p.name()),
            Self::LeechInstanceMax(p) => write!(f, "leech-instance-max.{}", p.name()),
            Self::DamageLeech(p, source) => {
                write!(f, "damage-leech.{}.{}", p.name(), source.name())
            }
            Self::LeechRate(p) => write!(f, "leech-rate.{}", p.name()),
            Self::InstantLeech(p) => write!(f, "instant-leech.{}", p.name()),
            Self::CannotLeech(p) => write!(f, "cannot-leech.{}", p.name()),
            Self::EnemyAccuracy => f.write_str("enemy-accuracy"),
            Self::CannotEvade => f.write_str("cannot-evade"),
            Self::EnemyHitReason(stat) => write!(f, "enemy-hit-reason.{stat}"),
            Self::PoolReason(stat) => write!(f, "pool-reason.{stat}"),
            Self::DotTakenReason(stat) => write!(f, "dot-taken-reason.{stat}"),
            Self::EnemyCritChance => f.write_str("enemy-crit-chance"),
            Self::CritExtraDamageReduction => f.write_str("crit-extra-damage-reduction"),
            Self::EnemyDamage => f.write_str("enemy-damage"),
            Self::DamageTakenAs(from, to, hits) => write!(
                f,
                "damage-taken-as.{}.{}{}",
                from.name(),
                to.name(),
                if *hits { ".hits" } else { "" }
            ),
            Self::ArmourAppliesTo(t) => write!(f, "armour-applies-to.{}", t.name()),
            Self::SpellSuppressionEffect => f.write_str("spell-suppression-effect"),
            Self::MaximumFortification => f.write_str("maximum-fortification"),
            Self::LowLifePercentage => f.write_str("low-life-percentage"),
            Self::GuardAbsorbRate => f.write_str("guard-absorb-rate"),
            Self::GuardAbsorbLimit => f.write_str("guard-absorb-limit"),
            Self::GuardAbsorbPerArmour => f.write_str("guard-absorb-per-armour"),
            Self::GuardAbsorbMax => f.write_str("guard-absorb-max"),
            Self::ManaBeforeLife(o) => write!(f, "mana-before-life.{}", o.name()),
            Self::BlockEffect => f.write_str("block-effect"),
            Self::AvoidProjectiles => f.write_str("avoid-projectiles"),
            Self::EhpReason(stat) => write!(f, "ehp-reason.{stat}"),
            Self::SelfDamageTaken(w, o) => write!(f, "self-damage-taken.{}.{}", w.name(), o.name()),
            Self::EnergyShieldRecharge => f.write_str("energy-shield-recharge"),
            Self::EnergyShieldRechargeFaster => f.write_str("energy-shield-recharge-faster"),
            Self::ZealotsOath => f.write_str("zealots-oath"),
            Self::RegenReason(p, stat) => write!(f, "regen-reason.{}.{stat}", p.name()),
            Self::RegenPerMinute(p) => write!(f, "regen-per-minute.{}", p.name()),
            Self::RegenPercentPerMinute(p) => write!(f, "regen-percent-per-minute.{}", p.name()),
            Self::RegenIncPerSpellBlock(p) => write!(f, "regen-inc-per-spell-block.{}", p.name()),
            Self::RegenPerFireResist(p) => write!(f, "regen-per-fire-resist.{}", p.name()),
            Self::ArcaneSurgeManaRegen => f.write_str("arcane-surge-mana-regen"),
            Self::ConsecratedWhileStationary => f.write_str("consecrated-while-stationary"),
            Self::EnemyPhysicalDamageReduction => f.write_str("enemy-physical-damage-reduction"),
            Self::BlockPer5(true) => f.write_str("spell-block-per-5-block"),
            Self::BlockPer5(false) => f.write_str("block-per-5-shield-block"),
            Self::MinionToYou(value, percent) => {
                write!(f, "minion-to-you.{}.{percent}", value.name())
            }
            Self::Reservation(pool, kind) => {
                write!(f, "reservation.{}.{}", pool.name(), kind.name())
            }
            Self::ReservationEfficiency(None) => f.write_str("reservation-efficiency"),
            Self::ReservationEfficiency(Some(pool)) => {
                write!(f, "reservation-efficiency.{}", pool.name())
            }
            Self::ResistConvert(max, to) => write!(
                f,
                "resist-convert.{}.{}",
                if *max { "maximum" } else { "resistance" },
                to.name()
            ),
            Self::RepeatCount => f.write_str("repeat-count"),
            Self::RepeatDamage(n) => write!(f, "repeat-damage.{n}"),
            Self::RepeatPerRepeatDamage => f.write_str("repeat-per-repeat-damage"),
            Self::RepeatFinalDoubleDamageChance => f.write_str("repeat-final-double-damage-chance"),
            Self::Repeats => f.write_str("repeats"),
            Self::BlockGain(g) => write!(f, "block-gain.{}", g.name()),
            Self::SuppressionLucky => f.write_str("suppression-lucky"),
            Self::ArmourEsRecoveryCap => f.write_str("armour-es-recovery-cap"),
            Self::EsProtectsMana => f.write_str("es-protects-mana"),
            Self::CanWither => f.write_str("can-wither"),
            Self::TakenFromStoneGolem => f.write_str("taken-from-stone-golem"),
            Self::WitherEffect => f.write_str("wither-effect"),
            Self::PseudoRecoup => f.write_str("pseudo-recoup"),
            Self::ManaRecoup => f.write_str("mana-recoup"),
            Self::LifeRecoup => f.write_str("life-recoup"),
            Self::Devotion => f.write_str("devotion"),
            Self::DevotionReader(i) => write!(f, "devotion-reader.{i}"),
            Self::ReserveLifeInsteadOfMana => f.write_str("reserve-life-instead-of-mana"),
            Self::EsRechargeAppliesToLife => f.write_str("es-recharge-applies-to-life"),
            Self::RepeatAverageDamage => f.write_str("repeat-average-damage"),
            Self::HeraldManaReservationEfficiency => {
                f.write_str("herald-mana-reservation-efficiency")
            }
            Self::Reserved(None) => f.write_str("reserved"),
            Self::Reserved(Some(pool)) => write!(f, "reserved.{}", pool.name()),
            Self::NonCurseAuraEffect => f.write_str("non-curse-aura-effect"),
            Self::SkillAuraEffectOnSelf => f.write_str("skill-aura-effect-on-self"),
            Self::CullingStrike => f.write_str("culling-strike"),
            Self::SlotIncreased(s, d) => write!(f, "slot-increased.{}.{}", s.row(), d.name()),
            Self::CannotBeEvaded => f.write_str("cannot-be-evaded"),
            Self::NeverCrit => f.write_str("never-crit"),
            Self::NoCritMultiplier => f.write_str("no-crit-multiplier"),
            Self::DealNo(t) => write!(f, "deal-no.{}", t.name()),
            Self::EnemyTypeDamageTaken(t) => write!(f, "enemy-damage-taken.{}", t.name()),
            Self::ChillDamageTaken(None) => f.write_str("chill-damage-taken"),
            Self::ChillDamageTaken(Some(t)) => write!(f, "chill-damage-taken.{}", t.name()),
            Self::ChillEffect => f.write_str("chill-effect"),
            Self::SkillChill => f.write_str("skill-chill"),
            Self::ActionSpeed => f.write_str("action-speed"),
            Self::MovementSpeed => f.write_str("movement-speed"),
            Self::MovementReason(stat) => write!(f, "movement-reason.{stat}"),
            Self::ManaCost => f.write_str("mana-cost"),
            Self::Cost => f.write_str("cost"),
            Self::ManaCostNoMult => f.write_str("mana-cost-no-mult"),
            Self::ManaCostChannelling(true) => f.write_str("mana-cost-channelling"),
            Self::ManaCostChannelling(false) => f.write_str("mana-cost-non-channelling"),
            Self::ManaCostEfficiency => f.write_str("mana-cost-efficiency"),
            Self::CostEfficiency => f.write_str("cost-efficiency"),
            Self::CostReason(stat) => write!(f, "cost-reason.{stat}"),
            Self::LifeCost => f.write_str("life-cost"),
            Self::EnergyShieldCost => f.write_str("energy-shield-cost"),
            Self::LifeCostEfficiency => f.write_str("life-cost-efficiency"),
            Self::EnergyShieldCostEfficiency => f.write_str("energy-shield-cost-efficiency"),
            Self::CostLifeInsteadOfMana => f.write_str("cost-life-instead-of-mana"),
            Self::CostEnergyShieldInsteadOfManaOrLife => {
                f.write_str("cost-energy-shield-instead-of-mana-or-life")
            }
            Self::HybridLifeCost => f.write_str("hybrid-life-cost"),
            Self::ManaCostAsLifeCost => f.write_str("mana-cost-as-life-cost"),
            Self::ManaCostAsEnergyShieldCost => f.write_str("mana-cost-as-energy-shield-cost"),
            Self::HitReason(stat) => write!(f, "hit-reason.{stat}"),
            Self::DamageReduction(t) => write!(f, "damage-reduction.{}", t.name()),
            Self::ElementalDamageReduction => f.write_str("elemental-damage-reduction"),
            Self::ReductionReason(stat) => write!(f, "reduction-reason.{stat}"),
            Self::DoubleDamageChance => f.write_str("double-damage-chance"),
            Self::Conversion(from, to) => {
                write!(f, "conversion.{}.{}", from.name(), to.name())
            }
            Self::Stack(kind, value) => write!(f, "stack.{}.{}", kind.name(), value.name()),
            Self::ArcaneSurgeCastSpeed => f.write_str("arcane-surge-cast-speed"),
            Self::CanGainConvergence => f.write_str("can-gain-convergence"),
            Self::ConsecratedGroundEffect => f.write_str("consecrated-ground-effect"),
            Self::ConsecratedGroundDamageTaken => f.write_str("consecrated-ground-damage-taken"),
            Self::AilmentElementalResistance => f.write_str("ailment-elemental-resistance"),
            Self::DamageVsChilled => f.write_str("damage-vs-chilled"),
            Self::ChillSecondsDamageTaken => f.write_str("chill-seconds-damage-taken"),
            Self::ExposureOnHit(t) => write!(f, "exposure-on-hit.{}", t.name()),
            Self::RegenIfExposureApplied(p) => {
                write!(f, "regen-if-exposure-applied.{}", p.name())
            }
            Self::GrantedPassive => f.write_str("granted-passive"),
            Self::GolemBuffEffect => f.write_str("golem-buff-effect"),
            Self::HeraldBuffEffect => f.write_str("herald-buff-effect"),
            Self::CurseEffect => f.write_str("curse-effect"),
            Self::SkillExposure(t) => write!(f, "skill-exposure.{}", t.name()),
            Self::GainAs(from, to) => write!(f, "gain-as.{}.{}", from.name(), to.name()),
            Self::IntimidatingCry(v) => write!(f, "intimidating-cry.{}", v.name()),
            Self::AilmentChance(a) => write!(f, "ailment-chance.{}", a.name()),
            Self::AilmentDuration => f.write_str("ailment-duration"),
            Self::AilmentFaster => f.write_str("ailment-faster"),
            Self::AilmentSlower => f.write_str("ailment-slower"),
            Self::AilmentEffect => f.write_str("ailment-effect"),
            Self::AilmentsNeverFromCrit => f.write_str("ailments-never-from-crit"),
            Self::EnemyAilmentDuration => f.write_str("enemy-ailment-duration"),
            Self::CannotInflict(a) => write!(f, "cannot-inflict.{}", a.name()),
            Self::ImpaleChance => f.write_str("impale-chance"),
            Self::ImpaleEffect => f.write_str("impale-effect"),
            Self::ImpaleStacksMax => f.write_str("impale-stacks-max"),
        }
    }
}

/// An item slot whose armour piece has local defences, in the order PoB
/// sums them (`Modules/CalcDefence.lua`: Helmet, Gloves, Boots, Body
/// Armour, Weapon 2; its Weapon 3 is a minion's Aegis item, not the
/// player's).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ArmourSlot {
    /// Helmet.
    Helmet,
    /// Gloves.
    Gloves,
    /// Boots.
    Boots,
    /// Body armour.
    BodyArmour,
    /// The off hand (a shield).
    Weapon2,
}

impl ArmourSlot {
    /// Every slot, in PoB's order.
    pub const ALL: [Self; 5] = [
        Self::Helmet,
        Self::Gloves,
        Self::Boots,
        Self::BodyArmour,
        Self::Weapon2,
    ];

    /// Stable name: PoB's slot name lowercased, spaces as `-`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Helmet => "helmet",
            Self::Gloves => "gloves",
            Self::Boots => "boots",
            Self::BodyArmour => "body-armour",
            Self::Weapon2 => "weapon-2",
        }
    }

    /// The breakdown row's node name (`slice-fields.json` revision 2:
    /// `step.slot.<source>`); a static table, so `Term::Step` holds it.
    #[must_use]
    pub const fn row(self) -> &'static str {
        match self {
            Self::Helmet => "slot.helmet",
            Self::Gloves => "slot.gloves",
            Self::Boots => "slot.boots",
            Self::BodyArmour => "slot.body-armour",
            Self::Weapon2 => "slot.weapon-2",
        }
    }

    /// The slot of a build's slot name; a swap slot in use counts as its
    /// main slot (PoB removes ` Swap` from the names of the weapon set in
    /// use).
    #[must_use]
    pub fn of(slot: &str) -> Option<Self> {
        match slot.strip_suffix(" Swap").unwrap_or(slot) {
            "Helmet" => Some(Self::Helmet),
            "Gloves" => Some(Self::Gloves),
            "Boots" => Some(Self::Boots),
            "Body Armour" => Some(Self::BodyArmour),
            "Weapon 2" => Some(Self::Weapon2),
            _ => None,
        }
    }
}

/// A primary defence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Defence {
    /// Armour.
    Armour,
    /// Evasion rating.
    Evasion,
    /// Energy shield.
    EnergyShield,
}

impl Defence {
    /// Every defence.
    pub const ALL: [Self; 3] = [Self::Armour, Self::Evasion, Self::EnergyShield];

    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Armour => "armour",
            Self::Evasion => "evasion",
            Self::EnergyShield => "energy-shield",
        }
    }
}

/// A value of an armour piece (`Classes/Item.lua`, the armour branch of
/// `BuildModListForSlotNum`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PieceValue {
    /// The piece is equipped (a `Flag`).
    Equipped,
    /// A defence: `Base` is the base's minimum and the local flat lines,
    /// `Increased` the local increases.
    Defence(Defence),
    /// The base's range of a defence (maximum less minimum).
    Variance(Defence),
    /// Where in the range the piece rolled, 0 to 1 (the item's
    /// `<Defence>BasePercentile`; else derived from [`PieceValue::Shown`];
    /// else 1).
    Percentile(Defence),
    /// The defence the item shows (`Armour: 1198`, a game copy's or PoB's
    /// written total), read when it has no `<Defence>BasePercentile`.
    Shown(Defence),
    /// The piece's quality, percent.
    Quality,
    /// A shield's chance to block: `Base` is the base's and the local
    /// flat lines, `Increased` the local increases.
    Block,
}

impl core::fmt::Display for PieceValue {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Equipped => f.write_str("equipped"),
            Self::Defence(d) => f.write_str(d.name()),
            Self::Variance(d) => write!(f, "variance.{}", d.name()),
            Self::Percentile(d) => write!(f, "percentile.{}", d.name()),
            Self::Shown(d) => write!(f, "shown.{}", d.name()),
            Self::Quality => f.write_str("quality"),
            Self::Block => f.write_str("block"),
        }
    }
}

/// A field only Path of Exile has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Poe1Field {
    /// Chance to suppress spell damage.
    SpellSuppressionChance,
    /// Chance to block spell damage.
    SpellBlockChance,
    /// The main skill's impale damage per second (PoB's `ImpaleDPS`; Phase
    /// 6). Present only when the skill impales.
    ImpaleDps,
    /// Life regenerated per second (PoB's `LifeRegen`; P6-10b).
    LifeRegen,
    /// Mana regenerated per second (PoB's `ManaRegen`; P6-10b).
    ManaRegen,
    /// Energy shield regenerated per second (PoB's `EnergyShieldRegen`;
    /// P6-10b).
    EnergyShieldRegen,
    /// Energy shield recharged per second (PoB's `EnergyShieldRecharge`;
    /// P6-10b).
    EnergyShieldRecharge,
    /// Seconds before energy shield starts to recharge (PoB's
    /// `EnergyShieldRechargeDelay`; P6-10b).
    EnergyShieldRechargeDelay,
    /// The movement speed modifier (PoB's `MovementSpeedMod`; P6-10b).
    MovementSpeedMod,
    /// The movement speed modifier with action speed (PoB's
    /// `EffectiveMovementSpeedMod`; P6-10b).
    EffectiveMovementSpeedMod,
    /// The main skill's mana cost (PoB's `ManaCost`; P6-10b).
    ManaCost,
    /// The main skill's mana cost per second (PoB's `ManaPerSecondCost`;
    /// P6-10b part 6). Present only with a cost and a computed speed.
    ManaPerSecondCost,
    /// The main skill's combined damage per second (PoB's `CombinedDPS`;
    /// P6-12): the hit's, impale's, and the damage over time's, culled.
    CombinedDps,
    /// A damage type's base damage reduction, percent (PoB's
    /// `Base<Type>DamageReduction`; P6-10b part 5).
    BaseDamageReduction(DamageType),
    /// The build's full damage per second (PoB's `FullDPS`; P6-12 part 2):
    /// the skills included in it, each computed as the main skill.
    FullDps,
    /// The largest total recovery per second from a pool's leech (PoB's
    /// `Max<Pool>LeechRate`; P6-10b part 7). Present with the pool.
    MaxLeechRate(Pool),
    /// The largest recovery of one leech of a pool (PoB's
    /// `Max<Pool>LeechInstance`; P6-10b part 7). Present with the pool.
    MaxLeechInstance(Pool),
    /// Life's leech rate cap in percent of life (PoB's
    /// `MaxLifeLeechRatePercent`; P6-10b part 7). Always present.
    MaxLifeLeechRatePercent,
    /// The main skill's life cost (PoB's `LifeCost`; P6-10b part 8).
    LifeCost,
    /// The main skill's energy shield cost (PoB's `ESCost`; P6-10b part 8).
    EnergyShieldCost,
    /// The main skill's life cost per second (PoB's `LifePerSecondCost`;
    /// P6-10b part 8). Present only with a cost and a computed speed.
    LifePerSecondCost,
    /// The main skill's energy shield cost per second (PoB's
    /// `ESPerSecondCost`; P6-10b part 8).
    EnergyShieldPerSecondCost,
    /// The main skill's leech of a pool per second (PoB's `<Pool>LeechRate`;
    /// P6-10b part 9). Present when the skill's hit leeches the pool.
    LeechRate(Pool),
    /// The multiplier of a type's damage taken over time, with resistance
    /// and base reduction (PoB's `<Type>TakenDotMult`; P6-10b part 10).
    TakenDotMult(DamageType),
    /// The chance to evade the enemy's attacks (PoB's `EvadeChance`; P6-10b
    /// part 11). Present with evasion.
    EvadeChance,
    /// The chance to evade melee attacks (PoB's `MeleeEvadeChance`).
    MeleeEvadeChance,
    /// The chance to evade projectile attacks (PoB's
    /// `ProjectileEvadeChance`).
    ProjectileEvadeChance,
    /// The multiplier of the enemy's hit of a type, after every mitigation
    /// (PoB's final `<Type>TakenHitMult`; P6-10b part 12).
    TakenHitMult(DamageType),
    /// The damage reduction against the enemy's hit of a type, armour
    /// included (PoB's `<Type>DamageReduction`; P6-10b part 12).
    DamageReduction(DamageType),
    /// The largest enemy hit of a type the character survives (PoB's
    /// `<Type>MaximumHitTaken`; P6-10b part 13).
    MaximumHitTaken(DamageType),
    /// The second smallest of the five (PoB's
    /// `SecondMinimalMaximumHitTaken`; P6-10b part 13).
    SecondMinimalMaximumHitTaken,
    /// The enemy hits it takes to die (PoB's `NumberOfDamagingHits`;
    /// P6-10b part 14).
    NumberOfDamagingHits,
    /// The same with block, suppression, and avoidance (PoB's
    /// `NumberOfMitigatedDamagingHits`).
    NumberOfMitigatedDamagingHits,
    /// The same with the chance not to be hit (PoB's `TotalNumberOfHits`).
    TotalNumberOfHits,
    /// The chance a hit deals damage after block, suppression, and
    /// avoidance (PoB's `ConfiguredDamageChance`).
    ConfiguredDamageChance,
    /// The chance not to be hit (PoB's `ConfiguredNotHitChance`).
    ConfiguredNotHitChance,
    /// The effective health pool (PoB's `TotalEHP`).
    TotalEhp,
}

impl Poe1Field {
    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::SpellSuppressionChance => "spell-suppression-chance",
            Self::SpellBlockChance => "spell-block-chance",
            Self::ImpaleDps => "impale-dps",
            Self::LifeRegen => "life-regen",
            Self::ManaRegen => "mana-regen",
            Self::EnergyShieldRegen => "energy-shield-regen",
            Self::EnergyShieldRecharge => "energy-shield-recharge",
            Self::EnergyShieldRechargeDelay => "energy-shield-recharge-delay",
            Self::MovementSpeedMod => "movement-speed",
            Self::EffectiveMovementSpeedMod => "effective-movement-speed",
            Self::ManaCost => "mana-cost",
            Self::ManaPerSecondCost => "mana-per-second-cost",
            Self::FullDps => "full-dps",
            Self::MaxLeechRate(p) => match p {
                Pool::Life => "max-life-leech-rate",
                Pool::Mana => "max-mana-leech-rate",
                Pool::EnergyShield => "max-energy-shield-leech-rate",
            },
            Self::MaxLeechInstance(p) => match p {
                Pool::Life => "max-life-leech-instance",
                Pool::Mana => "max-mana-leech-instance",
                Pool::EnergyShield => "max-energy-shield-leech-instance",
            },
            Self::MaxLifeLeechRatePercent => "max-life-leech-rate-percent",
            Self::LifeCost => "life-cost",
            Self::EnergyShieldCost => "energy-shield-cost",
            Self::LifePerSecondCost => "life-per-second-cost",
            Self::EnergyShieldPerSecondCost => "energy-shield-per-second-cost",
            Self::EvadeChance => "evade-chance",
            Self::TakenHitMult(t) => match t {
                DamageType::Physical => "physical-taken-hit-mult",
                DamageType::Fire => "fire-taken-hit-mult",
                DamageType::Cold => "cold-taken-hit-mult",
                DamageType::Lightning => "lightning-taken-hit-mult",
                DamageType::Chaos => "chaos-taken-hit-mult",
            },
            Self::MaximumHitTaken(t) => match t {
                DamageType::Physical => "physical-maximum-hit-taken",
                DamageType::Fire => "fire-maximum-hit-taken",
                DamageType::Cold => "cold-maximum-hit-taken",
                DamageType::Lightning => "lightning-maximum-hit-taken",
                DamageType::Chaos => "chaos-maximum-hit-taken",
            },
            Self::SecondMinimalMaximumHitTaken => "second-minimal-maximum-hit-taken",
            Self::NumberOfDamagingHits => "number-of-damaging-hits",
            Self::NumberOfMitigatedDamagingHits => "number-of-mitigated-damaging-hits",
            Self::TotalNumberOfHits => "total-number-of-hits",
            Self::ConfiguredDamageChance => "configured-damage-chance",
            Self::ConfiguredNotHitChance => "configured-not-hit-chance",
            Self::TotalEhp => "total-ehp",
            Self::DamageReduction(t) => match t {
                DamageType::Physical => "physical-damage-reduction",
                DamageType::Fire => "fire-damage-reduction",
                DamageType::Cold => "cold-damage-reduction",
                DamageType::Lightning => "lightning-damage-reduction",
                DamageType::Chaos => "chaos-damage-reduction",
            },
            Self::MeleeEvadeChance => "melee-evade-chance",
            Self::ProjectileEvadeChance => "projectile-evade-chance",
            Self::TakenDotMult(t) => match t {
                DamageType::Physical => "physical-taken-dot-mult",
                DamageType::Fire => "fire-taken-dot-mult",
                DamageType::Cold => "cold-taken-dot-mult",
                DamageType::Lightning => "lightning-taken-dot-mult",
                DamageType::Chaos => "chaos-taken-dot-mult",
            },
            Self::LeechRate(p) => match p {
                Pool::Life => "life-leech-rate",
                Pool::Mana => "mana-leech-rate",
                Pool::EnergyShield => "energy-shield-leech-rate",
            },
            Self::CombinedDps => "combined-dps",
            Self::BaseDamageReduction(t) => match t {
                DamageType::Physical => "base-physical-damage-reduction",
                DamageType::Fire => "base-fire-damage-reduction",
                DamageType::Cold => "base-cold-damage-reduction",
                DamageType::Lightning => "base-lightning-damage-reduction",
                DamageType::Chaos => "base-chaos-damage-reduction",
            },
        }
    }
}

/// The Path of Exile slice: the 24 fields of
/// `docs/phase-3/slice-fields.json` (revisions 1 and 2), in its order, in our
/// names (`TotalDPS` is [`Field::HitDps`]).
pub const SLICE: [Field; 24] = [
    Field::Strength,
    Field::Dexterity,
    Field::Intelligence,
    Field::Life,
    Field::Mana,
    Field::EnergyShield,
    Field::Armour,
    Field::Evasion,
    Field::Resistance(DamageType::Fire),
    Field::Resistance(DamageType::Cold),
    Field::Resistance(DamageType::Lightning),
    Field::Resistance(DamageType::Chaos),
    Field::BlockChance,
    Field::Poe1(Poe1Field::SpellBlockChance),
    Field::Poe1(Poe1Field::SpellSuppressionChance),
    Field::LifeUnreserved,
    Field::ManaUnreserved,
    Field::AverageDamage,
    Field::Speed,
    Field::HitSpeed,
    Field::HitChance,
    Field::CritChance,
    Field::CritMultiplier,
    Field::HitDps,
];

/// The Path of Exile rules.
#[derive(Debug, Clone, Copy, Default)]
pub struct Poe1Rules;

impl Ruleset for Poe1Rules {
    fn game(&self) -> Game {
        Game::Poe1
    }

    fn slice(&self) -> &[Field] {
        &SLICE
    }

    fn mapping(&self) -> &dyn StatMapping {
        &Poe1Mapping
    }

    fn collect(&self, input: &CalcInput<'_>, ingest: &mut Ingest<'_>) -> Result<(), CalcError> {
        collect::collect(input, ingest);
        Ok(())
    }

    fn phases(&self) -> &[Phase] {
        &phases::PHASES
    }

    fn minion(
        &self,
        input: &CalcInput<'_>,
        player: &crate::store::ModStore,
    ) -> Option<Result<crate::engine::ActorState, crate::state::UnsupportedReason>> {
        minion_actor::minion(input, player)
    }

    fn states(
        &self,
        input: &CalcInput<'_>,
    ) -> std::collections::BTreeMap<crate::condition::ConditionKey, bool> {
        states::states(input)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::quantity::Quantity;

    #[test]
    fn poe1_names_should_be_stable() {
        assert_eq!(
            Quantity::Poe1(Poe1Quantity::SpellSuppressionChance).to_string(),
            "poe1.spell-suppression-chance"
        );
        assert_eq!(
            Field::Poe1(Poe1Field::SpellBlockChance).to_string(),
            "poe1.spell-block-chance"
        );
        assert_eq!(
            Poe1Quantity::SpellBlockChance.to_string(),
            "spell-block-chance"
        );
        assert_eq!(
            Quantity::Poe1(Poe1Quantity::Piece(
                ArmourSlot::BodyArmour,
                PieceValue::Percentile(Defence::EnergyShield)
            ))
            .to_string(),
            "poe1.piece.body-armour.percentile.energy-shield"
        );
        assert_eq!(ArmourSlot::of("Weapon 2 Swap"), Some(ArmourSlot::Weapon2));
        assert_eq!(ArmourSlot::of("Weapon 1"), None);
        for slot in ArmourSlot::ALL {
            assert_eq!(slot.row(), format!("slot.{}", slot.name()));
        }
        assert_eq!(
            Quantity::Poe1(Poe1Quantity::EnemyDamageTaken).to_string(),
            "poe1.enemy-damage-taken"
        );
        assert_eq!(
            Quantity::Poe1(Poe1Quantity::EnemyEvasion).to_string(),
            "poe1.enemy-evasion"
        );
        assert_eq!(
            Quantity::Poe1(Poe1Quantity::Onslaught).to_string(),
            "poe1.onslaught"
        );
        for (q, name) in [
            (
                Poe1Quantity::EnemyResistance(DamageType::Cold),
                "poe1.enemy-resistance.cold",
            ),
            (
                Poe1Quantity::ExposureChance(DamageType::Fire),
                "poe1.exposure-chance.fire",
            ),
            (Poe1Quantity::ExtraExposure(None), "poe1.extra-exposure"),
            (
                Poe1Quantity::ExtraExposure(Some(DamageType::Cold)),
                "poe1.extra-exposure.cold",
            ),
            (Poe1Quantity::ShockMinimum, "poe1.shock-minimum"),
            (Poe1Quantity::ShockEffect, "poe1.shock-effect"),
        ] {
            assert_eq!(Quantity::Poe1(q).to_string(), name);
        }
        assert_eq!(
            Quantity::Poe1(Poe1Quantity::Charge(
                charges::ChargeKind::Power,
                charges::ChargeValue::Per(charges::PerCharge::CritMultiplier)
            ))
            .to_string(),
            "poe1.charge.power.per.crit-multiplier"
        );
        assert_eq!(
            Poe1Field::SpellSuppressionChance.name(),
            "spell-suppression-chance"
        );
    }

    #[test]
    fn the_slice_should_have_each_field_once() {
        let unique: BTreeSet<_> = SLICE.iter().collect();
        assert_eq!(unique.len(), SLICE.len());
    }

    #[test]
    fn the_rules_should_run_the_phases_in_order() {
        assert_eq!(Poe1Rules.game(), Game::Poe1);
        let names: Vec<_> = Poe1Rules.phases().iter().map(|p| p.name).collect();
        assert_eq!(
            names,
            [
                "attributes",
                "life-mana",
                "resistances",
                "block",
                "primary-defences",
                "evade",
                "unreserved",
                "damage-reduction",
                "damage-taken",
                "enemy-hit",
                "regeneration",
                "leech",
                "movement",
                "minion",
                "hit",
                "dot",
                "costs",
                "combined",
                "minion-hit",
                "full-dps"
            ]
        );
        assert_eq!(
            Quantity::Poe1(Poe1Quantity::SkillBaseDamage(DamageType::Cold, Bound::Min)).to_string(),
            "poe1.skill-base-damage.cold.min"
        );
    }
}
