//! Stacks and config multipliers (Phase 3 plan, step 8; P3-8n): rage,
//! Inspiration charges, Momentum, Intensity, and the non-instant spells
//! cast recently.
//!
//! PoB v2.67.2 reads each as a multiplier (`Multiplier:<Name>`) that
//! modifiers per stack read; `ModStore` scales such a modifier by
//! `floor(multiplier + 0.0001)`, at most its limit (`Classes/ModStore.lua`
//! 326 to 375). The engine has no multipliers: [`stacks`] counts each kind
//! as PoB does, after every other source, and turns each modifier per
//! stack ([`StackValue::Per`]) into its target times the count, with the
//! modifier's source, as [`super::charges`] does.
//!
//! - **Rage** (`Modules/CalcPerform.lua` 881 to 897): only with
//!   `Condition:CanGainRage` (here [`StackValue::CanGain`], "Gain # Rage on
//!   Melee Hit", `ModParser.lua` 2533; `RageRegen` stats are unmapped, so
//!   reasons). The maximum is `floor(MaximumRage)`, the character's
//!   `maximum_rage` plus "+# to Maximum Rage" (no `MORE` mapped); the
//!   stacks `max(min(Multiplier:RageStack, maximum), 0)`, the config's
//!   `multiplierRage` (a count, `ConfigOptions.lua` 1062, conditioned on
//!   `CanGainRage`; no minimum rage mapped); the effect `floor(stacks x
//!   RageEffect)` (no rage effect mapped) as attack `Damage` MORE.
//!   "Nearby Enemies are Intimidated while you have Rage" makes the enemy
//!   Intimidated while `Multiplier:RageStack` is at least 1 (5031; the
//!   effect is [`super::damage_taken`]'s).
//! - **Inspiration charges** (`CalcPerform.lua` 972, 1061, 1095): the
//!   config's `overrideInspirationCharges` (`countAllowZero`), else the
//!   maximum, the character's `maximum_righteous_charges` (no modifier
//!   mapped). Inspiration Support's elemental damage MORE and crit chance
//!   INC per charge (`Data/Skills/sup_str.lua` 4614 to 4619).
//! - **Momentum** (`CalcOffence.lua` 1005 to 1015): with a maximum above 0
//!   (Momentum Support's `MomentumStacksMax`), the config's
//!   `MomentumStacks` (a count) at most the maximum, else `min((maximum +
//!   extra) / 2, maximum)`, the extra `MomentumStacksExtra`; attack
//!   `Speed` INC per stack (`sup_dex.lua` 3123 to 3131).
//! - **Intensity** (`ConfigOptions.lua` 503; `CalcSetup.lua` 522): the
//!   config's `intensifyIntensity` (a count), at most
//!   `Multiplier:IntensityLimit`, 3 plus "+# to maximum Intensity"
//!   (`ModParser.lua` 5572); Intensify's area `Damage` MORE per intensity
//!   (`sup_int.lua` 4040).
//! - **Non-instant spells cast recently** (`ConfigOptions.lua` 1493): the
//!   config's count; "#% increased Cast Speed for each different
//!   Non-Instant Spell you've Cast Recently" (`ModParser.lua` 1850).
//! - **Auras and heralds affecting you** (P6-11s; `CalcPerform.lua` 2112
//!   to 2126): PoB's `Multiplier:Herald` plus `Multiplier:AuraAffectingSelf`
//!   ("for each of your Aura or Herald Skills affecting you", `ModParser.lua`
//!   1398), counted from the skills' types
//!   ([`super::gems::aura_or_herald_count`]); "#% increased Damage for each
//!   of your Aura or Herald Skills affecting you". `Multiplier:Herald`
//!   alone ("for each Herald affecting you", 1397): "#% increased Damage
//!   for each Herald affecting you" (P6-11af).
//! - **Nearby enemies** (P6-11v; `ConfigOptions.lua` 1223 to 1231):
//!   `Multiplier:NearbyEnemies` is the config's `multiplierNearbyEnemies`
//!   plus `multiplierNearbyRareOrUniqueEnemies` (each a BASE in combat),
//!   and `Condition:OnlyOneNearbyEnemy` holds when the first is 1. Lunaris's
//!   "for each nearby Enemy, up to 8%" lines (`ModParser.lua` 1913: the
//!   tag's `limitTotal`, so `min(value x count, 8)`) on physical damage
//!   reduction and movement speed, and "while there is only one nearby
//!   Enemy" (1774: `limit = 1` with the condition) on physical damage
//!   reduction.
//! - **Gale Force** (P6-11ia; `ConfigOptions.lua` 925): the config's
//!   `multiplierGaleForce` with `Condition:CanGainGaleForce`, which
//!   Gathering Winds' two-line stat carries (`ModParser.lua` 2636); its
//!   Tailwind effect per Gale Force and Wind Ward's "#% less Damage taken
//!   per Gale Force". PoB caps nothing (`MaximumGaleForce` is unread).
//!
//! A config value PoB would read otherwise than a number is a reason under
//! its input. Other modifiers that read or change these counts (rage
//! effect, minimum rage, maximum Inspiration charges, rage on other hits)
//! are unmapped, so reasons.

use loom_domain::{ConfigInputKind, ConfigSet, ConfigValue};

use super::Poe1Quantity;
use super::config::last_input as last;
use crate::condition::Flags;
use crate::ingest::Ingest;
use crate::input::CalcData;
use crate::quantity::{Op, Quantity};
use crate::source::Source;
use crate::state::{IndeterminateReason, UnsupportedReason};
use crate::store::{Conditions, Effect};

/// A kind of stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StackKind {
    /// Rage.
    Rage,
    /// Inspiration charges.
    Inspiration,
    /// Momentum.
    Momentum,
    /// Intensity.
    Intensity,
    /// Different non-instant spells cast recently.
    NonInstantSpells,
    /// Aura and herald skills affecting you (P6-11s).
    AuraOrHerald,
    /// Herald skills affecting you (P6-11af).
    Herald,
    /// Nearby enemies (P6-11v).
    NearbyEnemies,
    /// Hits taken recently (P6-11cg; PoB's `Multiplier:BeenHitRecently`,
    /// the config's `multiplierBeenHitRecently`).
    HitsTakenRecently,
    /// Totems summoned (P6-11ep; PoB's `TotemsSummoned`, [`totems`]).
    Totems,
    /// Nearby corpses (P6-11ev; PoB's `Multiplier:NearbyCorpse`, the
    /// config's `multiplierNearbyCorpse`).
    NearbyCorpses,
    /// Your brands attached to the enemy (P6-11ew; PoB's enemy
    /// `Multiplier:BrandsAttached`, [`brands`]).
    BrandsAttached,
    /// Golems summoned (P6-11gc; PoB's `ActiveGolemLimit`, [`minions`]).
    Golems,
    /// Skeletons summoned (P6-11ie; PoB's `ActiveSkeletonLimit`,
    /// [`minions`]).
    Skeletons,
    /// Zombies raised (P6-11ie; PoB's `ActiveZombieLimit`, [`minions`]).
    Zombies,
    /// Raging spirits summoned (P6-11ie; PoB's `ActiveRagingSpiritLimit`,
    /// [`minions`]).
    RagingSpirits,
    /// Phantasms summoned (P6-11ie; PoB's `ActivePhantasmLimit`,
    /// [`minions`]).
    Phantasms,
    /// Trinity's lowest resonance (P6-11gd; PoB's
    /// `Multiplier:ResonanceCount`, the config's `configResonanceCount`).
    Resonance,
    /// Seconds you have been stationary (P6-11gr; PoB's
    /// `Multiplier:StationarySeconds`, the config's `conditionStationary`).
    StationarySeconds,
    /// Mana spent recently (P6-11gx; PoB's `Multiplier:ManaSpentRecently`,
    /// the config's `multiplierManaSpentRecently`).
    ManaSpentRecently,
    /// Eaten souls (P6-11hf; PoB's `Multiplier:SoulEaterStack`, the config's
    /// `multiplierSoulEater`, with `Condition:CanHaveSoulEater`).
    SoulEater,
    /// Gale Force (P6-11ia; PoB's `Multiplier:GaleForce`, the config's
    /// `multiplierGaleForce`, with `Condition:CanGainGaleForce`).
    GaleForce,
    /// Warcries used recently (close-out M2; PoB's
    /// `Multiplier:WarcryUsedRecently`, the config's
    /// `multiplierWarcryUsedRecently`).
    WarcriesUsedRecently,
    /// Corpses consumed recently (close-out M2; PoB's
    /// `Multiplier:CorpseConsumedRecently`, the config's
    /// `multiplierCorpseConsumedRecently`).
    CorpsesConsumedRecently,
    /// The seconds the enemy has been in your presence (close-out M2; the
    /// enemy's `Multiplier:EnemyPresenceSeconds`, the config's
    /// `enemyMultiplierEnemyPresenceSeconds`).
    EnemyPresenceSeconds,
    /// Defiance (open items S7; PoB's `Multiplier:Defiance`, the config's
    /// `multiplierDefiance`, at most 10).
    Defiance,
    /// The enemy's elemental ailments (the agent list, 2026-11-27; Elemental
    /// Hit's `Multiplier:ElementalHitAilmentOnEnemy`, 1 for each of ignited,
    /// chilled, frozen, shocked, scorched, brittle, sapped,
    /// [`enemy_elemental_ailments`]).
    EnemyElementalAilments,
}

impl StackKind {
    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Rage => "rage",
            Self::Inspiration => "inspiration",
            Self::Momentum => "momentum",
            Self::Intensity => "intensity",
            Self::NonInstantSpells => "non-instant-spells",
            Self::AuraOrHerald => "aura-or-herald",
            Self::Herald => "herald",
            Self::NearbyEnemies => "nearby-enemies",
            Self::HitsTakenRecently => "hits-taken-recently",
            Self::Totems => "totems",
            Self::NearbyCorpses => "nearby-corpses",
            Self::BrandsAttached => "brands-attached",
            Self::Golems => "golems",
            Self::Skeletons => "skeletons",
            Self::Zombies => "zombies",
            Self::RagingSpirits => "raging-spirits",
            Self::Phantasms => "phantasms",
            Self::Resonance => "resonance",
            Self::StationarySeconds => "stationary-seconds",
            Self::ManaSpentRecently => "mana-spent-recently",
            Self::SoulEater => "soul-eater",
            Self::GaleForce => "gale-force",
            Self::WarcriesUsedRecently => "warcries-used-recently",
            Self::CorpsesConsumedRecently => "corpses-consumed-recently",
            Self::EnemyPresenceSeconds => "enemy-presence-seconds",
            Self::Defiance => "defiance",
            Self::EnemyElementalAilments => "enemy-elemental-ailments",
        }
    }
}

