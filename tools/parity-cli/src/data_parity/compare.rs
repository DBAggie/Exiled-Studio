//! The comparison: every value of the PoB export with the bundle's value
//! for the same row, in PoB's form. Each bundle value is put into PoB's
//! form by the conversion PoB's exporter made (cited at each one, PoB
//! v2.67.2), then both are compared exactly. [`TRANSFORMS`] lists them for
//! the report.
//!
//! Numbers: PoB's data files hold Lua's `tostring` of each number (14
//! significant digits), so a bundle number is compared at 14 significant
//! digits ([`lua`]); `f32` values are widened first, as PoB's exporter
//! widened them. An absent value and 0 are the same where PoB writes a
//! field only when it is not 0 (named at each comparison).

use std::collections::{BTreeMap, BTreeSet};

use loom_calc::poe1::enemy::Boss;
use loom_data::bundle::Bundle;
use loom_data::bundle::passives::{Passive, PassiveTables, StatAmount};
use loom_data::bundle::properties::{MonsterLevel, MonsterType, PropertyTables};
use loom_data::bundle::skills::{Resource, SkillTables, StatSet};
use loom_data::stat_desc::{NotRenderedReason, StatValue, VariantFilter};
use serde::Serialize;
use serde_json::{Value, json};

use super::pob::{self, Export};

/// The stat-description scope of passive and mastery text (the file GGG's
/// tree export renders node text with), by name ([`Bundle::scope_root`]).
pub const PASSIVE_SCOPE: &str = "passive_skill_stat_descriptions";

/// Templates whose stats PoB exports as `characterConstants` and
/// `monsterConstants` (`v2.67.2:src/Export/Scripts/miscdata.lua:54`, `:75`).
const CHARACTER_TEMPLATE: &str = "metadata/characters/character";
const MONSTER_TEMPLATE: &str = "metadata/monsters/monster";

/// The quality stat PoB's exporter leaves out
/// (`v2.67.2:src/Export/Scripts/skills.lua:431`).
const DUMMY_QUALITY_STAT: &str = "dummy_stat_display_nothing";

/// A `monster_levels` column as a number.
type MonsterColumn = fn(&MonsterLevel) -> Option<f64>;

/// PoB's monster tables, the bundle's `monster_levels` field for each, and
/// how to read it (`v2.67.2:src/Export/Scripts/miscdata.lua`).
#[expect(clippy::cast_precision_loss, reason = "game integers")]
pub const MONSTER_TABLES: [(&str, &str, MonsterColumn); 11] = [
    ("monsterEvasionTable", "evasion", |r| Some(r.evasion as f64)),
    ("monsterAccuracyTable", "accuracy", |r| {
        Some(r.accuracy as f64)
    }),
    ("monsterLifeTable", "life", |r| Some(r.life as f64)),
    ("monsterLifeTable2", "alt_life_1", |r| {
        r.alt_life_1.map(|v| v as f64)
    }),
    ("monsterLifeTable3", "alt_life_2", |r| {
        Some(r.alt_life_2 as f64)
    }),
    ("monsterAllyLifeTable", "ally_life", |r| {
        Some(r.ally_life as f64)
    }),
    ("monsterDamageTable", "damage", |r| {
        Some(f64::from(r.damage))
    }),
    ("monsterAllyDamageTable", "ally_damage", |r| {
        Some(f64::from(r.ally_damage))
    }),
    ("monsterArmourTable", "armour", |r| Some(r.armour as f64)),
    ("monsterAilmentThresholdTable", "ailment_threshold", |r| {
        Some(r.ailment_threshold as f64)
    }),
    (
        "monsterPhysConversionMultiTable",
        "phys_conversion_multi",
        |r| r.phys_conversion_multi.map(|v| v as f64),
    ),
];

/// A conversion from the bundle's form to PoB's, for the report.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Transform {
    /// The table.
    pub table: &'static str,
    /// The fields (patterns as in the rules).
    pub fields: &'static str,
    /// What is done to the bundle value.
    pub transform: &'static str,
    /// PoB's line that did it.
    pub source: &'static str,
}

