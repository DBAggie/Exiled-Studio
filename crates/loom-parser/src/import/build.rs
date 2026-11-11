//! Resolving a Build's item lines (Phase 2 plan, step 6; P2-6b).
//!
//! [`resolve_build`] runs the step 2 pipeline over every item of a
//! [`Build`] with the data of a validated bundle ([`BundleData`]) and
//! writes each modifier line's [`LineResolution`]. It is the harness's
//! per-item pipeline (`tools/parity-cli/src/resolve_corpus.rs`,
//! `count_item`), mapped onto the domain instead of the report:
//!
//! 1. **Base.** The import adapter names the item's base
//!    ([`ItemBaseName`]): a legacy format's naming rules stay in its
//!    adapter (plan decision 5), and this crate cannot depend on the
//!    adapter's crate. [`find_base`] gives the rows and the scope. An item
//!    without one gets [`UnresolvedReason::NoBase`] on every pending
//!    modifier line.
//! 2. **Runs.** Consecutive pending modifier lines of one section are
//!    resolved together ([`Index::resolve_lines_composed`]); any other
//!    line ends a run. A line read alone with no template is tried
//!    case-folded ([`tolerance::case_folded`]).
//! 3. **Ambiguity.** An ambiguous reading goes through the mod-row filter
//!    ([`mods::filter`]) with the base's rows, the rarity, the influences,
//!    and the first line's section and flags. One candidate left:
//!    resolved; otherwise [`UnresolvedReason::Ambiguous`] with the kept
//!    candidates' stats.
//! 4. **Split section.** A line with no template whose adjacent line of
//!    another section also has none, and which resolves with it as one
//!    two-line description ([`tolerance::split_section_with`]), takes the
//!    pair's reading: the earlier line is resolved with
//!    [`Adapted::SplitSection`], the later one continues it. The harness
//!    also pairs a line with a neighbour that resolved alone; here both
//!    halves must lack a template (a neighbour that resolved alone is
//!    skipped and the other neighbour tried), so a resolved line is never
//!    rewritten and the link is always valid (Spike B's 4 split-section
//!    lines are 2 such pairs).
//! 5. **Typed kinds.** A line still without a template is tried as a
//!    granted skill ([`Grafts::resolve`], with the gem from the bundle),
//!    then as an item property or catalyst quality
//!    ([`Properties::resolve`]).
//! 6. Anything left keeps its reason. A multi-line reading writes the
//!    result on the first line and [`LineResolution::Continues`] on the
//!    others; a multi-line reading that does not resolve gives each line
//!    the reason.
//!
//! Only [`LineResolution::Pending`] lines are touched: excluded lines keep
//! their exclusion, and a line resolved earlier keeps its result (and ends
//! a run, as any non-pending line does). No line is added, removed, or
//! changed but for its resolution, so source lines in equal source lines
//! out. Completeness is derived from the lines ([`Build::completeness`]).
//!
//! The harness's report-only details (the filter's evidence, mod backing,
//! the composed wrapper's ID, categories) are not written (P2-1 decision).
//!
//! [`Grafts::resolve`]: super::grafts::Grafts::resolve
//! [`Properties::resolve`]: super::properties::Properties::resolve

use std::collections::BTreeMap;

use loom_domain::{
    Adapted, Build, Completeness, DataVersion, Game, GemId, ItemBody, ItemsEntry, LineKind,
    LineResolution, ModifierLine, NoBase, PropertyValue, Rarity, Resolved, StatId, StatLine,
    StatValue, TypedResolution, UnresolvedReason,
};

use super::base::{BaseError, BaseQuery, FoundBase, ItemData, find_base};
use super::bundle::{BundleData, IndexError};
use super::mods::{self, Context, LineContext};
use super::properties::PropertyMatch;
use super::tolerance;
use crate::resolve::{Candidate, Index, Outcome, RawValue, ResolvedStat, Unresolved};

