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
//! real-bundle test checks every row).
//!
//! **A gem by name (RV-12).** PoB reads "+N to Level of all Raise Spectre
//! Gems" as a keyword that is the gem's name (`gemIsType`, `CalcTools.lua`
//! 163 to 165: the name, or the same gem whatever its transfigured form).
//! The engine names the gem by its base item, which every form of a gem
//! shares ([`GEM_STATS`], [`GemLevels::gem_bonus`]). Any other gem level
//! stat that names a gem stays unmapped: a reason.
//!
//! **Socketed gems (P6-11q).** PoB reads "+N to Level of Socketed <words>
//! Gems" as a `GemProperty` LIST modifier `{ keyword = <the words>, key =
//! "level", value = N }` with the tag `SocketedIn` for the item's slot
//! (`ModParser.lua` 3178 to 3182; no words: `all`). A group tabulates its
//! `GemProperty` modifiers with its slot (`CalcSetup.lua` 1631 to 1638:
//! the group's slot without " Swap"), so the item's levels reach every gem
//! of a group in the item's slot whose gem matches the keyword (`ModStore`
//! 719 to 760, `applyGemMods`). The stats of [`SOCKETED_STATS`] on an item
//! in an equipment slot map to [`Poe1Quantity::SocketedGemLevel`] for that
//! slot ([`InSlot`]); [`GemLevels::bonus`] adds them to the gems of the
//! groups in that slot. The socketed stats that scale with a count ("per
//! Socketed Gem", "per # Player Levels") or hold only with one gem
//! socketed stay unmapped: reasons.

use std::collections::BTreeMap;

use loom_domain::StatId;

use super::Poe1Quantity;
use crate::condition::Conditions;
use crate::ingest::{EffectTemplate, StatMap, StatMapping, TypedMap};
use crate::quantity::{Op, Quantity};
use crate::source::Source;
use crate::store::Effect;
use loom_domain::TypedResolution;

/// The socketed gem level stats and their keyword (module docs): the
/// words of "+# to Level of Socketed <words> Gems", lower-cased (none:
/// `all`), sorted by stat ID. The hex stat's text says "Curse".
pub const SOCKETED_STATS: [(&str, &str); 30] = [
    ("local_socketed_active_skill_gem_level_+", "skill"),
    ("local_socketed_area_of_effect_gem_level_+", "aoe"),
    ("local_socketed_aura_gem_level_+", "aura"),
    ("local_socketed_bow_gem_level_+", "bow"),
    ("local_socketed_chaos_gem_level_+", "chaos"),
    ("local_socketed_cold_gem_level_+", "cold"),
    ("local_socketed_curse_gem_level_+", "curse"),
    ("local_socketed_dexterity_gem_level_+", "dexterity"),
    ("local_socketed_duration_gem_level_+", "duration"),
    ("local_socketed_elemental_gem_level_+", "elemental"),
    ("local_socketed_fire_gem_level_+", "fire"),
    ("local_socketed_gem_level_+", "all"),
    ("local_socketed_golem_gem_level_+", "golem"),
    ("local_socketed_herald_gem_level_+", "herald"),
    ("local_socketed_hex_gem_level_+", "curse"),
    ("local_socketed_intelligence_gem_level_+", "intelligence"),
    ("local_socketed_lightning_gem_level_+", "lightning"),
    ("local_socketed_melee_gem_level_+", "melee"),
    ("local_socketed_minion_gem_level_+", "minion"),
    ("local_socketed_movement_gem_level_+", "movement"),
    ("local_socketed_non_vaal_gem_level_+", "non-vaal"),
    ("local_socketed_physical_gem_level_+", "physical"),
    ("local_socketed_projectile_gem_level_+", "projectile"),
    ("local_socketed_spell_gem_level_+", "spell"),
    ("local_socketed_strength_gem_level_+", "strength"),
    ("local_socketed_support_gem_level_+", "support"),
    ("local_socketed_trap_and_mine_gem_level_+", "trap or mine"),
    ("local_socketed_trap_gem_level_+", "trap"),
    ("local_socketed_vaal_gem_level_+", "vaal"),
    ("local_socketed_warcry_gem_level_+", "warcry"),
];

