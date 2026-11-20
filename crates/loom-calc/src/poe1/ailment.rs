//! Damaging ailments (Phase 6 plan, P6-2b on): the shared base PoB v2.67.2
//! builds every ailment on (`Modules/CalcOffence.lua` 4036 to 4287) and
//! ignite (4866 to 5206), with PoB's order of operations, as each hit's
//! secondary effect:
//!
//! 1. **Chance:** on a hit, the `AilmentChance` sum in the hit's config, at
//!    most 100; on a crit, 100 (crits always ignite).
//! 2. **The ailment's config** (`igniteCfg`): `Dot`, `Ailment`, `Ignite`,
//!    `FireDot`, and the skill's keywords; not the hit's `Spell`, `Attack`,
//!    `Hit`, `Area`, `Projectile`, or `Melee`.
//! 3. **Rate and duration:**
//!    - `rateMod = mod(faster) / mod(slower)`;
//!    - `duration = 4 x mod(duration) / rateMod`, where `mod` is `(1 + inc
//!      / 100) x more`.
//! 4. **Stacks:**
//!    - `stacks = hitChance / 100 x chance x duration x rate`, where `chance
//!      = onHit / 100 x (1 - crit / 100) + onCrit / 100 x crit / 100`;
//!    - the roll average is 50, or `stacks / (stacks + 1) x 100` when more
//!      than one ignite would stack.
//! 5. **Source damage:** the hit's fire damage recomputed in the ignite's
//!    config (`calcAilmentSourceDamage`: `calcDamage` with the hit's bases
//!    and conversion, times the share fire keeps), then `(min + (max - min)
//!    x roll / 100) x (1 + DoT multiplier / 100)`; from crits, the DoT
//!    multiplier also has the character's +50 for critical ailments
//!    (`critical_ailment_dot_multiplier_+`).
//! 6. **Base** (`calcAilmentDamage`):
//!    - the hit and crit sources weighted by the chances from non-crits and
//!      crits, at the crit chance over the stacks, `1 - (1 - crit / 100) ^
//!      max(1, stacks)`;
//!    - then times `IgnitePercentBase` (0.9).
//! 7. **DPS:** `base x mod(effect) x rateMod x min(stacks, 1) x effMult`,
//!    at most `DotDpsCap`; `effMult` is the enemy's fire resistance (no
//!    penetration) and damage taken, as a DoT's.
//!
//! Not modelled, each a reason through its unmapped stat or the reasons
//! below:
//! - ignites that stack, ignite to chaos, other types that can ignite;
//! - Ruthless Blow, Fist of War, and warcry ailment effects (1 without
//!   their stats);
//! - the enemy's ailment avoidance (no source modelled);
//! - burning ground from ignite;
//! - dual wielding's per-hand ailments (P6-2a: the field is `NotComputed`
//!   for a dual-wielding attack).

use super::enemy::Enemy;
use super::hit::{self, HitSources};
use super::phases::{self, Valued};
use super::pob_skill::{PobSkill, Value};
use super::{Poe1Quantity, keystones};
use crate::condition::{ConditionKey, Flags, QueryContext};
use crate::engine::Calc;
use crate::error::CalcError;
use crate::graph::Term;
use crate::input::CalcData;
use crate::quantity::{DamageType, Op, Quantity};
use crate::source::Source;
use crate::state::{Reasons, UnsupportedReason};

/// A damaging ailment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Ailment {
    /// Ignite (fire).
    Ignite,
    /// Bleeding (physical).
    Bleed,
    /// Poison (chaos).
    Poison,
}

impl Ailment {
    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Ignite => "ignite",
            Self::Bleed => "bleed",
            Self::Poison => "poison",
        }
    }

    /// The field of the ailment's DPS.
    #[must_use]
    pub const fn field(self) -> crate::result::Field {
        match self {
            Self::Ignite => crate::result::Field::IgniteDps,
            Self::Bleed => crate::result::Field::BleedDps,
            Self::Poison => crate::result::Field::PoisonDps,
        }
    }

    /// Every damaging ailment, in PoB's order (bleed, poison, ignite).
    pub const ALL: [Self; 3] = [Self::Bleed, Self::Poison, Self::Ignite];

    /// The ailment's keyword flag.
    #[must_use]
    pub const fn flag(self) -> Flags {
        match self {
            Self::Ignite => Flags::IGNITE,
            Self::Bleed => Flags::BLEED,
            Self::Poison => Flags::POISON,
        }
    }
}

/// PoB's `data.misc.IgnitePercentBase` (`Modules/Data.lua` 260): an ignite
/// deals 90% of its source damage per second.
pub const IGNITE_PERCENT_BASE: f64 = 0.9;
/// PoB's `data.misc.IgniteDurationBase` (`Modules/Data.lua` 261), seconds.
pub const IGNITE_DURATION_BASE: f64 = 4.0;
/// PoB's `data.misc.BleedPercentBase` (`Modules/Data.lua` 256): a bleed
/// deals 70% of its source damage per second (`x 70 / 100`).
pub const BLEED_PERCENT_BASE: f64 = 70.0;
/// PoB's `data.misc.BleedDurationBase` (`Modules/Data.lua` 257), seconds.
pub const BLEED_DURATION_BASE: f64 = 5.0;
/// PoB's `data.misc.PoisonPercentBase` (`Modules/Data.lua` 258).
pub const POISON_PERCENT_BASE: f64 = 0.30;
/// PoB's `data.misc.PoisonDurationBase` (`Modules/Data.lua` 259), seconds.
pub const POISON_DURATION_BASE: f64 = 2.0;

