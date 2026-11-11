//! The skill tables of bundle format 2 (Phase 3 plan, step 1a), from the
//! extract (`loom_data::bundle::skills` has the target types):
//! - `ActiveSkillType` to skill types (`FlagStat` by stat ID);
//! - `ActiveSkills` to active skills (types, minion types, weapon classes
//!   (PoE1 `WeaponRestriction_ItemClassesKeys`), `IsManuallyCasted`);
//! - `GrantedEffects` with `GrantedEffectsPerLevel` (levels sorted
//!   ascending; costs paired by position: PoE1 types per level, PoE2 per
//!   effect; reservations: PoE1 mana and life, flat and percent; PoE2
//!   `Reservation`, which PoB's PoE2 exporter reads as `SpiritReservation`,
//!   `PathOfBuilding-PoE2@49e93925d:src/Export/Scripts/skills.lua:249`) and
//!   `GrantedEffectQualityStats` (`dummy_stat_display_nothing` kept: it is
//!   the game's row);
//! - `GrantedEffectStatSets` with `GrantedEffectStatSetsPerLevel` (float
//!   stats with their value, resolved value, interpolation number, and
//!   `EffectivenessCostConstants.Multiplier`; integer stats with the
//!   interpolation numbers that follow the float stats' when the row has
//!   them);
//! - `SkillGems` (by base item ID), `GemEffects`, `GemTags`.
//!
//! Every value is copied as stored; the summary counts what was not
//! (references past a table's end, parallel lists of different lengths,
//! level lists put in order).

use std::collections::BTreeMap;

use loom_data::bundle::skills::{
    ActiveSkill, Cost, EffectLevel, FloatStat, GemEffect, GemTag, GrantedEffect, LevelStat,
    QualityStat, Reservation, Resource, SkillGem, SkillTables, SkillType, StatSet, StatSetLevel,
};
use serde::Serialize;
use serde_json::Value;

use super::{Counts, at, column, text_column};
use crate::extract::{Read, Table};
use crate::mod_filter::ints;

/// What the skill conversion did not copy one to one.
#[derive(Debug, Default, Serialize)]
pub struct SkillSummary {
    /// Granted effects whose level rows were not in ascending order
    /// (sorted; equal levels keep table order and fail the validator).
    pub effect_levels_sorted: usize,
    /// Stat sets whose level rows were not in ascending order (likewise).
    pub stat_set_levels_sorted: usize,
    /// Stat sets without level rows.
    pub stat_sets_without_levels: usize,
    /// Rows whose parallel lists (stats and values, cost types and
    /// amounts) differ in length; the shorter length is used.
    pub parallel_lists_unequal: usize,
    /// Granted effects with more than one quality row (their stats are
    /// joined in table order).
    pub effects_with_several_quality_rows: usize,
    /// Level or quality rows whose owner row is past the table's end
    /// (dropped).
    pub rows_without_owner: usize,
}

fn bump(cell: &mut usize) {
    *cell += 1;
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "the extract wrote an f32 as its shortest text; it reads back to the same f32"
)]
fn to_f32(v: f64) -> f32 {
    v as f32
}

/// A float cell; a non-finite or missing value is an error (the extract
/// writes non-finite floats as strings).
fn float(t: &Table, row: &[Value], col: usize) -> Result<f32, String> {
    row.get(col)
        .and_then(Value::as_f64)
        .map(to_f32)
        .ok_or_else(|| format!("{}: a float cell is not a finite number", t.table))
}

fn floats(t: &Table, row: &[Value], col: usize) -> Result<Vec<f32>, String> {
    let Some(items) = row.get(col).and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    items
        .iter()
        .map(|v| {
            v.as_f64()
                .map(to_f32)
                .ok_or_else(|| format!("{}: a float list holds a non-number", t.table))
        })
        .collect()
}

