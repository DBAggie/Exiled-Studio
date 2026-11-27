//! The effective health pool (Phase 6 P6-10b, part 14): PoB v2.67.2's
//! `NumberOfDamagingHits`, `NumberOfMitigatedDamagingHits`,
//! `TotalNumberOfHits`, `ConfiguredDamageChance`, `ConfiguredNotHitChance`,
//! and `TotalEHP` (`Modules/CalcDefence.lua` 188 to 427, 1645 to 1667,
//! 2568 to 2913), for the config's damage category (`Average` and the
//! four hit categories since open items S7, `enemy_hit::Category`).
//!
//! What PoB does:
//! - **Hits to die** (`numberOfHitsToDie`): with the pools full (life
//!   `LifeRecoverable`, energy shield `EnergyShieldRecoveryCap`), each
//!   enemy hit of `<Type>TakenHit` (each part rounded up) takes energy
//!   shield first unless the type bypasses it (chaos), then life
//!   (`calcs.reducePoolsByDamage`), until life is 0 or 50 iterations;
//!   after the first hit it runs itself on 8 times the damage (4 with a
//!   limited speed-up) and counts `max((hits - 1) x 8 - 1, 1)` hits at
//!   once; the last hit's overkill over its damage is taken off; 10^8
//!   damage survived is infinite.
//! - **Mitigated hits:** each type's hit times `blockEffect x
//!   suppressionEffect x (1 - avoid / 100)`: `blockEffect = 1 - block x
//!   BlockEffect / 100`, `block` the average of the four effective block
//!   chances (attack, projectile, spell, spell projectile) over 100;
//!   `suppressionEffect = 1 - chance / 2 x SpellSuppressionEffect / 100`
//!   below a 100% chance (at 100% the hit's reduction has it); `avoid` a
//!   type's avoidance with half the projectile avoidance, only with a
//!   type's avoidance; `ConfiguredDamageChance = 100 x` the product with
//!   the average avoidance over five; the hits again unless it is 100.
//! - **Not hit:** the four categories' chance not to be hit (evade, dodge,
//!   avoiding all damage from hits, avoiding projectiles without a type's
//!   avoidance), averaged: `ConfiguredNotHitChance`; `TotalNumberOfHits =
//!   mitigated / (1 - not hit / 100)`; `TotalEHP = TotalNumberOfHits x` the
//!   enemy's damage before its modifiers (`totalEnemyDamageIn`).
//!
//! - **Guard** (P6-11ae): the shared guard takes `GuardAbsorbRate`% of
//!   each type's damage, at most what is left of it, before energy shield
//!   (floored after each take), and what is left carries to the next hit.
//!
//! Not modelled, each a reason (unmapped stats, markers, or config): a
//! type's own guard, ward, a type's own or the shared aegis (the shared
//! elemental one is P6-11hl's), allies taking damage, life loss
//! prevented, recoup, life, mana, or energy shield gained on block or
//! suppression, dodge, avoiding all damage from hits or a type's damage,
//! lucky or unlucky block and suppression, the enemy reducing block.

use super::Poe1Field;
use super::phases;
use crate::engine::Calc;
use crate::error::CalcError;
use crate::graph::Term;
use crate::quantity::DamageType;
use crate::result::Field;
use crate::state::Reasons;

/// PoB's `data.misc.ehpCalcSpeedUp`.
const SPEED_UP: f64 = 8.0;
/// PoB's limited speed-up (`LimitEHPSpeedup`, P6-11ak).
const LIMITED_SPEED_UP: f64 = 4.0;
/// PoB's `data.misc.ehpCalcMaxDamage`.
const MAX_DAMAGE: f64 = 100_000_000.0;
/// PoB's `data.misc.ehpCalcMaxIterationsToCalc`.
const MAX_ITERATIONS: u32 = 50;
/// PoB's `data.misc.SuppressionEffect`.
const SUPPRESSION_EFFECT: f64 = 40.0;

/// The damage types in PoB's order (`dmgTypeList`).
const TYPES: [DamageType; 5] = [
    DamageType::Physical,
    DamageType::Lightning,
    DamageType::Cold,
    DamageType::Fire,
    DamageType::Chaos,
];

/// The pools a hit drains.
#[derive(Debug, Clone, Copy)]
struct Pools {
    life: f64,
    energy_shield: f64,
    mana: f64,
    /// The shared guard left (P6-11ae).
    guard: f64,
    /// Summon Stone Golem of Safeguarding's life left (P6-11ar).
    golem: f64,
    /// The shared elemental aegis left (P6-11hl).
    elemental_aegis: f64,
    /// The ward left (P6-11hn).
    ward: f64,
    /// Frost Shield's life left (P6-11ho).
    frost: f64,
}

