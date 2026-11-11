//! The item pipeline's game data from a data bundle (Phase 2 plan, step 6;
//! P2-6a): [`BundleData`], the bundle's [`ItemData`].
//!
//! Built from a loaded [`Bundle`] that the validator accepted
//! (`loom_data::bundle::validate`): every reference resolves and every ID
//! is unique, so building cannot fail. A reference that does not resolve
//! anyway (an unvalidated bundle) is skipped, never guessed.
//!
//! The bundle names rows by ID; the pipeline's types number them. The
//! mapping:
//! - **mod rows:** [`ModRow`] `n` is the bundle's mod `n` (base implicits
//!   name mods by ID and are mapped to that number). A `(0, 0)` stat goes
//!   to [`ModRow::unset`], as the extract reader puts it;
//! - **item classes:** class `n` is `item_classes[n]` (craft classes,
//!   essence classes, and a base's class, each sorted by number);
//! - **mod types:** each distinct `mod_type` ID gets a number, in order of
//!   first use. Only equality of mod types is read (`type_spawn`);
//! - **local stats:** from each stat's `local` and `weapon_local`;
//! - **bases:** the base items by name, in bundle order;
//! - **granted skills, quality types:** from the `poe1` section when the
//!   bundle has one; without it no base grants a skill and no line is a
//!   catalyst quality;
//! - **texts:** properties, labels, compositions, and the granted-skill
//!   wrapper, from the client texts the item rules name.

use std::collections::BTreeMap;

use loom_data::bundle::{Bundle, ScopeRootError};
use loom_data::stat_desc::ScopeError;

use super::base::{BaseRow, ItemData, ItemRules};
use super::grafts::{Graft, Grafts};
use super::mods::{Locality, ModRow, Mods};
use super::properties::{Properties, QualityType};
use crate::resolve::Index;
use crate::resolve::compose::Composition;

/// A bundle's game data, as the item pipeline reads it (module docs).
#[derive(Debug)]
pub struct BundleData<'b> {
    bundle: &'b Bundle,
    bases: BTreeMap<String, Vec<BaseRow>>,
    tags_unknown: usize,
    mods: Mods,
    grafts: Grafts,
    gems: BTreeMap<String, String>,
    properties: Properties,
    compositions: Vec<Composition>,
}

/// Why a scope of the item rules gives no index.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IndexError {
    /// No single stat-description file has the scope's name.
    #[error("{scope}: {error}")]
    Root {
        /// The scope.
        scope: String,
        /// Why.
        error: ScopeRootError,
    },
    /// The scope's chain does not build.
    #[error("{scope}: {error}")]
    Chain {
        /// The scope.
        scope: String,
        /// Why.
        error: ScopeError,
    },
}

impl IndexError {
    /// Stable code (ADR-014); the validator reports both as
    /// `BUNDLE_SCOPE`.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        "BUNDLE_SCOPE"
    }
}

impl<'b> BundleData<'b> {
    /// Builds the pipeline's data from a validated bundle (module docs).
    #[must_use]
    pub fn new(bundle: &'b Bundle) -> Self {
        let mods = mods(bundle);
        let (bases, tags_unknown) = bases(bundle);
        let text = &bundle.item_rules.text;
        let texts = || {
            bundle
                .client_texts
                .iter()
                .map(|t| (t.id.as_str(), t.text.as_str()))
        };
        let text_of = |id: &str| texts().find(|(i, _)| *i == id).map(|(_, t)| t);
        let poe1 = bundle.poe1.as_ref();
        let (grafts, gems) = poe1
            .and_then(|p| p.grafts.as_ref())
            .map(|g| {
                let pairs = g.bases.iter().map(|b| {
                    (
                        b.base.clone(),
                        Graft {
                            id: b.graft.clone(),
                            skill: b.skill.clone(),
                        },
                    )
                });
                let wrapper = text.granted_skill_wrapper.as_deref().and_then(text_of);
                let gems = g
                    .bases
                    .iter()
                    .map(|b| (b.graft.clone(), b.gem.clone()))
                    .collect();
                (Grafts::new(pairs, wrapper, &g.levels), gems)
            })
            .unwrap_or_default();
        let qualities = poe1.and_then(|p| p.quality_types.as_ref()).map(|q| {
            q.iter()
                .map(|q| QualityType {
                    id: q.id.clone(),
                    description: q.description.clone(),
                    stats: q.stats.clone(),
                })
                .collect()
        });
        let properties = Properties::new(text, texts(), qualities);
        let compositions = text
            .compositions
            .iter()
            .filter_map(|(id, stat)| Composition::new(id, text_of(id)?, stat, 1))
            .collect();
        Self {
            bundle,
            bases,
            tags_unknown,
            mods,
            grafts,
            gems,
            properties,
            compositions,
        }
    }

