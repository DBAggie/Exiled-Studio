//! Triggers (Phase 6 P6-8a): a triggered skill's rate as PoB v2.67.2
//! computes it (`Modules/CalcTriggers.lua`, `defaultTriggerHandler` 390
//! to 880 and its configs 896 to 1570).
//!
//! **Which trigger** (`calcs.triggers`, 1592 to 1617): PoB looks the config
//! up by the skill's id, its name, then the name of the trigger support
//! that applies to it (`triggeredBy`). The engine ports these configs
//! ([`Config`]):
//! - **Cast On Critical Strike** (1151): the source is the attack in the
//!   same group with the highest rate; the triggered skills are the group's
//!   spells it supports; the trigger chance is the source's hit chance
//!   times its crit chance.
//! - **Cast when Damage Taken** (1175), **Cast when Stunned** (1190), and
//!   **Cast on Ward Break** (1195): global triggers whose source is the
//!   skill itself; the rate is the trigger's cooldown cap.
//! - **Cast on Melee Kill** (1155, P6-8a part 2): as Cast On Critical
//!   Strike from the group's melee attacks, without the crit chance, only
//!   while the character has killed recently (PoB's
//!   `Condition:KilledRecently`: the config's check or its kill count);
//!   else self-cast.
//! - **Cast while Channelling** (1393, `CWCHandler` 219 to 388): the
//!   source is the group's channelling skill the support's first part
//!   supports, its interval the part's `cast_while_channelling_time_ms`
//!   (PoB's `triggerTime`); the rate is the rotation of the group's spells
//!   it triggers at `1 / interval`, without a trigger cooldown or chance.
//! - **Spellslinger** (1200, open items S3): from any wand projectile
//!   attack, on use; the main spell alone in the rotation.
//! - **Arcanist Brand** (1321, open items S3): the source is the brand's
//!   activations, computed in its own context ([`arcanist_rate`]); the
//!   rotation of the group's spells it supports, with no trigger cooldown.
//! - **Cast on Death** (1342, open items S3): PoB's config returns none;
//!   its `triggeredOnDeath` gives the skill a stand-in time of a minute
//!   ([`ON_DEATH_TIME`]) and keeps its average damage as the combined DPS
//!   (`showAverage`). Its damaging ailments, which PoB counts without the
//!   rate (`CalcOffence.lua` 4909), are not computed.
//! - **The item lines that trigger a socketed spell** (open items S3, part
//!   5; [`super::item_supports::TRIGGER_ROWS`]): PoB gives the item's slot
//!   a trigger support and finds the config by the support's name. Poet's
//!   Pen (1009, a wand attack) and Asenath's Chant (1031, a bow attack) on
//!   use; Mjolner (1109, an attack with a mace or a one-handed weapon in
//!   another group); Cospri's Malice (1125, a melee attack with a sword or
//!   a one-handed weapon, on crit); Wing of the Wyvern (1117, a bow attack
//!   in another group); Seven Teachings (1131, an unarmed melee attack, on
//!   crit; the engine computes no unarmed attack, so its source is not
//!   found); the fire spell on hit enchantment (1521, a melee attack); and
//!   Svalinn's and Festering Resentment's trigger on block (1498, 1505), a
//!   global trigger whose rotation runs at the cap. The source's weapon is
//!   the main hand's (PoB's `skillCfg.flags`). The source rate is halved
//!   while dual wielding unless the attack hits with both weapons
//!   (441 to 449). A line below 100% leaves the rate unsupported: PoB
//!   ignores the chance, the game does not.
//!
//! Mark On Hit (1275) and Hextouch (1278) trigger a mark or a hex: their
//! effects apply as any curse's ([`super::gems`]), and the engine computes
//! no field from a curse's rate, so their configs have nothing to give.
//!
//! The Holy Relic's Nova (1167, a minion skill the character's attacks
//! trigger) waits for the skills' level requirements in the bundle: PoB
//! picks a minion skill's level by them (`CalcActiveSkill.lua` 895 to
//! 902), so the minion's skill is not computed yet.
//!
//! A skill with another trigger (a trigger support or an inbuilt trigger
//! not listed) is not computed ([`Kind::Unported`]).
//!
//! **The handler** (390 to 880):
//! 1. **The source:** each candidate is computed as its own skill (PoB's
//!    `calcs.buildActiveSkill`; the engine runs [`crate::calculate`] on a
//!    copy of the build with the candidate as its main skill,
//!    [`super::skill::with_main_skill`]); the highest `HitSpeed` or `Speed`
//!    wins, the first on a tie (`findTriggerSkill`, 74 to 87). A triggered
//!    skill, a disabled one, and the triggered skill itself never are. No
//!    source: PoB reports the skill as self-cast ([`Outcome::SelfCast`]).
//! 2. **The cooldowns:** the trigger's (the trigger support's level
//!    cooldown) and the triggered skill's, each rounded up to a server
//!    tick unless the skill stores more than one use; the cap is `1 /
//!    max(both)` (`TriggerRateCap`, 575 to 585).
//! 3. **The effective source rate:** the source's rate; a global trigger's
//!    is the cap (714 to 719).
//! 4. **The trigger chance** (727 to 790): times the source's hit chance
//!    when it is an attack and not 100, times its crit chance for a trigger
//!    on crit when not 100, times the config's chance when not 100.
//! 5. **The trigger rate** (793 to 860): `min(cap, rate)` for a skill that
//!    stores more than one use and is the only one triggered; the
//!    effective rate for a global trigger; else PoB's simulation of the
//!    triggered skills' rotation ([`rotation_rate`],
//!    `calcMultiSpellRotationImpact`, 87 to 133).
//! 6. **`Speed`** is the trigger rate (`CalcOffence.lua` 2223 to 2227).
//!
//! **Not modelled:** added cooldown and added cast time (each from a stat
//! the mapping does not know, so a reason; cooldown recovery is in since
//! open items S3), Unleash, projectiles that hit several times, and
//! trigger-related damage (`TriggeredDamage`: unmapped stats).
//!
//! **Skills an item grants and triggers** (open items S3, part 7; PoB's
//! `triggerExtraSkill`): the item's line ([`GRANTED_TRIGGERS`]) and the
//! group PoB saves for the skill (`Item:<id>:<Unique>, <base>`). The config
//! is PoB's by the skill's ID, then the unique's name ([`GRANTED_BY_SKILL`],
//! [`GRANTED_BY_UNIQUE`]); the source a melee or attack skill, or a
//! damaging one for some, a bow attack on use for Lioneye's Paws; the
//! line's chance a factor, Starcaller's on crit; the skill's own cooldown,
//! 1 s for Moonbender's Wing and Lioneye's Paws; the main skill alone in
//! the rotation. Such a skill has no cost, as PoB clears it
//! ([`granted_and_triggered`]). The configs whose skill no field reads
//! (minions, Shade Form, Contaminate) are not listed.
//!
//! **Triggered attacks** (open items S3, part 6): the attack path takes a
//! ported trigger's rate as every pass's speed (`CalcOffence.lua` 2223 to
//! 2227). **Manaforged Arrows** (1378) is the first: a bow attack, on use,
//! whose rate is divided by `ceil(threshold x triggered / source)`, each
//! skill's `ManaCostRaw` ([`super::costs::mana_cost_raw`], the source's in
//! its own context), 522 to 556; a source without a cost never triggers
//! it.

use super::Poe1Quantity;
use super::skill::{self, GemEffectRef, MainSkill};
use crate::condition::Flags;
use crate::engine::Calc;
use crate::quantity::{Op, Quantity};
use crate::result::Field;
use crate::source::Source;
use crate::state::{FieldState, Reasons, UnsupportedReason};

/// The state of PoB's `TriggeredDamage` turned into `Damage` (P6-8a part
/// 2: Cast while Channelling's less damage): settled by the spell path
/// from [`Outcome::Rate`]'s flag before the damage
/// (`mapping::STATE_ROWS`).
pub const TRIGGERED_DAMAGE: &str = "TriggeredDamage";

/// PoB's `data.misc.ServerTickTime`.
const SERVER_TICK_TIME: f64 = 0.033;