/// What the estimate reads of the character.
pub(super) struct Character {
    /// `LifeRecoverable` (at least 1).
    pub(super) life: f64,
    /// Maximum life (`output.Life`).
    pub(super) max_life: f64,
    /// `EnergyShieldRecoveryCap`.
    pub(super) energy_shield: f64,
    /// `ManaUnreserved`.
    pub(super) mana: f64,
    /// Each type's share taken from mana before life (mind over matter, a
    /// fraction).
    pub(super) mom: [f64; 5],
    /// The shared guard's absorb rate, percent, and its limit
    /// (`sharedGuardAbsorbRate`, `sharedGuardAbsorb`; P6-11ae).
    pub(super) guard_rate: f64,
    pub(super) guard: f64,
    /// Any recoup (PoB's `anyRecoup` above 0; P6-11am): the mitigated
    /// estimate tracks recoupable damage, which limits its speed-up.
    pub(super) recoup: bool,
    /// Eldritch Battery (PoB's `EnergyShieldProtectsMana`; P6-11ap): the
    /// shield takes mind over matter's share before mana, not the hit.
    pub(super) eb: bool,
    /// Every type bypasses energy shield (PoB's `UnblockedDamageDoesBypassES`;
    /// P6-11hy), as chaos does.
    pub(super) all_bypass: bool,
    /// Summon Stone Golem of Safeguarding's share of each hit, percent, and
    /// its life (PoB's `alliesTakenBeforeYou.stoneGolem`; P6-11ar).
    pub(super) golem_rate: f64,
    pub(super) golem: f64,
    /// The share of life lost to hits that is lost over time instead,
    /// percent, at most 100 (PoB's `preventedLifeLoss`; P6-11ce).
    pub(super) prevented: f64,
    /// The share of the loss below half life that is prevented, percent
    /// (PoB's `LifeLossBelowHalfPrevented` BASE, uncapped; open items S2:
    /// Petrified Blood).
    pub(super) below_half: f64,
    /// The shared elemental aegis (PoB's `sharedElementalAegis`; P6-11hl).
    pub(super) elemental_aegis: f64,
    /// The percent of missing unreserved life gained before each enemy hit
    /// (PoB's `MissingLifeBeforeEnemyHit`; P6-11hm).
    pub(super) missing_life: f64,
    /// Ward (PoB's `output.Ward`; P6-11hn).
    pub(super) ward: f64,
    /// Frost Shield's share of each hit, percent, and its life (PoB's
    /// `alliesTakenBeforeYou.frostShield`, after the golem's; P6-11ho).
    pub(super) frost_rate: f64,
    pub(super) frost: f64,
}

/// What each enemy hit gives back (PoB's `DamageIn.LifeWhenHit`,
/// `ManaWhenHit`, `EnergyShieldWhenHit`; P6-11ak): block's gains times the
/// block chance.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(super) struct Gains {
    pub(super) life: f64,
    pub(super) mana: f64,
    pub(super) energy_shield: f64,
    /// PoB's `LimitEHPSpeedup` for another cause (tracked recoup).
    pub(super) limit: bool,
    /// The percent of missing life gained before each hit (PoB's
    /// `DamageIn.MissingLifeBeforeEnemyHit`, the mitigated run's top level
    /// alone; P6-11hm).
    pub(super) missing_life: f64,
}

impl Gains {
    /// Whether any flat gain is not 0.
    fn any(&self) -> bool {
        self.life != 0.0 || self.mana != 0.0 || self.energy_shield != 0.0
    }

    /// PoB's `GainWhenHit`: a flat gain or missing life gained.
    fn when_hit(&self) -> bool {
        self.any() || self.missing_life != 0.0
    }
}

/// One run of `numberOfHitsToDie`'s state shared with its recursive call.
struct Run {
    iterations: u32,
}

/// PoB's `calcLifeHitPoolWithLossPrevention` (`CalcDefence.lua` 177 to
/// 181): `aboveLow / (1 - p / 100) + min(life, half) / (1 - b / 100) / (1 -
/// p / 100)`, `b` the loss below half prevented (open items S2).
pub(super) fn life_hit_pool(life: f64, max_life: f64, prevented: f64, below_half: f64) -> f64 {
    let half = max_life * 0.5;
    let kept = 1.0 - prevented / 100.0;
    (life - half).max(0.0) / kept + life.min(half) / (1.0 - below_half / 100.0) / kept
}

impl Character {
    /// PoB's `preventedLifeLossTotal` above 0 (`CalcDefence.lua` 2233 to
    /// 2244): some loss is prevented, above or below half life (the share
    /// below half counts once loss above half does not prevent it all).
    pub(super) fn prevents_loss(&self) -> bool {
        self.prevented > 0.0 || (self.below_half > 0.0 && self.prevented < 100.0)
    }
}

/// `calcs.reducePoolsByDamage` for life and energy shield alone: the pools
/// left and the overkill damage.
fn reduce(pools: Pools, damage: &[Option<f64>; 5], ch: &Character) -> (Pools, f64) {
    let (pools, overkill, _) = reduce_full(pools, damage, ch);
    (pools, overkill)
}

