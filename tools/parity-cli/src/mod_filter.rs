//! The `Mods` filter for ambiguous lines and the `mod-backed` measure
//! (Spike B, PR B4a; plan "Ambiguity" and step 4a).
//!
//! **Fit** (domain and a generation type that fits the line's section),
//! for one row of the item's base (`BaseItemTypes`, every row with the
//! base's name; a row fits when it fits any of them). Enum values are the
//! vendored schema's `ModDomains` and `ModGenerationType` (both
//! `indexing: 1`):
//! - explicit `{crafted}`: domain `CRAFTED`, prefix or suffix, and the row's
//!   `CraftingItemClassRestrictions` empty or naming the base's item class;
//! - explicit on a unique: the base's domain, `UNIQUE`;
//! - explicit otherwise: prefix or suffix in the base's domain, or `DELVE`
//!   (base domain `ITEM`, `BASE_JEWEL`, `ABYSS_JEWEL`), or `UNVEILED`
//!   (base domain `ITEM`); an essence-only row also needs the base's class
//!   among the classes `Essences` gives it, when known (PR B4y);
//! - implicit `{exarch}` / `{eater}`: `EXARCH_IMPLICIT` / `EATER_IMPLICIT`
//!   in the base's domain; other implicits: `UNIQUE` (base implicits and
//!   unique mods), `CORRUPTED`, `EXARCH_IMPLICIT`, or `EATER_IMPLICIT` in
//!   the base's domain (PoB items do not always carry the eldritch flags),
//!   or a synthesis generation type in a synthesis domain;
//! - enchant: `ENCHANTMENT`, a flask enchantment type, or `UNIQUE`, in the
//!   base's domain or `ITEM` (flask enchantments are `ITEM` rows; Harvest
//!   quality enchants, Heist enchants, cluster jewel enchants, and anoints
//!   (`GrantedPassive*`) are `UNIQUE` rows without spawn weights, PoE1
//!   26530219);
//! - scourge: `SCOURGE_UPSIDE` / `SCOURGE_DOWNSIDE`; crucible:
//!   `WEAPON_TREE` / `WEAPON_TREE_RECOMBINED`; both in the base's domain;
//! - class requirement: nothing.
//!
//! **Backing.** A row *backs* a candidate when at least one candidate stat is
//! on the row and every candidate stat that is not on the row admits 0 (the
//! description's other stats, not rolled). Row stats with `(min, max) =
//! (0, 0)` are not on the row. *Equal* when every row stat is a candidate
//! stat; *hybrid* otherwise (the row's other stats are other lines of the
//! item, not checked). At the *values* level, every candidate stat on the
//! row also overlaps the row's `[min, max]`.
//!
//! **Filter** of an ambiguous line, each step applied only when it keeps at
//! least one candidate and removes at least one:
//! 1. `mods`: candidates backed by a fitting row. When no candidate is
//!    backed, every candidate stays (`no-backed-candidate`), no tie-break.
//! 2. `base-implicit` (implicit, not eldritch): candidates backed by one of
//!    the base's `Implicit_ModsKeys` rows.
//! 3. `spawn-tags` (tie-breaker, reported): candidates backed by a fitting
//!    row with a positive spawn weight on the base's tags (the first row tag
//!    the base has gives the weight). Skipped, with a note, for `{crafted}`
//!    lines (bench crafts do not spawn), when the base's tags are unknown,
//!    when the item has an influence and a candidate that does not spawn is
//!    backed by an influence row of that influence (the item's influence
//!    tags need `InfluenceTags`, not extracted), and when a candidate that
//!    does not spawn is backed by an essence-only row whose classes are
//!    unknown. A row an essence with known classes puts on the base's class
//!    counts as spawning. Eldritch rows do not count on implicit lines
//!    without an eldritch flag.
//! 4. `values` (tie-breaker, reported): candidates backed at the values
//!    level.
//!
//! One candidate left: the line is resolved, with the steps that narrowed it.

use std::collections::{BTreeMap, BTreeSet};

use loom_data::stat_desc::Condition;
use loom_parser::pob_item::{Modifier, Section};
use loom_parser::resolve::{Candidate, RawValue, ResolvedStat};
use serde_json::Value;

use crate::extract::{Read, Table};

