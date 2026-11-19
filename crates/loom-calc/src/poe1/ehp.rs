//! The effective health pool (Phase 6 P6-10b, part 14): PoB v2.67.2's
//! `NumberOfDamagingHits`, `NumberOfMitigatedDamagingHits`,
//! `TotalNumberOfHits`, `ConfiguredDamageChance`, `ConfiguredNotHitChance`,
//! and `TotalEHP` (`Modules/CalcDefence.lua` 188 to 427, 1645 to 1667,
//! 2568 to 2913), for the default `Average` estimate.
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
//! type's own guard, ward, aegis, allies taking damage, life loss
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
}

impl Gains {
    /// Whether any gain is not 0 (PoB's `GainWhenHit`).
    fn any(&self) -> bool {
        self.life != 0.0 || self.mana != 0.0 || self.energy_shield != 0.0
    }
}

/// One run of `numberOfHitsToDie`'s state shared with its recursive call.
struct Run {
    iterations: u32,
}

/// PoB's `calcLifeHitPoolWithLossPrevention` with no loss prevented.
fn life_hit_pool(life: f64, max_life: f64) -> f64 {
    let half = max_life * 0.5;
    (life - half).max(0.0) + life.min(half)
}

/// `calcs.reducePoolsByDamage` for life and energy shield alone: the pools
/// left and the overkill damage.
fn reduce(pools: Pools, damage: &[Option<f64>; 5], ch: &Character) -> (Pools, f64) {
    let damage: [Option<f64>; 5] = damage.map(|d| d.filter(|&v| v > 0.0).map(crate::math::ceil));
    let mut energy_shield = pools.energy_shield.max(0.0);
    let mut mana = pools.mana.max(0.0);
    let mut life = pools.life.max(0.0);
    let mut guard = pools.guard;
    let life_pool = life_hit_pool(life, ch.max_life);
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
        if ch.eb && ty != DamageType::Chaos {
            // `CalcDefence.lua` 254 to 256, with no bypass.
            es_pools[i] = crate::math::floor(
                max_mom
                    .min(energy_shield)
                    .min(with_mana / (1.0 - effect) - with_mana),
            );
            mom_pools[i] = crate::math::floor((max_mom - es_pools[i]).min(mom_pool));
            continue;
        }
        es_pools[i] = if ty == DamageType::Chaos {
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
        let bypass = if ty == DamageType::Chaos { 1.0 } else { 0.0 };
        // The shared guard takes its share first (P6-11ae).
        if guard > 0.0 {
            let taken = (remainder * ch.guard_rate / 100.0).min(guard);
            guard = crate::math::floor(guard - taken);
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
        if life > 0.0 {
            let taken = remainder.min(life);
            life -= taken;
            remainder -= taken;
        }
        overkill += remainder;
    }
    (
        Pools {
            life: crate::math::floor(life),
            energy_shield: crate::math::floor(energy_shield),
            mana: crate::math::floor(mana),
            guard,
        },
        crate::math::ceil(overkill),
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
        if gains.any() && (iteration_multiplier > 1.0 || cycles > 1.0) {
            let mult = iteration_multiplier * cycles;
            pools.life = (pools.life + gains.life * (mult - 1.0)).min(mult * ch.life);
            pools.mana = (pools.mana + gains.mana * (mult - 1.0)).min(mult * ch.mana);
            pools.energy_shield = (pools.energy_shield + gains.energy_shield * (mult - 1.0))
                .min(mult * ch.energy_shield);
        }
        if clamp {
            if pools.life > 0.0 {
                pools.life = pools.life.min(ch.life);
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
            let speed_up = if gains.any() || gains.limit {
                LIMITED_SPEED_UP
            } else {
                SPEED_UP
            };
            let faster = damage_in.map(|d| d * speed_up);
            let n = hits_to_die(&faster, cycles * speed_up, run, ch, gains, false);
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
    let damaging = hits_to_die(taken_hit, 1.0, &mut run, ch, Gains::default(), false);
    // Block, suppression, avoidance (`Average`).
    let block_chance = (m.block + m.block + m.spell_block + m.spell_block) / 4.0 / 100.0;
    let block_effect = 1.0 - block_chance * m.block_effect / 100.0;
    let mut suppress = m.suppression / 100.0;
    let suppression_effect = if suppress < 1.0 {
        suppress /= 2.0;
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
            energy_shield: m.energy_shield_on_block * block_chance
                + m.energy_shield_on_spell_block / 2.0 * block_chance,
            limit: false,
        }
    } else {
        Gains::default()
    };
    // Tracked recoup (P6-11am; `CalcDefence.lua` 2828 to 2840): the run is
    // made, its speed-up limited.
    gains.limit = ch.recoup;
    let mitigated = if damage_chance == 100.0 && !gains.any() && !gains.limit {
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
    let not_hit = (melee + projectile + spell + spell_projectile) / 4.0;
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
        };
        let pools = Pools {
            life: ch.life,
            energy_shield: 0.0,
            mana: 0.0,
            guard: ch.guard,
        };
        let hit = [Some(1000.0), None, None, None, None];
        let (after, overkill) = reduce(pools, &hit, &ch);
        assert_eq!((after.guard, after.life, overkill), (250.0, 1750.0, 0.0));
        let (after, _) = reduce(after, &hit, &ch);
        assert_eq!((after.guard, after.life), (0.0, 1000.0));
    }
}