/// Every conversion the comparison makes.
pub const TRANSFORMS: [Transform; 17] = [
    Transform {
        table: "*",
        fields: "*",
        transform: "numbers at 14 significant digits (Lua's tostring, which wrote PoB's data files); f32 widened to f64 first",
        source: "v2.67.2:src/Export/Main.lua (tables written with tostring)",
    },
    Transform {
        table: "skills",
        fields: "cast_time",
        transform: "milliseconds / 1000",
        source: "v2.67.2:src/Export/Scripts/skills.lua:251",
    },
    Transform {
        table: "skills",
        fields: "level.*.cooldown",
        transform: "milliseconds / 1000; absent when 0",
        source: "v2.67.2:src/Export/Scripts/skills.lua:292-293",
    },
    Transform {
        table: "skills",
        fields: "level.*.damage_effectiveness, level.*.base_multiplier",
        transform: "raw / 10000 + 1; absent when 0",
        source: "v2.67.2:src/Export/Scripts/skills.lua:311-312, 320-321",
    },
    Transform {
        table: "skills",
        fields: "level.*.crit_chance",
        transform: "hundredths / 100; the bundle's attack_crit_chance (PoB's spec: OffhandCritChance) when not 0, else spell_crit_chance (PoB's spec: AttackCritChance); absent when both are 0",
        source: "v2.67.2:src/Export/Scripts/skills.lua:314-319; v2.67.2:src/Export/spec.lua:6166, 6173",
    },
    Transform {
        table: "skills",
        fields: "level.*.mana_multiplier",
        transform: "cost multiplier - 100; absent when 0",
        source: "v2.67.2:src/Export/Scripts/skills.lua:283-284",
    },
    Transform {
        table: "skills",
        fields: "level.*.*_reservation_percent",
        transform: "hundredths of a percent / 100",
        source: "v2.67.2:src/Export/Scripts/skills.lua:274-282",
    },
    Transform {
        table: "skills",
        fields: "level.*.stat.*",
        transform: "float stats: value / max(interpolation base, 0.00001) in f64 of the f32 columns; a stat absent on one side is 0 (PoB fills missing stats with 0)",
        source: "v2.67.2:src/Export/Scripts/skills.lua:342, 354",
    },
    Transform {
        table: "skills",
        fields: "quality_stats",
        transform: "per mille / 1000, without dummy_stat_display_nothing",
        source: "v2.67.2:src/Export/Scripts/skills.lua:431-432",
    },
    Transform {
        table: "skills",
        fields: "base_effectiveness, incremental_effectiveness",
        transform: "absent is 1 and 0",
        source: "v2.67.2:src/Export/Scripts/skills.lua:158-163",
    },
    Transform {
        table: "skills",
        fields: "types",
        transform: "sorted set of skill type IDs against PoB's SkillType names (numbered by ActiveSkillType row)",
        source: "v2.67.2:src/Export/Scripts/skills.lua:1-4, 11-13; v2.67.2:src/Data/Global.lua:217",
    },
    Transform {
        table: "bases",
        fields: "*",
        transform: "matched by name: PoB's display name (o for \u{f6}, trimmed), Royale bases skipped",
        source: "v2.67.2:src/Export/Scripts/bases.lua:116-117, 342",
    },
    Transform {
        table: "bases",
        fields: "crit_chance, attack_rate",
        transform: "crit hundredths / 100; attack rate round(1000 / attack time, 2)",
        source: "v2.67.2:src/Export/Scripts/bases.lua:209-210",
    },
    Transform {
        table: "bases",
        fields: "movement_penalty",
        transform: "negated movement speed; absent when 0",
        source: "v2.67.2:src/Export/Scripts/bases.lua:237-238",
    },
    Transform {
        table: "game_constants",
        fields: "value",
        transform: "value / divisor",
        source: "v2.67.2:src/Export/Scripts/miscdata.lua:47-52",
    },
    Transform {
        table: "passives, mastery_effects",
        fields: "stats",
        transform: "stats rendered with the bundle's passive_skill_stat_descriptions scope (a stat without a description shows no line, as in game), lines sorted; PoB's lines split at newlines, sorted, without the 'Grants N Passive Skill Point(s)' line (skill_points is compared against grantedPassivePoints)",
        source: "game:passive_skill_stat_descriptions.txt (the tree file's text is GGG's tree export)",
    },
    Transform {
        table: "passives",
        fields: "mastery",
        transform: "PoB's isMastery against the bundle passive's mastery group",
        source: "v2.67.2:src/TreeData/3_29/tree.lua (isMastery)",
    },
];

/// One difference: the same row's field, in PoB's form, on both sides.
/// `null` means the row or field is absent on that side.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Difference {
    /// The table.
    pub table: &'static str,
    /// The row key.
    pub key: String,
    /// The field.
    pub field: String,
    /// The bundle's value in PoB's form.
    pub bundle: Value,
    /// PoB's value.
    pub pob: Value,
}

/// Counts of one table.
#[derive(Debug, Clone, Default, Serialize)]
pub struct TableCounts {
    /// Values compared.
    pub compared: usize,
    /// Values that differ.
    pub differences: usize,
    /// PoB rows the bundle does not have (each also a difference).
    pub pob_only_rows: usize,
    /// Bundle rows, levels, and additional stat sets PoB does not have
    /// (not differences: no PoB build reads them; listed in the report).
    pub bundle_only_rows: usize,
}

/// A bundle row, level, or stat set PoB does not have.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BundleOnly {
    /// The table.
    pub table: &'static str,
    /// The row key; `<key>:level.<n>` for a level, `<key>:stat_set.<n>` for
    /// an additional stat set (PoB exports the main one only).
    pub key: String,
}

/// The result of a comparison.
#[derive(Debug, Default)]
pub struct Comparison {
    /// Counts by table.
    pub tables: BTreeMap<&'static str, TableCounts>,
    /// Every difference, in comparison order.
    pub differences: Vec<Difference>,
    /// Every bundle row, level, and stat set PoB does not have.
    pub bundle_only: Vec<BundleOnly>,
}

impl Comparison {
    fn table(&mut self, table: &'static str) -> &mut TableCounts {
        self.tables.entry(table).or_default()
    }

    /// Compares one value.
    pub(super) fn check(
        &mut self,
        table: &'static str,
        key: &str,
        field: impl Into<String>,
        bundle: Value,
        pob: Value,
    ) {
        let counts = self.table(table);
        counts.compared += 1;
        if bundle != pob {
            counts.differences += 1;
            self.differences.push(Difference {
                table,
                key: key.to_owned(),
                field: field.into(),
                bundle,
                pob,
            });
        }
    }

    /// A PoB row the bundle does not have.
    fn pob_only(&mut self, table: &'static str, key: &str) {
        let counts = self.table(table);
        counts.compared += 1;
        counts.differences += 1;
        counts.pob_only_rows += 1;
        self.differences.push(Difference {
            table,
            key: key.to_owned(),
            field: "row".to_owned(),
            bundle: Value::Null,
            pob: json!("present"),
        });
    }

    fn bundle_only(&mut self, table: &'static str, keys: impl IntoIterator<Item = String>) {
        let before = self.bundle_only.len();
        self.bundle_only
            .extend(keys.into_iter().map(|key| BundleOnly { table, key }));
        let added = self.bundle_only.len() - before;
        self.table(table).bundle_only_rows += added;
    }
}