/// `ModDomains` values used by the fit rules.
pub mod domain {
    /// `ITEM`.
    pub const ITEM: i64 = 1;
    /// `CRAFTED`.
    pub const CRAFTED: i64 = 9;
    /// `BASE_JEWEL`.
    pub const BASE_JEWEL: i64 = 10;
    /// `ABYSS_JEWEL`.
    pub const ABYSS_JEWEL: i64 = 13;
    /// `DELVE`.
    pub const DELVE: i64 = 16;
    /// `SYNTHESIS_A`, `SYNTHESIS_GLOBALS`, `SYNTHESIS_BONUS`.
    pub const SYNTHESIS: [i64; 3] = [18, 19, 20];
    /// `UNVEILED`.
    pub const UNVEILED: i64 = 28;
}

/// `ModGenerationType` values used by the fit rules.
pub mod generation {
    /// `PREFIX`.
    pub const PREFIX: i64 = 1;
    /// `SUFFIX`.
    pub const SUFFIX: i64 = 2;
    /// `UNIQUE`.
    pub const UNIQUE: i64 = 3;
    /// `CORRUPTED`.
    pub const CORRUPTED: i64 = 5;
    /// `ENCHANTMENT`.
    pub const ENCHANTMENT: i64 = 10;
    /// `SYNTHESIS_A`, `SYNTHESIS_GLOBALS`, `SYNTHESIS_BONUS`.
    pub const SYNTHESIS: [i64; 3] = [15, 16, 17];
    /// `FLASK_ENCHANTMENT_ENKINDLING`, `FLASK_ENCHANTMENT_INSTILLING`.
    pub const FLASK_ENCHANTMENT: [i64; 2] = [21, 22];
    /// `SCOURGE_UPSIDE`, `SCOURGE_DOWNSIDE`.
    pub const SCOURGE: [i64; 2] = [24, 25];
    /// `EXARCH_IMPLICIT`.
    pub const EXARCH: i64 = 28;
    /// `EATER_IMPLICIT`.
    pub const EATER: i64 = 29;
    /// `WEAPON_TREE`, `WEAPON_TREE_RECOMBINED`.
    pub const WEAPON_TREE: [i64; 2] = [31, 32];
}

/// `InfluenceTypes` value of rows without an influence (`NONE`, indexing 0).
const NO_INFLUENCE: i64 = 6;

/// Influence line display (`pob_item::INFLUENCES`) to `InfluenceTypes`
/// (`SHAPER` 0, `ELDER` 1, `CRUSADER` 2, `EYRIE` 3, `BASILISK` 4,
/// `ADJUDICATOR` 5), after PoB's `influenceInfo` (`v2.67.2:
/// src/Modules/ItemTools.lua`: Redeemer is eyrie, Hunter basilisk, Warlord
/// adjudicator). The eldritch displays have no `InfluenceTypes` value.
const INFLUENCE_TYPES: [(&str, i64); 6] = [
    ("Shaper", 0),
    ("Elder", 1),
    ("Crusader", 2),
    ("Redeemer", 3),
    ("Hunter", 4),
    ("Warlord", 5),
];

/// The `InfluenceTypes` value of an influence line (`<display> Item`).
#[must_use]
pub fn influence_type(line: &str) -> Option<i64> {
    let display = line.trim().strip_suffix(" Item")?;
    INFLUENCE_TYPES
        .iter()
        .find(|(d, _)| *d == display)
        .map(|(_, v)| *v)
}

/// One `Mods` row, as the filter needs it.
#[derive(Debug, Clone, Default)]
pub struct ModRow {
    /// `Id`.
    pub id: String,
    /// `Domain`.
    pub domain: i64,
    /// `GenerationType`.
    pub generation: i64,
    /// `(stat, min, max)` with `min <= max`; stats with `(0, 0)` left out.
    pub stats: Vec<(String, i64, i64)>,
    /// Stats with `(0, 0)`: absent, or a value set outside the row
    /// (`GrantedPassive*` for anoints, `JewelExpansionPassiveNodes` for
    /// cluster jewels); only used to name why a line is not backed.
    pub unset: Vec<String>,
    /// `(Tags.Id, weight)` in row order.
    pub spawn: Vec<(String, i64)>,
    /// `InfluenceTypes`, `None` for `NONE`.
    pub influence: Option<i64>,
    /// `CraftingItemClassRestrictions` (`ItemClasses` rows).
    pub craft_classes: Vec<i64>,
    /// `IsEssenceOnlyModifier`.
    pub essence_only: bool,
    /// `ItemClasses` rows an essence puts this mod on (`Essences`), `None`
    /// when no `Essences` column names the row or the table is absent.
    pub essence_classes: Option<Vec<i64>>,
}

