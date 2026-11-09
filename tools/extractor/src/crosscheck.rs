//! `cross-check` (Spike A, step 6): compares the extractor's output with a
//! Path of Building data export and puts every difference into exactly one
//! category from a fixed list, naming the rule that put it there.
//!
//! Five table families are compared, each like for like:
//!
//! | Family | PoB keys | Our keys |
//! |---|---|---|
//! | `mods` | `src/Data/Mod*.lua` entry keys | `Mods.Id` |
//! | `bases` | `src/Data/Bases/*.lua` names | `BaseItemTypes.Name` (PoB's name clean-up applied) |
//! | `skills` | `src/Data/Skills/*.lua` keys | `GrantedEffects.Id` |
//! | `stats` | stat IDs in `src/Data/StatDescriptions/**/*.lua` | `Stats.Id` |
//! | `passives` | node IDs in `src/TreeData/<newest>/tree.lua` | `PassiveSkills.PassiveSkillGraphId` |
//!
//! Gaps go both ways: PoB keys missing from our output (`pob_missing`) and
//! our keys PoB does not have (`ours_extra`). Each gap gets the first rule
//! that matches; a gap no rule explains is `uncategorized`. Rules reproduce
//! PoB's export behaviour from its own spec files and scripts. A rule taken
//! from a script is tied to a snippet of that script; if the snippet is
//! missing (PoB changed the script), the rule is disabled and listed, so an
//! outdated rule never explains a gap.
//!
//! The gate (ACTION-PLAN, Spike A): per family, no uncategorized gap, no PoB
//! line that looked like a key but did not parse, and `patch-delta` gaps at
//! most [`PATCH_DELTA_PERCENT`] of PoB's keys.

pub mod ours;
pub mod pob;

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use serde_json::Value;

use crate::schema::{Game, Schema};
use ours::{Description, StatDescFile, Table};

/// Format of the report.
pub const REPORT_FORMAT: u32 = 1;

/// Largest share of PoB keys per family that may be `patch-delta`
/// (maintainer decision 2026-11-08).
pub const PATCH_DELTA_PERCENT: usize = 2;

/// Tables read from the extract output, as `tables/<Name>.json`.
pub const TABLES: [&str; 7] = [
    "BaseItemTypes",
    "BuffTemplates",
    "GrantedEffects",
    "ModFamily",
    "Mods",
    "PassiveSkills",
    "Stats",
];

/// Folder of the stat-description files in the extract output.
pub const STAT_DESCRIPTIONS_DIR: &str = "stat_descriptions";

/// PoB folders read: path, file extension, and whether sub-folders are read.
/// The tree file comes on top ([`tree_path`]).
pub const POB_DIRS: [(&str, &str, bool); 8] = [
    ("src/Export", ".lua", false),
    ("src/Data", ".lua", false),
    ("src/Data/Bases", ".lua", false),
    ("src/Data/Skills", ".lua", false),
    ("src/Data/StatDescriptions", ".lua", true),
    ("src/Export/Bases", ".txt", false),
    ("src/Export/Skills", ".txt", false),
    ("src/Export/Scripts", ".lua", false),
];

const BASES_SCRIPT: &str = "src/Export/Scripts/bases.lua";
const MODS_SCRIPT: &str = "src/Export/Scripts/mods.lua";
const STATDESC_SCRIPT: &str = "src/Export/statdesc.lua";
const SKILLS_SCRIPT: &str = "src/Export/Scripts/skills.lua";

/// Mod files whose keys are not mod IDs, with the reason. They are listed in
/// the report with how many of their keys happen to equal a `Mods.Id`.
const NON_MOD_ID_FILES: [(&str, &str); 4] = [
    ("ModCache.lua", "parser cache keyed by mod text"),
    (
        "ModFoulbornMap.lua",
        "keyed by unique item name (Export/Scripts/mapUniqueToFoulborn.lua)",
    ),
    (
        "ModRunes.lua",
        "keyed by rune or soul-core base name (Export/Scripts/soulcores.lua)",
    ),
    (
        "ModScalability.lua",
        "keyed by stat text (Export/Scripts/modScalability.lua)",
    ),
];

/// The category of a gap (plan step 6; fixed list).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Category {
    /// The file or row is not in the installed game data.
    NotInIndex,
    /// PoB's export leaves it out by a rule the cross-check reproduces.
    PobFiltered,
    /// PoB made it up; no game row has this key.
    PobGenerated,
    /// Differs between PoB's export patch and the installed patch.
    PatchDelta,
    /// The table did not decode cleanly (manifest status not `ok`).
    SchemaMismatch,
    /// No rule explains it.
    Uncategorized,
}

/// Errors from the cross-check. Each variant has a stable [`code`](Self::code).
#[derive(Debug, thiserror::Error)]
pub enum CrossCheckError {
    /// An input file is not valid JSON of the expected shape.
    #[error("{path}: {source}")]
    Json {
        /// The file.
        path: String,
        /// The parse error.
        #[source]
        source: serde_json::Error,
    },
    /// A file the cross-check needs is not in the inputs.
    #[error("missing input file {0}")]
    MissingFile(String),
    /// A table lacks a column the cross-check needs.
    #[error("table {table} has no column {column}")]
    MissingColumn {
        /// Table name.
        table: String,
        /// Column name.
        column: String,
    },
    /// The extract output is for the other game.
    #[error("the extract output is for {found}, not {expected}")]
    GameMismatch {
        /// Game asked for.
        expected: &'static str,
        /// Game in the manifest.
        found: String,
    },
    /// A PoB file is not in a shape the cross-check can reproduce.
    #[error("{path}: {what}")]
    UnsupportedPob {
        /// The file.
        path: String,
        /// What is wrong.
        what: String,
    },
    /// Two stat-description output files map to the same PoB file name.
    #[error("stat-description files {first} and {second} both map to PoB name {stem}")]
    NameCollision {
        /// PoB-style name.
        stem: String,
        /// First file.
        first: String,
        /// Second file.
        second: String,
    },
}

impl CrossCheckError {
    /// Stable error code (ADR-014).
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Json { .. } => "EXTRACT_XCHECK_JSON",
            Self::MissingFile(_) => "EXTRACT_XCHECK_MISSING_FILE",
            Self::MissingColumn { .. } => "EXTRACT_XCHECK_MISSING_COLUMN",
            Self::GameMismatch { .. } => "EXTRACT_XCHECK_GAME_MISMATCH",
            Self::UnsupportedPob { .. } => "EXTRACT_XCHECK_UNSUPPORTED_POB",
            Self::NameCollision { .. } => "EXTRACT_XCHECK_NAME_COLLISION",
        }
    }
}

/// The cross-check build and the PoB checkout, recorded in the report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Provenance {
    /// SHA-256 of the extract output's `manifest.json`.
    pub extract_manifest_sha256: String,
    /// Commit of the PoB checkout, if found.
    pub pob_commit: Option<String>,
    /// Git commit of the cross-check build, if known.
    pub tool_commit: Option<String>,
    /// Whether the cross-check's tree had changes, if known.
    pub tool_dirty: Option<bool>,
    /// Crate version.
    pub tool_version: String,
}

/// Everything the cross-check reads, already in memory.
#[derive(Debug, Clone, Copy)]
pub struct Inputs<'a> {
    /// The game.
    pub game: Game,
    /// The schema (for enumeration names).
    pub schema: &'a Schema,
    /// Extract output files by path relative to the output folder
    /// (`manifest.json`, `tables/Mods.json`, `stat_descriptions/...`).
    pub ours: &'a BTreeMap<String, Vec<u8>>,
    /// PoB files by path relative to the checkout, with `/` separators.
    pub pob: &'a BTreeMap<String, String>,
    /// Tree version folder read (see [`newest_tree_version`]).
    pub tree_version: &'a str,
}

/// Path of the tree file for a version folder.
#[must_use]
pub fn tree_path(version: &str) -> String {
    format!("src/TreeData/{version}/tree.lua")
}

/// The newest plain tree version among PoB's `src/TreeData` folder names:
/// `<major>_<minor>` only (no `_ruthless` or `_alternate` variants),
/// compared as numbers.
#[must_use]
pub fn newest_tree_version<'a>(names: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    names
        .into_iter()
        .filter_map(|name| {
            let (major, minor) = name.split_once('_')?;
            let major: u32 = major.parse().ok()?;
            let minor: u32 = minor.parse().ok()?;
            Some(((major, minor), name))
        })
        .max()
        .map(|(_, name)| name)
}

/// Keys of one gap rule and why they are there.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuleResult {
    /// Counts per detail (for example per mod domain and generation type);
    /// empty when the rule has no details.
    pub breakdown: BTreeMap<String, usize>,
    /// Number of keys.
    pub count: usize,
    /// What the rule checks and where in PoB it comes from.
    pub evidence: String,
    /// The keys, sorted (left out of the summary for `pob-filtered`).
    pub keys: Vec<String>,
}

/// The gaps in one direction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Gaps {
    /// Gaps by category, then by rule name.
    pub by_category: BTreeMap<Category, BTreeMap<String, RuleResult>>,
    /// Number of gaps.
    pub total: usize,
}

impl Gaps {
    fn count(&self, category: Category) -> usize {
        self.by_category
            .get(&category)
            .map_or(0, |rules| rules.values().map(|r| r.count).sum())
    }
}

/// A PoB file left out of the comparison.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Excluded {
    /// Keys in the file.
    pub keys: usize,
    /// Of those, keys that equal one of our keys (evidence of the key space).
    pub keys_matching_ours: usize,
    /// Why the file is not compared.
    pub reason: &'static str,
}

/// A PoB line that looked like a key but did not parse.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct UnparsedLine {
    /// The file.
    pub file: String,
    /// 1-based line number.
    pub line: usize,
    /// The line.
    pub text: String,
}

/// Gate result of one family.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FamilyGate {
    /// Every condition holds.
    pub met: bool,
    /// `patch-delta` gaps in both directions.
    pub patch_delta: usize,
    /// Most `patch-delta` gaps allowed ([`PATCH_DELTA_PERCENT`] of PoB keys, rounded down).
    pub patch_delta_limit: usize,
    /// Uncategorized gaps in both directions.
    pub uncategorized: usize,
    /// PoB lines that did not parse.
    pub unparsed: usize,
}

