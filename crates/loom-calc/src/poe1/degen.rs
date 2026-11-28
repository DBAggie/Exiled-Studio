//! Degeneration (open items S7, part 8): PoB v2.67.2's `TotalBuildDegen`,
//! `NetLifeRegen`, `NetManaRegen`, `NetEnergyShieldRegen`, and
//! `TotalNetRegen` (`Modules/CalcDefence.lua` 1288 to 1294, 1802 to 1821,
//! 3339 to 3486).
//!
//! What PoB does:
//! - **The build's degeneration:** each type's `<Type>Degen` BASE sum (Blood
//!   Rage's and Righteous Fire's per point of life, energy shield, or mana,
//!   PoB's `PerStat` with `floor(stat + 0.0001)`; Vaal Temptation's flat),
//!   for each type it is taken as over time (`damageOverTimeShiftTable`: the
//!   type's `DamageTakenAs` without the hits-only ones, the rest to itself),
//!   times that type's `<Type>TakenDotMult`, into `<Type>BuildDegen` and
//!   `TotalBuildDegen`; none when the total is 0.
//! - **Net regeneration** (only with a total): each pool's
//!   `<Pool>RegenRecovery` (its regeneration, less its own degeneration, plus
//!   its recovery), less each type's degeneration split as a hit is: mind
//!   over matter's share to mana; with energy shield regeneration above 0,
//!   the share that does not bypass energy shield to it (to the mind over
//!   matter part alone with Eldritch Battery); the rest to life.
//!   `TotalNetRegen` is the three's sum.
//!
//! - **Mana Burn** (the agent list, 2026-11-27; tinctures' debuff on you):
//!   the config's `multiplierManaBurnStacks` sets PoB's
//!   `EffectiveManaBurnStacks` (`CalcPerform.lua` 902 to 915), and
//!   `ManaDegenPercentTincture` BASE 1 per stack (`CalcSetup.lua` 545)
//!   gives mana its own degeneration, `max(mana x stacks / 100, stacks)`
//!   (`CalcDefence.lua` 1289 to 1292, at least 1 per stack), taken from
//!   mana's regeneration in `ManaRegenRecovery`, so in `NetManaRegen`
//!   alone. PoB's `ManaBurnStacks` multiplier has no mapped reader.
//!
//! Not modelled, each a reason on the net fields: a pool's own degeneration
//! (rage's life loss, Frost Shield's energy shield) and recovery (Pious
//! Path's), from [`super::Poe1Quantity::NetRegenReason`] markers; a bypass
//! between 0 and 100, and the other sources of a type's degeneration, have
//! no mapped stat (unmapped stats, so reasons on every field).

use super::phases;
use super::{DegenOf, Poe1Field, Poe1Quantity, Pool, TakenOf};
use crate::condition::{Applies, Flags};
use crate::engine::Calc;
use crate::error::CalcError;
use crate::graph::Term;
use crate::quantity::{DamageType, Op, Quantity};
use crate::result::Field;
use crate::state::{Reasons, UnsupportedReason};

/// The damage types in PoB's order (`dmgTypeList`).
const TYPES: [DamageType; 5] = [
    DamageType::Physical,
    DamageType::Lightning,
    DamageType::Cold,
    DamageType::Fire,
    DamageType::Chaos,
];