/// A number at 14 significant digits, as Lua's `tostring` writes it.
pub fn lua(x: f64) -> f64 {
    if !x.is_finite() || x == 0.0 {
        return x;
    }
    format!("{x:.13e}").parse().unwrap_or(x)
}

/// A number as JSON, integral values as integers, so `5` and `5.0` are one
/// value.
pub fn num(x: f64) -> Value {
    const EXACT: f64 = 9_007_199_254_740_992.0; // 2^53
    if x.fract() == 0.0 && x.abs() < EXACT {
        // Integral and within the exact range: the cast is exact.
        #[expect(clippy::cast_possible_truncation, reason = "integral below 2^53")]
        return json!(x as i64);
    }
    // NaN or infinity (a zero divisor): a value of its own, never equal to
    // an absent one.
    serde_json::Number::from_f64(x).map_or_else(|| json!(format!("non-finite {x}")), Value::Number)
}

/// A bundle number in PoB's form.
fn ours(x: f64) -> Value {
    num(lua(x))
}

/// An integer in PoB's form (exact below 2^53, which game data never
/// reaches).
#[expect(
    clippy::cast_precision_loss,
    reason = "game integers are far below 2^53"
)]
fn int(x: i64) -> Value {
    num(x as f64)
}

fn opt(x: Option<f64>) -> Value {
    x.map_or(Value::Null, num)
}

/// `Some(x)` when not 0: PoB writes these fields only when not 0.
#[expect(
    clippy::cast_precision_loss,
    reason = "game integers are far below 2^53"
)]
fn nonzero(x: i64) -> Option<f64> {
    (x != 0).then_some(x as f64)
}

fn strings<'a>(items: impl IntoIterator<Item = &'a str>) -> Value {
    Value::Array(items.into_iter().map(|s| json!(s)).collect())
}

fn sorted_set<'a>(items: impl IntoIterator<Item = &'a str>) -> Value {
    strings(items.into_iter().collect::<BTreeSet<_>>())
}

/// Compares the bundle with the export.
///
/// # Errors
/// A message when the bundle lacks the calc tables (format 1), or the
/// passive stat-description scope `passive_scope` (a scope name, normally
/// [`PASSIVE_SCOPE`]) does not load.
pub fn compare(
    bundle: &Bundle,
    export: &Export,
    passive_scope: &str,
    bosses: &[Boss],
) -> Result<Comparison, String> {
    let (Some(skills), Some(passives), Some(properties)) =
        (&bundle.skills, &bundle.passives, &bundle.properties)
    else {
        return Err("the bundle has no calc tables (format 2 is needed)".to_owned());
    };
    let mut out = Comparison::default();
    compare_skills(&mut out, skills, export);
    compare_gems(&mut out, skills, export);
    compare_bases(&mut out, bundle, properties, export);
    compare_characters(&mut out, passives, export);
    compare_constants(&mut out, passives, properties, export);
    compare_monster_levels(&mut out, properties, export);
    compare_bosses(&mut out, properties, export, bosses);
    compare_pantheons(&mut out, bundle, export)?;
    let effects: std::collections::BTreeSet<&str> = skills
        .granted_effects
        .iter()
        .map(|e| e.id.as_str())
        .collect();
    super::gem_effects::compare_gem_effects(&mut out, export, &effects);
    compare_passives(&mut out, bundle, passives, export, passive_scope)?;
    Ok(out)
}

fn compare_skills(out: &mut Comparison, skills: &SkillTables, export: &Export) {
    const T: &str = "skills";
    let effects: BTreeMap<&str, _> = skills
        .granted_effects
        .iter()
        .map(|e| (e.id.as_str(), e))
        .collect();
    let sets: BTreeMap<&str, &StatSet> = skills
        .stat_sets
        .iter()
        .map(|s| (s.id.as_str(), s))
        .collect();
    let actives: BTreeMap<&str, _> = skills
        .active_skills
        .iter()
        .map(|a| (a.id.as_str(), a))
        .collect();
    out.bundle_only(
        T,
        effects
            .keys()
            .filter(|id| !export.skills.contains_key(**id))
            .map(|id| (*id).to_owned()),
    );
    for (id, skill) in &export.skills {
        let Some(effect) = effects.get(id.as_str()) else {
            out.pob_only(T, id);
            continue;
        };
        // An effect without a stat set has empty stats (none in the data
        // read; any would show as differences).
        let set = effect
            .stat_sets
            .first()
            .and_then(|s| sets.get(s.as_str()).copied());
        // PoB writes castTime for active skills (skills.lua:251); a support
        // with one is compared too.
        if !effect.support || skill.cast_time.is_some() {
            #[expect(clippy::cast_precision_loss, reason = "milliseconds")]
            let cast = effect.cast_time as f64 / 1000.0;
            out.check(T, id, "cast_time", ours(cast), opt(skill.cast_time));
        }
        let types = effect
            .active_skill
            .as_deref()
            .and_then(|a| actives.get(a))
            .map(|a| a.types.iter().map(String::as_str).collect::<Vec<_>>())
            .unwrap_or_default();
        out.check(
            T,
            id,
            "types",
            sorted_set(types),
            sorted_set(skill.types.iter().map(String::as_str)),
        );
        let base = set.map_or(1.0, |s| f64::from(s.base_effectiveness));
        out.check(
            T,
            id,
            "base_effectiveness",
            ours(base),
            num(skill.base_effectiveness.unwrap_or(1.0)),
        );
        let incremental = set.map_or(0.0, |s| f64::from(s.incremental_effectiveness));
        out.check(
            T,
            id,
            "incremental_effectiveness",
            ours(incremental),
            num(skill.incremental_effectiveness.unwrap_or(0.0)),
        );
        let constants: Vec<Value> = set
            .map(|s| s.constant_stats.as_slice())
            .unwrap_or_default()
            .iter()
            .map(|c| json!([c.stat, int(c.value)]))
            .collect();
        let pob_constants: Vec<Value> = skill
            .constant_stats
            .iter()
            .map(|c| json!([c.stat, num(c.value)]))
            .collect();
        out.check(
            T,
            id,
            "constant_stats",
            Value::Array(constants),
            Value::Array(pob_constants),
        );
        #[expect(clippy::cast_precision_loss, reason = "per mille")]
        let quality: Vec<Value> = effect
            .quality
            .iter()
            .filter(|q| q.stat != DUMMY_QUALITY_STAT)
            .map(|q| json!([q.stat, ours(q.per_mille as f64 / 1000.0)]))
            .collect();
        let pob_quality: Vec<Value> = skill
            .quality_stats
            .iter()
            .map(|q| json!([q.stat, num(q.value)]))
            .collect();
        out.check(
            T,
            id,
            "quality_stats",
            Value::Array(quality),
            Value::Array(pob_quality),
        );
        out.bundle_only(
            T,
            (2..=effect.stat_sets.len()).map(|n| format!("{id}:stat_set.{n}")),
        );
        compare_levels(out, id, effect, set, skill);
    }
}