/// The comparison of one table family.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FamilyReport {
    /// What is compared with what.
    pub compared: &'static str,
    /// Rules switched off because their PoB snippet was not found.
    pub disabled_rules: Vec<String>,
    /// PoB files not compared, by path.
    pub excluded_pob_files: BTreeMap<String, Excluded>,
    /// Gate result.
    pub gate: FamilyGate,
    /// Keys in both.
    pub matched: usize,
    /// Distinct keys in our output.
    pub our_keys: usize,
    /// Our keys PoB does not have.
    pub ours_extra: Gaps,
    /// Distinct PoB keys.
    pub pob_keys: usize,
    /// PoB keys missing from our output.
    pub pob_missing: Gaps,
    /// Keys read per PoB file (a key may be in several files).
    pub pob_sources: BTreeMap<String, usize>,
    /// PoB lines that looked like keys but did not parse.
    pub unparsed: Vec<UnparsedLine>,
}

/// Inputs recorded in the report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReportInputs {
    /// Extractor commit recorded in the extract manifest.
    pub extract_commit: Option<String>,
    /// Extractor dirty flag recorded in the extract manifest.
    pub extract_dirty: Option<bool>,
    /// Builds and checkouts.
    pub provenance: Provenance,
    /// PoB tree version folder read.
    pub pob_tree_version: String,
    /// Steam build ID recorded in the extract manifest.
    pub steam_build_id: Option<String>,
}

/// Overall gate result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Gate {
    /// Every family meets the gate.
    pub met: bool,
    /// Why not, one line per failed condition.
    pub not_met: Vec<String>,
}

/// The cross-check report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Report {
    /// Per table family.
    pub families: BTreeMap<&'static str, FamilyReport>,
    /// [`REPORT_FORMAT`].
    pub format: u32,
    /// `poe1` or `poe2`.
    pub game: &'static str,
    /// Gate verdict.
    pub gate: Gate,
    /// Inputs.
    pub inputs: ReportInputs,
}

impl Report {
    /// The report as pretty JSON with a final newline.
    ///
    /// # Errors
    /// If serialisation fails.
    pub fn to_json(&self) -> Result<Vec<u8>, serde_json::Error> {
        let mut json = serde_json::to_vec_pretty(self)?;
        json.push(b'\n');
        Ok(json)
    }

    /// The report without the key lists of `pob-filtered` rules (counts
    /// stay), small enough to commit.
    #[must_use]
    pub fn summary(&self) -> Self {
        let mut summary = self.clone();
        for family in summary.families.values_mut() {
            for gaps in [&mut family.pob_missing, &mut family.ours_extra] {
                if let Some(rules) = gaps.by_category.get_mut(&Category::PobFiltered) {
                    for rule in rules.values_mut() {
                        rule.keys.clear();
                    }
                }
            }
        }
        summary
    }
}

const fn game_name(game: Game) -> &'static str {
    match game {
        Game::Poe1 => "poe1",
        Game::Poe2 => "poe2",
    }
}

/// A rule's test: `Some(detail)` if the key matches (`detail` may be empty).
type Test<'a> = Box<dyn Fn(&str) -> Option<String> + 'a>;

struct Rule<'a> {
    category: Category,
    name: &'static str,
    evidence: String,
    test: Test<'a>,
}

impl<'a> Rule<'a> {
    fn new(
        category: Category,
        name: &'static str,
        evidence: impl Into<String>,
        test: impl Fn(&str) -> bool + 'a,
    ) -> Self {
        Self::detailed(category, name, evidence, move |k| test(k).then(String::new))
    }

    fn detailed(
        category: Category,
        name: &'static str,
        evidence: impl Into<String>,
        test: impl Fn(&str) -> Option<String> + 'a,
    ) -> Self {
        Self {
            category,
            name,
            evidence: evidence.into(),
            test: Box::new(test),
        }
    }
}

/// Puts each gap under the first rule that matches.
fn categorize<'k>(gaps: impl IntoIterator<Item = &'k str>, rules: &[Rule<'_>]) -> Gaps {
    let mut out = Gaps {
        by_category: BTreeMap::new(),
        total: 0,
    };
    for key in gaps {
        out.total += 1;
        let found = rules
            .iter()
            .find_map(|rule| (rule.test)(key).map(|detail| (rule, detail)));
        let (category, name, evidence, detail) = match &found {
            Some((rule, detail)) => (
                rule.category,
                rule.name,
                rule.evidence.as_str(),
                detail.as_str(),
            ),
            None => (Category::Uncategorized, "none", "no rule matched", ""),
        };
        let result = out
            .by_category
            .entry(category)
            .or_default()
            .entry(name.to_owned())
            .or_insert_with(|| RuleResult {
                breakdown: BTreeMap::new(),
                count: 0,
                evidence: evidence.to_owned(),
                keys: Vec::new(),
            });
        result.count += 1;
        result.keys.push(key.to_owned());
        if !detail.is_empty() {
            *result.breakdown.entry(detail.to_owned()).or_default() += 1;
        }
    }
    out
}

/// One family being assembled.
#[derive(Default)]
struct Family<'a> {
    pob_keys: BTreeSet<String>,
    our_keys: BTreeSet<String>,
    pob_sources: BTreeMap<String, usize>,
    excluded: BTreeMap<String, Excluded>,
    unparsed: Vec<UnparsedLine>,
    disabled: Vec<String>,
    pob_missing_rules: Vec<Rule<'a>>,
    ours_extra_rules: Vec<Rule<'a>>,
}

impl<'a> Family<'a> {
    fn add_pob(&mut self, path: &str, keys: pob::Keys) {
        self.pob_sources.insert(path.to_owned(), keys.keys.len());
        self.pob_keys.extend(keys.keys);
        self.unparsed_lines(path, keys.unparsed);
    }

    fn unparsed_lines(&mut self, path: &str, lines: Vec<pob::Unparsed>) {
        self.unparsed
            .extend(lines.into_iter().map(|u| UnparsedLine {
                file: path.to_owned(),
                line: u.line,
                text: u.text,
            }));
    }

    /// Adds `rule` only if `snippet` is in PoB's `script`; otherwise records
    /// it as disabled.
    fn anchored(
        &mut self,
        ctx: &Ctx<'_>,
        script: &str,
        snippet: &str,
        extra: bool,
        rule: Rule<'a>,
    ) {
        if ctx
            .pob
            .get(script)
            .is_some_and(|text| text.contains(snippet))
        {
            if extra {
                self.ours_extra_rules.push(rule);
            } else {
                self.pob_missing_rules.push(rule);
            }
        } else {
            self.disabled
                .push(format!("{}: `{snippet}` not found in {script}", rule.name));
        }
    }

    fn finish(mut self, compared: &'static str, status: Option<&str>) -> FamilyReport {
        if let Some(status) = status.filter(|s| *s != "ok") {
            for rules in [&mut self.pob_missing_rules, &mut self.ours_extra_rules] {
                rules.push(Rule::new(
                    Category::SchemaMismatch,
                    "table-not-ok",
                    format!("the extract manifest gives the table status `{status}`"),
                    |_| true,
                ));
            }
        }
        let missing = self.pob_keys.difference(&self.our_keys).map(String::as_str);
        let pob_missing = categorize(missing, &self.pob_missing_rules);
        let extra = self.our_keys.difference(&self.pob_keys).map(String::as_str);
        let ours_extra = categorize(extra, &self.ours_extra_rules);
        let matched = self.pob_keys.intersection(&self.our_keys).count();
        let pob_keys = self.pob_keys.len();
        let both = |c| pob_missing.count(c) + ours_extra.count(c);
        let patch_delta = both(Category::PatchDelta);
        let uncategorized = both(Category::Uncategorized);
        self.unparsed.sort();
        let gate = FamilyGate {
            met: pob_keys > 0
                && uncategorized == 0
                && self.unparsed.is_empty()
                && patch_delta * 100 <= pob_keys * PATCH_DELTA_PERCENT,
            patch_delta,
            patch_delta_limit: pob_keys * PATCH_DELTA_PERCENT / 100,
            uncategorized,
            unparsed: self.unparsed.len(),
        };
        FamilyReport {
            compared,
            disabled_rules: self.disabled,
            excluded_pob_files: self.excluded,
            gate,
            matched,
            our_keys: self.our_keys.len(),
            ours_extra,
            pob_keys,
            pob_missing,
            pob_sources: self.pob_sources,
            unparsed: self.unparsed,
        }
    }
}

/// Inputs, parsed.
struct Ctx<'a> {
    game: Game,
    schema: &'a Schema,
    ours: &'a BTreeMap<String, Vec<u8>>,
    pob: &'a BTreeMap<String, String>,
    manifest: ours::Manifest,
    /// Our stat-description files by the name PoB gives the converted file
    /// (lowercase, see [`pob_stat_desc_name`]).
    stat_descs: BTreeMap<String, StatDescFile>,
}

/// File name without folder.
fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// PoB's name for one of our stat-description files: the path inside
/// [`STAT_DESCRIPTIONS_DIR`] without `.txt`/`.csd` and `.json`, lowercase;
/// files below `specific_skill_stat_descriptions/<folder>/` are flattened
/// with `_`, as PoB's `Export/statdesc.lua` writes them.
fn pob_stat_desc_name(path: &str) -> Option<String> {
    let rel = path
        .strip_prefix(STAT_DESCRIPTIONS_DIR)?
        .strip_prefix('/')?
        .strip_suffix(".json")?;
    let rel = rel
        .strip_suffix(".txt")
        .or_else(|| rel.strip_suffix(".csd"))
        .unwrap_or(rel)
        .to_ascii_lowercase();
    const SPECIFIC: &str = "specific_skill_stat_descriptions/";
    Some(match rel.strip_prefix(SPECIFIC) {
        Some(rest) => format!("{SPECIFIC}{}", rest.replace('/', "_")),
        None => rel,
    })
}

