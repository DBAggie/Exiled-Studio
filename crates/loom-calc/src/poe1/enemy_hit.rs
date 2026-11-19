//! The enemy's hit on the character (Phase 6 P6-10b, part 12): PoB
//! v2.67.2's `<Type>TakenHitMult` and `<Type>DamageReduction`
//! (`Modules/CalcDefence.lua` 1635 to 2105, `calcs.buildDefenceEstimations`),
//! for the config's default damage category, `Average`.
//!
//! What PoB does:
//! - **The enemy's crit:** `EnemyCritChance = clamp(crit x (1 +
//!   EnemyCritChance INC / 100 + the enemy's CritChance INC / 100) x (1 -
//!   ConfiguredEvadeChance / 100), 0, 100)`, `crit` the config's
//!   `enemyCritChance` or its placeholder (5), `ConfiguredEvadeChance` the
//!   `Average` one, `(MeleeEvadeChance + ProjectileEvadeChance) / 4`
//!   (1658, PoB's own sum over four); `EnemyCritEffect = 1 + chance / 100 x
//!   damage / 100 x (1 - CritExtraDamageReduction / 100)`, `damage` the
//!   config's `enemyCritDamage` or the monster constant
//!   `base_critical_strike_multiplier` - 100, plus the enemy's
//!   `CritMultiplier` BASE.
//! - **The enemy's damage of each type:** the config's `enemy<Type>Damage`
//!   or its placeholder (`ConfigOptions.lua`, `enemyIsBoss`: `round(table
//!   x 1.5 x mult)` of the monster damage table at the boss kind's level,
//!   `mult` 1, `stdBossDPSMult` 4/4.4, `pinnacleBossDPSMult` 8/4.4,
//!   `uberBossDPSMult` 10/4.25; physical alone for a non-boss; chaos the
//!   rounded fifth over 2.5, or over 4 for the Uber boss), times the
//!   enemy's damage modifier and the crit effect. The penetration is the
//!   config's `enemy<Type>Pen` or the placeholder (`pinnacleBossPen` 3,
//!   `uberBossPen` 8 for the elements).
//! - **Taken as:** each type's share moved to another
//!   (`<Type>DamageTakenAs<Other>`, `<Type>DamageFromHitsTakenAs<Other>`,
//!   and the elemental forms), the rest `max(100 - moved, 0)`.
//! - **The hit:** for each type, `resMult = 1 - (resist - pen) / 100`; the
//!   armour's reduction `round(armour x applies / 100 / (armour x applies
//!   / 100 + damage x resMult x 5) x 100)` (`calcs.armourReduction`;
//!   physical's applies 100, `CalcDefence.lua` 1497), at most the maximum
//!   reduction, plus the base reduction (part 5), at most the maximum;
//!   `reductMult = 1 - max(min(max, total - overwhelm), 0) / 100`;
//!   `<Type>DamageReduction = 100 - reductMult x 100`.
//! - `takenMult`, for `Average`, is the mean of the type's spell and
//!   attack taken multipliers (`(1 + inc / 100) x more` of the type's
//!   damage taken when hit, part 10's names, with `SpellDamageTaken` or
//!   `AttackDamageTaken`); `TakenHit = max(damage x resMult x reductMult +
//!   takenFlat, 0) x takenMult x spellSuppressMult`, `takenFlat` the flat
//!   damage taken (attacks' and spells' halved, projectile ones' quartered);
//!   `<Type>TakenHitMult = TakenHit / damage`, 0 without damage.
//! - **Fortification** (`CalcPerform.lua` 632 to 653): while Fortified (the
//!   config's `buffFortification`), `stacks = min(overrideFortification or
//!   max, max)`, `max` the character constant `base_max_fortification`
//!   plus `MaximumFortification` BASE; `DamageTakenWhenHit` MORE `-stacks`,
//!   in the hit's more product.
//! - `spellSuppressMult` is `1 - SpellSuppressionEffect / 100 / 2` at an
//!   effective suppression chance of 100, else 1 (`SuppressionEffect` 40
//!   plus `SpellSuppressionEffect` BASE).
//!
//! Not modelled, so reasons on these fields: another damage category, an
//! unlucky estimate, a boss skill preset, the enemy's impale, armour based
//! on ward, ignored resistances or reductions, and the enemy's modifiers
//! the engine does not carry (curses that lower its damage are a marker,
//! [`super::gems::BuffStat::Defence`]).