/// The levels of one skill. PoB's exporter walks the rows by index and
/// uses the only row of a table that has one for every level
/// (`v2.67.2:src/Export/Scripts/skills.lua:262-266`), so a level is found
/// by number, or is the single row.
fn compare_levels(
    out: &mut Comparison,
    id: &str,
    effect: &loom_data::bundle::skills::GrantedEffect,
    set: Option<&StatSet>,
    skill: &pob::Skill,
) {
    const T: &str = "skills";
    let mut levels: Vec<(i64, &pob::Level)> = Vec::new();
    for (key, level) in &skill.levels {
        match key.parse() {
            Ok(n) => levels.push((n, level)),
            Err(_) => out.check(T, id, format!("level.{key}"), Value::Null, json!("present")),
        }
    }
    levels.sort_by_key(|(n, _)| *n);
    let pob_levels: BTreeSet<i64> = levels.iter().map(|(n, _)| *n).collect();
    let bundle_levels: BTreeSet<i64> = effect
        .levels
        .iter()
        .map(|l| l.level)
        .chain(
            set.into_iter()
                .flat_map(|s| s.levels.iter().map(|l| l.level)),
        )
        .collect();
    out.bundle_only(
        T,
        bundle_levels
            .difference(&pob_levels)
            .map(|n| format!("{id}:level.{n}")),
    );
    for (n, level) in levels {
        let find = |count: usize, has: &dyn Fn(usize) -> bool| {
            (0..count).find(|&i| has(i)).or((count == 1).then_some(0))
        };
        let effect_level =
            find(effect.levels.len(), &|i| effect.levels[i].level == n).map(|i| &effect.levels[i]);
        let set_level = set
            .and_then(|s| find(s.levels.len(), &|i| s.levels[i].level == n).map(|i| &s.levels[i]));
        let (Some(el), Some(sl)) = (effect_level, set_level) else {
            out.check(T, id, format!("level.{n}"), Value::Null, json!("present"));
            continue;
        };
        let at = |field: &str| format!("level.{n}.{field}");
        // Stats by name.
        let mut ours_stats: BTreeMap<&str, f64> = BTreeMap::new();
        for f in &sl.float_stats {
            let base = f64::from(f.interpolation_base).max(0.000_01);
            ours_stats.insert(&f.stat, f64::from(f.value) / base);
        }
        for s in &sl.stats {
            #[expect(clippy::cast_precision_loss, reason = "game integers")]
            ours_stats.insert(&s.stat, s.value as f64);
        }
        let pob_stats: BTreeMap<&str, f64> = skill
            .stats
            .iter()
            .map(String::as_str)
            .zip(level.values.iter().copied())
            .collect();
        let names: BTreeSet<&str> = ours_stats.keys().chain(pob_stats.keys()).copied().collect();
        for name in names {
            let a = ours_stats.get(name).copied().unwrap_or(0.0);
            let b = pob_stats.get(name).copied().unwrap_or(0.0);
            out.check(T, id, at(&format!("stat.{name}")), ours(a), num(b));
        }
        // Costs by resource.
        let costs: BTreeMap<&str, i64> = el
            .costs
            .iter()
            .map(|c| (c.resource.as_str(), c.amount))
            .collect();
        let resources: BTreeSet<&str> = costs
            .keys()
            .copied()
            .chain(level.cost.keys().map(String::as_str))
            .collect();
        for resource in resources {
            let a = costs.get(resource).copied().map_or(Value::Null, int);
            let b = opt(level.cost.get(resource).copied());
            out.check(T, id, at(&format!("cost.{resource}")), a, b);
        }
        let per = |x: Option<f64>, divisor: f64, plus: f64| x.map(|v| v / divisor + plus);
        let fields: [(&str, Option<f64>, Option<f64>); 12] = [
            (
                "damage_effectiveness",
                per(sl.damage_effectiveness.and_then(nonzero), 10_000.0, 1.0),
                level.damage_effectiveness,
            ),
            (
                "crit_chance",
                per(
                    nonzero(sl.attack_crit_chance).or(nonzero(sl.spell_crit_chance)),
                    100.0,
                    0.0,
                ),
                level.crit_chance,
            ),
            (
                "base_multiplier",
                per(nonzero(sl.base_multiplier), 10_000.0, 1.0),
                level.base_multiplier,
            ),
            (
                "cooldown",
                per(nonzero(el.cooldown), 1000.0, 0.0),
                level.cooldown,
            ),
            ("stored_uses", nonzero(el.stored_uses), level.stored_uses),
            (
                "attack_speed_multiplier",
                nonzero(el.attack_speed_multiplier),
                level.attack_speed_multiplier,
            ),
            ("attack_time", nonzero(el.attack_time), level.attack_time),
            (
                "mana_multiplier",
                nonzero(el.cost_multiplier - 100),
                level.mana_multiplier,
            ),
            (
                "mana_reservation_flat",
                reservation(el, Resource::Mana, false),
                level.mana_reservation_flat,
            ),
            (
                "mana_reservation_percent",
                per(reservation(el, Resource::Mana, true), 100.0, 0.0),
                level.mana_reservation_percent,
            ),
            (
                "life_reservation_flat",
                reservation(el, Resource::Life, false),
                level.life_reservation_flat,
            ),
            (
                "life_reservation_percent",
                per(reservation(el, Resource::Life, true), 100.0, 0.0),
                level.life_reservation_percent,
            ),
        ];
        for (field, a, b) in fields {
            out.check(T, id, at(field), a.map_or(Value::Null, ours), opt(b));
        }
    }
}