fn flag(row: &[Value], col: Option<usize>) -> bool {
    col.and_then(|c| row.get(c))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn int_or_0(row: &[Value], col: Option<usize>) -> i64 {
    col.and_then(|c| Table::int(row, c)).unwrap_or(0)
}

/// IDs of the rows a list cell names.
fn ids(list: &[String], row: &[Value], col: Option<usize>, counts: &Counts) -> Vec<String> {
    col.map(|c| {
        ints(row, c)
            .into_iter()
            .filter_map(|n| at(list, Some(n), counts))
            .collect()
    })
    .unwrap_or_default()
}

/// Rows of `t` grouped by the owner row in column `owner`, in table order.
fn by_owner<'t>(
    t: &'t Table,
    owner: usize,
    owners: usize,
    summary: &mut SkillSummary,
) -> BTreeMap<usize, Vec<&'t [Value]>> {
    let mut out: BTreeMap<usize, Vec<&[Value]>> = BTreeMap::new();
    for r in &t.rows {
        match Table::int(r, owner).and_then(|n| usize::try_from(n).ok()) {
            Some(n) if n < owners => out.entry(n).or_default().push(r),
            _ => bump(&mut summary.rows_without_owner),
        }
    }
    out
}

/// Sorts by level (stable); `true` when the order changed.
fn sort_levels<T>(levels: &mut [T], level: impl Fn(&T) -> i64) -> bool {
    let sorted = levels
        .iter()
        .zip(levels.iter().skip(1))
        .all(|(a, b)| level(a) <= level(b));
    if !sorted {
        levels.sort_by_key(level);
    }
    !sorted
}

/// Pairs two parallel lists, counting a length difference.
fn pair<A, B>(a: Vec<A>, b: Vec<B>, summary: &mut SkillSummary) -> Vec<(A, B)> {
    if a.len() != b.len() {
        bump(&mut summary.parallel_lists_unequal);
    }
    a.into_iter().zip(b).collect()
}

struct Lists {
    stats: Vec<String>,
    types: Vec<String>,
    classes: Vec<String>,
    actives: Vec<String>,
    effects: Vec<String>,
    sets: Vec<String>,
    bases: Vec<String>,
    gem_effects: Vec<String>,
    tags: Vec<String>,
    cost_types: Vec<String>,
}

/// Converts the skill tables (module docs).
///
/// # Errors
/// A message when a table or column is missing, or a float is not finite.
pub fn convert(
    read: Read<'_>,
    classes: &[String],
    counts: &Counts,
) -> Result<(SkillTables, SkillSummary), String> {
    let lists = Lists {
        stats: text_column(read, "Stats", "Id")?,
        types: text_column(read, "ActiveSkillType", "Id")?,
        classes: classes.to_vec(),
        actives: text_column(read, "ActiveSkills", "Id")?,
        effects: text_column(read, "GrantedEffects", "Id")?,
        sets: text_column(read, "GrantedEffectStatSets", "Id")?,
        bases: text_column(read, "BaseItemTypes", "Id")?,
        gem_effects: text_column(read, "GemEffects", "Id")?,
        tags: text_column(read, "GemTags", "Id")?,
        cost_types: text_column(read, "CostTypes", "Id")?,
    };
    let mut summary = SkillSummary::default();
    let tables = SkillTables {
        skill_types: skill_types(read, &lists, counts)?,
        active_skills: active_skills(read, &lists, counts)?,
        granted_effects: granted_effects(read, &lists, counts, &mut summary)?,
        stat_sets: stat_sets(read, &lists, counts, &mut summary)?,
        gems: gems(read, &lists, counts)?,
        gem_effects: gem_effects(read, &lists, counts)?,
        gem_tags: gem_tags(read)?,
    };
    Ok((tables, summary))
}

fn skill_types(read: Read<'_>, l: &Lists, counts: &Counts) -> Result<Vec<SkillType>, String> {
    let t = Table::require(read, "ActiveSkillType")?;
    let (id, stat) = (t.column("Id")?, t.column("FlagStat")?);
    Ok(t.rows
        .iter()
        .map(|r| SkillType {
            id: Table::text(r, id).unwrap_or("").to_owned(),
            flag_stat: at(&l.stats, Table::int(r, stat), counts),
        })
        .collect())
}

