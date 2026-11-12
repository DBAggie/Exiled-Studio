//! `calc-run` (Phase 3 plan, step 5): each build through import,
//! resolution with the bundle, and the engine (`loom_calc::calculate`),
//! written as the calc result document v1
//! (`loom_serialization::dto::calc::v1`), plus a report, a scrubbed
//! summary, and the calculation time per build.
//!
//! Outputs in `<dir>` (which must not exist):
//! - `documents/<id>.json`: each build's document (deterministic);
//! - `report.json`: inputs, and per build its outcome, the document's
//!   SHA-256, and its state counts (deterministic for one tool commit:
//!   `calc-parity` reads it);
//! - `summary.json`: counts only (states, reasons by kind, failure codes),
//!   and calculation time (min, median, max, total);
//! - `timing.json`: calculation time per build, a baseline for ADR-013's
//!   performance budgets (Phase 4); not gated and not deterministic.
//!
//! Builds are named as `oracle-run` names fixtures (the file name, or the
//! SHA-256 with `--anonymous`), so `calc-parity` pairs the two runs.
//!
//! **Data overlay (plan decision 5).** `--data-overlay <overlay.json>`
//! (written by `data-parity` for the same bundle) calculates through
//! [`OverlayData`]: the bundle with PoB's values for the rows
//! `data-parity` found different. A `data-version` register entry counts
//! only when this run matches the oracle (`calc-parity --overlay-run`).
//! Every `CalcData` read consults [`OverlayData::pob_value`]; an overlay
//! row the engine reads but cannot apply refuses the run.
//!
//! **Tree version.** `--scope <file>` (`docs/phase-3/calc-scope-poe1.json`,
//! for the bundle's Steam build) gives the data's passive tree version, so
//! the engine checks each build's; without it a build that allocates
//! passives is `Partial` (`unchecked-tree-version`).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use loom_calc::{CalcData, CalcInput, CalcOptions, CharacterBase, StatAmount};
use loom_data::bundle::Bundle;
use loom_domain::{Build, DataVersion, TreeVersion};
use loom_parser::import::{BundleData, ItemBaseName, ItemData as _, Resolver};
use loom_serialization::dto::calc::v1 as calc_dto;
use loom_serialization::legacy::{pob_item, pob_xml};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::build_round_trip::{Format, Input};
use crate::oracle::sha256_hex;

/// Report format.
pub const FORMAT: u32 = 1;

/// The tool's commit.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolRecord {
    /// `git rev-parse HEAD`.
    pub commit: Option<String>,
    /// Whether the tree had changes.
    pub dirty: Option<bool>,
}

/// The bundle the run read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BundleRecord {
    /// `poe1` or `poe2`.
    pub game: String,
    /// Its Steam build.
    pub steam_build: Option<u64>,
    /// Its data schema.
    pub data_schema: u32,
    /// SHA-256 of its `bundle.json`.
    pub manifest_sha256: String,
}

/// The overlay the run read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverlayRecord {
    /// SHA-256 of `overlay.json`.
    pub sha256: String,
    /// Rows per table.
    pub rows: BTreeMap<String, usize>,
}

/// The scope file a run took the data's tree version from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScopeRecord {
    /// SHA-256 of the scope file.
    pub sha256: String,
    /// The tree version it names for the bundle's Steam build.
    pub tree_version: String,
}

/// Where a build stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Step {
    /// Import (`IMPORT_*`, `DTO_*` of a build document).
    Read,
    /// Resolution with the bundle.
    Resolve,
    /// The engine (`CALC_*`).
    Calculate,
    /// Writing or reading back the document (`DTO_*`).
    Document,
}

/// Field counts by state.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StateCounts {
    /// Complete fields.
    pub complete: usize,
    /// Partial fields.
    pub partial: usize,
    /// Unsupported (not computed) fields.
    pub unsupported: usize,
    /// Indeterminate fields.
    pub indeterminate: usize,
}

impl StateCounts {
    fn add(&mut self, state: &calc_dto::StateKind) {
        match state {
            calc_dto::StateKind::Complete => self.complete += 1,
            calc_dto::StateKind::Partial => self.partial += 1,
            calc_dto::StateKind::Unsupported => self.unsupported += 1,
            calc_dto::StateKind::Indeterminate => self.indeterminate += 1,
        }
    }
}

/// One build's outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildRecord {
    /// The file name, or the SHA-256 with `--anonymous`.
    pub id: String,
    /// SHA-256 of the build's bytes.
    pub sha256: String,
    /// `ok` (a document was written) or `failed`.
    pub status: String,
    /// Where it stopped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<Step>,
    /// The stable error code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// SHA-256 of `documents/<id>.json`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_sha256: Option<String>,
    /// The result's state (`complete`, `partial`, `unsupported`,
    /// `indeterminate`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    /// Field counts by state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fields: Option<StateCounts>,
}