/// What a stack modifier changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StackValue {
    /// The maximum (or Intensity's limit): `Base`.
    Maximum,
    /// Momentum counted as more than it is (`MomentumStacksExtra`): `Base`.
    Extra,
    /// The stacks can be gained (rage's `Condition:CanGainRage`): a `Flag`.
    CanGain,
    /// The enemy is Intimidated while you have rage: a `Flag`.
    Intimidate,
    /// Increased rage effect (PoB's `RageEffect` INC; P6-11dt): `Base`
    /// (a line) or `Increased` (Berserk's buff), the percent summed.
    Effect,
    /// The ballista totem limit (PoB's `ActiveBallistaLimit` BASE with
    /// `SkillType.TotemsAreBallistae`; P6-11ep): `Base`, counted only for a
    /// ballista main skill.
    BallistaMaximum,
    /// The brands attached to an enemy (PoB's `BrandsAttachedLimit` BASE
    /// on the character; P6-11hv, Runebinder): `Base`, added to each brand
    /// skill's own limit.
    AttachMaximum,
    /// An effect per stack.
    Per(PerStack),
}

impl StackValue {
    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Maximum => "maximum",
            Self::Extra => "extra",
            Self::CanGain => "can-gain",
            Self::Intimidate => "intimidate",
            Self::Effect => "effect",
            Self::BallistaMaximum => "ballista-maximum",
            Self::AttachMaximum => "attach-maximum",
            Self::Per(p) => p.name(),
        }
    }
}

/// An effect per stack: its target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PerStack {
    /// Increased attack speed (`Speed` INC, `ModFlag.Attack`).
    AttackSpeed,
    /// Increased cast speed (`Speed` INC, `ModFlag.Cast`).
    CastSpeed,
    /// Increased critical strike chance (`CritChance` INC).
    CritChance,
    /// More elemental damage (`ElementalDamage` MORE).
    ElementalDamageMore,
    /// More area damage (`Damage` MORE, `ModFlag.Area`).
    AreaDamageMore,
    /// Increased damage (`Damage` INC).
    Damage,
    /// Physical damage reduction (`PhysicalDamageReduction` BASE), its total
    /// at most 8 (P6-11v).
    PhysicalReductionUpTo8,
    /// Increased movement speed (`MovementSpeed` INC), its total at most 8
    /// (P6-11v).
    MovementSpeedUpTo8,
    /// Physical damage reduction while only one enemy is nearby
    /// (`PhysicalDamageReduction` BASE, P6-11v).
    PhysicalReductionIfOnlyOne,
    /// Physical damage reduction (`PhysicalDamageReduction` BASE), its
    /// total at most 5 (P6-11cg).
    PhysicalReductionUpTo5,
    /// More damage (`Damage` MORE; P6-11ep).
    DamageMore,
    /// Mana regeneration, percent per minute (`ManaRegenPercent` BASE per
    /// second, the stat per minute; P6-11ep).
    ManaRegenPercentPerMinute,
    /// Life regeneration, percent per minute (`LifeRegenPercent` BASE per
    /// second, the stat per minute; P6-11ep).
    LifeRegenPercentPerMinute,
    /// Increased effect of Arcane Surge on you (`ArcaneSurgeEffect` INC;
    /// P6-11ep).
    ArcaneSurgeEffect,
    /// More damage with at least one stack (`Damage` MORE with a
    /// `MultiplierThreshold` of 1; P6-11ev).
    DamageMoreWithAny,
    /// The enemy's increased damage with at least one stack (the enemy's
    /// `Damage` INC with a `MultiplierThreshold` of 1; P6-11ev).
    EnemyDamageWithAny,
    /// Energy shield regeneration, percent per minute (`EnergyShieldRegenPercent`
    /// BASE per second, the stat per minute), its total at most 2% per
    /// second (`limit` 2, `limitTotal`; P6-11ev).
    EnergyShieldRegenPercentPerMinuteUpTo2,
    /// Missing unreserved life gained before each enemy hit, percent
    /// (`MissingLifeBeforeEnemyHit` BASE; open items S7).
    MissingLifeBeforeHit,
    /// The enemy's increased damage taken (`DamageTaken` INC on the enemy;
    /// P6-11ew).
    EnemyDamageTaken,
    /// Elemental penetration while the lowest resonance is at least 25
    /// (`ElementalPenetration` BASE with a `MultiplierThreshold` of 25;
    /// P6-11gd): the target is fire's, and [`stacks`] adds cold's and
    /// lightning's (PoB's `ElementalPenetration` is all three).
    ElementalPenetrationIfAll25,
    /// Attack speed per enemy in close range (P6-11gr; `Speed` INC with
    /// `ModFlag.Attack`, `AtCloseRange`).
    AttackSpeedAtCloseRange,
    /// Armour per second stationary, up to 100% (P6-11gr; `Armour` INC,
    /// `Stationary`, the multiplier's `limit` 100 / value).
    ArmourUpTo100,
    /// Increased effect of Tailwind on you (`TailwindEffectOnSelf` INC;
    /// P6-11ia).
    TailwindEffect,
    /// More damage taken (`DamageTaken` MORE; P6-11ia).
    DamageTakenMore,
    /// Increased spell damage (`Damage` INC, `ModFlag.Spell`; P6-11ie).
    SpellDamage,
    /// Increased attack damage (`Damage` INC, `ModFlag.Attack`; P6-11ie).
    AttackDamage,
    /// Increased attack and cast speed (`Speed` INC; P6-11ie).
    Speed,
    /// Increased movement speed (`MovementSpeed` INC; P6-11ie).
    MovementSpeed,
    /// Increased armour (`Armour` INC; close-out M2).
    Armour,
    /// More damage with hits and ailments against a rare or unique enemy
    /// (`Damage` MORE, `KeywordFlag.Hit` or `Ailment`, the enemy's
    /// `RareOrUnique`; close-out M2).
    DamageMoreVsRareOrUnique,
}

impl PerStack {
    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::AttackSpeed => "per.attack-speed",
            Self::CastSpeed => "per.cast-speed",
            Self::CritChance => "per.crit-chance",
            Self::ElementalDamageMore => "per.elemental-damage-more",
            Self::AreaDamageMore => "per.area-damage-more",
            Self::Damage => "per.damage",
            Self::PhysicalReductionUpTo8 => "per.physical-reduction-up-to-8",
            Self::MovementSpeedUpTo8 => "per.movement-speed-up-to-8",
            Self::PhysicalReductionIfOnlyOne => "per.physical-reduction-if-only-one",
            Self::PhysicalReductionUpTo5 => "per.physical-reduction-up-to-5",
            Self::DamageMore => "per.damage-more",
            Self::ManaRegenPercentPerMinute => "per.mana-regen-percent-per-minute",
            Self::LifeRegenPercentPerMinute => "per.life-regen-percent-per-minute",
            Self::ArcaneSurgeEffect => "per.arcane-surge-effect",
            Self::AttackSpeedAtCloseRange => "per.attack-speed-at-close-range",
            Self::ArmourUpTo100 => "per.armour-up-to-100",
            Self::TailwindEffect => "per.tailwind-effect",
            Self::DamageTakenMore => "per.damage-taken-more",
            Self::SpellDamage => "per.spell-damage",
            Self::AttackDamage => "per.attack-damage",
            Self::Speed => "per.speed",
            Self::MovementSpeed => "per.movement-speed",
            Self::Armour => "per.armour",
            Self::DamageMoreVsRareOrUnique => "per.damage-more-vs-rare-or-unique",
            Self::DamageMoreWithAny => "per.damage-more-with-any",
            Self::EnemyDamageTaken => "per.enemy-damage-taken",
            Self::ElementalPenetrationIfAll25 => "per.elemental-penetration-if-all-25",
            Self::EnemyDamageWithAny => "per.enemy-damage-with-any",
            Self::EnergyShieldRegenPercentPerMinuteUpTo2 => {
                "per.energy-shield-regen-percent-per-minute-up-to-2"
            }
            Self::MissingLifeBeforeHit => "per.missing-life-before-hit",
        }
    }

    /// The condition an effect per stack needs (close-out M2), if any.
    #[must_use]
    pub const fn state(self) -> Option<&'static str> {
        match self {
            Self::DamageMoreVsRareOrUnique => Some(super::conditions::ENEMY_RARE_OR_UNIQUE),
            _ => None,
        }
    }

    /// The effect one stack has: quantity, operation, and the flags a
    /// query needs (all of `.2`, one of `.3`).
    #[must_use]
    pub const fn target(self) -> (Quantity, Op, Flags, Flags) {
        match self {
            Self::AttackSpeed => (Quantity::Speed, Op::Increased, Flags::ATTACK, Flags::NONE),
            Self::CastSpeed => (Quantity::Speed, Op::Increased, Flags::CAST, Flags::NONE),
            Self::CritChance => (
                Quantity::CritChance,
                Op::Increased,
                Flags::NONE,
                Flags::NONE,
            ),
            Self::ElementalDamageMore => {
                (Quantity::Damage, Op::More, Flags::NONE, Flags::ELEMENTAL)
            }
            Self::AreaDamageMore => (Quantity::Damage, Op::More, Flags::AREA, Flags::NONE),
            Self::Damage => (Quantity::Damage, Op::Increased, Flags::NONE, Flags::NONE),
            Self::SpellDamage => (Quantity::Damage, Op::Increased, Flags::SPELL, Flags::NONE),
            Self::AttackDamage => (Quantity::Damage, Op::Increased, Flags::ATTACK, Flags::NONE),
            Self::Speed => (Quantity::Speed, Op::Increased, Flags::NONE, Flags::NONE),
            Self::PhysicalReductionUpTo8
            | Self::PhysicalReductionIfOnlyOne
            | Self::PhysicalReductionUpTo5 => (
                Quantity::Poe1(Poe1Quantity::DamageReduction(
                    crate::quantity::DamageType::Physical,
                )),
                Op::Base,
                Flags::NONE,
                Flags::NONE,
            ),
            Self::MovementSpeedUpTo8 | Self::MovementSpeed => (
                Quantity::Poe1(Poe1Quantity::MovementSpeed),
                Op::Increased,
                Flags::NONE,
                Flags::NONE,
            ),
            Self::DamageMore => (Quantity::Damage, Op::More, Flags::NONE, Flags::NONE),
            Self::AttackSpeedAtCloseRange => {
                (Quantity::Speed, Op::Increased, Flags::ATTACK, Flags::NONE)
            }
            Self::ArmourUpTo100 | Self::Armour => {
                (Quantity::Armour, Op::Increased, Flags::NONE, Flags::NONE)
            }
            Self::DamageMoreVsRareOrUnique => (
                Quantity::Damage,
                Op::More,
                Flags::NONE,
                Flags::HIT.union(Flags::AILMENT),
            ),
            Self::TailwindEffect => (
                Quantity::Poe1(Poe1Quantity::TailwindEffect),
                Op::Increased,
                Flags::NONE,
                Flags::NONE,
            ),
            Self::DamageTakenMore => (
                Quantity::Poe1(Poe1Quantity::SelfDamageTaken(
                    super::TakenWhen::Always,
                    super::TakenOf::All,
                )),
                Op::More,
                Flags::NONE,
                Flags::NONE,
            ),
            Self::ManaRegenPercentPerMinute => (
                Quantity::Poe1(Poe1Quantity::RegenPercentPerMinute(super::Pool::Mana)),
                Op::Base,
                Flags::NONE,
                Flags::NONE,
            ),
            Self::LifeRegenPercentPerMinute => (
                Quantity::Poe1(Poe1Quantity::RegenPercentPerMinute(super::Pool::Life)),
                Op::Base,
                Flags::NONE,
                Flags::NONE,
            ),
            Self::ArcaneSurgeEffect => (
                Quantity::Poe1(Poe1Quantity::ArcaneSurgeEffect),
                Op::Increased,
                Flags::NONE,
                Flags::NONE,
            ),
            Self::DamageMoreWithAny => (Quantity::Damage, Op::More, Flags::NONE, Flags::NONE),
            Self::ElementalPenetrationIfAll25 => (
                Quantity::Penetration(crate::quantity::DamageType::Fire),
                Op::Base,
                Flags::NONE,
                Flags::NONE,
            ),
            Self::EnemyDamageTaken => (
                Quantity::Poe1(Poe1Quantity::EnemyDamageTaken),
                Op::Increased,
                Flags::NONE,
                Flags::NONE,
            ),
            Self::EnemyDamageWithAny => (
                Quantity::Poe1(Poe1Quantity::EnemyDamage),
                Op::Increased,
                Flags::NONE,
                Flags::NONE,
            ),
            Self::EnergyShieldRegenPercentPerMinuteUpTo2 => (
                Quantity::Poe1(Poe1Quantity::RegenPercentPerMinute(
                    super::Pool::EnergyShield,
                )),
                Op::Base,
                Flags::NONE,
                Flags::NONE,
            ),
            Self::MissingLifeBeforeHit => (
                Quantity::Poe1(Poe1Quantity::MissingLifeBeforeHit),
                Op::Base,
                Flags::NONE,
                Flags::NONE,
            ),
        }
    }
}