impl ModRow {
    /// The spawn weight on an item with `tags`.
    fn weight(&self, tags: &[String]) -> i64 {
        self.spawn
            .iter()
            .find(|(t, _)| tags.contains(t))
            .map_or(0, |(_, w)| *w)
    }

    /// Whether an essence can put this row on `base`: `true` when the
    /// essence classes are unknown (the guard in [`filter`] handles that).
    fn essence_on(&self, base: &BaseRow) -> bool {
        self.essence_classes
            .as_ref()
            .is_none_or(|c| base.class.is_some_and(|k| c.contains(&k)))
    }

    /// Whether an essence with known classes puts this row on `base`.
    fn essence_known_on(&self, base: &BaseRow) -> bool {
        self.essence_classes
            .as_ref()
            .is_some_and(|c| base.class.is_some_and(|k| c.contains(&k)))
    }

    fn range(&self, stat: &str) -> Option<(i64, i64)> {
        self.stats
            .iter()
            .find(|(s, _, _)| s == stat)
            .map(|(_, lo, hi)| (*lo, *hi))
    }

    /// `Some(true)` equal, `Some(false)` hybrid, `None` when the row does
    /// not back `stats` (module docs).
    fn backs(&self, stats: &[ResolvedStat]) -> Option<bool> {
        let mut on = 0;
        for s in stats {
            if self.range(&s.stat).is_some() {
                on += 1;
            } else if !admits(&s.value, 0, 0) {
                return None;
            }
        }
        (on > 0).then(|| {
            self.stats
                .iter()
                .all(|(r, _, _)| stats.iter().any(|s| s.stat == *r))
        })
    }

    /// Whether every candidate stat on the row overlaps the row's range.
    fn values_overlap(&self, stats: &[ResolvedStat]) -> bool {
        stats.iter().all(|s| {
            self.range(&s.stat)
                .is_none_or(|(lo, hi)| admits(&s.value, lo, hi))
        })
    }
}

/// Whether a value read from a line allows some raw value in `[lo, hi]`.
fn admits(value: &RawValue, lo: i64, hi: i64) -> bool {
    match value {
        RawValue::Exact(s) | RawValue::Ranged(s) => s.min <= hi && lo <= s.max,
        RawValue::Hidden { condition, .. } => match *condition {
            Condition::Range { min, max } => {
                min.map_or(lo, |m| m.max(lo)) <= max.map_or(hi, |m| m.min(hi))
            }
            Condition::Not(v) => !(lo == hi && lo == v),
        },
        RawValue::Lookup(keys) => keys.iter().any(|k| (lo..=hi).contains(k)),
    }
}

/// The `Mods` table, indexed by stat.
#[derive(Debug, Default)]
pub struct Mods {
    rows: Vec<ModRow>,
    by_stat: BTreeMap<String, Vec<usize>>,
    by_unset: BTreeMap<String, Vec<usize>>,
    /// Rows whose spawn tag and weight arrays differ in length (read with
    /// no spawn weights).
    pub bad_spawn_rows: usize,
}

/// `Mods` row stat columns (`StatsKeyN`, `StatNMin`, `StatNMax`).
const STAT_SLOTS: usize = 8;

fn ints(row: &[Value], col: usize) -> Vec<i64> {
    row.get(col)
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_i64).collect())
        .unwrap_or_default()
}

/// `Tags.Id` by row.
///
/// # Errors
/// A message when the table or column is missing.
pub fn tag_ids(read: Read<'_>) -> Result<Vec<String>, String> {
    ids(read, "Tags")
}

fn ids(read: Read<'_>, table: &str) -> Result<Vec<String>, String> {
    let t = Table::require(read, table)?;
    let id = t.column("Id")?;
    Ok(t.rows
        .iter()
        .map(|r| Table::text(r, id).unwrap_or("").to_owned())
        .collect())
}