fn reservation(
    level: &loom_data::bundle::skills::EffectLevel,
    resource: Resource,
    percent: bool,
) -> Option<f64> {
    level
        .reservations
        .iter()
        .find(|r| r.resource == resource && r.percent == percent)
        .and_then(|r| nonzero(r.amount))
}

fn compare_gems(out: &mut Comparison, skills: &SkillTables, export: &Export) {
    const T: &str = "gems";
    let effects: BTreeMap<&str, _> = skills
        .gem_effects
        .iter()
        .map(|e| (e.id.as_str(), e))
        .collect();
    let mut owner = BTreeMap::new();
    for gem in &skills.gems {
        for effect in &gem.effects {
            owner.entry(effect.as_str()).or_insert(gem);
        }
    }
    let pob_variants: BTreeSet<&str> = export
        .gems
        .values()
        .map(|g| g.variant_id.as_str())
        .collect();
    out.bundle_only(
        T,
        effects
            .keys()
            .filter(|id| !pob_variants.contains(*id))
            .map(|id| (*id).to_owned()),
    );
    for gem in export.gems.values() {
        let key = gem.variant_id.as_str();
        let (Some(effect), Some(owner)) = (effects.get(key), owner.get(key)) else {
            out.pob_only(T, key);
            continue;
        };
        out.check(
            T,
            key,
            "base_item",
            json!(owner.base_item),
            json!(gem.game_id),
        );
        out.check(
            T,
            key,
            "requirements",
            json!([
                int(owner.strength),
                int(owner.dexterity),
                int(owner.intelligence)
            ]),
            json!([num(gem.req_str), num(gem.req_dex), num(gem.req_int)]),
        );
        out.check(
            T,
            key,
            "granted_effect",
            json!(effect.granted_effect),
            json!(gem.granted_effect_id),
        );
        out.check(
            T,
            key,
            "secondary_granted_effect",
            json!(effect.additional_granted_effects.first()),
            json!(gem.secondary_granted_effect_id),
        );
        out.check(
            T,
            key,
            "tags",
            sorted_set(effect.tags.iter().map(String::as_str)),
            sorted_set(gem.tags.iter().map(String::as_str)),
        );
        out.check(
            T,
            key,
            "vaal",
            json!(owner.vaal),
            json!(gem.vaal_gem.unwrap_or(false)),
        );
    }
}

/// PoB's display name of a base: `ö` as `o`, trimmed
/// (`v2.67.2:src/Export/Scripts/bases.lua:116-117`).
fn pob_name(name: &str) -> String {
    name.replace('\u{f6}', "o").trim().to_owned()
}