/// The stack modifiers the engine models (module docs), sorted by stat
/// ID, each as PoB v2.67.2 reads its text or skill stat.
pub const MODIFIERS: &[(&str, StackKind, StackValue)] = &[
    // P6-11gx: "#% increased Effect of Arcane Surge on you per 200 Mana spent
    // Recently, up to 50%" (`ArcaneSurgeEffect` INC per `ManaSpentRecently`,
    // `div` 200, `limit` 50).
    (
        "arcane_surge_effect_+%_per_200_mana_spent_recently_up_to_50%",
        StackKind::ManaSpentRecently,
        StackValue::Per(PerStack::ArcaneSurgeEffect),
    ),
    // P6-11ep: "#% increased effect of Arcane Surge on you per Summoned Totem"
    // (`ArcaneSurgeEffect` INC per `TotemsSummoned`).
    (
        "arcane_surge_effect_+%_per_active_totem",
        StackKind::Totems,
        StackValue::Per(PerStack::ArcaneSurgeEffect),
    ),
    // Close-out M2: "Every Rage also grants #% increased Armour" (`Armour`
    // INC per `Multiplier:RageEffect`).
    (
        "armour_+%_per_rage",
        StackKind::Rage,
        StackValue::Per(PerStack::Armour),
    ),
    // P6-11gr: "#% increased Armour per second you've been stationary, up
    // to a maximum of 100%" (`Armour` INC per `StationarySeconds`).
    (
        "armour_+%_per_second_while_stationary_up_to_100",
        StackKind::StationarySeconds,
        StackValue::Per(PerStack::ArmourUpTo100),
    ),
    // Close-out M2: "#% increased Attack and Cast Speed for each corpse
    // Consumed Recently, up to a maximum of 200%" (`Speed` INC per
    // `CorpseConsumedRecently`, `limit` 50).
    (
        "attack_and_cast_speed_+%_per_corpse_consumed_recently",
        StackKind::CorpsesConsumedRecently,
        StackValue::Per(PerStack::Speed),
    ),
    // P6-11ie: "#% increased Attack and Cast Speed per Summoned Raging
    // Spirit" (`Speed` INC, `PerStat` `ActiveRagingSpiritLimit`).
    (
        "attack_and_cast_speed_+%_per_summoned_raging_spirit",
        StackKind::RagingSpirits,
        StackValue::Per(PerStack::Speed),
    ),
    // P6-11ie: "#% increased Attack Damage per Raised Zombie" (`Damage` INC,
    // `ModFlag.Attack`, `PerStat` `ActiveZombieLimit`).
    (
        "attack_damage_+%_per_raised_zombie",
        StackKind::Zombies,
        StackValue::Per(PerStack::AttackDamage),
    ),
    // P6-11ep: "Attack Skills have +# to maximum number of Summoned Ballista
    // Totems" (`ActiveBallistaLimit` BASE, `SkillType.TotemsAreBallistae`).
    (
        "attack_skills_additional_ballista_totems_allowed",
        StackKind::Totems,
        StackValue::BallistaMaximum,
    ),
    // P6-11gr: "#% increased Attack Speed per Enemy in Close Range" (`Speed`
    // INC, `ModFlag.Attack`, per `NearbyEnemies`, `AtCloseRange`).
    (
        "attack_speed_+%_per_enemy_in_close_range",
        StackKind::NearbyEnemies,
        StackValue::Per(PerStack::AttackSpeedAtCloseRange),
    ),
    // P6-11dt: "Every Rage also grants #% increased Attack Speed" (`Speed`
    // INC with `ModFlag.Attack` per `Multiplier:RageEffect`).
    (
        "attack_speed_+%_per_rage",
        StackKind::Rage,
        StackValue::Per(PerStack::AttackSpeed),
    ),
    // P6-11gc: "+# to maximum number of Summoned Golems" (`ActiveGolemLimit`
    // BASE, on every golem skill's list).
    (
        "base_number_of_golems_allowed",
        StackKind::Golems,
        StackValue::Maximum,
    ),
    // P6-11ie: "+# to maximum number of Summoned Raging Spirits"
    // (`ActiveRagingSpiritLimit` BASE, on every raging spirit skill's list).
    (
        "base_number_of_raging_spirits_allowed",
        StackKind::RagingSpirits,
        StackValue::Maximum,
    ),
    // P6-11ie: "+# to maximum number of Skeletons" (`ActiveSkeletonLimit`
    // BASE).
    (
        "base_number_of_skeletons_allowed",
        StackKind::Skeletons,
        StackValue::Maximum,
    ),
    // P6-11ie: "+# to maximum number of Summoned Phantasms"
    // (`ActivePhantasmLimit` BASE).
    (
        "base_number_of_support_ghosts_allowed",
        StackKind::Phantasms,
        StackValue::Maximum,
    ),
    // P6-11hs: Multiple Totems' "+# to maximum number of Summoned Totems"
    // on the supported skill (`SkillStatMap.lua` 1883: `ActiveTotemLimit`
    // BASE).
    (
        "base_number_of_totems_allowed",
        StackKind::Totems,
        StackValue::Maximum,
    ),
    // P6-11ie: "+# to maximum number of Raised Zombies" (`ActiveZombieLimit`
    // BASE).
    (
        "base_number_of_zombies_allowed",
        StackKind::Zombies,
        StackValue::Maximum,
    ),
    // P6-11ew: "You can Cast an additional Brand" (`ActiveBrandLimit` BASE).
    (
        "base_total_number_of_sigils_allowed",
        StackKind::BrandsAttached,
        StackValue::Maximum,
    ),
    (
        "cast_speed_+%_per_num_unique_spells_cast_recently",
        StackKind::NonInstantSpells,
        StackValue::Per(PerStack::CastSpeed),
    ),
    (
        "count_as_momentum_+_if_not_channelled",
        StackKind::Momentum,
        StackValue::Extra,
    ),
    (
        "critical_strike_chance_+%_per_righteous_charge",
        StackKind::Inspiration,
        StackValue::Per(PerStack::CritChance),
    ),
    (
        "damage_+%_for_each_herald_affecting_you",
        StackKind::Herald,
        StackValue::Per(PerStack::Damage),
    ),
    // P6-11gc: "#% increased Damage per Summoned Golem" (`Damage` INC,
    // `PerStat` `ActiveGolemLimit`).
    (
        "damage_+%_per_active_golem",
        StackKind::Golems,
        StackValue::Per(PerStack::Damage),
    ),
    // Close-out M2: "#% increased Damage for each time you've Warcried
    // Recently" (`Damage` INC per `WarcryUsedRecently`).
    (
        "damage_+%_per_warcry_used_recently",
        StackKind::WarcriesUsedRecently,
        StackValue::Per(PerStack::Damage),
    ),
    (
        "damage_+%_per_your_aura_or_herald_skill_affecting_you",
        StackKind::AuraOrHerald,
        StackValue::Per(PerStack::Damage),
    ),
    // P6-11gd: Trinity's penetration while every resonance is at least 25
    // (`ElementalPenetration` BASE, `MultiplierThreshold` 25 on
    // `ResonanceCount`).
    (
        "damage_penetrates_%_elemental_resistances_while_all_resonance_is_25",
        StackKind::Resonance,
        StackValue::Per(PerStack::ElementalPenetrationIfAll25),
    ),
    // P6-11ia: "#% less Damage taken per Gale Force" (`DamageTaken` MORE per
    // `Multiplier:GaleForce`).
    (
        "damage_taken_+%_final_per_gale_force",
        StackKind::GaleForce,
        StackValue::Per(PerStack::DamageTakenMore),
    ),
    // P6-11hf: "#% chance to gain Soul Eater for # seconds on Killing Blow ... with Double Strike or Dual Strike" (`ModParser.lua` 4306).
    (
        "double_and_dual_strike_soul_eater_for_20_seconds_on_rare_or_unique_kill_chance_%",
        StackKind::SoulEater,
        StackValue::CanGain,
    ),
    // P6-11gd: Trinity's more elemental damage per 5 lowest resonance
    // (`ElementalDamage` MORE, `Multiplier` `ResonanceCount` `div` 5).
    (
        "elemental_damage_+%_final_per_5_lowest_resonance",
        StackKind::Resonance,
        StackValue::Per(PerStack::ElementalDamageMore),
    ),
    (
        "elemental_damage_+%_final_per_righteous_charge",
        StackKind::Inspiration,
        StackValue::Per(PerStack::ElementalDamageMore),
    ),
    // The agent list (2026-11-27): Elemental Hit's "#% more Damage for each
    // different Elemental Ailment on the enemy" (`Damage` MORE per
    // `Multiplier:ElementalHitAilmentOnEnemy`, its base mods).
    (
        "elemental_hit_damage_+%_final_per_enemy_elemental_ailment",
        StackKind::EnemyElementalAilments,
        StackValue::Per(PerStack::DamageMore),
    ),
    (
        "gain_max_rage_on_losing_temporal_chains_debuff",
        StackKind::Rage,
        StackValue::CanGain,
    ),
    // P6-11hf: "Eat a Soul when you Hit a Rare or Unique Enemy, no more than once every # seconds" (`ModParser.lua` 4305).
    (
        "gain_soul_eater_stack_on_hit_vs_unique_cooldown_ms",
        StackKind::SoulEater,
        StackValue::CanGain,
    ),
    // P6-11bc: "Gain # Rage on Attack Hit": PoB's `Condition:CanGainRage`,
    // as on melee hit; P6-11bm: the other lines PoB parses as that flag.
    (
        "gain_x_rage_on_attack_hit",
        StackKind::Rage,
        StackValue::CanGain,
    ),
    (
        "gain_x_rage_on_hit_with_axes",
        StackKind::Rage,
        StackValue::CanGain,
    ),
    (
        "gain_x_rage_on_hit_with_axes_swords",
        StackKind::Rage,
        StackValue::CanGain,
    ),
    (
        "gain_x_rage_on_hit_with_tinctured_weapons",
        StackKind::Rage,
        StackValue::CanGain,
    ),
    (
        "gain_x_rage_on_melee_hit",
        StackKind::Rage,
        StackValue::CanGain,
    ),
    (
        "gain_x_rage_per_200_mana_spent",
        StackKind::Rage,
        StackValue::CanGain,
    ),
    ("gain_x_rage_when_hit", StackKind::Rage, StackValue::CanGain),
    (
        "gain_x_rage_when_you_use_a_life_flask",
        StackKind::Rage,
        StackValue::CanGain,
    ),
    // Close-out M2: "Deal #% more Damage with Hits and Ailments to Rare and
    // Unique Enemies for each second they've ever been in your Presence, up
    // to a maximum of 100%" (`Damage` MORE per the enemy's
    // `EnemyPresenceSeconds`, `limit` 100, the enemy's `RareOrUnique`).
    (
        "gladiator_damage_vs_rare_unique_enemies_+%_final_per_second_in_your_presence_up_to_100%",
        StackKind::EnemyPresenceSeconds,
        StackValue::Per(PerStack::DamageMoreVsRareOrUnique),
    ),
    // P6-11ep: "#% more Damage per Summoned Totem" (`Damage` MORE per
    // `TotemsSummoned`).
    (
        "hierophant_passive_damage_+%_final_per_totem",
        StackKind::Totems,
        StackValue::Per(PerStack::DamageMore),
    ),
    // P6-11ep: "Regenerate #% of Mana per second for each Summoned Totem"
    // (`ManaRegenPercent` BASE per `TotemsSummoned`).
    (
        "mana_regeneration_rate_per_minute_%_per_active_totem",
        StackKind::Totems,
        StackValue::Per(PerStack::ManaRegenPercentPerMinute),
    ),
    (
        "maximum_intensify_stacks",
        StackKind::Intensity,
        StackValue::Maximum,
    ),
    ("maximum_rage", StackKind::Rage, StackValue::Maximum),
    // Open items S7: "Gain #% of Missing Unreserved Life before being Hit by
    // an Enemy Per Defiance" (`MissingLifeBeforeEnemyHit` BASE per
    // `Multiplier:Defiance`; its `ModCache` parse).
    (
        "missing_unreserved_life_%_gained_as_life_before_hit_per_defiance",
        StackKind::Defiance,
        StackValue::Per(PerStack::MissingLifeBeforeHit),
    ),
    (
        "movement_speed_+%_per_nearby_enemy",
        StackKind::NearbyEnemies,
        StackValue::Per(PerStack::MovementSpeedUpTo8),
    ),
    // P6-11ie: "#% increased Movement Speed per Summoned Phantasm"
    // (`MovementSpeed` INC, `PerStat` `ActivePhantasmLimit`).
    (
        "movement_speed_+%_per_summoned_phantasm",
        StackKind::Phantasms,
        StackValue::Per(PerStack::MovementSpeed),
    ),
    (
        "nearby_enemies_are_intimidated_while_you_have_rage",
        StackKind::Rage,
        StackValue::Intimidate,
    ),
    // P6-11ev: "With at least one nearby corpse, you and nearby Allies deal #% more
    // Damage" (PoB's `ExtraAura` of `Damage` MORE, `MultiplierThreshold`
    // `NearbyCorpse` 1).
    (
        "necromancer_damage_+%_final_for_you_and_allies_with_nearby_corpse",
        StackKind::NearbyCorpses,
        StackValue::Per(PerStack::DamageMoreWithAny),
    ),
    // P6-11ev: "With at least one nearby corpse, nearby Enemies deal #% reduced
    // Damage" (the enemy's `Damage` INC, `MultiplierThreshold` `NearbyCorpse`
    // 1).
    (
        "necromancer_damage_+%_for_nearby_enemies_with_nearby_corpse",
        StackKind::NearbyCorpses,
        StackValue::Per(PerStack::EnemyDamageWithAny),
    ),
    // P6-11ev: "For each nearby corpse, you and nearby Allies Regenerate #% of
    // Energy Shield per second, up to 2.0% per second" (PoB's `ExtraAura` of
    // `EnergyShieldRegenPercent` BASE per `NearbyCorpse`, `limit` 2,
    // `limitTotal`).
    (
        "necromancer_energy_shield_regeneration_rate_per_minute_%_for_you_and_allies_per_nearby_corpse",
        StackKind::NearbyCorpses,
        StackValue::Per(PerStack::EnergyShieldRegenPercentPerMinuteUpTo2),
    ),
    // P6-11ep: "+# to maximum number of Summoned Totems" (`ActiveTotemLimit`
    // BASE).
    (
        "number_of_additional_totems_allowed",
        StackKind::Totems,
        StackValue::Maximum,
    ),
    (
        "physical_damage_reduction_%_if_only_one_enemy_nearby",
        StackKind::NearbyEnemies,
        StackValue::Per(PerStack::PhysicalReductionIfOnlyOne),
    ),
    // P6-11cg: "#% additional Physical Damage Reduction for each Hit you've
    // taken Recently up to a maximum of 5%" (`PhysicalDamageReduction` BASE
    // per `Multiplier:BeenHitRecently`, `limit` 5, `limitTotal`).
    (
        "physical_damage_reduction_%_per_hit_you_have_taken_recently",
        StackKind::HitsTakenRecently,
        StackValue::Per(PerStack::PhysicalReductionUpTo5),
    ),
    (
        "physical_damage_reduction_%_per_nearby_enemy",
        StackKind::NearbyEnemies,
        StackValue::Per(PerStack::PhysicalReductionUpTo8),
    ),
    // P6-11dt: "#% increased Rage Effect" (`RageEffect` INC).
    ("rage_effect_+%", StackKind::Rage, StackValue::Effect),
    // P6-11ew: "Enemies take #% increased Damage for each of your Brands Attached to
    // them" (the enemy's `DamageTaken` INC per `Multiplier:BrandsAttached`).
    (
        "sigil_attached_target_damage_taken_+%",
        StackKind::BrandsAttached,
        StackValue::Per(PerStack::EnemyDamageTaken),
    ),
    // P6-11hf: "Soul Eater" (`ModParser.lua` 3975).
    (
        "soul_eater_from_unique",
        StackKind::SoulEater,
        StackValue::CanGain,
    ),
    // P6-11hf: "+# to maximum number of Eaten Souls" (`SoulEaterMax` BASE, `ModParser.lua` 4310).
    (
        "soul_eater_maximum_stacks_+",
        StackKind::SoulEater,
        StackValue::Maximum,
    ),
    // P6-11ie: "#% increased Spell Damage per Summoned Skeleton" (`Damage`
    // INC, `ModFlag.Spell`, `PerStat` `ActiveSkeletonLimit`).
    (
        "spell_damage_+%_per_summoned_skeleton",
        StackKind::Skeletons,
        StackValue::Per(PerStack::SpellDamage),
    ),
    (
        "support_momentum_attack_speed_+%_per_stack",
        StackKind::Momentum,
        StackValue::Per(PerStack::AttackSpeed),
    ),
    (
        "support_momentum_max_stacks",
        StackKind::Momentum,
        StackValue::Maximum,
    ),
    (
        "support_spell_boost_area_damage_+%_final_per_charge",
        StackKind::Intensity,
        StackValue::Per(PerStack::AreaDamageMore),
    ),
    // P6-11ep: "You and your Totems Regenerate #% of Life per second for each
    // Summoned Totem" (`LifeRegenPercent` BASE per `TotemsSummoned`; the
    // totems' share is theirs).
    // P6-11ia: "Gain 1 Gale Force when you use a Skill" with "#% increased
    // Effect of Tailwind on you per Gale Force", one stat of two lines
    // (`ModParser.lua` 2636: `Condition:CanGainGaleForce`; `TailwindEffectOnSelf`
    // INC per `Multiplier:GaleForce`): [`stacks`] reads its presence as the
    // flag too.
    (
        "tailwind_effect_on_self_+%_per_gale_force",
        StackKind::GaleForce,
        StackValue::Per(PerStack::TailwindEffect),
    ),
    // The agent list (a game-truth line PoB reads): "Each Totem applies #%
    // increased Damage taken to Enemies near it" (`ModParser.lua` 4367: the
    // enemy's `DamageTaken` INC per `TotemsSummoned`).
    (
        "totems_nearby_enemies_damage_taken_+%",
        StackKind::Totems,
        StackValue::Per(PerStack::EnemyDamageTaken),
    ),
    (
        "you_and_totem_life_regeneration_rate_per_minute_%_per_active_totem",
        StackKind::Totems,
        StackValue::Per(PerStack::LifeRegenPercentPerMinute),
    ),
];

