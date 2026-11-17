//! The main skill's own damage over time (Phase 6 plan, P6-1), as PoB
//! v2.67.2 computes it (`Modules/CalcOffence.lua` 5647 to 5720,
//! "Calculate skill DOT components"):
//!
//! 1. **The DoT config** (`dotCfg`): the skill's flags with `Dot`; `Area`,
//!    `Projectile`, `Spell`, `Attack`, and `Hit` kept only when the skill's
//!    data says its DoT is one (`dotIsArea`, ...; PoB's hand-written base
//!    mods, [`super::pob_skill`]); per damage type the type's DoT keyword
//!    (`KeywordFlag.FireDot`).
//! 2. **Base** per type: the skill's `<Type>Dot` (the game's
//!    `base_<type>_damage_to_deal_per_minute / 60`; Righteous Fire's from
//!    the character's life and energy shield, its `preDamageFunc`), 0 for a
//!    type the skill cannot deal (`DealNo<Type>`).
//! 3. **Per type** `base x (1 + inc / 100) x more x (1 + mult / 100) x
//!    effMult`: `inc` and `more` of `Damage` in the DoT config (the type's
//!    own and elemental names too, each more rounded per name, as a hit's),
//!    `mult` the damage over time multipliers (`DotMultiplier`), `effMult`
//!    `(1 - resist / 100) x (1 + taken inc / 100) x taken more`: the
//!    enemy's resistance without penetration (`calcResistForType`; physical
//!    the enemy's physical damage reduction, which nothing modelled gives,
//!    0) and its damage taken.
//! 4. **`TotalDot`** the sum over the types, at most `DotDpsCap`
//!    (35791394).
//!
//! Not modelled, each a reason or `NotComputed`: stacking DoT (`DotCanStack`,
//! speed times duration), burning, caustic, and Corrupting Blood ground
//! (PoB writes them to their own outputs), auras' effect on their DoT,
//! brands' attached count, and every kind the hit leaves out
//! ([`NOT_COMPUTED_KINDS`]: triggered, traps, mines, minions, brands).
//! Totems' DoT is `Partial` through [`super::hit::TOTEM_REASON`].

use loom_domain::Build;

use super::gem_levels::GemLevels;
use super::hit;
use super::phases::{self, Valued};
use super::pob_skill::{self, PobSkill};
use super::skill::{self, MainSkill};
use crate::condition::{Flags, QueryContext};
use crate::engine::Calc;
use crate::error::CalcError;
use crate::graph::Term;
use crate::input::CalcData;
use crate::quantity::{DamageType, Op, Quantity};
use crate::result::Field;
use crate::source::Source;
use crate::state::{Reasons, UnsupportedReason};

/// PoB's `data.misc.DotDpsCap`: `(2 ^ 31 - 1) / 60`.
pub const DOT_DPS_CAP: f64 = 35_791_394.0;

/// The game's stat of a type's base damage over time, per minute.
pub const BASE_DOT_STATS: [(&str, DamageType); 5] = [
    (
        "base_physical_damage_to_deal_per_minute",
        DamageType::Physical,
    ),
    (
        "base_lightning_damage_to_deal_per_minute",
        DamageType::Lightning,
    ),
    ("base_cold_damage_to_deal_per_minute", DamageType::Cold),
    ("base_fire_damage_to_deal_per_minute", DamageType::Fire),
    ("base_chaos_damage_to_deal_per_minute", DamageType::Chaos),
];

/// Righteous Fire's stats: its burn per minute, per 100 of maximum life
/// and maximum energy shield (PoB's `RFLifeMultiplier`, `RFESMultiplier`,
/// `div = 6000`).
pub const RF_LIFE_STAT: &str = "base_righteous_fire_%_of_max_life_to_deal_to_nearby_per_minute";
/// See [`RF_LIFE_STAT`].
pub const RF_ES_STAT: &str =
    "base_righteous_fire_%_of_max_energy_shield_to_deal_to_nearby_per_minute";