fn compare_bases(
    out: &mut Comparison,
    bundle: &Bundle,
    properties: &PropertyTables,
    export: &Export,
) {
    const T: &str = "bases";
    let weapons: BTreeMap<&str, _> = properties
        .weapon_types
        .iter()
        .map(|w| (w.base_item.as_str(), w))
        .collect();
    let armours: BTreeMap<&str, _> = properties
        .armour_types
        .iter()
        .map(|a| (a.base_item.as_str(), a))
        .collect();
    let shields: BTreeMap<&str, i64> = properties
        .shield_types
        .iter()
        .map(|s| (s.base_item.as_str(), s.block))
        .collect();
    // Candidates by PoB's name; PoB skips Royale bases (bases.lua:342).
    let mut by_name: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for base in &bundle.base_items {
        let id = base.id.as_str();
        if (weapons.contains_key(id) || armours.contains_key(id)) && !id.contains("Royale") {
            by_name.entry(pob_name(&base.name)).or_default().push(id);
        }
    }
    let mut matched = BTreeSet::new();
    for (name, base) in &export.bases {
        let candidates = by_name.get(name).map(Vec::as_slice).unwrap_or_default();
        let [id] = candidates else {
            if candidates.is_empty() {
                out.pob_only(T, name);
            } else {
                out.check(
                    T,
                    name,
                    "row",
                    strings(candidates.iter().copied()),
                    json!("one base"),
                );
            }
            continue;
        };
        matched.insert(*id);
        let pob_field = |map: &Option<BTreeMap<String, f64>>, field: &str| {
            map.as_ref().and_then(|m| m.get(field).copied())
        };
        // Weapons: every field is always written (bases.lua:208-211).
        if let Some(w) = weapons.get(id) {
            #[expect(clippy::cast_precision_loss, reason = "game integers")]
            let rate = if w.attack_time == 0 {
                None
            } else {
                // PoB's round(): floor(x * 100 + 0.5) / 100.
                Some((1000.0 / w.attack_time as f64 * 100.0 + 0.5).floor() / 100.0)
            };
            #[expect(clippy::cast_precision_loss, reason = "game integers")]
            let fields = [
                ("physical_min", Some(w.damage_min as f64), "PhysicalMin"),
                ("physical_max", Some(w.damage_max as f64), "PhysicalMax"),
                (
                    "crit_chance",
                    Some(w.crit_chance as f64 / 100.0),
                    "CritChanceBase",
                ),
                ("attack_rate", rate, "AttackRateBase"),
                ("range", Some(w.range as f64), "Range"),
            ];
            for (field, a, pob) in fields {
                out.check(
                    T,
                    name,
                    field,
                    a.map_or(Value::Null, ours),
                    opt(pob_field(&base.weapon, pob)),
                );
            }
        } else if base.weapon.is_some() {
            out.check(T, name, "weapon", Value::Null, json!("present"));
        }
        // Armour: a range is written when its maximum is not 0, the
        // movement penalty when not 0, block for shields
        // (bases.lua:219-242); absent is 0 here.
        if let Some(a) = armours.get(id) {
            let ranges = [
                ("armour", &a.armour, "Armour"),
                ("evasion", &a.evasion, "Evasion"),
                ("energy_shield", &a.energy_shield, "EnergyShield"),
                ("ward", &a.ward, "Ward"),
            ];
            for (field, range, pob) in ranges {
                for (end, value) in [("min", range.min), ("max", range.max)] {
                    let pob_end = if end == "min" { "BaseMin" } else { "BaseMax" };
                    out.check(
                        T,
                        name,
                        format!("{field}_{end}"),
                        int(value),
                        num(pob_field(&base.armour, &format!("{pob}{pob_end}")).unwrap_or(0.0)),
                    );
                }
            }
            out.check(
                T,
                name,
                "movement_penalty",
                int(-a.movement_speed),
                num(pob_field(&base.armour, "MovementPenalty").unwrap_or(0.0)),
            );
            out.check(
                T,
                name,
                "block",
                int(shields.get(id).copied().unwrap_or(0)),
                num(pob_field(&base.armour, "BlockChance").unwrap_or(0.0)),
            );
        } else if base.armour.is_some() {
            out.check(T, name, "armour", Value::Null, json!("present"));
        }
    }
    out.bundle_only(
        T,
        by_name
            .values()
            .flatten()
            .filter(|id| !matched.contains(*id))
            .map(|id| (*id).to_owned()),
    );
}

fn compare_characters(out: &mut Comparison, passives: &PassiveTables, export: &Export) {
    const T: &str = "characters";
    let mut seen = BTreeSet::new();
    for class in &export.tree.classes {
        let Some(character) = passives.characters.iter().find(|c| c.name == class.name) else {
            out.pob_only(T, &class.name);
            continue;
        };
        seen.insert(character.id.as_str());
        let key = class.name.as_str();
        out.check(
            T,
            key,
            "strength",
            int(character.strength),
            num(class.base_str),
        );
        out.check(
            T,
            key,
            "dexterity",
            int(character.dexterity),
            num(class.base_dex),
        );
        out.check(
            T,
            key,
            "intelligence",
            int(character.intelligence),
            num(class.base_int),
        );
        let ascendancies = passives
            .ascendancies
            .iter()
            .filter(|a| a.character.as_deref() == Some(character.id.as_str()))
            .map(|a| a.id.as_str());
        out.check(
            T,
            key,
            "ascendancies",
            sorted_set(ascendancies),
            sorted_set(class.ascendancies.iter().map(String::as_str)),
        );
    }
    out.bundle_only(
        T,
        passives
            .characters
            .iter()
            .filter(|c| !seen.contains(c.id.as_str()))
            .map(|c| c.name.clone()),
    );
}

fn template_stats<'a>(passives: &'a PassiveTables, id: &str) -> BTreeMap<&'a str, i64> {
    passives
        .object_templates
        .iter()
        .find(|t| t.id == id)
        .map(|t| {
            t.stats
                .iter()
                .map(|s: &StatAmount| (s.stat.as_str(), s.value))
                .collect()
        })
        .unwrap_or_default()
}

fn compare_constants(
    out: &mut Comparison,
    passives: &PassiveTables,
    properties: &PropertyTables,
    export: &Export,
) {
    for (table, template, pob) in [
        (
            "character_constants",
            CHARACTER_TEMPLATE,
            &export.character_constants,
        ),
        (
            "monster_constants",
            MONSTER_TEMPLATE,
            &export.monster_constants,
        ),
    ] {
        let stats = template_stats(passives, template);
        let keys: BTreeSet<&str> = stats
            .keys()
            .copied()
            .chain(pob.keys().map(String::as_str))
            .collect();
        for key in keys {
            out.check(
                table,
                key,
                "value",
                stats.get(key).copied().map_or(Value::Null, int),
                opt(pob.get(key).copied()),
            );
        }
    }
    const T: &str = "game_constants";
    let ours_constants: BTreeMap<&str, f64> = properties
        .game_constants
        .iter()
        .map(|c| {
            #[expect(clippy::cast_precision_loss, reason = "game integers")]
            let value = c.value as f64 / c.divisor as f64;
            (c.id.as_str(), value)
        })
        .collect();
    let keys: BTreeSet<&str> = ours_constants
        .keys()
        .copied()
        .chain(export.game_constants.keys().map(String::as_str))
        .collect();
    for key in keys {
        out.check(
            T,
            key,
            "value",
            ours_constants.get(key).copied().map_or(Value::Null, ours),
            opt(export.game_constants.get(key).copied()),
        );
    }
}

