//! When an effect applies (Phase 3 plan, step 4): skill and damage flags,
//! and named conditions the build or its config settles.
//!
//! An effect applies to a query when the query's flags hold all of the
//! effect's `all` flags and, if `any` is not empty, at least one of them,
//! and every named condition has the value the effect needs. A named
//! condition the query's context does not settle is unknown: the effect is
//! not applied and the result is `Partial` with `UnmodelledCondition`,
//! never a silent zero (ADR-014).

use core::fmt;
use core::ops::BitOr;
use std::collections::BTreeMap;

use crate::quantity::DamageType;

/// A set of skill and damage flags.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Flags(u32);

impl Flags {
    /// No flag.
    pub const NONE: Self = Self(0);
    /// A spell.
    pub const SPELL: Self = Self(1);
    /// An attack.
    pub const ATTACK: Self = Self(1 << 1);
    /// A hit.
    pub const HIT: Self = Self(1 << 2);
    /// Melee.
    pub const MELEE: Self = Self(1 << 3);
    /// Area.
    pub const AREA: Self = Self(1 << 4);
    /// Projectile.
    pub const PROJECTILE: Self = Self(1 << 5);
    /// Physical damage.
    pub const PHYSICAL: Self = Self(1 << 6);
    /// Fire damage.
    pub const FIRE: Self = Self(1 << 7);
    /// Cold damage.
    pub const COLD: Self = Self(1 << 8);
    /// Lightning damage.
    pub const LIGHTNING: Self = Self(1 << 9);
    /// Chaos damage.
    pub const CHAOS: Self = Self(1 << 10);
    /// Fire, cold, or lightning (for an `any` condition).
    pub const ELEMENTAL: Self = Self(Self::FIRE.0 | Self::COLD.0 | Self::LIGHTNING.0);

    /// The raw bits.
    #[must_use]
    pub const fn bits(self) -> u32 {
        self.0
    }

    /// Both sets.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Whether every flag of `other` is set.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Whether a flag of `other` is set.
    #[must_use]
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    /// Whether no flag is set.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// The flag of a damage type.
    #[must_use]
    pub const fn damage(t: DamageType) -> Self {
        match t {
            DamageType::Physical => Self::PHYSICAL,
            DamageType::Fire => Self::FIRE,
            DamageType::Cold => Self::COLD,
            DamageType::Lightning => Self::LIGHTNING,
            DamageType::Chaos => Self::CHAOS,
        }
    }
}

impl BitOr for Flags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        self.union(rhs)
    }
}

/// A named condition ("on full life", a config input's condition), set by
/// a game's rules; the core only compares names.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConditionKey(String);

impl ConditionKey {
    /// A condition by name.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// The name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ConditionKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// What an effect needs to apply.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Conditions {
    /// Flags the query must all have.
    pub all: Flags,
    /// Flags of which the query must have one (no requirement when empty).
    pub any: Flags,
    /// Named conditions and the value each must have.
    pub states: Vec<(ConditionKey, bool)>,
}

impl Conditions {
    /// No condition: applies to every query.
    #[must_use]
    pub fn none() -> Self {
        Self::default()
    }

    /// Requires every flag of `flags`.
    #[must_use]
    pub fn with_all(mut self, flags: Flags) -> Self {
        self.all = self.all | flags;
        self
    }

    /// Requires one of `flags`.
    #[must_use]
    pub fn with_any(mut self, flags: Flags) -> Self {
        self.any = self.any | flags;
        self
    }

    /// Requires the named condition to be `value`.
    #[must_use]
    pub fn with_state(mut self, key: ConditionKey, value: bool) -> Self {
        self.states.push((key, value));
        self
    }