/// A trigger config the engine ports (module docs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Config {
    /// Cast On Critical Strike (and its awakened gem).
    CastOnCrit,
    /// Cast when Damage Taken.
    CastWhenDamageTaken,
    /// Cast when Stunned.
    CastWhenStunned,
    /// Cast on Ward Break.
    CastOnWardBreak,
    /// Cast on Melee Kill.
    CastOnMeleeKill,
    /// Cast while Channelling (and its awakened gem).
    CastWhileChannelling,
    /// Spellslinger's support part (open items S3; `CalcTriggers.lua` 1200
    /// to 1212): the source is any wand projectile attack, the trigger on
    /// use (no hit chance), the rotation the main skill alone.
    Spellslinger,
    /// Arcanist Brand's support part (open items S3; `CalcTriggers.lua`
    /// 1321 to 1341): the source is the brand's activations, computed in
    /// its own context ([`arcanist_rate`]).
    ArcanistBrand,
    /// Cast on Death (open items S3): PoB's config returns none
    /// (`CalcTriggers.lua` 1342); its `triggeredOnDeath` sets the skill's
    /// time to a stand-in minute ([`ON_DEATH_TIME`], `CalcPerform.lua` 1438
    /// to 1447) and keeps `showAverage`.
    CastOnDeath,
    /// Poet's Pen (open items S3, part 5; 1009): a wand attack, on use; the
    /// rotation the spells of the item's slot.
    PoetsPen,
    /// Asenath's Chant (1031): a bow attack, on use; the rotation the
    /// spells of the item's slot.
    AsenathsChant,
    /// Mjolner (1109): an attack with a mace or a one-handed weapon in
    /// another group; the rotation the group's lightning spells.
    Mjolner,
    /// Cospri's Malice (1125): a melee attack with a sword or a one-handed
    /// weapon, on crit; the rotation the cold spells of the item's slot.
    CosprisMalice,
    /// Wing of the Wyvern (1117): a bow attack in another group; the
    /// rotation the group's spells.
    WingOfTheWyvern,
    /// Seven Teachings (1131): an unarmed melee attack, on crit; the
    /// rotation the group's spells.
    SevenTeachings,
    /// Svalinn's and Festering Resentment's trigger on block (1498, 1505):
    /// a global trigger whose rotation is the group's spells at the cap.
    CastOnBlock,
    /// The fire spell on hit enchantment (1521): a melee attack; the
    /// rotation the group's spells.
    FireSpellOnHit,
    /// Manaforged Arrows (open items S3; 1378): a bow attack, on use; the
    /// source's rate over the uses whose mana reaches the support's
    /// threshold of the triggered attack's (522 to 556); the main skill
    /// alone in the rotation.
    ManaforgedArrows,
    /// A skill an item grants and triggers from a melee or attack skill
    /// (open items S3, part 7; Ngamahu's Flame 973 and its Molten Burst by
    /// ID 1566, Limbsplit 947, The Cauteriser 950): the main skill alone in
    /// the rotation, its own cooldown, the line's chance.
    UniqueMeleeOrAttack,
    /// Starcaller (979): as [`Self::UniqueMeleeOrAttack`], on crit (its
    /// line's `onCrit`, `ModParser.lua` 3309).
    Starcaller,
    /// Moonbender's Wing (968): as [`Self::UniqueMeleeOrAttack`] with PoB's
    /// cooldown of 1 s.
    MoonbendersWing,
    /// A skill an item grants and triggers from a damaging or attack skill
    /// (Cameria's Avarice 976, Uul-Netol's Embrace 982, Duskblight 953,
    /// Mark of the Elder and of the Shaper 1003, 1006, Sporeguard 1000).
    UniqueDamageOrAttack,
    /// Lioneye's Paws (956): a bow attack, on use, PoB's cooldown of 1 s.
    LioneyesPaws,
}

impl Config {
    /// A trigger an item's line gives ([`super::item_supports::TRIGGER_ROWS`]):
    /// its source rate is halved while dual wielding (`CalcTriggers.lua`
    /// 441 to 449, `itemSupportTrigger`).
    const fn is_item_line(self) -> bool {
        matches!(
            self,
            Self::PoetsPen
                | Self::AsenathsChant
                | Self::Mjolner
                | Self::CosprisMalice
                | Self::WingOfTheWyvern
                | Self::SevenTeachings
                | Self::CastOnBlock
                | Self::FireSpellOnHit
        )
    }

    /// PoB's `triggerOnUse`: the source's hit chance is not a factor.
    const fn on_use(self) -> bool {
        matches!(
            self,
            Self::Spellslinger
                | Self::PoetsPen
                | Self::AsenathsChant
                | Self::ManaforgedArrows
                | Self::LioneyesPaws
        )
    }

    /// A skill an item grants and triggers (open items S3, part 7): its
    /// chance is its line's ([`GRANTED_TRIGGERS`]).
    const fn is_granted(self) -> bool {
        matches!(
            self,
            Self::UniqueMeleeOrAttack
                | Self::Starcaller
                | Self::MoonbendersWing
                | Self::UniqueDamageOrAttack
                | Self::LioneyesPaws
        )
    }

    /// PoB's `skillData.cooldown = 1` for the skill the item grants (959,
    /// 970): the normal gem's, which has none.
    const fn cooldown_override(self) -> Option<f64> {
        match self {
            Self::MoonbendersWing | Self::LioneyesPaws => Some(1.0),
            _ => None,
        }
    }

    /// The config has no `triggeredSkillCond`: PoB simulates the main
    /// skill alone (809).
    const fn main_alone(self) -> bool {
        matches!(self, Self::Spellslinger | Self::ManaforgedArrows) || self.is_granted()
    }

    /// PoB's `triggerOnCrit` (the support's `SkillData` or stat): the
    /// source's crit chance is a factor.
    const fn on_crit(self) -> bool {
        matches!(
            self,
            Self::CastOnCrit | Self::CosprisMalice | Self::SevenTeachings | Self::Starcaller
        )
    }

    /// The rotation's skills share the main skill's slot (PoB's
    /// `socketGroup.slot`) rather than its group (`slotMatch`, 32 to 38).
    const fn rotation_by_slot(self) -> bool {
        matches!(
            self,
            Self::PoetsPen | Self::AsenathsChant | Self::CosprisMalice
        )
    }
}

/// PoB's stand-in time of a skill Cast on Death triggers: `60 x 1000`
/// (`CalcPerform.lua` 1447, "technically any large value would do"), read
/// as seconds by `CalcOffence.lua` 2214 to 2221 over the skill's
/// `CooldownRecovery` INC.
const ON_DEATH_TIME: f64 = 60.0 * 1000.0;

/// The trigger supports ported, by the granted effect PoB marks
/// `isTrigger` (`Data/Skills/sup_*.lua`).
const SUPPORTS: &[(&str, Config)] = &[
    ("SupportCastOnCritTriggered", Config::CastOnCrit),
    ("SupportCastOnCritTriggeredPlus", Config::CastOnCrit),
    ("SupportCastWhenDamageTaken", Config::CastWhenDamageTaken),
    ("SupportCastWhenStunned", Config::CastWhenStunned),
    ("SupportCastOnWardBreak", Config::CastOnWardBreak),
    ("SupportCastOnMeleeKillTriggered", Config::CastOnMeleeKill),
    (
        "SupportCastWhileChannellingTriggered",
        Config::CastWhileChannelling,
    ),
    (
        "SupportCastWhileChannellingTriggeredPlus",
        Config::CastWhileChannelling,
    ),
    ("SupportSpellslinger", Config::Spellslinger),
    ("SupportBrandSupport", Config::ArcanistBrand),
    ("SupportCastOnDeath", Config::CastOnDeath),
    ("SupportManaforgedArrows", Config::ManaforgedArrows),
    // The item lines' supports (open items S3, part 5), by PoB's names.
    ("SupportTriggerSpellOnAttack", Config::PoetsPen),
    ("SupportTriggerSpellOnBowAttack", Config::AsenathsChant),
    (
        "SupportUniqueMjolnerLightningSpellsCastOnHit",
        Config::Mjolner,
    ),
    (
        "SupportUniqueCosprisMaliceColdSpellsCastOnMeleeCriticalStrike",
        Config::CosprisMalice,
    ),
    (
        "SupportTriggerSpellOnBowAttackFreezeHit",
        Config::WingOfTheWyvern,
    ),
    (
        "SupportTriggerSpellOnUnarmedMeleeCriticalHit",
        Config::SevenTeachings,
    ),
    ("SupportTriggerElementalSpellOnBlock", Config::CastOnBlock),
    (
        "SupportUniqueVirulenceSpellsCastOnBlock",
        Config::CastOnBlock,
    ),
    ("SupportTriggerFireSpellOnHit", Config::FireSpellOnHit),
];