/// `(Tags.Id, weight)` pairs of a row. Tags and weights are paired by
/// position before anything is dropped, so a bad entry (unknown tag index,
/// non-integer weight) drops its own pair only and never shifts the rest;
/// `None` when the two arrays differ in length (measured: never).
fn spawn_pairs(
    row: &[Value],
    tags: usize,
    weights: usize,
    ids: &[String],
) -> Option<Vec<(String, i64)>> {
    let array = |c: usize| row.get(c).and_then(Value::as_array);
    let (t, w) = (array(tags)?, array(weights)?);
    (t.len() == w.len()).then(|| {
        t.iter()
            .zip(w)
            .filter_map(|(t, w)| {
                let tag = usize::try_from(t.as_i64()?).ok().and_then(|i| ids.get(i))?;
                Some((tag.clone(), w.as_i64()?))
            })
            .collect()
    })
}

/// `Essences` columns (PoE1 schema) to the `ItemClasses.Id`s they cover.
/// Following PoB, the `Staff` column covers warstaves and the `Dagger`
/// column rune daggers (`v2.67.2:src/Data/Bases/staff.lua` and `dagger.lua`
/// type them `Staff` and `Dagger` with a `subType`). The `Display_*` columns
/// are summaries and are not read, except `Display_Quiver_ModsKey`: the
/// schema names no other quiver column, and it is the only column that
/// names the 25 quiver essence rows (`QuiverAddedChaosEssence1_`, ...;
/// PoE1 26530219). The 125 essence-only rows no column names are legacy
/// essence mods (`ChanceToDodgeEssence4`, ...): their classes stay unknown.
const ESSENCE_COLUMNS: [(&str, &[&str]); 22] = [
    ("Display_Quiver_ModsKey", &["Quiver"]),
    ("Helmet_ModsKey", &["Helmet"]),
    ("BodyArmour_ModsKey", &["Body Armour"]),
    ("Boots_ModsKey", &["Boots"]),
    ("Gloves_ModsKey", &["Gloves"]),
    ("Shield_ModsKey", &["Shield"]),
    ("Belt_ModsKey", &["Belt"]),
    ("Amulet_ModsKey", &["Amulet"]),
    ("Ring_ModsKey", &["Ring"]),
    ("Bow_ModsKey", &["Bow"]),
    ("Wand_ModsKey", &["Wand"]),
    ("Staff_ModsKey", &["Staff", "Warstaff"]),
    ("TwoHandSword_ModsKey", &["Two Hand Sword"]),
    ("TwoHandAxe_ModsKey", &["Two Hand Axe"]),
    ("TwoHandMace_ModsKey", &["Two Hand Mace"]),
    ("Claw_ModsKey", &["Claw"]),
    ("Dagger_ModsKey", &["Dagger", "Rune Dagger"]),
    ("OneHandSword_ModsKey", &["One Hand Sword"]),
    (
        "OneHandThrustingSword_ModsKey",
        &["Thrusting One Hand Sword"],
    ),
    ("OneHandAxe_ModsKey", &["One Hand Axe"]),
    ("OneHandMace_ModsKey", &["One Hand Mace"]),
    ("Sceptre_ModsKey", &["Sceptre"]),
];

/// `Mods` row to the `ItemClasses` rows essences put it on; `None` when the
/// extract has no `Essences` table (extracts before PR B4y).
///
/// # Errors
/// A message when `Essences` exists but `ItemClasses`, a column, or a class
/// of [`ESSENCE_COLUMNS`] is missing.
fn essence_classes(read: Read<'_>) -> Result<Option<BTreeMap<usize, BTreeSet<i64>>>, String> {
    let Some(essences) = Table::read(read, "Essences")? else {
        return Ok(None);
    };
    let classes = ids(read, "ItemClasses")?;
    let class_row = |id: &str| -> Result<i64, String> {
        classes
            .iter()
            .position(|c| c == id)
            .and_then(|n| i64::try_from(n).ok())
            .ok_or_else(|| format!("ItemClasses: no row {id}"))
    };
    let mut out: BTreeMap<usize, BTreeSet<i64>> = BTreeMap::new();
    for (column, ids) in ESSENCE_COLUMNS {
        let col = essences.column(column)?;
        let rows: Vec<i64> = ids
            .iter()
            .map(|id| class_row(id))
            .collect::<Result<_, _>>()?;
        for r in &essences.rows {
            if let Some(n) = Table::int(r, col).and_then(|n| usize::try_from(n).ok()) {
                out.entry(n).or_default().extend(&rows);
            }
        }
    }
    Ok(Some(out))
}