    /// Whether the effect applies in `cx`.
    pub(crate) fn applies(&self, cx: &QueryContext) -> Applies {
        if !cx.flags.contains(self.all) || (!self.any.is_empty() && !cx.flags.intersects(self.any))
        {
            return Applies::No;
        }
        let mut unknown = Vec::new();
        for (key, want) in &self.states {
            match cx.states.get(key) {
                Some(have) if have == want => {}
                Some(_) => return Applies::No,
                None => unknown.push(key.clone()),
            }
        }
        if unknown.is_empty() {
            Applies::Yes
        } else {
            Applies::Unknown(unknown)
        }
    }
}

/// Whether an effect applies to a query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Applies {
    /// It applies.
    Yes,
    /// A flag or a settled condition rules it out.
    No,
    /// Every settled part allows it, but these conditions are not settled.
    Unknown(Vec<ConditionKey>),
}

/// What a query is about: the flags of the thing measured and the named
/// conditions the build and its config settle.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QueryContext {
    /// The flags of the thing measured (for example a spell's fire hit).
    pub flags: Flags,
    /// Named conditions with a settled value. One not listed is unknown.
    pub states: BTreeMap<ConditionKey, bool>,
}

impl QueryContext {
    /// A context with `flags` and no settled condition.
    #[must_use]
    pub fn with_flags(flags: Flags) -> Self {
        Self {
            flags,
            states: BTreeMap::new(),
        }
    }

    /// Settles a named condition.
    #[must_use]
    pub fn with_state(mut self, key: ConditionKey, value: bool) -> Self {
        self.states.insert(key, value);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(name: &str) -> ConditionKey {
        ConditionKey::new(name)
    }

    #[test]
    fn conditions_none_should_apply_to_every_query() {
        assert_eq!(
            Conditions::none().applies(&QueryContext::default()),
            Applies::Yes
        );
    }

    #[test]
    fn all_flags_should_need_every_flag() {
        let c = Conditions::none().with_all(Flags::SPELL | Flags::FIRE);
        assert_eq!(
            c.applies(&QueryContext::with_flags(Flags::SPELL)),
            Applies::No
        );
        assert_eq!(
            c.applies(&QueryContext::with_flags(
                Flags::SPELL | Flags::FIRE | Flags::HIT
            )),
            Applies::Yes
        );
    }

    #[test]
    fn any_flags_should_need_one_flag() {
        let c = Conditions::none().with_any(Flags::ELEMENTAL);
        assert_eq!(
            c.applies(&QueryContext::with_flags(Flags::PHYSICAL)),
            Applies::No
        );
        assert_eq!(
            c.applies(&QueryContext::with_flags(Flags::COLD)),
            Applies::Yes
        );
    }

    #[test]
    fn settled_state_should_decide_and_unsettled_should_be_unknown() {
        let c = Conditions::none().with_state(key("full-life"), true);
        let on = QueryContext::default().with_state(key("full-life"), true);
        let off = QueryContext::default().with_state(key("full-life"), false);
        assert_eq!(c.applies(&on), Applies::Yes);
        assert_eq!(c.applies(&off), Applies::No);
        assert_eq!(
            c.applies(&QueryContext::default()),
            Applies::Unknown(vec![key("full-life")])
        );
    }

    #[test]
    fn a_failed_flag_or_state_should_win_over_an_unknown_state() {
        let c = Conditions::none()
            .with_all(Flags::ATTACK)
            .with_state(key("a"), true)
            .with_state(key("b"), true);
        assert_eq!(c.applies(&QueryContext::default()), Applies::No);
        let cx = QueryContext::with_flags(Flags::ATTACK).with_state(key("b"), false);
        assert_eq!(c.applies(&cx), Applies::No);
    }

    #[test]
    fn flags_should_combine() {
        assert!(Flags::ELEMENTAL.contains(Flags::FIRE));
        assert!(!Flags::ELEMENTAL.contains(Flags::CHAOS));
        assert!(Flags::NONE.is_empty());
        assert_eq!(Flags::damage(DamageType::Cold), Flags::COLD);
        assert_eq!((Flags::SPELL | Flags::HIT).bits(), 0b101);
    }
}