impl<'a> Ctx<'a> {
    fn new(inputs: Inputs<'a>) -> Result<Self, CrossCheckError> {
        let manifest =
            ours::Manifest::parse(Self::file(inputs.ours, "manifest.json")?).map_err(|source| {
                CrossCheckError::Json {
                    path: "manifest.json".to_owned(),
                    source,
                }
            })?;
        let expected = game_name(inputs.game);
        if manifest.game != expected {
            return Err(CrossCheckError::GameMismatch {
                expected,
                found: manifest.game,
            });
        }
        let mut stat_descs = BTreeMap::new();
        let mut paths: BTreeMap<String, &str> = BTreeMap::new();
        for (path, bytes) in inputs.ours {
            let Some(name) = pob_stat_desc_name(path) else {
                continue;
            };
            let file = StatDescFile::parse(bytes).map_err(|source| CrossCheckError::Json {
                path: path.clone(),
                source,
            })?;
            if let Some(first) = paths.insert(name.clone(), path) {
                return Err(CrossCheckError::NameCollision {
                    stem: name,
                    first: first.to_owned(),
                    second: path.clone(),
                });
            }
            stat_descs.insert(name, file);
        }
        Ok(Self {
            game: inputs.game,
            schema: inputs.schema,
            ours: inputs.ours,
            pob: inputs.pob,
            manifest,
            stat_descs,
        })
    }

    fn file<'b>(
        files: &'b BTreeMap<String, Vec<u8>>,
        path: &str,
    ) -> Result<&'b [u8], CrossCheckError> {
        files
            .get(path)
            .map(Vec::as_slice)
            .ok_or_else(|| CrossCheckError::MissingFile(path.to_owned()))
    }

    fn table(&self, name: &str) -> Result<Table, CrossCheckError> {
        let path = format!("tables/{name}.json");
        Table::parse(Self::file(self.ours, &path)?)
            .map_err(|source| CrossCheckError::Json { path, source })
    }

    fn status(&self, table: &str) -> Option<&str> {
        self.manifest.tables.get(table).map(|t| t.status.as_str())
    }

    fn pob_file(&self, path: &str) -> Result<&'a str, CrossCheckError> {
        self.pob
            .get(path)
            .map(String::as_str)
            .ok_or_else(|| CrossCheckError::MissingFile(path.to_owned()))
    }

    /// PoB files directly in `dir` (or below it, if `deep`) ending in `ext`.
    fn pob_files(
        &self,
        dir: &str,
        ext: &str,
        deep: bool,
    ) -> impl Iterator<Item = (&'a str, &'a str)> {
        let prefix = format!("{dir}/");
        let ext = ext.to_owned();
        self.pob.iter().filter_map(move |(path, text)| {
            let rest = path.strip_prefix(&prefix)?;
            (rest.ends_with(&ext) && (deep || !rest.contains('/')))
                .then_some((path.as_str(), text.as_str()))
        })
    }

    /// A spec file list from an export script (see [`pob::quoted_list`]).
    fn spec_list(&self, script: &str, marker: &str) -> Result<Vec<String>, CrossCheckError> {
        pob::quoted_list(self.pob_file(script)?, marker).ok_or_else(|| {
            CrossCheckError::UnsupportedPob {
                path: script.to_owned(),
                what: format!("no `{marker} {{ ... }}` list"),
            }
        })
    }

    /// `NAME(value)` for an enumeration value, or the bare value.
    fn enum_label(&self, enumeration: Option<&str>, value: Option<u64>) -> String {
        let Some(value) = value else {
            return "null".to_owned();
        };
        let name = enumeration
            .and_then(|e| self.schema.enumeration(self.game, e))
            .and_then(|e| {
                let i = value.checked_sub(u64::from(e.indexing))?;
                e.enumerators.get(usize::try_from(i).ok()?)?.clone()
            });
        name.map_or_else(|| value.to_string(), |n| format!("{n}({value})"))
    }
}

fn column(table: &Table, name: &str) -> Result<usize, CrossCheckError> {
    table
        .column(name)
        .ok_or_else(|| CrossCheckError::MissingColumn {
            table: table.table.clone(),
            column: name.to_owned(),
        })
}

/// PoB's name for a stat-description file as written in a script or an
/// `include` line (`tincture_stat_descriptions.txt`,
/// `Metadata/StatDescriptions/stat_descriptions.txt`): the part after the
/// last `statdescriptions/`, lowercase, without extension, flattened like
/// [`pob_stat_desc_name`].
fn pob_name_of(file: &str) -> String {
    let lower = file.to_ascii_lowercase();
    let rel = lower
        .rfind("statdescriptions/")
        .and_then(|i| lower.get(i + "statdescriptions/".len()..))
        .unwrap_or(&lower);
    let rel = rel
        .strip_suffix(".txt")
        .or_else(|| rel.strip_suffix(".csd"))
        .unwrap_or(rel);
    const SPECIFIC: &str = "specific_skill_stat_descriptions/";
    match rel.strip_prefix(SPECIFIC) {
        Some(rest) => format!("{SPECIFIC}{}", rest.replace('/', "_")),
        None => rel.to_owned(),
    }
}

/// PoB's `statDescriptor` after `loadStatFile(files...)` (Export/statdesc.lua),
/// rebuilt from our parsed files: each stat maps to the last description
/// that names it in load order (includes read in place, before the file's
/// own entries), or to `None` if a later `no_description` names it.
///
/// Our output keeps `no_description` apart from the descriptions, so a
/// file's `no_description` entries are applied before its descriptions; PoB
/// applies them in line order. Stat-description files put them first.
struct Describer<'c> {
    by_stat: BTreeMap<&'c str, Option<(usize, &'c Description)>>,
    next_order: usize,
}

impl<'c> Describer<'c> {
    /// Loads `files` (names as the script writes them) in order.
    fn load(ctx: &'c Ctx<'_>, files: &[String]) -> Result<Self, CrossCheckError> {
        let mut describer = Self {
            by_stat: BTreeMap::new(),
            next_order: 0,
        };
        for file in files {
            describer.load_file(ctx, &pob_name_of(file), 0)?;
        }
        Ok(describer)
    }

    fn load_file(
        &mut self,
        ctx: &'c Ctx<'_>,
        name: &str,
        depth: usize,
    ) -> Result<(), CrossCheckError> {
        if depth > 16 {
            return Err(CrossCheckError::UnsupportedPob {
                path: name.to_owned(),
                what: "includes nest more than 16 deep".to_owned(),
            });
        }
        let file = ctx.stat_descs.get(name).ok_or_else(|| {
            CrossCheckError::MissingFile(format!("{STAT_DESCRIPTIONS_DIR}/{name}"))
        })?;
        for include in &file.includes {
            self.load_file(ctx, &pob_name_of(include), depth + 1)?;
        }
        for stat in &file.no_description {
            self.by_stat.insert(stat, None);
        }
        for desc in &file.descriptions {
            let order = self.next_order;
            self.next_order += 1;
            for stat in desc.all_stats() {
                self.by_stat.insert(stat, Some((order, desc)));
            }
        }
        Ok(())
    }

    /// The first English variant (not `table_only`) whose conditions hold
    /// for the stats' minimum values (`matchLimit`).
    fn match_limit<'d>(desc: &'d Description, mins: &[i64]) -> Option<&'d ours::Variant> {
        desc.variants
            .iter()
            .filter(|v| !v.flags.iter().any(|f| f == "table_only"))
            .find(|v| {
                v.conditions
                    .iter()
                    .enumerate()
                    .all(|(i, c)| c.holds(mins.get(i).copied().unwrap_or(0)))
            })
    }

    /// The lines `describeStats` gives for `stats` (stat, min, max), in
    /// description order: stats that are 0 to 0 are skipped; each
    /// description takes its first matching variant, and if none matches,
    /// tries again with a minimum of 0 raised to 1 for ranges above 0.
    fn lines(&self, stats: &[(&str, i64, i64)]) -> Vec<&'c str> {
        let values: BTreeMap<&str, (i64, i64)> =
            stats.iter().map(|&(s, min, max)| (s, (min, max))).collect();
        let descs: BTreeMap<usize, &Description> = stats
            .iter()
            .filter(|&&(_, min, max)| min != 0 || max != 0)
            .filter_map(|(s, _, _)| self.by_stat.get(s).copied().flatten())
            .collect();
        let mut out = Vec::new();
        for desc in descs.values() {
            let vals: Vec<(i64, i64)> = desc
                .all_stats()
                .map(|s| values.get(s).copied().unwrap_or((0, 0)))
                .collect();
            let mins: Vec<i64> = vals.iter().map(|v| v.0).collect();
            let found = Self::match_limit(desc, &mins).or_else(|| {
                let raised: Vec<i64> = vals
                    .iter()
                    .map(|&(min, max)| if min == 0 && max > 0 { 1 } else { min })
                    .collect();
                Self::match_limit(desc, &raised)
            });
            if let Some(variant) = found {
                out.push(variant.text.as_str());
            }
        }
        out
    }
}