/// An equipment slot whose item's socketed gem levels reach the groups
/// socketed in it (P6-11q).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GemSlot {
    /// The main hand.
    Weapon1,
    /// The off hand.
    Weapon2,
    /// The helmet.
    Helmet,
    /// The body armour.
    BodyArmour,
    /// The gloves.
    Gloves,
    /// The boots.
    Boots,
}

impl GemSlot {
    /// Every slot, in [`GemSlot`]'s order.
    pub const ALL: [Self; 6] = [
        Self::Weapon1,
        Self::Weapon2,
        Self::Helmet,
        Self::BodyArmour,
        Self::Gloves,
        Self::Boots,
    ];

    /// The state that the main skill's group is socketed in this slot
    /// (P6-11gl; PoB's `SocketedIn` tag on the main skill), which
    /// [`super::states`] settles.
    #[must_use]
    pub const fn main_skill_state(self) -> &'static str {
        match self {
            Self::Weapon1 => "MainSkillSocketedIn:weapon-1",
            Self::Weapon2 => "MainSkillSocketedIn:weapon-2",
            Self::Helmet => "MainSkillSocketedIn:helmet",
            Self::BodyArmour => "MainSkillSocketedIn:body-armour",
            Self::Gloves => "MainSkillSocketedIn:gloves",
            Self::Boots => "MainSkillSocketedIn:boots",
        }
    }

    /// The slot of PoB's slot name, " Swap" dropped as PoB drops it for a
    /// group's slot (`CalcSetup.lua` 1634). `None`: a slot without gem
    /// sockets (jewellery, flasks, jewels).
    #[must_use]
    pub fn of(slot: &str) -> Option<Self> {
        Some(match slot.strip_suffix(" Swap").unwrap_or(slot) {
            "Weapon 1" => Self::Weapon1,
            "Weapon 2" => Self::Weapon2,
            "Helmet" => Self::Helmet,
            "Body Armour" => Self::BodyArmour,
            "Gloves" => Self::Gloves,
            "Boots" => Self::Boots,
            _ => return None,
        })
    }

    /// The slot of an armour piece's slot.
    #[must_use]
    pub const fn of_armour(slot: super::ArmourSlot) -> Self {
        match slot {
            super::ArmourSlot::Helmet => Self::Helmet,
            super::ArmourSlot::Gloves => Self::Gloves,
            super::ArmourSlot::Boots => Self::Boots,
            super::ArmourSlot::BodyArmour => Self::BodyArmour,
            super::ArmourSlot::Weapon2 => Self::Weapon2,
        }
    }

    /// Stable name: PoB's slot name lowercased, spaces as `-`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Weapon1 => "weapon-1",
            Self::Weapon2 => "weapon-2",
            Self::Helmet => "helmet",
            Self::BodyArmour => "body-armour",
            Self::Gloves => "gloves",
            Self::Boots => "boots",
        }
    }
}

/// The quantity of a socketed gem level stat on an item in `slot`, if the
/// stat is one of [`SOCKETED_STATS`].
#[must_use]
pub fn socketed_quantity(stat: &str, slot: GemSlot) -> Option<Quantity> {
    if stat == SOCKETED_COST_STAT {
        return Some(Quantity::Poe1(Poe1Quantity::SocketedCostMultiplier(slot)));
    }
    SOCKETED_STATS
        .binary_search_by(|(id, _)| (*id).cmp(stat))
        .ok()
        .map(|i| Quantity::Poe1(Poe1Quantity::SocketedGemLevel(SOCKETED_STATS[i].0, slot)))
}

/// "Socketed Gems deal #% more Elemental Damage" (P6-11gl; PoB's
/// `ElementalDamage` MORE with `SocketedIn` the item's slot,
/// `ModParser.lua` 1237): the main skill's hit, while it is socketed in the
/// item's slot ([`GemSlot::main_skill_state`]).
pub const SOCKETED_ELEMENTAL_DAMAGE_STAT: &str =
    "local_display_socketed_gems_elemental_damage_+%_final";