/// `report.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    /// [`FORMAT`].
    pub format: u32,
    /// The tool.
    pub tool: ToolRecord,
    /// The bundle.
    pub bundle: BundleRecord,
    /// The overlay, when one was given.
    pub overlay: Option<OverlayRecord>,
    /// The scope file, when one was given.
    pub scope: Option<ScopeRecord>,
    /// Whether builds are named by SHA-256.
    pub anonymous: bool,
    /// Every build, in input order.
    pub builds: Vec<BuildRecord>,
}

/// Calculation time.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Timing {
    /// Builds calculated.
    pub builds: usize,
    /// Microseconds, smallest.
    pub min_micros: u128,
    /// Microseconds, median (the lower of the two middle ones).
    pub median_micros: u128,
    /// Microseconds, largest.
    pub max_micros: u128,
    /// Microseconds, all builds.
    pub total_micros: u128,
}

/// `summary.json`: counts only.
#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    /// [`FORMAT`].
    pub format: u32,
    /// The tool.
    pub tool: ToolRecord,
    /// The bundle.
    pub bundle: BundleRecord,
    /// The overlay.
    pub overlay: Option<OverlayRecord>,
    /// The scope file.
    pub scope: Option<ScopeRecord>,
    /// Builds read.
    pub builds: usize,
    /// Builds with a document.
    pub ok: usize,
    /// Builds without one.
    pub failed: usize,
    /// Failure codes to builds.
    pub failures: BTreeMap<String, usize>,
    /// Build states to builds.
    pub build_states: BTreeMap<String, usize>,
    /// Field states over all builds.
    pub field_states: StateCounts,
    /// Reason kinds (`unmapped-stat`, `not-computed`, ...) to fields
    /// carrying them, over all builds.
    pub field_reasons: BTreeMap<String, usize>,
    /// Reason kinds to builds carrying them at build level.
    pub build_reasons: BTreeMap<String, usize>,
    /// Calculation time.
    pub timing: Timing,
}

/// `timing.json`.
#[derive(Debug, Clone, Serialize)]
pub struct TimingFile {
    /// Microseconds per build id.
    pub builds: Vec<(String, u128)>,
}

/// One overlay row as `data-parity` writes it.
#[derive(Debug, Clone, Deserialize)]
struct OverlayRowIn {
    table: String,
    key: String,
    field: String,
    pob: Value,
}

/// `overlay.json` as `data-parity` writes it.
#[derive(Debug, Clone, Deserialize)]
struct OverlayIn {
    format: u32,
    inputs: OverlayInputs,
    rows: Vec<OverlayRowIn>,
}

#[derive(Debug, Clone, Deserialize)]
struct OverlayInputs {
    game: String,
    steam_build: Option<u64>,
    bundle_manifest_sha256: String,
    data_schema: u32,
}

/// The bundle with PoB's values for the rows `data-parity` found
/// different (plan decision 5). Tool-side only: the core keeps the game's
/// data.
pub struct OverlayData<'b> {
    bundle: &'b Bundle,
    rows: BTreeMap<(String, String, String), Value>,
}

impl<'b> OverlayData<'b> {
    /// Reads `overlay.json` for `bundle`, whose `bundle.json` has the
    /// SHA-256 `manifest_sha256`. Refuses an overlay made for another
    /// bundle, and a row given twice.
    pub fn read(bytes: &[u8], bundle: &'b Bundle, manifest_sha256: &str) -> Result<Self, String> {
        let overlay: OverlayIn =
            serde_json::from_slice(bytes).map_err(|e| format!("overlay: {e}"))?;
        if overlay.format != crate::data_parity::FORMAT {
            return Err(format!("overlay: format {}", overlay.format));
        }
        let i = &overlay.inputs;
        if i.game != bundle.game.name()
            || i.steam_build != bundle.manifest.source.steam_build
            || i.data_schema != bundle.manifest.data_schema
            || i.bundle_manifest_sha256 != manifest_sha256
        {
            return Err(format!(
                "overlay: made for {} Steam build {:?} (bundle.json {}), not this bundle",
                i.game, i.steam_build, i.bundle_manifest_sha256
            ));
        }
        let mut rows = BTreeMap::new();
        for row in overlay.rows {
            let key = (row.table, row.key, row.field);
            check_applicable(&key, &row.pob)?;
            if rows.insert(key.clone(), row.pob).is_some() {
                return Err(format!("overlay: row {key:?} twice"));
            }
        }
        Ok(Self { bundle, rows })
    }

