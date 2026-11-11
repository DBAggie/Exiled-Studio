//! Loads the `Mods` filter's rows from the extract (Spike B, PR B4a; plan
//! "Ambiguity" and step 4a). Since P2-2c the filter itself is
//! `loom_parser::import::mods`, and the PoE1 rules below are data in
//! `crate::poe1` (`item_rules().mods`); this module reads the tables. The
//! rules and the evidence for each step stay documented here.
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
//!    backed, no tie-break runs (`no-backed-candidate`); if some fitting
//!    row carries a candidate's stat at `(0, 0)` (note `row-value-unset`:
//!    the value is set outside the row, as for anoints), `unset-fit` keeps
//!    the candidates with such a row, then `numbered-stat` runs (step 7).
//!    Backed candidates never reach `numbered-stat`: several backed rows are
//!    a real ambiguity, not an unknown slot.
//! 2. `base-implicit` (implicit, not eldritch): candidates backed by one of
//!    the base's `Implicit_ModsKeys` rows.
//! 3. `local-stat` (PR B5a; explicit lines of non-unique items): when the
//!    kept candidates are a local and a global reading (some carry a local
//!    stat, `Stats.IsLocal` or `IsWeaponLocal`, the others none), a base with
//!    the property keeps the local reading and any other base the global
//!    one. The property is the base tag `weapon` for `IsWeaponLocal` stats
//!    and `armour` for the other local stats (a local stat changes the
//!    item's own weapon or defence values, which only those bases have).
//!    Skipped, with a note, when a base row's tags are unknown
//!    (`local-stat-tags-unknown`), when the item's base rows disagree on the
//!    carrier tag (`local-stat-bases-differ`), when a candidate mixes weapon
//!    and item locality (`local-stat-mixed`), and when a global base would
//!    drop a local reading that a fitting row spawns on the base
//!    (`local-stat-spawns`; flask- and jewel-local stats). Measured on PoE1 26530219:
//!    every line the later steps had already resolved between such a pair
//!    (107) gets the same reading, except one PoB-crafted Crystal Belt line
//!    the `values` step had read as local ES (belts have no ES property).
//! 4. `spawn-tags` (tie-breaker, reported): candidates backed by a fitting
//!    row with a positive spawn weight on the base's tags (the first row tag
//!    the base has gives the weight). Skipped, with a note, for `{crafted}`
//!    lines (bench crafts do not spawn), when the base's tags are unknown,
//!    when the item has an influence and a candidate that does not spawn is
//!    backed by an influence row of that influence (the item's influence
//!    tags need `InfluenceTags`, not extracted), and when a candidate that
//!    does not spawn is backed by an essence-only row whose classes are
//!    unknown. A row an essence with known classes puts on the base's class
//!    counts as spawning, essence-only or not (`Essences` also names
//!    ordinary affixes, such as `ColdResist7` on armour): an essence can put
//!    it on the item, so the base can carry it whatever its tag weight. Eldritch rows do not count on implicit lines
//!    without an eldritch flag. A legacy essence row (essence-only, no
//!    `Essences` column names it) whose `ModType` has rows that are not
//!    essence-only (same domain and generation type) counts as spawning
//!    where those rows spawn, and nowhere
//!    when they spawn nowhere (PR B5a: `ChanceToDodgeEssence4`..`7` share
//!    `ChanceToSuppressSpellsOld` with rows that no longer spawn); only a
//!    legacy essence row whose `ModType` has no other row keeps the guard.
//! 5. `values` (tie-breaker, reported): candidates backed at the values
//!    level.
//! 6. `spawn-values` (PR B5a; tie-breaker, reported; only when
//!    `spawn-tags` ran): candidates backed by one fitting row that both
//!    spawns on the base and holds the line's values (a candidate that
//!    spawns only through a row whose range misses the value, and holds the
//!    value only on rows that do not spawn, is dropped).
//! 7. `numbered-stat` (Spike B step 4d, maintainer decision 2026-11-10;
//!    only after `unset-fit`): candidates whose stats are equal up to a numbered suffix (`x`, `x_2`
//!    .. `x_9`; anoints: `mod_granted_passive_hash` .. `_5`) are one
//!    reading; the one with the lowest numbers is kept, the others are
//!    dropped as alternatives (note `numbered-stat-equivalents`). No game
//!    data says which of the five an anoint uses (`BlightCraftingResults`
//!    names only the passive), and PoB does not pick one either (`allocates
//!    (.+)` gives one `GrantedPassive` list mod, `v2.67.2:
//!    src/Modules/ModParser.lua`).
//!
//! One candidate left: the line is resolved, with the steps that narrowed it.

use std::collections::{BTreeMap, BTreeSet};