fn mods(ctx: &Ctx<'_>) -> Result<FamilyReport, CrossCheckError> {
    let table = ctx.table("Mods")?;
    let stats = ctx.table("Stats")?;
    let families = ctx.table("ModFamily")?;
    let (id, domain, generation, family_col, aura_flags) = (
        column(&table, "Id")?,
        column(&table, "Domain")?,
        column(&table, "GenerationType")?,
        column(&table, "Families")?,
        column(&table, "AuraFlags")?,
    );
    let stat_id = column(&stats, "Id")?;
    let family_id = column(&families, "Id")?;
    let buffs = ctx.table("BuffTemplates")?;
    // Picked by reference: the names differ between the games' schemas.
    let buff_stats = buffs
        .columns
        .iter()
        .position(|c| c.array && c.references.as_deref() == Some("Stats"))
        .ok_or_else(|| CrossCheckError::MissingColumn {
            table: buffs.table.clone(),
            column: "array of Stats keys".to_owned(),
        })?;
    let buff_cols: Vec<usize> = table
        .columns
        .iter()
        .enumerate()
        .filter(|(_, c)| c.references.as_deref() == Some("BuffTemplates"))
        .map(|(i, _)| i)
        .collect();
    let enum_ref = |col: usize| table.columns.get(col).and_then(|c| c.references.as_deref());
    // describeMod reads stat slots 1 to 6: key `StatsKey<n>` (PoE1) or
    // `Stat<n>` (PoE2), values `Stat<n>Min`/`Stat<n>Max` (PoE1) or the
    // interval `Stat<n>Value` (PoE2).
    let mut slots = Vec::new();
    for n in 1..=6 {
        let key = [format!("StatsKey{n}"), format!("Stat{n}")]
            .into_iter()
            .find_map(|name| table.column(&name))
            .ok_or_else(|| CrossCheckError::MissingColumn {
                table: table.table.clone(),
                column: format!("StatsKey{n} or Stat{n}"),
            })?;
        let values = match (
            table.column(&format!("Stat{n}Min")),
            table.column(&format!("Stat{n}Max")),
        ) {
            (Some(min), Some(max)) => StatValues::MinMax(min, max),
            _ => StatValues::Interval(column(&table, &format!("Stat{n}Value"))?),
        };
        slots.push((key, values));
    }
    let rows: BTreeMap<&str, &Vec<Value>> = table
        .rows
        .iter()
        .filter_map(|row| Some((ours::text(row, id)?, row)))
        .collect();
    let combo = |row: &[Value]| {
        format!(
            "{} / {}",
            ctx.enum_label(enum_ref(domain), ours::uint(row, domain)),
            ctx.enum_label(enum_ref(generation), ours::uint(row, generation))
        )
    };

    let mut read = Vec::new();
    let mut excluded = BTreeMap::new();
    for (path, text) in ctx.pob_files("src/Data", ".lua", false) {
        let name = file_name(path);
        if !name.starts_with("Mod") {
            continue;
        }
        let keys = pob::mod_keys(text);
        if let Some((_, reason)) = NON_MOD_ID_FILES.iter().find(|(n, _)| *n == name) {
            let distinct: BTreeSet<&String> = keys.keys.iter().collect();
            excluded.insert(
                path.to_owned(),
                Excluded {
                    keys: distinct.len(),
                    keys_matching_ours: distinct
                        .iter()
                        .filter(|k| rows.contains_key(k.as_str()))
                        .count(),
                    reason,
                },
            );
        } else {
            read.push((path, keys));
        }
    }
    let exported_combos: BTreeSet<String> = read
        .iter()
        .flat_map(|(_, keys)| &keys.keys)
        .filter_map(|k| rows.get(k.as_str()))
        .map(|row| combo(row))
        .collect();
    let exported = &exported_combos;

    let stat_files =
        pob::call_args(ctx.pob_file(MODS_SCRIPT)?, "loadStatFile").ok_or_else(|| {
            CrossCheckError::UnsupportedPob {
                path: MODS_SCRIPT.to_owned(),
                what: "no loadStatFile(...) call".to_owned(),
            }
        })?;
    let describer = Describer::load(ctx, &stat_files)?;
    // Stat row indices of a mod's buff templates (one key or an array of keys).
    let buff_template_stats = |row: &[Value]| -> BTreeSet<u64> {
        buff_cols
            .iter()
            .filter_map(|&c| row.get(c))
            .flat_map(|v| match v {
                Value::Array(keys) => keys.iter().filter_map(Value::as_u64).collect::<Vec<_>>(),
                other => other.as_u64().into_iter().collect(),
            })
            .filter_map(|b| buffs.rows.get(usize::try_from(b).ok()?))
            .filter_map(|b| b.get(buff_stats).and_then(Value::as_array))
            .flatten()
            .filter_map(Value::as_u64)
            .collect()
    };
    // The stats describeMod describes, with their (min, max) values: slots
    // 1 to 6, leaving out stats of the mod's buff templates.
    let mod_stats = |row: &[Value]| -> Vec<(&str, i64, i64)> {
        let skip = buff_template_stats(row);
        slots
            .iter()
            .filter_map(|(key, values)| {
                let index = ours::uint(row, *key)?;
                if skip.contains(&index) {
                    return None;
                }
                let stat = stats.rows.get(usize::try_from(index).ok()?)?;
                let (min, max) = values.read(row)?;
                Some((ours::text(stat, stat_id)?, min, max))
            })
            .collect()
    };
    let lines = |row: &[Value]| describer.lines(&mod_stats(row));
    let non_empty = |row: &[Value], col: usize| {
        row.get(col)
            .and_then(Value::as_array)
            .is_some_and(|a| !a.is_empty())
    };
    let first_family = |row: &[Value]| -> Option<&str> {
        let first = row.get(family_col)?.as_array()?.first()?.as_u64()?;
        ours::text(families.rows.get(usize::try_from(first).ok()?)?, family_id)
    };
    let unique_generation = |row: &[Value]| {
        ctx.enum_label(enum_ref(generation), ours::uint(row, generation))
            .starts_with("UNIQUE(")
    };
    let mut id_patterns = Vec::new();
    for raw in pob::excluded_id_patterns(ctx.pob_file(MODS_SCRIPT)?) {
        let pattern =
            pob::LuaPattern::parse(&raw).map_err(|what| CrossCheckError::UnsupportedPob {
                path: MODS_SCRIPT.to_owned(),
                what,
            })?;
        id_patterns.push((raw, pattern));
    }
    let row = |k: &str| rows.get(k).copied();

    // Declared after everything its rules borrow.
    let mut family = Family {
        our_keys: rows.keys().map(|k| (*k).to_owned()).collect(),
        excluded,
        ..Family::default()
    };
    for (path, keys) in read {
        family.add_pob(path, keys);
    }
    family.ours_extra_rules.push(Rule::detailed(
        Category::PobFiltered,
        "id-pattern-excluded",
        "the Id matches a pattern that a writer in Export/Scripts/mods.lua excludes (`not mod.Id:match(\"<pattern>\")`, read from the script; counted per pattern in `breakdown`)",
        move |k| {
            id_patterns
                .iter()
                .find(|(_, p)| p.find(k))
                .map(|(raw, _)| raw.clone())
        },
    ));
    family.anchored(
        ctx,
        STATDESC_SCRIPT,
        "v.min ~= 0 or v.max ~= 0",
        true,
        Rule::new(
            Category::PobFiltered,
            "zero-values",
            "every stat describeMod reads (slots 1 to 6, without buff-template stats) is 0 to 0; Export/statdesc.lua's describeStats skips such stats (`v.min ~= 0 or v.max ~= 0`), so describeMod gives no line and Export/Scripts/mods.lua does not write the mod (`#orders > 0`)",
            move |k| {
                row(k).is_some_and(|r| {
                    let stats = mod_stats(r);
                    !stats.is_empty() && stats.iter().all(|&(_, min, max)| min == 0 && max == 0)
                })
            },
        ),
    );
    let loaded = stat_files.join(", ");
    family.anchored(
        ctx,
        MODS_SCRIPT,
        "#orders > 0",
        true,
        Rule::new(
            Category::PobFiltered,
            "no-description-line",
            format!("Export/Scripts/mods.lua writes a mod only if describeMod gives it a line (`#orders > 0`); reproducing describeStats over the files mods.lua loads ({loaded}, with their includes) gives this mod none (stat slots 1 to 6, without the stats of its buff templates): no stat has a description there, or no English variant matches its values"),
            move |k| row(k).is_some_and(|r| lines(r).is_empty()),
        ),
    );
    family.anchored(
        ctx,
        MODS_SCRIPT,
        "not stats[1]:match(\"^DNT\")",
        true,
        Rule::new(
            Category::PobFiltered,
            "dnt-first-line",
            "Export/Scripts/mods.lua skips a mod whose first line starts with `DNT` (`not stats[1]:match(\"^DNT\")`), after statdesc.lua's escapeGGGString turns `[DNT]` into `DNT`",
            move |k| {
                row(k).is_some_and(|r| {
                    lines(r)
                        .first()
                        .is_some_and(|l| pob::escape_ggg_string(l).starts_with("DNT"))
                })
            },
        ),
    );
    family.anchored(
        ctx,
        MODS_SCRIPT,
        "Id ~= \"AuraBonus\"",
        true,
        Rule::new(
            Category::PobFiltered,
            "aura-bonus-family",
            "Export/Scripts/mods.lua's item and item-exclusive writers skip mods whose first family is `AuraBonus`",
            move |k| row(k).and_then(|r| first_family(r)) == Some("AuraBonus"),
        ),
    );
    family.anchored(
        ctx,
        MODS_SCRIPT,
        "(mod.Family[1] and mod.Family[1].Id ~= \"AuraBonus\")",
        true,
        Rule::new(
            Category::PobFiltered,
            "unique-generation-without-family",
            "Export/Scripts/mods.lua's item-exclusive writer needs a mod family (`mod.Family[1] and ...`); this UNIQUE-generation mod has none",
            move |k| row(k).is_some_and(|r| unique_generation(r) && !non_empty(r, family_col)),
        ),
    );
    family.anchored(
        ctx,
        MODS_SCRIPT,
        "#mod.AuraFlags == 0",
        true,
        Rule::new(
            Category::PobFiltered,
            "aura-flags",
            "Export/Scripts/mods.lua's explicit-mod writer skips mods with aura flags (`#mod.AuraFlags == 0`)",
            move |k| row(k).is_some_and(|r| non_empty(r, aura_flags)),
        ),
    );
    family.ours_extra_rules.push(Rule::detailed(
        Category::PobFiltered,
        "domain-generation-not-exported",
        "PoB's mod files hold no mod of this domain and generation type (counted per pair in `breakdown`)",
        move |k| {
            let c = combo(row(k)?);
            (!exported.contains(&c)).then_some(c)
        },
    ));
    family.ours_extra_rules.push(Rule::detailed(
        Category::PatchDelta,
        "passes-reproduced-filters",
        "PoB exports mods of this domain and generation type, and none of the reproduced filters above leaves this one out, so PoB's writers would have written it: the row is not in the game data PoB exported from (newer than PoB's export)",
        move |k| {
            let c = combo(row(k)?);
            exported.contains(&c).then_some(c)
        },
    ));
    Ok(family.finish(
        "PoB mod IDs (src/Data/Mod*.lua, files keyed by mod Id) vs Mods.Id",
        ctx.status("Mods"),
    ))
}

/// Where a mod stat slot's values are.
#[derive(Debug, Clone, Copy)]
enum StatValues {
    /// Two columns (PoE1 `Stat<n>Min`, `Stat<n>Max`).
    MinMax(usize, usize),
    /// One `[min, max]` interval column (PoE2 `Stat<n>Value`).
    Interval(usize),
}

impl StatValues {
    fn read(self, row: &[Value]) -> Option<(i64, i64)> {
        match self {
            Self::MinMax(min, max) => Some((row.get(min)?.as_i64()?, row.get(max)?.as_i64()?)),
            Self::Interval(col) => match row.get(col)?.as_array()?.as_slice() {
                [min, max] => Some((min.as_i64()?, max.as_i64()?)),
                _ => None,
            },
        }
    }
}