/// PoB's `calcs.reducePoolsByDamage(nil, damage, actor)` (open items S7):
/// the hit taken from full pools, as maximum hit's smoothing passes take
/// it (`CalcDefence.lua` 3233 to 3235): its `OverkillDamage` and
/// `hitPoolRemaining`.
pub(super) fn overkill_from_full(damage: &[Option<f64>; 5], ch: &Character) -> (f64, f64) {
    let pools = Pools {
        life: ch.life,
        energy_shield: ch.energy_shield,
        mana: ch.mana,
        guard: ch.guard,
        golem: ch.golem,
        elemental_aegis: ch.elemental_aegis,
        ward: ch.ward,
        frost: ch.frost,
    };
    let (_, overkill, remaining) = reduce_full(pools, damage, ch);
    (overkill, remaining)
}

/// `reduce`, with PoB's `hitPoolRemaining` (`CalcDefence.lua` 396 to 403):
/// the life hit pool left (0 below 1 life), plus the smallest mind over
/// matter pool and the smallest energy shield pool left over the hit's
/// types, floored.
fn reduce_full(pools: Pools, damage: &[Option<f64>; 5], ch: &Character) -> (Pools, f64, f64) {
    let damage: [Option<f64>; 5] = damage.map(|d| d.filter(|&v| v > 0.0).map(crate::math::ceil));
    let mut energy_shield = pools.energy_shield.max(0.0);
    let mut mana = pools.mana.max(0.0);
    let mut life = pools.life.max(0.0);
    let mut guard = pools.guard;
    let mut golem = pools.golem;
    let mut frost = pools.frost;
    let mut elemental_aegis = pools.elemental_aegis;
    let mut ward = pools.ward.max(0.0);
    let ward_before = ward;
    let life_pool = life_hit_pool(life, ch.max_life, ch.prevented, ch.below_half);
    // Each type's mind over matter pool and energy shield pool: all of the
    // shield, or none for chaos (its bypass is 100%: `min(pool / 1 - pool,
    // es)` is 0).
    let mut es_pools = [0.0; 5];
    let mut mom_pools = [0.0; 5];
    for (i, ty) in TYPES.into_iter().enumerate() {
        let effect = ch.mom[i];
        let max_mom = life_pool / (1.0 - effect) - life_pool;
        let mom_pool = if effect < 1.0 {
            max_mom.min(mana)
        } else {
            mana
        };
        let with_mana = life_pool + mom_pool;
        let bypasses = ty == DamageType::Chaos || ch.all_bypass;
        if ch.eb && !bypasses {
            // `CalcDefence.lua` 254 to 256, with no bypass.
            es_pools[i] = crate::math::floor(
                max_mom
                    .min(energy_shield)
                    .min(with_mana / (1.0 - effect) - with_mana),
            );
            mom_pools[i] = crate::math::floor((max_mom - es_pools[i]).min(mom_pool));
            continue;
        }
        es_pools[i] = if bypasses {
            crate::math::floor((with_mana / 1.0 - with_mana).min(energy_shield))
        } else {
            crate::math::floor(energy_shield)
        };
        mom_pools[i] = crate::math::floor(mom_pool);
    }
    let mut overkill = 0.0;
    for (i, ty) in TYPES.into_iter().enumerate() {
        let Some(mut remainder) = damage[i] else {
            continue;
        };
        let bypass = if ty == DamageType::Chaos || ch.all_bypass {
            1.0
        } else {
            0.0
        };
        // An ally takes its share first (P6-11ar; `CalcDefence.lua` 266 to
        // 277), floored after each take.
        if golem > 0.0 {
            let taken = (remainder * ch.golem_rate / 100.0).min(golem);
            golem = crate::math::floor(golem - taken);
            remainder -= taken;
        }
        // Frost Shield after it (P6-11ho; `allyLifePoolList`'s order).
        if frost > 0.0 {
            let taken = (remainder * ch.frost_rate / 100.0).min(frost);
            frost = crate::math::floor(frost - taken);
            remainder -= taken;
        }
        // The shared elemental aegis takes an elemental hit next, all of it
        // up to what is left (P6-11hl; `CalcDefence.lua` 285 to 290).
        if ty != DamageType::Physical && ty != DamageType::Chaos && elemental_aegis > 0.0 {
            let taken = remainder.min(elemental_aegis);
            elemental_aegis -= taken;
            remainder -= taken;
        }
        // The shared guard takes its share next (P6-11ae).
        if guard > 0.0 {
            let taken = (remainder * ch.guard_rate / 100.0).min(guard);
            guard = crate::math::floor(guard - taken);
            remainder -= taken;
        }
        // Ward takes all of the hit up to what is left (P6-11hn;
        // `CalcDefence.lua` 309 to 315, no bypass, always active).
        if ward > 0.0 {
            let taken = remainder.min(ward);
            ward -= taken;
            remainder -= taken;
        }
        if energy_shield > 0.0 && bypass < 1.0 && !ch.eb {
            let taken = (remainder * (1.0 - bypass)).min(es_pools[i]);
            for (j, pool) in es_pools.iter_mut().enumerate() {
                if damage[j].is_some() {
                    *pool = (*pool - taken).max(0.0);
                }
            }
            energy_shield -= taken;
            remainder -= taken;
        }
        let effect = ch.mom[i];
        if effect > 0.0 {
            let mut mom_damage = crate::math::ceil(remainder * effect);
            // Eldritch Battery's shield takes the share first (`CalcDefence.lua`
            // 330 to 339).
            if ch.eb && energy_shield > 0.0 && bypass < 1.0 {
                let taken = crate::math::ceil((mom_damage * (1.0 - bypass)).min(es_pools[i]));
                for (j, pool) in es_pools.iter_mut().enumerate() {
                    if damage[j].is_some() {
                        *pool = crate::math::floor((*pool - taken).max(0.0));
                    }
                }
                energy_shield -= taken;
                remainder -= taken;
                mom_damage -= taken;
            }
            let taken = crate::math::ceil(mom_damage.min(mom_pools[i]));
            for (j, pool) in mom_pools.iter_mut().enumerate() {
                if damage[j].is_some() {
                    *pool = crate::math::floor((*pool - taken).max(0.0));
                }
            }
            mana -= taken;
            remainder -= taken;
        }
        // Life loss prevented (P6-11ce; `CalcDefence.lua` 348 to 384): the
        // damage life can still take caps the hit, and the prevented share
        // is lost over time instead. With loss below half prevented too
        // (open items S2, Petrified Blood: PoB's `preventedLifeLossBelowHalf`
        // not 0), the part above half takes the prevented share, and below
        // half both shares come off what is left.
        if ch.prevents_loss() {
            let half = ch.max_life * 0.5;
            let share = ch.prevented / 100.0;
            let below = ch.below_half / 100.0;
            let above_low = (life - half).max(0.0) / (1.0 - share);
            let can_take = above_low + life.min(half).max(0.0) / (1.0 - below) / (1.0 - share);
            if can_take < remainder {
                overkill += remainder - can_take;
                remainder = can_take;
            }
            if (1.0 - share) * ch.below_half != 0.0 {
                let split = remainder.min(above_low);
                life -= split * (1.0 - share);
                remainder -= split;
                if life <= half {
                    remainder -= remainder * share;
                    remainder -= remainder * below;
                }
            } else {
                let over_time = remainder * ch.prevented / 100.0;
                remainder -= over_time;
            }
        }
        if life > 0.0 {
            let taken = remainder.min(life);
            life -= taken;
            remainder -= taken;
        }
        overkill += remainder;
    }
    let present = |pools: &[f64; 5]| {
        pools
            .iter()
            .zip(&damage)
            .filter(|(_, d)| d.is_some())
            .map(|(p, _)| *p)
            .fold(f64::INFINITY, f64::min)
    };
    let finite = |v: f64| if v.is_finite() { v } else { 0.0 };
    let life_left = if life >= 1.0 {
        life_hit_pool(life, ch.max_life, ch.prevented, ch.below_half)
    } else {
        0.0
    };
    let remaining =
        crate::math::floor(life_left + finite(present(&mom_pools)) + finite(present(&es_pools)));
    (
        Pools {
            life: crate::math::floor(life),
            energy_shield: crate::math::floor(energy_shield),
            mana: crate::math::floor(mana),
            guard,
            golem,
            elemental_aegis,
            // A hit that took any ward breaks it: no chance to avoid the
            // break is modelled (409).
            ward: crate::math::floor(if ward < ward_before { 0.0 } else { ward }),
            frost,
        },
        crate::math::ceil(overkill),
        remaining,
    )
}