    /// PoB's integer for a field the engine reads, else `ours`.
    fn int(&self, table: &str, key: &str, field: &str, ours: i64) -> i64 {
        self.pob_value(table, key, field)
            .and_then(Value::as_i64)
            .unwrap_or(ours)
    }

    /// PoB's value for a bundle row's field, in PoB's form, when the
    /// overlay has one (every `CalcData` read below consults it).
    pub fn pob_value(&self, table: &str, key: &str, field: &str) -> Option<&Value> {
        self.rows
            .get(&(table.to_owned(), key.to_owned(), field.to_owned()))
    }

    /// Rows per table.
    pub fn counts(&self) -> BTreeMap<String, usize> {
        let mut out = BTreeMap::new();
        for (table, _, _) in self.rows.keys() {
            *out.entry(table.clone()).or_default() += 1;
        }
        out
    }
}

/// The overlay rows of tables the engine reads, by field: the integer
/// fields it substitutes.
const APPLIED: [(&str, &str); 4] = [
    ("characters", "strength"),
    ("characters", "dexterity"),
    ("characters", "intelligence"),
    ("character_constants", "value"),
];

/// Fields the engine reads but an overlay row cannot give: PoB's passive
/// text is not stat IDs. Such a row refuses the overlay rather than
/// calculate with the game's value under the overlay's name. Rows of
/// fields the engine does not read (a passive's name or ascendancy)
/// change nothing.
const NOT_APPLICABLE: [(&str, &str); 2] = [("passives", "stats"), ("mastery_effects", "stats")];

fn check_applicable(key: &(String, String, String), pob: &Value) -> Result<(), String> {
    let (table, _, field) = key;
    let at = (table.as_str(), field.as_str());
    if NOT_APPLICABLE.contains(&at) {
        return Err(format!(
            "overlay: row {key:?} cannot be applied (PoB's text is not stat IDs)"
        ));
    }
    if APPLIED.contains(&at) && !pob.is_i64() {
        return Err(format!("overlay: row {key:?} is not an integer: {pob}"));
    }
    Ok(())
}

/// Each read takes the bundle's row with PoB's values for the overlay's
/// rows ([`APPLIED`]): characters by class name, character constants by
/// stat. Passive and mastery effect stats have no applicable row
/// ([`NOT_APPLICABLE`] refuses them when the overlay is read).
impl CalcData for OverlayData<'_> {
    fn data_version(&self) -> DataVersion {
        self.bundle.data_version()
    }

    fn character(&self, class: &str) -> Option<CharacterBase> {
        let c = self.bundle.character(class)?;
        Some(CharacterBase {
            strength: self.int("characters", class, "strength", c.strength),
            dexterity: self.int("characters", class, "dexterity", c.dexterity),
            intelligence: self.int("characters", class, "intelligence", c.intelligence),
            ..c
        })
    }

    fn character_constant(&self, stat: &str) -> Option<i64> {
        let ours = self.bundle.character_constant(stat)?;
        Some(self.int("character_constants", stat, "value", ours))
    }

    fn passive_stats(&self, node: u32) -> Option<&[StatAmount]> {
        self.bundle.passive_stats(node)
    }

    fn mastery_effect_stats(&self, effect: u32) -> Option<&[StatAmount]> {
        self.bundle.mastery_effect_stats(effect)
    }
}

/// What `calc-run` is asked to do.
pub struct Request<'a> {
    /// The bundle.
    pub bundle: &'a Bundle,
    /// The pipeline over it.
    pub resolver: &'a Resolver<'a, 'a>,
    /// Its data (for the item base names).
    pub data: &'a BundleData<'a>,
    /// The bundle's record.
    pub bundle_record: BundleRecord,
    /// The overlay, if any.
    pub overlay: Option<(&'a OverlayData<'a>, OverlayRecord)>,
    /// The scope file's record; its tree version is the data's
    /// (`CalcOptions::tree_version`).
    pub scope: Option<ScopeRecord>,
    /// Name builds by SHA-256.
    pub anonymous: bool,
    /// The tool.
    pub tool: ToolRecord,
}

/// The outputs of a run, before they are written.
pub struct Outputs {
    /// `report.json`.
    pub report: Report,
    /// `summary.json`.
    pub summary: Summary,
    /// `timing.json`.
    pub timing: TimingFile,
    /// Each build's document, by id.
    pub documents: Vec<(String, String)>,
}