use loom_domain::ModSection;
pub use loom_parser::import::mods::{
    Context, LineContext, Locality, ModRow, Mods, backing, filter,
};
use loom_serialization::legacy::pob_item::Section;
use serde_json::Value;

use crate::extract::{Read, Table};
use crate::poe1::{self, generation};

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
    for (column, class_ids) in ESSENCE_COLUMNS {
        let col = essences.column(column)?;
        let rows: Vec<i64> = class_ids
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

/// Local stats of `Stats` (`IsWeaponLocal`, else `IsLocal`).
///
/// # Errors
/// A message when the table or a column is missing.
fn locality(read: Read<'_>) -> Result<BTreeMap<String, Locality>, String> {
    let t = Table::require(read, "Stats")?;
    let (id, local, weapon) = (
        t.column("Id")?,
        t.column("IsLocal")?,
        t.column("IsWeaponLocal")?,
    );
    let flag = |r: &[Value], c: usize| r.get(c).and_then(Value::as_bool).unwrap_or(false);
    Ok(t.rows
        .iter()
        .filter_map(|r| {
            let kind = if flag(r, weapon) {
                Locality::Weapon
            } else if flag(r, local) {
                Locality::Item
            } else {
                return None;
            };
            Some((Table::text(r, id)?.to_owned(), kind))
        })
        .collect())
}

/// Reads `Mods`, `Stats`, and `Tags`, and `Essences` with `ItemClasses`
/// when the extract has them. Row `n` of the result is row `n`
/// of `Mods` (`Implicit_ModsKeys` index it).
///
/// # Errors
/// A message when a table or column is missing.
pub fn load(read: Read<'_>) -> Result<Mods, String> {
    let stats = ids(read, "Stats")?;
    let locality = locality(read)?;
    let tags = tag_ids(read)?;
    let t = Table::require(read, "Mods")?;
    let col = |n: &str| t.column(n);
    let (id, dom, gen_) = (col("Id")?, col("Domain")?, col("GenerationType")?);
    let mod_type = col("ModTypeKey")?;
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
            influence: Table::int(r, influence).filter(|v| *v != poe1::NO_INFLUENCE),
            craft_classes: ints(r, craft),
            essence_only: r.get(essence).and_then(Value::as_bool).unwrap_or(false),
            essence_classes: None,
            mod_type: Table::int(r, mod_type),
            type_spawn: None,
        });
    }
    if let Some(classes) = essence_classes(read)? {
        for (n, list) in classes {
            let row = rows
                .get_mut(n)
                .ok_or_else(|| format!("Essences: Mods row {n} is not in Mods"))?;
            row.essence_classes = Some(list.into_iter().collect());
        }
    }
    let mut mods = Mods::from_rows(rows);
    mods.bad_spawn_rows = bad_spawn_rows;
    mods.locality = locality;
    Ok(mods)
}

/// `(rows, eldritch)`: rows carrying `stat` (at a value other than
/// `(0, 0)`), and how many of them are eldritch implicits
/// (`EXARCH_IMPLICIT`, `EATER_IMPLICIT`). Evidence for a wrapper's pairing
/// (B5c).
#[must_use]
pub fn eldritch_share(mods: &Mods, stat: &str) -> (usize, usize) {
    let eldritch = [generation::EXARCH, generation::EATER];
    let (mut rows, mut found) = (0, 0);
    for r in mods.rows_with_stat(stat) {
        rows += 1;
        found += usize::from(eldritch.contains(&r.generation));
    }
    (rows, found)
}

/// The `InfluenceTypes` value of an influence line (`<display> Item`),
/// through the PoE1 rules.
#[cfg(test)]
fn influence_type(line: &str) -> Option<i64> {
    let display = line.trim().strip_suffix(" Item")?;
    poe1::item_rules()
        .mods
        .influence_value(&loom_domain::InfluenceId::new(display))
}

/// PoB's section of a domain section (the inverse of
/// `pob_item::domain_section`); `None` for an `Other` name PoB does not
/// write.
#[must_use]
pub fn pob_section(section: &ModSection) -> Option<Section> {
    match section {
        ModSection::Enchant => Some(Section::Enchant),
        ModSection::Implicit => Some(Section::Implicit),
        ModSection::Explicit => Some(Section::Explicit),
        ModSection::Other(name) => [
            Section::Scourge,
            Section::Crucible,
            Section::ClassRequirement,
        ]
        .into_iter()
        .find(|s| s.name() == name),
    }
}

/// A domain section's name as a static string, for the report; `other`
/// for a name PoB does not write.
#[must_use]
pub fn section_name(section: &ModSection) -> &'static str {
    pob_section(section).map_or("other", Section::name)
}

#[cfg(test)]
mod tests;