/// The base name an import adapter found for an item (owned
/// [`BaseQuery`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemBaseName {
    /// The name to look up.
    pub name: String,
    /// A name to look up when `name` is not a base (the adapter's own
    /// rule).
    pub fallback: Option<String>,
}

/// Why a build's items were not resolved. The build is unchanged.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ResolveError {
    /// The bundle is for another game than the build.
    #[error("the build is {build:?} and the data bundle is {data:?}")]
    GameMismatch {
        /// The build's game.
        build: Game,
        /// The bundle's game.
        data: Game,
    },
    /// The build records another data bundle (ADR-010: a build is never
    /// reinterpreted with other data silently).
    #[error("the build records data {recorded:?}, not {data:?}")]
    DataVersionMismatch {
        /// The version the build records.
        recorded: DataVersion,
        /// The bundle's version.
        data: DataVersion,
    },
    /// The bundle's references did not all resolve: it was not validated.
    #[error("{skipped} data references did not resolve; validate the bundle first")]
    Unvalidated {
        /// References skipped ([`BundleData::skipped_references`]).
        skipped: usize,
    },
    /// A scope of the item rules gives no index.
    #[error(transparent)]
    Index(#[from] IndexError),
}

impl ResolveError {
    /// Stable code (ADR-014).
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::GameMismatch { .. } => "IMPORT_GAME_MISMATCH",
            Self::DataVersionMismatch { .. } => "IMPORT_DATA_VERSION_MISMATCH",
            Self::Unvalidated { .. } => "IMPORT_DATA_UNVALIDATED",
            Self::Index(e) => e.code(),
        }
    }
}

/// The data version a bundle gives a build (ADR-010).
#[must_use]
pub fn data_version(data: &BundleData<'_>) -> DataVersion {
    let b = data.bundle();
    DataVersion {
        game: b.game,
        schema_version: b.manifest.data_schema,
        steam_build: b.manifest.source.steam_build,
    }
}

/// The pipeline over one bundle: its data and the index of every scope,
/// built once.
#[derive(Debug)]
pub struct Resolver<'d, 'b> {
    data: &'d BundleData<'b>,
    indexes: BTreeMap<&'b str, Index<'b>>,
}

impl<'d, 'b> Resolver<'d, 'b> {
    /// Builds every scope's index.
    ///
    /// # Errors
    /// [`ResolveError::Unvalidated`] when references were skipped, and
    /// [`ResolveError::Index`] when a scope does not build.
    pub fn new(data: &'d BundleData<'b>) -> Result<Self, ResolveError> {
        let skipped = data.skipped_references();
        if skipped > 0 {
            return Err(ResolveError::Unvalidated { skipped });
        }
        Ok(Self {
            data,
            indexes: data.indexes()?,
        })
    }

    /// Resolves the pending modifier lines of every item of `build`
    /// (module docs) and records the bundle's data version on it.
    /// `base_of` is the import adapter's base name of an item, or its
    /// stable reason for naming none ([`NoBase::Unnamed`]).
    ///
    /// # Errors
    /// [`ResolveError::GameMismatch`] and
    /// [`ResolveError::DataVersionMismatch`]; the build is then unchanged.
    pub fn resolve_build(
        &self,
        build: &mut Build,
        mut base_of: impl FnMut(&ItemBody) -> Result<ItemBaseName, String>,
    ) -> Result<Completeness, ResolveError> {
        let version = data_version(self.data);
        if build.game() != version.game {
            return Err(ResolveError::GameMismatch {
                build: build.game(),
                data: version.game,
            });
        }
        match &build.data {
            Some(recorded) if *recorded != version => {
                return Err(ResolveError::DataVersionMismatch {
                    recorded: recorded.clone(),
                    data: version,
                });
            }
            Some(_) => {}
            None => build.data = Some(version),
        }
        for entry in build.items.entries.typed_mut() {
            if let ItemsEntry::Item(item) = entry {
                let base = base_of(&item.body);
                self.resolve_item(&mut item.body, base.as_ref().map_err(String::as_str));
            }
        }
        Ok(build.completeness())
    }