use loom_domain::{ConfigInputKind, ConfigValue};

use super::enemy::{self, BossKind};
use super::phases::{self, Valued};
use super::{Poe1Field, Poe1Quantity, TakenOf, TakenWhen};
use crate::condition::Flags;
use crate::engine::Calc;
use crate::error::CalcError;
use crate::graph::Term;
use crate::quantity::{DamageType, Op, Quantity};
use crate::result::Field;
use crate::state::{IndeterminateReason, Reasons, UnsupportedReason};

/// The damage types in PoB's order (`dmgTypeList`).
const TYPES: [DamageType; 5] = [
    DamageType::Physical,
    DamageType::Lightning,
    DamageType::Cold,
    DamageType::Fire,
    DamageType::Chaos,
];

/// PoB's `data.misc.stdBossDPSMult`.
const STD_BOSS_DPS_MULT: f64 = 4.0 / 4.40;
/// PoB's `data.misc.pinnacleBossDPSMult`.
const PINNACLE_BOSS_DPS_MULT: f64 = 8.0 / 4.40;
/// PoB's `data.misc.pinnacleBossPen`.
const PINNACLE_BOSS_PEN: f64 = 15.0 / 5.0;
/// PoB's `data.misc.uberBossDPSMult`.
const UBER_BOSS_DPS_MULT: f64 = 10.0 / 4.25;
/// PoB's `data.misc.uberBossPen`.
const UBER_BOSS_PEN: f64 = 40.0 / 5.0;
/// PoB's enemy crit chance placeholder (`ConfigOptions.lua` 2063).
const ENEMY_CRIT_CHANCE: f64 = 5.0;
/// PoB's `data.misc.SuppressionEffect`.
const SUPPRESSION_EFFECT: f64 = 40.0;
/// The character constant of the largest damage reduction.
const MAX_STAT: &str = "maximum_physical_damage_reduction_%";

/// The config inputs that change the estimate in a way not modelled: a
/// damage category other than `Average`, an unlucky estimate, a boss skill
/// preset (their default values aside).
const UNMODELLED: [(&str, &str); 3] = [
    ("enemyDamageType", "Average"),
    ("EHPUnluckyWorstOf", "1"),
    ("presetBossSkills", "None"),
];

/// The fields of each type.
const fn fields(ty: DamageType) -> [Field; 2] {
    [
        Field::Poe1(Poe1Field::TakenHitMult(ty)),
        Field::Poe1(Poe1Field::DamageReduction(ty)),
    ]
}