/// PoB's `numberOfHitsToDie` (module docs), `cycles` its acceleration,
/// `gains` what each hit gives back (P6-11ak: before each accelerated
/// iteration, `gainMult - 1` hits' worth; after each hit, one, each pool
/// at most its cap). With gains PoB limits the speed-up to 4
/// (`LimitEHPSpeedup`, `CalcDefence.lua` 2682, 2837). `clamp`: the
/// mitigated run's top level, where PoB's `MissingLifeBeforeEnemyHit` and
/// `MissingManaBeforeEnemyHit` are 0, a true value in Lua: before each
/// hit, life and mana go back to at most their caps (2661 to 2666), which
/// drops the life and mana an accelerated iteration gained above them (a
/// PoB oracle probe matches it; the recursion does not carry the fields).
fn hits_to_die(
    damage_in: &[f64; 5],
    cycles: f64,
    run: &mut Run,
    ch: &Character,
    gains: Gains,
    clamp: bool,
) -> f64 {
    let mut num_hits: f64 = damage_in.iter().sum();
    if num_hits == 0.0 {
        return f64::INFINITY;
    }
    num_hits = 0.0;
    let mut pools = Pools {
        life: ch.life,
        energy_shield: ch.energy_shield,
        mana: ch.mana,
        guard: ch.guard,
        golem: ch.golem,
        elemental_aegis: ch.elemental_aegis,
        // An accelerated run leaves ward out: it breaks (2590 to 2593).
        ward: if cycles > 1.0 { 0.0 } else { ch.ward },
        frost: ch.frost,
    };
    let mut iteration_multiplier = 1.0;
    let mut damage_total = 0.0;
    let mut cycles_ran = false;
    let mut overkill = 0.0;
    while pools.life > 0.0 && run.iterations < MAX_ITERATIONS {
        run.iterations += 1;
        damage_total = 0.0;
        let mut damage = [None; 5];
        for (i, &d) in damage_in.iter().enumerate() {
            damage[i] = (d > 0.0).then_some(d * iteration_multiplier);
            damage_total += d;
        }
        if (gains.any() || clamp) && (iteration_multiplier > 1.0 || cycles > 1.0) {
            let mult = iteration_multiplier * cycles;
            // Missing life before each of the hits at once (`CalcDefence.lua`
            // 2649 to 2651; P6-11hm).
            if clamp && gains.missing_life != 0.0 {
                pools.life = (pools.life
                    + gains.missing_life * (ch.life - pools.life) * (mult - 1.0) / 100.0)
                    .min(mult * ch.life);
            }
        }
        if gains.any() && (iteration_multiplier > 1.0 || cycles > 1.0) {
            let mult = iteration_multiplier * cycles;
            pools.life = (pools.life + gains.life * (mult - 1.0)).min(mult * ch.life);
            pools.mana = (pools.mana + gains.mana * (mult - 1.0)).min(mult * ch.mana);
            pools.energy_shield = (pools.energy_shield + gains.energy_shield * (mult - 1.0))
                .min(mult * ch.energy_shield);
        }
        if clamp {
            // `min(life + missing% x (unreserved - life) / 100, recoverable)`
            // (2661 to 2663; P6-11hm), the clamp alone at 0.
            if pools.life > 0.0 {
                pools.life =
                    (pools.life + gains.missing_life * (ch.life - pools.life) / 100.0).min(ch.life);
            }
            if pools.mana > 0.0 {
                pools.mana = pools.mana.min(ch.mana);
            }
        }
        let (next, over) = reduce(pools, &damage, ch);
        pools = next;
        overkill = over;
        if pools.life > 0.0 && damage_total >= MAX_DAMAGE {
            return f64::INFINITY;
        }
        if gains.any() && pools.life > 0.0 {
            pools.life = (pools.life + gains.life).min(ch.life);
            pools.mana = (pools.mana + gains.mana).min(ch.mana);
            pools.energy_shield = (pools.energy_shield + gains.energy_shield).min(ch.energy_shield);
        }
        iteration_multiplier = 1.0;
        if !cycles_ran && pools.life > 0.0 && run.iterations < MAX_ITERATIONS {
            let speed_up = if gains.when_hit() || gains.limit {
                LIMITED_SPEED_UP
            } else {
                SPEED_UP
            };
            let faster = damage_in.map(|d| d * speed_up);
            // The recursion carries `GainWhenHit` and the flat gains, not
            // the missing life (2702 to 2708).
            let inner = Gains {
                missing_life: 0.0,
                limit: gains.limit || gains.missing_life != 0.0,
                ..gains
            };
            let n = hits_to_die(&faster, cycles * speed_up, run, ch, inner, false);
            iteration_multiplier = ((n - 1.0) * speed_up - 1.0).max(1.0);
            if iteration_multiplier == f64::INFINITY {
                return f64::INFINITY;
            }
            cycles_ran = true;
        }
        num_hits += iteration_multiplier;
    }
    if pools.life == 0.0 && cycles == 1.0 {
        num_hits -= overkill / damage_total;
    }
    let total: f64 = damage_in.iter().map(|d| d * num_hits).sum();
    if pools.life >= 0.0 && total >= MAX_DAMAGE {
        return f64::INFINITY;
    }
    if num_hits.is_nan() {
        return 0.0;
    }
    num_hits.max(0.0)
}