/// The phase (module docs).
///
/// # Errors
/// A graph error.
pub fn phase(calc: &mut Calc<'_>) -> Result<(), CalcError> {
    let value = |calc: &Calc<'_>, f: Field| calc.field(f).and_then(|r| r.value);
    let (Some(life), Some(mana), Some(es)) = (
        value(calc, Field::Life),
        value(calc, Field::Mana),
        value(calc, Field::EnergyShield),
    ) else {
        return Ok(());
    };
    // Righteous Fire's threshold (PoB's `StatThreshold` on `LifeUnreserved`
    // at least 2), as the hit settles it later (`poe1::hit`; the first
    // settled value stays).
    if let Some(unreserved) = value(calc, Field::LifeUnreserved) {
        calc.settle_state(
            crate::condition::ConditionKey::new(super::gems::LIFE_THRESHOLD),
            unreserved >= 2.0,
        );
    }
    let cx = calc.context(Flags::NONE);
    let mut reasons = Reasons::new();
    // Each type's `<Type>Degen` BASE sum, `PerStat` per point.
    let mut base = [0.0; 5];
    for (i, ty) in TYPES.into_iter().enumerate() {
        for e in calc.store().effects() {
            let Quantity::Poe1(Poe1Quantity::Degen(t, of)) = e.quantity else {
                continue;
            };
            if t != ty || e.op != Op::Base {
                continue;
            }
            match e.conditions.applies(&cx) {
                Applies::Yes => {
                    let per = match of {
                        DegenOf::Flat => 1.0,
                        DegenOf::Life => crate::math::floor(life + 0.0001),
                        DegenOf::EnergyShield => crate::math::floor(es + 0.0001),
                        DegenOf::Mana => crate::math::floor(mana + 0.0001),
                    };
                    base[i] += e.value * per;
                }
                Applies::No => {}
                Applies::Unknown(keys) => {
                    for condition in keys {
                        reasons
                            .partial
                            .insert(UnsupportedReason::UnmodelledCondition {
                                condition,
                                quantity: e.quantity,
                                source: e.source.clone(),
                            });
                    }
                }
            }
        }
    }
    if base.iter().all(|b| *b <= 0.0) {
        return Ok(());
    }
    // Each type's shift over time (`CalcDefence.lua` 1802 to 1821): the
    // `DamageTakenAs` sums without the hits-only ones.
    let mut shares = [[0.0; 5]; 5];
    for (i, from) in TYPES.into_iter().enumerate() {
        let mut moved = 0.0;
        for (j, to) in TYPES.into_iter().enumerate() {
            if i == j {
                continue;
            }
            let mut share = 0.0;
            for of in super::enemy_hit::taken_as_sources(from) {
                share += sum(
                    calc,
                    Poe1Quantity::DamageTakenAs(of, to, false),
                    &cx,
                    &mut reasons,
                )?;
            }
            shares[i][j] = share;
            moved += share;
        }
        shares[i][i] = (100.0 - moved).max(0.0);
    }
    let mut dot_mult = [0.0; 5];
    let mut deps = vec![Field::Life, Field::Mana, Field::EnergyShield];
    for (i, ty) in TYPES.into_iter().enumerate() {
        let f = Field::Poe1(Poe1Field::TakenDotMult(ty));
        let Some(m) = value(calc, f) else {
            return Ok(());
        };
        dot_mult[i] = m;
        deps.push(f);
    }
    // `<Type>BuildDegen` and `TotalBuildDegen` (3341 to 3391), in PoB's
    // order: each type, then each type it is taken as.
    let mut build_degen: [Option<f64>; 5] = [None; 5];
    let mut total = 0.0;
    for i in 0..5 {
        if base[i] <= 0.0 {
            continue;
        }
        for c in 0..5 {
            let percent = shares[i][c];
            if percent > 0.0 {
                let part = base[i] * (percent / 100.0) * dot_mult[c];
                build_degen[c] = Some(build_degen[c].unwrap_or(0.0) + part);
                total += part;
            }
        }
    }
    if total == 0.0 {
        return Ok(());
    }
    let node = phases::constant(calc, "build-degen", total)?;
    let result = phases::total(calc, &[node], Term::Result)?;
    let degen_field = Field::Poe1(Poe1Field::TotalBuildDegen);
    calc.set_derived(degen_field, phases::query(result, reasons.clone()), &deps);
    net_regen(calc, &build_degen, degen_field, reasons)
}

