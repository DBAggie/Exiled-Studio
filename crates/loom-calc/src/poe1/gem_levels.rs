//! Gem levels (Phase 3 plan, step 8; P3-8i): "+1 to Level of all Cold
//! Skill Gems" and its kin raise the level of every gem of the build whose
//! tags match.
//!
//! What PoB v2.67.2 does:
//! - `Modules/ModParser.lua` 3280 to 3290 reads "+N to level of all <words>
//!   gems" as a `GemProperty` LIST modifier `{ keywordList = <the words>,
//!   key = "level", value = N }` (a single gem's name gives `keyword`
//!   instead: not modelled here);
//! - `Modules/CalcSetup.lua` 1641 and 1769 tabulate the `GemProperty`
//!   modifiers once per group, after the passives, items, and config are
//!   in the modifier database; `applyGemMods` (397 to 429) adds `value` to
//!   the level of each support and active effect whose gem matches every
//!   keyword (`calcLib.gemIsType`, `Modules/CalcTools.lua` 164 to 176);
//! - the gem is the gem instance's `gemData`: a gem given by `gemId`. A
//!   skill an item grants (only `skillId`) has none, as `data.gemForSkill`
//!   is keyed by the granted effect's table, not its ID (`Modules/Data.lua`
//!   1094; `Classes/SkillsTab.lua` 1182), so it gains no level;
//! - `calcLib.validateGemLevel` then clamps a level the effect does not
//!   have ([`super::skill`]).
//!
//! **The engine.** The stats of [`LEVEL_STATS`] map to
//! [`Poe1Quantity::GemLevel`] (`Base`, unconditional). [`GemLevels`] sums
//! them from the collected effects, and [`super::skill::main_skill`] and
//! [`super::skill::group_skills`] add each gem's matching total to its
//! level, so every reader of a gem's level (its stats, its stat set's
//! level row, its costs and reservation, a support's cost multiplier)
//! sees the level PoB uses. The gem's tags are its gem effect's
//! (`GemEffects.Tags`, which PoB's exporter writes as `gem.tags`).
//!
//! Each row's keywords are the words of its English description between
//! "Level of all" and "Gems", lower-cased, as PoB's pattern reads them (a
//! real-bundle test checks every row). Gem level stats that depend on the
//! socket ("+1 to Level of Socketed Gems") or name a gem stay unmapped:
//! reasons.

use std::collections::BTreeMap;

use super::Poe1Quantity;
use crate::quantity::{Op, Quantity};
use crate::source::Source;
use crate::store::Effect;

/// The global gem level stats and their keywords (module docs), sorted by
/// stat ID.
pub const LEVEL_STATS: [(&str, &[&str]); 23] = [
    ("all_skill_gem_level_+", &["skill"]),
    ("aura_skill_gem_level_+", &["aura", "skill"]),
    ("chaos_skill_gem_level_+", &["chaos", "skill"]),
    (
        "chaos_spell_skill_gem_level_+",
        &["chaos", "spell", "skill"],
    ),
    ("cold_skill_gem_level_+", &["cold", "skill"]),
    ("cold_spell_skill_gem_level_+", &["cold", "spell", "skill"]),
    ("critical_support_gem_level_+", &["critical", "support"]),
    ("curse_skill_gem_level_+", &["curse", "skill"]),
    ("dexterity_skill_gem_level_+", &["dexterity", "skill"]),
    ("fire_skill_gem_level_+", &["fire", "skill"]),
    ("fire_spell_skill_gem_level_+", &["fire", "spell", "skill"]),
    ("herald_skill_gem_level_+", &["herald", "skill"]),
    ("intelligence_skill_gem_level_+", &["intelligence", "skill"]),
    ("lightning_skill_gem_level_+", &["lightning", "skill"]),
    (
        "lightning_spell_skill_gem_level_+",
        &["lightning", "spell", "skill"],
    ),
    ("link_skill_gem_level_+", &["link", "skill"]),
    ("melee_skill_gem_level_+", &["melee", "skill"]),
    ("minion_skill_gem_level_+", &["minion", "skill"]),
    (
        "non_exceptional_support_gem_level_+",
        &["non-exceptional", "support"],
    ),
    ("physical_skill_gem_level_+", &["physical", "skill"]),
    (
        "physical_spell_skill_gem_level_+",
        &["physical", "spell", "skill"],
    ),
    ("spell_skill_gem_level_+", &["spell", "skill"]),
    ("strength_skill_gem_level_+", &["strength", "skill"]),
];

/// The quantity of a gem level stat, if it is one of [`LEVEL_STATS`].
#[must_use]
pub fn quantity(stat: &str) -> Option<Quantity> {
    LEVEL_STATS
        .binary_search_by(|(id, _)| (*id).cmp(stat))
        .ok()
        .map(|i| Quantity::Poe1(Poe1Quantity::GemLevel(LEVEL_STATS[i].0)))
}

/// `calcLib.gemIsType(gem, keyword, true)` for a gem with `tags`, without
/// its name comparisons (no row names a gem).
#[must_use]
pub fn is_type(tags: &[String], keyword: &str) -> bool {
    let has = |t: &str| tags.iter().any(|g| g == t);
    match keyword {
        "all" => true,
        "elemental" => has("fire") || has("cold") || has("lightning"),
        "aoe" => has("area"),
        "trap or mine" => has("trap") || has("mine"),
        "active skill" | "grants_active_skill" | "skill" => {
            has("grants_active_skill") && !has("support")
        }
        "non-vaal" => !has("vaal"),
        "non-exceptional" => !has("exceptional"),
        other => has(other),
    }
}

