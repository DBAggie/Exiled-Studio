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
use super::{BlockGain, Poe1Field, Poe1Quantity, TakenOf, TakenWhen};
use crate::condition::{Flags, QueryContext};
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

/// Summon Stone Golem of Safeguarding (P6-11ar): its granted effect, the
/// config input that overrides its life (`ConfigOptions.lua` 809), and the
/// share of its redirect under the `Average` damage category, the only one
/// the engine computes (`CalcDefence.lua` 2463: melee only, so a quarter).
const STONE_GOLEM: &str = "SummonStoneGolemAltY";
/// The config input that overrides the golem's life.
pub const STONE_GOLEM_LIFE_INPUT: &str = "TotalStoneGolemLife";
const STONE_GOLEM_AVERAGE_SHARE: f64 = 0.25;

/// The life of Summon Stone Golem of Safeguarding, the pool its redirect
/// drains (P6-11ar; `CalcPerform.lua` 3597 to 3677): the config override
/// when set, else the golem's `Life` computed with the build's first
/// active Summon Stone Golem of Safeguarding as the main skill (PoB's
/// `initMinionModDB`, the skill's and the buffs' minion modifiers, then
/// `doActorAttribsConditions`, which the engine's minion actor follows),
/// its reasons carried. `None`, with a reason, inside a nested calculation
/// (which computes no other skill) or when the golem's life is not
/// computed; `Some(0)` without such a skill.
fn stone_golem_life(
    calc: &Calc<'_>,
    set: Option<&loom_domain::ConfigSet>,
    reasons: &mut Reasons,
) -> Option<f64> {
    let input = *calc.input();
    if let Some(ConfigValue::Number(n)) = set
        .and_then(|s| super::config::last_input(s, STONE_GOLEM_LIFE_INPUT, ConfigInputKind::Input))
    {
        return Some(n.value().0);
    }
    let levels = super::gem_levels::GemLevels::from_effects(calc.store().effects());
    let Some((g, a, skill)) = super::skill::active_skills(input.build, input.data, &levels)
        .into_iter()
        .find(|(_, _, s)| s.active_gem.effect.id == STONE_GOLEM)
    else {
        return Some(0.0);
    };
    let unmodelled = UnsupportedReason::UnmodelledSkill {
        source: crate::source::Source::Gem {
            group: g - 1,
            gem: skill.active_gem.gem,
        },
        what: "ally-life",
    };
    if input.options.nested {
        reasons.partial.insert(unmodelled);
        return None;
    }
    let b = super::skill::with_main_skill(input.build, g, a);
    let mut options = input.options.clone();
    options.nested = true;
    let sub = crate::input::CalcInput {
        build: &b,
        data: input.data,
        options: &options,
    };
    let life = crate::calculate(&sub).ok().and_then(|r| {
        let f = r
            .fields
            .get(&Field::Minion(crate::result::MinionField::Life))?;
        super::triggers::carry(&f.state, reasons)
            .then_some(f.value)
            .flatten()
    });
    if life.is_none() {
        reasons.partial.insert(unmodelled);
    }
    life
}

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
    // The enemy's `NeverCrit` (Hexpass, P6-11fl): 0 (`CalcDefence.lua`
    // 1694).
    let never = {
        let (store, graph) = calc.parts();
        store
            .flag(graph, Quantity::Poe1(Poe1Quantity::EnemyNeverCrit), &cx)?
            .value
            > 0.0
    };
    let crit_chance = if never {
        0.0
    } else {
        (crit * (1.0 + crit_inc / 100.0) * (1.0 - evade / 100.0)).clamp(0.0, 100.0)
    };
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
    let mut damage_in = 0.0;
    for (i, ty) in TYPES.into_iter().enumerate() {
        let name = format!("enemy{}Damage", pob_type(ty));
        let damage = number(&name).unwrap_or_else(|| placeholder(ty));
        damage_in += damage;
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
    let fortify_more = match fortification_stacks(calc, &cx, &mut reasons)? {
        Some(stacks) => 1.0 - stacks / 100.0,
        None => 1.0,
    };
    // Spell suppression, its effective chance (P6-11al).
    let suppression =
        read(calc, Field::Poe1(Poe1Field::SpellSuppressionChance)).map_or(0.0, |v| v.value);
    let suppression = effective_suppression(calc, suppression, &cx, &mut reasons)?;
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
    let mut parts: [Option<TypeHit>; 5] = [None, None, None, None, None];
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
        let (hit_inc, hit_more) = taken_modifiers(
            calc,
            &of,
            &[TakenWhen::Always, TakenWhen::Hit],
            fortify_more,
            &cx,
            &mut type_reasons,
        )?;
        let (spell_inc, spell_more) = taken_modifiers(
            calc,
            &[TakenOf::All],
            &[TakenWhen::Spell],
            1.0,
            &cx,
            &mut type_reasons,
        )?;
        let (attack_inc, attack_more) = taken_modifiers(
            calc,
            &[TakenOf::All],
            &[TakenWhen::Attack],
            1.0,
            &cx,
            &mut type_reasons,
        )?;
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
        parts[i] = Some(TypeHit {
            res_mult,
            flat_dr: reduction.value / 100.0,
            overwhelm,
            flat,
            after: taken_mult * suppress_mult,
            effective_armour,
            damage_reduction,
            taken_hit,
            reasons: type_reasons,
        });
    }
    maximum_hit(calc, set, &parts, &shares, max, damage_mult, damage_in)
}