/// The phase (module docs).
///
/// # Errors
/// A graph error.
#[expect(
    clippy::too_many_lines,
    reason = "PoB's one estimate, step by step with its citations"
)]
pub fn phase(calc: &mut Calc<'_>) -> Result<(), CalcError> {
    let input = *calc.input();
    let set = super::collect::active_config(input.build);
    // The config inputs not modelled.
    for (name, default) in UNMODELLED {
        let value = set.and_then(|s| super::config::last_input(s, name, ConfigInputKind::Input));
        let is_default = match value {
            None => true,
            Some(ConfigValue::String(s)) => s.as_str() == default,
            Some(ConfigValue::Number(n)) => n.value().0.to_string() == default,
            Some(ConfigValue::Boolean(_)) => false,
        };
        if !is_default {
            for ty in TYPES {
                for f in fields(ty) {
                    calc.set_unsupported(
                        f,
                        UnsupportedReason::UnmodelledConfig {
                            input: name.to_owned(),
                        },
                    );
                }
            }
            return Ok(());
        }
    }
    let read = |calc: &Calc<'_>, f: Field| {
        calc.field(f).and_then(|r| {
            Some(Valued {
                node: r.node?,
                value: r.value?,
            })
        })
    };
    let (Some(melee), Some(projectile), Some(armour)) = (
        read(calc, Field::Poe1(Poe1Field::MeleeEvadeChance)),
        read(calc, Field::Poe1(Poe1Field::ProjectileEvadeChance)),
        read(calc, Field::Armour),
    ) else {
        return Ok(());
    };
    let mut reasons = phases::marker_reasons(
        calc,
        |q| match q {
            Quantity::Poe1(Poe1Quantity::EnemyHitReason(stat)) => Some(stat),
            _ => None,
        },
        is_read,
    );
    let foe = enemy::enemy(input.build, input.data);
    let number = |name: &str| match set
        .and_then(|s| super::config::last_input(s, name, ConfigInputKind::Input))
    {
        Some(ConfigValue::Number(n)) => Some(n.value().0),
        _ => None,
    };
    // The monster damage at the boss kind's level.
    let level = match foe.kind {
        BossKind::Pinnacle => foe.level.max(84),
        BossKind::Uber => foe.level.max(85),
        BossKind::None | BossKind::Boss => foe.level,
    };
    let Some(table) = input.data.monster_damage(level) else {
        reasons
            .indeterminate
            .insert(IndeterminateReason::MissingData {
                table: "monster_levels",
                key: level.to_string(),
            });
        return Ok(());
    };
    let (mult, pen) = match foe.kind {
        BossKind::None => (1.0, 0.0),
        BossKind::Boss => (STD_BOSS_DPS_MULT, 0.0),
        BossKind::Pinnacle => (PINNACLE_BOSS_DPS_MULT, PINNACLE_BOSS_PEN),
        BossKind::Uber => (UBER_BOSS_DPS_MULT, UBER_BOSS_PEN),
    };
    let base_damage = crate::math::round_half_up(table * 1.5 * mult);
    let placeholder = |ty: DamageType| match (foe.kind, ty) {
        (_, DamageType::Physical) => base_damage,
        (BossKind::None, _) => 0.0,
        (BossKind::Uber, DamageType::Chaos) => crate::math::round_half_up(base_damage / 4.0),
        (_, DamageType::Chaos) => crate::math::round_half_up(base_damage / 2.5),
        _ => base_damage,
    };
    let pen_placeholder = |ty: DamageType| match ty {
        DamageType::Fire | DamageType::Cold | DamageType::Lightning => pen,
        _ => 0.0,
    };
    let cx = calc.context(Flags::NONE);
    // The enemy's crit (module docs).
    let crit = number("enemyCritChance").unwrap_or(ENEMY_CRIT_CHANCE);
    let crit_inc = sum(
        calc,
        Poe1Quantity::EnemyCritChance,
        Op::Increased,
        &cx,
        &mut reasons,
    )?;
    let evade = (melee.value + projectile.value) / 4.0;
    let crit_chance = (crit * (1.0 + crit_inc / 100.0) * (1.0 - evade / 100.0)).clamp(0.0, 100.0);
    let crit_damage = match number("enemyCritDamage") {
        Some(v) => v,
        None => {
            if let Some(m) = input
                .data
                .monster_constant("base_critical_strike_multiplier")
            {
                #[expect(clippy::cast_precision_loss, reason = "a small integer")]
                let m = m as f64;
                m - 100.0
            } else {
                reasons
                    .indeterminate
                    .insert(IndeterminateReason::MissingData {
                        table: "monster_constants",
                        key: "base_critical_strike_multiplier".to_owned(),
                    });
                0.0
            }
        }
    };
    let crit_damage = crit_damage.max(0.0);
    let crit_reduction = sum(
        calc,
        Poe1Quantity::CritExtraDamageReduction,
        Op::Base,
        &cx,
        &mut reasons,
    )?
    .min(100.0);
    let crit_effect =
        1.0 + crit_chance / 100.0 * (crit_damage / 100.0) * (1.0 - crit_reduction / 100.0);
    // The enemy's damage modifier (`calcLib.mod(enemyDB, "Damage", ...)`):
    // the delirium's increase.
    let (inc, more) = {
        let q = Quantity::Poe1(Poe1Quantity::EnemyDamage);
        let (store, graph) = calc.parts();
        let inc = store.increased(graph, q, &cx)?;
        let more = store.more_rounded(graph, q, &cx)?;
        reasons.extend(&inc.reasons);
        reasons.extend(&more.reasons);
        (inc.value, more.value)
    };
    let damage_mult = inc * more;
    // Each type's damage from the enemy.
    let mut enemy_damage = [0.0; 5];
    for (i, ty) in TYPES.into_iter().enumerate() {
        let name = format!("enemy{}Damage", pob_type(ty));
        let damage = number(&name).unwrap_or_else(|| placeholder(ty));
        enemy_damage[i] = damage * damage_mult * crit_effect;
    }
    // Taken as (1802 to 1879): each type's shift table, the same type's
    // share first, then each type's moved shares in PoB's order.
    let mut shares = [[0.0; 5]; 5];
    for (i, from) in TYPES.into_iter().enumerate() {
        let mut moved = 0.0;
        for (j, to) in TYPES.into_iter().enumerate() {
            if i == j {
                continue;
            }
            let mut share = 0.0;
            for hits in [false, true] {
                for of in taken_as_sources(from) {
                    share += sum(
                        calc,
                        Poe1Quantity::DamageTakenAs(of, to, hits),
                        Op::Base,
                        &cx,
                        &mut reasons,
                    )?;
                }
            }
            shares[i][j] = share;
            moved += share;
        }
        shares[i][i] = (100.0 - moved).max(0.0);
    }
    let mut taken = [0.0; 5];
    for i in 0..5 {
        taken[i] = enemy_damage[i] * shares[i][i] / 100.0;
    }
    for i in 0..5 {
        for j in 0..5 {
            if i != j {
                taken[j] += enemy_damage[i] * shares[i][j] / 100.0;
            }
        }
    }
    let max = match input.data.character_constant(MAX_STAT) {
        #[expect(clippy::cast_precision_loss, reason = "a small integer")]
        Some(v) => v as f64,
        None => 90.0,
    };
    // Fortification's less damage taken from hits.
    let fortify_more = if super::conditions::check(set, "buffFortification") == Some(true) {
        #[expect(clippy::cast_precision_loss, reason = "a small integer")]
        let base = input
            .data
            .character_constant("base_max_fortification")
            .map_or(20.0, |v| v as f64);
        let max = base
            + sum(
                calc,
                Poe1Quantity::MaximumFortification,
                Op::Base,
                &cx,
                &mut reasons,
            )?;
        let stacks = number("overrideFortification").unwrap_or(max).min(max);
        1.0 - stacks / 100.0
    } else {
        1.0
    };
    // Spell suppression.
    let suppression =
        read(calc, Field::Poe1(Poe1Field::SpellSuppressionChance)).map_or(0.0, |v| v.value);
    let suppress_mult = if suppression >= 100.0 {
        let effect = (SUPPRESSION_EFFECT
            + sum(
                calc,
                Poe1Quantity::SpellSuppressionEffect,
                Op::Base,
                &cx,
                &mut reasons,
            )?)
        .max(0.0);
        1.0 - effect / 100.0 / 2.0
    } else {
        1.0
    };
    for (i, ty) in TYPES.into_iter().enumerate() {
        let mut type_reasons = reasons.clone();
        let resist = if ty == DamageType::Physical {
            0.0
        } else {
            read(calc, Field::Resistance(ty)).map_or(0.0, |v| v.value)
        };
        let Some(reduction) = read(calc, Field::Poe1(Poe1Field::BaseDamageReduction(ty))) else {
            continue;
        };
        let pen =
            number(&format!("enemy{}Pen", pob_type(ty))).unwrap_or_else(|| pen_placeholder(ty));
        let overwhelm = if ty == DamageType::Physical {
            number("enemyPhysicalOverwhelm").unwrap_or(0.0)
        } else {
            0.0
        };
        let damage = taken[i];
        let res_mult = 1.0 - (resist - pen) / 100.0;
        let applies = if ty == DamageType::Physical {
            100.0
        } else {
            0.0
        } + sum(
            calc,
            Poe1Quantity::ArmourAppliesTo(ty),
            Op::Base,
            &cx,
            &mut type_reasons,
        )?;
        let applies = applies.min(100.0);
        let effective_armour = armour.value * applies / 100.0;
        let armour_reduct = if applies > 0.0 {
            armour_reduction(effective_armour, damage * res_mult).min(max)
        } else {
            0.0
        };
        let total_reduct = (armour_reduct + reduction.value).min(max);
        let reduct_mult = 1.0 - (max.min(total_reduct - overwhelm)).max(0.0) / 100.0;
        let damage_reduction = 100.0 - reduct_mult * 100.0;
        // The taken multipliers: the type's when hit, with the spell's and
        // the attack's, averaged.
        let mut of = vec![TakenOf::All, TakenOf::Type(ty)];
        if matches!(
            ty,
            DamageType::Fire | DamageType::Cold | DamageType::Lightning
        ) {
            of.push(TakenOf::Elemental);
        }
        let (hit_inc, mut hit_more) = taken_modifiers(
            calc,
            &of,
            &[TakenWhen::Always, TakenWhen::Hit],
            &cx,
            &mut type_reasons,
        )?;
        let (spell_inc, spell_more) = taken_modifiers(
            calc,
            &[TakenOf::All],
            &[TakenWhen::Spell],
            &cx,
            &mut type_reasons,
        )?;
        let (attack_inc, attack_more) = taken_modifiers(
            calc,
            &[TakenOf::All],
            &[TakenWhen::Attack],
            &cx,
            &mut type_reasons,
        )?;
        hit_more *= fortify_more;
        let spell = ((1.0 + (hit_inc + spell_inc) / 100.0) * (hit_more * spell_more)).max(0.0);
        let attack = ((1.0 + (hit_inc + attack_inc) / 100.0) * (hit_more * attack_more)).max(0.0);
        let taken_mult = (spell + attack) / 2.0;
        // The flat damage taken.
        let mut flat = 0.0;
        for (when, share) in [
            (TakenWhen::Always, 1.0),
            (TakenWhen::Hit, 1.0),
            (TakenWhen::Attack, 0.5),
            (TakenWhen::ProjectileAttack, 0.25),
            (TakenWhen::Spell, 0.5),
            (TakenWhen::SpellProjectile, 0.25),
        ] {
            let mut part = 0.0;
            // PoB sums the type's projectile attack damage alone.
            let names: &[TakenOf] = if when == TakenWhen::ProjectileAttack {
                &[TakenOf::Type(ty)]
            } else {
                &[TakenOf::All, TakenOf::Type(ty)]
            };
            for &o in names {
                part += sum(
                    calc,
                    Poe1Quantity::SelfDamageTaken(when, o),
                    Op::Base,
                    &cx,
                    &mut type_reasons,
                )?;
            }
            flat += part * share;
        }
        let base_mult = res_mult * reduct_mult;
        let taken_hit = (damage * base_mult + flat).max(0.0) * taken_mult * suppress_mult;
        let taken_hit_mult = if damage > 0.0 {
            taken_hit / damage
        } else {
            0.0
        };
        let deps = [
            Field::Armour,
            Field::Poe1(Poe1Field::BaseDamageReduction(ty)),
            Field::Poe1(Poe1Field::MeleeEvadeChance),
            Field::Poe1(Poe1Field::ProjectileEvadeChance),
        ];
        for (field, value) in [
            (Field::Poe1(Poe1Field::TakenHitMult(ty)), taken_hit_mult),
            (
                Field::Poe1(Poe1Field::DamageReduction(ty)),
                damage_reduction,
            ),
        ] {
            let node = phases::constant(calc, "enemy-hit", value)?;
            let result = phases::total(calc, &[node], Term::Result)?;
            calc.set_derived(field, phases::query(result, type_reasons.clone()), &deps);
        }
    }
    Ok(())
}