/// How PoB's base export treats one `BaseItemTypes` row.
#[derive(Debug, Clone, PartialEq, Eq)]
enum BaseExport {
    NotInSpec,
    Royale,
    Dnt,
    /// Written under its own (cleaned) name.
    Plain,
    /// Written under the spec's display name.
    Renamed(String),
    /// `Energy Blade`, written as `Energy Blade One/Two Handed`.
    EnergyBlade,
}

fn bases(ctx: &Ctx<'_>) -> Result<FamilyReport, CrossCheckError> {
    let table = ctx.table("BaseItemTypes")?;
    let (id, name, inherits) = (
        column(&table, "Id")?,
        column(&table, "Name")?,
        column(&table, "InheritsFrom")?,
    );
    let script = ctx.pob_file(BASES_SCRIPT)?;
    let has = |snippet: &str| script.contains(snippet);
    let fold = has("gsub(\"\\195\\182\",\"o\")");
    let royale = has("string.find(baseItemType.Id, \"Royale\")");
    let dnt = has("displayName:find(\"DNT\")");
    let energy = has("\"Energy Blade One Handed\"");
    let clean = |n: &str| {
        let n = n.trim();
        if fold {
            n.replace('ö', "o")
        } else {
            n.to_owned()
        }
    };

    let mut family = Family::default();
    for (on, rule, snippet) in [
        (fold, "o-fold", "gsub(\"\\195\\182\",\"o\")"),
        (royale, "royale", "string.find(baseItemType.Id, \"Royale\")"),
        (dnt, "dnt-name", "displayName:find(\"DNT\")"),
        (energy, "energy-blade-rename", "\"Energy Blade One Handed\""),
    ] {
        if !on {
            family
                .disabled
                .push(format!("{rule}: `{snippet}` not found in {BASES_SCRIPT}"));
        }
    }
    let mut exact: BTreeMap<String, Option<String>> = BTreeMap::new();
    let mut matches = Vec::new();
    for item_type in ctx.spec_list(BASES_SCRIPT, "local itemTypes =")? {
        let path = format!("src/Export/Bases/{item_type}.txt");
        let spec = pob::base_spec(ctx.pob_file(&path)?);
        family.unparsed_lines(&path, spec.unparsed);
        for (base, display) in spec.bases {
            exact.insert(base, display);
        }
        for m in spec.matches {
            let col = match m.column.as_str() {
                "Id" => id,
                "BaseType" => inherits,
                other => {
                    return Err(CrossCheckError::UnsupportedPob {
                        path,
                        what: format!("#baseMatch column `{other}` has no known schema column"),
                    });
                }
            };
            let pattern = pob::LuaPattern::parse(&m.value).map_err(|what| {
                CrossCheckError::UnsupportedPob {
                    path: path.clone(),
                    what,
                }
            })?;
            matches.push((col, pattern));
        }
    }
    for (path, text) in ctx.pob_files("src/Data/Bases", ".lua", false) {
        family.add_pob(path, pob::base_keys(text));
    }

    let mut by_name: BTreeMap<String, Vec<BaseExport>> = BTreeMap::new();
    let mut generated: BTreeMap<String, &'static str> = BTreeMap::new();
    for row in &table.rows {
        let row_id = ours::text(row, id).unwrap_or("");
        let own = clean(ours::text(row, name).unwrap_or(""));
        let in_exact = exact.get(row_id);
        let in_match = matches
            .iter()
            .any(|(col, pattern)| ours::text(row, *col).is_some_and(|v| pattern.find(v)));
        let export = match in_exact {
            None if !in_match => BaseExport::NotInSpec,
            None if royale && row_id.contains("Royale") => BaseExport::Royale,
            _ => {
                let display = in_exact.cloned().flatten().map(|d| clean(&d));
                let shown = display.clone().unwrap_or_else(|| own.clone());
                if dnt && shown.contains("DNT") {
                    BaseExport::Dnt
                } else if energy && shown == "Energy Blade" {
                    generated.insert("Energy Blade One Handed".to_owned(), "energy-blade-rename");
                    generated.insert("Energy Blade Two Handed".to_owned(), "energy-blade-rename");
                    BaseExport::EnergyBlade
                } else if let Some(display) = display.filter(|d| *d != own) {
                    generated.insert(display.clone(), "display-name-in-spec");
                    BaseExport::Renamed(display)
                } else {
                    BaseExport::Plain
                }
            }
        };
        by_name.entry(own).or_default().push(export);
    }
    family.our_keys = by_name.keys().cloned().collect();

    // Why PoB has no entry under this name; `None` if PoB writes some row
    // under it, so its absence is not explained.
    let reason = |exports: &[BaseExport]| -> Option<&'static str> {
        let any = |f: fn(&BaseExport) -> bool| exports.iter().any(f);
        if any(|e| *e == BaseExport::Plain) {
            None
        } else if any(|e| matches!(e, BaseExport::Renamed(_))) {
            Some("renamed-by-spec")
        } else if any(|e| *e == BaseExport::EnergyBlade) {
            Some("energy-blade-rename")
        } else if any(|e| *e == BaseExport::Dnt) {
            Some("dnt-name")
        } else if any(|e| *e == BaseExport::Royale) {
            Some("royale")
        } else {
            Some("not-in-base-spec")
        }
    };
    let reasons: BTreeMap<String, &'static str> = by_name
        .iter()
        .filter_map(|(n, e)| Some((n.clone(), reason(e)?)))
        .collect();
    let reasons = std::rc::Rc::new(reasons);
    let extra = [
        (
            "not-in-base-spec",
            "no row with this name is named by a #base or matched by a #baseMatch in the base specs Export/Scripts/bases.lua processes (its itemTypes list)",
        ),
        (
            "royale",
            "Export/Scripts/bases.lua skips #baseMatch rows whose Id contains `Royale`",
        ),
        (
            "dnt-name",
            "Export/Scripts/bases.lua skips bases whose display name contains `DNT`",
        ),
        (
            "renamed-by-spec",
            "every exported row with this name gets a display name from its #base line",
        ),
        (
            "energy-blade-rename",
            "Export/Scripts/bases.lua writes `Energy Blade` as `Energy Blade One Handed` / `Two Handed`",
        ),
    ];
    for (rule, evidence) in extra {
        let reasons = std::rc::Rc::clone(&reasons);
        family
            .ours_extra_rules
            .push(Rule::new(Category::PobFiltered, rule, evidence, move |k| {
                reasons.get(k) == Some(&rule)
            }));
    }
    let generated = std::rc::Rc::new(generated);
    for (rule, evidence) in [
        (
            "display-name-in-spec",
            "a #base line in a processed base spec gives this display name to a game row",
        ),
        (
            "energy-blade-rename",
            "Export/Scripts/bases.lua writes `Energy Blade` under this name",
        ),
    ] {
        let generated = std::rc::Rc::clone(&generated);
        family.pob_missing_rules.push(Rule::new(
            Category::PobGenerated,
            rule,
            evidence,
            move |k| generated.get(k) == Some(&rule),
        ));
    }
    Ok(family.finish(
        "PoB base names (src/Data/Bases/*.lua) vs BaseItemTypes.Name, cleaned as PoB does (trimmed; `ö` written as `o`)",
        ctx.status("BaseItemTypes"),
    ))
}

fn skills(ctx: &Ctx<'_>) -> Result<FamilyReport, CrossCheckError> {
    let table = ctx.table("GrantedEffects")?;
    let id = column(&table, "Id")?;
    let mut family = Family {
        our_keys: table
            .rows
            .iter()
            .filter_map(|r| ours::text(r, id))
            .map(str::to_owned)
            .collect(),
        ..Family::default()
    };
    let mut listed = BTreeSet::new();
    let mut hand_written = BTreeSet::new();
    for name in ctx.spec_list(SKILLS_SCRIPT, "for _, name in pairs(")? {
        let path = format!("src/Export/Skills/{name}.txt");
        let spec = pob::skill_spec(ctx.pob_file(&path)?);
        family.unparsed_lines(&path, spec.unparsed);
        listed.extend(spec.listed);
        hand_written.extend(spec.hand_written);
    }
    for (path, text) in ctx.pob_files("src/Data/Skills", ".lua", false) {
        family.add_pob(path, pob::skill_keys(text));
    }
    let listed = std::rc::Rc::new(listed);
    let in_list = std::rc::Rc::clone(&listed);
    family.pob_missing_rules.push(Rule::new(
        Category::PobGenerated,
        "hand-written-in-spec",
        "written out by hand in a processed skill spec (`skills[\"Id\"] = {` in src/Export/Skills/*.txt), not generated from a game row",
        move |k| hand_written.contains(k) && !in_list.contains(k),
    ));
    let in_list = std::rc::Rc::clone(&listed);
    family.ours_extra_rules.push(Rule::new(
        Category::PobFiltered,
        "not-in-skill-spec",
        "no `#skill <Id>` line in the skill specs Export/Scripts/skills.lua processes",
        move |k| !in_list.contains(k),
    ));
    family.anchored(
        ctx,
        SKILLS_SCRIPT,
        "if not granted then",
        true,
        Rule::new(
            Category::PatchDelta,
            "listed-not-exported",
            "listed by `#skill` but not in PoB's output; Export/Scripts/skills.lua leaves out a listed skill only when its GrantedEffects row is missing from the game data it read, so the row is newer than PoB's export",
            move |k| listed.contains(k),
        ),
    );
    Ok(family.finish(
        "PoB skill IDs (src/Data/Skills/*.lua) vs GrantedEffects.Id",
        ctx.status("GrantedEffects"),
    ))
}