/// PoB's bosses (`data.bosses`) against the bundle's monster types, by
/// a boss list (the engine's `loom_calc::poe1::enemy::BOSSES`, PoB's
/// `Export/Enemies/Bosses.txt`, in a run): each boss's armour and evasion
/// multipliers and uber mark; a boss on one side only is a difference.
fn compare_bosses(
    out: &mut Comparison,
    properties: &PropertyTables,
    export: &Export,
    bosses: &[Boss],
) {
    const T: &str = "bosses";
    let ours: BTreeMap<&str, (&Boss, Option<&MonsterType>)> = bosses
        .iter()
        .map(|b| {
            let row = properties
                .monster_types
                .iter()
                .find(|m| m.id == b.monster_type);
            (b.name, (b, row))
        })
        .collect();
    let names: BTreeSet<&str> = ours
        .keys()
        .copied()
        .chain(export.bosses.keys().map(String::as_str))
        .collect();
    for name in names {
        let pob = export.bosses.get(name);
        let (boss, row) = match ours.get(name) {
            Some((b, r)) => (Some(*b), *r),
            None => (None, None),
        };
        out.check(
            T,
            name,
            "armour",
            row.map_or(Value::Null, |r| int(r.armour)),
            opt(pob.map(|p| p.armour_mult)),
        );
        out.check(
            T,
            name,
            "evasion",
            row.map_or(Value::Null, |r| int(r.evasion)),
            opt(pob.map(|p| p.evasion_mult)),
        );
        out.check(
            T,
            name,
            "uber",
            boss.map_or(Value::Null, |b| Value::Bool(b.uber)),
            pob.map_or(Value::Null, |p| Value::Bool(p.is_uber)),
        );
    }
}

/// The stat-description scope PoB's pantheon exporter renders soul lines
/// with (`v2.67.2:src/Export/Scripts/pantheons.lua`: `stat_descriptions.txt`).
pub const PANTHEON_SCOPE: &str = "stat_descriptions";

/// PoB's pantheon (`data.pantheons`) against the bundle's
/// (`poe1/pantheons.json`, P3-12g), by god ID: the major mark, the souls'
/// names in order, and each soul's mods. PoB keeps each mod as the line
/// its exporter rendered from the stat and its value
/// (`table.concat(describeStats(stats), ' ')`) and the value; the bundle
/// keeps the stat. So each of the bundle's stats is rendered with the
/// game's stat descriptions ([`PANTHEON_SCOPE`]) into PoB's form, and the
/// lines and values are compared. A god on one side only is a difference;
/// a bundle without the file has every PoB god on PoB's side only.
///
/// # Errors
/// A message when the scope does not load.
fn compare_pantheons(out: &mut Comparison, bundle: &Bundle, export: &Export) -> Result<(), String> {
    const T: &str = "pantheons";
    let gods: BTreeMap<&str, &loom_data::bundle::poe1::Pantheon> = bundle
        .poe1
        .as_ref()
        .and_then(|p| p.pantheons.as_ref())
        .into_iter()
        .flatten()
        .map(|g| (g.id.as_str(), g))
        .collect();
    out.bundle_only(
        T,
        gods.keys()
            .filter(|id| !export.pantheons.contains_key(**id))
            .map(|id| (*id).to_owned()),
    );
    if export.pantheons.is_empty() {
        return Ok(());
    }
    let root = bundle
        .scope_root(PANTHEON_SCOPE)
        .map_err(|e| format!("stat descriptions {PANTHEON_SCOPE}: {e}"))?;
    let scope = bundle
        .stat_descriptions
        .scope(root)
        .map_err(|e| format!("stat descriptions {PANTHEON_SCOPE}: {e}"))?;
    for (id, pob) in &export.pantheons {
        let Some(god) = gods.get(id.as_str()) else {
            out.pob_only(T, id);
            continue;
        };
        out.check(T, id, "major", json!(god.major), json!(pob.is_major_god));
        out.check(
            T,
            id,
            "souls",
            json!(god.souls.iter().map(|s| &s.name).collect::<Vec<_>>()),
            json!(pob.souls.iter().map(|s| &s.name).collect::<Vec<_>>()),
        );
        for (n, (soul, pob_soul)) in god.souls.iter().zip(&pob.souls).enumerate() {
            let lines: Vec<String> = soul
                .stats
                .iter()
                .map(|(stat, value)| {
                    let one = [StatAmount {
                        stat: stat.clone(),
                        value: *value,
                    }];
                    render(bundle, &scope, &one).join(" ")
                })
                .collect();
            out.check(
                T,
                id,
                format!("soul.{}.lines", n + 1),
                json!(lines),
                json!(pob_soul.mods.iter().map(|m| &m.line).collect::<Vec<_>>()),
            );
            out.check(
                T,
                id,
                format!("soul.{}.values", n + 1),
                Value::Array(soul.stats.iter().map(|(_, v)| int(*v)).collect()),
                Value::Array(
                    pob_soul
                        .mods
                        .iter()
                        .flat_map(|m| &m.value)
                        .map(|v| num(*v))
                        .collect(),
                ),
            );
        }
    }
    Ok(())
}

fn compare_monster_levels(out: &mut Comparison, properties: &PropertyTables, export: &Export) {
    const T: &str = "monster_levels";
    // PoB's entry i is level i + 1; the bundle's rows are found by level.
    let rows: BTreeMap<i64, &MonsterLevel> = properties
        .monster_levels
        .iter()
        .map(|r| (r.level, r))
        .collect();
    let last = rows.keys().next_back().copied().unwrap_or(0);
    for (table, field, column) in MONSTER_TABLES {
        let pob = export
            .monster_tables
            .get(table)
            .map(Vec::as_slice)
            .unwrap_or_default();
        let count = i64::try_from(pob.len()).unwrap_or(i64::MAX).max(last);
        for level in 1..=count {
            let ours_value = rows.get(&level).and_then(|r| column(r));
            let pob_value = usize::try_from(level - 1)
                .ok()
                .and_then(|i| pob.get(i).copied());
            out.check(
                T,
                &level.to_string(),
                field,
                ours_value.map_or(Value::Null, ours),
                opt(pob_value),
            );
        }
    }
}