/// The stack modifier `stat` is, if any ([`MODIFIERS`]).
#[must_use]
pub fn modifier(stat: &str) -> Option<(StackKind, StackValue)> {
    MODIFIERS
        .binary_search_by(|(id, _, _)| (*id).cmp(stat))
        .ok()
        .map(|i| (MODIFIERS[i].1, MODIFIERS[i].2))
}

/// The config's rage input.
pub const RAGE_INPUT: &str = "multiplierRage";
/// The config's Inspiration charge input.
pub const INSPIRATION_INPUT: &str = "overrideInspirationCharges";
/// The config's Momentum input.
pub const MOMENTUM_INPUT: &str = "MomentumStacks";
/// The config's Intensity input.
pub const INTENSITY_INPUT: &str = "intensifyIntensity";
/// The config's non-instant spells input.
pub const SPELLS_INPUT: &str = "multiplierNonInstantSpellCastRecently";
/// The config's lowest resonance input (P6-11gd; `ConfigOptions.lua`
/// 708, Trinity's).
pub const RESONANCE_INPUT: &str = "configResonanceCount";
/// The largest resonance the config sets (`m_min(val, 50)`).
const RESONANCE_LIMIT: f64 = 50.0;
/// Trinity's penetration threshold (its `MultiplierThreshold`).
const RESONANCE_THRESHOLD: f64 = 25.0;
/// Trinity's more elemental damage per this much resonance (its `div`).
const RESONANCE_DIV: f64 = 5.0;