/// The character template's damage over time multiplier of ailments from
/// critical strikes (PoB's `data.characterConstants`).
pub const CRIT_AILMENT_DOT_MULTIPLIER: &str = "critical_ailment_dot_multiplier_+";

/// The skill keyword flags an ailment's config keeps from its hit.
const KEYWORDS: Flags = Flags::SKILL_PHYSICAL
    .union(Flags::KEYWORD_ATTACK)
    .union(Flags::KEYWORD_BOW)
    .union(Flags::KEYWORD_ARROW)
    // Totem, trap, and mine (P6-11bk; PoB's ailment config keeps every
    // keyword but `Hit`).
    .union(Flags::KEYWORD_TOTEM)
    .union(Flags::KEYWORD_TRAP)
    .union(Flags::KEYWORD_MINE)
    // The skill's types (P6-11co; PoB's ailment config keeps `skillTypes`).
    .union(Flags::SKILL_RANGED_ATTACK)
    .union(Flags::SKILL_BRAND)
    .union(Flags::SKILL_MARK)
    // `band(cfg.flags, ModFlag.WeaponMask)` (P6-11b).
    .union(Flags::AILMENT_WEAPON_MASK)
    .union(Flags::KEYWORD_SPELL)
    .union(Flags::SKILL_FIRE)
    .union(Flags::SKILL_COLD)
    .union(Flags::SKILL_LIGHTNING)
    .union(Flags::SKILL_CHAOS);

/// What an ailment reads from its hit (one pass: a spell, or an attack's
/// main hand).
pub(super) struct AilmentHit<'a> {
    /// The hit's flags.
    pub(super) flags: Flags,
    /// The hit's bases and conversion.
    pub(super) sources: &'a HitSources,
    /// The hit's crit chance, percent (PoB's `output.CritChance`).
    pub(super) crit: Valued,
    /// The hit's chance to hit, percent.
    pub(super) hit_chance: Valued,
    /// Hits per second.
    pub(super) rate: Valued,
    /// The main skill's PoB data and source, when PoB has it.
    pub(super) skill: Option<(&'static PobSkill, Source)>,
}

/// The ailment's config (module docs, step 2) with the hit's settled crit
/// state.
fn ailment_context(calc: &Calc<'_>, hit: &AilmentHit<'_>, extra: Flags) -> QueryContext {
    let mut flags = Flags::from_bits_and(hit.flags, KEYWORDS) | Flags::DOT | Flags::AILMENT | extra;
    // `band(cfg.flags, ModFlag.Melee) ~= 0 and ModFlag.MeleeHit`.
    if hit.flags.contains(Flags::MELEE) {
        flags = flags | Flags::MELEE_HIT;
    }
    calc.context(flags).with_state(
        ConditionKey::new(keystones::CRIT_IN_PAST_8_SEC),
        hit.crit.value != 0.0,
    )
}

/// `(1 + inc / 100) x more` of `quantity` in `cx` (PoB's `calcLib.mod`).
fn modifier(
    calc: &mut Calc<'_>,
    quantity: Quantity,
    cx: &QueryContext,
    reasons: &mut Reasons,
) -> Result<Valued, CalcError> {
    let (inc, more) = hit::modifiers_of(calc, quantity, cx, reasons)?;
    phases::product(calc, &[inc, more], Term::Step("modifier"))
}

/// The slowest the enemy's debuffs can expire (PoB's
/// `data.misc.BuffExpirationSlowCap`, `Data.lua` 235; P6-11cv).
const BUFF_EXPIRATION_SLOW_CAP: f64 = 0.25;

/// How an ailment's source damage meets the roll (PoB's three forms).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Roll {
    /// Ignite: `(min + (max - min) x roll / 100) x mult`.
    SourceThenMultiplier,
    /// Bleed: `min x mult + (max x mult - min x mult) x roll / 100`.
    MultiplierFirst,
    /// Poison: `(min + max) / 2 x mult`, no roll.
    Midpoint,
}

/// What differs between the damaging ailments (module docs).
struct Spec {
    /// The damage types whose hit damage the ailment deals, in PoB's order.
    sources: &'static [DamageType],
    /// The type that must be dealable (`canDeal`).
    gate: DamageType,
    /// The type of its damage over time (its DoT multiplier and the enemy's
    /// resistance).
    dot_type: DamageType,
    /// Base duration, seconds.
    duration_base: f64,
    /// Ignite's crit always inflicts it; bleed's and poison's chance on a
    /// crit is the chance on a hit (no mapped stat is crit only).
    crit_always: bool,
    /// The source and the roll.
    roll: Roll,
    /// The crit chance over the stacks (ignite, bleed) or the hit's
    /// (poison).
    crit_over_stacks: bool,
    /// Only attacks inflict it (bleed).
    attack_only: bool,
    /// The physical source also takes the modifiers of chaos damage
    /// (poison: PoB's `calcAilmentSourceDamage(..., "Physical",
    /// dmgTypeFlags.Chaos)`, `CalcOffence.lua` 4655 to 4694).
    physical_as_chaos: bool,
    /// The keys of PoB's skill data that change the ailment's duration or
    /// stacks (`bleedIsSkillEffect`, ...), not modelled: a reason.
    skill_keys: &'static [&'static str],
}