/// Arcanist Brand's active skill (`act_int.lua` 545): the source of the
/// spells its support part triggers.
const ARCANIST_SKILL: &str = "BrandSupport";

/// A brand's activation period, milliseconds (PoB's `repeatFrequency`,
/// `div = 1000`, `SkillStatMap.lua` 2161).
const REPEAT_STAT: &str = "base_sigil_repeat_frequency_ms";

/// A brand's own attach limit (PoB's `BrandsAttachedLimit` BASE,
/// `SkillStatMap.lua` 2155).
const ATTACH_STAT: &str = "base_number_of_sigils_allowed_per_target";

/// The state of Arcanist Brand's more damage against the branded enemy
/// (`TriggeredDamage` MORE with `Condition:TargetingBrandedEnemy`): settled
/// by the spell path with [`TRIGGERED_DAMAGE`] and the config's
/// `targetBrandedEnemy` (on unless set off, `ConfigOptions.lua` 326).
pub const TRIGGERED_DAMAGE_BRANDED: &str = "TriggeredDamage:TargetingBrandedEnemy";

/// The config check that the skill targets the branded enemy, on by
/// default ([`TRIGGERED_DAMAGE_BRANDED`]).
const TARGET_BRANDED_INPUT: &str = "targetBrandedEnemy";

/// Whether the config has the skill target the branded enemy: PoB's
/// `defaultState = true` check, saved only when off; `None` for a value of
/// another type.
#[must_use]
pub fn targeting_branded_enemy(input: &crate::input::CalcInput<'_>) -> Option<bool> {
    let set = super::collect::active_config(input.build);
    match set.and_then(|s| {
        super::config::last_input(s, TARGET_BRANDED_INPUT, loom_domain::ConfigInputKind::Input)
    }) {
        None | Some(loom_domain::ConfigValue::Boolean(true)) => Some(true),
        Some(loom_domain::ConfigValue::Boolean(false)) => Some(false),
        Some(_) => None,
    }
}

/// The item lines that grant a skill and trigger it (open items S3, part 7;
/// PoB's `triggerExtraSkill`, `ModParser.lua` 2054 to 2071 and 3278 to
/// 3335): `(stat, chance, on crit)`, sorted. `chance`: the stat's value is
/// the chance to trigger (PoB's `triggerChance`; 100 when the text has no
/// "% chance"); else its value is the skill's level. The skill acts through
/// the group PoB saves for it (`source` `Item:<id>:<Unique>, <base>`).
pub const GRANTED_TRIGGERS: [(&str, bool, bool); 9] = [
    (
        "local_display_attack_with_level_X_bone_nova_on_bleeding_enemy_kill",
        false,
        false,
    ),
    ("local_display_molten_burst_on_melee_hit_%", true, false),
    ("local_display_starfall_on_melee_critical_hit_%", true, true),
    (
        "local_display_trigger_level_x_gore_shockwave_on_melee_hit_with_atleast_150_strength",
        false,
        false,
    ),
    (
        "local_display_trigger_level_x_icicle_nova_on_hit_vs_frozen_enemy",
        false,
        false,
    ),
    (
        "local_display_trigger_level_x_lightning_warp_on_hit_with_this_weapon",
        false,
        false,
    ),
    (
        "local_display_trigger_level_x_rain_of_arrows_on_bow_attack",
        false,
        false,
    ),
    (
        "local_display_trigger_level_x_stalking_pustule_on_kill",
        false,
        false,
    ),
    (
        "local_display_trigger_tentacle_smash_on_kill_%_chance",
        true,
        false,
    ),
];

/// The row of `stat` in [`GRANTED_TRIGGERS`].
#[must_use]
pub fn granted_trigger(stat: &str) -> Option<&'static (&'static str, bool, bool)> {
    GRANTED_TRIGGERS
        .binary_search_by(|(id, _, _)| (*id).cmp(stat))
        .ok()
        .map(|i| &GRANTED_TRIGGERS[i])
}

/// PoB's configs of the skills items grant and trigger, by the skill's ID
/// first (`configTable[skillId]`, 1602), then by the unique's name in the
/// group's source (`getUniqueItemTriggerName`, 1575 to 1590).
const GRANTED_BY_SKILL: &[(&str, Config)] =
    &[("TriggeredMoltenStrike", Config::UniqueMeleeOrAttack)];

/// The unique configs, by PoB's lowercase name ([`GRANTED_BY_SKILL`]).
const GRANTED_BY_UNIQUE: &[(&str, Config)] = &[
    ("cameria's avarice", Config::UniqueDamageOrAttack),
    ("duskblight", Config::UniqueDamageOrAttack),
    ("limbsplit", Config::UniqueMeleeOrAttack),
    ("lioneye's paws", Config::LioneyesPaws),
    ("mark of the elder", Config::UniqueDamageOrAttack),
    ("mark of the shaper", Config::UniqueDamageOrAttack),
    ("moonbender's wing", Config::MoonbendersWing),
    ("ngamahu's flame", Config::UniqueMeleeOrAttack),
    ("sporeguard", Config::UniqueDamageOrAttack),
    ("starcaller", Config::Starcaller),
    ("the cauteriser", Config::UniqueMeleeOrAttack),
    ("uul-netol's embrace", Config::UniqueDamageOrAttack),
];

/// The item and the unique's name of an item's group `source`
/// (`Item:<id>:<Unique>, <base>`; PoB's `.*:.*:(.*),.*`).
fn granted_source(source: &str) -> Option<(u64, &str)> {
    let rest = source.strip_prefix("Item:")?;
    let (id, rest) = rest.split_once(':')?;
    let (name, _) = rest.rsplit_once(',')?;
    Some((id.parse().ok()?, name))
}

/// The config of a skill an item grants (`skill.source`), if PoB has one.
fn granted_config(skill: &MainSkill<'_>) -> Option<Config> {
    if !skill.item_granted {
        return None;
    }
    if let Some((_, c)) = GRANTED_BY_SKILL
        .iter()
        .find(|(id, _)| *id == skill.effect().id)
    {
        return Some(*c);
    }
    let (_, name) = granted_source(skill.source.as_deref()?)?;
    let name = name.to_lowercase();
    GRANTED_BY_UNIQUE
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, c)| *c)
}

/// The chance, percent, of the line on the main skill's item that grants
/// and triggers it ([`GRANTED_TRIGGERS`]); `None` when the item has no
/// such line (PoB: the skill is not triggered, `srcInstance.triggered`).
fn granted_chance(calc: &Calc<'_>, skill: &MainSkill<'_>) -> Option<f64> {
    let (item, _) = granted_source(skill.source.as_deref()?)?;
    let q = Quantity::Poe1(Poe1Quantity::GrantedTriggerChance);
    calc.store()
        .effects()
        .iter()
        .filter(|e| e.quantity == q)
        .filter(|e| matches!(&e.source, Source::Item { item: i, .. } if u64::from(i.0) == item))
        .map(|e| if e.op == Op::Base { e.value } else { 100.0 })
        .reduce(f64::min)
}

/// Whether `skill` is a skill an item grants and triggers (open items S3,
/// part 7): an item's group with the item's line ([`GRANTED_TRIGGERS`]).
/// PoB clears such a skill's cost (`SkillsTab.lua` 1161 to 1165), as the
/// game gives it none.
#[must_use]
pub(super) fn granted_and_triggered(calc: &Calc<'_>, skill: &MainSkill<'_>) -> bool {
    skill.item_granted && granted_chance(calc, skill).is_some()
}

