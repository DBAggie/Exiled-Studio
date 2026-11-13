//! `data-parity <bundle-dir> <pob-export> --rules <file> --out <dir>`
//! (Phase 3 plan, step 2): the values a calculation reads, in the bundle
//! against PoB v2.67.2's data, before any calc comparison (decision 5).
//!
//! PoB's side is the export `tools/oracle/data/pob_data.lua` writes on the
//! pinned oracle runtime (maintainer-run; outside the oracle adapter and
//! its protocol). The comparison ([`compare`]) puts each bundle value into
//! PoB's form with the conversion PoB's exporter made, and compares
//! exactly. Every difference gets a category from the rules file
//! ([`rules`]: `patch-delta`, `pob-derivation`, `representation`, each
//! with its evidence) or is `uncategorized`.
//!
//! Writes `summary.json` (counts, inputs, rules), `report.json` (every
//! difference with its category), and `overlay.json`: PoB's value for each
//! difference of a category that changes a value (`patch-delta`,
//! `pob-derivation`), which `calc-run --data-overlay` substitutes (step 5)
//! so a `data-version` divergence is accepted only when the calc matches
//! with PoB's data. `representation` differences are the same game value,
//! so they are not in the overlay.
//!
//! Exit 0 when every difference has a category and every rule matched one;
//! 1 otherwise; 2 on a usage error; 3 when an input does not read.

pub mod compare;
pub mod gem_effects;
pub mod pob;
pub mod rules;

#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, BTreeSet};

use loom_data::bundle::Bundle;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use self::compare::{Difference, TRANSFORMS, TableCounts, Transform};
use self::rules::{Category, Rules};
use crate::resolve_corpus::Tool;

/// The tables compared (also the names rules use).
pub const TABLES: [&str; 12] = [
    "skills",
    "gems",
    "bases",
    "characters",
    "character_constants",
    "monster_constants",
    "game_constants",
    "monster_levels",
    "passives",
    "mastery_effects",
    "pantheons",
    "gem_effects",
];

/// Summary and overlay format.
pub const FORMAT: u32 = 1;

/// The count name of differences no rule covers.
pub const UNCATEGORIZED: &str = "uncategorized";

/// What the export and the comparison leave out on purpose, with the
/// reason, for the report.
pub const NOT_COMPARED: [(&str, &str, &str); 9] = [
    (
        "skills",
        "level requirement, Vaal soul cost and stored uses, soul gain prevention",
        "not carried by the bundle (P3-1a, context file)",
    ),
    (
        "skills",
        "stat interpolation kinds",
        "kept raw in the bundle until P3-9a gives them a meaning with evidence",
    ),
    (
        "skills",
        "PvP damage multiplier, descriptions, parts, base flags, stat maps",
        "PvP is not calculated; the rest is PoB's own model, not game data",
    ),
    (
        "skills",
        "support constraints: allowed, added, excluded, and minion types; supports gems only; cannot be supported; weapon restrictions",
        "not exported in this step; P3-9a (skill model) compares them before support matching is calculated",
    ),
    (
        "skills",
        "additional stat sets (35 PoE1 effects have more than one)",
        "PoB v2.67.2 exports the main stat set only (skills.lua:256); listed per effect under bundle_only",
    ),
    (
        "*",
        "bundle rows and skill levels PoB does not have",
        "no PoB build reads them, so they cannot differ in an oracle comparison; listed in report.json bundle_only, not gating",
    ),
    (
        "gems",
        "names, natural maximum level",
        "display data; the calc reads gem levels from the build",
    ),
    (
        "passives",
        "tree geometry (groups, orbits, connections) and class start nodes",
        "P3-3 decides the tree version label with evidence; the slice reads node stats",
    ),
    (
        "monster_levels",
        "experience, difficulty, and the PoE2-only columns",
        "not carried by the bundle (P3-1c)",
    ),
];

/// One difference with its category.
#[derive(Debug, Clone, Serialize)]
pub struct Classified {
    /// The difference.
    #[serde(flatten)]
    pub difference: Difference,
    /// The category; `null` when uncategorized.
    pub category: Option<Category>,
    /// The rule that gave it.
    pub rule: Option<String>,
}

/// Recorded inputs.
#[derive(Debug, Clone, Serialize)]
pub struct Inputs {
    /// `poe1` or `poe2`.
    pub game: String,
    /// The bundle's Steam build.
    pub steam_build: Option<u64>,
    /// SHA-256 of the bundle's `bundle.json`.
    pub bundle_manifest_sha256: String,
    /// The bundle's data schema.
    pub data_schema: u32,
    /// SHA-256 of the PoB export.
    pub pob_export_sha256: String,
    /// PoB's version, as the export records it.
    pub pob_version: Option<String>,
    /// The tree version the export read.
    pub tree_version: String,
    /// The runtime the export ran on (LuaJIT's `jit.version`).
    pub pob_runtime: Option<String>,
    /// SHA-256 of the rules file.
    pub rules_sha256: String,
}