/// The `SkillData` keys of PoB's DoT this phase reads (its flags; P6-6b
/// review: an explicit list, so a new key is a reason).
pub const DOT_SKILL_DATA_KEYS: [&str; 5] = [
    "dotIsArea",
    "dotIsAttack",
    "dotIsHit",
    "dotIsProjectile",
    "dotIsSpell",
];

/// The `FLAG` base mods of PoB's DoT this phase reads: the ground and
/// stacking DoTs it leaves not computed.
pub const DOT_FLAGS: [&str; 5] = [
    "DotCanStack",
    "dotIsBurningGround",
    "dotIsCausticGround",
    "dotIsCorruptingBlood",
    "dotIsHeraldOfAsh",
];

/// Vaal Righteous Fire's stats: the share of life and energy shield it
/// sacrifices and the burn per second, percent of that (PoB's
/// `percentSacrificed`, `RFMultiplier`, `div = 100`).
pub const VAAL_RF_SACRIFICE_STAT: &str = "vaal_righteous_fire_life_and_es_%_to_lose_on_use";
/// See [`VAAL_RF_SACRIFICE_STAT`].
pub const VAAL_RF_DAMAGE_STAT: &str = "vaal_righteous_fire_life_and_es_%_as_damage_per_second";

/// Skill stats this phase reads from the gem instance itself, so the
/// mapping gives them no effect ([`super::mapping::SKILL_FLAGS`]).
pub const DOT_SKILL_STATS: [&str; 16] = [
    "base_physical_damage_to_deal_per_minute",
    "base_lightning_damage_to_deal_per_minute",
    "base_cold_damage_to_deal_per_minute",
    "base_fire_damage_to_deal_per_minute",
    "base_chaos_damage_to_deal_per_minute",
    RF_LIFE_STAT,
    RF_ES_STAT,
    VAAL_RF_SACRIFICE_STAT,
    VAAL_RF_DAMAGE_STAT,
    SPELL_DOT_STAT,
    PROJECTILE_DOT_STAT,
    // Righteous Fire's own buff and degeneration (PoB's `GlobalEffect`
    // buff on the character, and life and energy shield lost): when it is
    // the main skill they change no field this engine computes (its burn is
    // not a spell, PoB's `dotCfg`; the slice has no degeneration).
    "righteous_fire_spell_damage_+%_final",
    "vaal_righteous_fire_spell_damage_+%_final",
    "base_nonlethal_fire_damage_%_of_maximum_life_taken_per_minute",
    "base_nonlethal_fire_damage_%_of_maximum_energy_shield_taken_per_minute",
    "modifiers_to_skill_effect_duration_also_affect_soul_prevention_duration",
];

/// "Spell damage modifiers apply to this skill's damage over time" (PoB's
/// `skill("dotIsSpell", true)`, `SkillStatMap.lua` 161).
pub const SPELL_DOT_STAT: &str = "spell_damage_modifiers_apply_to_skill_dot";
/// "Projectile damage modifiers apply to this skill's damage over time"
/// (PoB's `skill("dotIsProjectile", true)`, `SkillStatMap.lua` 164).
pub const PROJECTILE_DOT_STAT: &str = "projectile_damage_modifiers_apply_to_skill_dot";

/// The skills whose PoB skill function changes their damage over time
/// (`Data/Skills/*.lua`: a function that reads or writes a `Dot` value),
/// other than the two Righteous Fires ported here and Herald of Ash (left
/// not computed). Found by searching every skill function of PoB v2.67.2
/// (`docs/phase-6/dot.md`).
pub const DOT_FUNCTIONS_NOT_PORTED: [&str; 3] = [
    "RighteousFireAltX",
    "WaveOfConviction",
    "WaveOfConvictionAltY",
];

/// Skill types whose DoT this phase leaves not computed: the kinds the
/// hit leaves out ([`hit::NOT_PLAIN_ATTACK`]; a test keeps them equal).
pub const NOT_COMPUTED_KINDS: [&str; 6] = [
    "Triggered",
    "Trapped",
    "RemoteMined",
    "Minion",
    "CreatesMinion",
    "Brand",
];