fn active_skills(read: Read<'_>, l: &Lists, counts: &Counts) -> Result<Vec<ActiveSkill>, String> {
    let t = Table::require(read, "ActiveSkills")?;
    let (id, name, types, minion) = (
        t.column("Id")?,
        t.column("DisplayedName")?,
        t.column("ActiveSkillTypes")?,
        t.column("MinionActiveSkillTypes")?,
    );
    let (weapons, manual) = (
        t.column("WeaponRestriction_ItemClassesKeys").ok(),
        t.column("IsManuallyCasted").ok(),
    );
    Ok(t.rows
        .iter()
        .map(|r| ActiveSkill {
            id: Table::text(r, id).unwrap_or("").to_owned(),
            name: Table::text(r, name).unwrap_or("").to_owned(),
            types: ids(&l.types, r, Some(types), counts),
            minion_types: ids(&l.types, r, Some(minion), counts),
            weapon_classes: ids(&l.classes, r, weapons, counts),
            manually_cast: flag(r, manual),
        })
        .collect())
}

/// The level rows of each granted effect.
fn effect_levels(
    read: Read<'_>,
    l: &Lists,
    effect_costs: &[Vec<String>],
    counts: &Counts,
    summary: &mut SkillSummary,
) -> Result<BTreeMap<usize, Vec<EffectLevel>>, String> {
    let t = Table::require(read, "GrantedEffectsPerLevel")?;
    let owner = t.column("GrantedEffect")?;
    let level = t.column("Level")?;
    let c = |n: &str| t.column(n).ok();
    let (cost_types, amounts) = (c("CostTypes"), t.column("CostAmounts")?);
    let reservation_columns = [
        (c("ManaReservationFlat"), Resource::Mana, false),
        (c("ManaReservationPercent"), Resource::Mana, true),
        (c("LifeReservationFlat"), Resource::Life, false),
        (c("LifeReservationPercent"), Resource::Life, true),
        (c("Reservation"), Resource::Spirit, false),
    ];
    let mut out = BTreeMap::new();
    for (n, rows) in by_owner(&t, owner, l.effects.len(), summary) {
        let mut levels = Vec::with_capacity(rows.len());
        for r in rows {
            // PoE1 names the cost types per level, PoE2 per effect.
            let types = match cost_types {
                Some(col) => ids(&l.cost_types, r, Some(col), counts),
                None => effect_costs.get(n).cloned().unwrap_or_default(),
            };
            let costs = pair(types, ints(r, amounts), summary)
                .into_iter()
                .map(|(resource, amount)| Cost { resource, amount })
                .collect();
            let reservations = reservation_columns
                .iter()
                .filter_map(|(col, resource, percent)| {
                    let amount = int_or_0(r, *col);
                    (amount != 0).then_some(Reservation {
                        resource: *resource,
                        percent: *percent,
                        amount,
                    })
                })
                .collect();
            levels.push(EffectLevel {
                level: Table::int(r, level).unwrap_or(0),
                costs,
                cost_multiplier: int_or_0(r, c("CostMultiplier")),
                cooldown: int_or_0(r, c("Cooldown")),
                stored_uses: int_or_0(r, c("StoredUses")),
                attack_speed_multiplier: int_or_0(r, c("AttackSpeedMultiplier")),
                attack_time: int_or_0(r, c("AttackTime")),
                reservations,
            });
        }
        if sort_levels(&mut levels, |x| x.level) {
            bump(&mut summary.effect_levels_sorted);
        }
        out.insert(n, levels);
    }
    Ok(out)
}