    /// Resolves the pending modifier lines of one item (module docs), with
    /// the adapter's base name or its reason for naming none. Unlike
    /// [`Resolver::resolve_build`], it checks neither the game nor the data
    /// version, and records nothing on a build: the caller vouches that the
    /// item belongs to a build of the bundle's game.
    pub fn resolve_item(&self, body: &mut ItemBody, base: Result<&ItemBaseName, &str>) {
        let found = match base {
            Err(reason) => Err(NoBase::Unnamed(reason.to_owned())),
            Ok(b) => find_base(
                self.data,
                BaseQuery {
                    name: &b.name,
                    fallback: b.fallback.as_deref(),
                },
            )
            .map_err(|e| match e {
                BaseError::NotABase => NoBase::NotFound,
                BaseError::ScopeConflict => NoBase::ScopeConflict,
            }),
        };
        let written = match found {
            Err(reason) => body
                .modifiers()
                .filter(|(_, m)| m.resolution == LineResolution::Pending)
                .map(|(n, _)| {
                    (
                        n,
                        LineResolution::Unresolved(UnresolvedReason::NoBase(reason.clone())),
                    )
                })
                .collect(),
            Ok(found) => match self.indexes.get(found.scope) {
                Some(index) => self.resolve_lines(body, &found, index),
                // Every rule scope is indexed by `new`; were one not, the
                // lines would stay pending, which is quarantined.
                None => BTreeMap::new(),
            },
        };
        for (n, resolution) in written {
            if let Some(LineKind::Modifier(m)) = body.lines.get_mut(n).map(|l| &mut l.kind) {
                m.resolution = resolution;
            }
        }
        debug_assert!(body.check_links().is_ok(), "resolve_item broke a link");
    }

    /// Every pending modifier line's resolution, by line index; reads the
    /// body as read (`split_section` needs it so).
    fn resolve_lines(
        &self,
        body: &ItemBody,
        found: &FoundBase<'_>,
        index: &Index<'_>,
    ) -> BTreeMap<usize, LineResolution> {
        let rules = &self.data.item_rules().mods;
        let ctx = Context {
            rules,
            mods: self.data.mods(),
            bases: found.rows,
            unique: body.rarity.is_none_or(Rarity::is_unique),
            influences: body
                .influences()
                .filter_map(|id| rules.influence_value(id))
                .collect(),
        };
        let mut out = Lines::default();
        let mut run: Vec<(usize, &ModifierLine)> = Vec::new();
        for (n, line) in body.lines.iter().enumerate() {
            match &line.kind {
                LineKind::Modifier(m) if m.resolution == LineResolution::Pending => {
                    if run
                        .last()
                        .is_some_and(|(_, prev)| prev.section != m.section)
                    {
                        self.resolve_run(&ctx, index, &std::mem::take(&mut run), &mut out);
                    }
                    run.push((n, m));
                }
                _ => self.resolve_run(&ctx, index, &std::mem::take(&mut run), &mut out),
            }
        }
        self.resolve_run(&ctx, index, &run, &mut out);
        out.pair_split_sections(index, body);
        self.typed_kinds(found.name, &mut out);
        out.finish()
    }