/// The main skill's base damage over time, per second, by [`hit::TYPES`]
/// position, from its instance's stats: `None` when it has none.
#[must_use]
pub fn skill_dot_bases(stats: &[(String, i64)]) -> Option<[f64; 5]> {
    let mut out = [0.0; 5];
    let mut any = false;
    for (stat, t) in BASE_DOT_STATS {
        let Some((_, v)) = stats.iter().find(|(s, _)| s == stat) else {
            continue;
        };
        if *v <= 0 {
            continue;
        }
        let Some(i) = hit::TYPES.iter().position(|x| *x == t) else {
            continue;
        };
        #[expect(clippy::cast_precision_loss, reason = "stat values are far below 2^53")]
        let v = *v as f64;
        if let Some(slot) = out.get_mut(i) {
            *slot = v / 60.0;
            any = true;
        }
    }
    any.then_some(out)
}

/// Righteous Fire's multipliers (`RFLifeMultiplier`, `RFESMultiplier`):
/// `None` unless both stats are present.
#[must_use]
pub fn righteous_fire(stats: &[(String, i64)]) -> Option<(f64, f64)> {
    let get = |name: &str| stats.iter().find(|(s, _)| s == name).map(|(_, v)| *v);
    let (life, es) = (get(RF_LIFE_STAT)?, get(RF_ES_STAT)?);
    #[expect(clippy::cast_precision_loss, reason = "stat values are far below 2^53")]
    Some((life as f64 / 6000.0, es as f64 / 6000.0))
}

/// Vaal Righteous Fire's `percentSacrificed` and `RFMultiplier`: `None`
/// unless both stats are present.
#[must_use]
pub fn vaal_righteous_fire(stats: &[(String, i64)]) -> Option<(f64, f64)> {
    let get = |name: &str| stats.iter().find(|(s, _)| s == name).map(|(_, v)| *v);
    let (sacrificed, damage) = (get(VAAL_RF_SACRIFICE_STAT)?, get(VAAL_RF_DAMAGE_STAT)?);
    #[expect(clippy::cast_precision_loss, reason = "stat values are far below 2^53")]
    Some((sacrificed as f64 / 100.0, damage as f64 / 100.0))
}

/// The main skill when it has damage over time of its own (module docs),
/// with its group (0-based); `None` otherwise. A skill whose hit the engine
/// computes is the hit's main skill too.
#[must_use]
pub fn dot_skill<'d>(
    build: &Build,
    data: &'d dyn CalcData,
    levels: &GemLevels,
) -> Option<(usize, MainSkill<'d>)> {
    let skill = skill::main_skill(build, data, levels).ok()?;
    let group = skill.group.checked_sub(1)?;
    if !skill::active_groups(build).ok()?.get(group)?.enabled {
        return None;
    }
    let gem = skill.active_gem;
    let instance = skill::instance(skill.effect(), gem.level, gem.quality, data);
    if skill_dot_bases(&instance.stats).is_none()
        && righteous_fire(&instance.stats).is_none()
        && vaal_righteous_fire(&instance.stats).is_none()
    {
        return None;
    }
    Some((group, skill))
}

/// Whether the DoT of a skill with `types` is one this phase computes.
fn computed_kind(skill: &MainSkill<'_>) -> bool {
    !NOT_COMPUTED_KINDS.iter().any(|t| skill.types.contains(*t))
}