/// The quality stats of each granted effect.
fn quality(
    read: Read<'_>,
    l: &Lists,
    counts: &Counts,
    summary: &mut SkillSummary,
) -> Result<BTreeMap<usize, Vec<QualityStat>>, String> {
    let t = Table::require(read, "GrantedEffectQualityStats")?;
    let owner = column(&t, &["GrantedEffectsKey", "GrantedEffect"])?;
    let stats = column(&t, &["StatsKeys", "Stats"])?;
    let values = t.column("StatsValuesPermille")?;
    let mut out = BTreeMap::new();
    for (n, rows) in by_owner(&t, owner, l.effects.len(), summary) {
        if rows.len() > 1 {
            bump(&mut summary.effects_with_several_quality_rows);
        }
        let mut list = Vec::new();
        for r in rows {
            let ids = ints(r, stats)
                .into_iter()
                .map(|k| at(&l.stats, Some(k), counts))
                .collect();
            for (stat, per_mille) in pair(ids, ints(r, values), summary) {
                if let Some(stat) = stat {
                    list.push(QualityStat { stat, per_mille });
                }
            }
        }
        out.insert(n, list);
    }
    Ok(out)
}

fn granted_effects(
    read: Read<'_>,
    l: &Lists,
    counts: &Counts,
    summary: &mut SkillSummary,
) -> Result<Vec<GrantedEffect>, String> {
    let t = Table::require(read, "GrantedEffects")?;
    let col = |n: &str| t.column(n);
    let id = col("Id")?;
    let (support, allowed, added, excluded) = (
        col("IsSupport")?,
        col("AllowedActiveSkillTypes")?,
        col("AddedActiveSkillTypes")?,
        col("ExcludedActiveSkillTypes")?,
    );
    let (gems_only, unsupported, cast, active, minion) = (
        col("SupportsGemsOnly")?,
        col("CannotBeSupported")?,
        col("CastTime")?,
        col("ActiveSkill")?,
        col("AddedMinionActiveSkillTypes")?,
    );
    let main_set = column(&t, &["StatSet1", "StatSet"])?;
    let more_sets = column(&t, &["StatSet2", "AdditionalStatSets"])?;
    let effect_costs: Vec<Vec<String>> = match t.column("CostTypes").ok() {
        Some(c) => t
            .rows
            .iter()
            .map(|r| ids(&l.cost_types, r, Some(c), counts))
            .collect(),
        None => Vec::new(),
    };
    let mut levels = effect_levels(read, l, &effect_costs, counts, summary)?;
    let mut quality = quality(read, l, counts, summary)?;
    Ok(t.rows
        .iter()
        .enumerate()
        .map(|(n, r)| {
            let mut stat_sets: Vec<String> = at(&l.sets, Table::int(r, main_set), counts)
                .into_iter()
                .collect();
            stat_sets.extend(ids(&l.sets, r, Some(more_sets), counts));
            GrantedEffect {
                id: Table::text(r, id).unwrap_or("").to_owned(),
                support: flag(r, Some(support)),
                active_skill: at(&l.actives, Table::int(r, active), counts),
                cast_time: Table::int(r, cast).unwrap_or(0),
                allowed_types: ids(&l.types, r, Some(allowed), counts),
                added_types: ids(&l.types, r, Some(added), counts),
                excluded_types: ids(&l.types, r, Some(excluded), counts),
                added_minion_types: ids(&l.types, r, Some(minion), counts),
                supports_gems_only: flag(r, Some(gems_only)),
                cannot_be_supported: flag(r, Some(unsupported)),
                stat_sets,
                levels: levels.remove(&n).unwrap_or_default(),
                quality: quality.remove(&n).unwrap_or_default(),
            }
        })
        .collect())
}