fn stats(ctx: &Ctx<'_>) -> Result<FamilyReport, CrossCheckError> {
    let table = ctx.table("Stats")?;
    let id = column(&table, "Id")?;
    const DIR: &str = "src/Data/StatDescriptions";
    // PoB name of each converted file -> stat IDs PoB lists in it.
    let mut pob_files: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut read = Vec::new();
    for (path, text) in ctx.pob_files(DIR, ".lua", true) {
        let keys = pob::stat_description_stats(text);
        let name = path
            .strip_prefix(DIR)
            .and_then(|p| p.strip_prefix('/'))
            .and_then(|p| p.strip_suffix(".lua"))
            .unwrap_or(path)
            .to_ascii_lowercase();
        pob_files
            .entry(name)
            .or_default()
            .extend(keys.keys.iter().cloned());
        read.push((path, keys));
    }
    // Our files that PoB converts, and the stats they name.
    let converted: Vec<&StatDescFile> = pob_files
        .keys()
        .filter_map(|name| ctx.stat_descs.get(name))
        .collect();
    let in_converted: BTreeSet<&str> = converted
        .iter()
        .flat_map(|f| &f.descriptions)
        .flat_map(Description::all_stats)
        .collect();
    let handed: BTreeSet<&str> = converted
        .iter()
        .flat_map(|f| &f.descriptions)
        .filter(|d| d.handed)
        .flat_map(Description::all_stats)
        .collect();
    let in_game: BTreeSet<&str> = ctx
        .stat_descs
        .values()
        .flat_map(|f| &f.descriptions)
        .flat_map(Description::all_stats)
        .collect();
    // For each PoB stat: the PoB files naming it, split by whether the game
    // still has that file.
    let files_of = |stat: &str| -> Vec<&String> {
        pob_files
            .iter()
            .filter(|(_, s)| s.contains(stat))
            .map(|(name, _)| name)
            .collect()
    };
    let stale = |stat: &str| {
        let files = files_of(stat);
        !files.is_empty()
            && files
                .iter()
                .all(|f| !ctx.stat_descs.contains_key(f.as_str()))
    };
    let placeholder = |stat: &str| {
        let files = files_of(stat);
        stat == "0"
            && !files.is_empty()
            && files.iter().all(|f| {
                ctx.stat_descs
                    .get(f.as_str())
                    .is_some_and(|d| d.descriptions.iter().any(|d| d.handed))
            })
    };
    // Declared after everything its rules borrow.
    let mut family = Family {
        our_keys: table
            .rows
            .iter()
            .filter_map(|r| ours::text(r, id))
            .map(str::to_owned)
            .collect(),
        ..Family::default()
    };
    for (path, keys) in read {
        family.add_pob(path, keys);
    }
    family.pob_missing_rules.push(Rule::new(
        Category::PobGenerated,
        "handed-placeholder",
        "PoB writes `0` as the stat list of handed (main hand / off hand) descriptions; every PoB file with it converts a game file that has a handed description",
        placeholder,
    ));
    family.pob_missing_rules.push(Rule::new(
        Category::NotInIndex,
        "stat-only-in-game-descriptions",
        "the installed stat-description files name this stat too, but the installed Stats table has no row for it",
        move |k| in_game.contains(k),
    ));
    family.pob_missing_rules.push(Rule::new(
        Category::NotInIndex,
        "stale-pob-description-file",
        "only PoB files whose game stat-description file is no longer installed name it (PoB keeps converted files of removed game files)",
        stale,
    ));
    family.ours_extra_rules.push(Rule::new(
        Category::PobFiltered,
        "handed-description",
        "named by a handed description in a file PoB converts; PoB writes `0` instead of the stat IDs",
        move |k| handed.contains(k),
    ));
    family.ours_extra_rules.push(Rule::new(
        Category::PobFiltered,
        "not-in-converted-descriptions",
        "PoB lists only stats named by the stat-description files it converts (src/Data/StatDescriptions); none of the installed counterparts of those files names this stat",
        move |k| !in_converted.contains(k),
    ));
    Ok(family.finish(
        "stat IDs in PoB's converted stat descriptions (src/Data/StatDescriptions/**/*.lua) vs Stats.Id",
        ctx.status("Stats"),
    ))
}

fn passives(ctx: &Ctx<'_>, tree_version: &str) -> Result<FamilyReport, CrossCheckError> {
    let table = ctx.table("PassiveSkills")?;
    let graph_id = column(&table, "PassiveSkillGraphId")?;
    let mut family = Family {
        our_keys: table
            .rows
            .iter()
            .filter_map(|r| ours::uint(r, graph_id))
            .map(|n| n.to_string())
            .collect(),
        ..Family::default()
    };
    let path = tree_path(tree_version);
    family.add_pob(&path, pob::tree_nodes(ctx.pob_file(&path)?));
    // No rules yet for our extra rows: the evidence for what is on the tree
    // is the game's passive skill graph, read in PR A6b.
    Ok(family.finish(
        "PoB tree node IDs (src/TreeData/<newest>/tree.lua) vs PassiveSkills.PassiveSkillGraphId",
        ctx.status("PassiveSkills"),
    ))
}

/// Runs the cross-check.
///
/// # Errors
/// If an input is missing, malformed, for the other game, or a PoB spec is
/// in a shape the rules cannot reproduce.
pub fn cross_check(inputs: Inputs<'_>, provenance: Provenance) -> Result<Report, CrossCheckError> {
    let ctx = Ctx::new(inputs)?;
    let mut families = BTreeMap::new();
    families.insert("bases", bases(&ctx)?);
    families.insert("mods", mods(&ctx)?);
    families.insert("passives", passives(&ctx, inputs.tree_version)?);
    families.insert("skills", skills(&ctx)?);
    families.insert("stats", stats(&ctx)?);
    let mut not_met = Vec::new();
    for (name, family) in &families {
        let g = &family.gate;
        if family.pob_keys == 0 {
            not_met.push(format!("{name}: no PoB keys read"));
        }
        if g.uncategorized > 0 {
            not_met.push(format!("{name}: {} uncategorized gaps", g.uncategorized));
        }
        if g.unparsed > 0 {
            not_met.push(format!("{name}: {} PoB lines not parsed", g.unparsed));
        }
        if g.patch_delta * 100 > family.pob_keys * PATCH_DELTA_PERCENT {
            not_met.push(format!(
                "{name}: {} patch-delta gaps, more than {PATCH_DELTA_PERCENT}% of {} PoB keys",
                g.patch_delta, family.pob_keys
            ));
        }
    }
    Ok(Report {
        families,
        format: REPORT_FORMAT,
        game: game_name(ctx.game),
        gate: Gate {
            met: not_met.is_empty(),
            not_met,
        },
        inputs: ReportInputs {
            extract_commit: ctx.manifest.extractor.commit.clone(),
            extract_dirty: ctx.manifest.extractor.dirty,
            provenance,
            pob_tree_version: inputs.tree_version.to_owned(),
            steam_build_id: ctx.manifest.steam_build_id.clone(),
        },
    })
}

#[cfg(test)]
#[expect(clippy::indexing_slicing, reason = "tests index known report entries")]
mod tests {
    use super::*;
    use serde_json::json;

    fn col(name: &str, kind: &str, references: Option<&str>, array: bool) -> Value {
        json!({"array": array, "interval": false, "name": name, "references": references, "type": kind})
    }

    fn table(name: &str, columns: Vec<Value>, rows: Value) -> Vec<u8> {
        serde_json::to_vec(&json!({"columns": columns, "rows": rows, "table": name})).unwrap()
    }

    fn id_table(name: &str, ids: &[&str]) -> Vec<u8> {
        let rows: Vec<Value> = ids.iter().map(|id| json!([id])).collect();
        table(name, vec![col("Id", "string", None, false)], json!(rows))
    }

    fn variant(text: &str, conditions: Value) -> Value {
        json!({"conditions": conditions, "flags": [], "handlers": [], "line": 1, "placeholders": [], "text": text})
    }

    fn desc(stats: &[&str], off_hand: Option<&[&str]>, variants: Vec<Value>) -> Value {
        json!({"handed": off_hand.is_some(), "line": 1, "name": null, "off_hand_stats": off_hand, "stats": stats, "variants": variants})
    }

    fn stat_file(descriptions: Vec<Value>, includes: &[&str], no_description: &[&str]) -> Vec<u8> {
        serde_json::to_vec(&json!({"descriptions": descriptions, "file_flags": [], "includes": includes, "languages": {}, "no_description": no_description, "source": "s"})).unwrap()
    }

    const ANY: &str = r#"[{"kind": "range", "max": null, "min": 1}]"#;