    /// Resolves one run of consecutive pending lines of one section.
    fn resolve_run(
        &self,
        ctx: &Context<'_>,
        index: &Index<'_>,
        run: &[(usize, &ModifierLine)],
        out: &mut Lines,
    ) {
        let texts: Vec<&str> = run.iter().map(|(_, m)| m.text.as_str()).collect();
        let compositions = self.data.compositions();
        let mut at = 0;
        for composed in index.resolve_lines_composed(&texts, compositions) {
            let mut resolution = composed.resolution;
            let mut adapted = None;
            if let Some(folded) = texts.get(at).and_then(|t| {
                tolerance::case_folded(index, t, &resolution, composed.composition.is_some())
            }) {
                resolution = folded;
                adapted = Some(Adapted::CaseFolded);
            }
            let start = at;
            at += resolution.lines;
            let Some(&(first, first_line)) = run.get(start) else {
                continue;
            };
            let covered: Vec<usize> = run
                .iter()
                .skip(start)
                .take(resolution.lines)
                .map(|(n, _)| *n)
                .collect();
            let state = match &resolution.outcome {
                Outcome::Resolved(c) => Ok(stats(&c.stats)),
                Outcome::Ambiguous(cs) => {
                    let kept = mods::filter(ctx, &LineContext::of(first_line), cs).kept;
                    match kept.as_slice() {
                        [one] if let Some(c) = cs.get(*one) => Ok(stats(&c.stats)),
                        _ => Err(UnresolvedReason::Ambiguous(
                            kept.iter()
                                .filter_map(|i| cs.get(*i))
                                .map(stat_ids)
                                .collect(),
                        )),
                    }
                }
                Outcome::Unresolved(Unresolved::NoTemplate) if covered.len() == 1 => {
                    out.no_template.insert(first, resolution.line.clone());
                    continue;
                }
                Outcome::Unresolved(u) => Err(reason(u)),
            };
            match state {
                Ok(result) => out.resolved(&covered, Resolved::Stats(result), adapted),
                Err(reason) => {
                    for n in covered {
                        out.done
                            .insert(n, LineResolution::Unresolved(reason.clone()));
                    }
                }
            }
        }
        // `resolve_lines` covers every line; a line it did not would stay
        // pending (quarantined), never lost.
    }

    /// Step 5: granted skills, then properties, for lines still without a
    /// template.
    fn typed_kinds(&self, base: &str, out: &mut Lines) {
        let data = self.data;
        let typed: Vec<(usize, TypedResolution)> = out
            .no_template
            .iter()
            .filter_map(|(n, text)| {
                let skill = data.grafts().resolve(base, text).and_then(|g| {
                    // The validator checks every graft's gem
                    // (`BUNDLE_DANGLING_REF`), and `BundleData` maps every
                    // graft it reads, so a resolved graft always has one.
                    let gem = data.granted_gem(&g.graft.id);
                    debug_assert!(gem.is_some(), "graft {} has no gem", g.graft.id);
                    let gem = gem?;
                    Some(TypedResolution::GrantedSkill {
                        source: g.graft.id,
                        gem: GemId::new(gem),
                        level: g.level,
                    })
                });
                skill
                    .or_else(|| data.properties().resolve(text).map(property))
                    .map(|t| (*n, t))
            })
            .collect();
        for (n, t) in typed {
            out.no_template.remove(&n);
            out.resolved(&[n], Resolved::Typed(t), None);
        }
    }
}

/// Resolves a build with a bundle's data ([`Resolver`]; module docs).
///
/// # Errors
/// [`ResolveError`]; the build is then unchanged.
pub fn resolve_build(
    build: &mut Build,
    data: &BundleData<'_>,
    base_of: impl FnMut(&ItemBody) -> Result<ItemBaseName, String>,
) -> Result<Completeness, ResolveError> {
    Resolver::new(data)?.resolve_build(build, base_of)
}

/// The resolutions of one item while they are worked out.
#[derive(Debug, Default)]
struct Lines {
    /// Decided lines.
    done: BTreeMap<usize, LineResolution>,
    /// Lines read alone with no template, with their normalized text;
    /// steps 4 and 5 may still resolve them.
    no_template: BTreeMap<usize, String>,
}

impl Lines {
    /// `result` on the first line of `covered`, the others continuing it.
    fn resolved(&mut self, covered: &[usize], result: Resolved, adapted: Option<Adapted>) {
        let Some((&first, rest)) = covered.split_first() else {
            return;
        };
        self.done
            .insert(first, LineResolution::Resolved { result, adapted });
        for n in rest {
            self.done
                .insert(*n, LineResolution::Continues { line: first });
        }
    }