/// The build's gem level totals per stat of [`LEVEL_STATS`].
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GemLevels {
    totals: BTreeMap<&'static str, f64>,
}

impl GemLevels {
    /// No gem level modifiers.
    #[must_use]
    pub fn none() -> Self {
        Self::default()
    }

    /// The totals of the `Base` effects on [`Poe1Quantity::GemLevel`]
    /// among `effects` (the mapping gives them no conditions).
    ///
    /// Read twice, by `collect` once every source of these effects is in
    /// and by the hit phase from the finished store: the steps after the
    /// gems (charges, damage taken, buffs) add none, so both see the same
    /// totals. Effects from a gem are left out: a gem's own stat cannot
    /// raise the levels its stats were read at (PoB tabulates the
    /// `GemProperty` modifiers before it creates any skill).
    #[must_use]
    pub fn from_effects(effects: &[Effect]) -> Self {
        let mut totals = BTreeMap::new();
        for e in effects {
            if matches!(e.source, Source::Gem { .. }) {
                continue;
            }
            if let (Quantity::Poe1(Poe1Quantity::GemLevel(stat)), Op::Base) = (e.quantity, e.op) {
                *totals.entry(stat).or_insert(0.0) += e.value;
            }
        }
        Self { totals }
    }

    /// The levels a gem with `tags` gains (`None`: a skill an item grants,
    /// which has no gem). Stat values are integers, so the sum is exact.
    #[must_use]
    pub fn bonus(&self, tags: Option<&[String]>) -> i64 {
        let Some(tags) = tags else { return 0 };
        let mut sum = 0.0;
        for (id, words) in LEVEL_STATS {
            if let Some(v) = self.totals.get(id)
                && words.iter().all(|w| is_type(tags, w))
            {
                sum += v;
            }
        }
        debug_assert!(sum.fract() == 0.0, "gem levels are whole: {sum}");
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a sum of integer stat values, far inside i64"
        )]
        let levels = sum.round() as i64;
        levels
    }

    /// Whether the build has any gem level modifier.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.totals.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::condition::Conditions;

    fn tags(t: &[&str]) -> Vec<String> {
        t.iter().map(|s| (*s).to_owned()).collect()
    }

    fn level(stat: &'static str, value: f64) -> Effect {
        Effect {
            quantity: quantity(stat).expect("a level stat"),
            op: Op::Base,
            value,
            conditions: Conditions::none(),
            source: Source::Rule("test"),
        }
    }

    #[test]
    fn the_table_should_be_sorted_and_unique() {
        assert!(LEVEL_STATS.windows(2).all(|w| w[0].0 < w[1].0));
        for (id, words) in LEVEL_STATS {
            assert!(id.ends_with("_gem_level_+"), "{id}");
            assert!(!words.is_empty(), "{id}");
        }
    }

    #[test]
    fn skill_should_mean_an_active_gem_not_a_support() {
        let ice_nova = tags(&["intelligence", "grants_active_skill", "spell", "cold"]);
        let support = tags(&["cold", "support", "intelligence"]);
        assert!(is_type(&ice_nova, "skill"));
        assert!(!is_type(&support, "skill"));
        assert!(is_type(&support, "support"));
        assert!(is_type(&ice_nova, "elemental"));
        assert!(is_type(&ice_nova, "non-exceptional"));
        assert!(!is_type(
            &tags(&["support", "exceptional"]),
            "non-exceptional"
        ));
        assert!(!is_type(&ice_nova, "fire"));
    }

    #[test]
    fn a_gem_should_gain_every_matching_total() {
        let levels = GemLevels::from_effects(&[
            level("cold_skill_gem_level_+", 1.0),
            level("cold_spell_skill_gem_level_+", 2.0),
            level("cold_spell_skill_gem_level_+", 1.0),
            level("all_skill_gem_level_+", 1.0),
            level("fire_skill_gem_level_+", 4.0),
            level("critical_support_gem_level_+", 2.0),
        ]);
        let ice_nova = tags(&["intelligence", "grants_active_skill", "spell", "cold"]);
        assert_eq!(levels.bonus(Some(&ice_nova)), 5);
        let herald = tags(&["grants_active_skill", "cold", "herald", "aura"]);
        assert_eq!(levels.bonus(Some(&herald)), 2);
        let support = tags(&["support", "cold", "critical"]);
        assert_eq!(levels.bonus(Some(&support)), 2);
        assert_eq!(levels.bonus(None), 0);
        assert!(!levels.is_empty());
        assert!(GemLevels::none().is_empty());
        assert_eq!(GemLevels::none().bonus(Some(&ice_nova)), 0);
    }

    #[test]
    fn a_negative_total_should_lower_the_level() {
        let levels = GemLevels::from_effects(&[level("spell_skill_gem_level_+", -2.0)]);
        assert_eq!(
            levels.bonus(Some(&tags(&["grants_active_skill", "spell"]))),
            -2
        );
    }

    #[test]
    fn other_quantities_should_not_count() {
        let mut other = level("all_skill_gem_level_+", 3.0);
        other.op = Op::Increased;
        let levels = GemLevels::from_effects(&[other]);
        assert!(levels.is_empty());
    }

    #[test]
    fn a_gems_own_level_stat_should_not_count() {
        let mut own = level("all_skill_gem_level_+", 3.0);
        own.source = Source::Gem { group: 0, gem: 1 };
        assert!(GemLevels::from_effects(&[own]).is_empty());
    }
}
