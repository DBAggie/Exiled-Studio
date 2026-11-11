//! Per-game item rules (Phase 2 plan, decision 5): the game-specific
//! numbers and names the item import pipeline (`loom_parser::import`)
//! reads, as data. A data bundle carries them for its game
//! (`data/item_rules.json`); nothing in the shared crates hard-codes them.
//! Moved here from `loom-parser` in P2-3a so the bundle model can hold
//! them (the crate graph has `loom-data` below `loom-parser`).

use loom_domain::{InfluenceId, ModSection};
use serde::{Deserialize, Serialize};

/// A [`ModSection`] written as its name (`enchant`, `implicit`,
/// `explicit`, or another section's name).
mod section {
    use loom_domain::ModSection;
    use serde::{Deserialize, Deserializer, Serializer};

    pub(super) fn serialize<S: Serializer>(s: &ModSection, out: S) -> Result<S::Ok, S::Error> {
        out.serialize_str(s.name())
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(input: D) -> Result<ModSection, D::Error> {
        let name = String::deserialize(input)?;
        Ok(match name.as_str() {
            "enchant" => ModSection::Enchant,
            "implicit" => ModSection::Implicit,
            "explicit" => ModSection::Explicit,
            _ => ModSection::Other(name),
        })
    }
}

/// Per-game item rules, read from the game's data bundle (Phase 2 plan,
/// decision 5); until step 3 a tool supplies them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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

/// Which mod domains a row may have, relative to a base row's domain.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DomainMatch {
    /// The base's own domain.
    pub own: bool,
    /// These domains, whatever the base's.
    pub fixed: Vec<i64>,
    /// `(row domain, base domains)`: the row domain on bases of these
    /// domains.
    pub related: Vec<(i64, Vec<i64>)>,
}

impl DomainMatch {
    /// Whether a row of domain `row` fits a base row of domain `base`.
    #[must_use]
    pub fn admits(&self, row: i64, base: i64) -> bool {
        (self.own && row == base)
            || self.fixed.contains(&row)
            || self
                .related
                .iter()
                .any(|(r, bases)| *r == row && bases.contains(&base))
    }
}

/// One way a row fits a line.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FitClause {
    /// Generation types the row may have.
    pub generations: Vec<i64>,
    /// Domains the row may have.
    pub domains: DomainMatch,
    /// The row's crafting class restriction must be empty or name the
    /// base's item class.
    pub craft_class: bool,
    /// An essence-only row needs the base's class among the classes an
    /// essence puts it on, when known.
    pub essence_class: bool,
}

/// The fit rule of the lines of one section (and flag, and rarity).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FitRule {
    /// The section, by name.
    #[serde(with = "section")]
    pub section: ModSection,
    /// A line flag (prefix key) the line must carry, if any.
    #[serde(default)]
    pub flag: Option<String>,
    /// Only for unique items (`Some(true)`), only for others
    /// (`Some(false)`), or both.
    #[serde(default)]
    pub unique: Option<bool>,
    /// The row fits when one clause holds.
    pub clauses: Vec<FitClause>,
}

/// Lines of one section that carry none of some flags.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Unflagged {
    /// The section, by name.
    #[serde(with = "section")]
    pub section: ModSection,
    /// The flags.
    pub flags: Vec<String>,
}

/// The base tags that carry the properties local stats change.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalCarriers {
    /// The tag of bases with weapon properties (weapon-local stats).
    pub weapon: String,
    /// The tag of bases with the defence properties other local stats
    /// change (other local stats).
    pub item: String,
}

/// Per-game rules of the filter (part of [`ItemRules`]). Check a
/// game's rules with [`ModRules::validate`]: rules with no fit rule would
/// let no row fit any line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModRules {
    /// Fit rules. Order matters: for a line, the first rule whose section,
    /// flag, and rarity match is the only one used.
    pub fits: Vec<FitRule>,
    /// On these lines, rows of `generations` do not count as spawning
    /// (they have spawn weights where the rows the lines usually carry
    /// have none, so counting them would favour them by construction).
    pub spawn_exclusion: Unflagged,
    /// Their generation types.
    pub spawn_excluded_generations: Vec<i64>,
    /// Lines whose candidates the base's own implicit rows may decide
    /// (`base-implicit`).
    pub base_implicit: Unflagged,
    /// Line flags of lines that do not spawn (`spawn-tags` is skipped).
    pub no_spawn_flags: Vec<String>,
    /// Carrier tags for `local-stat`.
    pub local_carriers: LocalCarriers,
    /// The names of the sections this game's import adapter writes besides
    /// the shared ones ([`ModSection::Other`]); a rule may name only these
    /// ([`ModRules::validate`]), so a typo cannot make a section fit
    /// nothing.
    pub sections: Vec<String>,
    /// Influence names (the domain's [`InfluenceId`]) to the rows'
    /// influence values.
    pub influences: Vec<(String, i64)>,
}

/// Why a game's [`ModRules`] cannot be used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModRulesError {
    /// No fit rule: no row would fit any line.
    NoFitRules,
    /// A fit rule (by index) has no clause, so its lines fit no row.
    EmptyRule(usize),
    /// A rule (by index) can never apply: an earlier rule of the same
    /// section has no flag and covers the same rarity.
    Shadowed(usize),
    /// A rule or gate names a section that is not shared and not in
    /// [`ModRules::sections`].
    UnknownSection(String),
}

impl ModRulesError {
    /// Stable code (ADR-014).
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::NoFitRules => "IMPORT_MOD_RULES_EMPTY",
            Self::EmptyRule(_) => "IMPORT_MOD_RULES_EMPTY_RULE",
            Self::Shadowed(_) => "IMPORT_MOD_RULES_SHADOWED",
            Self::UnknownSection(_) => "IMPORT_MOD_RULES_UNKNOWN_SECTION",
        }
    }
}

impl ModRules {
    /// Checks the rules (see [`ModRulesError`]).
    ///
    /// # Errors
    /// The first problem found.
    pub fn validate(&self) -> Result<(), ModRulesError> {
        if self.fits.is_empty() {
            return Err(ModRulesError::NoFitRules);
        }
        let named = self
            .fits
            .iter()
            .map(|r| &r.section)
            .chain([&self.spawn_exclusion.section, &self.base_implicit.section]);
        for section in named {
            if let ModSection::Other(name) = section
                && !self.sections.contains(name)
            {
                return Err(ModRulesError::UnknownSection(name.clone()));
            }
        }
        for (n, rule) in self.fits.iter().enumerate() {
            if rule.clauses.is_empty() {
                return Err(ModRulesError::EmptyRule(n));
            }
            let shadowed = self.fits.iter().take(n).any(|earlier| {
                earlier.section == rule.section
                    && earlier.flag.is_none()
                    && earlier.unique.is_none_or(|u| rule.unique == Some(u))
            });
            if shadowed {
                return Err(ModRulesError::Shadowed(n));
            }
        }
        Ok(())
    }

    /// The rows' influence value of an influence; `None` for an influence
    /// the rows do not name (the item's other influences still count).
    #[must_use]
    pub fn influence_value(&self, id: &InfluenceId) -> Option<i64> {
        self.influences
            .iter()
            .find(|(n, _)| n == id.as_str())
            .map(|(_, v)| *v)
    }
}