/// "Socketed Skill Gems get a #% Cost & Reservation Multiplier" (P6-11bi;
/// the game's stat ID keeps its misspelling).
pub const SOCKETED_COST_STAT: &str = "local_display_socketed_gems_get_mana_multplier_%";

/// The mapping of the lines of an item in an equipment slot: its socketed
/// gem level stats act on the groups in `slot` (module docs); every other
/// stat is `inner`'s.
pub struct InSlot<'a> {
    /// The item's mapping otherwise.
    pub inner: &'a dyn StatMapping,
    /// The item's slot.
    pub slot: GemSlot,
}

impl StatMapping for InSlot<'_> {
    fn map(&self, stat: &StatId) -> StatMap {
        // A support the item grants its socketed gems (P6-11r).
        // P6-11gl: more elemental damage for the main skill's hit while
        // it is socketed here.
        if stat.as_str() == SOCKETED_ELEMENTAL_DAMAGE_STAT {
            return StatMap::Effects(vec![EffectTemplate {
                quantity: Quantity::Damage,
                op: Op::More,
                scale: 1.0,
                divisor: 1.0,
                conditions: Conditions::none()
                    .with_any(crate::condition::Flags::ELEMENTAL)
                    .with_state(
                        crate::condition::ConditionKey::new(self.slot.main_skill_state()),
                        true,
                    ),
            }]);
        }
        let support = super::item_supports::row(stat.as_str())
            .map(|(id, _, _)| Quantity::Poe1(Poe1Quantity::ItemSupport(id, self.slot)));
        // A trigger line whose value is not its chance (open items S3,
        // part 5) is a flag: its value (a cooldown shown in seconds, read
        // back as a range) is not read.
        let op = if super::item_supports::TRIGGER_ROWS
            .iter()
            .any(|(s, _, _)| *s == stat.as_str())
            && !super::item_supports::TRIGGER_CHANCE_STATS.contains(&stat.as_str())
        {
            Op::Flag
        } else {
            Op::Base
        };
        match socketed_quantity(stat.as_str(), self.slot).or(support) {
            Some(quantity) => StatMap::Effects(vec![EffectTemplate {
                quantity,
                op,
                scale: 1.0,
                divisor: 1.0,
                conditions: Conditions::none(),
            }]),
            None => self.inner.map(stat),
        }
    }

    fn typed(&self, resolution: &TypedResolution, source: &Source) -> TypedMap {
        self.inner.typed(resolution, source)
    }

    fn elsewhere(&self, quantity: Quantity) -> bool {
        self.inner.elsewhere(quantity)
    }
}

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

/// The gem level stats that name one gem (module docs), and the gem's
/// base item: "+# to Level of all Raise Spectre Gems".
pub const GEM_STATS: [(&str, &str); 1] = [(
    "raise_spectre_gem_level_+",
    "Metadata/Items/Gems/SkillGemRaiseSpectre",
)];