/// One `GrantedEffectStatSetsPerLevel` row.
fn stat_set_level(
    t: &Table,
    r: &[Value],
    l: &Lists,
    interpolation_bases: &[f32],
    counts: &Counts,
    summary: &mut SkillSummary,
) -> Result<StatSetLevel, String> {
    let c = |n: &str| t.column(n);
    let float_ids: Vec<Option<String>> = ints(r, c("FloatStats")?)
        .into_iter()
        .map(|k| at(&l.stats, Some(k), counts))
        .collect();
    let values = floats(t, r, c("FloatStatsValues")?)?;
    let resolved = ints(r, c("BaseResolvedValues")?);
    let bases: Vec<f32> = ints(r, c("InterpolationBases")?)
        .into_iter()
        .map(|k| {
            usize::try_from(k)
                .ok()
                .and_then(|k| interpolation_bases.get(k))
                .copied()
                .unwrap_or_else(|| {
                    counts.dangling.set(counts.dangling.get() + 1);
                    f32::NAN
                })
        })
        .collect();
    let interpolations = ints(r, c("StatInterpolations")?);
    let n_float = float_ids.len();
    if [values.len(), resolved.len(), bases.len()]
        .iter()
        .any(|n| *n != n_float)
        || interpolations.len() < n_float
    {
        bump(&mut summary.parallel_lists_unequal);
    }
    let mut float_stats = Vec::new();
    for (i, stat) in float_ids.into_iter().enumerate() {
        let (Some(stat), Some(value), Some(resolved), Some(base), Some(interp)) = (
            stat,
            values.get(i),
            resolved.get(i),
            bases.get(i),
            interpolations.get(i),
        ) else {
            continue;
        };
        if base.is_nan() {
            continue;
        }
        float_stats.push(FloatStat {
            stat,
            value: *value,
            resolved: *resolved,
            interpolation: *interp,
            interpolation_base: *base,
        });
    }
    let extra_interpolations = interpolations.get(n_float..).unwrap_or_default();
    let stat_ids: Vec<Option<String>> = ints(r, c("AdditionalStats")?)
        .into_iter()
        .map(|k| at(&l.stats, Some(k), counts))
        .collect();
    let stat_values = ints(r, c("AdditionalStatsValues")?);
    if !extra_interpolations.is_empty() && extra_interpolations.len() != stat_ids.len() {
        bump(&mut summary.parallel_lists_unequal);
    }
    let stats = pair(stat_ids, stat_values, summary)
        .into_iter()
        .enumerate()
        .filter_map(|(i, (stat, value))| {
            Some(LevelStat {
                stat: stat?,
                value,
                interpolation: extra_interpolations.get(i).copied(),
            })
        })
        .collect();
    Ok(StatSetLevel {
        level: Table::int(r, c("GemLevel")?).unwrap_or(0),
        spell_crit_chance: Table::int(r, c("SpellCritChance")?).unwrap_or(0),
        attack_crit_chance: Table::int(r, c("AttackCritChance")?).unwrap_or(0),
        base_multiplier: Table::int(r, c("BaseMultiplier")?).unwrap_or(0),
        damage_effectiveness: c("DamageEffectiveness")
            .ok()
            .and_then(|col| Table::int(r, col)),
        float_stats,
        stats,
        flags: ids(&l.stats, r, Some(c("AdditionalFlags")?), counts),
        granted_effects: ids(&l.effects, r, Some(c("GrantedEffects")?), counts),
    })
}