/// The first parts of the two-part trigger gems, on the source skill:
/// Cast On Critical Strike's and Cast on Melee Kill's on the attack, Cast
/// while Channelling's on the channelling skill.
const SOURCE_PARTS: &[(&str, Config)] = &[
    ("SupportAwakenedCastOnCriticalStrike", Config::CastOnCrit),
    (
        "SupportAwakenedCastWhileChannelling",
        Config::CastWhileChannelling,
    ),
    ("SupportCastOnCriticalStrike", Config::CastOnCrit),
    ("SupportCastOnMeleeKill", Config::CastOnMeleeKill),
    ("SupportCastWhileChannelling", Config::CastWhileChannelling),
];

/// Cast while Channelling's interval on the channelling skill (PoB's
/// `triggerTime`, `div = 1000`, `Data/Skills/sup_int.lua` 689).
const CWC_TIME_STAT: &str = "cast_while_channelling_time_ms";

/// PoB's trigger supports (`isTrigger = true`, `Data/Skills/sup_*.lua`):
/// a skill one of them supports is triggered.
const TRIGGER_SUPPORTS: &[&str] = &[
    "SupportBrandSupport",
    "SupportCastOnCritTriggered",
    "SupportCastOnCritTriggeredPlus",
    "SupportCastOnDeath",
    "SupportCastOnMeleeKillTriggered",
    "SupportCastOnWardBreak",
    "SupportCastWhenDamageTaken",
    "SupportCastWhenStunned",
    "SupportCastWhileChannellingTriggered",
    "SupportCastWhileChannellingTriggeredPlus",
    "SupportCurseOnHitCurse",
    "SupportCurseOnHitCursePlus",
    "SupportManaforgedArrows",
    "SupportMarkOnHit",
    "SupportSpellslinger",
    "SupportTriggerElementalSpellOnBlock",
    "SupportTriggerFireSpellOnHit",
    "SupportTriggerSpellOnAttack",
    "SupportTriggerSpellOnBowAttack",
    "SupportTriggerSpellOnBowAttackFreezeHit",
    "SupportTriggerSpellOnUnarmedMeleeCriticalHit",
    "SupportUniqueCosprisMaliceColdSpellsCastOnMeleeCriticalStrike",
    "SupportUniqueMjolnerLightningSpellsCastOnHit",
    "SupportUniqueVirulenceSpellsCastOnBlock",
];

/// Manaforged Arrows' threshold, percent of the triggered attack's mana
/// cost (PoB's `ManaForgedArrowsPercentThreshold`, `div = 100`,
/// `Data/Skills/sup_dex.lua` 2911).
const MANAFORGED_THRESHOLD_STAT: &str = "support_manaforged_arrows_mana_cost_%_threshold";

/// The stat that makes Cast when Damage Taken's config apply (PoB's
/// `triggeredByDamageTaken`, `SkillStatMap.lua` 258).
const DAMAGE_TAKEN_STAT: &str = "cast_on_damage_taken_threshold";

/// What triggers a skill.
#[derive(Debug, Clone)]
pub enum Kind<'d> {
    /// Nothing: the skill is not triggered.
    NotTriggered,
    /// A ported config, with its trigger support if any.
    Ported(Config, Option<GemEffectRef<'d>>),
    /// A trigger the engine does not port.
    Unported,
}

/// What triggers `skill` (module docs).
#[must_use]
pub fn kind<'d>(skill: &MainSkill<'d>) -> Kind<'d> {
    if let Some(s) = skill
        .supports
        .iter()
        .find(|s| TRIGGER_SUPPORTS.contains(&s.effect.id.as_str()))
    {
        return SUPPORTS
            .iter()
            .find(|(id, _)| *id == s.effect.id)
            .map_or(Kind::Unported, |(_, c)| Kind::Ported(*c, Some(*s)));
    }
    // A skill an item grants and triggers (open items S3, part 7): PoB's
    // config by the skill's ID or the unique's name.
    if let Some(config) = granted_config(skill) {
        return Kind::Ported(config, None);
    }
    // A skill PoB marks triggered without a ported config. A skill with
    // only `InbuiltTrigger` and no PoB config is self-cast in PoB, as here.
    if skill.types.contains("Triggered") {
        return Kind::Unported;
    }
    Kind::NotTriggered
}

/// A triggered skill's rate, or why there is none.
#[derive(Debug)]
pub enum Outcome {
    /// Not triggered, or no source: PoB reports the skill as self-cast.
    SelfCast,
    /// The trigger rate, per second, with the reasons it carries (the
    /// source's fields'), and whether the trigger's source is the skill
    /// itself (Cast while Channelling, the global triggers): PoB then
    /// turns the skill's `TriggeredDamage` into `Damage`
    /// (`addTriggerIncMoreMods`, 23 to 30, its tag `SkillType.Triggered`
    /// read in the source's config) ([`TRIGGERED_DAMAGE`]).
    Rate(f64, Reasons, bool),
    /// The rate cannot be computed.
    Unsupported(UnsupportedReason),
}

/// One skill of a trigger's rotation (PoB's `packageSkillDataForSimulation`).
struct Rotated {
    /// The skill's own cooldown, seconds.
    cooldown: f64,
    /// Whether it is the main skill.
    main: bool,
    /// Its cooldown recovery factor (PoB's `icdr`).
    icdr: f64,
}