/// The DoT config's flags before the type (module docs, step 1): PoB's
/// skill data (`dotIsArea` and kin, its hand-written base mods) or the
/// skill's own stats that set them (`spell_damage_modifiers_apply_to_skill_dot`).
fn dot_flags(skill: &MainSkill<'_>, pob: Option<&PobSkill>, stats: &[(String, i64)]) -> Flags {
    let has = |t: &str| skill.types.contains(t);
    let stat = |name: &str| stats.iter().any(|(s, v)| s == name && *v != 0);
    let data = |key: &str| {
        pob.and_then(|p| p.skill_data(key))
            .is_some_and(|v| v.truthy())
            || (key == "dotIsSpell" && stat(SPELL_DOT_STAT))
            || (key == "dotIsProjectile" && stat(PROJECTILE_DOT_STAT))
    };
    let pob_flag = |name: &str| pob.is_some_and(|p| p.flag(name));
    // The skill's own flags (`skillCfg.flags`: PoB's base flags, else the
    // game's types), as its hit would have them.
    let mut skill_flags = skill.keyword_flags();
    for (flag, pob_name, game_type) in [
        (Flags::SPELL, "spell", "Spell"),
        (Flags::ATTACK, "attack", "Attack"),
        (Flags::AREA, "area", "Area"),
        (Flags::PROJECTILE, "projectile", "Projectile"),
        (Flags::MELEE, "melee", "Melee"),
    ] {
        if pob_flag(pob_name) || (pob.is_none() && has(game_type)) {
            skill_flags = skill_flags | flag;
        }
    }
    if pob_flag("hit") || has("Damage") || has("Attack") {
        skill_flags = skill_flags | Flags::HIT;
    }
    if !has("Attack") {
        skill_flags = skill_flags | Flags::CAST;
    }
    // The keyword flags PoB's `dotCfg` keeps (`keywordFlags` less `Hit`).
    if skill_flags.contains(Flags::SPELL) {
        skill_flags = skill_flags | Flags::KEYWORD_SPELL;
    }
    if skill_flags.contains(Flags::ATTACK) {
        skill_flags = skill_flags | Flags::KEYWORD_ATTACK;
    }
    // `KeywordFlag.Arrow` from the skill's stat (P6-11c).
    if stat(hit::ARROW_STAT) {
        skill_flags = skill_flags | Flags::KEYWORD_ARROW;
    }
    // `dotCfg` (`CalcOffence.lua` 5648 to 5671): `Dot` added; `Area`,
    // `Projectile`, `Spell`, `Attack`, and `Hit` removed unless the skill's
    // DoT is one; every other flag kept.
    // Melee, else projectile (`CalcActiveSkill.lua` 361 to 366; P6-6c
    // review).
    if skill_flags.contains(Flags::MELEE) {
        skill_flags = skill_flags.without(Flags::PROJECTILE);
    }
    let mut flags = skill_flags | Flags::DOT;
    for (flag, key) in [
        (Flags::AREA, "dotIsArea"),
        (Flags::PROJECTILE, "dotIsProjectile"),
        (Flags::SPELL, "dotIsSpell"),
        (Flags::ATTACK, "dotIsAttack"),
        (Flags::HIT, "dotIsHit"),
    ] {
        if !data(key) {
            flags = flags.without(flag);
        }
    }
    flags
}

/// The weapon flags an attack's DoT config keeps (P6-11b review): PoB's
/// `dotCfg.flags` is `skillCfg.flags` with `Dot` (`CalcOffence.lua` 5649),
/// and `skillCfg.flags` holds `weapon1Flags or weapon2Flags`
/// (`CalcActiveSkill.lua` 455); its keyword flags keep `KeywordFlag.Bow`
/// (406). The main hand's weapon, else the off hand's; none for a skill
/// that is not an attack, or with no weapon (unarmed is not modelled).
/// PoB also strips `KeywordFlag.Hit` from a DoT that is a hit, which the
/// engine's one `HIT` flag (`ModFlag.Hit`) does not tell apart: a "Hits
/// and Ailments" line would reach such a DoT (no vetted skill has one).
fn attack_weapon_flags(input: &crate::input::CalcInput<'_>, skill: &MainSkill<'_>) -> Flags {
    if !skill.types.contains("Attack") {
        return Flags::NONE;
    }
    let hands = super::collect::hands(input);
    let Some(weapon) = hands
        .main
        .as_ref()
        .or(hands.off.as_ref())
        .and_then(|(_, w)| hit::weapon_type_flags(&w.class))
    else {
        return Flags::NONE;
    };
    let main_bow = hands.main.as_ref().is_some_and(|(_, w)| w.class == "Bow");
    if main_bow {
        weapon | Flags::KEYWORD_BOW
    } else {
        weapon
    }
}