    /// Extract output with one gap per rule (see the expectations in
    /// `cross_check_should_put_each_gap_under_its_rule`).
    fn ours() -> BTreeMap<String, Vec<u8>> {
        let any: Value = serde_json::from_str(ANY).unwrap();
        let mut files = BTreeMap::new();
        let manifest = json!({"extractor": {"commit": "abc", "dirty": false, "version": "0"}, "format": 2, "game": "poe1", "steam_build_id": "7",
            "tables": {"Mods": {"status": "ok"}, "Stats": {"status": "ok"}, "BaseItemTypes": {"status": "ok"}, "GrantedEffects": {"status": "ok"}, "PassiveSkills": {"status": "row-size-mismatch"}}});
        files.insert(
            "manifest.json".to_owned(),
            serde_json::to_vec(&manifest).unwrap(),
        );
        let stats = ["s_a", "s_b", "s_dnt", "main_x", "off_x", "unused"];
        files.insert("tables/Stats.json".to_owned(), id_table("Stats", &stats));
        files.insert(
            "tables/ModFamily.json".to_owned(),
            id_table("ModFamily", &["Fam", "AuraBonus"]),
        );
        files.insert(
            "tables/BuffTemplates.json".to_owned(),
            table(
                "BuffTemplates",
                vec![
                    col("Id", "string", None, false),
                    col("StatsKey", "foreignrow", Some("Stats"), true),
                ],
                json!([["Buff", [0]]]),
            ),
        );
        let mut columns = vec![
            col("Id", "string", None, false),
            col("Domain", "enumrow", Some("ModDomains"), false),
            col(
                "GenerationType",
                "enumrow",
                Some("ModGenerationType"),
                false,
            ),
            col("Families", "foreignrow", Some("ModFamily"), true),
            col("AuraFlags", "enumrow", Some("ModAuraFlags"), true),
            col("BuffTemplate1", "foreignrow", Some("BuffTemplates"), false),
        ];
        for n in 1..=6 {
            columns.push(col(
                &format!("StatsKey{n}"),
                "foreignrow",
                Some("Stats"),
                false,
            ));
            columns.push(col(&format!("Stat{n}Min"), "i32", None, false));
            columns.push(col(&format!("Stat{n}Max"), "i32", None, false));
        }
        /// Id, domain, generation, families, aura flags, buff template, stat, min, max.
        type ModRow = (
            &'static str,
            u64,
            u64,
            &'static [u64],
            &'static [u64],
            Option<u64>,
            u64,
            i64,
            i64,
        );
        let mods: [ModRow; 10] = [
            ("Kept", 1, 1, &[0], &[], None, 0, 5, 10),
            ("RoyaleX", 1, 1, &[0], &[], None, 0, 5, 10),
            ("ZeroMod", 1, 1, &[0], &[], None, 0, 0, 0),
            ("NoLine", 1, 1, &[0], &[], None, 1, 1, 1),
            ("BuffOnly", 1, 1, &[0], &[], Some(0), 0, 1, 1),
            ("DntMod", 1, 1, &[0], &[], None, 2, 1, 1),
            ("AuraBonusMod", 1, 1, &[1], &[], None, 0, 1, 1),
            ("FlaggedMod", 1, 1, &[0], &[1], None, 0, 1, 1),
            ("MonsterMod", 3, 1, &[0], &[], None, 0, 1, 1),
            ("NewMod", 1, 1, &[0], &[], None, 0, 1, 1),
        ];
        let rows: Vec<Value> = mods
            .iter()
            .map(
                |(id, domain, generation, families, flags, buff, stat, min, max)| {
                    let mut row = vec![
                        json!(id),
                        json!(domain),
                        json!(generation),
                        json!(families),
                        json!(flags),
                        json!(buff),
                    ];
                    row.extend([json!(stat), json!(min), json!(max)]);
                    for _ in 2..=6 {
                        row.extend([Value::Null, json!(0), json!(0)]);
                    }
                    Value::Array(row)
                },
            )
            .collect();
        files.insert(
            "tables/Mods.json".to_owned(),
            table("Mods", columns, json!(rows)),
        );
        files.insert(
            "tables/BaseItemTypes.json".to_owned(),
            table(
                "BaseItemTypes",
                vec![
                    col("Id", "string", None, false),
                    col("Name", "string", None, false),
                    col("InheritsFrom", "string", None, false),
                ],
                json!([
                    [
                        "Metadata/Items/Rings/Ring1",
                        "Iron Ring",
                        "Metadata/Items/Rings/AbstractRing"
                    ],
                    [
                        "Metadata/Items/Rings/Ring12",
                        "Two-Stone Ring",
                        "Metadata/Items/Rings/AbstractRing"
                    ],
                    [
                        "Metadata/Items/Rings/RingRoyale1",
                        "Royale Ring",
                        "Metadata/Items/Rings/AbstractRing"
                    ],
                    [
                        "Metadata/Items/Amulets/Amulet1",
                        "Coral Amulet",
                        "Metadata/Items/Amulets/AbstractAmulet"
                    ],
                    [
                        "Metadata/Items/Rings/Ring2",
                        " Maelstr\u{f6}m Ring",
                        "Metadata/Items/Rings/AbstractRing"
                    ]
                ]),
            ),
        );
        files.insert(
            "tables/GrantedEffects.json".to_owned(),
            id_table("GrantedEffects", &["Fireball", "Unlisted", "NewSkill"]),
        );
        files.insert(
            "tables/PassiveSkills.json".to_owned(),
            table(
                "PassiveSkills",
                vec![col("PassiveSkillGraphId", "u16", None, false)],
                json!([[1], [2], [3]]),
            ),
        );
        files.insert(
            "stat_descriptions/stat_descriptions.txt.json".to_owned(),
            stat_file(
                vec![
                    desc(&["s_a"], None, vec![variant("{0} a", any.clone())]),
                    desc(&["s_dnt"], None, vec![variant("[DNT] x", any.clone())]),
                    desc(
                        &["main_x"],
                        Some(&["off_x"]),
                        vec![variant("h", any.clone())],
                    ),
                    desc(&["game_only"], None, vec![variant("g", any)]),
                ],
                &[],
                &[],
            ),
        );
        files
    }

    const MODS_LUA: &str = "loadStatFile(\"stat_descriptions.txt\")\nand not mod.Id:match(\"Royale\")\nif #orders > 0 and not stats[1]:match(\"^DNT\") then\n(mod.Family[1] and mod.Family[1].Id ~= \"AuraBonus\")\nand #mod.AuraFlags == 0\n";

    fn pob() -> BTreeMap<String, String> {
        let files: [(&str, &str); 12] = [
            (
                "src/Data/ModItem.lua",
                "return {\n\t[\"Kept\"] = { },\n\t[\"PobOnly\"] = { },\n}",
            ),
            (
                "src/Data/ModScalability.lua",
                "return {\n\t[\"some text\"] = { },\n}",
            ),
            ("src/Export/Scripts/mods.lua", MODS_LUA),
            (
                "src/Export/statdesc.lua",
                "if s ~= \"Type\" and (v.min ~= 0 or v.max ~= 0) then",
            ),
            (
                "src/Data/Bases/ring.lua",
                "itemBases[\"Iron Ring\"] = {\nitemBases[\"Two-Stone Ring (Fire/Cold)\"] = {\nitemBases[\"Maelstrom Ring\"] = {\n",
            ),
            (
                "src/Export/Bases/ring.txt",
                "#type Ring\n#baseMatch Metadata/Items/Rings/Ring%d\n#baseMatch Metadata/Items/Rings/RingRoyale\n#base Metadata/Items/Rings/Ring12 Two-Stone Ring (Fire/Cold)\n",
            ),
            (
                "src/Export/Scripts/bases.lua",
                "local itemTypes = {\n\t\"ring\",\n}\nif not string.find(baseItemType.Id, \"Royale\") then\ndisplayName = displayName:gsub(\"\\195\\182\",\"o\")\n",
            ),
            (
                "src/Data/Skills/act_int.lua",
                "skills[\"Fireball\"] = {\nskills[\"Manual\"] = {\n",
            ),
            (
                "src/Export/Skills/act_int.txt",
                "#skill Fireball\n#skillEnd\n#skill NewSkill\nskills[\"Manual\"] = {\n",
            ),
            (
                "src/Export/Scripts/skills.lua",
                "for _, name in pairs({\"act_int\"}) do\n\tif not granted then\n",
            ),
            (
                "src/Data/StatDescriptions/stat_descriptions.lua",
                "stats={\n[1]=\"s_a\"\n}\nstats={\n[1]=\"s_dnt\"\n}\nstats={\n[1]=\"0\",\n[2]=\"0\"\n}\nstats={\n[1]=\"game_only\"\n}\n",
            ),
            (
                "src/Data/StatDescriptions/removed_stat_descriptions.lua",
                "stats={\n[1]=\"gone_stat\"\n}\n",
            ),
        ];
        let mut out: BTreeMap<String, String> = files
            .iter()
            .map(|(p, t)| ((*p).to_owned(), (*t).to_owned()))
            .collect();
        out.insert(
            tree_path("3_29"),
            "\t\t[\"skill\"]= 1,\n\t\t[\"skill\"]= 2,\n".to_owned(),
        );
        out
    }

    fn provenance() -> Provenance {
        Provenance {
            extract_manifest_sha256: "m".to_owned(),
            pob_commit: Some("p".to_owned()),
            tool_commit: None,
            tool_dirty: None,
            tool_version: "0".to_owned(),
        }
    }

    fn run(
        ours: &BTreeMap<String, Vec<u8>>,
        pob: &BTreeMap<String, String>,
    ) -> Result<Report, CrossCheckError> {
        let schema = Schema::vendored().unwrap();
        cross_check(
            Inputs {
                game: Game::Poe1,
                schema: &schema,
                ours,
                pob,
                tree_version: "3_29",
            },
            provenance(),
        )
    }

    /// `category/rule -> keys` for one direction.
    fn rules(gaps: &Gaps) -> BTreeMap<String, Vec<String>> {
        gaps.by_category
            .iter()
            .flat_map(|(category, rules)| {
                rules.iter().map(move |(rule, result)| {
                    (
                        format!(
                            "{}/{rule}",
                            serde_json::to_value(category).unwrap().as_str().unwrap()
                        ),
                        result.keys.clone(),
                    )
                })
            })
            .collect()
    }

    fn expect(pairs: &[(&str, &[&str])]) -> BTreeMap<String, Vec<String>> {
        pairs
            .iter()
            .map(|(rule, keys)| {
                (
                    (*rule).to_owned(),
                    keys.iter().map(|k| (*k).to_owned()).collect(),
                )
            })
            .collect()
    }

    #[test]
    fn cross_check_should_put_each_gap_under_its_rule() {
        let report = run(&ours(), &pob()).unwrap();
        let mods = &report.families["mods"];
        assert_eq!(
            rules(&mods.pob_missing),
            expect(&[("uncategorized/none", &["PobOnly"])])
        );
        assert_eq!(
            rules(&mods.ours_extra),
            expect(&[
                ("pob-filtered/id-pattern-excluded", &["RoyaleX"]),
                ("pob-filtered/zero-values", &["ZeroMod"]),
                ("pob-filtered/no-description-line", &["BuffOnly", "NoLine"]),
                ("pob-filtered/dnt-first-line", &["DntMod"]),
                ("pob-filtered/aura-bonus-family", &["AuraBonusMod"]),
                ("pob-filtered/aura-flags", &["FlaggedMod"]),
                (
                    "pob-filtered/domain-generation-not-exported",
                    &["MonsterMod"]
                ),
                ("patch-delta/passes-reproduced-filters", &["NewMod"]),
            ])
        );
        let breakdown =
            &mods.ours_extra.by_category[&Category::PobFiltered]["domain-generation-not-exported"]
                .breakdown;
        assert_eq!(
            breakdown.keys().collect::<Vec<_>>(),
            ["MONSTER(3) / PREFIX(1)"]
        );
        let excluded = &mods.excluded_pob_files["src/Data/ModScalability.lua"];
        assert_eq!((excluded.keys, excluded.keys_matching_ours), (1, 0));
        assert!(!mods.gate.met);

        let bases = &report.families["bases"];
        assert_eq!(bases.matched, 2, "Iron Ring and the cleaned Maelstrom Ring");
        assert_eq!(
            rules(&bases.pob_missing),
            expect(&[(
                "pob-generated/display-name-in-spec",
                &["Two-Stone Ring (Fire/Cold)"]
            )])
        );
        assert_eq!(
            rules(&bases.ours_extra),
            expect(&[
                ("pob-filtered/not-in-base-spec", &["Coral Amulet"]),
                ("pob-filtered/renamed-by-spec", &["Two-Stone Ring"]),
                ("pob-filtered/royale", &["Royale Ring"]),
            ])
        );
        assert!(bases.gate.met);
        assert_eq!(
            bases.disabled_rules.len(),
            2,
            "no DNT or Energy Blade snippet in bases.lua"
        );

        let skills = &report.families["skills"];
        assert_eq!(
            rules(&skills.pob_missing),
            expect(&[("pob-generated/hand-written-in-spec", &["Manual"])])
        );
        assert_eq!(
            rules(&skills.ours_extra),
            expect(&[
                ("pob-filtered/not-in-skill-spec", &["Unlisted"]),
                ("patch-delta/listed-not-exported", &["NewSkill"])
            ])
        );
        assert!(skills.unparsed.is_empty(), "#skillEnd is not a #skill line");
        // 1 patch-delta gap of 2 PoB keys is over 2%.
        assert!(!skills.gate.met);

        let stats = &report.families["stats"];
        assert_eq!(
            rules(&stats.pob_missing),
            expect(&[
                ("not-in-index/stale-pob-description-file", &["gone_stat"]),
                (
                    "not-in-index/stat-only-in-game-descriptions",
                    &["game_only"]
                ),
                ("pob-generated/handed-placeholder", &["0"]),
            ])
        );
        assert_eq!(
            rules(&stats.ours_extra),
            expect(&[
                ("pob-filtered/handed-description", &["main_x", "off_x"]),
                (
                    "pob-filtered/not-in-converted-descriptions",
                    &["s_b", "unused"]
                ),
            ])
        );
        assert!(stats.gate.met);

        // PassiveSkills is not `ok` in the manifest: its gaps fall to schema-mismatch.
        let passives = &report.families["passives"];
        assert_eq!(
            rules(&passives.ours_extra),
            expect(&[("schema-mismatch/table-not-ok", &["3"])])
        );
        assert!(passives.gate.met);

        assert!(!report.gate.met);
        assert_eq!(
            report.gate.not_met,
            [
                "mods: 1 uncategorized gaps",
                "mods: 1 patch-delta gaps, more than 2% of 2 PoB keys",
                "skills: 1 patch-delta gaps, more than 2% of 2 PoB keys",
            ]
        );
        assert_eq!(report.inputs.steam_build_id.as_deref(), Some("7"));
        assert_eq!(report.inputs.extract_commit.as_deref(), Some("abc"));
    }