/// PoB's name of a damage type (`dmgTypeList`).
const fn pob_type(ty: DamageType) -> &'static str {
    match ty {
        DamageType::Physical => "Physical",
        DamageType::Fire => "Fire",
        DamageType::Cold => "Cold",
        DamageType::Lightning => "Lightning",
        DamageType::Chaos => "Chaos",
    }
}

/// The names a type's damage taken as reads: the type's, and the
/// elemental types' for fire, cold, and lightning.
fn taken_as_sources(from: DamageType) -> Vec<TakenOf> {
    let mut of = vec![TakenOf::Type(from)];
    if matches!(
        from,
        DamageType::Fire | DamageType::Cold | DamageType::Lightning
    ) {
        of.push(TakenOf::Elemental);
    }
    of
}

/// PoB's `calcs.armourReduction(armour, raw)`: `round(armour / (armour +
/// raw x 5) x 100)`, 0 for both 0.
fn armour_reduction(armour: f64, raw: f64) -> f64 {
    if armour == 0.0 && raw == 0.0 {
        return 0.0;
    }
    crate::math::round_half_up(armour / (armour + raw * 5.0) * 100.0)
}

/// Whether the estimate reads `quantity` (its unsettled values).
const fn is_read(quantity: Quantity) -> bool {
    matches!(
        quantity,
        Quantity::Poe1(
            Poe1Quantity::EnemyCritChance
                | Poe1Quantity::CritExtraDamageReduction
                | Poe1Quantity::EnemyDamage
                | Poe1Quantity::DamageTakenAs(..)
                | Poe1Quantity::ArmourAppliesTo(_)
                | Poe1Quantity::SpellSuppressionEffect
                | Poe1Quantity::SelfDamageTaken(..)
                | Poe1Quantity::EnemyHitReason(_)
                | Poe1Quantity::MaximumFortification
        )
    )
}