/// The config's nearby enemies input (P6-11v).
pub const NEARBY_INPUT: &str = "multiplierNearbyEnemies";
/// The total limit of Lunaris's "for each nearby Enemy, up to 8%" (the
/// stat descriptions' fixed text; P6-11v).
pub const NEARBY_LIMIT_TOTAL: f64 = 8.0;

/// The total of physical damage reduction per hit taken recently (PoB's
/// `limit = 5`, `limitTotal`, from the text "up to a maximum of 5%").
pub const HITS_TAKEN_LIMIT_TOTAL: f64 = 5.0;

/// The config's Defiance (`ConfigOptions.lua` 1059:
/// `Multiplier:Defiance` BASE `min(count, 10)` in combat; open items S7).
pub const DEFIANCE_INPUT: &str = "multiplierDefiance";

/// The most Defiance the config counts (`ConfigOptions.lua` 1060).
const DEFIANCE_MAX: f64 = 10.0;

/// The inputs this module reads.
pub const INPUTS: [&str; 16] = [
    DEFIANCE_INPUT,
    RAGE_INPUT,
    INSPIRATION_INPUT,
    MOMENTUM_INPUT,
    INTENSITY_INPUT,
    SPELLS_INPUT,
    NEARBY_INPUT,
    TOTEMS_INPUT,
    CORPSES_INPUT,
    RESONANCE_INPUT,
    BRANDS_INPUT,
    super::buffs::STATIONARY_INPUT,
    MANA_SPENT_INPUT,
    SOUL_EATER_INPUT,
    GALE_FORCE_INPUT,
    PRESENCE_INPUT,
];

/// The seconds the enemy has been in your presence (`ConfigOptions.lua`
/// 1768: the enemy's `Multiplier:EnemyPresenceSeconds` BASE; close-out M2).
pub const PRESENCE_INPUT: &str = "enemyMultiplierEnemyPresenceSeconds";

/// The config's Gale Force (`ConfigOptions.lua` 925: `Multiplier:GaleForce`
/// BASE in combat with `Condition:CanGainGaleForce`; P6-11ia).
pub const GALE_FORCE_INPUT: &str = "multiplierGaleForce";

/// The config's eaten souls (`ConfigOptions.lua` 986:
/// `Multiplier:SoulEaterStack` BASE in combat; P6-11hf).
pub const SOUL_EATER_INPUT: &str = "multiplierSoulEater";

/// The attack and cast speed one eaten soul gives (`CalcPerform.lua` 937,
/// 938: `Speed` INC 5 per `SoulEaterStack`).
const SOUL_EATER_SPEED: f64 = 5.0;

/// The config's mana spent recently (`ConfigOptions.lua` 1358:
/// `Multiplier:ManaSpentRecently`; P6-11gx).
pub const MANA_SPENT_INPUT: &str = "multiplierManaSpentRecently";

/// The config's count of brands attached to the enemy, when not the
/// maximum (`ConfigOptions.lua` 323: `Multiplier:ConfigBrandsAttachedToEnemy`).
pub const BRANDS_INPUT: &str = "BrandsAttachedToEnemy";

/// The active brand limit's base (`CalcSetup.lua` 529).
pub(super) const ACTIVE_BRAND_LIMIT: f64 = 3.0;

/// The config's count of nearby corpses (`ConfigOptions.lua` 1125:
/// `Multiplier:NearbyCorpse` BASE in combat).
pub const CORPSES_INPUT: &str = "multiplierNearbyCorpse";

/// The most energy shield regenerated per second from nearby corpses, in
/// percent per minute (2% per second; P6-11ev).
const CORPSE_ES_REGEN_LIMIT: f64 = 120.0;

/// The config's count of summoned totems, when not the maximum
/// (`ConfigOptions.lua` 1112: `TotemsSummoned` OVERRIDE in combat).
pub const TOTEMS_INPUT: &str = "TotemsSummoned";

/// Whether `input` is one this module reads.
#[must_use]
pub fn is_input(input: &str) -> bool {
    INPUTS.contains(&input)
}

/// A count option (`BuildModList`: the input, else the placeholder; a
/// count applies when not 0, a `countAllowZero` also when 0): `Ok(None)`
/// when it does not apply, `Err(())` for a value that is not a finite
/// number.
pub(super) fn count(
    set: Option<&ConfigSet>,
    name: &str,
    allow_zero: bool,
) -> Result<Option<f64>, ()> {
    let Some(set) = set else { return Ok(None) };
    let read = |kind| match last(set, name, kind) {
        None => Ok(None),
        Some(ConfigValue::Number(n)) if n.value().0.is_finite() => {
            Ok(Some(n.value().0).filter(|v| allow_zero || *v != 0.0))
        }
        Some(_) => Err(()),
    };
    match read(ConfigInputKind::Input)? {
        Some(v) => Ok(Some(v)),
        None => read(ConfigInputKind::Placeholder),
    }
}

/// The `Base` sum of the unconditional effects on `Stack(kind, value)` so
/// far, and the first one's source. A flag counts when the sum is not 0:
/// rage's flags have a description that shows them only for a value of at
/// least 1. A flag whose description shows it for any value (the import
/// then assumes 0, as Convergence's, `poe1::buffs`) must be read by
/// presence instead.
fn sum(ingest: &Ingest<'_>, kind: StackKind, value: StackValue) -> (f64, Option<Source>) {
    let mut total = 0.0;
    let mut first = None;
    for e in ingest.effects() {
        if e.quantity == Quantity::Poe1(Poe1Quantity::Stack(kind, value))
            // The rage effect's increases (P6-11dt) are `Increased` from a
            // buff, `Base` from a line: both are the percent summed.
            && matches!(e.op, Op::Base | Op::Flag | Op::Increased)
            && e.conditions == Conditions::none()
        {
            total += e.value;
            first.get_or_insert_with(|| e.source.clone());
        }
    }
    (total, first)
}

/// PoB's multiplier: `floor(value + 0.0001)`.
fn multiplier(value: f64) -> f64 {
    (value + 0.0001).floor()
}

/// The rage stack the config sets (`Multiplier:RageStack`): the input when
/// rage can be gained, else 0. `Err` for an input of another type.
fn rage_stack(set: Option<&ConfigSet>, ingest: &Ingest<'_>) -> Result<f64, ()> {
    if sum(ingest, StackKind::Rage, StackValue::CanGain).0 == 0.0 {
        return Ok(0.0);
    }
    Ok(count(set, RAGE_INPUT, false)?.unwrap_or(0.0))
}

/// The source that makes the enemy Intimidated through rage, if any
/// (module docs): read by [`super::damage_taken`].
#[must_use]
pub fn intimidates(set: Option<&ConfigSet>, ingest: &Ingest<'_>) -> Option<Source> {
    let (flag, source) = sum(ingest, StackKind::Rage, StackValue::Intimidate);
    if flag == 0.0 {
        return None;
    }
    // A rage input of another type is a reason of [`stacks`].
    (rage_stack(set, ingest).ok()? >= 1.0)
        .then_some(source)
        .flatten()
}

/// The source of Gathering Winds' Gale Force stat, if any (P6-11ia): its
/// node also grants Tailwind in PoB, read by [`super::buffs`].
#[must_use]
pub fn grants_tailwind(ingest: &Ingest<'_>) -> Option<Source> {
    ingest
        .effects()
        .iter()
        .find(|e| {
            e.quantity
                == Quantity::Poe1(Poe1Quantity::Stack(
                    StackKind::GaleForce,
                    StackValue::Per(PerStack::TailwindEffect),
                ))
                && e.conditions == Conditions::none()
        })
        .map(|e| e.source.clone())
}