/// Renders stats with the passive scope; lines sorted. A stat that does
/// not render is shown as `<stat id: reason>`, so it differs visibly.
fn render(
    bundle: &Bundle,
    scope: &loom_data::stat_desc::Scope<'_>,
    stats: &[StatAmount],
) -> Vec<String> {
    let map: BTreeMap<String, StatValue> = stats
        .iter()
        .map(|s| (s.stat.clone(), StatValue::exact(s.value)))
        .collect();
    let rendered = scope.render(&map, bundle, VariantFilter::default());
    let mut lines: Vec<String> = rendered
        .lines
        .into_iter()
        .flat_map(|l| {
            l.text
                .split('\n')
                .map(|s| s.trim().to_owned())
                .collect::<Vec<_>>()
        })
        .filter(|l| !l.is_empty())
        .collect();
    // A stat without a description shows no line in the game, as in the
    // tree text; any other reason is a rendering failure and shows.
    lines.extend(
        rendered
            .not_rendered
            .into_iter()
            .filter(|n| {
                !matches!(
                    n.reason,
                    NotRenderedReason::Zero
                        | NotRenderedReason::NoDescription
                        | NotRenderedReason::DeclaredNoDescription
                )
            })
            .map(|n| format!("<{}: {}>", n.stat, n.reason.name())),
    );
    lines.sort();
    lines
}

/// PoB's lines, split at newlines and sorted. The passive points a node
/// grants are a column in the game data (`skill_points`) and a field in
/// the tree file (`grantedPassivePoints`), compared as numbers; the tree
/// text also writes them as a line ("Grants 1 Passive Skill Point"), which
/// is left out here.
fn pob_lines(stats: &[String]) -> Vec<String> {
    let mut lines: Vec<String> = stats
        .iter()
        .flat_map(|s| s.split('\n'))
        .map(str::trim)
        .filter(|line| {
            let points = line.strip_prefix("Grants ").and_then(|rest| {
                rest.strip_suffix(" Passive Skill Points")
                    .or_else(|| rest.strip_suffix(" Passive Skill Point"))
            });
            !line.is_empty() && points.is_none_or(|n| n.parse::<i64>().is_err())
        })
        .map(str::to_owned)
        .collect();
    lines.sort();
    lines
}

fn compare_passives(
    out: &mut Comparison,
    bundle: &Bundle,
    passives: &PassiveTables,
    export: &Export,
    passive_scope: &str,
) -> Result<(), String> {
    const T: &str = "passives";
    let root = bundle
        .scope_root(passive_scope)
        .map_err(|e| format!("stat descriptions {passive_scope}: {e}"))?;
    let scope = bundle
        .stat_descriptions
        .scope(root)
        .map_err(|e| format!("stat descriptions {passive_scope}: {e}"))?;
    let by_node: BTreeMap<i64, &Passive> = passives.passives.iter().map(|p| (p.node, p)).collect();
    let effects: BTreeMap<i64, _> = passives
        .mastery_effects
        .iter()
        .map(|e| (e.hash, e))
        .collect();
    let mut nodes: Vec<&pob::Node> = export.tree.nodes.values().collect();
    nodes.sort_by_key(|n| n.skill);
    let pob_nodes: BTreeSet<i64> = nodes.iter().map(|n| n.skill).collect();
    out.bundle_only(
        T,
        by_node
            .keys()
            .filter(|n| !pob_nodes.contains(n))
            .map(ToString::to_string),
    );
    let mut effects_seen = BTreeSet::new();
    for node in nodes {
        let key = node.skill.to_string();
        let Some(passive) = by_node.get(&node.skill) else {
            out.pob_only(T, &key);
            continue;
        };
        out.check(T, &key, "name", json!(passive.name), json!(node.name));
        out.check(
            T,
            &key,
            "keystone",
            json!(passive.keystone),
            json!(node.is_keystone),
        );
        out.check(
            T,
            &key,
            "notable",
            json!(passive.notable),
            json!(node.is_notable),
        );
        out.check(
            T,
            &key,
            "jewel_socket",
            json!(passive.jewel_socket),
            json!(node.is_jewel_socket),
        );
        out.check(
            T,
            &key,
            "mastery",
            json!(passive.mastery_group.is_some()),
            json!(node.is_mastery),
        );
        out.check(
            T,
            &key,
            "ascendancy",
            json!(passive.ascendancy),
            json!(node.ascendancy_name),
        );
        out.check(
            T,
            &key,
            "stats",
            json!(render(bundle, &scope, &passive.stats)),
            json!(pob_lines(&node.stats)),
        );
        out.check(
            T,
            &key,
            "skill_points",
            int(passive.skill_points),
            int(node.granted_passive_points.unwrap_or(0)),
        );
        for effect in &node.mastery_effects {
            const E: &str = "mastery_effects";
            if !effects_seen.insert(effect.effect) {
                continue;
            }
            let effect_key = effect.effect.to_string();
            let Some(ours_effect) = effects.get(&effect.effect) else {
                out.pob_only(E, &effect_key);
                continue;
            };
            out.check(
                E,
                &effect_key,
                "stats",
                json!(render(bundle, &scope, &ours_effect.stats)),
                json!(pob_lines(&effect.stats)),
            );
        }
    }
    out.bundle_only(
        "mastery_effects",
        effects
            .keys()
            .filter(|h| !effects_seen.contains(*h))
            .map(ToString::to_string),
    );
    Ok(())
}