/// The rate of the main skill `skill` when a ported trigger triggers it
/// (module docs).
#[must_use]
pub fn rate(calc: &Calc<'_>, skill: &MainSkill<'_>, source: &Source, flags: Flags) -> Outcome {
    let Kind::Ported(config, support) = kind(skill) else {
        return Outcome::SelfCast;
    };
    let input = *calc.input();
    let levels = super::gem_levels::GemLevels::from_effects(calc.store().effects());
    let unsupported = |what| {
        Outcome::Unsupported(UnsupportedReason::UnmodelledSkill {
            source: source.clone(),
            what,
        })
    };
    let instance_stat = |s: &GemEffectRef<'_>, stat: &str| {
        skill::instance(s.effect, s.level, s.quality, input.data)
            .stats
            .iter()
            .find(|(id, _)| id == stat)
            .map(|(_, v)| *v)
    };
    // Cast on Death (open items S3): PoB's stand-in time over the skill's
    // cooldown recovery INC (`CalcOffence.lua` 2214 to 2221: no more, no
    // action speed, no tick cap), its `TriggeredDamage` as `Damage`
    // (`CalcPerform.lua` 1440 to 1445).
    if config == Config::CastOnDeath {
        let cx = calc.context(flags);
        let mut reasons = Reasons::new();
        let mut inc = 0.0;
        for q in [
            Quantity::Poe1(Poe1Quantity::SkillTiming(
                super::skill_modifiers::SkillTiming::CooldownRecovery,
            )),
            Quantity::Poe1(Poe1Quantity::TypedCooldownRecovery),
        ] {
            for e in super::repeats::applying(calc, q, Op::Increased, &cx, &mut reasons) {
                inc += e.value;
            }
        }
        let divisor = 1.0 + inc / 100.0;
        if divisor <= 0.0 {
            return unsupported("cooldown-recovery");
        }
        return Outcome::Rate(divisor / ON_DEATH_TIME, reasons, true);
    }
    // Cast on Melee Kill applies only after a kill (its config, 1155).
    if config == Config::CastOnMeleeKill {
        match killed_recently(&input) {
            Some(true) => {}
            Some(false) => return Outcome::SelfCast,
            None => return unsupported("killed-recently"),
        }
    }
    // Cast when Damage Taken applies only with its threshold
    // (`triggeredByDamageTaken`); else PoB reports self-cast.
    if config == Config::CastWhenDamageTaken
        && support
            .as_ref()
            .and_then(|s| instance_stat(s, DAMAGE_TAKEN_STAT))
            .is_none()
    {
        return Outcome::SelfCast;
    }
    // An item line below 100%: PoB ignores its chance (open items S3,
    // part 5), the game does not.
    if config.is_item_line()
        && let Some(s) = &support
        && item_line_chance(calc, skill, s.effect.id.as_str()).is_some_and(|c| c < 100.0)
    {
        return unsupported("item-trigger-chance");
    }
    // A skill an item grants is triggered only with its line (open items
    // S3, part 7); the line's chance is PoB's `triggerChance`.
    let granted_line = if config.is_granted() {
        match granted_chance(calc, skill) {
            Some(c) => Some(c),
            None => return Outcome::SelfCast,
        }
    } else {
        None
    };
    let seconds = |ms: i64| {
        #[expect(clippy::cast_precision_loss, reason = "milliseconds, exact as f64")]
        let s = ms as f64 / 1000.0;
        s
    };
    let level_row = skill::effect_level(skill.effect(), skill.active_gem.level);
    let stored_uses = level_row.map_or(0, |l| l.stored_uses);
    let triggered_cd = config
        .cooldown_override()
        .unwrap_or_else(|| level_row.map_or(0.0, |l| seconds(l.cooldown)));
    let trigger_cd = support
        .as_ref()
        .and_then(|s| skill::effect_level(s.effect, s.level))
        .map_or(0.0, |l| seconds(l.cooldown));
    let mut reasons = Reasons::new();
    // The triggered skill's cooldown recovery (open items S3; PoB's
    // `icdr = calcLib.mod(skillModList, skillCfg, "CooldownRecovery")`,
    // 558): both cooldowns are divided by it (575 to 576), in the cap and
    // in the rotation (`packageSkillDataForSimulation`, 64).
    let Some(icdr) = cooldown_recovery(calc, flags, &mut reasons) else {
        return unsupported("cooldown-recovery");
    };
    // PoB's `ignoresTickRate`: more than one stored use.
    let ignores_tick = stored_uses > 1;
    let tick_rate = 1.0 / SERVER_TICK_TIME;
    let rounded = |cd: f64, ignore: bool| {
        if ignore {
            cd
        } else {
            crate::math::ceil(cd * tick_rate) / tick_rate
        }
    };
    let action = crate::math::max(
        rounded(trigger_cd / icdr, false),
        rounded(triggered_cd / icdr, ignores_tick),
    );
    let cap = if action == 0.0 {
        f64::INFINITY
    } else {
        1.0 / action
    };
    let global = matches!(
        config,
        Config::CastWhenDamageTaken | Config::CastWhenStunned | Config::CastOnWardBreak
    );
    if global {
        // `skillFlags.globalTrigger` without `triggeredSkillCond`: the
        // effective rate is the cap, and the trigger rate the effective
        // rate (or `min(cap, rate)` when it ignores the tick rate).
        if !cap.is_finite() {
            return unsupported("trigger-no-cooldown");
        }
        return Outcome::Rate(cap, reasons, true);
    }
    let skills = skill::active_skills(input.build, input.data, &levels);
    let main_group = skill.group;
    let is_main = |s: &MainSkill<'_>| {
        s.active_gem.gem == skill.active_gem.gem && s.effect().id == skill.effect().id
    };
    let main_slot = group_slot(input.build, main_group);
    let rotation_of = |config: Config| -> Vec<Rotated> {
        skills
            .iter()
            .filter(|(g, _, s)| {
                // PoB's `slotMatch` (the group, for a skill that is not an
                // item's) or the item's slot (`socketGroup.slot`).
                let together = if config.rotation_by_slot() {
                    main_slot.is_some() && group_slot(input.build, *g) == main_slot
                } else {
                    *g == main_group
                };
                together
                    && s.types.contains("Spell")
                    && s.supports.iter().any(|x| {
                        SUPPORTS
                            .iter()
                            .any(|(id, c)| *c == config && *id == x.effect.id)
                    })
            })
            .map(|(_, _, s)| Rotated {
                cooldown: skill::effect_level(s.effect(), s.active_gem.level)
                    .map_or(0.0, |l| seconds(l.cooldown)),
                main: is_main(s),
                // The main skill's factor for each: the engine computes
                // the main skill's modifiers only. The rotations ported
                // share one trigger support, whose recovery is the same
                // on each.
                icdr,
            })
            .collect()
    };
    if config == Config::CastOnBlock {
        // A global trigger whose source is the skill (1498 to 1511): the
        // effective rate is the cap (714 to 719); the config has
        // `triggeredSkillCond`, so PoB simulates the rotation of the
        // group's spells the support supports at it (809), with the
        // trigger's cooldown and a chance of 100.
        if !cap.is_finite() {
            return unsupported("trigger-no-cooldown");
        }
        let rotation = rotation_of(Config::CastOnBlock);
        if !rotation.iter().any(|r| r.main) {
            return unsupported("trigger-rotation");
        }
        let trigger = if trigger_cd != 0.0 {
            trigger_cd
        } else {
            triggered_cd
        };
        return Outcome::Rate(rotation_rate(&rotation, cap, trigger, 100.0), reasons, true);
    }
    if config == Config::ArcanistBrand {
        // The brand's activations are the effective source rate. The
        // config has `triggeredSkillCond`, so PoB always simulates the
        // rotation of the group's spells the brand triggers (its charge
        // branch, 792, compares two tables by identity), with no trigger
        // cooldown and a chance of 100 (809); the cap is not read.
        let Some(source_rate) = arcanist_rate(calc, &skills, &mut reasons) else {
            return unsupported("trigger-source");
        };
        let rotation = rotation_of(Config::ArcanistBrand);
        if !rotation.iter().any(|r| r.main) {
            return unsupported("trigger-rotation");
        }
        if source_rate <= 0.0 {
            return unsupported("trigger-source");
        }
        return Outcome::Rate(
            rotation_rate(&rotation, source_rate, 0.0, 100.0),
            reasons,
            true,
        );
    }
    if config == Config::CastWhileChannelling {
        // `CWCHandler`: the first channelling skill of the group the
        // support's first part supports, not triggered (219 to 236).
        let interval = skills.iter().find_map(|(g, _, s)| {
            if *g != main_group || is_main(s) || s.types.contains("Triggered") {
                return None;
            }
            s.supports
                .iter()
                .find(|x| {
                    SOURCE_PARTS
                        .iter()
                        .any(|(id, c)| *c == Config::CastWhileChannelling && *id == x.effect.id)
                })
                .and_then(|x| instance_stat(x, CWC_TIME_STAT))
        });
        let rotation = rotation_of(Config::CastWhileChannelling);
        let Some(ms) = interval.filter(|ms| *ms > 0) else {
            return Outcome::SelfCast;
        };
        if !rotation.iter().any(|r| r.main) {
            return Outcome::SelfCast;
        }
        return Outcome::Rate(
            rotation_rate(&rotation, 1.0 / seconds(ms), 0.0, 100.0),
            reasons,
            true,
        );
    }
    // The source (`findTriggerSkill`).
    let player_main = skill::main_skill(input.build, input.data, &levels).ok();
    let is_player_main = |g: usize, s: &MainSkill<'_>| {
        player_main.as_ref().is_some_and(|m| {
            m.group == g && m.active_gem.gem == s.active_gem.gem && m.effect().id == s.effect().id
        })
    };
    let hands = super::collect::hands(&input);
    // Spellslinger's source: a projectile attack with the wand flag (PoB's
    // `ModFlag.Wand` in the attack's `skillCfg`: the main hand holds a
    // wand), in any group (1209 to 1210).
    let wand = matches!(&hands.main_hand, super::collect::Held::Weapon(c) if c == "Wand");
    // The item configs' sources by the attack's weapon flags (open items
    // S3, part 5): PoB's `skillCfg.flags` hold the main hand's
    // (`CalcActiveSkill.lua` 293 to 309, 455; an empty hand is `Unarmed`
    // without `Weapon1H`, 205 to 221). `None`: a hand not known.
    let weapon = match &hands.main_hand {
        super::collect::Held::Weapon(c) => super::hit::weapon_type_flags(c),
        super::collect::Held::Empty => Some(Flags::UNARMED),
        _ => None,
    };
    let weapon_has = |f: Flags| weapon.map(|w| w.intersects(f));
    if matches!(
        config,
        Config::PoetsPen
            | Config::AsenathsChant
            | Config::Mjolner
            | Config::CosprisMalice
            | Config::WingOfTheWyvern
            | Config::SevenTeachings
            | Config::ManaforgedArrows
            | Config::LioneyesPaws
    ) && weapon.is_none()
    {
        return unsupported("trigger-source");
    }
    let mut best: Option<Source3> = None;
    for (g, a, s) in &skills {
        let melee = s.types.contains("Melee");
        let attack = s.types.contains("Attack");
        // A damaging skill too for some granted triggers (PoB's
        // `SkillType.Damage or SkillType.Attack`).
        let damage = config == Config::UniqueDamageOrAttack && s.types.contains("Damage");
        let candidate = (attack || damage)
            && !s.types.contains("Triggered")
            && !s.types.contains("InbuiltTrigger")
            && !s.types.contains("OtherThingUsesSkill")
            && !is_player_main(*g, s)
            && match config {
                Config::CastOnCrit => *g == main_group,
                Config::CastOnMeleeKill => *g == main_group && melee,
                Config::Spellslinger => wand && s.types.contains("Projectile"),
                Config::PoetsPen => weapon_has(Flags::WAND) == Some(true),
                Config::AsenathsChant => weapon_has(Flags::BOW) == Some(true),
                Config::Mjolner => {
                    *g != main_group && weapon_has(Flags::MACE | Flags::WEAPON_1H) == Some(true)
                }
                Config::CosprisMalice => {
                    melee && weapon_has(Flags::SWORD | Flags::WEAPON_1H) == Some(true)
                }
                Config::WingOfTheWyvern => *g != main_group && weapon_has(Flags::BOW) == Some(true),
                Config::SevenTeachings => melee && weapon_has(Flags::UNARMED) == Some(true),
                Config::FireSpellOnHit => melee,
                Config::ManaforgedArrows => weapon_has(Flags::BOW) == Some(true),
                Config::UniqueMeleeOrAttack
                | Config::Starcaller
                | Config::MoonbendersWing
                | Config::UniqueDamageOrAttack => true,
                Config::LioneyesPaws => attack && weapon_has(Flags::BOW) == Some(true),
                Config::CastWhenDamageTaken
                | Config::CastWhenStunned
                | Config::CastOnWardBreak
                | Config::CastWhileChannelling
                | Config::ArcanistBrand
                | Config::CastOnDeath
                | Config::CastOnBlock => false,
            };
        if !candidate || s.disabled_by_hands(&hands) != Ok(false) {
            continue;
        }
        let b = skill::with_main_skill(input.build, *g, *a);
        let mut options = input.options.clone();
        options.nested = true;
        let sub = crate::input::CalcInput {
            build: &b,
            data: input.data,
            options: &options,
        };
        let Ok(result) = crate::calculate(&sub) else {
            return unsupported("trigger-source");
        };
        let speed = result
            .fields
            .get(&Field::HitSpeed)
            .filter(|f| f.value.is_some())
            .or_else(|| result.fields.get(&Field::Speed));
        let Some(rate) = speed.and_then(|f| f.value) else {
            return unsupported("trigger-source");
        };
        if best.as_ref().is_none_or(|b| rate > b.rate) {
            best = Some(Source3 {
                rate,
                attack,
                at: (*g, *a),
                double_hits: super::hit::double_hits(s, input.data),
                speed: speed.map(|f| f.state.clone()),
                hit: result
                    .fields
                    .get(&Field::HitChance)
                    .map(|f| (f.value, f.state.clone())),
                crit: result
                    .fields
                    .get(&Field::CritChance)
                    .map(|f| (f.value, f.state.clone())),
            });
        }
    }
    let Some(src) = best else {
        return Outcome::SelfCast;
    };
    let mut carry = |state: Option<&FieldState>| state.is_none_or(|s| carry(s, &mut reasons));
    if !carry(src.speed.as_ref()) {
        return unsupported("trigger-source");
    }
    // The trigger chance (727 to 790): the source is an attack; a trigger
    // on use (`triggerOnUse`, 732) skips its hit chance.
    let on_use = config.on_use() || !src.attack;
    let mut chance = 100.0;
    let (hit, hit_state) = src
        .hit
        .clone()
        .unwrap_or((Some(100.0), FieldState::Complete));
    let Some(hit) = hit else {
        return unsupported("trigger-source");
    };
    if hit != 100.0 && !on_use {
        if !carry(Some(&hit_state)) {
            return unsupported("trigger-source");
        }
        chance = chance * hit / 100.0;
    }
    if config.on_crit() && src.attack {
        let Some((Some(crit), crit_state)) = src.crit.clone() else {
            return unsupported("trigger-source");
        };
        if crit != 100.0 {
            if !carry(Some(&crit_state)) {
                return unsupported("trigger-source");
            }
            chance = chance * crit / 100.0;
        }
    }
    // The line's chance of a skill an item grants (open items S3, part 7;
    // PoB's `srcInstance.triggerChance`, 784 to 789).
    if let Some(line) = granted_line
        && line != 100.0
    {
        chance = chance * line / 100.0;
    }
    // The rotation: the group's spells the trigger supports
    // (`triggeredByCoc`, `triggeredByMeleeKill`, and `slotMatch`); a
    // config without `triggeredSkillCond` (Spellslinger) simulates the main
    // skill alone (809).
    let rotation = if config.main_alone() {
        vec![Rotated {
            cooldown: triggered_cd,
            main: true,
            icdr,
        }]
    } else {
        rotation_of(config)
    };
    if !rotation.iter().any(|r| r.main) {
        return unsupported("trigger-rotation");
    }
    let mut effective = src.rate;
    // An item's trigger from an attack while dual wielding, unless the
    // attack hits with both weapons: half the rate (441 to 449).
    if config.is_item_line() && !src.double_hits {
        match hands.dual_wielding() {
            Some(true) => effective /= 2.0,
            Some(false) => {}
            None => return unsupported("trigger-source"),
        }
    }
    // Manaforged Arrows (522 to 556): the source's uses until the mana they
    // spend reaches the threshold of the triggered attack's, each raw
    // (`ManaCostRaw`): the rate over `ceil(threshold x triggered /
    // source)`; a source without a cost never triggers it.
    if config == Config::ManaforgedArrows {
        let Some(threshold) = support
            .as_ref()
            .and_then(|s| instance_stat(s, MANAFORGED_THRESHOLD_STAT))
        else {
            return unsupported("manaforged-threshold");
        };
        let Some(triggered) = super::costs::mana_cost_raw(calc, skill, &mut reasons) else {
            return unsupported("manaforged-cost");
        };
        if triggered > 0.0 {
            let Some(source_cost) = source_mana_cost_raw(calc, &skills, src.at, &mut reasons)
            else {
                return unsupported("manaforged-cost");
            };
            if source_cost <= 0.0 {
                return Outcome::Rate(0.0, reasons, false);
            }
            #[expect(clippy::cast_precision_loss, reason = "a percentage, exact as f64")]
            let threshold = threshold as f64 / 100.0;
            effective /= crate::math::ceil(triggered * threshold / source_cost);
        }
    }
    let value = if ignores_tick && rotation.len() == 1 {
        crate::math::min(cap, effective)
    } else {
        let trigger = if trigger_cd != 0.0 {
            trigger_cd
        } else {
            triggered_cd
        };
        rotation_rate(&rotation, effective, trigger, chance)
    };
    Outcome::Rate(value, reasons, false)
}

