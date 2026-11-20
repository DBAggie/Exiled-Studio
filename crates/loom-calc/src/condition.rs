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
pub struct Flags(u64);

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
    /// Not an attack: spells and other casts (PoB's `ModFlag.Cast`, which
    /// "cast speed" needs).
    pub const CAST: Self = Self(1 << 11);
    /// Fire, cold, or lightning (for an `any` condition).
    pub const ELEMENTAL: Self = Self(Self::FIRE.0 | Self::COLD.0 | Self::LIGHTNING.0);
    /// A physical skill: the skill's own keyword, whatever damage types its
    /// hit deals (the damage type flags above describe one part of a hit).
    pub const SKILL_PHYSICAL: Self = Self(1 << 12);
    /// A fire skill (skill keyword).
    pub const SKILL_FIRE: Self = Self(1 << 13);
    /// A cold skill (skill keyword).
    pub const SKILL_COLD: Self = Self(1 << 14);
    /// A lightning skill (skill keyword).
    pub const SKILL_LIGHTNING: Self = Self(1 << 15);
    /// A chaos skill (skill keyword).
    pub const SKILL_CHAOS: Self = Self(1 << 16);
    /// A fire, cold, or lightning skill (for an `any` condition).
    pub const SKILL_ELEMENTAL: Self =
        Self(Self::SKILL_FIRE.0 | Self::SKILL_COLD.0 | Self::SKILL_LIGHTNING.0);
    /// Damage over time (PoB's `ModFlag.Dot`, Phase 6).
    pub const DOT: Self = Self(1 << 17);
    /// Physical damage over time (PoB's `KeywordFlag.PhysicalDot`).
    pub const PHYSICAL_DOT: Self = Self(1 << 18);
    /// Fire damage over time (PoB's `KeywordFlag.FireDot`).
    pub const FIRE_DOT: Self = Self(1 << 19);
    /// Cold damage over time (PoB's `KeywordFlag.ColdDot`).
    pub const COLD_DOT: Self = Self(1 << 20);
    /// Lightning damage over time (PoB's `KeywordFlag.LightningDot`).
    pub const LIGHTNING_DOT: Self = Self(1 << 21);
    /// Chaos damage over time (PoB's `KeywordFlag.ChaosDot`).
    pub const CHAOS_DOT: Self = Self(1 << 22);
    /// A damaging ailment (PoB's `ModFlag.Ailment` and `KeywordFlag.Ailment`).
    pub const AILMENT: Self = Self(1 << 23);
    /// Ignite (PoB's `KeywordFlag.Ignite`).
    pub const IGNITE: Self = Self(1 << 24);
    /// Bleeding (PoB's `KeywordFlag.Bleed`).
    pub const BLEED: Self = Self(1 << 25);
    /// Poison (PoB's `KeywordFlag.Poison`).
    pub const POISON: Self = Self(1 << 26);
    /// An attack skill (PoB's `KeywordFlag.Attack`, "with Attack Skills";
    /// Phase 6): unlike [`Self::ATTACK`] (`ModFlag.Attack`), an attack's
    /// ailments and damage over time keep it.
    pub const KEYWORD_ATTACK: Self = Self(1 << 27);
    /// A spell skill (PoB's `KeywordFlag.Spell`, "with Spell Skills"; Phase
    /// 6): unlike [`Self::SPELL`] (`ModFlag.Spell`), a spell's ailments and
    /// damage over time keep it.
    pub const KEYWORD_SPELL: Self = Self(1 << 28);
    /// An ailment from a melee hit (PoB's `ModFlag.MeleeHit`, which an
    /// ailment's config gets when its hit is melee; Phase 6).
    pub const MELEE_HIT: Self = Self(1 << 29);
    /// An attack with a weapon, not unarmed (PoB's `ModFlag.Weapon`;
    /// P6-11b: the weapon flags of an attack's hand,
    /// [`crate::poe1::hit::weapon_type_flags`]).
    pub const WEAPON: Self = Self(1 << 30);
    /// An axe (PoB's `ModFlag.Axe`).
    pub const AXE: Self = Self(1 << 31);
    /// A bow (PoB's `ModFlag.Bow`).
    pub const BOW: Self = Self(1 << 32);
    /// A claw (PoB's `ModFlag.Claw`).
    pub const CLAW: Self = Self(1 << 33);
    /// A dagger (PoB's `ModFlag.Dagger`).
    pub const DAGGER: Self = Self(1 << 34);
    /// A mace or sceptre (PoB's `ModFlag.Mace`).
    pub const MACE: Self = Self(1 << 35);
    /// A staff (PoB's `ModFlag.Staff`).
    pub const STAFF: Self = Self(1 << 36);
    /// A sword (PoB's `ModFlag.Sword`).
    pub const SWORD: Self = Self(1 << 37);
    /// A wand (PoB's `ModFlag.Wand`).
    pub const WAND: Self = Self(1 << 38);
    /// Unarmed (PoB's `ModFlag.Unarmed`).
    pub const UNARMED: Self = Self(1 << 39);
    /// A fishing rod (PoB's `ModFlag.Fishing`).
    pub const FISHING: Self = Self(1 << 40);
    /// A melee weapon (PoB's `ModFlag.WeaponMelee`).
    pub const WEAPON_MELEE: Self = Self(1 << 41);
    /// A ranged weapon (PoB's `ModFlag.WeaponRanged`).
    pub const WEAPON_RANGED: Self = Self(1 << 42);
    /// A one-handed weapon (PoB's `ModFlag.Weapon1H`).
    pub const WEAPON_1H: Self = Self(1 << 43);
    /// A two-handed weapon (PoB's `ModFlag.Weapon2H`).
    pub const WEAPON_2H: Self = Self(1 << 44);
    /// A bow skill (PoB's `KeywordFlag.Bow`: an attack whose main hand
    /// holds a bow, `CalcActiveSkill.lua` 406; its ailments keep it).
    pub const KEYWORD_BOW: Self = Self(1 << 45);
    /// An arrow skill (PoB's `KeywordFlag.Arrow`: the skill's
    /// `skill_can_fire_arrows`, `SkillStatMap.lua` 1341 and
    /// `CalcActiveSkill.lua` 412; P6-11c; its ailments and DoT keep it).
    pub const KEYWORD_ARROW: Self = Self(1 << 46);
    /// A totem skill (PoB's `KeywordFlag.Totem`, `CalcActiveSkill.lua` 415:
    /// the skill's `totem` flag; P6-11bk: the skill type `SummonsTotem`).
    pub const KEYWORD_TOTEM: Self = Self(1 << 47);
    /// A trap skill (PoB's `KeywordFlag.Trap`, 417: `trap`, not a totem;
    /// the skill type `Trapped`).
    pub const KEYWORD_TRAP: Self = Self(1 << 48);
    /// A mine skill (PoB's `KeywordFlag.Mine`, 419: `mine`, neither a totem
    /// nor a trap; the skill type `RemoteMined`).
    pub const KEYWORD_MINE: Self = Self(1 << 49);
    /// A skill of the `RangedAttack` type (PoB's `SkillType.RangedAttack`
    /// tag, "Projectile Attack Skills"; P6-11co): the skill's types after
    /// its supports added theirs.
    pub const SKILL_RANGED_ATTACK: Self = Self(1 << 50);
    /// A skill of the `Brand` type (PoB's `SkillType.Brand` tag; P6-11cx).
    pub const SKILL_BRAND: Self = Self(1 << 51);
    /// A skill of the `Mark` type (PoB's `SkillType.Mark` tag; P6-11cx).
    pub const SKILL_MARK: Self = Self(1 << 52);
    /// A skill of the `Melee` type (PoB's `SkillType.Melee` tag; P6-11dw).
    pub const SKILL_MELEE: Self = Self(1 << 53);
    /// A skill that applies a curse (PoB's `KeywordFlag.Curse`, set from
    /// `SkillType.AppliesCurse`, `CalcActiveSkill.lua` 379; P6-11eo).
    pub const KEYWORD_CURSE: Self = Self(1 << 54);
    /// The weapon flags an attack's ailments keep (PoB's
    /// `ModFlag.WeaponMask`, `0x2FFF0000`: the types, melee, ranged, and
    /// two-handed; not one-handed (`0x10000000` is outside the mask) and not
    /// `ModFlag.Weapon`).
    pub const AILMENT_WEAPON_MASK: Self = Self(
        Self::AXE.0
            | Self::BOW.0
            | Self::CLAW.0
            | Self::DAGGER.0
            | Self::MACE.0
            | Self::STAFF.0
            | Self::SWORD.0
            | Self::WAND.0
            | Self::UNARMED.0
            | Self::FISHING.0
            | Self::WEAPON_MELEE.0
            | Self::WEAPON_RANGED.0
            | Self::WEAPON_2H.0,
    );

    /// The raw bits.
    #[must_use]
    pub const fn bits(self) -> u64 {
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

    /// The skill keyword of a damage type.
    #[must_use]
    pub const fn skill_keyword(t: DamageType) -> Self {
        match t {
            DamageType::Physical => Self::SKILL_PHYSICAL,
            DamageType::Fire => Self::SKILL_FIRE,
            DamageType::Cold => Self::SKILL_COLD,
            DamageType::Lightning => Self::SKILL_LIGHTNING,
            DamageType::Chaos => Self::SKILL_CHAOS,
        }
    }

    /// The damage over time keyword of a damage type (PoB's
    /// `KeywordFlag[type .. "Dot"]`).
    #[must_use]
    pub const fn dot_keyword(t: DamageType) -> Self {
        match t {
            DamageType::Physical => Self::PHYSICAL_DOT,
            DamageType::Fire => Self::FIRE_DOT,
            DamageType::Cold => Self::COLD_DOT,
            DamageType::Lightning => Self::LIGHTNING_DOT,
            DamageType::Chaos => Self::CHAOS_DOT,
        }
    }

    /// The flags of `a` that `b` also has.
    #[must_use]
    pub const fn from_bits_and(a: Self, b: Self) -> Self {
        Self(a.0 & b.0)
    }

    /// The set without the flags of `other`.
    #[must_use]
    pub const fn without(self, other: Self) -> Self {
        Self(self.0 & !other.0)
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

    #[test]
    fn skill_keywords_should_be_apart_from_damage_types() {
        let types = [
            DamageType::Physical,
            DamageType::Fire,
            DamageType::Cold,
            DamageType::Lightning,
            DamageType::Chaos,
        ];
        let mut all = Flags::NONE;
        for t in types {
            let k = Flags::skill_keyword(t);
            assert!(!k.is_empty());
            assert!(!all.intersects(k), "{t:?} shares a bit");
            assert!(!k.intersects(Flags::damage(t)));
            all = all | k;
        }
        let damage = types.iter().fold(Flags::NONE, |a, t| a | Flags::damage(*t));
        assert!(!all.intersects(damage | Flags::SPELL | Flags::ATTACK | Flags::CAST));
        assert_eq!(
            Flags::SKILL_ELEMENTAL,
            Flags::SKILL_FIRE | Flags::SKILL_COLD | Flags::SKILL_LIGHTNING
        );
        // A cold skill's physical hit is not cold damage, and a cold hit of
        // a fire skill is not a cold skill's.
        let cold_skill = Conditions::none().with_all(Flags::SKILL_COLD);
        assert_eq!(
            cold_skill.applies(&QueryContext::with_flags(
                Flags::SKILL_COLD | Flags::PHYSICAL
            )),
            Applies::Yes
        );
        assert_eq!(
            cold_skill.applies(&QueryContext::with_flags(Flags::SKILL_FIRE | Flags::COLD)),
            Applies::No
        );
    }
}