/// One type's values of the hit that maximum hit reads (PoB's
/// `<Type>ResistTakenHitMulti`, the base reduction, overwhelm,
/// `<Type>takenFlat`, `<Type>AfterReductionTakenHitMulti`,
/// `<Type>EffectiveAppliedArmour`, `<Type>DamageReduction`).
struct TypeHit {
    res_mult: f64,
    flat_dr: f64,
    overwhelm: f64,
    flat: f64,
    after: f64,
    effective_armour: f64,
    damage_reduction: f64,
    taken_hit: f64,
    reasons: Reasons,
}

/// The config checks that lower the pools (P6-11ad; `CalcDefence.lua`
/// 1061 and 2215 to 2220): low life caps `LifeRecoverable` at `Life x
/// LowLifePercentage` (the threshold, else `LowPoolThreshold`), low
/// energy shield caps `EnergyShieldRecoveryCap` at `EnergyShield x
/// LowPoolThreshold`. A value that is not a boolean is a reason.
const POOL_INPUTS: [&str; 2] = [LOW_LIFE_INPUT, LOW_ES_INPUT];
/// The config check of low life.
const LOW_LIFE_INPUT: &str = "conditionLowLife";
/// The config check of low energy shield.
const LOW_ES_INPUT: &str = "conditionLowEnergyShield";
/// PoB's `data.misc.LowPoolThreshold` (`Data.lua` 233).
const LOW_POOL_THRESHOLD: f64 = 0.5;