/// One build through the pipeline: the document, or where it stopped.
fn one(
    input: &Input,
    request: &Request<'_>,
) -> Result<(String, calc_dto::Document, u128), (Step, String)> {
    let text = std::str::from_utf8(&input.bytes)
        .map_err(|_| (Step::Read, "IMPORT_ENCODING".to_owned()))?;
    let mut build: Build = match input.format {
        Format::PobXml => pob_xml::import(text).map_err(|e| (Step::Read, e.code().to_owned()))?,
        Format::BuildDocument => loom_serialization::dto::v1::from_json(text)
            .map_err(|e| (Step::Read, e.code().to_owned()))?,
    };
    request
        .resolver
        .resolve_build(&mut build, |body| {
            pob_item::item_base(body, request.data.base_names())
                .map(|(name, fallback)| ItemBaseName { name, fallback })
                .map_err(|e| e.name().to_owned())
        })
        .map_err(|e| (Step::Resolve, e.code().to_owned()))?;
    let options = CalcOptions {
        tree_version: request
            .scope
            .as_ref()
            .map(|s| TreeVersion::new(s.tree_version.as_str())),
        ..CalcOptions::default()
    };
    let data: &dyn CalcData = match &request.overlay {
        Some((overlay, _)) => *overlay,
        None => request.bundle,
    };
    let input = CalcInput {
        build: &build,
        data,
        options: &options,
    };
    let start = Instant::now();
    let result = loom_calc::calculate(&input);
    let micros = start.elapsed().as_micros();
    let result = result.map_err(|e| (Step::Calculate, e.code().to_owned()))?;
    let json = calc_dto::to_json(&result).map_err(|e| (Step::Document, e.code().to_owned()))?;
    let doc = calc_dto::from_json(&json).map_err(|e| (Step::Document, e.code().to_owned()))?;
    Ok((json, doc, micros))
}

/// The tag of a serialized enum value: a reason's kind (`not-computed`,
/// `unmapped-stat`), a state's or an outcome's name.
pub fn reason_kind<T: Serialize + ?Sized>(reason: &T) -> String {
    match serde_json::to_value(reason) {
        Ok(Value::String(s)) => s,
        Ok(Value::Object(m)) => m.keys().next().cloned().unwrap_or_default(),
        _ => String::new(),
    }
}

/// The name of a state (`complete`, ...).
pub fn state_name(state: &calc_dto::StateKind) -> String {
    reason_kind(state)
}

/// Runs every input. `Err` when two builds would have the same id.
pub fn run(inputs: &[Input], request: &Request<'_>) -> Result<Outputs, String> {
    let mut builds = Vec::new();
    let mut documents = Vec::new();
    let mut times = Vec::new();
    let mut summary = Summary {
        format: FORMAT,
        tool: request.tool.clone(),
        bundle: request.bundle_record.clone(),
        overlay: request.overlay.as_ref().map(|(_, r)| r.clone()),
        scope: request.scope.clone(),
        builds: 0,
        ok: 0,
        failed: 0,
        failures: BTreeMap::new(),
        build_states: BTreeMap::new(),
        field_states: StateCounts::default(),
        field_reasons: BTreeMap::new(),
        build_reasons: BTreeMap::new(),
        timing: Timing::default(),
    };
    for input in inputs {
        let sha256 = sha256_hex(&input.bytes);
        let id = if request.anonymous {
            sha256.clone()
        } else {
            input.file.clone()
        };
        // Joined into a path below, and paired with oracle-run's ids.
        if builds.iter().any(|b: &BuildRecord| b.id == id) {
            return Err(if request.anonymous {
                format!("two builds have the same bytes ({id})")
            } else {
                format!("two builds are named {id}")
            });
        }
        summary.builds += 1;
        let record = match one(input, request) {
            Ok((json, doc, micros)) => {
                summary.ok += 1;
                let mut counts = StateCounts::default();
                for f in &doc.result.fields {
                    counts.add(&f.state);
                    summary.field_states.add(&f.state);
                    let mut kinds: Vec<String> = f
                        .reason
                        .iter()
                        .chain(&f.unsupported)
                        .map(reason_kind)
                        .chain(f.indeterminate.iter().map(reason_kind))
                        .collect();
                    kinds.sort();
                    kinds.dedup();
                    for k in kinds {
                        *summary.field_reasons.entry(k).or_default() += 1;
                    }
                }
                let r = &doc.result.reasons;
                let mut kinds: Vec<String> = r
                    .unsupported
                    .iter()
                    .map(reason_kind)
                    .chain(r.indeterminate.iter().map(reason_kind))
                    .collect();
                kinds.sort();
                kinds.dedup();
                for k in kinds {
                    *summary.build_reasons.entry(k).or_default() += 1;
                }
                let state = state_name(&doc.result.state);
                *summary.build_states.entry(state.clone()).or_default() += 1;
                times.push((id.clone(), micros));
                let document_sha256 = sha256_hex(json.as_bytes());
                documents.push((id.clone(), json));
                BuildRecord {
                    id,
                    sha256,
                    status: "ok".to_owned(),
                    step: None,
                    code: None,
                    document_sha256: Some(document_sha256),
                    state: Some(state),
                    fields: Some(counts),
                }
            }
            Err((step, code)) => {
                summary.failed += 1;
                *summary.failures.entry(code.clone()).or_default() += 1;
                BuildRecord {
                    id,
                    sha256,
                    status: "failed".to_owned(),
                    step: Some(step),
                    code: Some(code),
                    document_sha256: None,
                    state: None,
                    fields: None,
                }
            }
        };
        builds.push(record);
    }
    let mut micros: Vec<u128> = times.iter().map(|(_, m)| *m).collect();
    micros.sort_unstable();
    summary.timing = Timing {
        builds: micros.len(),
        min_micros: micros.first().copied().unwrap_or(0),
        median_micros: micros
            .get(micros.len().saturating_sub(1) / 2)
            .copied()
            .unwrap_or(0),
        max_micros: micros.last().copied().unwrap_or(0),
        total_micros: micros.iter().sum(),
    };
    Ok(Outputs {
        report: Report {
            format: FORMAT,
            tool: request.tool.clone(),
            bundle: request.bundle_record.clone(),
            overlay: summary.overlay.clone(),
            scope: request.scope.clone(),
            anonymous: request.anonymous,
            builds,
        },
        summary,
        timing: TimingFile { builds: times },
        documents,
    })
}

