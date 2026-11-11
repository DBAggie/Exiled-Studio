//! An item's base and stat-description scope (Phase 2 plan, step 2b;
//! lifted from Spike B's `resolve_corpus.rs` `Bases`).
//!
//! - **Scope:** each base row's mod domain gives a scope root through
//!   [`ItemRules::scope_of_domain`]: a listed domain's own scope, else the
//!   default. All rows of the base name must give one scope.
//! - **Base:** the import adapter names the base ([`BaseQuery`]); the name
//!   is looked up as written, then the adapter's fallback name, if any.

use std::collections::BTreeSet;

pub use loom_data::rules::{ItemRules, TextRules};

/// One base item row of a base name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BaseRow {
    /// Mod domain (0 when the data has none, which no rule lists).
    pub domain: i64,
    /// Item class row.
    pub class: Option<i64>,
    /// Tags (item type chain plus the row's own), `None` when unknown.
    pub tags: Option<Vec<String>>,
    /// Implicit mod rows.
    pub implicits: Vec<usize>,
}

/// The game data the item pipeline reads.
pub trait ItemData {
    /// The game's item rules.
    fn item_rules(&self) -> &ItemRules;

    /// Every base name, sorted, without repeats.
    fn base_names(&self) -> impl Iterator<Item = &str>;

    /// A base name as the data stores it, with its rows in data order
    /// (never empty); `None` when no row has the name. An empty slice is
    /// read as no base.
    fn base(&self, name: &str) -> Option<(&str, &[BaseRow])>;
}

/// The base name an import adapter found for an item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BaseQuery<'q> {
    /// The name to look up.
    pub name: &'q str,
    /// A name to look up when `name` is not a base (an adapter's own rule,
    /// for example a format suffix removed).
    pub fallback: Option<&'q str>,
}

/// Why an item has no base.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaseError {
    /// Neither name is a base name.
    NotABase,
    /// The rows of the name give more than one scope.
    ScopeConflict,
}

impl BaseError {
    /// Stable code (ADR-014).
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::NotABase => "IMPORT_BASE_NOT_FOUND",
            Self::ScopeConflict => "IMPORT_BASE_SCOPE_CONFLICT",
        }
    }
}

/// An item's base as [`find_base`] found it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoundBase<'d> {
    /// The base name found.
    pub name: &'d str,
    /// The scope root.
    pub scope: &'d str,
    /// Whether the fallback name was used.
    pub fallback_used: bool,
    /// The base's rows.
    pub rows: &'d [BaseRow],
}

/// Finds a base: `query.name`, else `query.fallback`, and its one scope.
///
/// # Errors
/// [`BaseError`] when no name is a base or the rows disagree on the scope.
pub fn find_base<'d, D: ItemData>(
    data: &'d D,
    query: BaseQuery<'_>,
) -> Result<FoundBase<'d>, BaseError> {
    let ((name, rows), fallback_used) = match (data.base(query.name), query.fallback) {
        (Some(found), _) => (found, false),
        (None, Some(f)) => (data.base(f).ok_or(BaseError::NotABase)?, true),
        (None, None) => return Err(BaseError::NotABase),
    };
    if rows.is_empty() {
        return Err(BaseError::NotABase);
    }
    let rules = data.item_rules();
    let scopes: BTreeSet<&str> = rows
        .iter()
        .map(|r| rules.scope_of_domain(r.domain))
        .collect();
    let mut scopes = scopes.into_iter();
    let (Some(scope), None) = (scopes.next(), scopes.next()) else {
        return Err(BaseError::ScopeConflict);
    };
    Ok(FoundBase {
        name,
        scope,
        fallback_used,
        rows,
    })
}

#[cfg(test)]
mod tests;