/// Adds the effects of the stacks (module docs): after every other source.
pub fn stacks(set: Option<&ConfigSet>, data: &dyn CalcData, ingest: &mut Ingest<'_>) {
    let reason = |input: &str| UnsupportedReason::UnmodelledConfig {
        input: input.to_owned(),
    };
    let config = |input: &str| Source::Config {
        input: input.to_owned(),
    };
    let per = |ingest: &Ingest<'_>, kind: StackKind| -> Vec<Effect> {
        ingest
            .effects()
            .iter()
            .filter(|e| {
                matches!(
                    e.quantity,
                    Quantity::Poe1(Poe1Quantity::Stack(k, StackValue::Per(_))) if k == kind
                ) && e.op == Op::Base
                    && e.conditions == Conditions::none()
            })
            .cloned()
            .collect()
    };
    let constant = |ingest: &mut Ingest<'_>, stat: &str| -> Option<f64> {
        match data.character_constant(stat) {
            #[expect(clippy::cast_precision_loss, reason = "a small integer, exact as f64")]
            Some(v) => Some(v as f64),
            None => {
                ingest.indeterminate(IndeterminateReason::MissingData {
                    table: "character_constants",
                    key: stat.to_owned(),
                });
                None
            }
        }
    };

    // Rage.
    if sum(ingest, StackKind::Rage, StackValue::CanGain).0 != 0.0 {
        match rage_stack(set, ingest) {
            Err(()) => ingest.reason(reason(RAGE_INPUT)),
            Ok(stack) => {
                if let Some(base) = constant(ingest, "maximum_rage") {
                    let max = (base + sum(ingest, StackKind::Rage, StackValue::Maximum).0).floor();
                    let stacks = stack.min(max).max(0.0);
                    // `floor(stacks x calcLib.mod(RageEffect))` (P6-11dt: the
                    // increases; no `RageEffect` MORE is mapped).
                    let inc = sum(ingest, StackKind::Rage, StackValue::Effect).0;
                    let effect = (stacks * (1.0 + inc / 100.0)).floor();
                    // "Every Rage also grants ..." (P6-11dt): PoB's
                    // `Multiplier:RageEffect`, the effect.
                    let effects = per(ingest, StackKind::Rage);
                    if !effects.is_empty() {
                        apply(ingest, &effects, multiplier(effect), None);
                    }
                    if effect != 0.0 {
                        ingest.effect(Effect {
                            quantity: Quantity::Damage,
                            op: Op::More,
                            value: effect,
                            conditions: Conditions::none().with_all(Flags::ATTACK),
                            source: config(RAGE_INPUT),
                        });
                    }
                }
            }
        }
    } else if sum(ingest, StackKind::Rage, StackValue::Intimidate).0 != 0.0 {
        // Without `CanGainRage` the config's stack does not apply; an input
        // of another type is still a reason.
        if count(set, RAGE_INPUT, false).is_err() {
            ingest.reason(reason(RAGE_INPUT));
        }
    }

    // Inspiration charges.
    let effects = per(ingest, StackKind::Inspiration);
    if !effects.is_empty() {
        match count(set, INSPIRATION_INPUT, true) {
            Err(()) => ingest.reason(reason(INSPIRATION_INPUT)),
            Ok(overridden) => {
                let n = match overridden {
                    Some(n) => Some(n),
                    None => constant(ingest, "maximum_righteous_charges").map(|b| {
                        (b + sum(ingest, StackKind::Inspiration, StackValue::Maximum).0).max(0.0)
                    }),
                };
                if let Some(n) = n {
                    apply(ingest, &effects, multiplier(n), None);
                }
            }
        }
    }

    // Momentum.
    let effects = per(ingest, StackKind::Momentum);
    let (max, _) = sum(ingest, StackKind::Momentum, StackValue::Maximum);
    if !effects.is_empty() && max > 0.0 {
        match count(set, MOMENTUM_INPUT, false) {
            Err(()) => ingest.reason(reason(MOMENTUM_INPUT)),
            Ok(configured) => {
                let (extra, _) = sum(ingest, StackKind::Momentum, StackValue::Extra);
                let n = configured.map_or(((max + extra) / 2.0).min(max), |c| c.min(max));
                apply(ingest, &effects, multiplier(n), None);
            }
        }
    }

    // Intensity.
    let effects = per(ingest, StackKind::Intensity);
    if !effects.is_empty() {
        match count(set, INTENSITY_INPUT, false) {
            Err(()) => ingest.reason(reason(INTENSITY_INPUT)),
            Ok(None) => {}
            Ok(Some(n)) => {
                let limit = 3.0 + sum(ingest, StackKind::Intensity, StackValue::Maximum).0;
                apply(ingest, &effects, multiplier(n).min(limit), None);
            }
        }
    }

    // Non-instant spells cast recently.
    let effects = per(ingest, StackKind::NonInstantSpells);
    if !effects.is_empty() {
        match count(set, SPELLS_INPUT, false) {
            Err(()) => ingest.reason(reason(SPELLS_INPUT)),
            Ok(None) => {}
            Ok(Some(n)) => apply(ingest, &effects, multiplier(n), None),
        }
    }

    // Hits taken recently (P6-11cg): the config's count, each effect's total
    // at most 5.
    let effects = per(ingest, StackKind::HitsTakenRecently);
    if !effects.is_empty() {
        let input = super::conditions::HIT_RECENTLY_COUNT_INPUT;
        match count(set, input, false) {
            Err(()) => ingest.reason(reason(input)),
            Ok(None) => {}
            Ok(Some(n)) => {
                let n = multiplier(n);
                for e in &effects {
                    let Quantity::Poe1(Poe1Quantity::Stack(_, StackValue::Per(p))) = e.quantity
                    else {
                        continue;
                    };
                    let value = (e.value * n).min(HITS_TAKEN_LIMIT_TOTAL);
                    if value == 0.0 {
                        continue;
                    }
                    let (quantity, op, all, any) = p.target();
                    ingest.effect(Effect {
                        quantity,
                        op,
                        value,
                        conditions: Conditions::none().with_all(all).with_any(any),
                        source: e.source.clone(),
                    });
                }
            }
        }
    }

    // Trinity's resonance (P6-11gd): the config's count, at most 50 (0
    // applies nothing); more elemental damage per 5, and the penetration
    // from 25.
    let effects = per(ingest, StackKind::Resonance);
    if !effects.is_empty() {
        match count(set, RESONANCE_INPUT, false) {
            Err(()) => ingest.reason(reason(RESONANCE_INPUT)),
            Ok(None) => {}
            Ok(Some(n)) => resonance(ingest, &effects, n.clamp(0.0, RESONANCE_LIMIT)),
        }
    }

    // Soul Eater (P6-11hf; `CalcPerform.lua` 935 to 938): with
    // `CanHaveSoulEater`, 5% increased attack and cast speed per eaten soul,
    // the config's count at most `SoulEaterMax` (the character's
    // `soul_eater_maximum_stacks` plus "+# to maximum number of Eaten
    // Souls"; the multiplier's `limit`).
    let (can, _) = sum(ingest, StackKind::SoulEater, StackValue::CanGain);
    if can != 0.0 {
        match count(set, SOUL_EATER_INPUT, false) {
            Err(()) => ingest.reason(reason(SOUL_EATER_INPUT)),
            Ok(None) => {}
            Ok(Some(n)) => {
                if let Some(base) = constant(ingest, "soul_eater_maximum_stacks") {
                    let max = base + sum(ingest, StackKind::SoulEater, StackValue::Maximum).0;
                    let souls = multiplier(n).min(max);
                    if souls != 0.0 {
                        let source = config(SOUL_EATER_INPUT);
                        for flags in [Flags::ATTACK, Flags::CAST] {
                            ingest.effect(Effect {
                                quantity: Quantity::Speed,
                                op: Op::Increased,
                                value: SOUL_EATER_SPEED * souls,
                                conditions: Conditions::none().with_all(flags),
                                source: source.clone(),
                            });
                        }
                    }
                }
            }
        }
    }

    // Gale Force (P6-11ia): with `CanGainGaleForce` (the Tailwind line's
    // stat, or the flag), each effect per Gale Force times the config's
    // count (PoB applies no maximum: `MaximumGaleForce` is read by nothing).
    let effects = per(ingest, StackKind::GaleForce);
    let can = sum(ingest, StackKind::GaleForce, StackValue::CanGain).0 != 0.0
        || grants_tailwind(ingest).is_some();
    if can && !effects.is_empty() {
        match count(set, GALE_FORCE_INPUT, false) {
            Err(()) => ingest.reason(reason(GALE_FORCE_INPUT)),
            Ok(None) => {}
            Ok(Some(n)) => apply(ingest, &effects, multiplier(n), None),
        }
    }

    // Warcries used and corpses consumed recently, the enemy's presence
    // (close-out M2): each count (read by `poe1::conditions` too, for the
    // conditions they imply), at most the line's `limit` (50 corpses, 100
    // seconds).
    for (kind, input, limit) in [
        (
            StackKind::WarcriesUsedRecently,
            super::conditions::WARCRY_RECENTLY_COUNT_INPUT,
            None,
        ),
        (
            StackKind::CorpsesConsumedRecently,
            super::conditions::CORPSES_CONSUMED_INPUT,
            Some(50.0),
        ),
        (StackKind::EnemyPresenceSeconds, PRESENCE_INPUT, Some(100.0)),
        // Defiance (open items S7): the config's count, at most 10
        // (`ConfigOptions.lua` 1059 to 1060).
        (StackKind::Defiance, DEFIANCE_INPUT, Some(DEFIANCE_MAX)),
    ] {
        let effects = per(ingest, kind);
        if effects.is_empty() {
            continue;
        }
        match count(set, input, false) {
            Err(()) => ingest.reason(reason(input)),
            Ok(None) => {}
            Ok(Some(n)) => {
                let n = multiplier(n);
                let n = limit.map_or(n, |l: f64| n.min(l));
                apply(ingest, &effects, n, None);
            }
        }
    }

    // Mana spent recently (P6-11gx): Arcane Surge's effect per 200, up to
    // 50% (the `Multiplier`'s `div` 200, `limit` 50, `limitTotal`).
    let effects = per(ingest, StackKind::ManaSpentRecently);
    if !effects.is_empty() {
        match count(set, MANA_SPENT_INPUT, false) {
            Err(()) => ingest.reason(reason(MANA_SPENT_INPUT)),
            Ok(None) => {}
            Ok(Some(n)) => {
                let steps = (n / 200.0 + 0.0001).floor();
                for e in &effects {
                    let value = (e.value * steps).min(50.0);
                    if value == 0.0 {
                        continue;
                    }
                    let Quantity::Poe1(Poe1Quantity::Stack(_, StackValue::Per(p))) = e.quantity
                    else {
                        continue;
                    };
                    let (quantity, op, all, any) = p.target();
                    ingest.effect(Effect {
                        quantity,
                        op,
                        value,
                        conditions: Conditions::none().with_all(all).with_any(any),
                        source: e.source.clone(),
                    });
                }
            }
        }
    }

    // Seconds stationary (P6-11gr): the config's count (`Multiplier:
    // StationarySeconds`; `Stationary` above 0); armour per second up to
    // 100% (the multiplier's `limit` 100 / value).
    let effects = per(ingest, StackKind::StationarySeconds);
    if !effects.is_empty() {
        let input = super::buffs::STATIONARY_INPUT;
        match count(set, input, false) {
            Err(()) => ingest.reason(reason(input)),
            Ok(None) => {}
            Ok(Some(n)) => {
                let n = multiplier(n);
                for e in &effects {
                    if e.value == 0.0 || n <= 0.0 {
                        continue;
                    }
                    let value = e.value * n.min(100.0 / e.value);
                    let Quantity::Poe1(Poe1Quantity::Stack(_, StackValue::Per(p))) = e.quantity
                    else {
                        continue;
                    };
                    let (quantity, op, all, any) = p.target();
                    ingest.effect(Effect {
                        quantity,
                        op,
                        value,
                        conditions: Conditions::none().with_all(all).with_any(any),
                        source: e.source.clone(),
                    });
                }
            }
        }
    }

    // Nearby enemies (P6-11v): both counts, the rare or unique one also
    // when 0 (`countAllowZero`).
    let effects = per(ingest, StackKind::NearbyEnemies);
    if !effects.is_empty() {
        let rare_input = super::conditions::NEARBY_RARE_OR_UNIQUE_INPUT;
        match (
            count(set, NEARBY_INPUT, false),
            count(set, rare_input, true),
        ) {
            (Err(()), _) => ingest.reason(reason(NEARBY_INPUT)),
            (_, Err(())) => ingest.reason(reason(rare_input)),
            (Ok(nearby), Ok(rare)) => {
                let n = multiplier(nearby.unwrap_or(0.0) + rare.unwrap_or(0.0));
                let only_one = nearby == Some(1.0);
                nearby_enemies(ingest, &effects, n, only_one);
            }
        }
    }
}

