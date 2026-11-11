//! The scope rules (Phase 3 plan, decision 3): archetype rules on the main
//! skill, the tree-version rule, the excluded mechanics, and the scope
//! file that fixes the tree version, the chosen skills, and which excluded
//! mechanics apply.
//!
//! Every rule reads game data (the bundle) and the build, never the
//! engine, so the engine cannot shrink its own gate.

use std::collections::BTreeSet;

use loom_data::bundle::skills::GrantedEffect;
use serde::{Deserialize, Serialize};

use super::main_skill::MainSkill;

/// Scope file format.
pub const FORMAT: u32 = 1;

/// The base item of a timeless jewel (`BaseItemTypes.Id`): its nodes come
/// from the timeless jewel tables, which the bundle does not carry.
pub const TIMELESS_JEWEL_BASE: &str = "Metadata/Items/Jewels/JewelTimeless";

/// The base item tag prefix of cluster jewels (`expansion_jewel_large`,
/// `_medium`, `_small` in `BaseItemTypes.TagsKeys`): their nodes come from
/// the passive tree expansion tables, which the bundle does not carry.
pub const CLUSTER_JEWEL_TAG: &str = "expansion_jewel_";

/// The skill type only Mirage Archer support requires (its allowed types
/// are exactly this one): a support with that list makes a mirage use the
/// skill.
pub const MIRAGE_TYPE: &str = "MirageArcherCanUse";

/// Which hit the skill is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    /// The skill has the `Spell` type, not `Attack`, and hits.
    Spell,
    /// The skill has the `Attack` type.
    Attack,
}

impl Kind {
    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Spell => "spell",
            Self::Attack => "attack",
        }
    }

    /// The kind of a skill with these types, if it is a hit the slice
    /// covers. A spell hits when PoB's `skillFlags.hit` would be set
    /// (`Modules/CalcActiveSkill.lua`: the `Attack`, `Damage`, or
    /// `Projectile` type), so an aura or a buff spell is no kind.
    #[must_use]
    pub fn of(types: &BTreeSet<String>) -> Option<Self> {
        let has = |t: &str| types.contains(t);
        if has("Attack") {
            Some(Self::Attack)
        } else if has("Spell") && (has("Damage") || has("Projectile")) {
            Some(Self::Spell)
        } else {
            None
        }
    }
}

/// An archetype rule: the main skill is not cast or used by the character
/// itself (ADR-013; plan decision 3). Read from the skill's types after its
/// supports added theirs, and from its active skill row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Archetype {
    /// Neither an attack nor a spell that hits ([`Kind::of`]).
    NotHit,
    /// The active skill is not cast by the character
    /// (`ActiveSkills.IsManuallyCasted` false).
    NotManuallyCast,
    /// `Triggered` or `InbuiltTrigger`.
    Triggered,
    /// `SummonsTotem`.
    Totem,
    /// `Trapped`.
    Trap,
    /// `RemoteMined`.
    Mine,
    /// `Brand`.
    Brand,
    /// Minion types, or `Minion` or `CreatesMinion`.
    Minion,
    /// A support that applies requires exactly [`MIRAGE_TYPE`].
    Mirage,
}

impl Archetype {
    /// Every rule, in report order.
    pub const ALL: [Self; 9] = [
        Self::NotHit,
        Self::NotManuallyCast,
        Self::Triggered,
        Self::Totem,
        Self::Trap,
        Self::Mine,
        Self::Brand,
        Self::Minion,
        Self::Mirage,
    ];

    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::NotHit => "not-hit",
            Self::NotManuallyCast => "not-manually-cast",
            Self::Triggered => "triggered",
            Self::Totem => "totem",
            Self::Trap => "trap",
            Self::Mine => "mine",
            Self::Brand => "brand",
            Self::Minion => "minion",
            Self::Mirage => "mirage",
        }
    }

    /// Whether the rule puts the main skill out of scope.
    #[must_use]
    pub fn applies(self, skill: &MainSkill<'_>) -> bool {
        let has = |t: &str| skill.types.contains(t);
        match self {
            Self::NotHit => Kind::of(&skill.types).is_none(),
            Self::NotManuallyCast => !skill.active.is_some_and(|a| a.manually_cast),
            Self::Triggered => has("Triggered") || has("InbuiltTrigger"),
            Self::Totem => has("SummonsTotem"),
            Self::Trap => has("Trapped"),
            Self::Mine => has("RemoteMined"),
            Self::Brand => has("Brand"),
            Self::Minion => {
                has("Minion")
                    || has("CreatesMinion")
                    || skill.active.is_some_and(|a| !a.minion_types.is_empty())
            }
            Self::Mirage => skill.supports.iter().any(|s| is_mirage(s)),
        }
    }
}

/// Whether a support makes a mirage use the skill ([`MIRAGE_TYPE`]).
fn is_mirage(support: &GrantedEffect) -> bool {
    matches!(support.allowed_types.as_slice(), [t] if t == MIRAGE_TYPE)
}

/// An excluded mechanic (plan decision 3, Q7): content the bundle cannot
/// describe, detected from game data. Which ones apply is the scope
/// file's list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Exclusion {
    /// A jewel of base [`TIMELESS_JEWEL_BASE`] in an allocated socket.
    TimelessJewel,
    /// A jewel whose base has a [`CLUSTER_JEWEL_TAG`] tag in an allocated
    /// socket.
    ClusterJewel,
    /// An allocated passive a `not-carried` rule of the data-parity rules
    /// names (P3-2: buff and skill grants, bloodline links).
    NotCarriedPassive,
}