/// The quantity of a gem level stat, if it is one of [`LEVEL_STATS`] or
/// [`GEM_STATS`].
#[must_use]
pub fn quantity(stat: &str) -> Option<Quantity> {
    LEVEL_STATS
        .binary_search_by(|(id, _)| (*id).cmp(stat))
        .ok()
        .map(|i| LEVEL_STATS[i].0)
        .or_else(|| {
            GEM_STATS
                .iter()
                .find(|(id, _)| *id == stat)
                .map(|(id, _)| *id)
        })
        .map(|id| Quantity::Poe1(Poe1Quantity::GemLevel(id)))
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
    /// The socketed totals per slot and stat of [`SOCKETED_STATS`].
    socketed: BTreeMap<(GemSlot, &'static str), f64>,
    /// The supports items grant (P6-11r): slot, row stat, level, in the
    /// order the effects were added.
    supports: Vec<(GemSlot, &'static str, i64)>,
    /// The cost and reservation multipliers items give the skills in a
    /// slot (P6-11bi), in percent.
    cost: Vec<(GemSlot, f64)>,
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
        let mut socketed = BTreeMap::new();
        let mut supports = Vec::new();
        let mut cost = Vec::new();
        for e in effects {
            // A trigger line's support is a flag (open items S3, part 5).
            let flag_support = e.op == Op::Flag
                && matches!(e.quantity, Quantity::Poe1(Poe1Quantity::ItemSupport(..)));
            if matches!(e.source, Source::Gem { .. }) || (e.op != Op::Base && !flag_support) {
                continue;
            }
            match e.quantity {
                Quantity::Poe1(Poe1Quantity::GemLevel(stat)) => {
                    *totals.entry(stat).or_insert(0.0) += e.value;
                }
                Quantity::Poe1(Poe1Quantity::SocketedGemLevel(stat, slot)) => {
                    *socketed.entry((slot, stat)).or_insert(0.0) += e.value;
                }
                Quantity::Poe1(Poe1Quantity::SocketedCostMultiplier(slot)) => {
                    cost.push((slot, e.value));
                }
                Quantity::Poe1(Poe1Quantity::ItemSupport(stat, slot)) => {
                    let fixed = super::item_supports::row(stat).map_or(0, |(_, l, _)| l);
                    #[expect(
                        clippy::cast_possible_truncation,
                        reason = "a gem level, a small integer stat value"
                    )]
                    let level = if fixed > 0 {
                        fixed
                    } else {
                        e.value.round() as i64
                    };
                    supports.push((slot, stat, level));
                }
                _ => {}
            }
        }
        Self {
            totals,
            socketed,
            supports,
            cost,
        }
    }

    /// The cost and reservation multipliers, in percent, the item in
    /// `slot` gives its socketed skills (P6-11bi); none without a slot.
    #[must_use]
    pub fn cost_multipliers(&self, slot: Option<GemSlot>) -> Vec<f64> {
        self.cost
            .iter()
            .filter(|(s, _)| Some(*s) == slot)
            .map(|(_, v)| *v)
            .collect()
    }

    /// The supports items grant the groups in `slot` (P6-11r): each
    /// granted effect with its level, in PoB's order (the effects' order,
    /// each row's effects in order).
    pub fn item_supports(&self, slot: GemSlot) -> impl Iterator<Item = (&'static str, i64)> + '_ {
        self.supports
            .iter()
            .filter(move |(s, _, _)| *s == slot)
            .flat_map(|&(_, stat, level)| {
                super::item_supports::row(stat)
                    .map_or(&[][..], |(_, _, effects)| effects)
                    .iter()
                    .map(move |e| (*e, level))
            })
    }

    /// The levels a gem with `tags` gains in a group in `slot` (`tags`
    /// `None`: a skill an item grants, which has no gem). Stat values are
    /// integers, so the sum is exact.
    #[must_use]
    pub fn bonus(&self, tags: Option<&[String]>, slot: Option<GemSlot>) -> i64 {
        let Some(tags) = tags else { return 0 };
        let mut sum = 0.0;
        for (id, words) in LEVEL_STATS {
            if let Some(v) = self.totals.get(id)
                && words.iter().all(|w| is_type(tags, w))
            {
                sum += v;
            }
        }
        if let Some(slot) = slot {
            for (id, word) in SOCKETED_STATS {
                if let Some(v) = self.socketed.get(&(slot, id))
                    && is_type(tags, word)
                {
                    sum += v;
                }
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

    /// The levels the gem with the base item `base_item` gains from the
    /// stats that name it ([`GEM_STATS`]).
    #[must_use]
    pub fn gem_bonus(&self, base_item: &str) -> i64 {
        let sum: f64 = GEM_STATS
            .iter()
            .filter(|(_, gem)| *gem == base_item)
            .filter_map(|(id, _)| self.totals.get(id))
            .sum();
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
        self.totals.is_empty() && self.socketed.is_empty() && self.supports.is_empty()
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
        assert_eq!(levels.bonus(Some(&ice_nova), None), 5);
        let herald = tags(&["grants_active_skill", "cold", "herald", "aura"]);
        assert_eq!(levels.bonus(Some(&herald), None), 2);
        let support = tags(&["support", "cold", "critical"]);
        assert_eq!(levels.bonus(Some(&support), None), 2);
        assert_eq!(levels.bonus(None, None), 0);
        assert!(!levels.is_empty());
        assert!(GemLevels::none().is_empty());
        assert_eq!(GemLevels::none().bonus(Some(&ice_nova), None), 0);
    }

    #[test]
    fn a_stat_that_names_a_gem_should_reach_only_that_gem() {
        let stat = GEM_STATS[0].0;
        assert_eq!(
            quantity(stat),
            Some(Quantity::Poe1(Poe1Quantity::GemLevel(stat)))
        );
        let levels = GemLevels::from_effects(&[level(stat, 1.0), level(stat, 2.0)]);
        assert_eq!(levels.gem_bonus(GEM_STATS[0].1), 3);
        assert_eq!(
            levels.gem_bonus("Metadata/Items/Gems/SkillGemRaiseZombie"),
            0
        );
        // It is no tag keyword: a minion spell gem gains nothing by its tags.
        assert_eq!(
            levels.bonus(
                Some(&tags(&["grants_active_skill", "spell", "minion"])),
                None
            ),
            0
        );
    }

    #[test]
    fn a_negative_total_should_lower_the_level() {
        let levels = GemLevels::from_effects(&[level("spell_skill_gem_level_+", -2.0)]);
        assert_eq!(
            levels.bonus(Some(&tags(&["grants_active_skill", "spell"])), None),
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

    fn socketed(stat: &'static str, slot: GemSlot, value: f64) -> Effect {
        Effect {
            quantity: socketed_quantity(stat, slot).expect("a socketed level stat"),
            op: Op::Base,
            value,
            conditions: Conditions::none(),
            source: Source::Rule("test"),
        }
    }

    #[test]
    fn socketed_levels_should_reach_only_their_slots_matching_gems() {
        let levels = GemLevels::from_effects(&[
            socketed("local_socketed_gem_level_+", GemSlot::Helmet, 1.0),
            socketed("local_socketed_support_gem_level_+", GemSlot::Helmet, 2.0),
            socketed("local_socketed_cold_gem_level_+", GemSlot::BodyArmour, 1.0),
            level("cold_skill_gem_level_+", 1.0),
        ]);
        let ice_nova = tags(&["intelligence", "grants_active_skill", "spell", "cold"]);
        let support = tags(&["cold", "support", "intelligence"]);
        assert_eq!(levels.bonus(Some(&ice_nova), Some(GemSlot::Helmet)), 2);
        assert_eq!(levels.bonus(Some(&support), Some(GemSlot::Helmet)), 3);
        assert_eq!(levels.bonus(Some(&ice_nova), Some(GemSlot::BodyArmour)), 2);
        assert_eq!(levels.bonus(Some(&support), Some(GemSlot::BodyArmour)), 1);
        assert_eq!(levels.bonus(Some(&ice_nova), Some(GemSlot::Boots)), 1);
        assert_eq!(levels.bonus(Some(&ice_nova), None), 1);
        assert_eq!(levels.bonus(None, Some(GemSlot::Helmet)), 0);
    }

    #[test]
    fn socketed_table_should_be_sorted_and_slots_named() {
        assert!(SOCKETED_STATS.windows(2).all(|w| w[0].0 < w[1].0));
        assert_eq!(GemSlot::of("Weapon 1 Swap"), Some(GemSlot::Weapon1));
        assert_eq!(GemSlot::of("Body Armour"), Some(GemSlot::BodyArmour));
        assert_eq!(GemSlot::of("Ring 1"), None);
        assert_eq!(GemSlot::of("Weapon 1 Abyssal Socket 1"), None);
        assert!(
            socketed_quantity(
                "local_socketed_gem_level_+_per_filled_socket",
                GemSlot::Helmet
            )
            .is_none()
        );
        let Some(Quantity::Poe1(q)) =
            socketed_quantity("local_socketed_gem_level_+", GemSlot::Gloves)
        else {
            panic!("quantity");
        };
        assert_eq!(
            q.to_string(),
            "socketed-gem-level.gloves.local_socketed_gem_level_+"
        );
    }

    #[test]
    fn a_gems_own_level_stat_should_not_count() {
        let mut own = level("all_skill_gem_level_+", 3.0);
        own.source = Source::Gem { group: 0, gem: 1 };
        assert!(GemLevels::from_effects(&[own]).is_empty());
    }
}
