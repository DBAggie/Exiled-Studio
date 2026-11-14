//! Text to calc result (Phase 4 plan, decision 0): the one pipeline both
//! the application (`loom-app`) and the parity tools (`calc-run`) run, so
//! the Phase 3 parity evidence covers what the application shows.
//!
//! 1. [`read_pob`] reads PoB XML or a share code; [`read_document`] a
//!    build document of any version the migrations reach.
//! 2. [`resolve`] resolves the build's pending lines with the bundle and
//!    returns the exact ends of the ranges it read ([`RangeEnds`]).
//! 3. [`options`] gives the engine what the import adapter settles: each
//!    item's base name and PoB's line values (rolls, scalings) from those
//!    ends, plus the tree version the host knows for the data.
//! 4. [`calculate`] runs the engine with the host's cancellation hook;
//!    [`document`] writes the calc result document v1.
//!
//! [`resolve_item`] re-reads one item after an edit; [`reresolve`]
//! recovers the range ends of a build read with its resolutions.
//!
//! Core crate (ADR-016): no filesystem, clocks, threads, or environment.
//! Errors carry the failing layer's stable code (ADR-014) and the step.

use std::collections::BTreeMap;

use loom_calc::{BaseName, CalcData, CalcInput, CalcOptions, CalcResult};
use loom_domain::{Build, ItemBody, ItemId, LineKind, LineResolution, TreeVersion};
use loom_parser::import::{BundleData, ItemBaseName, ItemData as _, Resolver};
use loom_serialization::dto::calc::v1 as calc_dto;
use loom_serialization::legacy::{pob_item, pob_xml, share_code};

pub use loom_parser::import::{BuildRangeEnds as RangeEnds, LineRangeEnds};

/// Where the pipeline stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Reading the input.
    Read,
    /// Resolving its lines.
    Resolve,
    /// Calculating.
    Calculate,
    /// Writing the calc result document.
    Document,
}

impl Step {
    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Resolve => "resolve",
            Self::Calculate => "calculate",
            Self::Document => "document",
        }
    }
}

/// A step failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{}: {code}: {message}", step.name())]
pub struct Error {
    /// The step.
    pub step: Step,
    /// The failing layer's stable code.
    pub code: &'static str,
    /// Detail in English.
    pub message: String,
}

impl Error {
    fn new(step: Step, code: &'static str, message: impl ToString) -> Self {
        Self {
            step,
            code,
            message: message.to_string(),
        }
    }
}

/// What a PoB input was.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PobFormat {
    /// PoB's build XML.
    Xml,
    /// A share code (the XML, deflated, in URL-safe base64).
    ShareCode,
}

/// Which PoB format `text` is: XML when it starts with `<` (after
/// whitespace and a byte-order mark), else a share code.
#[must_use]
pub fn pob_format(text: &str) -> PobFormat {
    if text
        .trim_start_matches('\u{feff}')
        .trim_start()
        .starts_with('<')
    {
        PobFormat::Xml
    } else {
        PobFormat::ShareCode
    }
}

/// Reads PoB XML or a share code ([`pob_format`]).
///
/// # Errors
/// The import adapter's code ([`Step::Read`]).
pub fn read_pob(text: &str) -> Result<Build, Error> {
    match pob_format(text) {
        PobFormat::Xml => pob_xml::import(text).map_err(|e| Error::new(Step::Read, e.code(), &e)),
        PobFormat::ShareCode => {
            share_code::import(text.trim()).map_err(|e| Error::new(Step::Read, e.code(), &e))
        }
    }
}

/// Reads a build document of any version the shipped migrations reach.
///
/// # Errors
/// The failing layer's code (`MIGRATE_*`, `DTO_*`; [`Step::Read`]).
pub fn read_document(json: &str) -> Result<Build, Error> {
    loom_serialization::dto::read(json)
        .map(|loaded| loaded.build)
        .map_err(|e| Error::new(Step::Read, e.code(), &e))
}

/// The import adapter's base name of an item, as the resolver and the
/// engine take it.
fn base_name(body: &ItemBody, data: &BundleData<'_>) -> Result<ItemBaseName, String> {
    pob_item::item_base(body, data.base_names())
        .map(|(name, fallback)| ItemBaseName { name, fallback })
        .map_err(|e| e.name().to_owned())
}

/// Resolves the build's pending lines and records the bundle's data
/// version on it; the exact ends of every range the call resolved.
///
/// # Errors
/// The resolver's code (game or data version mismatch; [`Step::Resolve`]);
/// the build is then unchanged.
pub fn resolve(
    resolver: &Resolver<'_, '_>,
    data: &BundleData<'_>,
    build: &mut Build,
) -> Result<RangeEnds, Error> {
    resolver
        .resolve_build_ranged(build, |body| base_name(body, data))
        .map(|(_, ends)| ends)
        .map_err(|e| Error::new(Step::Resolve, e.code(), &e))
}

/// Resolves one item's pending lines (after an edit); the exact ends of
/// the ranges it read. The caller vouches that the item is of a build of
/// the bundle's game.
pub fn resolve_item(
    resolver: &Resolver<'_, '_>,
    data: &BundleData<'_>,
    body: &mut ItemBody,
) -> LineRangeEnds {
    let base = base_name(body, data);
    resolver.resolve_item_ranged(body, base.as_ref().map_err(String::as_str))
}