impl Exclusion {
    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::TimelessJewel => "timeless-jewel",
            Self::ClusterJewel => "cluster-jewel",
            Self::NotCarriedPassive => "not-carried-passive",
        }
    }
}

/// One excluded mechanic of the scope file.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExcludedMechanic {
    /// The mechanic.
    pub id: Exclusion,
    /// Why it is out of the slice.
    pub reason: String,
}

/// The chosen skills, by granted effect ID (`GrantedEffects.Id`).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Chosen {
    /// The self-cast spell.
    pub spell: Option<String>,
    /// The non-triggered attack.
    pub attack: Option<String>,
}

impl Chosen {
    /// The chosen skill of a kind.
    #[must_use]
    pub fn of(&self, kind: Kind) -> Option<&str> {
        match kind {
            Kind::Spell => self.spell.as_deref(),
            Kind::Attack => self.attack.as_deref(),
        }
    }
}

/// The scope file (`docs/phase-3/calc-scope-poe1.json`).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Scope {
    /// [`FORMAT`].
    pub format: u32,
    /// `poe1`.
    pub game: String,
    /// The Steam build of the bundle the file is for.
    pub steam_build: u64,
    /// `proposed` until the maintainer confirms, then `confirmed`.
    pub status: String,
    /// PoB's label of the bundle's passive tree.
    pub tree_version: String,
    /// Why that label is the bundle's tree.
    pub tree_evidence: String,
    /// The fewest in-scope builds a chosen skill needs.
    pub minimum_per_skill: usize,
    /// The chosen skills; `null` until chosen.
    pub skills: Chosen,
    /// The excluded mechanics that apply.
    pub excluded_mechanics: Vec<ExcludedMechanic>,
}

impl Scope {
    /// Reads and checks a scope file: format, game, status, no mechanic
    /// twice, a minimum of at least 1, and non-empty evidence.
    pub fn read(bytes: &[u8]) -> Result<Self, String> {
        let scope: Self = serde_json::from_slice(bytes).map_err(|e| format!("scope file: {e}"))?;
        if scope.format != FORMAT {
            return Err(format!(
                "scope file: format {} (want {FORMAT})",
                scope.format
            ));
        }
        if scope.game != "poe1" {
            return Err(format!("scope file: game {} (only poe1)", scope.game));
        }
        if !matches!(scope.status.as_str(), "proposed" | "confirmed") {
            return Err(format!("scope file: status {}", scope.status));
        }
        if scope.minimum_per_skill == 0 {
            return Err("scope file: minimum_per_skill is 0".to_owned());
        }
        if scope.tree_version.is_empty() || scope.tree_evidence.trim().is_empty() {
            return Err("scope file: the tree version needs a label and evidence".to_owned());
        }
        let mut seen = BTreeSet::new();
        for m in &scope.excluded_mechanics {
            if !seen.insert(m.id) {
                return Err(format!("scope file: {} listed twice", m.id.name()));
            }
            if m.reason.trim().is_empty() {
                return Err(format!("scope file: {} has no reason", m.id.name()));
            }
        }
        Ok(scope)
    }

    /// Whether an excluded mechanic applies.
    #[must_use]
    pub fn excludes(&self, e: Exclusion) -> bool {
        self.excluded_mechanics.iter().any(|m| m.id == e)
    }
}

/// Why a build is out of scope. A build can have several.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Reason {
    /// The build did not import (a stable code).
    Import(&'static str),
    /// The build did not resolve with the bundle (a stable code).
    Resolve(&'static str),
    /// Not a PoE1 build.
    NotPoe1,
    /// No main skill (the problem's name).
    MainSkill(&'static str),
    /// An archetype rule.
    Archetype(Archetype),
    /// The active spec is not typed, so its tree cannot be read.
    UntypedSpec,
    /// The active spec's tree version is not the bundle's.
    TreeVersion,
    /// An allocated node or mastery effect is not in the bundle.
    UnknownPassive,
    /// An excluded mechanic.
    Excluded(Exclusion),
}

impl Reason {
    /// Stable name: `<group>:<name>`.
    #[must_use]
    pub fn name(&self) -> String {
        match self {
            Self::Import(c) => format!("import:{c}"),
            Self::Resolve(c) => format!("resolve:{c}"),
            Self::NotPoe1 => "game:not-poe1".to_owned(),
            Self::MainSkill(p) => format!("main-skill:{p}"),
            Self::Archetype(a) => format!("archetype:{}", a.name()),
            Self::UntypedSpec => "tree:untyped-spec".to_owned(),
            Self::TreeVersion => "tree:version".to_owned(),
            Self::UnknownPassive => "tree:unknown-passive".to_owned(),
            Self::Excluded(e) => format!("excluded:{}", e.name()),
        }
    }

    /// The field state a calculation would give (plan decision 14): the
    /// inputs that the data cannot settle are `indeterminate`; the rest
    /// are `out-of-scope` (calculated as far as the engine goes, not in
    /// the gate's denominator).
    #[must_use]
    pub const fn state(&self) -> &'static str {
        match self {
            Self::TreeVersion | Self::UnknownPassive | Self::MainSkill(_) => "indeterminate",
            _ => "out-of-scope",
        }
    }
}