/// The maximum hit of each type (P6-10b part 13): PoB's
/// `<Type>MaximumHitTaken` and `SecondMinimalMaximumHitTaken`
/// (`CalcDefence.lua` 2214 to 2556, 3120 to 3325).
///
/// What PoB does, without the pools the engine does not model (each such
/// stat an unmapped one or a marker, so a reason):
/// - `LifeRecoverable = max(LifeUnreserved, 1)`; its hit pool, with no
///   life loss prevented, `max(life - half, 0) + min(life, half)`, `half`
///   half of maximum life;
/// - each type's total hit pool: the life pool, plus the energy shield
///   (`EnergyShieldRecoveryCap`, the maximum) unless the type bypasses it
///   (chaos does, 100%), plus ward, aegis, and guard (none);
/// - for each type the hit converts to (the type itself without taken as):
///   with no armour applied, `max(pool / share / (resMult x (1 -
///   DamageReduction / 100)) - flat, 0) / takenMult`; with armour, PoB's
///   quadratic in the raw hit, `floor(max(min(RAW, maxDR), noDR))`;
/// - `<Type>MaximumHitTaken = round(min / enemy damage multiplier)`; a hit
///   that converts (PoB's smoothing passes) is not computed;
/// - `SecondMinimalMaximumHitTaken` the second smallest of the five.
fn maximum_hit(
    calc: &mut Calc<'_>,
    set: Option<&loom_domain::ConfigSet>,
    parts: &[Option<TypeHit>; 5],
    shares: &[[f64; 5]; 5],
    max: f64,
    enemy_damage_mult: f64,
    damage_in: f64,
) -> Result<(), CalcError> {
    let fields = TYPES.map(|t| Field::Poe1(Poe1Field::MaximumHitTaken(t)));
    let second = Field::Poe1(Poe1Field::SecondMinimalMaximumHitTaken);
    for name in POOL_INPUTS {
        if super::conditions::check(set, name).is_none() {
            for f in fields.into_iter().chain([second]) {
                calc.set_unsupported(
                    f,
                    UnsupportedReason::UnmodelledConfig {
                        input: name.to_owned(),
                    },
                );
            }
            return Ok(());
        }
    }
    let mut pool_reasons = phases::marker_reasons(
        calc,
        |q| match q {
            Quantity::Poe1(Poe1Quantity::PoolReason(stat)) => Some(stat),
            _ => None,
        },
        |q| matches!(q, Quantity::Poe1(Poe1Quantity::ManaBeforeLife(_))),
    );
    let value = |calc: &Calc<'_>, f: Field| calc.field(f).and_then(|r| r.value);
    let (Some(life), Some(unreserved), Some(es)) = (
        value(calc, Field::Life),
        value(calc, Field::LifeUnreserved),
        value(calc, Field::EnergyShield),
    ) else {
        return Ok(());
    };
    let cx = calc.context(Flags::NONE);
    let recoverable = if super::conditions::check(set, LOW_LIFE_INPUT) == Some(true) {
        let threshold = sum(
            calc,
            Poe1Quantity::LowLifePercentage,
            Op::Base,
            &cx,
            &mut pool_reasons,
        )?;
        let threshold = if threshold > 0.0 {
            threshold
        } else {
            LOW_POOL_THRESHOLD
        };
        (life * threshold).min(unreserved)
    } else {
        unreserved
    }
    .max(1.0);
    // `EnergyShieldRecoveryCap` (`CalcDefence.lua` 1055 to 1063): capped
    // at armour with Divine Shield's flag (P6-11am) when armour is lower,
    // then at half with the low energy shield check.
    let armour_cap = {
        let (store, graph) = calc.parts();
        let q = store.flag(
            graph,
            Quantity::Poe1(Poe1Quantity::ArmourEsRecoveryCap),
            &cx,
        )?;
        pool_reasons.extend(&q.reasons);
        q.value != 0.0
    };
    // Eldritch Battery (P6-11ap): energy shield protects mana instead of
    // life. Checked with `has` first, so a build without it gains no node.
    let eb = phases::has(calc, Quantity::Poe1(Poe1Quantity::EsProtectsMana)) && {
        let (store, graph) = calc.parts();
        let q = store.flag(graph, Quantity::Poe1(Poe1Quantity::EsProtectsMana), &cx)?;
        pool_reasons.extend(&q.reasons);
        q.value != 0.0
    };
    let armour_value = value(calc, Field::Armour).unwrap_or(0.0);
    let low_es = super::conditions::check(set, LOW_ES_INPUT) == Some(true);
    let capping = (armour_cap && armour_value < es) || low_es;
    let es = if capping {
        let cap = if armour_cap { armour_value } else { es };
        if low_es {
            (es * LOW_POOL_THRESHOLD).min(cap)
        } else {
            cap
        }
    } else {
        es
    };
    // Any recoup (PoB's `anyRecoup`, `CalcDefence.lua` 1385 to 1465):
    // Divine Shield's pseudo recoup, and mana (P6-11as) and life (P6-10a)
    // recoup times their recovery rates (read only when present).
    let pseudo = sum(
        calc,
        Poe1Quantity::PseudoRecoup,
        Op::Base,
        &cx,
        &mut pool_reasons,
    )?;
    let mana_recoup = if phases::has(calc, Quantity::Poe1(Poe1Quantity::ManaRecoup)) {
        let base = sum(
            calc,
            Poe1Quantity::ManaRecoup,
            Op::Base,
            &cx,
            &mut pool_reasons,
        )?;
        base * super::regen::recovery_rate(calc, super::Pool::Mana, &mut pool_reasons)?.value
    } else {
        0.0
    };
    let life_recoup = if phases::has(calc, Quantity::Poe1(Poe1Quantity::LifeRecoup)) {
        let base = sum(
            calc,
            Poe1Quantity::LifeRecoup,
            Op::Base,
            &cx,
            &mut pool_reasons,
        )?;
        base * super::regen::recovery_rate(calc, super::Pool::Life, &mut pool_reasons)?.value
    } else {
        0.0
    };
    let recoup = pseudo + mana_recoup + life_recoup > 0.0;
    // The shared guard (P6-11ae; `CalcDefence.lua` 2379 to 2395): its
    // rate, and its limit, `GuardAbsorbLimit` BASE with Molten Shell's
    // share of armour (`PerStat` `Armour`, at most `GuardAbsorbMax` in
    // total). A type's own guard has no modelled source.
    let guard_rate = sum(
        calc,
        Poe1Quantity::GuardAbsorbRate,
        Op::Base,
        &cx,
        &mut pool_reasons,
    )?
    .min(100.0);
    let guard = if guard_rate > 0.0 {
        let flat = sum(
            calc,
            Poe1Quantity::GuardAbsorbLimit,
            Op::Base,
            &cx,
            &mut pool_reasons,
        )?;
        let per = sum(
            calc,
            Poe1Quantity::GuardAbsorbPerArmour,
            Op::Base,
            &cx,
            &mut pool_reasons,
        )?;
        let of_armour = if per == 0.0 {
            0.0
        } else {
            let cap = sum(
                calc,
                Poe1Quantity::GuardAbsorbMax,
                Op::Base,
                &cx,
                &mut pool_reasons,
            )?;
            let armour = value(calc, Field::Armour).unwrap_or(0.0);
            (per * crate::math::floor(armour + 0.0001)).min(cap)
        };
        flat + of_armour
    } else {
        0.0
    };
    // Summon Stone Golem of Safeguarding's life before yours (P6-11ar;
    // `CalcDefence.lua` 2459 to 2473): a quarter of the redirect under the
    // `Average` category, its life the pool (`buildAllyLifePools`: both
    // above 0, the share at most 100%).
    let golem_rate = sum(
        calc,
        Poe1Quantity::TakenFromStoneGolem,
        Op::Base,
        &cx,
        &mut pool_reasons,
    )? * STONE_GOLEM_AVERAGE_SHARE;
    let (golem_rate, golem) = if golem_rate > 0.0 {
        match stone_golem_life(calc, set, &mut pool_reasons) {
            Some(l) if l > 0.0 => (golem_rate.min(100.0), l),
            _ => (0.0, 0.0),
        }
    } else {
        (0.0, 0.0)
    };
    // Life loss prevented (P6-11ce): PoB's `LifeHitPool` with
    // `calcLifeHitPoolWithLossPrevention` (`CalcDefence.lua` 2233 to 2245).
    let prevented = sum(
        calc,
        Poe1Quantity::LifeLossPrevented,
        Op::Base,
        &cx,
        &mut pool_reasons,
    )?
    .min(100.0);
    let life_pool = super::ehp::life_hit_pool(recoverable, life, prevented);
    let mana = value(calc, Field::ManaUnreserved).unwrap_or(0.0).max(0.0);
    // Mind over matter (P6-10b part 15; `CalcDefence.lua` 2303 to 2377):
    // the shared share, and each type's own on top of it.
    let shared = sum(
        calc,
        Poe1Quantity::ManaBeforeLife(TakenOf::All),
        Op::Base,
        &cx,
        &mut pool_reasons,
    )?
    .min(100.0);
    let mut mom = [0.0; 5];
    for (i, ty) in TYPES.into_iter().enumerate() {
        let own = sum(
            calc,
            Poe1Quantity::ManaBeforeLife(TakenOf::Type(ty)),
            Op::Base,
            &cx,
            &mut pool_reasons,
        )?
        .min(100.0 - shared);
        mom[i] = (shared + own) / 100.0;
    }
    let mut pools = [0.0; 5];
    for (i, ty) in TYPES.into_iter().enumerate() {
        // Chaos bypasses energy shield (`CalcDefence.lua` 2282 to 2291).
        let bypass = if ty == DamageType::Chaos { 100.0 } else { 0.0 };
        let with_mana = mom_hit_pool(
            life_pool,
            mom[i],
            mana,
            (eb && bypass < 100.0).then_some(es),
        );
        // With Eldritch Battery the shield is in the mind over matter pool
        // alone (`CalcDefence.lua` 2536 to 2552).
        pools[i] = if bypass < 100.0 && !eb {
            with_mana + es
        } else {
            with_mana
        };
        // The guard (`CalcDefence.lua` 3136 to 3146).
        if guard_rate >= 100.0 {
            pools[i] += guard;
        } else if guard_rate > 0.0 {
            let share = guard_rate / 100.0;
            let protected = guard / share * (1.0 - share);
            pools[i] = (pools[i] - protected).max(0.0) + pools[i].min(protected) / (1.0 - share);
        }
        // The golem's pool, undone after the guard (3147 to 3160).
        if golem > 0.0 {
            if golem_rate >= 100.0 {
                pools[i] += golem;
            } else {
                let share = golem_rate / 100.0;
                let protected = golem / share * (1.0 - share);
                pools[i] =
                    (pools[i] - protected).max(0.0) + pools[i].min(protected) / (1.0 - share);
            }
        }
    }
    let mut results = [None; 5];
    for (i, ty) in TYPES.into_iter().enumerate() {
        let Some(part) = &parts[i] else { continue };
        let mut reasons = part.reasons.clone();
        reasons.extend(&pool_reasons);
        let mut part_min = f64::INFINITY;
        let mut smoothing = false;
        for (c, converted) in parts.iter().enumerate() {
            let Some(conv) = converted else { continue };
            let percent = shares[i][c];
            if !(percent > 0.0 || conv.flat != 0.0) {
                continue;
            }
            reasons.extend(&conv.reasons);
            let multi = percent / 100.0;
            let pool = pools[c];
            let taken_multi = conv.after;
            let hit = if multi <= 0.0 {
                if conv.flat.max(0.0) * taken_multi >= pool {
                    0.0
                } else {
                    f64::INFINITY
                }
            } else if conv.effective_armour == 0.0 && percent == 100.0 {
                let dr_multi = conv.res_mult * (1.0 - conv.damage_reduction / 100.0);
                (pool / multi / dr_multi - conv.flat).max(0.0) / taken_multi
            } else {
                let rx = conv.res_mult * multi;
                let ow = conv.overwhelm / 100.0;
                let armour = conv.effective_armour;
                let a = 5.0 * (1.0 - conv.flat_dr + ow) * taken_multi * rx * rx;
                let b = ((ow - conv.flat_dr) * armour * taken_multi
                    - 5.0 * (pool - conv.flat * taken_multi))
                    * rx;
                let cc = -armour * (pool - conv.flat * taken_multi);
                let raw = if a == 0.0 {
                    f64::INFINITY
                } else {
                    (crate::math::sqrt((b * b - 4.0 * a * cc).max(0.0)) - b) / (2.0 * a)
                };
                let no_dr = pool / multi / conv.res_mult / taken_multi
                    * (1.0 - conv.flat * taken_multi / pool);
                let max_dr = no_dr / (1.0 - (max - conv.overwhelm) / 100.0);
                smoothing |= percent != 100.0;
                crate::math::floor(raw.min(max_dr).max(no_dr))
            };
            part_min = part_min.min(hit);
        }
        if smoothing {
            calc.set_unsupported(fields[i], UnsupportedReason::NotComputed);
            continue;
        }
        if !part_min.is_finite() {
            continue;
        }
        let max_hit = crate::math::round_half_up(part_min / enemy_damage_mult);
        results[i] = Some(max_hit);
        let node = phases::constant(calc, "maximum-hit", max_hit)?;
        let result = phases::total(calc, &[node], Term::Result)?;
        calc.set_derived(
            fields[i],
            phases::query(result, reasons),
            &[
                Field::Life,
                Field::LifeUnreserved,
                Field::EnergyShield,
                Field::Poe1(Poe1Field::TakenHitMult(ty)),
            ],
        );
    }
    // The second smallest, in PoB's order.
    if results.iter().all(Option::is_some) {
        let mut minimum = f64::INFINITY;
        let mut second_min = f64::INFINITY;
        for v in results.into_iter().flatten() {
            if v < minimum {
                second_min = minimum;
                minimum = v;
            } else if v < second_min {
                second_min = v;
            }
        }
        let mut reasons = Reasons::new();
        for f in fields {
            if let Some(r) = calc.field(f)
                && let crate::state::FieldState::Partial(p) = &r.state
            {
                reasons.partial.extend(p.iter().cloned());
            }
        }
        let node = phases::constant(calc, "second-minimal-maximum-hit", second_min)?;
        let result = phases::total(calc, &[node], Term::Result)?;
        calc.set_derived(second, phases::query(result, reasons), &fields);
    }
    // The effective health pool (P6-10b part 14).
    if parts.iter().all(Option::is_some) {
        let mut reasons = pool_reasons.clone();
        reasons.extend(&phases::marker_reasons(
            calc,
            |q| match q {
                Quantity::Poe1(Poe1Quantity::EhpReason(stat)) => Some(stat),
                _ => None,
            },
            |q| q == Quantity::Poe1(Poe1Quantity::AvoidProjectiles),
        ));
        let mut taken_hit = [0.0; 5];
        for (i, p) in parts.iter().enumerate() {
            if let Some(p) = p {
                reasons.extend(&p.reasons);
                taken_hit[i] = p.taken_hit;
            }
        }
        let field = |calc: &Calc<'_>, f: Field| calc.field(f).and_then(|r| r.value).unwrap_or(0.0);
        let cx = calc.context(Flags::NONE);
        let suppression_effect = sum(
            calc,
            Poe1Quantity::SpellSuppressionEffect,
            Op::Base,
            &cx,
            &mut reasons,
        )?;
        let avoid_projectiles = sum(
            calc,
            Poe1Quantity::AvoidProjectiles,
            Op::Base,
            &cx,
            &mut reasons,
        )?
        .min(AVOID_CHANCE_CAP);
        // `BlockEffect = 100 - BlockEffect BASE` (`CalcDefence.lua` 768).
        let block_effect =
            100.0 - sum(calc, Poe1Quantity::BlockEffect, Op::Base, &cx, &mut reasons)?;
        // The gains on block (P6-11ak; `CalcDefence.lua` 1525 to 1532): a
        // flat sum, and each percent mod `ceil(stat x percent / 100)`
        // (PoB's `PercentStat`).
        let armour = value(calc, Field::Armour).unwrap_or(0.0);
        let gain = |calc: &mut Calc<'_>, g: BlockGain, of: f64, reasons: &mut Reasons| {
            let q = Quantity::Poe1(Poe1Quantity::BlockGain(g));
            let mut total = 0.0;
            for e in calc.store().effects() {
                if e.quantity != q || e.op != Op::Base {
                    continue;
                }
                match e.conditions.applies(&cx) {
                    crate::condition::Applies::Yes => {
                        total += if of > 0.0 {
                            crate::math::ceil(e.value * of / 100.0)
                        } else {
                            e.value
                        };
                    }
                    crate::condition::Applies::No => {}
                    crate::condition::Applies::Unknown(keys) => {
                        for condition in keys {
                            reasons
                                .partial
                                .insert(UnsupportedReason::UnmodelledCondition {
                                    condition,
                                    quantity: q,
                                    source: e.source.clone(),
                                });
                        }
                    }
                }
            }
            total
        };
        let life_on_block = gain(calc, BlockGain::Life, 0.0, &mut reasons)
            + gain(calc, BlockGain::LifePercent, life, &mut reasons);
        let energy_shield_on_block = gain(
            calc,
            BlockGain::EnergyShieldPercentOfArmour,
            armour,
            &mut reasons,
        );
        let energy_shield = value(calc, Field::EnergyShield).unwrap_or(0.0);
        let energy_shield_on_spell_block =
            gain(calc, BlockGain::EnergyShieldSpell, 0.0, &mut reasons)
                + gain(
                    calc,
                    BlockGain::EnergyShieldSpellPercent,
                    energy_shield,
                    &mut reasons,
                );
        let spell_dodge = spell_dodge(calc, &cx, &mut reasons)?;
        let mitigation = super::ehp::Mitigation {
            life_on_block,
            energy_shield_on_block,
            energy_shield_on_spell_block,
            // Lucky block (P6-11gt): each chance rolled twice.
            block: {
                let chance = field(calc, Field::BlockChance);
                lucky(calc, chance, Poe1Quantity::BlockLucky, &cx, &mut reasons)?
            },
            spell_block: {
                let chance = field(calc, Field::Poe1(Poe1Field::SpellBlockChance));
                lucky(calc, chance, Poe1Quantity::BlockLucky, &cx, &mut reasons)?
            },
            block_effect,
            suppression: {
                let chance = field(calc, Field::Poe1(Poe1Field::SpellSuppressionChance));
                effective_suppression(calc, chance, &cx, &mut reasons)?
            },
            suppression_effect,
            melee_evade: field(calc, Field::Poe1(Poe1Field::MeleeEvadeChance)),
            projectile_evade: field(calc, Field::Poe1(Poe1Field::ProjectileEvadeChance)),
            avoid_projectiles,
            attack_dodge: 0.0,
            spell_dodge,
            avoid_all: 0.0,
        };
        let character = super::ehp::Character {
            life: recoverable,
            max_life: life,
            energy_shield: es,
            mana,
            mom,
            guard_rate,
            guard,
            recoup,
            eb,
            golem_rate,
            golem,
            prevented,
        };
        super::ehp::estimate(
            calc,
            &taken_hit,
            damage_in,
            &character,
            &mitigation,
            &reasons,
        )?;
    }
    Ok(())
}