impl Mods {
    /// Reads `Mods`, `Stats`, and `Tags`, and `Essences` with `ItemClasses`
    /// when the extract has them. Row `n` of the result is row `n`
    /// of `Mods` (`Implicit_ModsKeys` index it).
    ///
    /// # Errors
    /// A message when a table or column is missing.
    pub fn load(read: Read<'_>) -> Result<Self, String> {
        let stats = ids(read, "Stats")?;
        let tags = tag_ids(read)?;
        let t = Table::require(read, "Mods")?;
        let col = |n: &str| t.column(n);
        let (id, dom, gen_) = (col("Id")?, col("Domain")?, col("GenerationType")?);
        let (spawn_tags, spawn_values) = (col("SpawnWeight_TagsKeys")?, col("SpawnWeight_Values")?);
        let (influence, craft, essence) = (
            col("InfluenceTypes")?,
            col("CraftingItemClassRestrictions")?,
            col("IsEssenceOnlyModifier")?,
        );
        let mut slots = Vec::with_capacity(STAT_SLOTS);
        for n in 1..=STAT_SLOTS {
            slots.push((
                col(&format!("StatsKey{n}"))?,
                col(&format!("Stat{n}Min"))?,
                col(&format!("Stat{n}Max"))?,
            ));
        }
        let mut rows = Vec::with_capacity(t.rows.len());
        let mut bad_spawn_rows = 0;
        for r in &t.rows {
            let (mut row_stats, mut unset) = (Vec::new(), Vec::new());
            for &(k, lo, hi) in &slots {
                let Some(name) = Table::int(r, k)
                    .and_then(|i| usize::try_from(i).ok())
                    .and_then(|i| stats.get(i))
                else {
                    continue;
                };
                match (Table::int(r, lo), Table::int(r, hi)) {
                    (Some(0), Some(0)) => unset.push(name.clone()),
                    (Some(a), Some(b)) => row_stats.push((name.clone(), a.min(b), a.max(b))),
                    _ => {}
                }
            }
            let spawn = spawn_pairs(r, spawn_tags, spawn_values, &tags).unwrap_or_else(|| {
                bad_spawn_rows += 1;
                Vec::new()
            });
            rows.push(ModRow {
                id: Table::text(r, id).unwrap_or("").to_owned(),
                domain: Table::int(r, dom).unwrap_or(0),
                generation: Table::int(r, gen_).unwrap_or(0),
                stats: row_stats,
                unset,
                spawn,
                influence: Table::int(r, influence).filter(|v| *v != NO_INFLUENCE),
                craft_classes: ints(r, craft),
                essence_only: r.get(essence).and_then(Value::as_bool).unwrap_or(false),
                essence_classes: None,
            });
        }
        if let Some(classes) = essence_classes(read)? {
            for (n, list) in classes {
                if let Some(row) = rows.get_mut(n) {
                    row.essence_classes = Some(list.into_iter().collect());
                }
            }
        }
        let mut mods = Self::from_rows(rows);
        mods.bad_spawn_rows = bad_spawn_rows;
        Ok(mods)
    }

    /// Essence-only rows whose item classes `Essences` gives, and the rest.
    #[must_use]
    pub fn essence_rows(&self) -> (usize, usize) {
        let only = self.rows.iter().filter(|r| r.essence_only);
        let known = only.clone().filter(|r| r.essence_classes.is_some()).count();
        (known, only.count() - known)
    }

    /// Indexes rows (row `n` keeps index `n`). Rows keep their
    /// `essence_classes`.
    #[must_use]
    pub fn from_rows(rows: Vec<ModRow>) -> Self {
        fn add(index: &mut BTreeMap<String, Vec<usize>>, stat: &str, n: usize) {
            let list = index.entry(stat.to_owned()).or_default();
            if list.last() != Some(&n) {
                list.push(n);
            }
        }
        let (mut by_stat, mut by_unset) = (BTreeMap::new(), BTreeMap::new());
        for (n, row) in rows.iter().enumerate() {
            for (stat, _, _) in &row.stats {
                add(&mut by_stat, stat, n);
            }
            for stat in &row.unset {
                add(&mut by_unset, stat, n);
            }
        }
        Self {
            rows,
            by_stat,
            by_unset,
            bad_spawn_rows: 0,
        }
    }