/// A rule and what it covered.
#[derive(Debug, Clone, Serialize)]
pub struct RuleUse {
    /// The rule.
    pub id: String,
    /// Its category.
    pub category: Category,
    /// Differences it covered.
    pub matched: usize,
    /// The count the rule expects.
    pub expected: usize,
    /// `patch-delta`: whether the cited notes name the change.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub named_in_notes: Option<bool>,
    /// Key patterns that matched no difference.
    pub stale_keys: Vec<String>,
}

/// `summary.json`.
#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    /// [`FORMAT`].
    pub format: u32,
    /// The tool.
    pub tool: Tool,
    /// The inputs.
    pub inputs: Inputs,
    /// Counts by table.
    pub tables: BTreeMap<&'static str, TableCounts>,
    /// Differences by category (`uncategorized` included).
    pub categories: BTreeMap<String, usize>,
    /// Each rule with its count, in file order.
    pub rules: Vec<RuleUse>,
    /// Rules that covered another number of differences than their count,
    /// or have a key pattern that matched nothing.
    pub failed_rules: Vec<String>,
    /// Bundle rows, levels, and stat sets PoB does not have, by table.
    pub bundle_only: BTreeMap<&'static str, usize>,
    /// Rows written to the overlay.
    pub overlay_rows: usize,
    /// The conversions from the bundle's form to PoB's.
    pub transforms: Vec<Transform>,
    /// What is left out on purpose.
    pub not_compared: Vec<NotCompared>,
    /// Whether the run passes: no uncategorized difference, no failed rule.
    pub pass: bool,
}

/// A field left out, with the reason.
#[derive(Debug, Clone, Serialize)]
pub struct NotCompared {
    /// The table.
    pub table: &'static str,
    /// The fields.
    pub fields: &'static str,
    /// Why.
    pub reason: &'static str,
}

/// `report.json`.
#[derive(Debug, Clone, Serialize)]
pub struct Report {
    /// [`FORMAT`].
    pub format: u32,
    /// Every difference, by table, key, and field.
    pub differences: Vec<Classified>,
    /// Every bundle row, level (`<key>:level.<n>`), and additional stat
    /// set (`<key>:stat_set.<n>`) PoB does not have.
    pub bundle_only: Vec<compare::BundleOnly>,
}

/// One overlay row: PoB's value for a bundle row, in PoB's form (the
/// transform named in the summary undoes the form).
#[derive(Debug, Clone, Serialize)]
pub struct OverlayRow {
    /// The table.
    pub table: &'static str,
    /// The row key.
    pub key: String,
    /// The field.
    pub field: String,
    /// PoB's value.
    pub pob: Value,
    /// The bundle's value in PoB's form.
    pub bundle: Value,
    /// Why they differ.
    pub category: Category,
    /// The rule.
    pub rule: String,
}

/// `overlay.json`.
#[derive(Debug, Clone, Serialize)]
pub struct Overlay {
    /// [`FORMAT`].
    pub format: u32,
    /// The inputs it was made from (the calc checks them before use).
    pub inputs: Inputs,
    /// The rows.
    pub rows: Vec<OverlayRow>,
}

/// The three outputs.
#[derive(Debug)]
pub struct Outputs {
    /// `summary.json`.
    pub summary: Summary,
    /// `report.json`.
    pub report: Report,
    /// `overlay.json`.
    pub overlay: Overlay,
}

/// SHA-256 as hex.
pub fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// The raw inputs of a run.
pub struct Request<'a> {
    /// The loaded, validated bundle.
    pub bundle: &'a Bundle,
    /// `bundle.json`'s bytes (hashed).
    pub bundle_manifest: &'a [u8],
    /// The PoB export's bytes.
    pub export: &'a [u8],
    /// The rules file's bytes.
    pub rules: &'a [u8],
    /// The scope passive text renders with ([`compare::PASSIVE_SCOPE`]).
    pub passive_scope: &'a str,
    /// The bosses to compare (`loom_calc::poe1::enemy::BOSSES` in a run).
    pub bosses: &'a [loom_calc::poe1::enemy::Boss],
    /// The tool record.
    pub tool: Tool,
}