const fn spec(ailment: Ailment) -> Spec {
    match ailment {
        Ailment::Ignite => Spec {
            sources: &[DamageType::Fire],
            gate: DamageType::Fire,
            dot_type: DamageType::Fire,
            duration_base: IGNITE_DURATION_BASE,
            crit_always: true,
            roll: Roll::SourceThenMultiplier,
            crit_over_stacks: true,
            attack_only: false,
            physical_as_chaos: false,
            skill_keys: &[],
        },
        Ailment::Bleed => Spec {
            sources: &[DamageType::Physical],
            gate: DamageType::Physical,
            dot_type: DamageType::Physical,
            duration_base: BLEED_DURATION_BASE,
            crit_always: false,
            roll: Roll::MultiplierFirst,
            crit_over_stacks: true,
            attack_only: true,
            physical_as_chaos: false,
            skill_keys: &[
                "bleedIsSkillEffect",
                "bleedDurationIsSkillDuration",
                "bleedBasePercent",
            ],
        },
        // PoB sums chaos first, then the other types that can poison
        // (physical always; lightning, cold, and fire with their flags, not
        // modelled).
        Ailment::Poison => Spec {
            sources: &[DamageType::Chaos, DamageType::Physical],
            gate: DamageType::Chaos,
            dot_type: DamageType::Chaos,
            duration_base: POISON_DURATION_BASE,
            crit_always: false,
            roll: Roll::Midpoint,
            crit_over_stacks: false,
            attack_only: false,
            physical_as_chaos: true,
            skill_keys: &["poisonIsSkillEffect", "poisonDurationIsSkillDuration"],
        },
    }
}

/// The hit's damage of `sources` recomputed in the ailment's config
/// (`calcAilmentSourceDamage`: `calcDamage` times the share the type keeps
/// after conversion), summed in PoB's order; `None` for a type the hit
/// cannot deal.
fn source_damage(
    calc: &mut Calc<'_>,
    hit: &AilmentHit<'_>,
    spec: &Spec,
    cx: &QueryContext,
    hit_cx: &QueryContext,
    reasons: &mut Reasons,
) -> Result<[Valued; 2], CalcError> {
    let mut mins = Vec::new();
    let mut maxes = Vec::new();
    let mut zero = None;
    for &t in spec.sources {
        let Some(index) = hit::TYPES.iter().position(|x| *x == t) else {
            continue;
        };
        if hit::cannot_deal(calc, t, hit_cx, reasons)? {
            continue;
        }
        let extra = if spec.physical_as_chaos && t == DamageType::Physical {
            Flags::damage(DamageType::Chaos)
        } else {
            Flags::NONE
        };
        let [min, max] = hit::calc_damage(
            calc,
            index,
            extra,
            &hit.sources.bases,
            hit.sources.conversion.as_ref(),
            cx,
            None,
            reasons,
            &mut zero,
        )?;
        let kept = hit
            .sources
            .conversion
            .as_ref()
            .and_then(|c| c.kept.get(index).copied().flatten());
        let (min, max) = match kept {
            Some(k) => (
                phases::product(calc, &[min, k], Term::Step("ailment-source-min"))?,
                phases::product(calc, &[max, k], Term::Step("ailment-source-max"))?,
            ),
            None => (min, max),
        };
        mins.push(min);
        maxes.push(max);
    }
    if mins.len() == 1 {
        return Ok([mins[0], maxes[0]]);
    }
    Ok([
        phases::total(calc, &mins, Term::Step("ailment-sources-min"))?,
        phases::total(calc, &maxes, Term::Step("ailment-sources-max"))?,
    ])
}