    /// Number of rows.
    #[must_use]
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Rows with at least one of `stats`, in row order.
    fn rows_for(&self, stats: &[ResolvedStat]) -> BTreeSet<usize> {
        stats
            .iter()
            .filter_map(|s| self.by_stat.get(&s.stat))
            .flatten()
            .copied()
            .collect()
    }

    fn row(&self, n: usize) -> Option<&ModRow> {
        self.rows.get(n)
    }

    /// Rows that carry one of `stats` at `(0, 0)`.
    fn unset_rows(&self, stats: &[ResolvedStat]) -> impl Iterator<Item = &ModRow> {
        stats
            .iter()
            .filter_map(|s| self.by_unset.get(&s.stat))
            .flatten()
            .filter_map(|n| self.rows.get(*n))
    }
}

/// One `BaseItemTypes` row of the item's base.
#[derive(Debug, Clone, Default)]
pub struct BaseRow {
    /// `ModDomain`.
    pub domain: i64,
    /// `ItemClassesKey`.
    pub class: Option<i64>,
    /// Tags (item type chain plus `TagsKeys`), `None` when unknown.
    pub tags: Option<Vec<String>>,
    /// `Implicit_ModsKeys` (`Mods` rows).
    pub implicits: Vec<usize>,
}

/// What the filter knows about an item.
pub struct Context<'a> {
    /// `Mods`.
    pub mods: &'a Mods,
    /// Every `BaseItemTypes` row with the base's name.
    pub bases: &'a [BaseRow],
    /// Unique or relic (or no rarity, which PoB reads as unique).
    pub unique: bool,
    /// `InfluenceTypes` of the item's influence lines.
    pub influences: Vec<i64>,
}

/// What the filter knows about a line (the first of a multi-line run).
#[derive(Debug, Clone, Copy)]
pub struct LineContext {
    /// Section.
    pub section: Section,
    /// `{crafted}`.
    pub crafted: bool,
    /// `{exarch}`.
    pub exarch: bool,
    /// `{eater}`.
    pub eater: bool,
}

impl LineContext {
    /// The context of a modifier line.
    #[must_use]
    pub fn of(m: &Modifier) -> Self {
        Self {
            section: m.section,
            crafted: m.has("crafted"),
            exarch: m.has("exarch"),
            eater: m.has("eater"),
        }
    }
}

fn explicit_domain(row: i64, base: i64) -> bool {
    row == base
        || (row == domain::DELVE
            && [domain::ITEM, domain::BASE_JEWEL, domain::ABYSS_JEWEL].contains(&base))
        || (row == domain::UNVEILED && base == domain::ITEM)
}

/// Whether `row` fits a line of `line` on an item with base row `base`.
fn fits(row: &ModRow, base: &BaseRow, unique: bool, line: LineContext) -> bool {
    use generation as g;
    let gen_ = row.generation;
    let affix = gen_ == g::PREFIX || gen_ == g::SUFFIX;
    let own = row.domain == base.domain;
    match line.section {
        Section::Explicit if line.crafted => {
            row.domain == domain::CRAFTED
                && affix
                && (row.craft_classes.is_empty()
                    || base.class.is_some_and(|c| row.craft_classes.contains(&c)))
        }
        Section::Explicit if unique => own && gen_ == g::UNIQUE,
        Section::Explicit => {
            affix
                && explicit_domain(row.domain, base.domain)
                && !(row.essence_only && !row.essence_on(base))
        }
        Section::Implicit if line.exarch => own && gen_ == g::EXARCH,
        Section::Implicit if line.eater => own && gen_ == g::EATER,
        Section::Implicit => {
            (own && [g::UNIQUE, g::CORRUPTED, g::EXARCH, g::EATER].contains(&gen_))
                || (g::SYNTHESIS.contains(&gen_) && domain::SYNTHESIS.contains(&row.domain))
        }
        Section::Enchant => {
            (gen_ == g::ENCHANTMENT || gen_ == g::UNIQUE || g::FLASK_ENCHANTMENT.contains(&gen_))
                && (own || row.domain == domain::ITEM)
        }
        Section::Scourge => own && g::SCOURGE.contains(&gen_),
        Section::Crucible => own && g::WEAPON_TREE.contains(&gen_),
        Section::ClassRequirement => false,
    }
}

/// A row that backs a candidate and fits one of the base rows.
#[derive(Debug, Clone, Copy)]
struct Fit {
    row: usize,
    equal: bool,
}