/// The main skill's cooldown recovery factor in a context of `flags`: PoB's
/// `calcLib.mod(skillModList, skillCfg, "CooldownRecovery")`, `(1 + inc /
/// 100) x more`, over the character's untagged and typed terms and the
/// skill's own (the quantities a self-cast spell's cooldown reads,
/// `hit::cooldown_limit`); `None` when it is not above 0 (PoB would divide
/// by it).
fn cooldown_recovery(calc: &Calc<'_>, flags: Flags, reasons: &mut Reasons) -> Option<f64> {
    let cx = calc.context(flags);
    let quantities = [
        Quantity::Poe1(Poe1Quantity::SkillTiming(
            super::skill_modifiers::SkillTiming::CooldownRecovery,
        )),
        Quantity::Poe1(Poe1Quantity::TypedCooldownRecovery),
    ];
    let mut inc = 0.0;
    let mut more = 1.0;
    for q in quantities {
        for e in super::repeats::applying(calc, q, Op::Increased, &cx, reasons) {
            inc += e.value;
        }
        for e in super::repeats::applying(calc, q, Op::More, &cx, reasons) {
            more *= 1.0 + e.value / 100.0;
        }
    }
    let icdr = (1.0 + inc / 100.0) * more;
    (icdr > 0.0).then_some(icdr)
}