/// The fortification stacks (`CalcPerform.lua` 632 to 653), `None` when
/// not fortified: while the config's `buffFortification` is on, `min(the
/// config's overrideFortification or the maximum, the maximum)`, the
/// maximum `base_max_fortification` (20 without it) plus
/// `MaximumFortification` BASE. Read by the enemy's hit and, per stack,
/// by regeneration (P6-11bu).
///
/// # Errors
/// A graph error.
pub(super) fn fortification_stacks(
    calc: &mut Calc<'_>,
    cx: &QueryContext,
    reasons: &mut Reasons,
) -> Result<Option<f64>, CalcError> {
    let set = super::collect::active_config(calc.input().build);
    if super::conditions::check(set, "buffFortification") != Some(true) {
        return Ok(None);
    }
    #[expect(clippy::cast_precision_loss, reason = "a small integer")]
    let base = calc
        .input()
        .data
        .character_constant("base_max_fortification")
        .map_or(20.0, |v| v as f64);
    let max = base
        + sum(
            calc,
            Poe1Quantity::MaximumFortification,
            Op::Base,
            cx,
            reasons,
        )?;
    let over = match set
        .and_then(|s| super::config::last_input(s, "overrideFortification", ConfigInputKind::Input))
    {
        Some(ConfigValue::Number(n)) => Some(n.value().0),
        _ => None,
    };
    Ok(Some(over.unwrap_or(max).min(max)))
}