/// The nearby enemies' effects (module docs): `count` enemies, and whether
/// only one is nearby.
fn nearby_enemies(ingest: &mut Ingest<'_>, effects: &[Effect], count: f64, only_one: bool) {
    for e in effects {
        let Quantity::Poe1(Poe1Quantity::Stack(_, StackValue::Per(p))) = e.quantity else {
            continue;
        };
        let value = match p {
            PerStack::PhysicalReductionUpTo8 | PerStack::MovementSpeedUpTo8 => {
                (e.value * count).min(NEARBY_LIMIT_TOTAL)
            }
            PerStack::PhysicalReductionIfOnlyOne if only_one => e.value * count.min(1.0),
            // P6-11gr: per enemy, while at close range.
            PerStack::AttackSpeedAtCloseRange => e.value * count,
            _ => 0.0,
        };
        if value == 0.0 {
            continue;
        }
        let (quantity, op, all, any) = p.target();
        let mut conditions = Conditions::none().with_all(all).with_any(any);
        if p == PerStack::AttackSpeedAtCloseRange {
            conditions = conditions.with_state(
                crate::condition::ConditionKey::new(super::conditions::AT_CLOSE_RANGE),
                true,
            );
        }
        ingest.effect(Effect {
            quantity,
            op,
            value,
            conditions,
            source: e.source.clone(),
        });
    }
}

/// Adds the effects per aura or herald skill affecting the character
/// (P6-11s, module docs): the count from the skills, after every source of
/// such effects.
pub fn aura_or_herald(input: &crate::input::CalcInput<'_>, ingest: &mut Ingest<'_>) {
    let of = |ingest: &Ingest<'_>, kind: StackKind| -> Vec<Effect> {
        ingest
            .effects()
            .iter()
            .filter(|e| {
                matches!(
                    e.quantity,
                    Quantity::Poe1(Poe1Quantity::Stack(k, StackValue::Per(_))) if k == kind
                ) && e.op == Op::Base
                    && e.conditions == Conditions::none()
            })
            .cloned()
            .collect()
    };
    let both = of(ingest, StackKind::AuraOrHerald);
    let heralds_only = of(ingest, StackKind::Herald);
    if both.is_empty() && heralds_only.is_empty() {
        return;
    }
    let (heralds, auras) = super::gems::herald_and_aura_counts(input);
    #[expect(
        clippy::cast_precision_loss,
        reason = "a small count of skills, exact as f64"
    )]
    let (heralds, auras) = (heralds as f64, auras as f64);
    apply(ingest, &both, multiplier(heralds + auras), None);
    apply(ingest, &heralds_only, multiplier(heralds), None);
}

/// Adds the effects per summoned totem (P6-11ep; PoB's `TotemsSummoned`,
/// `CalcPerform.lua` 1334 to 1338, read by `PerStat` tags): the config's
/// count, else, when any active skill is a totem
/// ([`super::gems::any_totem_skill`]), the main skill's limit: the
/// character's `base_number_of_totems_allowed`, "+# to maximum number of
/// Summoned Totems", and, for a main skill of `TotemsAreBallistae` and
/// `Attack`, the ballista limit (`ActiveTotemLimit` and
/// `ActiveBallistaLimit` summed). 0 otherwise, as PoB's unset output.
pub fn totems(
    input: &crate::input::CalcInput<'_>,
    set: Option<&ConfigSet>,
    ingest: &mut Ingest<'_>,
) {
    let effects: Vec<Effect> = ingest
        .effects()
        .iter()
        .filter(|e| {
            matches!(
                e.quantity,
                Quantity::Poe1(Poe1Quantity::Stack(StackKind::Totems, StackValue::Per(_)))
            ) && e.op == Op::Base
                && e.conditions == Conditions::none()
        })
        .cloned()
        .collect();
    if effects.is_empty() {
        return;
    }
    let configured = match count(set, TOTEMS_INPUT, false) {
        Ok(c) => c,
        Err(()) => {
            ingest.reason(UnsupportedReason::UnmodelledConfig {
                input: TOTEMS_INPUT.to_owned(),
            });
            return;
        }
    };
    let n = match configured {
        Some(c) => c,
        None if super::gems::any_totem_skill(input) => {
            let Some(base) = input
                .data
                .character_constant("base_number_of_totems_allowed")
            else {
                ingest.indeterminate(IndeterminateReason::MissingData {
                    table: "character_constants",
                    key: "base_number_of_totems_allowed".to_owned(),
                });
                return;
            };
            #[expect(clippy::cast_precision_loss, reason = "a small integer, exact as f64")]
            let mut limit = base as f64 + sum(ingest, StackKind::Totems, StackValue::Maximum).0;
            let levels = super::gem_levels::GemLevels::none();
            if let Ok(main) = super::skill::main_skill(input.build, input.data, &levels)
                && main.types.contains("TotemsAreBallistae")
                && main.types.contains("Attack")
            {
                limit += sum(ingest, StackKind::Totems, StackValue::BallistaMaximum).0;
            }
            limit
        }
        None => 0.0,
    };
    apply(ingest, &effects, multiplier(n), None);
}

/// Adds Trinity's effects at a lowest resonance of `n` (P6-11gd;
/// `sup_int.lua` 6592 to 6597): the more elemental damage per 5
/// (`floor(n / 5 + 0.0001)`, PoB's `div`), and the elemental penetration,
/// fire, cold, and lightning, from 25.
fn resonance(ingest: &mut Ingest<'_>, effects: &[Effect], n: f64) {
    use crate::quantity::DamageType;
    for e in effects {
        let Quantity::Poe1(Poe1Quantity::Stack(_, StackValue::Per(p))) = e.quantity else {
            continue;
        };
        let (quantity, op, all, any) = p.target();
        let conditions = Conditions::none().with_all(all).with_any(any);
        if p == PerStack::ElementalPenetrationIfAll25 {
            if n < RESONANCE_THRESHOLD {
                continue;
            }
            for t in [DamageType::Fire, DamageType::Cold, DamageType::Lightning] {
                ingest.effect(Effect {
                    quantity: Quantity::Penetration(t),
                    op,
                    value: e.value,
                    conditions: conditions.clone(),
                    source: e.source.clone(),
                });
            }
            continue;
        }
        let value = e.value * multiplier(n / RESONANCE_DIV);
        if value == 0.0 {
            continue;
        }
        ingest.effect(Effect {
            quantity,
            op,
            value,
            conditions,
            source: e.source.clone(),
        });
    }
}

/// The minion counts (P6-11gc, P6-11ie): each kind, PoB's limit output
/// its `PerStat` reads, and the gem stat that sets it (`SkillStatMap.lua`).
pub const MINION_LIMITS: [(StackKind, &str, &str); 5] = [
    (
        StackKind::Golems,
        "ActiveGolemLimit",
        "base_number_of_golems_allowed",
    ),
    (
        StackKind::Skeletons,
        "ActiveSkeletonLimit",
        "base_number_of_skeletons_allowed",
    ),
    (
        StackKind::Zombies,
        "ActiveZombieLimit",
        "base_number_of_zombies_allowed",
    ),
    (
        StackKind::RagingSpirits,
        "ActiveRagingSpiritLimit",
        "base_number_of_raging_spirits_allowed",
    ),
    (
        StackKind::Phantasms,
        "ActivePhantasmLimit",
        "base_number_of_support_ghosts_allowed",
    ),
];