/// Writes the outputs to `out`, which must not exist (written to a
/// sibling folder and renamed at the end, so a failed run leaves none).
pub fn write(out: &Path, outputs: &Outputs) -> Result<(), String> {
    if out.exists() {
        return Err(format!("{}: already exists", out.display()));
    }
    let partial: PathBuf = {
        let mut name = out.file_name().unwrap_or_default().to_os_string();
        name.push(".partial");
        out.with_file_name(name)
    };
    if partial.exists() {
        return Err(format!(
            "{}: already exists (an earlier run?)",
            partial.display()
        ));
    }
    let docs = partial.join("documents");
    std::fs::create_dir_all(&docs).map_err(|e| format!("{}: {e}", docs.display()))?;
    let write = |path: PathBuf, text: &str| {
        std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))
    };
    for (id, json) in &outputs.documents {
        write(docs.join(format!("{id}.json")), json)?;
    }
    write(partial.join("report.json"), &pretty(&outputs.report)?)?;
    write(partial.join("summary.json"), &pretty(&outputs.summary)?)?;
    write(partial.join("timing.json"), &pretty(&outputs.timing)?)?;
    std::fs::rename(&partial, out).map_err(|e| format!("{}: {e}", out.display()))
}

/// Pretty JSON with a final newline.
pub fn pretty(value: &impl Serialize) -> Result<String, String> {
    serde_json::to_string_pretty(value)
        .map(|t| t + "\n")
        .map_err(|e| e.to_string())
}

/// Reads a `calc-run` directory's report (and the SHA-256 of its bytes).
pub fn read_report(dir: &Path) -> Result<(Report, String), String> {
    let path = dir.join("report.json");
    let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let report: Report = serde_json::from_slice(&bytes)
        .map_err(|e| format!("{}: not a calc-run report: {e}", path.display()))?;
    if report.format != FORMAT {
        return Err(format!("{}: format {}", path.display(), report.format));
    }
    for b in &report.builds {
        if b.id.is_empty() || b.id == "." || b.id == ".." || b.id.contains(['/', '\\']) {
            return Err(format!(
                "{}: build id {:?} is not a file name",
                path.display(),
                b.id
            ));
        }
    }
    Ok((report, sha256_hex(&bytes)))
}

/// Reads a build's document from a `calc-run` directory and checks it
/// against the SHA-256 the report recorded.
pub fn read_document(dir: &Path, build: &BuildRecord) -> Result<calc_dto::Document, String> {
    let path = dir.join("documents").join(format!("{}.json", build.id));
    let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    if build.document_sha256.as_deref() != Some(sha256_hex(&bytes).as_str()) {
        return Err(format!(
            "{}: differs from the report's SHA-256",
            path.display()
        ));
    }
    let text = std::str::from_utf8(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    calc_dto::from_json(text).map_err(|e| format!("{}: {} ({e})", path.display(), e.code()))
}

#[cfg(test)]
mod tests;