/// PoB's `calcMoMEBPool` (`CalcDefence.lua` 2309 to 2321): the life pool
/// with the mana mind over matter can take (`floor(min(mana, life / (1 -
/// share) - life))`), and with Eldritch Battery (`es`, the shield of a type
/// that does not bypass it; P6-11ap) the shield that protects that mana
/// first, `floor(min(es, max, (life + mana) / (1 - share) - (life +
/// mana)))`, the mana used then what the shield leaves of the maximum.
fn mom_hit_pool(life_pool: f64, share: f64, mana: f64, es: Option<f64>) -> f64 {
    let max_pool = if share < 1.0 {
        life_pool / (1.0 - share) - life_pool
    } else {
        f64::INFINITY
    };
    let usable = crate::math::floor(mana.min(max_pool));
    let shield = es.map_or(0.0, |es| {
        let with_mana = life_pool + usable;
        crate::math::floor(es.min(max_pool).min(with_mana / (1.0 - share) - with_mana))
    });
    let used = crate::math::floor((max_pool - shield).min(usable));
    life_pool + used + shield
}

/// PoB's `data.misc.AvoidChanceCap`.
const AVOID_CHANCE_CAP: f64 = 75.0;

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
                | Poe1Quantity::EnemyNeverCrit
                | Poe1Quantity::CritExtraDamageReduction
                | Poe1Quantity::EnemyDamage
                | Poe1Quantity::DamageTakenAs(..)
                | Poe1Quantity::ArmourAppliesTo(_)
                | Poe1Quantity::SpellSuppressionEffect
                | Poe1Quantity::SelfDamageTaken(..)
                | Poe1Quantity::EnemyHitReason(_)
                | Poe1Quantity::MaximumFortification
                | Poe1Quantity::LowLifePercentage
                | Poe1Quantity::GuardAbsorbRate
                | Poe1Quantity::GuardAbsorbLimit
                | Poe1Quantity::GuardAbsorbPerArmour
                | Poe1Quantity::GuardAbsorbMax
                | Poe1Quantity::BlockGain(_)
                | Poe1Quantity::SuppressionLucky
                | Poe1Quantity::BlockLucky
                | Poe1Quantity::ArmourEsRecoveryCap
                | Poe1Quantity::EsProtectsMana
                | Poe1Quantity::SuppressionToSpellDodge
                | Poe1Quantity::TakenFromStoneGolem
                | Poe1Quantity::PseudoRecoup
                | Poe1Quantity::ManaRecoup
                | Poe1Quantity::LifeRecoup
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