    #[test]
    fn cross_check_should_disable_rules_whose_script_snippet_is_gone() {
        let mut pob = pob();
        pob.insert(
            "src/Export/Scripts/mods.lua".to_owned(),
            "loadStatFile(\"stat_descriptions.txt\")\n".to_owned(),
        );
        let report = run(&ours(), &pob).unwrap();
        let mods = &report.families["mods"];
        assert_eq!(mods.disabled_rules.len(), 5, "{:?}", mods.disabled_rules);
        // No exclusion patterns and no DNT rule: these now pass every filter.
        let extra = rules(&mods.ours_extra);
        let passed = &extra["patch-delta/passes-reproduced-filters"];
        assert!(passed.contains(&"RoyaleX".to_owned()) && passed.contains(&"DntMod".to_owned()));
        assert_eq!(extra["pob-filtered/zero-values"], ["ZeroMod"]);
    }

    #[test]
    fn cross_check_should_report_unparsed_pob_lines_and_fail_the_family() {
        let mut pob = pob();
        pob.insert(
            "src/Data/Skills/broken.lua".to_owned(),
            "skills[\"Unclosed = {\n".to_owned(),
        );
        let report = run(&ours(), &pob).unwrap();
        let skills = &report.families["skills"];
        assert_eq!(skills.unparsed.len(), 1);
        assert_eq!(skills.unparsed[0].file, "src/Data/Skills/broken.lua");
        assert!(
            report
                .gate
                .not_met
                .contains(&"skills: 1 PoB lines not parsed".to_owned())
        );
    }

    #[test]
    fn cross_check_should_reject_wrong_game_missing_files_and_bad_specs() {
        let schema = Schema::vendored().unwrap();
        let (ours, pob) = (ours(), pob());
        let err = cross_check(
            Inputs {
                game: Game::Poe2,
                schema: &schema,
                ours: &ours,
                pob: &pob,
                tree_version: "3_29",
            },
            provenance(),
        )
        .unwrap_err();
        assert_eq!(err.code(), "EXTRACT_XCHECK_GAME_MISMATCH");

        let mut missing = ours.clone();
        missing.remove("tables/Stats.json");
        assert_eq!(
            run(&missing, &pob).unwrap_err().code(),
            "EXTRACT_XCHECK_MISSING_FILE"
        );

        let mut bad = pob.clone();
        bad.insert(
            "src/Export/Bases/ring.txt".to_owned(),
            "#baseMatch Weird Metadata/x\n".to_owned(),
        );
        assert_eq!(
            run(&ours, &bad).unwrap_err().code(),
            "EXTRACT_XCHECK_UNSUPPORTED_POB"
        );

        let mut pattern = pob.clone();
        pattern.insert(
            "src/Export/Bases/ring.txt".to_owned(),
            "#baseMatch Metadata/(x)\n".to_owned(),
        );
        assert_eq!(
            run(&ours, &pattern).unwrap_err().code(),
            "EXTRACT_XCHECK_UNSUPPORTED_POB"
        );

        let mut no_list = pob;
        no_list.insert(
            "src/Export/Scripts/skills.lua".to_owned(),
            "nothing here".to_owned(),
        );
        assert_eq!(
            run(&ours, &no_list).unwrap_err().code(),
            "EXTRACT_XCHECK_UNSUPPORTED_POB"
        );
    }

    #[test]
    fn summary_should_drop_only_pob_filtered_key_lists() {
        let report = run(&ours(), &pob()).unwrap();
        let summary = report.summary();
        let stats = &summary.families["stats"];
        let filtered =
            &stats.ours_extra.by_category[&Category::PobFiltered]["not-in-converted-descriptions"];
        assert_eq!(filtered.count, 2);
        assert!(filtered.keys.is_empty());
        assert_eq!(
            stats.pob_missing.by_category[&Category::NotInIndex]["stale-pob-description-file"].keys,
            ["gone_stat"]
        );
        assert_eq!(
            report.to_json().unwrap(),
            run(&ours(), &pob()).unwrap().to_json().unwrap()
        );
    }

    #[test]
    fn describer_should_follow_pob_load_order_and_match_limit() {
        let mut ours = ours();
        let range = |min: i64| json!([{"kind": "range", "max": null, "min": min}]);
        ours.insert(
            "stat_descriptions/base.txt.json".to_owned(),
            stat_file(
                vec![
                    desc(&["a"], None, vec![variant("base a", range(1))]),
                    desc(&["b"], None, vec![variant("b", range(1))]),
                ],
                &[],
                &[],
            ),
        );
        let mut table_only = variant("table", range(1));
        table_only["flags"] = json!(["table_only"]);
        ours.insert(
            "stat_descriptions/top.txt.json".to_owned(),
            stat_file(
                vec![
                    desc(&["a"], None, vec![variant("top a", range(1))]),
                    desc(&["c"], None, vec![table_only, variant("c pos", range(1))]),
                    desc(
                        &["n"],
                        None,
                        vec![variant("neg", json!([{"kind": "not", "value": 0}]))],
                    ),
                ],
                &["Metadata/StatDescriptions/base.txt"],
                &["b"],
            ),
        );
        let pob = pob();
        let schema = Schema::vendored().unwrap();
        let ctx = Ctx::new(Inputs {
            game: Game::Poe1,
            schema: &schema,
            ours: &ours,
            pob: &pob,
            tree_version: "3_29",
        })
        .unwrap();
        let d = Describer::load(&ctx, &["top.txt".to_owned()]).unwrap();
        // The later description of `a` wins; `b` is no_description later on.
        assert_eq!(d.lines(&[("a", 2, 2)]), ["top a"]);
        assert!(d.lines(&[("b", 2, 2)]).is_empty());
        // table_only variants are skipped; a 0..5 range retries with min 1.
        assert_eq!(d.lines(&[("c", 0, 5)]), ["c pos"]);
        assert!(d.lines(&[("c", -3, -1)]).is_empty());
        // 0 to 0 stats are skipped; `!0` holds for 2.
        assert!(d.lines(&[("n", 0, 0)]).is_empty());
        assert_eq!(d.lines(&[("n", 2, 2), ("a", 1, 1)]), ["top a", "neg"]);
        assert_eq!(
            Describer::load(&ctx, &["missing.txt".to_owned()])
                .err()
                .map(|e| e.code()),
            Some("EXTRACT_XCHECK_MISSING_FILE")
        );
    }

    #[test]
    fn categorize_should_use_the_first_matching_rule() {
        let rules = [
            Rule::new(Category::PobFiltered, "short", "e1", |k| k.len() < 3),
            Rule::detailed(Category::PatchDelta, "starts-x", "e2", |k| {
                k.starts_with('x').then(|| "x".to_owned())
            }),
        ];
        let gaps = categorize(["ab", "xab", "xa", "long"], &rules);
        assert_eq!(gaps.total, 4);
        assert_eq!(gaps.count(Category::PobFiltered), 2);
        assert_eq!(
            gaps.by_category[&Category::PatchDelta]["starts-x"].breakdown["x"],
            1
        );
        assert_eq!(
            gaps.by_category[&Category::Uncategorized]["none"].keys,
            ["long"]
        );
    }

    #[test]
    fn names_should_map_between_our_files_and_pob_files() {
        assert_eq!(
            pob_stat_desc_name(
                "stat_descriptions/specific_skill_stat_descriptions/arc/Arc_Statset_0.csd.json"
            )
            .as_deref(),
            Some("specific_skill_stat_descriptions/arc_arc_statset_0")
        );
        assert_eq!(
            pob_stat_desc_name("stat_descriptions/Gem_Stat_Descriptions.txt.json").as_deref(),
            Some("gem_stat_descriptions")
        );
        assert_eq!(pob_stat_desc_name("tables/Stats.json"), None);
        assert_eq!(
            pob_name_of("Metadata/StatDescriptions/stat_descriptions.txt"),
            "stat_descriptions"
        );
        assert_eq!(
            pob_name_of("tincture_stat_descriptions.txt"),
            "tincture_stat_descriptions"
        );
        assert_eq!(
            newest_tree_version(["3_9", "3_29_ruthless", "3_29", "3_10", "legion", "2_6"]),
            Some("3_29")
        );
        assert_eq!(newest_tree_version(["legion"]), None);
    }
}