/// The block, suppression, and avoidance the estimate reads.
pub(super) struct Mitigation {
    /// The enemy's damage category (open items S7).
    pub(super) category: super::enemy_hit::Category,
    /// The effective attack and spell block chances (projectile ones are
    /// the same without their own lines).
    pub(super) block: f64,
    pub(super) spell_block: f64,
    /// `100 - BlockEffect BASE`.
    pub(super) block_effect: f64,
    /// The effective spell suppression chance.
    pub(super) suppression: f64,
    /// `SpellSuppressionEffect` BASE.
    pub(super) suppression_effect: f64,
    /// The melee and projectile evade chances.
    pub(super) melee_evade: f64,
    pub(super) projectile_evade: f64,
    /// `AvoidProjectilesChance` (capped).
    pub(super) avoid_projectiles: f64,
    /// Life gained on block, energy shield gained on block, and energy
    /// shield gained on blocking spell damage (PoB's `LifeOnBlock`,
    /// `EnergyShieldOnBlock`, `EnergyShieldOnSpellBlock`; P6-11ak).
    pub(super) life_on_block: f64,
    pub(super) energy_shield_on_block: f64,
    pub(super) energy_shield_on_spell_block: f64,
    /// `EffectiveAttackDodgeChance`, `EffectiveSpellDodgeChance`, and
    /// `AvoidAllDamageFromHitsChance` (none of their stats is mapped: 0).
    pub(super) attack_dodge: f64,
    pub(super) spell_dodge: f64,
    pub(super) avoid_all: f64,
}