/// The `op` sum of `quantity` in `cx`, its reasons into `reasons`.
fn sum(
    calc: &mut Calc<'_>,
    quantity: Poe1Quantity,
    op: Op,
    cx: &crate::condition::QueryContext,
    reasons: &mut Reasons,
) -> Result<f64, CalcError> {
    let (store, graph) = calc.parts();
    let q = store.sum(
        graph,
        Quantity::Poe1(quantity),
        op,
        cx,
        Term::Step("estimate-sum"),
    )?;
    reasons.extend(&q.reasons);
    Ok(q.value)
}

/// The `INC` sum and more product of the damage taken names.
fn taken_modifiers(
    calc: &mut Calc<'_>,
    of: &[TakenOf],
    when: &[TakenWhen],
    cx: &crate::condition::QueryContext,
    reasons: &mut Reasons,
) -> Result<(f64, f64), CalcError> {
    let mut inc = 0.0;
    let mut more = 1.0;
    for w in when {
        for o in of {
            let q = Quantity::Poe1(Poe1Quantity::SelfDamageTaken(*w, *o));
            let (store, graph) = calc.parts();
            let i = store.sum(graph, q, Op::Increased, cx, Term::Step("taken-increased"))?;
            let m = store.more_rounded(graph, q, cx)?;
            reasons.extend(&i.reasons);
            reasons.extend(&m.reasons);
            inc += i.value;
            more *= m.value;
        }
    }
    Ok((inc, more))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// PoB's `calcs.armourReduction`: 1,000 armour against a 200 hit is
    /// 1000 / (1000 + 1000) = 50%.
    #[test]
    fn the_armour_reduction_should_be_pobs() {
        assert_eq!(armour_reduction(1000.0, 200.0), 50.0);
        assert_eq!(armour_reduction(0.0, 0.0), 0.0);
        assert_eq!(armour_reduction(0.0, 100.0), 0.0);
    }
}
