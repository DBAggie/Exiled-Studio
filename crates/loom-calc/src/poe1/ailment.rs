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
use super::{Poe1Quantity, keystones};
use crate::condition::{ConditionKey, Flags, QueryContext};
use crate::engine::Calc;
use crate::error::CalcError;
use crate::graph::Term;
use crate::input::CalcData;
use crate::quantity::{DamageType, Op, Quantity};
use crate::state::Reasons;

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

/// The character template's damage over time multiplier of ailments from
/// critical strikes (PoB's `data.characterConstants`).
pub const CRIT_AILMENT_DOT_MULTIPLIER: &str = "critical_ailment_dot_multiplier_+";

/// The skill keyword flags an ailment's config keeps from its hit.
const KEYWORDS: Flags = Flags::SKILL_PHYSICAL
    .union(Flags::KEYWORD_ATTACK)
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
}

/// The ailment's config (module docs, step 2) with the hit's settled crit
/// state.
fn ailment_context(calc: &Calc<'_>, hit: &AilmentHit<'_>, extra: Flags) -> QueryContext {
    let flags = Flags::from_bits_and(hit.flags, KEYWORDS) | Flags::DOT | Flags::AILMENT | extra;
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

/// The main skill's ignite (module docs): its DPS with the reasons of what
/// it read, or `None` when the hit cannot ignite (no fire damage, or no
/// chance on hit and no crit).
///
/// # Errors
/// A graph error.
pub(super) fn ignite(
    calc: &mut Calc<'_>,
    hit: &AilmentHit<'_>,
    enemy: &Enemy,
    data: &dyn CalcData,
) -> Result<Option<(Valued, Reasons)>, CalcError> {
    let mut reasons = Reasons::new();
    let fire = DamageType::Fire;
    let Some(fire_index) = hit::TYPES.iter().position(|t| *t == fire) else {
        return Ok(None);
    };
    let hit_cx = calc.context(hit.flags).with_state(
        ConditionKey::new(keystones::CRIT_IN_PAST_8_SEC),
        hit.crit.value != 0.0,
    );
    // `canDeal.Fire`.
    if hit::cannot_deal(calc, fire, &hit_cx, &mut reasons)? {
        return Ok(None);
    }
    // Step 1: the chances.
    let chance_quantity = Quantity::Poe1(Poe1Quantity::AilmentChance(Ailment::Ignite));
    let on_hit = hit::sum_of(
        calc,
        chance_quantity,
        Op::Base,
        &hit_cx,
        Term::Step("ignite-chance-on-hit"),
        &mut reasons,
    )?;
    let hundred = phases::constant(calc, "hundred", 100.0)?;
    let on_hit = phases::pair(
        calc,
        on_hit,
        hundred,
        Term::Step("ignite-chance-capped"),
        true,
    )?;
    if on_hit.value == 0.0 && hit.crit.value == 0.0 {
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
        None => phases::constant(calc, "ignite-chance-on-crit", 100.0)?,
    };
    if on_hit.value == 0.0 && (on_crit.value == 0.0 || hit.crit.value == 0.0) {
        return Ok(None);
    }

    // Step 5 first: no fire source, no ignite.
    let cx = ailment_context(calc, hit, Flags::IGNITE | Flags::FIRE_DOT);
    let mut zero = None;
    let [min, max] = hit::calc_damage(
        calc,
        fire_index,
        Flags::NONE,
        &hit.sources.bases,
        hit.sources.conversion.as_ref(),
        &cx,
        None,
        &mut reasons,
        &mut zero,
    )?;
    let kept = hit
        .sources
        .conversion
        .as_ref()
        .and_then(|c| c.kept.get(fire_index).copied().flatten());
    let (min, max) = match kept {
        Some(k) => (
            phases::product(calc, &[min, k], Term::Step("ignite-source-min"))?,
            phases::product(calc, &[max, k], Term::Step("ignite-source-max"))?,
        ),
        None => (min, max),
    };
    if min.value == 0.0 && max.value == 0.0 {
        return Ok(None);
    }

    // Step 3: rate and duration (the rate in the hit's config, with the
    // ignite's flag so only ignite's and every ailment's rows apply).
    let rate_cx = calc.context(hit.flags | Flags::IGNITE);
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
    let rate_mod = hit::ratio(calc, faster, slower, Term::Step("ignite-rate-mod"))?;
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
            Term::Step("ignite-duration-mods"),
        )?
    } else {
        duration_mod
    };
    let zero_value = phases::constant(calc, "zero", 0.0)?;
    let duration_mod = phases::pair(
        calc,
        duration_mod,
        zero_value,
        Term::Step("ignite-duration-mod"),
        false,
    )?;
    let base_duration = phases::constant(calc, "ignite-duration-base", IGNITE_DURATION_BASE)?;
    let scaled = phases::product(
        calc,
        &[base_duration, duration_mod],
        Term::Step("ignite-duration-scaled"),
    )?;
    let duration = hit::ratio(calc, scaled, rate_mod, Term::Step("ignite-duration"))?;

    // Step 4: the chance per hit, the stacks, and the roll.
    let one = phases::constant(calc, "one", 1.0)?;
    let crit_share = hit::percent_of(calc, hit.crit, Term::Step("crit-share"))?;
    let non_crit_share = hit::minus(calc, one, crit_share, Term::Step("non-crit-share"))?;
    let on_hit_share = hit::percent_of(calc, on_hit, Term::Step("ignite-on-hit-share"))?;
    let from_hits = phases::product(
        calc,
        &[on_hit_share, non_crit_share],
        Term::Step("ignite-chance-from-hits"),
    )?;
    let on_crit_share = hit::percent_of(calc, on_crit, Term::Step("ignite-on-crit-share"))?;
    let from_crits = phases::product(
        calc,
        &[on_crit_share, hit.crit],
        Term::Step("ignite-chance-from-crits-scaled"),
    )?;
    let from_crits = hit::percent_of(calc, from_crits, Term::Step("ignite-chance-from-crits"))?;
    let chance = phases::total(calc, &[from_hits, from_crits], Term::Step("ignite-chance"))?;
    let hit_share = hit::percent_of(calc, hit.hit_chance, Term::Step("hit-share"))?;
    let stacks = phases::product(
        calc,
        &[hit_share, chance],
        Term::Step("ignite-applications"),
    )?;
    let stacks = phases::product(
        calc,
        &[stacks, duration, hit.rate],
        Term::Step("ignite-stacks"),
    )?;
    let roll = if stacks.value > 1.0 {
        // `(stacks - (maxStacks - 1) / 2) / (stacks + 1) x 100`, one stack.
        let plus_one = phases::total(calc, &[stacks, one], Term::Step("ignite-stacks-plus-one"))?;
        let share = hit::ratio(calc, stacks, plus_one, Term::Step("ignite-roll-share"))?;
        phases::product(calc, &[share, hundred], Term::Step("ignite-roll"))?
    } else {
        phases::constant(calc, "ignite-roll-middle", 50.0)?
    };

    // Step 5: the source damage at the roll, times the DoT multiplier (fire
    // type, so the fire DoT multiplier applies).
    let mult_cx = ailment_context(calc, hit, Flags::IGNITE | Flags::FIRE_DOT | Flags::FIRE);
    let mult = hit::sum_of(
        calc,
        Quantity::DotMultiplier,
        Op::Base,
        &mult_cx,
        Term::Step("ignite-dot-multiplier"),
        &mut reasons,
    )?;
    // From crits, the character template's `critical_ailment_dot_multiplier_+`
    // too (PoB's `DotMultiplier` BASE with `Condition:CriticalStrike`,
    // `CalcSetup.lua` 502; the crit sub-pass, `CritIgniteDotMulti`).
    let crit_bonus = match data.character_constant(CRIT_AILMENT_DOT_MULTIPLIER) {
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
                    table: "character_constants",
                    key: CRIT_AILMENT_DOT_MULTIPLIER.to_owned(),
                });
            0.0
        }
    };
    let crit_bonus = phases::constant(calc, "critical-ailment-dot-multiplier", crit_bonus)?;
    let crit_mult = phases::total(
        calc,
        &[mult, crit_bonus],
        Term::Step("ignite-crit-dot-multiplier"),
    )?;
    let crit_mult = phases::factor(
        calc,
        crit_mult,
        Term::Step("ignite-crit-dot-multiplier-factor"),
    )?;
    let mult = phases::factor(calc, mult, Term::Step("ignite-dot-multiplier-factor"))?;
    let spread = hit::minus(calc, max, min, Term::Step("ignite-source-spread"))?;
    let spread = phases::product(calc, &[spread, roll], Term::Step("ignite-spread-rolled"))?;
    let spread = hit::percent_of(calc, spread, Term::Step("ignite-spread-share"))?;
    let rolled = phases::total(calc, &[min, spread], Term::Step("ignite-source-rolled"))?;
    let source = phases::product(calc, &[rolled, mult], Term::Step("ignite-source"))?;
    let crit_source =
        phases::product(calc, &[rolled, crit_mult], Term::Step("ignite-crit-source"))?;

    // Step 6: the crit chance over the stacks, then the weighted base.
    let exponent = phases::pair(calc, one, stacks, Term::Step("ignite-crit-exponent"), false)?;
    let survives = hit::power_of(
        calc,
        non_crit_share,
        exponent,
        Term::Step("no-crit-over-stacks"),
    )?;
    let any_crit = hit::minus(calc, one, survives, Term::Step("crit-over-stacks"))?;
    let ailment_crit =
        phases::product(calc, &[hundred, any_crit], Term::Step("ignite-crit-chance"))?;
    let ailment_crit_share = hit::percent_of(calc, ailment_crit, Term::Step("ignite-crit-share"))?;
    let ailment_non_crit = hit::minus(
        calc,
        one,
        ailment_crit_share,
        Term::Step("ignite-non-crit-share"),
    )?;
    let chance_from_hit = phases::product(
        calc,
        &[on_hit, ailment_non_crit],
        Term::Step("ignite-weight-hits"),
    )?;
    let chance_from_crit = phases::product(
        calc,
        &[on_crit, ailment_crit],
        Term::Step("ignite-weight-crits-scaled"),
    )?;
    let chance_from_crit =
        hit::percent_of(calc, chance_from_crit, Term::Step("ignite-weight-crits"))?;
    let weights = phases::total(
        calc,
        &[chance_from_hit, chance_from_crit],
        Term::Step("ignite-weights"),
    )?;
    let base_hit = phases::product(
        calc,
        &[source, chance_from_hit],
        Term::Step("ignite-base-hits-scaled"),
    )?;
    let base_hit = hit::ratio(calc, base_hit, weights, Term::Step("ignite-base-hits"))?;
    let base_crit = phases::product(
        calc,
        &[crit_source, chance_from_crit],
        Term::Step("ignite-base-crits-scaled"),
    )?;
    let base_crit = hit::ratio(calc, base_crit, weights, Term::Step("ignite-base-crits"))?;
    let base = phases::total(
        calc,
        &[base_hit, base_crit],
        Term::Step("ignite-base-weighted"),
    )?;
    let percent = phases::constant(calc, "ignite-percent-base", IGNITE_PERCENT_BASE)?;
    let base = phases::product(calc, &[base, percent], Term::Base)?;

    // Step 7: effect, rate, active ignites, the enemy.
    let effect = modifier(
        calc,
        Quantity::Poe1(Poe1Quantity::AilmentEffect),
        &cx,
        &mut reasons,
    )?;
    let active = phases::pair(calc, stacks, one, Term::Step("active-ignites"), true)?;
    let enemy_effect = hit::dot_enemy_effect(calc, enemy, data, fire, &cx, &mut reasons)?;
    let dps = phases::product(
        calc,
        &[base, effect, rate_mod, active, enemy_effect],
        Term::Step("ignite-dps"),
    )?;
    let cap = phases::constant(calc, "dot-dps-cap", super::dot::DOT_DPS_CAP)?;
    let dps = phases::pair(calc, dps, cap, Term::Result, true)?;
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
}