/// Sets every resolved or unresolved line (items and custom config lines)
/// back to pending; excluded lines keep their exclusion, as the import
/// adapter set them.
pub fn reset_resolutions(build: &mut Build) {
    let reset = |r: &mut LineResolution| {
        if !matches!(r, LineResolution::Excluded(_)) {
            *r = LineResolution::Pending;
        }
    };
    for entry in build.items.entries.typed_mut() {
        if let loom_domain::ItemsEntry::Item(item) = entry {
            for line in &mut item.body.lines {
                if let LineKind::Modifier(m) = &mut line.kind {
                    reset(&mut m.resolution);
                }
            }
        }
    }
    for set in build.config.sets.typed_mut() {
        for line in set.custom_modifiers.iter_mut().flat_map(|b| &mut b.lines) {
            reset(&mut line.resolution);
        }
        for line in set.inputs.typed_mut().flat_map(|i| &mut i.lines) {
            reset(&mut line.resolution);
        }
    }
}

/// What re-resolving a build read with its resolutions gave.
#[derive(Debug)]
pub enum Reresolved {
    /// The same resolutions: the range ends are the import's.
    Same(RangeEnds),
    /// Different resolutions (another resolver than the import's): the
    /// stored ones stand, and no ends are recovered.
    Changed,
}

/// Recovers the range ends of a build read with its resolutions (a build
/// document stores resolutions, not ends; Phase 4 plan, decision 7): a
/// clone with its lines reset to pending is resolved again and compared
/// with the build. The build itself is never changed.
///
/// # Errors
/// As [`resolve`] (another game or data version).
pub fn reresolve(
    resolver: &Resolver<'_, '_>,
    data: &BundleData<'_>,
    build: &Build,
) -> Result<Reresolved, Error> {
    let mut clone = build.clone();
    reset_resolutions(&mut clone);
    let ends = resolve(resolver, data, &mut clone)?;
    Ok(if clone == *build {
        Reresolved::Same(ends)
    } else {
        Reresolved::Changed
    })
}

/// The engine's options for `build`: each item's base name and PoB's line
/// values from the range ends, and the data's tree version, if the host
/// knows it.
#[must_use]
pub fn options(
    build: &Build,
    data: &BundleData<'_>,
    ends: &RangeEnds,
    tree_version: Option<TreeVersion>,
) -> CalcOptions {
    // The same base names the resolver used, for the engine's armour
    // pieces (the legacy format's naming rules stay in the adapter).
    let item_bases = build
        .items
        .items()
        .filter_map(|item| {
            pob_item::item_base(&item.body, data.base_names())
                .ok()
                .map(|(name, fallback)| (item.id, BaseName { name, fallback }))
        })
        .collect();
    // PoB's rules for values the text does not settle (rolls, scalings):
    // the adapter's, from the ranges' exact ends the resolver read.
    let item_lines = build
        .items
        .items()
        .filter_map(|item| {
            let item_ends: pob_item::LineEnds = ends
                .get(&item.id)
                .map(|lines| {
                    lines
                        .iter()
                        .map(|(n, stats)| {
                            (
                                *n,
                                stats.iter().map(|e| e.map(|e| (e.from, e.to))).collect(),
                            )
                        })
                        .collect()
                })
                .unwrap_or_default();
            let values = pob_item::line_values(item, &item_ends);
            (!values.is_empty()).then_some((item.id, values))
        })
        .collect();
    CalcOptions {
        tree_version,
        item_bases,
        item_lines,
        ..CalcOptions::default()
    }
}

/// Calculates `build` with the engine, polling `cancel`.
///
/// # Errors
/// The engine's code (`CALC_*`, cancellation included; [`Step::Calculate`]).
pub fn calculate(
    build: &Build,
    data: &dyn CalcData,
    options: &CalcOptions,
    cancel: &dyn Fn() -> bool,
) -> Result<CalcResult, Error> {
    let input = CalcInput {
        build,
        data,
        options,
    };
    loom_calc::calculate_cancellable(&input, cancel)
        .map_err(|e| Error::new(Step::Calculate, e.code(), &e))
}

/// The calc result document v1 of a result, as JSON.
///
/// # Errors
/// The document layer's code ([`Step::Document`]).
pub fn document(result: &CalcResult) -> Result<String, Error> {
    calc_dto::to_json(result).map_err(|e| Error::new(Step::Document, e.code(), &e))
}

/// Range ends with the items that no longer exist removed and one item's
/// replaced (after an item edit).
#[must_use]
pub fn with_item_ends(ends: &RangeEnds, item: ItemId, lines: LineRangeEnds) -> RangeEnds {
    let mut out: RangeEnds = ends.clone();
    if lines.is_empty() {
        out.remove(&item);
    } else {
        out.insert(item, lines);
    }
    out
}

/// Every item's base name, by ID (for display).
#[must_use]
pub fn base_names(build: &Build, data: &BundleData<'_>) -> BTreeMap<ItemId, String> {
    build
        .items
        .items()
        .filter_map(|item| {
            pob_item::item_base(&item.body, data.base_names())
                .ok()
                .map(|(name, _)| (item.id, name))
        })
        .collect()
}