/// The DoT phase (module docs). Sets [`Field::DotDps`] only when the main
/// skill has damage over time of its own.
///
/// # Errors
/// A graph error.
pub fn dot(calc: &mut Calc<'_>) -> Result<(), CalcError> {
    let input = *calc.input();
    let levels = GemLevels::from_effects(calc.store().effects());
    let Some((group, skill)) = dot_skill(input.build, input.data, &levels) else {
        return Ok(());
    };
    let source = Source::Gem {
        group,
        gem: skill.active_gem.gem,
    };
    if !computed_kind(&skill) {
        calc.set_unsupported(Field::DotDps, UnsupportedReason::NotComputed);
        return Ok(());
    }
    let pob = pob_skill::skill(&skill.effect().id);
    let mut reasons = Reasons::new();
    // Ground DoTs and stacking DoTs go to outputs this phase does not
    // compute (module docs).
    let ground = pob.is_some_and(|p| {
        p.base_flag("dotIsBurningGround")
            || p.base_flag("dotIsCausticGround")
            || p.base_flag("dotIsCorruptingBlood")
            || p.base_flag("DotCanStack")
            // PoB removes the skill's own increases from Herald of Ash's
            // DoT (`CalcOffence.lua` 5702): not modelled.
            || p.base_flag("dotIsHeraldOfAsh")
    });
    if ground {
        calc.set_unsupported(Field::DotDps, UnsupportedReason::NotComputed);
        return Ok(());
    }
    // A skill function of PoB's that changes the DoT and is not ported
    // ([`DOT_FUNCTIONS_NOT_PORTED`]): a reason (Phase 6 review).
    if DOT_FUNCTIONS_NOT_PORTED.contains(&skill.effect().id.as_str()) {
        reasons.partial.insert(UnsupportedReason::UnmodelledSkill {
            source: source.clone(),
            what: "skill-function",
        });
    }
    // PoB's hand-written data the DoT does not read (P6-6b, the skill
    // sweep): no PoB data for the skill (a variant PoB v2.67.2 lacks, whose
    // DoT PoB does not compute), or a base mod other than the display data
    // and the DoT's own keys and flags (Exsanguinate's stages, a tagged
    // mod): a reason.
    let read = |m: &pob_skill::PobBaseMod| {
        !m.tagged
            && match (m.name, m.key) {
                ("SkillData", Some(k)) => {
                    super::hit::DISPLAY_SKILL_DATA.contains(&k) || DOT_SKILL_DATA_KEYS.contains(&k)
                }
                (name, None) => m.kind == "FLAG" && DOT_FLAGS.contains(&name),
                _ => false,
            }
    };
    // A stat of its stat map read under a tag (P6-6b review: Conflagration's
    // DoT in part 2 only, Animosity's per stack): a reason too.
    if !pob.is_some_and(|p| p.base_mods.iter().all(read) && p.own_stats.is_empty()) {
        reasons.partial.insert(UnsupportedReason::UnmodelledSkill {
            source: source.clone(),
            what: "pob-skill-data",
        });
    }
    if skill.types.contains("SummonsTotem") {
        reasons.partial.insert(UnsupportedReason::UnmodelledSkill {
            source: source.clone(),
            what: super::hit::TOTEM_REASON,
        });
    }
    if skill.types.contains("Aura") {
        reasons.partial.insert(UnsupportedReason::UnmodelledSkill {
            source: source.clone(),
            what: "aura-dot",
        });
    }
    let sources = std::iter::once(&skill.active_gem)
        .chain(&skill.supports)
        .map(|r| Source::Gem { group, gem: r.gem })
        .collect();
    calc.set_skill_sources(sources);

    let gem = skill.active_gem;
    let instance = skill::instance(skill.effect(), gem.level, gem.quality, input.data);
    let mut deps: Vec<Field> = Vec::new();
    let mut bases: [Option<Valued>; 5] = [None; 5];
    if let Some((life_mult, es_mult)) = righteous_fire(&instance.stats) {
        // `preDamageFunc`: `FireDot = Life x RFLifeMultiplier + EnergyShield
        // x RFESMultiplier` when `LifeUnreserved > 1`.
        deps.extend([Field::Life, Field::EnergyShield, Field::LifeUnreserved]);
        let unreserved = calc.field(Field::LifeUnreserved).and_then(|f| f.value);
        let (Some(life), Some(es)) = (
            calc.field(Field::Life).and_then(|f| f.node.zip(f.value)),
            calc.field(Field::EnergyShield)
                .and_then(|f| f.node.zip(f.value)),
        ) else {
            calc.set_unsupported(
                Field::DotDps,
                UnsupportedReason::DependsOnNotComputed { field: Field::Life },
            );
            return Ok(());
        };
        if unreserved.is_some_and(|u| u > 1.0) {
            let life = Valued {
                node: life.0,
                value: life.1,
            };
            let es = Valued {
                node: es.0,
                value: es.1,
            };
            let lm = phases::constant(calc, "rf-life-multiplier", life_mult)?;
            let em = phases::constant(calc, "rf-es-multiplier", es_mult)?;
            let from_life = phases::product(calc, &[life, lm], Term::Step("rf-from-life"))?;
            let from_es = phases::product(calc, &[es, em], Term::Step("rf-from-es"))?;
            let base = phases::total(calc, &[from_life, from_es], Term::Base)?;
            if let Some(i) = hit::TYPES.iter().position(|t| *t == DamageType::Fire)
                && let Some(slot) = bases.get_mut(i)
            {
                *slot = Some(base);
            }
        }
    }
    if let Some((sacrificed, multiplier)) = vaal_righteous_fire(&instance.stats) {
        // `preDamageFunc`: `FireDot = (Life + EnergyShield) x
        // percentSacrificed x RFMultiplier`.
        deps.extend([Field::Life, Field::EnergyShield]);
        let (Some(life), Some(es)) = (
            calc.field(Field::Life).and_then(|f| f.node.zip(f.value)),
            calc.field(Field::EnergyShield)
                .and_then(|f| f.node.zip(f.value)),
        ) else {
            calc.set_unsupported(
                Field::DotDps,
                UnsupportedReason::DependsOnNotComputed { field: Field::Life },
            );
            return Ok(());
        };
        let life = Valued {
            node: life.0,
            value: life.1,
        };
        let es = Valued {
            node: es.0,
            value: es.1,
        };
        let pool = phases::total(calc, &[life, es], Term::Step("vaal-rf-pool"))?;
        let s = phases::constant(calc, "vaal-rf-sacrificed", sacrificed)?;
        let m = phases::constant(calc, "vaal-rf-multiplier", multiplier)?;
        let base = phases::product(calc, &[pool, s, m], Term::Base)?;
        if let Some(i) = hit::TYPES.iter().position(|t| *t == DamageType::Fire)
            && let Some(slot) = bases.get_mut(i)
        {
            *slot = Some(base);
        }
    }
    if let Some(values) = skill_dot_bases(&instance.stats) {
        for (i, v) in values.into_iter().enumerate() {
            if v > 0.0
                && let Some(slot) = bases.get_mut(i)
            {
                *slot = Some(hit::skill_leaf(calc, source.clone(), v)?);
            }
        }
    }
    let enemy = super::enemy::enemy(input.build, input.data);
    let flags = dot_flags(&skill, pob, &instance.stats) | attack_weapon_flags(&input, &skill);
    let mut totals = Vec::new();
    for (i, t) in hit::TYPES.into_iter().enumerate() {
        let Some(Some(base)) = bases.get(i).copied() else {
            continue;
        };
        let typed = flags | Flags::damage(t) | Flags::dot_keyword(t);
        let cx = calc.context(typed);
        // `canDeal`: "Cannot deal <Type> Damage" drops the type's DoT.
        if hit::cannot_deal(calc, t, &cx, &mut reasons)? {
            continue;
        }
        let total = type_dot(calc, &enemy, input.data, t, base, &cx, &mut reasons)?;
        totals.push(total);
    }
    let instance_total = phases::total(calc, &totals, Term::Step("dot-instance"))?;
    let cap = phases::constant(calc, "dot-dps-cap", DOT_DPS_CAP)?;
    let total = phases::pair(calc, instance_total, cap, Term::Result, true)?;
    calc.set_derived(Field::DotDps, phases::query(total, reasons), &deps);
    Ok(())
}