/// PoB's `EffectiveSpellSuppressionChance` (`CalcDefence.lua` 1131 to
/// 1157; P6-11al): lucky, `(1 - (1 - chance)^2) x 100`. Unlucky, extreme
/// luck, and Unexciting are unmapped stats, so reasons.
/// Acrobatics' maximum chance to dodge spell hits (PoB's
/// `SpellDodgeChanceMax` OVERRIDE 75 from the keystone, `ModCache.lua`
/// 9946).
const ACROBATICS_SPELL_DODGE_MAX: f64 = 75.0;

/// PoB's `EffectiveSpellDodgeChance` (P6-11el; `CalcDefence.lua` 1124,
/// 1164 to 1172): with Acrobatics, half the spell suppression chance's
/// `BASE` sum (before the keystone's override), at most 75%; 0 without
/// it. The only other source of spell dodge in the game is gone, and
/// PoB's enemy `CannotBeDodged` comes from a boss skill preset, already a
/// reason of the estimate ([`UNMODELLED`]).
fn spell_dodge(
    calc: &mut Calc<'_>,
    cx: &crate::condition::QueryContext,
    reasons: &mut Reasons,
) -> Result<f64, CalcError> {
    let q = Quantity::Poe1(Poe1Quantity::SuppressionToSpellDodge);
    if !phases::has(calc, q) {
        return Ok(0.0);
    }
    let (store, graph) = calc.parts();
    let flag = store.flag(graph, q, cx)?;
    reasons.extend(&flag.reasons);
    if flag.value == 0.0 {
        return Ok(0.0);
    }
    let suppression = store.sum(
        graph,
        Quantity::Poe1(Poe1Quantity::SpellSuppressionChance),
        Op::Base,
        cx,
        Term::Step("suppression-base"),
    )?;
    reasons.extend(&suppression.reasons);
    Ok((suppression.value / 2.0).clamp(0.0, ACROBATICS_SPELL_DODGE_MAX))
}