impl Context<'_> {
    fn fitting(&self, line: LineContext, stats: &[ResolvedStat]) -> Vec<Fit> {
        let mut out = Vec::new();
        for n in self.mods.rows_for(stats) {
            let Some(row) = self.mods.row(n) else {
                continue;
            };
            let Some(equal) = row.backs(stats) else {
                continue;
            };
            if self.bases.iter().any(|b| fits(row, b, self.unique, line)) {
                out.push(Fit { row: n, equal });
            }
        }
        out
    }

    fn row(&self, fit: Fit) -> Option<&ModRow> {
        self.mods.row(fit.row)
    }

    /// Whether some fitting row spawns on one of the base rows: a positive
    /// weight on its tags, or an essence with known classes puts the row on
    /// the base's class. On an
    /// implicit line without an eldritch flag, eldritch rows do not count:
    /// they have spawn weights and `UNIQUE` / `CORRUPTED` rows do not, so
    /// counting them would favour an eldritch reading by construction.
    fn spawns(&self, line: LineContext, fits_: &[Fit]) -> bool {
        let unflagged_implicit = line.section == Section::Implicit && !line.exarch && !line.eater;
        fits_.iter().any(|f| {
            self.row(*f).is_some_and(|row| {
                let eldritch = [generation::EXARCH, generation::EATER].contains(&row.generation);
                !(unflagged_implicit && eldritch)
                    && self.bases.iter().any(|b| {
                        fits(row, b, self.unique, line)
                            && (b.tags.as_ref().is_some_and(|t| row.weight(t) > 0)
                                || row.essence_known_on(b))
                    })
            })
        })
    }

    /// Whether some fitting row is essence-only with unknown classes
    /// (essence mods spawn through `Essences`, not the base's tags; quiver
    /// mods and rows no `Essences` column names have no known classes).
    fn essence_backed(&self, fits_: &[Fit]) -> bool {
        fits_.iter().any(|f| {
            self.row(*f)
                .is_some_and(|row| row.essence_only && row.essence_classes.is_none())
        })
    }

    /// Whether some fitting row is an influence row of one of the item's
    /// influences that spawns somewhere.
    fn influence_backed(&self, fits_: &[Fit]) -> bool {
        fits_.iter().any(|f| {
            self.row(*f).is_some_and(|row| {
                row.influence.is_some_and(|i| self.influences.contains(&i))
                    && row.spawn.iter().any(|(_, w)| *w > 0)
            })
        })
    }

    /// Whether a fitting row carries one of `stats` at `(0, 0)`.
    fn unset_fit(&self, line: LineContext, stats: &[ResolvedStat]) -> bool {
        self.mods
            .unset_rows(stats)
            .any(|row| self.bases.iter().any(|b| fits(row, b, self.unique, line)))
    }

    fn base_implicit(&self, stats: &[ResolvedStat]) -> bool {
        self.bases
            .iter()
            .flat_map(|b| &b.implicits)
            .filter_map(|n| self.mods.row(*n))
            .any(|row| row.backs(stats).is_some())
    }
}

/// How the filter decided an ambiguous line.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Decision {
    /// Indexes of the candidates kept, in input order.
    pub kept: Vec<usize>,
    /// Steps that removed candidates, in order.
    pub rules: Vec<&'static str>,
    /// Why a step was skipped, or what a step removed that is worth a look.
    pub notes: BTreeSet<&'static str>,
    /// Whether any candidate was backed by a fitting row.
    pub backed_any: bool,
}

impl Decision {
    /// `resolved/<rules>` when one candidate is left, else
    /// `ambiguous/no-backed-candidate` or `ambiguous/several-backed`.
    #[must_use]
    pub fn name(&self) -> String {
        if self.kept.len() == 1 {
            format!("resolved/{}", self.rules.join("+"))
        } else if self.backed_any {
            "ambiguous/several-backed".to_owned()
        } else {
            "ambiguous/no-backed-candidate".to_owned()
        }
    }
}

fn narrow(
    kept: &mut Vec<usize>,
    rules: &mut Vec<&'static str>,
    rule: &'static str,
    keep: impl Fn(usize) -> bool,
) {
    let next: Vec<usize> = kept.iter().copied().filter(|i| keep(*i)).collect();
    if !next.is_empty() && next.len() < kept.len() {
        *kept = next;
        rules.push(rule);
    }
}