/// A damaging ailment of the main skill's hit (module docs): its DPS (for
/// poison, every poison's together, PoB's `TotalPoisonDPS`) with the
/// reasons of what it read, or `None` when the hit does not inflict it.
///
/// # Errors
/// A graph error.
pub(super) fn damaging(
    calc: &mut Calc<'_>,
    hit: &AilmentHit<'_>,
    enemy: &Enemy,
    data: &dyn CalcData,
    ailment: Ailment,
) -> Result<Option<(Valued, Reasons)>, CalcError> {
    let spec = spec(ailment);
    let mut reasons = Reasons::new();
    if spec.attack_only && !hit.flags.contains(Flags::ATTACK) {
        return Ok(None);
    }
    // Puncture's `bleedIsSkillEffect`, Viper Strike's `poisonIsSkillEffect`,
    // and kin: PoB adds the skill's duration to the ailment's (4311, 4580),
    // not modelled (P6-2c review).
    if let Some((pob, source)) = &hit.skill
        && spec
            .skill_keys
            .iter()
            .any(|k| pob.skill_data(k).is_some_and(Value::truthy))
    {
        reasons.partial.insert(UnsupportedReason::UnmodelledSkill {
            source: source.clone(),
            what: "ailment-skill-data",
        });
    }
    let hit_cx = calc.context(hit.flags).with_state(
        ConditionKey::new(keystones::CRIT_IN_PAST_8_SEC),
        hit.crit.value != 0.0,
    );
    // `canDeal.<Type>`, and "Cannot cause Bleeding" (`CannotBleed`) and kin.
    if hit::cannot_deal(calc, spec.gate, &hit_cx, &mut reasons)? {
        return Ok(None);
    }
    let cannot = Quantity::Poe1(Poe1Quantity::CannotInflict(ailment));
    if hit::flag_of(calc, cannot, &hit_cx, &mut reasons)?.is_some() {
        return Ok(None);
    }
    // Step 1: the chances.
    let on_hit = hit::sum_of(
        calc,
        Quantity::Poe1(Poe1Quantity::AilmentChance(ailment)),
        Op::Base,
        &hit_cx,
        Term::Step("ailment-chance-on-hit"),
        &mut reasons,
    )?;
    let hundred = phases::constant(calc, "hundred", 100.0)?;
    let on_hit = phases::pair(
        calc,
        on_hit,
        hundred,
        Term::Step("ailment-chance-capped"),
        true,
    )?;
    if on_hit.value == 0.0 && (!spec.crit_always || hit.crit.value == 0.0) {
        return Ok(None);
    }
    // Elemental Overload's `AilmentsAreNeverFromCrit`: the chance on a crit
    // is the chance on a hit, and no critical DoT multiplier (4176 to 4180,
    // 5015 to 5018).
    let never_from_crit = hit::flag_of(
        calc,
        Quantity::Poe1(Poe1Quantity::AilmentsNeverFromCrit),
        &hit_cx,
        &mut reasons,
    )?;
    let on_crit = match never_from_crit {
        Some(_) => on_hit,
        None if spec.crit_always => phases::constant(calc, "ailment-chance-on-crit", 100.0)?,
        None => on_hit,
    };
    if on_hit.value == 0.0 && (on_crit.value == 0.0 || hit.crit.value == 0.0) {
        return Ok(None);
    }

    // Step 5 first: no source damage, no ailment.
    let ailment_flags = ailment.flag() | Flags::dot_keyword(spec.dot_type);
    let cx = ailment_context(calc, hit, ailment_flags);
    let [min, max] = source_damage(calc, hit, &spec, &cx, &hit_cx, &mut reasons)?;
    if min.value == 0.0 && max.value == 0.0 {
        return Ok(None);
    }

    // Step 3: rate and duration (the rate in the hit's config, with the
    // ailment's flag so only its and every ailment's rows apply). PoB's
    // bleed and poison rate is `mod(Faster) + enemy inc / 100` with no
    // slower term (4315, 4575); `faster / slower` is the same while no
    // bleed or poison row maps to `AilmentSlower` and the enemy's own
    // faster bleed and poison are unmapped (P6-2c review).
    let rate_cx = calc.context(hit.flags | ailment.flag());
    let faster = modifier(
        calc,
        Quantity::Poe1(Poe1Quantity::AilmentFaster),
        &rate_cx,
        &mut reasons,
    )?;
    let slower = modifier(
        calc,
        Quantity::Poe1(Poe1Quantity::AilmentSlower),
        &rate_cx,
        &mut reasons,
    )?;
    let rate_mod = hit::ratio(calc, faster, slower, Term::Step("ailment-rate-mod"))?;
    // `calcLib.mod(skill, dotCfg, durations) x calcLib.mod(enemy, nil,
    // self durations)`.
    let duration_mod = modifier(
        calc,
        Quantity::Poe1(Poe1Quantity::AilmentDuration),
        &cx,
        &mut reasons,
    )?;
    let enemy_duration = Quantity::Poe1(Poe1Quantity::EnemyAilmentDuration);
    let duration_mod = if phases::has(calc, enemy_duration) {
        let on_enemy = modifier(calc, enemy_duration, &cx, &mut reasons)?;
        phases::product(
            calc,
            &[duration_mod, on_enemy],
            Term::Step("ailment-duration-mods"),
        )?
    } else {
        duration_mod
    };
    let zero_value = phases::constant(calc, "zero", 0.0)?;
    let duration_mod = phases::pair(
        calc,
        duration_mod,
        zero_value,
        Term::Step("ailment-duration-mod"),
        false,
    )?;
    let base_duration = phases::constant(calc, "ailment-duration-base", spec.duration_base)?;
    let scaled = phases::product(
        calc,
        &[base_duration, duration_mod],
        Term::Step("ailment-duration-scaled"),
    )?;
    let duration = hit::ratio(calc, scaled, rate_mod, Term::Step("ailment-duration"))?;
    // The enemy's debuffs expiring slower or faster (P6-11cv; PoB's
    // `debuffDurationMult = 1 / max(BuffExpirationSlowCap,
    // mod(enemy, BuffExpireFaster))`, `CalcOffence.lua` 1457 to 1459, on
    // ignite, bleed, and poison, 4317, 4588, 4904).
    let expire = Quantity::Poe1(Poe1Quantity::EnemyBuffExpireFaster);
    let duration = if phases::has(calc, expire) {
        let faster = modifier(calc, expire, &cx, &mut reasons)?;
        let cap = phases::constant(calc, "buff-expiration-slow-cap", BUFF_EXPIRATION_SLOW_CAP)?;
        let faster = phases::pair(calc, faster, cap, Term::Step("debuff-expiry"), false)?;
        hit::ratio(
            calc,
            duration,
            faster,
            Term::Step("ailment-duration-debuff"),
        )?
    } else {
        duration
    };

    // Step 4: the chance per hit, the stacks, and the roll.
    let one = phases::constant(calc, "one", 1.0)?;
    let crit_share = hit::percent_of(calc, hit.crit, Term::Step("crit-share"))?;
    let non_crit_share = hit::minus(calc, one, crit_share, Term::Step("non-crit-share"))?;
    let on_hit_share = hit::percent_of(calc, on_hit, Term::Step("ailment-on-hit-share"))?;
    let from_hits = phases::product(
        calc,
        &[on_hit_share, non_crit_share],
        Term::Step("ailment-chance-from-hits"),
    )?;
    let on_crit_share = hit::percent_of(calc, on_crit, Term::Step("ailment-on-crit-share"))?;
    let from_crits = phases::product(
        calc,
        &[on_crit_share, hit.crit],
        Term::Step("ailment-chance-from-crits-scaled"),
    )?;
    let from_crits = hit::percent_of(calc, from_crits, Term::Step("ailment-chance-from-crits"))?;
    let chance = phases::total(calc, &[from_hits, from_crits], Term::Step("ailment-chance"))?;
    let hit_share = hit::percent_of(calc, hit.hit_chance, Term::Step("hit-share"))?;
    let stacks = phases::product(
        calc,
        &[hit_share, chance],
        Term::Step("ailment-applications"),
    )?;
    let stacks = phases::product(
        calc,
        &[stacks, duration, hit.rate],
        Term::Step("ailment-stacks"),
    )?;
    let roll = if spec.roll != Roll::Midpoint && stacks.value > 1.0 {
        // `(stacks - (maxStacks - 1) / 2) / (stacks + 1) x 100`, one stack.
        let plus_one = phases::total(calc, &[stacks, one], Term::Step("ailment-stacks-plus-one"))?;
        let share = hit::ratio(calc, stacks, plus_one, Term::Step("ailment-roll-share"))?;
        phases::product(calc, &[share, hundred], Term::Step("ailment-roll"))?
    } else {
        phases::constant(calc, "ailment-roll-middle", 50.0)?
    };

    // Step 5: the DoT multipliers (the type's own too), from hits and from
    // crits.
    let mult_cx = ailment_context(calc, hit, ailment_flags | Flags::damage(spec.dot_type));
    let mult = hit::sum_of(
        calc,
        Quantity::DotMultiplier,
        Op::Base,
        &mult_cx,
        Term::Step("ailment-dot-multiplier"),
        &mut reasons,
    )?;
    // From crits, the character template's `critical_ailment_dot_multiplier_+`
    // too (PoB's `DotMultiplier` BASE with `Condition:CriticalStrike`,
    // `CalcSetup.lua` 502; the crit sub-pass); a minion's is the monster
    // template's (`initMinionModDB`, `CalcPerform.lua` 1147; P6-7c3d).
    let (constant, table) = match calc.actor() {
        crate::engine::Actor::Player => (
            data.character_constant(CRIT_AILMENT_DOT_MULTIPLIER),
            "character_constants",
        ),
        crate::engine::Actor::Minion => (
            data.monster_constant(CRIT_AILMENT_DOT_MULTIPLIER),
            "monster_constants",
        ),
    };
    let crit_bonus = match constant {
        Some(_) if never_from_crit.is_some() => 0.0,
        Some(v) => {
            #[expect(clippy::cast_precision_loss, reason = "a small integer, exact as f64")]
            let v = v as f64;
            v
        }
        None => {
            reasons
                .indeterminate
                .insert(crate::state::IndeterminateReason::MissingData {
                    table,
                    key: CRIT_AILMENT_DOT_MULTIPLIER.to_owned(),
                });
            0.0
        }
    };
    let crit_bonus = phases::constant(calc, "critical-ailment-dot-multiplier", crit_bonus)?;
    let crit_mult = phases::total(
        calc,
        &[mult, crit_bonus],
        Term::Step("ailment-crit-dot-multiplier"),
    )?;
    let crit_mult = phases::factor(
        calc,
        crit_mult,
        Term::Step("ailment-crit-dot-multiplier-factor"),
    )?;
    let mult = phases::factor(calc, mult, Term::Step("ailment-dot-multiplier-factor"))?;
    let (source, crit_source) = match spec.roll {
        Roll::SourceThenMultiplier => {
            let spread = hit::minus(calc, max, min, Term::Step("ailment-source-spread"))?;
            let spread =
                phases::product(calc, &[spread, roll], Term::Step("ailment-spread-rolled"))?;
            let spread = hit::percent_of(calc, spread, Term::Step("ailment-spread-share"))?;
            let rolled = phases::total(calc, &[min, spread], Term::Step("ailment-source-rolled"))?;
            (
                phases::product(calc, &[rolled, mult], Term::Step("ailment-source"))?,
                phases::product(
                    calc,
                    &[rolled, crit_mult],
                    Term::Step("ailment-crit-source"),
                )?,
            )
        }
        Roll::MultiplierFirst => {
            let at = |calc: &mut Calc<'_>, m: Valued| -> Result<Valued, CalcError> {
                let lo = phases::product(calc, &[min, m], Term::Step("ailment-source-min-scaled"))?;
                let hi = phases::product(calc, &[max, m], Term::Step("ailment-source-max-scaled"))?;
                let spread = hit::minus(calc, hi, lo, Term::Step("ailment-source-spread"))?;
                let spread =
                    phases::product(calc, &[spread, roll], Term::Step("ailment-spread-rolled"))?;
                let spread = hit::percent_of(calc, spread, Term::Step("ailment-spread-share"))?;
                phases::total(calc, &[lo, spread], Term::Step("ailment-source"))
            };
            (at(calc, mult)?, at(calc, crit_mult)?)
        }
        Roll::Midpoint => {
            let sum = phases::total(calc, &[min, max], Term::Step("ailment-source-sum"))?;
            let two = phases::constant(calc, "two", 2.0)?;
            let middle = hit::ratio(calc, sum, two, Term::Step("ailment-source-middle"))?;
            (
                phases::product(calc, &[middle, mult], Term::Step("ailment-source"))?,
                phases::product(
                    calc,
                    &[middle, crit_mult],
                    Term::Step("ailment-crit-source"),
                )?,
            )
        }
    };

    // Step 6: the crit chance the ailments see, then the weighted base.
    let ailment_crit = if spec.crit_over_stacks {
        let exponent = phases::pair(
            calc,
            one,
            stacks,
            Term::Step("ailment-crit-exponent"),
            false,
        )?;
        let survives = hit::power_of(
            calc,
            non_crit_share,
            exponent,
            Term::Step("no-crit-over-stacks"),
        )?;
        let any_crit = hit::minus(calc, one, survives, Term::Step("crit-over-stacks"))?;
        phases::product(
            calc,
            &[hundred, any_crit],
            Term::Step("ailment-crit-chance"),
        )?
    } else {
        hit.crit
    };
    let ailment_crit_share = hit::percent_of(calc, ailment_crit, Term::Step("ailment-crit-share"))?;
    let ailment_non_crit = hit::minus(
        calc,
        one,
        ailment_crit_share,
        Term::Step("ailment-non-crit-share"),
    )?;
    let chance_from_hit = phases::product(
        calc,
        &[on_hit, ailment_non_crit],
        Term::Step("ailment-weight-hits"),
    )?;
    let chance_from_crit = phases::product(
        calc,
        &[on_crit, ailment_crit],
        Term::Step("ailment-weight-crits-scaled"),
    )?;
    let chance_from_crit =
        hit::percent_of(calc, chance_from_crit, Term::Step("ailment-weight-crits"))?;
    let weights = phases::total(
        calc,
        &[chance_from_hit, chance_from_crit],
        Term::Step("ailment-weights"),
    )?;
    let base_hit = phases::product(
        calc,
        &[source, chance_from_hit],
        Term::Step("ailment-base-hits-scaled"),
    )?;
    let base_hit = hit::ratio(calc, base_hit, weights, Term::Step("ailment-base-hits"))?;
    let base_crit = phases::product(
        calc,
        &[crit_source, chance_from_crit],
        Term::Step("ailment-base-crits-scaled"),
    )?;
    let base_crit = hit::ratio(calc, base_crit, weights, Term::Step("ailment-base-crits"))?;
    let base = phases::total(
        calc,
        &[base_hit, base_crit],
        Term::Step("ailment-base-weighted"),
    )?;
    // Ignite and poison `x percent`; bleed `x 70 / 100`.
    let base = match ailment {
        Ailment::Ignite => {
            let percent = phases::constant(calc, "ignite-percent-base", IGNITE_PERCENT_BASE)?;
            phases::product(calc, &[base, percent], Term::Base)?
        }
        Ailment::Poison => {
            let percent = phases::constant(calc, "poison-percent-base", POISON_PERCENT_BASE)?;
            phases::product(calc, &[base, percent], Term::Base)?
        }
        Ailment::Bleed => {
            let percent = phases::constant(calc, "bleed-percent-base", BLEED_PERCENT_BASE)?;
            let scaled =
                phases::product(calc, &[base, percent], Term::Step("bleed-percent-scaled"))?;
            hit::percent_of(calc, scaled, Term::Base)?
        }
    };
    // Ruthless's ailment effect on bleed and poison (`CalcOffence.lua`
    // 4451, 4721; P6-11bt), summed with the hit's context as PoB's `cfg`.
    let base = match ailment {
        Ailment::Bleed | Ailment::Poison => {
            match hit::ruthless_effect(calc, &hit_cx, super::RuthlessValue::Ailment, &mut reasons)?
            {
                Some(r) => phases::product(calc, &[base, r], Term::Step("ruthless-ailment"))?,
                None => base,
            }
        }
        Ailment::Ignite => base,
    };

    // Step 7: effect, rate, the active ailments, the enemy.
    let effect = modifier(
        calc,
        Quantity::Poe1(Poe1Quantity::AilmentEffect),
        &cx,
        &mut reasons,
    )?;
    let enemy_effect = hit::dot_enemy_effect(calc, enemy, data, spec.dot_type, &cx, &mut reasons)?;
    let cap = phases::constant(calc, "dot-dps-cap", super::dot::DOT_DPS_CAP)?;
    let dps = if ailment == Ailment::Poison {
        // One poison's DPS, capped; then every poison's, capped.
        let single = phases::product(
            calc,
            &[base, effect, rate_mod, enemy_effect],
            Term::Step("poison-single-dps"),
        )?;
        let single = phases::pair(calc, single, cap, Term::Step("poison-single-capped"), true)?;
        let total = phases::product(calc, &[single, stacks], Term::Step("poison-dps"))?;
        phases::pair(calc, total, cap, Term::Result, true)?
    } else {
        let active = phases::pair(calc, stacks, one, Term::Step("active-ailments"), true)?;
        let dps = phases::product(
            calc,
            &[base, effect, rate_mod, active, enemy_effect],
            Term::Step("ailment-dps"),
        )?;
        phases::pair(calc, dps, cap, Term::Result, true)?
    };
    Ok((dps.value > 0.0).then_some((dps, reasons)))
}