/// Arcanist Brand's activations per second (open items S3; PoB's `trigRate`,
/// `CalcTriggers.lua` 1321 to 1341), with the reasons its terms carry into
/// `reasons`; `None` without the brand or when a term cannot be read.
///
/// - **The brand** is the first Arcanist Brand among the active skills, in
///   any group (PoB's `activeSkillList` loop, 1324).
/// - **Its own context:** the build calculated up to its modifiers with the
///   brand as the main skill ([`crate::engine::collected`], as PoB's brand
///   skill list), with the brand's flags: `ModFlag.Cast` and `Spell`, the
///   spell keyword, and its types' keywords (`SkillType.Brand`).
/// - **The rate** is `repeatFrequency x (1 + inc / 100) x more x attached`:
///   `repeatFrequency` the brand's [`REPEAT_STAT`] over 1000 (a period,
///   which PoB multiplies as it is; 1 s for the gem), `inc` the `Speed` and
///   `BrandActivationFrequency` INC sums, `more` the
///   `BrandActivationFrequency` MORE product, `attached` PoB's
///   `attachedBrandCount` (`CalcPerform.lua` 1322 to 1326): the brand's
///   attach limit (its gems' [`ATTACH_STAT`] and the character's) at most
///   the active brand limit (3 and the additions), or the config's
///   `BrandsAttachedToEnemy` at most that.
fn arcanist_rate(
    calc: &Calc<'_>,
    skills: &[(usize, usize, MainSkill<'_>)],
    reasons: &mut Reasons,
) -> Option<f64> {
    use super::stacks::{StackKind, StackValue};
    let input = *calc.input();
    let (g, a, brand) = skills
        .iter()
        .find(|(_, _, s)| s.effect().id == ARCANIST_SKILL)?;
    let gem_stat = |stat: &str| -> f64 {
        std::iter::once(&brand.active_gem)
            .chain(&brand.supports)
            .filter_map(|gem| {
                skill::instance(gem.effect, gem.level, gem.quality, input.data)
                    .stats
                    .iter()
                    .find(|(id, _)| id == stat)
                    .map(|(_, v)| *v)
            })
            .map(|v| {
                #[expect(clippy::cast_precision_loss, reason = "small integers, exact as f64")]
                let v = v as f64;
                v
            })
            .sum()
    };
    let repeat = gem_stat(REPEAT_STAT) / 1000.0;
    let b = skill::with_main_skill(input.build, *g, *a);
    let mut options = input.options.clone();
    options.nested = true;
    options.own_skill_context = true;
    let sub = crate::input::CalcInput {
        build: &b,
        data: input.data,
        options: &options,
    };
    let context = crate::engine::collected(&sub, &super::Poe1Rules).ok()?;
    let flags = Flags::CAST | Flags::SPELL | Flags::KEYWORD_SPELL | brand.keyword_flags();
    let cx = context.context(flags);
    let frequency = Quantity::Poe1(Poe1Quantity::BrandActivationFrequency);
    let mut inc = 0.0;
    for q in [Quantity::Speed, frequency] {
        for e in super::repeats::applying(&context, q, Op::Increased, &cx, reasons) {
            inc += e.value;
        }
    }
    let mut more = 1.0;
    for e in super::repeats::applying(&context, frequency, Op::More, &cx, reasons) {
        more *= 1.0 + e.value / 100.0;
    }
    let stack = |value| {
        let q = Quantity::Poe1(Poe1Quantity::Stack(StackKind::BrandsAttached, value));
        let mut r = Reasons::new();
        let total: f64 = super::repeats::applying(&context, q, Op::Base, &cx, &mut r)
            .iter()
            .map(|e| e.value)
            .sum();
        (total, r)
    };
    let (attach_extra, r) = stack(StackValue::AttachMaximum);
    reasons.extend(&r);
    let (active_extra, r) = stack(StackValue::Maximum);
    reasons.extend(&r);
    let limit = (gem_stat(ATTACH_STAT) + attach_extra)
        .min(super::stacks::ACTIVE_BRAND_LIMIT + active_extra);
    let set = super::collect::active_config(input.build);
    let attached = match super::stacks::count(set, super::stacks::BRANDS_INPUT, false) {
        Ok(Some(c)) => c.min(limit),
        Ok(None) => limit,
        Err(()) => return None,
    };
    // The brand context's own reasons (its gems' and supports' unmapped
    // stats) beyond the character's.
    for r in &context.global().partial {
        if !calc.global().partial.contains(r) {
            reasons.partial.insert(r.clone());
        }
    }
    for r in &context.global().indeterminate {
        if !calc.global().indeterminate.contains(r) {
            reasons.indeterminate.insert(r.clone());
        }
    }
    Some(repeat * (1.0 + inc / 100.0) * more * attached)
}

/// The slot of active group `group` (1-based), as the item supports read
/// it ([`super::gem_levels::GemSlot`]); `None` without one.
fn group_slot(build: &loom_domain::Build, group: usize) -> Option<super::gem_levels::GemSlot> {
    let groups = skill::active_groups(build).ok()?;
    let g = groups.get(group.checked_sub(1)?)?;
    super::gem_levels::GemSlot::of(g.slot.as_ref()?.as_str())
}

/// The chance, in percent, of the item line that gives `support` to the
/// main skill's slot (open items S3, part 5): the line's value for a
/// chance row ([`super::item_supports::TRIGGER_CHANCE_STATS`]), else 100;
/// the lowest when several lines give it. `None` when no line does.
fn item_line_chance(calc: &Calc<'_>, skill: &MainSkill<'_>, support: &str) -> Option<f64> {
    let slot = group_slot(calc.input().build, skill.group)?;
    calc.store()
        .effects()
        .iter()
        .filter(|e| matches!(e.op, Op::Base | Op::Flag))
        .filter_map(|e| match e.quantity {
            Quantity::Poe1(Poe1Quantity::ItemSupport(stat, s)) if s == slot => {
                Some((stat, e.value))
            }
            _ => None,
        })
        .filter(|(stat, _)| {
            super::item_supports::row(stat)
                .is_some_and(|(_, _, effects)| effects.contains(&support))
        })
        .map(|(stat, value)| {
            if super::item_supports::TRIGGER_CHANCE_STATS.contains(&stat) {
                value
            } else {
                100.0
            }
        })
        .reduce(f64::min)
}

/// The `ManaCostRaw` of the active skill `at` (group and skill, 1-based)
/// as PoB's `calcs.buildActiveSkill` computes it (open items S3:
/// Manaforged Arrows' source): the build collected with it as the main
/// skill ([`crate::engine::collected`]), then [`super::costs::mana_cost_raw`].
/// Its reasons beyond the character's go into `reasons`. `None` when the
/// skill is not found or the cost cannot be read.
fn source_mana_cost_raw(
    calc: &Calc<'_>,
    skills: &[(usize, usize, MainSkill<'_>)],
    at: (usize, usize),
    reasons: &mut Reasons,
) -> Option<f64> {
    let input = *calc.input();
    let (_, _, source) = skills.iter().find(|(g, a, _)| (*g, *a) == at)?;
    let b = skill::with_main_skill(input.build, at.0, at.1);
    let mut options = input.options.clone();
    options.nested = true;
    options.own_skill_context = true;
    let sub = crate::input::CalcInput {
        build: &b,
        data: input.data,
        options: &options,
    };
    let context = crate::engine::collected(&sub, &super::Poe1Rules).ok()?;
    let cost = super::costs::mana_cost_raw(&context, source, reasons)?;
    for r in &context.global().partial {
        if !calc.global().partial.contains(r) {
            reasons.partial.insert(r.clone());
        }
    }
    for r in &context.global().indeterminate {
        if !calc.global().indeterminate.contains(r) {
            reasons.indeterminate.insert(r.clone());
        }
    }
    Some(cost)
}

/// PoB's `Condition:KilledRecently` from the config (`ConfigOptions.lua`:
/// the check, or a kill count above 0 through `multiplierKilledRecently`);
/// `None` for a value of another type.
fn killed_recently(input: &crate::input::CalcInput<'_>) -> Option<bool> {
    let set = super::collect::active_config(input.build);
    let check = super::conditions::check(set, "conditionKilledRecently")?;
    let count = match set.and_then(|s| {
        super::config::last_input(
            s,
            "multiplierKilledRecently",
            loom_domain::ConfigInputKind::Input,
        )
    }) {
        None => false,
        Some(loom_domain::ConfigValue::Number(n)) => n.value().0 > 0.0,
        Some(_) => return None,
    };
    Some(check || count)
}

/// Adds a field's reasons to `reasons` (its partial and indeterminate
/// ones); `false` when the field is unsupported (P6-8a: a trigger's
/// source; P6-8b: a mirage's copy).
pub(super) fn carry(state: &FieldState, reasons: &mut Reasons) -> bool {
    match state {
        FieldState::Complete => true,
        FieldState::Partial(p) => {
            reasons.partial.extend(p.iter().cloned());
            true
        }
        FieldState::Indeterminate {
            reasons: r,
            partial,
        } => {
            reasons.indeterminate.extend(r.iter().cloned());
            reasons.partial.extend(partial.iter().cloned());
            true
        }
        FieldState::Unsupported(_) => false,
    }
}

/// The source skill's fields a trigger reads.
struct Source3 {
    rate: f64,
    /// Whether it is an attack: only an attack's hit and crit chances are
    /// read (732).
    attack: bool,
    /// The source's group and active skill (1-based, as
    /// [`skill::active_skills`] gives them).
    at: (usize, usize),
    /// PoB's `doubleHitsWhenDualWielding` ([`super::hit::double_hits`]).
    double_hits: bool,
    speed: Option<FieldState>,
    hit: Option<(Option<f64>, FieldState)>,
    crit: Option<(Option<f64>, FieldState)>,
}

/// PoB's `calcMultiSpellRotationImpact` (`CalcTriggers.lua` 87 to 133):
/// 1,000 source uses at `source_rate`, each triggering the next skill of
/// the rotation off cooldown (its cooldown the larger of its own and the
/// trigger's, from the start of the tick to the next tick after it ends);
/// the main skill's rate is `1 / (time / count + (1 / rate) / chance x 100
/// - 1 / rate)`.
fn rotation_rate(rotation: &[Rotated], source_rate: f64, trigger_cd: f64, chance: f64) -> f64 {
    let increment = 1.0 / source_rate;
    let sim_time = increment * 1000.0;
    let ceil_b = |x: f64| SERVER_TICK_TIME * crate::math::ceil(x / SERVER_TICK_TIME);
    let floor_b = |x: f64| SERVER_TICK_TIME * crate::math::floor(x / SERVER_TICK_TIME);
    // Each skill's cooldown and the trigger's over its recovery (69).
    let cds: Vec<f64> = rotation
        .iter()
        .map(|r| crate::math::max(r.cooldown / r.icdr, trigger_cd / r.icdr))
        .collect();
    let n = rotation.len();
    let mut next_trig = vec![0.0_f64; n];
    let mut count = vec![0_u32; n];
    let mut index = 0;
    let mut next = 0.0_f64;
    while next < sim_time {
        let mut current = index;
        loop {
            if next_trig[current] <= next {
                count[current] += 1;
                next_trig[current] = ceil_b(floor_b(next) + cds[current]);
                break;
            }
            current = (current + 1) % n;
            if current == index {
                break;
            }
        }
        index = (index + 1) % n;
        next += increment;
    }
    rotation
        .iter()
        .zip(&count)
        .find(|(r, _)| r.main)
        .map_or(0.0, |(_, c)| {
            1.0 / (sim_time / f64::from(*c) + (increment / chance * 100.0) - increment)
        })
}

/// Whether Cast on Death supports `skill` (PoB's `triggeredOnDeath`): its
/// `showAverage` stays (`CalcOffence.lua` 2214, not cleared as a trigger
/// rate's is, 2227).
#[must_use]
pub fn on_death(skill: &MainSkill<'_>) -> bool {
    skill
        .supports
        .iter()
        .any(|s| SUPPORTS.contains(&(s.effect.id.as_str(), Config::CastOnDeath)))
}

/// Whether Arcanist Brand's support part supports `skill` (PoB's
/// `triggeredByBrand`; its `addFlags` `brand`).
#[must_use]
pub fn brand_triggered(skill: &MainSkill<'_>) -> bool {
    skill
        .supports
        .iter()
        .any(|s| SUPPORTS.contains(&(s.effect.id.as_str(), Config::ArcanistBrand)))
}

/// Whether the actor's main skill is computed through a trigger the
/// engine ports, or not triggered at all.
#[must_use]
pub fn computable(skill: &MainSkill<'_>) -> bool {
    !matches!(kind(skill), Kind::Unported)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One skill, no trigger cooldown, every source use triggers it: the
    /// source's rate.
    #[test]
    fn a_lone_skill_without_cooldown_should_follow_the_source() {
        let r = rotation_rate(
            &[Rotated {
                cooldown: 0.0,
                main: true,
                icdr: 1.0,
            }],
            2.0,
            0.0,
            100.0,
        );
        assert!((r - 2.0).abs() < 1e-9, "{r}");
    }

    /// Open items S3: Spellslinger's 0.6 s cooldown over its 38% cooldown
    /// recovery and a 2.25 source rate give PoB's 1.197 (PoB v2.67.2's
    /// `SkillTriggerRate` on the Spellslinger fixture with Power Siphon as
    /// the source, `docs/phase-6/triggers.md`, "Part 3").
    #[test]
    fn the_cooldown_recovery_should_divide_the_cooldowns_in_the_rotation() {
        let r = rotation_rate(
            &[Rotated {
                cooldown: 0.0,
                main: true,
                icdr: 1.38,
            }],
            2.25,
            0.6,
            100.0,
        );
        assert!((r - 1.197).abs() < 1e-9, "{r}");
    }

    /// Open items S3, part 7: the granted triggers' tables are sorted, and an
    /// item group's source gives the item and the unique's name as PoB's
    /// pattern (`.*:.*:(.*),.*`) does.
    #[test]
    fn granted_triggers_should_be_found_by_line_and_unique() {
        assert!(GRANTED_TRIGGERS.windows(2).all(|w| w[0].0 < w[1].0));
        assert!(GRANTED_BY_UNIQUE.windows(2).all(|w| w[0].0 < w[1].0));
        assert_eq!(
            granted_trigger("local_display_molten_burst_on_melee_hit_%"),
            Some(&("local_display_molten_burst_on_melee_hit_%", true, false))
        );
        assert_eq!(
            granted_source("Item:12:Ngamahu's Flame, Abyssal Axe"),
            Some((12, "Ngamahu's Flame"))
        );
        assert_eq!(granted_source("Tree:4494"), None);
        // Moonbender's Wing and Lioneye's Paws: PoB's cooldown of 1 s.
        assert_eq!(Config::MoonbendersWing.cooldown_override(), Some(1.0));
        assert_eq!(Config::UniqueMeleeOrAttack.cooldown_override(), None);
    }

    /// PoB's rotation fed Smite's attack rate on Limbsplit's Woodsplitter
    /// (PoB's 1.105, a 1 s cooldown, a 22% hit chance) gives PoB's trigger
    /// rate to the bit (open items S3, part 7; DIV-0700 is the engine's
    /// floored rate, Q11).
    #[test]
    fn the_rotation_should_give_pobs_rate_for_limbsplit() {
        let r = rotation_rate(
            &[Rotated {
                cooldown: 1.0,
                main: true,
                icdr: 1.0,
            }],
            1.105,
            1.0,
            22.0,
        );
        assert!((r - 0.199_262_295_081_967_24).abs() < 1e-12, "{r}");
    }

    /// A 0.15 s trigger cooldown caps a fast source near PoB's tick-rounded
    /// cap; two skills share the triggers.
    #[test]
    fn the_rotation_should_share_and_cap_the_triggers() {
        let one = rotation_rate(
            &[Rotated {
                cooldown: 0.0,
                main: true,
                icdr: 1.0,
            }],
            20.0,
            0.15,
            100.0,
        );
        assert!(one > 5.0 && one <= 1.0 / 0.165 + 1e-9, "{one}");
        let two = rotation_rate(
            &[
                Rotated {
                    cooldown: 0.0,
                    main: true,
                    icdr: 1.0,
                },
                Rotated {
                    cooldown: 0.0,
                    main: false,
                    icdr: 1.0,
                },
            ],
            1.0,
            0.15,
            100.0,
        );
        assert!((two - 0.5).abs() < 0.01, "{two}");
    }
}
