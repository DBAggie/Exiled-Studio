//! An item's base and stat-description scope (Phase 2 plan, step 2b;
//! lifted from Spike B's `resolve_corpus.rs` `Bases`).
//!
//! - **Scope:** each base row's mod domain gives a scope root through
//!   [`ItemRules::scope_of_domain`]: a listed domain's own scope, else the
//!   default. All rows of the base name must give one scope.
//! - **Base:** the import adapter names the base ([`BaseQuery`]); the name
//!   is looked up as written, then the adapter's fallback name, if any.

use std::collections::BTreeSet;

use super::mods::ModRules;

/// Per-game item rules, read from the game's data bundle (Phase 2 plan,
/// decision 5); until step 3 a tool supplies them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemRules {
    /// The stat-description scope root of every domain not listed in
    /// `domain_scopes` (for example `stat_descriptions`).
    pub default_scope: String,
    /// Mod domains with their own scope root; the first entry of a domain
    /// wins.
    pub domain_scopes: Vec<(i64, String)>,
    /// The mod-row filter's rules.
    pub mods: ModRules,
    /// The client texts the pipeline reads lines through.
    pub text: TextRules,
}

/// The client texts (by ID) the pipeline reads item lines through; their
/// texts come from the game data.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextRules {
    /// Wrappers the client composes a modifier line with: `(text ID, the
    /// stat the wrapper stands for)`, in the order they are tried.
    pub compositions: Vec<(String, String)>,
    /// Property texts with numeric placeholders.
    pub property_ids: Vec<String>,
    /// Property labels written as `<label>: <number>`.
    pub label_ids: Vec<String>,
    /// The wrapper of a granted skill's line (`{1}` level, `{0}` name).
    pub granted_skill_wrapper: Option<String>,
}

impl ItemRules {
    /// The scope root of a base row's mod domain.
    #[must_use]
    pub fn scope_of_domain(&self, domain: i64) -> &str {
        self.domain_scopes
            .iter()
            .find(|(d, _)| *d == domain)
            .map_or(self.default_scope.as_str(), |(_, s)| s.as_str())
    }

    /// Every scope root: the default, then the domain scopes in order,
    /// without repeats.
    #[must_use]
    pub fn scopes(&self) -> Vec<&str> {
        let mut out = vec![self.default_scope.as_str()];
        for (_, s) in &self.domain_scopes {
            if !out.contains(&s.as_str()) {
                out.push(s);
            }
        }
        out
    }
}

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