/// Filters the candidates of an ambiguous line (module docs).
#[must_use]
pub fn filter(ctx: &Context<'_>, line: LineContext, candidates: &[Candidate]) -> Decision {
    let fit: Vec<Vec<Fit>> = candidates
        .iter()
        .map(|c| ctx.fitting(line, &c.stats))
        .collect();
    let mut d = Decision {
        kept: (0..candidates.len()).collect(),
        backed_any: fit.iter().any(|f| !f.is_empty()),
        ..Decision::default()
    };
    if !d.backed_any {
        if candidates.iter().any(|c| ctx.unset_fit(line, &c.stats)) {
            d.notes.insert("row-value-unset");
        }
        return d;
    }
    let has_fit = |i: usize| fit.get(i).is_some_and(|f| !f.is_empty());
    narrow(&mut d.kept, &mut d.rules, "mods", has_fit);

    if d.kept.len() > 1 && line.section == Section::Implicit && !line.exarch && !line.eater {
        narrow(&mut d.kept, &mut d.rules, "base-implicit", |i| {
            candidates
                .get(i)
                .is_some_and(|c| ctx.base_implicit(&c.stats))
        });
    }

    if d.kept.len() > 1 {
        let spawns = |i: usize| fit.get(i).is_some_and(|f| ctx.spawns(line, f));
        if line.crafted {
            d.notes.insert("crafted-no-spawn-tags");
        } else if ctx.bases.iter().all(|b| b.tags.is_none()) {
            d.notes.insert("base-tags-unknown");
        } else if !ctx.influences.is_empty()
            && d.kept
                .iter()
                .any(|&i| !spawns(i) && fit.get(i).is_some_and(|f| ctx.influence_backed(f)))
        {
            d.notes.insert("influence-tags-unknown");
        } else if d
            .kept
            .iter()
            .any(|&i| !spawns(i) && fit.get(i).is_some_and(|f| ctx.essence_backed(f)))
        {
            d.notes.insert("essence-tags-unknown");
        } else {
            narrow(&mut d.kept, &mut d.rules, "spawn-tags", spawns);
        }
    }

    if d.kept.len() > 1 {
        narrow(&mut d.kept, &mut d.rules, "values", |i| {
            let (Some(c), Some(f)) = (candidates.get(i), fit.get(i)) else {
                return false;
            };
            f.iter()
                .any(|x| ctx.row(*x).is_some_and(|r| r.values_overlap(&c.stats)))
        });
    }
    d
}

/// `mod-backed` of a resolved line: `Ok((true, id))` backed by an equal
/// row, `Ok((false, id))` only by hybrid rows (`id`: the first such row's
/// `Id`), or why it is not backed: `row-value-unset` (a fitting row carries
/// a stat of the line at `(0, 0)`, the value set outside the row; checked
/// first), `no-row-with-stats` (no row backs the stats), `no-fitting-row`
/// (rows back them only in other domains or generation types),
/// `values-outside` (fitting rows, no value overlap).
///
/// # Errors
/// The reason, as above.
pub fn backing<'m>(
    ctx: &Context<'m>,
    line: LineContext,
    stats: &[ResolvedStat],
) -> Result<(bool, &'m str), &'static str> {
    let backed = backing_row(ctx, line, stats);
    if backed.is_err() && ctx.unset_fit(line, stats) {
        return Err("row-value-unset");
    }
    backed
}

fn backing_row<'m>(
    ctx: &Context<'m>,
    line: LineContext,
    stats: &[ResolvedStat],
) -> Result<(bool, &'m str), &'static str> {
    let any_row = ctx
        .mods
        .rows_for(stats)
        .into_iter()
        .filter_map(|n| ctx.mods.row(n))
        .any(|r| r.backs(stats).is_some());
    if !any_row {
        return Err("no-row-with-stats");
    }
    let fits_ = ctx.fitting(line, stats);
    if fits_.is_empty() {
        return Err("no-fitting-row");
    }
    let mods: &'m Mods = ctx.mods;
    fits_
        .iter()
        .filter_map(|f| mods.row(f.row).map(|r| (f.equal, r)))
        .filter(|(_, r)| r.values_overlap(stats))
        // Equal rows first, then row order.
        .min_by_key(|(equal, _)| !equal)
        .map(|(equal, r)| (equal, r.id.as_str()))
        .ok_or("values-outside")
}

#[cfg(test)]
mod tests;