    /// The bundle.
    #[must_use]
    pub const fn bundle(&self) -> &'b Bundle {
        self.bundle
    }

    /// The mod rows.
    #[must_use]
    pub const fn mods(&self) -> &Mods {
        &self.mods
    }

    /// Bases that grant a skill.
    #[must_use]
    pub const fn grafts(&self) -> &Grafts {
        &self.grafts
    }

    /// The gem a graft grants, by the graft's ID.
    #[must_use]
    pub fn granted_gem(&self, graft: &str) -> Option<&str> {
        self.gems.get(graft).map(String::as_str)
    }

    /// Property texts, labels, and quality types.
    #[must_use]
    pub const fn properties(&self) -> &Properties {
        &self.properties
    }

    /// The composition wrappers read, in the rules' order.
    #[must_use]
    pub fn compositions(&self) -> &[Composition] {
        &self.compositions
    }

    /// Composition IDs the rules name that were not read (no client text,
    /// or a text of another shape).
    #[must_use]
    pub fn compositions_missing(&self) -> Vec<&str> {
        self.bundle
            .item_rules
            .text
            .compositions
            .iter()
            .map(|(id, _)| id.as_str())
            .filter(|id| self.compositions.iter().all(|c| c.id != *id))
            .collect()
    }

    /// Base rows whose tags are unknown.
    #[must_use]
    pub const fn tags_unknown(&self) -> usize {
        self.tags_unknown
    }

    /// The stat-description index of every scope of the item rules, by
    /// scope name, with the bundle's lookup tables.
    ///
    /// # Errors
    /// [`IndexError`] for the first scope that does not build.
    pub fn indexes(&self) -> Result<BTreeMap<&'b str, Index<'b>>, IndexError> {
        let b = self.bundle;
        let mut out = BTreeMap::new();
        for scope in b.item_rules.scopes() {
            let root = b.scope_root(scope).map_err(|error| IndexError::Root {
                scope: scope.to_owned(),
                error,
            })?;
            let chain = b
                .stat_descriptions
                .scope(root)
                .map_err(|error| IndexError::Chain {
                    scope: scope.to_owned(),
                    error,
                })?;
            out.insert(scope, Index::build_with(&chain, b));
        }
        Ok(out)
    }
}

impl ItemData for BundleData<'_> {
    fn item_rules(&self) -> &ItemRules {
        &self.bundle.item_rules
    }

    fn base_names(&self) -> impl Iterator<Item = &str> {
        self.bases.keys().map(String::as_str)
    }

    fn base(&self, name: &str) -> Option<(&str, &[BaseRow])> {
        self.bases
            .get_key_value(name)
            .map(|(k, v)| (k.as_str(), v.as_slice()))
    }
}

/// Position of each ID in `ids`.
fn numbers<'a>(ids: impl Iterator<Item = &'a str>) -> BTreeMap<&'a str, usize> {
    let mut out = BTreeMap::new();
    for (n, id) in ids.enumerate() {
        out.entry(id).or_insert(n);
    }
    out
}

/// The class numbers of `ids`, sorted, without repeats or unknown IDs.
fn class_numbers(classes: &BTreeMap<&str, usize>, ids: &[String]) -> Vec<i64> {
    let mut out: Vec<i64> = ids
        .iter()
        .filter_map(|c| classes.get(c.as_str()))
        .filter_map(|n| i64::try_from(*n).ok())
        .collect();
    out.sort_unstable();
    out.dedup();
    out
}

fn mods(b: &Bundle) -> Mods {
    let classes = numbers(b.item_classes.iter().map(String::as_str));
    let mut types: BTreeMap<&str, i64> = BTreeMap::new();
    let rows = b
        .mods
        .iter()
        .map(|m| {
            let (unset, stats): (Vec<_>, Vec<_>) =
                m.stats.iter().partition(|s| s.min == 0 && s.max == 0);
            let next = i64::try_from(types.len()).unwrap_or(i64::MAX);
            ModRow {
                id: m.id.clone(),
                domain: m.domain,
                generation: m.generation,
                stats: stats
                    .into_iter()
                    .map(|s| (s.stat.clone(), s.min.min(s.max), s.min.max(s.max)))
                    .collect(),
                unset: unset.into_iter().map(|s| s.stat.clone()).collect(),
                spawn: m.spawn.iter().map(|s| (s.tag.clone(), s.weight)).collect(),
                influence: m.influence,
                craft_classes: class_numbers(&classes, &m.craft_classes),
                essence_only: m.essence_only,
                essence_classes: m
                    .essence_classes
                    .as_ref()
                    .map(|c| class_numbers(&classes, c)),
                mod_type: m
                    .mod_type
                    .as_deref()
                    .map(|t| *types.entry(t).or_insert(next)),
                type_spawn: None,
            }
        })
        .collect();
    let mut mods = Mods::from_rows(rows);
    mods.locality = b
        .stats
        .iter()
        .filter_map(|s| {
            let kind = if s.weapon_local {
                Locality::Weapon
            } else if s.local {
                Locality::Item
            } else {
                return None;
            };
            Some((s.id.clone(), kind))
        })
        .collect();
    mods
}

/// Base rows by name, and the count whose tags are unknown.
fn bases(b: &Bundle) -> (BTreeMap<String, Vec<BaseRow>>, usize) {
    let classes = numbers(b.item_classes.iter().map(String::as_str));
    let mod_rows = numbers(b.mods.iter().map(|m| m.id.as_str()));
    let mut out: BTreeMap<String, Vec<BaseRow>> = BTreeMap::new();
    let mut unknown = 0;
    for x in &b.base_items {
        unknown += usize::from(x.tags.is_none());
        out.entry(x.name.clone()).or_default().push(BaseRow {
            domain: x.domain,
            class: x
                .item_class
                .as_deref()
                .and_then(|c| classes.get(c))
                .and_then(|n| i64::try_from(*n).ok()),
            tags: x.tags.clone(),
            implicits: x
                .implicits
                .iter()
                .filter_map(|i| mod_rows.get(i.as_str()).copied())
                .collect(),
        });
    }
    (out, unknown)
}