/// Adds the effects per summoned minion (P6-11gc golems, P6-11ie
/// skeletons, zombies, raging spirits, phantasms; PoB's `PerStat` on the
/// limit output `CalcPerform.lua` 1412 sets): for each kind of
/// [`MINION_LIMITS`], [`super::gems::minion_limit`] with the character's
/// "+# to maximum number of" lines. 0 without a skill of that minion.
/// Spectres are not counted: PoB's list is the build's spectres, which the
/// build model does not keep, so their lines stay reasons.
pub fn minions(input: &crate::input::CalcInput<'_>, ingest: &mut Ingest<'_>) {
    for (kind, limit, stat) in MINION_LIMITS {
        let effects: Vec<Effect> = ingest
            .effects()
            .iter()
            .filter(|e| {
                matches!(
                    e.quantity,
                    Quantity::Poe1(Poe1Quantity::Stack(k, StackValue::Per(_))) if k == kind
                ) && e.op == Op::Base
                    && e.conditions == Conditions::none()
            })
            .cloned()
            .collect();
        if effects.is_empty() {
            continue;
        }
        let extra = sum(ingest, kind, StackValue::Maximum).0;
        let n = super::gems::minion_limit(input, limit, stat, extra);
        apply(ingest, &effects, multiplier(n), None);
    }
}

/// Adds the effects per nearby corpse (P6-11ev; module docs): the config's
/// `multiplierNearbyCorpse`. "With at least one nearby corpse" lines
/// (`MultiplierThreshold` 1) apply once from 1; the energy shield
/// regeneration per corpse totals at most 2% per second.
pub fn nearby_corpses(set: Option<&ConfigSet>, ingest: &mut Ingest<'_>) {
    let effects: Vec<Effect> = ingest
        .effects()
        .iter()
        .filter(|e| {
            matches!(
                e.quantity,
                Quantity::Poe1(Poe1Quantity::Stack(
                    StackKind::NearbyCorpses,
                    StackValue::Per(_)
                ))
            ) && e.op == Op::Base
                && e.conditions == Conditions::none()
        })
        .cloned()
        .collect();
    if effects.is_empty() {
        return;
    }
    let n = match count(set, CORPSES_INPUT, false) {
        Ok(c) => multiplier(c.unwrap_or(0.0)),
        Err(()) => {
            ingest.reason(UnsupportedReason::UnmodelledConfig {
                input: CORPSES_INPUT.to_owned(),
            });
            return;
        }
    };
    if n == 0.0 {
        return;
    }
    for e in effects {
        let Quantity::Poe1(Poe1Quantity::Stack(_, StackValue::Per(p))) = e.quantity else {
            continue;
        };
        let value = match p {
            PerStack::DamageMoreWithAny | PerStack::EnemyDamageWithAny => e.value,
            PerStack::EnergyShieldRegenPercentPerMinuteUpTo2 => {
                (e.value * n).min(CORPSE_ES_REGEN_LIMIT)
            }
            _ => e.value * n,
        };
        let (quantity, op, all, any) = p.target();
        ingest.effect(Effect {
            quantity,
            op,
            value,
            conditions: Conditions::none().with_all(all).with_any(any),
            source: e.source.clone(),
        });
    }
}

/// Adds the effects per brand attached to the enemy (P6-11ew; PoB's
/// `CalcPerform.lua` 1320 to 1332): for each brand skill, its attach limit
/// ([`super::gems::brand_attach_limit`]: its gems' `BrandsAttachedLimit`,
/// plus the character's, [`StackValue::AttachMaximum`])
/// at most the active brand limit (3 plus "You can Cast an additional
/// Brand"), or the config's count at most that; the largest over the brand
/// skills. 0 without one, as PoB's unset multiplier.
pub fn brands(
    input: &crate::input::CalcInput<'_>,
    set: Option<&ConfigSet>,
    ingest: &mut Ingest<'_>,
) {
    let effects: Vec<Effect> = ingest
        .effects()
        .iter()
        .filter(|e| {
            matches!(
                e.quantity,
                Quantity::Poe1(Poe1Quantity::Stack(
                    StackKind::BrandsAttached,
                    StackValue::Per(_)
                ))
            ) && e.op == Op::Base
                && e.conditions == Conditions::none()
        })
        .cloned()
        .collect();
    if effects.is_empty() {
        return;
    }
    let configured = match count(set, BRANDS_INPUT, false) {
        Ok(c) => c,
        Err(()) => {
            ingest.reason(UnsupportedReason::UnmodelledConfig {
                input: BRANDS_INPUT.to_owned(),
            });
            return;
        }
    };
    let Some(attach) = super::gems::brand_attach_limit(input) else {
        return;
    };
    // P6-11hv: the character's `BrandsAttachedLimit` (Runebinder) is in
    // every brand skill's list too.
    let attach = attach + sum(ingest, StackKind::BrandsAttached, StackValue::AttachMaximum).0;
    let active = ACTIVE_BRAND_LIMIT + sum(ingest, StackKind::BrandsAttached, StackValue::Maximum).0;
    let limit = attach.min(active);
    let n = match configured {
        Some(c) => c.min(limit),
        None => limit,
    };
    apply(ingest, &effects, multiplier(n), None);
}

/// The enemy's elemental ailments (the agent list, 2026-11-27): Elemental
/// Hit's base mods give `Multiplier:ElementalHitAilmentOnEnemy` 1 for each
/// of the enemy's `Ignited`, `Chilled`, `Frozen`, `Shocked`, `Scorched`,
/// `Brittle`, `Sapped` (`act_dex.lua`): the settled checks, the config's
/// scorch, brittle, and sap, and the chill and shock with their guaranteed
/// sources ([`super::damage_taken::chill`], [`super::damage_taken::shock`]).
/// After the gems and the damage taken. A value it cannot read is a reason.
pub fn enemy_elemental_ailments(
    set: Option<&ConfigSet>,
    settled: &std::collections::BTreeMap<crate::condition::ConditionKey, bool>,
    ingest: &mut Ingest<'_>,
) {
    let reason = |input: &str| UnsupportedReason::UnmodelledConfig {
        input: input.to_owned(),
    };
    let effects: Vec<Effect> = ingest
        .effects()
        .iter()
        .filter(|e| {
            matches!(
                e.quantity,
                Quantity::Poe1(Poe1Quantity::Stack(
                    StackKind::EnemyElementalAilments,
                    StackValue::Per(_)
                ))
            ) && e.op == Op::Base
                && e.conditions == Conditions::none()
        })
        .cloned()
        .collect();
    if effects.is_empty() {
        return;
    }
    let mut n = 0.0;
    // The shocked check is settled (the config's, Skitterbots'); a guaranteed
    // shock counts below.
    let shocked = settled.get(&crate::condition::ConditionKey::new(
        super::conditions::ENEMY_SHOCKED,
    )) == Some(&true);
    if shocked {
        n += 1.0;
    }
    for key in [
        super::conditions::ENEMY_IGNITED,
        super::conditions::ENEMY_FROZEN,
    ] {
        match settled.get(&crate::condition::ConditionKey::new(key)) {
            Some(true) => n += 1.0,
            Some(false) => {}
            None => {
                ingest.reason(UnsupportedReason::UnmodelledCondition {
                    condition: crate::condition::ConditionKey::new(key),
                    quantity: Quantity::Poe1(Poe1Quantity::Stack(
                        StackKind::EnemyElementalAilments,
                        StackValue::Per(PerStack::DamageMore),
                    )),
                    source: effects[0].source.clone(),
                });
                return;
            }
        }
    }
    // Chilled: the config's check (even at no configured effect, PoB's
    // condition), else a guaranteed chill.
    let chill_checked = set.is_some_and(|s| {
        matches!(
            last(
                s,
                super::damage_taken::CHILLED_INPUT,
                ConfigInputKind::Input
            ),
            Some(ConfigValue::Boolean(true))
        )
    });
    match super::damage_taken::chill(set, ingest) {
        Ok(Some(_)) => n += 1.0,
        Ok(None) if chill_checked => n += 1.0,
        Ok(None) => {}
        Err(input) => {
            ingest.reason(reason(input));
            return;
        }
    }
    let Some(set) = set else {
        apply(ingest, &effects, n, None);
        return;
    };
    match super::damage_taken::shock(set, ingest) {
        Ok(Some(_)) if !shocked => n += 1.0,
        Ok(Some(_)) => {}
        Ok(None) => {}
        Err(input) => {
            ingest.reason(reason(input));
            return;
        }
    }
    for input in [
        super::damage_taken::SCORCHED_INPUT,
        BRITTLE_INPUT,
        super::damage_taken::SAPPED_INPUT,
    ] {
        match last(set, input, ConfigInputKind::Input) {
            None | Some(ConfigValue::Boolean(false)) => {}
            Some(ConfigValue::Boolean(true)) => n += 1.0,
            Some(_) => {
                ingest.reason(reason(input));
                return;
            }
        }
    }
    apply(ingest, &effects, n, None);
}

/// The config's check of the enemy's brittle (`ConfigOptions.lua`; its
/// effect is not modelled, a reason of its own).
const BRITTLE_INPUT: &str = "conditionEnemyBrittle";

/// Each effect per stack as its target times `count`, with its source (or
/// `source` when given).
fn apply(ingest: &mut Ingest<'_>, effects: &[Effect], count: f64, source: Option<&Source>) {
    if count == 0.0 {
        return;
    }
    for e in effects {
        let Quantity::Poe1(Poe1Quantity::Stack(_, StackValue::Per(p))) = e.quantity else {
            continue;
        };
        let (quantity, op, all, any) = p.target();
        ingest.effect(Effect {
            quantity,
            op,
            value: e.value * count,
            conditions: match p.state() {
                Some(state) => Conditions::none()
                    .with_all(all)
                    .with_any(any)
                    .with_state(crate::condition::ConditionKey::new(state), true),
                None => Conditions::none().with_all(all).with_any(any),
            },
            source: source.cloned().unwrap_or_else(|| e.source.clone()),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_modifiers_should_be_sorted_and_the_inputs_not_named_only() {
        assert!(MODIFIERS.windows(2).all(|w| w[0].0 < w[1].0));
        for input in INPUTS {
            assert!(is_input(input));
            assert!(!super::super::config::named_only(input), "{input}");
        }
        assert_eq!(
            modifier("maximum_rage"),
            Some((StackKind::Rage, StackValue::Maximum))
        );
    }

    #[test]
    fn a_multiplier_should_floor_as_pob() {
        assert_eq!(multiplier(2.5), 2.0);
        assert_eq!(multiplier(2.99995), 3.0);
        assert_eq!(multiplier(0.0), 0.0);
    }
}