/// Runs the comparison.
///
/// # Errors
/// A message when an input does not read: the export or rules file, a
/// rules file for another game, Steam build, or PoB version, or a bundle
/// without the calc tables.
pub fn run(request: &Request<'_>) -> Result<Outputs, String> {
    let bundle = request.bundle;
    let game = bundle.manifest.game.clone();
    if game != "poe1" {
        return Err(format!(
            "data-parity compares PoE1 bundles (PoB v2.67.2 is the PoE1 oracle); this bundle is {game}"
        ));
    }
    let export = pob::parse(request.export)?;
    let rules = Rules::parse(request.rules, &game)?;
    // The evidence holds for one pair of inputs only.
    if bundle.manifest.source.steam_build != Some(rules.steam_build) {
        return Err(format!(
            "the rules file is for Steam build {}, the bundle is from {}",
            rules.steam_build,
            bundle
                .manifest
                .source
                .steam_build
                .map_or_else(|| "no Steam build".to_owned(), |b| b.to_string())
        ));
    }
    if export.pob.version.as_deref() != Some(rules.pob_version.as_str()) {
        return Err(format!(
            "the rules file is for PoB {}, the export is from {}",
            rules.pob_version,
            export
                .pob
                .version
                .as_deref()
                .unwrap_or("an unknown version")
        ));
    }
    let comparison = compare::compare(bundle, &export, request.passive_scope, request.bosses)?;
    let inputs = Inputs {
        game,
        steam_build: bundle.manifest.source.steam_build,
        bundle_manifest_sha256: sha256(request.bundle_manifest),
        data_schema: bundle.manifest.data_schema,
        pob_export_sha256: sha256(request.export),
        pob_version: export.pob.version.clone(),
        pob_runtime: export.pob.runtime.clone(),
        tree_version: export.tree_version.clone(),
        rules_sha256: sha256(request.rules),
    };

    let mut matched: BTreeMap<&str, usize> = BTreeMap::new();
    let mut keys_used: BTreeSet<(&str, &str)> = BTreeSet::new();
    let mut categories: BTreeMap<String, usize> = [
        Category::PatchDelta,
        Category::PobDerivation,
        Category::Representation,
        Category::NotCarried,
    ]
    .into_iter()
    .map(Category::name)
    .chain([UNCATEGORIZED])
    .map(|c| (c.to_owned(), 0))
    .collect();
    let mut differences = Vec::with_capacity(comparison.differences.len());
    let mut rows = Vec::new();
    for difference in comparison.differences {
        let rule = rules.classify(difference.table, &difference.key, &difference.field);
        let category = rule.map(|r| r.category);
        let name = category.map_or(UNCATEGORIZED, Category::name);
        *categories.entry(name.to_owned()).or_default() += 1;
        if let Some(rule) = rule {
            *matched.entry(rule.id.as_str()).or_default() += 1;
            for pattern in &rule.keys {
                if rules::glob(pattern, &difference.key) {
                    keys_used.insert((rule.id.as_str(), pattern.as_str()));
                }
            }
            if rule.category.overlays() {
                rows.push(OverlayRow {
                    table: difference.table,
                    key: difference.key.clone(),
                    field: difference.field.clone(),
                    pob: difference.pob.clone(),
                    bundle: difference.bundle.clone(),
                    category: rule.category,
                    rule: rule.id.clone(),
                });
            }
        }
        differences.push(Classified {
            difference,
            category,
            rule: rule.map(|r| r.id.clone()),
        });
    }
    let rule_uses: Vec<RuleUse> = rules
        .rules
        .iter()
        .map(|r| RuleUse {
            id: r.id.clone(),
            category: r.category,
            matched: matched.get(r.id.as_str()).copied().unwrap_or(0),
            expected: r.count,
            named_in_notes: r.named_in_notes,
            stale_keys: r
                .keys
                .iter()
                .filter(|k| !keys_used.contains(&(r.id.as_str(), k.as_str())))
                .cloned()
                .collect(),
        })
        .collect();
    let failed_rules: Vec<String> = rule_uses
        .iter()
        .filter(|r| r.matched != r.expected || !r.stale_keys.is_empty())
        .map(|r| r.id.clone())
        .collect();
    let pass = categories.get(UNCATEGORIZED).copied().unwrap_or(0) == 0 && failed_rules.is_empty();
    let mut bundle_only: BTreeMap<&'static str, usize> = BTreeMap::new();
    for row in &comparison.bundle_only {
        *bundle_only.entry(row.table).or_default() += 1;
    }
    let summary = Summary {
        format: FORMAT,
        tool: request.tool.clone(),
        inputs: inputs.clone(),
        tables: comparison.tables,
        categories,
        rules: rule_uses,
        failed_rules,
        bundle_only,
        overlay_rows: rows.len(),
        transforms: TRANSFORMS.to_vec(),
        not_compared: NOT_COMPARED
            .iter()
            .map(|&(table, fields, reason)| NotCompared {
                table,
                fields,
                reason,
            })
            .collect(),
        pass,
    };
    Ok(Outputs {
        summary,
        report: Report {
            format: FORMAT,
            differences,
            bundle_only: comparison.bundle_only,
        },
        overlay: Overlay {
            format: FORMAT,
            inputs,
            rows,
        },
    })
}