/// PoB's `data.misc.ImpaleStoredDamageBase` (`Modules/Data.lua` 262): an
/// impale stores 10% of the hit's physical damage.
pub const IMPALE_STORED_DAMAGE_BASE: f64 = 0.1;
/// The character template's impale stacks (PoB's `ImpaleStacksMax` BASE,
/// `CalcSetup.lua` 48).
pub const IMPALE_STACKS_STAT: &str = "impaled_debuff_number_of_reflected_hits";

/// The main skill's impale (P6-2d; PoB v2.67.2 `CalcOffence.lua` 5472 to
/// 5520 and 6027 to 6047): its DPS with the reasons of what it read, or
/// `None` when the hit cannot impale. Every impale on the enemy (PoB's
/// default stacks, `CalcPerform.lua` 3321 to 3326: the maximum), each
/// storing `0.1 x (1 + effect inc / 100) x round(effect more, 2)` of the
/// hit's physical average before the enemy, reduced by the enemy's armour
/// against the stored damage, times the chance to impale and the enemy's
/// damage taken from hits: `stored hit x ((1 + modifier) - 1) x hit chance
/// / 100 x rate`. A configured number of impales on the enemy is an
/// unmodelled config input (a reason); so are the additional duration
/// chance and the impale's own physical damage reduction stats.
///
/// # Errors
/// A graph error.
pub(super) fn impale(
    calc: &mut Calc<'_>,
    hit: &AilmentHit<'_>,
    enemy: &Enemy,
    data: &dyn CalcData,
) -> Result<Option<(Valued, Reasons)>, CalcError> {
    let mut reasons = Reasons::new();
    let [Some(stored_hit), Some(stored_crit)] = hit.sources.physical else {
        return Ok(None);
    };
    let hit_cx = calc.context(hit.flags).with_state(
        ConditionKey::new(keystones::CRIT_IN_PAST_8_SEC),
        hit.crit.value != 0.0,
    );
    if hit::cannot_deal(calc, DamageType::Physical, &hit_cx, &mut reasons)? {
        return Ok(None);
    }
    // `ImpaleChance = min(100, Sum)`; on a crit the same sum (no mapped
    // stat is crit only).
    let chance = hit::sum_of(
        calc,
        Quantity::Poe1(Poe1Quantity::ImpaleChance),
        Op::Base,
        &hit_cx,
        Term::Step("impale-chance-sum"),
        &mut reasons,
    )?;
    let hundred = phases::constant(calc, "hundred", 100.0)?;
    let chance = phases::pair(calc, chance, hundred, Term::Step("impale-chance"), true)?;
    if chance.value == 0.0 {
        return Ok(None);
    }
    // `impaleChance = min(chance / 100, 1) x (1 - crit) + min(chance / 100,
    // 1) x crit`, `crit = CritChance / 100`.
    let one = phases::constant(calc, "one", 1.0)?;
    let share = hit::percent_of(calc, chance, Term::Step("impale-chance-share"))?;
    let share = phases::pair(calc, share, one, Term::Step("impale-chance-capped"), true)?;
    let crit = hit::percent_of(calc, hit.crit, Term::Step("crit-share"))?;
    let non_crit = hit::minus(calc, one, crit, Term::Step("non-crit-share"))?;
    let from_hits = phases::product(calc, &[share, non_crit], Term::Step("impale-from-hits"))?;
    let from_crits = phases::product(calc, &[share, crit], Term::Step("impale-from-crits"))?;
    let impale_chance = phases::total(
        calc,
        &[from_hits, from_crits],
        Term::Step("impale-chance-per-hit"),
    )?;
    // The stacks: the maximum (`ImpaleStacksMax` BASE x (1 + additional
    // duration chance / 100); the chance since P6-11dp, `CalcOffence.lua`
    // 5477).
    let base_stacks = match data.character_constant(IMPALE_STACKS_STAT) {
        Some(v) => {
            #[expect(clippy::cast_precision_loss, reason = "a small integer, exact as f64")]
            let v = v as f64;
            v
        }
        None => {
            reasons
                .indeterminate
                .insert(crate::state::IndeterminateReason::MissingData {
                    table: "character_constants",
                    key: IMPALE_STACKS_STAT.to_owned(),
                });
            0.0
        }
    };
    let base_stacks = phases::constant(calc, "impale-stacks-base", base_stacks)?;
    let more_stacks = hit::sum_of(
        calc,
        Quantity::Poe1(Poe1Quantity::ImpaleStacksMax),
        Op::Base,
        &hit_cx,
        Term::Step("impale-stacks-added"),
        &mut reasons,
    )?;
    let stacks = phases::total(
        calc,
        &[base_stacks, more_stacks],
        Term::Step("impale-stacks"),
    )?;
    let chance_q = Quantity::Poe1(Poe1Quantity::ImpaleAdditionalDurationChance);
    let stacks = if phases::has(calc, chance_q) {
        let chance = hit::sum_of(
            calc,
            chance_q,
            Op::Base,
            &hit_cx,
            Term::Step("impale-additional-duration-chance"),
            &mut reasons,
        )?;
        let factor = phases::factor(calc, chance, Term::Step("impale-additional-duration"))?;
        phases::product(calc, &[stacks, factor], Term::Step("impale-stacks-lasting"))?
    } else {
        stacks
    };
    // The stored hit: `0 + crit average x crit + hit average x (1 - crit)`
    // (PoB adds the crit pass first, 3415 to 3420).
    let from_crit_pass = phases::product(
        calc,
        &[stored_crit, crit],
        Term::Step("impale-stored-from-crits"),
    )?;
    let from_hit_pass = phases::product(
        calc,
        &[stored_hit, non_crit],
        Term::Step("impale-stored-from-hits"),
    )?;
    let stored_hit = phases::total(
        calc,
        &[from_crit_pass, from_hit_pass],
        Term::Step("impale-stored-hit"),
    )?;
    // `storedExpectedDamageModifier = (1 + inc / 100) x round(more, 2)`.
    let effect = Quantity::Poe1(Poe1Quantity::ImpaleEffect);
    let inc = hit::sum_of(
        calc,
        effect,
        Op::Increased,
        &hit_cx,
        Term::Step("impale-effect-inc"),
        &mut reasons,
    )?;
    let inc = hit::percent_of(calc, inc, Term::Step("impale-effect-share"))?;
    let inc = phases::total(calc, &[one, inc], Term::Step("impale-effect-increased"))?;
    let (_, more) = hit::modifiers_of(calc, effect, &hit_cx, &mut reasons)?;
    let modifier = phases::product(calc, &[inc, more], Term::Step("impale-effect"))?;
    let base = phases::constant(calc, "impale-stored-damage-base", IMPALE_STORED_DAMAGE_BASE)?;
    let stored = phases::product(calc, &[base, modifier], Term::Step("impale-stored-damage"))?;
    let hit_mod = phases::product(calc, &[stored, stacks], Term::Step("impale-hit-damage-mod"))?;
    // The enemy's armour against the stored damage (`armourReductionF(armour,
    // hitMod x stored hit)`), its physical damage reduction 0, at most the
    // cap.
    let raw = phases::product(calc, &[hit_mod, stored_hit], Term::Step("impale-raw"))?;
    let resist = hit::impale_armour_reduction(calc, enemy, data, raw, &mut reasons)?;
    // `impaleTaken`: the enemy's damage taken from hits, physical too.
    let taken = hit::hit_damage_taken(calc, DamageType::Physical, &mut reasons)?;
    let resist_share = hit::percent_of(calc, resist, Term::Step("impale-reduction-share"))?;
    let through = hit::minus(calc, one, resist_share, Term::Step("impale-through"))?;
    let mut factors = vec![hit_mod, through, impale_chance];
    if let Some(t) = taken {
        factors.push(t);
    }
    let damage_mod = phases::product(calc, &factors, Term::Step("impale-damage-modifier"))?;
    let modifier = phases::total(calc, &[one, damage_mod], Term::Step("impale-modifier"))?;
    // `ImpaleDPS = stored hit x (modifier - 1) x hit chance / 100 x 1`, then
    // `x rate`.
    let gain = hit::minus(calc, modifier, one, Term::Step("impale-gain"))?;
    let hit_share = hit::percent_of(calc, hit.hit_chance, Term::Step("hit-share"))?;
    let per_hit = phases::product(
        calc,
        &[stored_hit, gain, hit_share],
        Term::Step("impale-per-hit"),
    )?;
    let dps = phases::product(calc, &[per_hit, hit.rate], Term::Result)?;
    Ok((dps.value > 0.0).then_some((dps, reasons)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ailment_names_and_flags_should_be_stable() {
        assert_eq!(Ailment::Ignite.name(), "ignite");
        assert_eq!(Ailment::Bleed.flag(), Flags::BLEED);
        assert_eq!(IGNITE_PERCENT_BASE, 0.9);
        assert_eq!(IGNITE_DURATION_BASE, 4.0);
    }

    #[test]
    fn the_ailment_config_should_keep_only_keywords() {
        let hit = Flags::SPELL | Flags::HIT | Flags::AREA | Flags::SKILL_FIRE | Flags::CAST;
        assert_eq!(Flags::from_bits_and(hit, KEYWORDS), Flags::SKILL_FIRE);
    }

    /// Whether a skill sets one of the ailment's unmodelled skill-data keys.
    fn reads_skill_keys(id: &str, ailment: Ailment) -> bool {
        let pob = super::super::pob_skill::skill(id).expect("PoB has the skill");
        spec(ailment)
            .skill_keys
            .iter()
            .any(|k| pob.skill_data(k).is_some_and(Value::truthy))
    }

    #[test]
    fn skills_that_change_their_ailment_should_be_reasons() {
        // Puncture's bleed and Viper Strike's poison take the skill's
        // duration (P6-2c review); Smite sets none of the keys.
        assert!(reads_skill_keys("Puncture", Ailment::Bleed));
        assert!(reads_skill_keys("ViperStrike", Ailment::Poison));
        assert!(!reads_skill_keys("Smite", Ailment::Bleed));
        assert!(!reads_skill_keys("Smite", Ailment::Poison));
        assert!(!reads_skill_keys("Puncture", Ailment::Ignite));
    }

    #[test]
    fn only_poison_should_read_chaos_modifiers_for_physical() {
        assert!(spec(Ailment::Poison).physical_as_chaos);
        assert!(!spec(Ailment::Bleed).physical_as_chaos);
        assert!(!spec(Ailment::Ignite).physical_as_chaos);
    }
}