fn effective_suppression(
    calc: &mut Calc<'_>,
    chance: f64,
    cx: &crate::condition::QueryContext,
    reasons: &mut Reasons,
) -> Result<f64, CalcError> {
    lucky(calc, chance, Poe1Quantity::SuppressionLucky, cx, reasons)
}

/// `chance` (a percent) rolled twice when the `flag` is on (PoB's
/// `IsLucky`: `1 - (1 - p)^2`), else as it is.
fn lucky(
    calc: &mut Calc<'_>,
    chance: f64,
    flag: Poe1Quantity,
    cx: &crate::condition::QueryContext,
    reasons: &mut Reasons,
) -> Result<f64, CalcError> {
    let (store, graph) = calc.parts();
    let lucky = store.flag(graph, Quantity::Poe1(flag), cx)?;
    reasons.extend(&lucky.reasons);
    if lucky.value == 0.0 {
        return Ok(chance);
    }
    let p = chance / 100.0;
    Ok((1.0 - (1.0 - p).powi(2)) * 100.0)
}

/// The `INC` sum and more product of the damage taken names. Each name's
/// more product is rounded to two decimals (PoB's `MoreInternal`);
/// `hit_all` multiplies `DamageTakenWhenHit`'s before its rounding:
/// fortification's MORE is a mod of that name (`CalcPerform.lua` 648),
/// rounded with the others (P6-11x: 20 stacks and Sand Stance's -14 are
/// `round(0.8 x 0.86, 2) = 0.69`).
fn taken_modifiers(
    calc: &mut Calc<'_>,
    of: &[TakenOf],
    when: &[TakenWhen],
    hit_all: f64,
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
            reasons.extend(&i.reasons);
            inc += i.value;
            if (*w, *o) == (TakenWhen::Hit, TakenOf::All) && hit_all != 1.0 {
                let m = store.more(graph, q, cx)?;
                reasons.extend(&m.reasons);
                more *= crate::math::round_half_up(m.value * hit_all * 100.0) / 100.0;
            } else {
                let m = store.more_rounded(graph, q, cx)?;
                reasons.extend(&m.reasons);
                more *= m.value;
            }
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