fn stat_sets(
    read: Read<'_>,
    l: &Lists,
    counts: &Counts,
    summary: &mut SkillSummary,
) -> Result<Vec<StatSet>, String> {
    let bases_table = Table::require(read, "EffectivenessCostConstants")?;
    let multiplier = bases_table.column("Multiplier")?;
    let interpolation_bases = bases_table
        .rows
        .iter()
        .map(|r| float(&bases_table, r, multiplier))
        .collect::<Result<Vec<f32>, String>>()?;
    let per_level = Table::require(read, "GrantedEffectStatSetsPerLevel")?;
    let owner = per_level.column("StatSet")?;
    let mut levels: BTreeMap<usize, Vec<StatSetLevel>> = BTreeMap::new();
    for (n, rows) in by_owner(&per_level, owner, l.sets.len(), summary) {
        let mut list = Vec::with_capacity(rows.len());
        for r in rows {
            list.push(stat_set_level(
                &per_level,
                r,
                l,
                &interpolation_bases,
                counts,
                summary,
            )?);
        }
        if sort_levels(&mut list, |x| x.level) {
            bump(&mut summary.stat_set_levels_sorted);
        }
        levels.insert(n, list);
    }
    let t = Table::require(read, "GrantedEffectStatSets")?;
    let col = |n: &str| t.column(n);
    let (id, implicit, constant, constant_values) = (
        col("Id")?,
        col("ImplicitStats")?,
        col("ConstantStats")?,
        col("ConstantStatsValues")?,
    );
    let (base, incremental) = (col("BaseEffectiveness")?, col("IncrementalEffectiveness")?);
    let damage_incremental = col("DamageIncrementalEffectiveness").ok();
    let mut out = Vec::with_capacity(t.rows.len());
    for (n, r) in t.rows.iter().enumerate() {
        let constant_ids = ints(r, constant)
            .into_iter()
            .map(|k| at(&l.stats, Some(k), counts))
            .collect();
        let constant_stats = pair(constant_ids, ints(r, constant_values), summary)
            .into_iter()
            .filter_map(|(s, v)| Some((s?, v)))
            .collect();
        let levels = levels.remove(&n).unwrap_or_default();
        if levels.is_empty() {
            bump(&mut summary.stat_sets_without_levels);
        }
        out.push(StatSet {
            id: Table::text(r, id).unwrap_or("").to_owned(),
            implicit_stats: ids(&l.stats, r, Some(implicit), counts),
            constant_stats,
            base_effectiveness: float(&t, r, base)?,
            incremental_effectiveness: float(&t, r, incremental)?,
            damage_incremental_effectiveness: damage_incremental
                .map(|c| float(&t, r, c))
                .transpose()?,
            levels,
        });
    }
    Ok(out)
}

fn gems(read: Read<'_>, l: &Lists, counts: &Counts) -> Result<Vec<SkillGem>, String> {
    let t = Table::require(read, "SkillGems")?;
    let base = column(&t, &["BaseItemTypesKey", "BaseItemType"])?;
    let effects = column(&t, &["GemVariants", "GemEffects"])?;
    let c = |n: &str| t.column(n).ok();
    Ok(t.rows
        .iter()
        .filter_map(|r| {
            Some(SkillGem {
                base_item: at(&l.bases, Table::int(r, base), counts)?,
                strength: int_or_0(r, c("StrengthRequirementPercent")),
                dexterity: int_or_0(r, c("DexterityRequirementPercent")),
                intelligence: int_or_0(r, c("IntelligenceRequirementPercent")),
                vaal: flag(r, c("IsVaalVariant")),
                effects: ids(&l.gem_effects, r, Some(effects), counts),
            })
        })
        .collect())
}

fn gem_effects(read: Read<'_>, l: &Lists, counts: &Counts) -> Result<Vec<GemEffect>, String> {
    let t = Table::require(read, "GemEffects")?;
    let (id, name, main, tags) = (
        t.column("Id")?,
        t.column("Name")?,
        t.column("GrantedEffect")?,
        t.column("GemTags")?,
    );
    let (second, more) = (
        t.column("GrantedEffect2").ok(),
        t.column("AdditionalGrantedEffects").ok(),
    );
    Ok(t.rows
        .iter()
        .map(|r| {
            let mut additional: Vec<String> = second
                .and_then(|c| at(&l.effects, Table::int(r, c), counts))
                .into_iter()
                .collect();
            additional.extend(ids(&l.effects, r, more, counts));
            GemEffect {
                id: Table::text(r, id).unwrap_or("").to_owned(),
                name: Table::text(r, name).unwrap_or("").to_owned(),
                granted_effect: at(&l.effects, Table::int(r, main), counts),
                additional_granted_effects: additional,
                tags: ids(&l.tags, r, Some(tags), counts),
            }
        })
        .collect())
}

fn gem_tags(read: Read<'_>) -> Result<Vec<GemTag>, String> {
    let t = Table::require(read, "GemTags")?;
    let id = t.column("Id")?;
    let name = column(&t, &["Tag", "Name"])?;
    Ok(t.rows
        .iter()
        .map(|r| GemTag {
            id: Table::text(r, id).unwrap_or("").to_owned(),
            name: Table::text(r, name).unwrap_or("").to_owned(),
        })
        .collect())
}

#[cfg(test)]
mod tests;