/// The net regeneration of each pool and their sum (`CalcDefence.lua` 3395
/// to 3468).
fn net_regen(
    calc: &mut Calc<'_>,
    build_degen: &[Option<f64>; 5],
    degen_field: Field,
    mut reasons: Reasons,
) -> Result<(), CalcError> {
    let value = |calc: &Calc<'_>, f: Field| calc.field(f).and_then(|r| r.value);
    let regen_fields = [
        Field::Poe1(Poe1Field::LifeRegen),
        Field::Poe1(Poe1Field::ManaRegen),
        Field::Poe1(Poe1Field::EnergyShieldRegen),
    ];
    let (Some(life_regen), Some(mana_regen), Some(es_regen)) = (
        value(calc, regen_fields[0]),
        value(calc, regen_fields[1]),
        value(calc, regen_fields[2]),
    ) else {
        return Ok(());
    };
    reasons.extend(&phases::marker_reasons(
        calc,
        |q| match q {
            Quantity::Poe1(Poe1Quantity::NetRegenReason(stat)) => Some(stat),
            _ => None,
        },
        |_| false,
    ));
    let cx = calc.context(Flags::NONE);
    // Mana's own degeneration from the config's Mana Burn stacks (module
    // docs): `ManaRegenRecovery = regen - degen` (1288 to 1295).
    let mana_regen = match mana_burn_stacks(calc) {
        Some(stacks) if stacks > 0.0 => {
            let mana = value(calc, Field::Mana).unwrap_or(0.0);
            let own = crate::math::max(mana * stacks / 100.0, stacks);
            mana_regen - own
        }
        _ => mana_regen,
    };
    // Mind over matter's share of each type, percent (2306, 2347).
    let shared = sum(
        calc,
        Poe1Quantity::ManaBeforeLife(TakenOf::All),
        &cx,
        &mut reasons,
    )?
    .min(100.0);
    let eb = flag(calc, Poe1Quantity::EsProtectsMana, &cx, &mut reasons)?;
    let all_bypass = flag(
        calc,
        Poe1Quantity::UnblockedDamageBypassesEs,
        &cx,
        &mut reasons,
    )?;
    // "Chaos Damage taken does not bypass Energy Shield" (the agent list).
    let chaos_not_bypass = flag(calc, Poe1Quantity::ChaosNotBypassEs, &cx, &mut reasons)?;
    // `<Pool>RegenRecovery` with no degeneration or recovery of the pool
    // itself (1293): its regeneration.
    let mut life_degen = 0.0;
    let mut mana_degen = 0.0;
    let mut es_degen = 0.0;
    for (i, ty) in TYPES.into_iter().enumerate() {
        let Some(degen) = build_degen[i] else {
            continue;
        };
        let own = sum(
            calc,
            Poe1Quantity::ManaBeforeLife(TakenOf::Type(ty)),
            &cx,
            &mut reasons,
        )?
        .min(100.0 - shared);
        let from_mana = own + shared;
        let bypass = if super::ehp::bypasses(ty, all_bypass, chaos_not_bypass) {
            100.0
        } else {
            0.0
        };
        let (life, es, mana) = if es_regen > 0.0 {
            let mana = degen * (bypass / 100.0) * (from_mana / 100.0);
            if eb {
                (
                    degen * (1.0 - from_mana / 100.0),
                    degen * (1.0 - bypass / 100.0) * (from_mana / 100.0),
                    mana,
                )
            } else {
                (
                    degen * (bypass / 100.0) * (1.0 - from_mana / 100.0),
                    degen * (1.0 - bypass / 100.0),
                    mana,
                )
            }
        } else {
            (
                degen * (1.0 - from_mana / 100.0),
                0.0,
                degen * (from_mana / 100.0),
            )
        };
        life_degen += life;
        mana_degen += mana;
        es_degen += es;
    }
    let net = [
        life_regen - life_degen,
        mana_regen - mana_degen,
        es_regen - es_degen,
    ];
    for (k, pool) in [Pool::Life, Pool::Mana, Pool::EnergyShield]
        .into_iter()
        .enumerate()
    {
        let node = phases::constant(calc, "net-regen", net[k])?;
        let result = phases::total(calc, &[node], Term::Result)?;
        // Every net field reads energy shield's regeneration too: it picks
        // the split.
        calc.set_derived(
            Field::Poe1(Poe1Field::NetRegen(pool)),
            phases::query(result, reasons.clone()),
            &[
                degen_field,
                regen_fields[0],
                regen_fields[1],
                regen_fields[2],
            ],
        );
    }
    let node = phases::constant(calc, "total-net-regen", net[0] + net[1] + net[2])?;
    let result = phases::total(calc, &[node], Term::Result)?;
    calc.set_derived(
        Field::Poe1(Poe1Field::TotalNetRegen),
        phases::query(result, reasons),
        &[
            degen_field,
            regen_fields[0],
            regen_fields[1],
            regen_fields[2],
        ],
    );
    Ok(())
}

/// The config input of the Mana Burn stacks on you (`ConfigOptions.lua`
/// 1105).
pub const MANA_BURN_INPUT: &str = "multiplierManaBurnStacks";

/// Whether the degeneration phase handles the config input `name` with
/// `value` (a number of [`MANA_BURN_INPUT`]).
#[must_use]
pub fn handles(name: &str, value: &loom_domain::ConfigValue) -> bool {
    name == MANA_BURN_INPUT && matches!(value, loom_domain::ConfigValue::Number(_))
}

/// The config's Mana Burn stacks, when set (`CalcPerform.lua` 902 to 915:
/// at most `MaxManaBurnStacks`, plus `EffectiveManaBurnStacks`; Weeping
/// Wounds instead with its keystone; those three have no mapped stat, so a
/// line that sets one is a reason of its own).
fn mana_burn_stacks(calc: &Calc<'_>) -> Option<f64> {
    let set = super::collect::active_config(calc.input().build)?;
    match super::config::last_input(set, MANA_BURN_INPUT, loom_domain::ConfigInputKind::Input)? {
        loom_domain::ConfigValue::Number(n) => Some(n.value().0),
        _ => None,
    }
}

/// The `BASE` sum of `quantity` in `cx`, its reasons into `reasons`.
fn sum(
    calc: &mut Calc<'_>,
    quantity: Poe1Quantity,
    cx: &crate::condition::QueryContext,
    reasons: &mut Reasons,
) -> Result<f64, CalcError> {
    let (store, graph) = calc.parts();
    let q = store.sum(
        graph,
        Quantity::Poe1(quantity),
        Op::Base,
        cx,
        Term::Step("degen-sum"),
    )?;
    reasons.extend(&q.reasons);
    Ok(q.value)
}

/// Whether `quantity`'s flag is set, read only when present (so a build
/// without it gains no node).
fn flag(
    calc: &mut Calc<'_>,
    quantity: Poe1Quantity,
    cx: &crate::condition::QueryContext,
    reasons: &mut Reasons,
) -> Result<bool, CalcError> {
    if !phases::has(calc, Quantity::Poe1(quantity)) {
        return Ok(false);
    }
    let (store, graph) = calc.parts();
    let q = store.flag(graph, Quantity::Poe1(quantity), cx)?;
    reasons.extend(&q.reasons);
    Ok(q.value != 0.0)
}