/// One type's damage over time (module docs, step 3).
fn type_dot(
    calc: &mut Calc<'_>,
    enemy: &super::enemy::Enemy,
    data: &dyn CalcData,
    t: DamageType,
    base: Valued,
    cx: &QueryContext,
    reasons: &mut Reasons,
) -> Result<Valued, CalcError> {
    let (inc, more) = hit::damage_modifiers(calc, cx, reasons)?;
    let mult = dot_multiplier(calc, cx, reasons)?;
    let effect = hit::dot_enemy_effect(calc, enemy, data, t, cx, reasons)?;
    phases::product(
        calc,
        &[base, inc, more, mult, effect],
        Term::Step("type-dot"),
    )
}

/// `1 + mult / 100`, `mult` the damage over time multipliers in `cx`
/// (`DotMultiplier` and `<Type>DotMultiplier` BASE; an override is not
/// modelled: no mapped stat sets one).
fn dot_multiplier(
    calc: &mut Calc<'_>,
    cx: &QueryContext,
    reasons: &mut Reasons,
) -> Result<Valued, CalcError> {
    let sum = hit::sum_of(
        calc,
        Quantity::DotMultiplier,
        Op::Base,
        cx,
        Term::Step("dot-multiplier"),
        reasons,
    )?;
    phases::factor(calc, sum, Term::Step("dot-multiplier-factor"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stats(rows: &[(&str, i64)]) -> Vec<(String, i64)> {
        rows.iter().map(|(s, v)| ((*s).to_owned(), *v)).collect()
    }

    #[test]
    fn bases_should_be_per_second_in_pobs_type_order() {
        let s = stats(&[("base_cold_damage_to_deal_per_minute", 600), ("other", 5)]);
        let b = skill_dot_bases(&s).expect("a DoT");
        assert_eq!(b, [0.0, 0.0, 10.0, 0.0, 0.0]);
        assert!(skill_dot_bases(&stats(&[("other", 5)])).is_none());
        assert!(skill_dot_bases(&stats(&[("base_fire_damage_to_deal_per_minute", 0)])).is_none());
    }

    #[test]
    fn righteous_fire_should_read_both_multipliers() {
        let s = stats(&[(RF_LIFE_STAT, 4200), (RF_ES_STAT, 4200)]);
        assert_eq!(righteous_fire(&s), Some((0.7, 0.7)));
        assert_eq!(righteous_fire(&stats(&[(RF_LIFE_STAT, 4200)])), None);
    }

    #[test]
    fn vaal_righteous_fire_should_multiply_its_two_shares() {
        let s = stats(&[(VAAL_RF_SACRIFICE_STAT, 60), (VAAL_RF_DAMAGE_STAT, 158)]);
        assert_eq!(vaal_righteous_fire(&s), Some((0.6, 1.58)));
        assert_eq!(
            vaal_righteous_fire(&stats(&[(VAAL_RF_DAMAGE_STAT, 158)])),
            None
        );
    }

    #[test]
    fn the_dot_cap_should_be_pobs() {
        assert_eq!(DOT_DPS_CAP, f64::from((i32::MAX) / 60));
    }
}