    /// Step 4: pairs of adjacent no-template lines of two sections that
    /// resolve as one two-line description.
    fn pair_split_sections(&mut self, index: &Index<'_>, body: &ItemBody) {
        let lines: Vec<usize> = self.no_template.keys().copied().collect();
        for n in lines {
            if !self.no_template.contains_key(&n) {
                continue; // Taken by an earlier pair.
            }
            let pairs = |o: usize| self.no_template.contains_key(&o);
            let Some((other, r)) = tolerance::split_section_with(index, body, n, pairs) else {
                continue;
            };
            let Outcome::Resolved(c) = &r.outcome else {
                continue;
            };
            self.no_template.remove(&n);
            self.no_template.remove(&other);
            let pair = [n.min(other), n.max(other)];
            self.resolved(
                &pair,
                Resolved::Stats(stats(&c.stats)),
                Some(Adapted::SplitSection),
            );
        }
    }

    /// Every decided line; the rest have no template.
    fn finish(mut self) -> BTreeMap<usize, LineResolution> {
        for n in std::mem::take(&mut self.no_template).into_keys() {
            self.done
                .insert(n, LineResolution::Unresolved(UnresolvedReason::NoTemplate));
        }
        self.done
    }
}

fn reason(u: &Unresolved) -> UnresolvedReason {
    match u {
        Unresolved::NoTemplate => UnresolvedReason::NoTemplate,
        Unresolved::NoVariantFits(_) => UnresolvedReason::NoVariantFits,
        Unresolved::OnlyShadowed => UnresolvedReason::OnlyShadowed,
    }
}

fn stat_ids(c: &Candidate) -> Vec<StatId> {
    c.stats.iter().map(|s| StatId::new(&s.stat)).collect()
}

fn stats(stats: &[ResolvedStat]) -> Vec<StatLine> {
    stats
        .iter()
        .map(|s| StatLine {
            stat: StatId::new(&s.stat),
            value: value(&s.value),
        })
        .collect()
}

fn value(v: &RawValue) -> StatValue {
    match v {
        RawValue::Exact(s) => StatValue::Exact {
            min: s.min,
            max: s.max,
        },
        RawValue::Ranged(s) => StatValue::Ranged {
            min: s.min,
            max: s.max,
        },
        RawValue::Hidden { assumed, .. } => StatValue::Hidden { assumed: *assumed },
        RawValue::Lookup(keys) => StatValue::Lookup(keys.clone()),
    }
}

/// A property match on the domain (P2-2d decisions): a template or a
/// label is an item property; a quality type is a catalyst quality when it
/// has stats, else an item property with the quality as its one value.
fn property(p: PropertyMatch) -> TypedResolution {
    match p {
        PropertyMatch::Template { property, values } => TypedResolution::ItemProperty {
            property,
            values: values
                .into_iter()
                .map(|(slot, text)| PropertyValue { slot, text })
                .collect(),
        },
        PropertyMatch::Label { property, value } => TypedResolution::ItemProperty {
            property,
            values: vec![PropertyValue {
                slot: 0,
                text: value,
            }],
        },
        PropertyMatch::Quality {
            quality_type,
            stats,
            quality,
        } if stats.is_empty() => TypedResolution::ItemProperty {
            property: quality_type,
            values: vec![PropertyValue {
                slot: 0,
                text: quality,
            }],
        },
        PropertyMatch::Quality {
            quality_type,
            stats,
            quality,
        } => TypedResolution::CatalystQuality {
            quality_type,
            stats: stats
                .into_iter()
                .map(|(stat, v)| StatLine {
                    stat: StatId::new(stat),
                    value: StatValue::Exact { min: v, max: v },
                })
                .collect(),
            quality,
        },
    }
}