/// The estimate (module docs): sets its six fields.
///
/// # Errors
/// A graph error.
pub(super) fn estimate(
    calc: &mut Calc<'_>,
    taken_hit: &[f64; 5],
    enemy_damage_in: f64,
    ch: &Character,
    m: &Mitigation,
    reasons: &Reasons,
) -> Result<(), CalcError> {
    let mut run = Run { iterations: 0 };
    // Life loss prevented limits the speed-up (P6-11ce; `CalcDefence.lua`
    // 2742: `LimitEHPSpeedup` when `preventedLifeLossTotal` is above 0).
    let tracks_loss = ch.prevents_loss();
    let damaging = hits_to_die(
        taken_hit,
        1.0,
        &mut run,
        ch,
        Gains {
            limit: tracks_loss,
            ..Gains::default()
        },
        false,
    );
    // Block, suppression, avoidance (`Average`).
    // The category's block chance (2748 to 2751; open items S7): a
    // projectile's is the attack's and a spell projectile's the spell's
    // (no own lines); `Average` the four's mean; an untyped hit's the
    // attack's (`EffectiveBlockChance`, as `Melee`).
    use super::enemy_hit::Category;
    let block_chance = match m.category {
        Category::Average => (m.block + m.block + m.spell_block + m.spell_block) / 4.0,
        Category::Melee | Category::Projectile | Category::Untyped => m.block,
        Category::Spell | Category::SpellProjectile => m.spell_block,
    } / 100.0;
    let block_effect = 1.0 - block_chance * m.block_effect / 100.0;
    // Suppression against a spell or `Average` (halved), none against an
    // attack or an untyped hit (2771 to 2779).
    let mut suppress = match m.category {
        Category::Melee | Category::Projectile | Category::Untyped => 0.0,
        _ => m.suppression / 100.0,
    };
    let suppression_effect = if suppress < 1.0 {
        if m.category == Category::Average {
            suppress /= 2.0;
        }
        1.0 - suppress * (SUPPRESSION_EFFECT + m.suppression_effect).max(0.0) / 100.0
    } else {
        1.0
    };
    // No type's own avoidance is modelled: `AvoidChance` is 0 and the
    // projectile avoidance goes to the chance not to be hit.
    let average_avoid = 0.0;
    let damage_chance = 100.0 * (block_effect * suppression_effect * (1.0 - average_avoid / 100.0));
    // The gains on block (P6-11ak; `CalcDefence.lua` 2759 to 2766): with
    // more than one damaging hit, each times the block chance, spell
    // block's energy shield at half (`Average`). No gain on suppression is
    // mapped.
    let mut gains = if damaging > 1.0 {
        Gains {
            life: m.life_on_block * block_chance,
            mana: 0.0,
            // Spell block's energy shield: all of it against a spell, half
            // for `Average`, none against an attack (2764 to 2768).
            energy_shield: m.energy_shield_on_block * block_chance
                + match m.category {
                    Category::Spell | Category::SpellProjectile => m.energy_shield_on_spell_block,
                    Category::Average => m.energy_shield_on_spell_block / 2.0,
                    Category::Melee | Category::Projectile | Category::Untyped => 0.0,
                } * block_chance,
            limit: false,
            missing_life: ch.missing_life,
        }
    } else {
        Gains::default()
    };
    // Tracked recoup (P6-11am; `CalcDefence.lua` 2828 to 2840): the run is
    // made, its speed-up limited.
    // Tracked life loss over time too (P6-11ce; 2832 to 2837).
    gains.limit = ch.recoup || tracks_loss;
    let mitigated = if damage_chance == 100.0 && !gains.when_hit() && !gains.limit {
        damaging
    } else {
        let damage_in = taken_hit
            .map(|d| d * (block_effect * suppression_effect * (1.0 - average_avoid / 100.0)));
        let mut run = Run { iterations: 0 };
        hits_to_die(&damage_in, 1.0, &mut run, ch, gains, damaging > 1.0)
    };
    // Not hit (1645 to 1667).
    let melee = 100.0
        - (1.0 - m.melee_evade / 100.0)
            * (1.0 - m.attack_dodge / 100.0)
            * (1.0 - m.avoid_all / 100.0)
            * 100.0;
    let projectile = 100.0
        - (1.0 - m.projectile_evade / 100.0)
            * (1.0 - m.attack_dodge / 100.0)
            * (1.0 - m.avoid_all / 100.0)
            * (1.0 - m.avoid_projectiles / 100.0)
            * 100.0;
    let spell = 100.0 - (1.0 - m.spell_dodge / 100.0) * (1.0 - m.avoid_all / 100.0) * 100.0;
    let spell_projectile = 100.0
        - (1.0 - m.spell_dodge / 100.0)
            * (1.0 - m.avoid_all / 100.0)
            * (1.0 - m.avoid_projectiles / 100.0)
            * 100.0;
    // `ConfiguredNotHitChance`: the category's (1655); an untyped hit's
    // the chance to avoid all damage from hits alone (1652).
    let not_hit = match m.category {
        Category::Average => (melee + projectile + spell + spell_projectile) / 4.0,
        Category::Melee => melee,
        Category::Projectile => projectile,
        Category::Spell => spell,
        Category::SpellProjectile => spell_projectile,
        Category::Untyped => 100.0 - (1.0 - m.avoid_all / 100.0) * 100.0,
    };
    let total_hits = mitigated / (1.0 - not_hit / 100.0);
    let ehp = total_hits * enemy_damage_in;
    let deps = [
        Field::Life,
        Field::EnergyShield,
        Field::Poe1(Poe1Field::TakenHitMult(DamageType::Physical)),
    ];
    for (field, value) in [
        (Poe1Field::NumberOfDamagingHits, damaging),
        (Poe1Field::NumberOfMitigatedDamagingHits, mitigated),
        (Poe1Field::TotalNumberOfHits, total_hits),
        (Poe1Field::ConfiguredDamageChance, damage_chance),
        (Poe1Field::ConfiguredNotHitChance, not_hit),
        (Poe1Field::TotalEhp, ehp),
    ] {
        // An infinite estimate (a hit the pools always survive) has no
        // value to compare.
        if !value.is_finite() {
            continue;
        }
        let node = phases::constant(calc, "ehp", value)?;
        let result = phases::total(calc, &[node], Term::Result)?;
        calc.set_derived(
            Field::Poe1(field),
            phases::query(result, reasons.clone()),
            &deps,
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Open items S2: Petrified Blood's loss below half prevented, PoB's
    /// `calcLifeHitPoolWithLossPrevention` (`CalcDefence.lua` 177 to 181).
    #[test]
    fn the_life_hit_pool_should_count_the_loss_below_half_prevented() {
        // 1,000 life, all recoverable, 40% below half: 500 + 500 / 0.6.
        let pool = life_hit_pool(1000.0, 1000.0, 0.0, 40.0);
        assert!((pool - (500.0 + 500.0 / 0.6)).abs() < 1e-9, "{pool}");
        // With 20% prevented above too: both divide below half.
        let pool = life_hit_pool(1000.0, 1000.0, 20.0, 40.0);
        assert!(
            (pool - (500.0 / 0.8 + 500.0 / 0.6 / 0.8)).abs() < 1e-9,
            "{pool}"
        );
        assert_eq!(life_hit_pool(400.0, 1000.0, 0.0, 0.0), 400.0);
    }

    /// A bare level 90 Witch (1,125 life, no energy shield) against PoB's
    /// default Pinnacle boss's physical hit of 1,113 (rounded up to 1,114
    /// before the pools): one hit and a fraction, as PoB's
    /// `NumberOfDamagingHits` for `ice-nova-witch.xml` (the hit there is
    /// 14,885.9; 1,125 / 14,886 = 0.07557...).
    #[test]
    fn hits_to_die_should_be_pobs() {
        let ch = Character {
            life: 1125.0,
            max_life: 1125.0,
            energy_shield: 0.0,
            mana: 590.0,
            mom: [0.0; 5],
            guard_rate: 0.0,
            guard: 0.0,
            recoup: false,
            eb: false,
            all_bypass: false,
            golem_rate: 0.0,
            golem: 0.0,
            prevented: 0.0,
            below_half: 0.0,
            elemental_aegis: 0.0,
            missing_life: 0.0,
            ward: 0.0,
            frost_rate: 0.0,
            frost: 0.0,
        };
        let mut run = Run { iterations: 0 };
        let hits = hits_to_die(
            &[14_885.9, 0.0, 0.0, 0.0, 0.0],
            1.0,
            &mut run,
            &ch,
            Gains::default(),
            false,
        );
        assert!((hits - 1125.0 / 14_885.9).abs() < 1e-3, "{hits}");
    }

    /// PoB's `reducePoolsByDamage` with a shared guard (P6-11ae): a guard
    /// of 1,000 taking 75% of a 1,000 hit keeps 250 and life takes 250;
    /// the next hit's 750 share is capped at the 250 left.
    #[test]
    fn a_guard_should_take_its_share_first() {
        let ch = Character {
            life: 2000.0,
            max_life: 2000.0,
            energy_shield: 0.0,
            mana: 0.0,
            mom: [0.0; 5],
            guard_rate: 75.0,
            guard: 1000.0,
            recoup: false,
            eb: false,
            all_bypass: false,
            golem_rate: 0.0,
            golem: 0.0,
            prevented: 0.0,
            below_half: 0.0,
            elemental_aegis: 0.0,
            missing_life: 0.0,
            ward: 0.0,
            frost_rate: 0.0,
            frost: 0.0,
        };
        let pools = Pools {
            life: ch.life,
            energy_shield: 0.0,
            mana: 0.0,
            guard: ch.guard,
            golem: 0.0,
            elemental_aegis: 0.0,
            ward: 0.0,
            frost: 0.0,
        };
        let hit = [Some(1000.0), None, None, None, None];
        let (after, overkill) = reduce(pools, &hit, &ch);
        assert_eq!((after.guard, after.life, overkill), (250.0, 1750.0, 0.0));
        let (after, _) = reduce(after, &hit, &ch);
        assert_eq!((after.guard, after.life), (0.0, 1000.0));
    }

    /// PoB's `hitPoolRemaining` (open items S7): the life left plus the
    /// smallest energy shield pool left over the hit's types; chaos's pool
    /// is none (its bypass is 100%), so a hit with a chaos part leaves the
    /// shield out; overkill past every pool.
    #[test]
    fn the_hit_pool_left_should_be_pobs() {
        let ch = Character {
            life: 1000.0,
            max_life: 1000.0,
            energy_shield: 500.0,
            mana: 0.0,
            mom: [0.0; 5],
            guard_rate: 0.0,
            guard: 0.0,
            recoup: false,
            eb: false,
            all_bypass: false,
            golem_rate: 0.0,
            golem: 0.0,
            prevented: 0.0,
            below_half: 0.0,
            elemental_aegis: 0.0,
            missing_life: 0.0,
            ward: 0.0,
            frost_rate: 0.0,
            frost: 0.0,
        };
        // 300 physical: the shield takes it, 200 left, life 1000.
        let (overkill, left) = overkill_from_full(&[Some(300.0), None, None, None, None], &ch);
        assert_eq!((overkill, left), (0.0, 1200.0));
        // With a chaos part, the smallest shield pool left is chaos's 0.
        let (_, left) = overkill_from_full(&[Some(300.0), None, None, None, Some(100.0)], &ch);
        assert_eq!(left, 900.0);
        // 2,000 physical: 500 shield, 1,000 life, 500 over.
        let (overkill, left) = overkill_from_full(&[Some(2000.0), None, None, None, None], &ch);
        assert_eq!((overkill, left), (500.0, 0.0));
    }

    /// PoB's `reducePoolsByDamage` with an ally before you (P6-11ar): the
    /// Stone Golem's 3.75% of a 1,000 hit (37.5, floored after the take,
    /// 62 left of 100) before the guard's 50% of the rest; the next hit's
    /// share is capped at what is left.
    #[test]
    fn an_ally_should_take_its_share_before_the_guard() {
        let ch = Character {
            life: 2000.0,
            max_life: 2000.0,
            energy_shield: 0.0,
            mana: 0.0,
            mom: [0.0; 5],
            guard_rate: 50.0,
            guard: 10_000.0,
            recoup: false,
            eb: false,
            all_bypass: false,
            golem_rate: 3.75,
            golem: 100.0,
            prevented: 0.0,
            below_half: 0.0,
            elemental_aegis: 0.0,
            missing_life: 0.0,
            ward: 0.0,
            frost_rate: 0.0,
            frost: 0.0,
        };
        let pools = Pools {
            life: ch.life,
            energy_shield: 0.0,
            mana: 0.0,
            guard: ch.guard,
            golem: ch.golem,
            elemental_aegis: 0.0,
            ward: 0.0,
            frost: 0.0,
        };
        let hit = [Some(1000.0), None, None, None, None];
        let (after, _) = reduce(pools, &hit, &ch);
        assert_eq!(after.golem, 62.0);
        assert_eq!(after.guard, crate::math::floor(10_000.0 - 962.5 * 0.5));
        let (after, _) = reduce(after, &hit, &ch);
        assert_eq!(after.golem, 24.0);
        let (after, _) = reduce(after, &hit, &ch);
        assert_eq!(after.golem, 0.0);
    }
}
