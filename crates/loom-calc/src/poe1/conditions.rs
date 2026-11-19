//! Named conditions the config settles (Phase 3 plan, step 8; P3-8n):
//! PoB's condition checks that only modifiers read, settled from the
//! active config set as [`super::states`] settles the weapon conditions.
//!
//! **PoB v2.67.2** (`Modules/ConfigOptions.lua`): each check option's
//! `apply` adds a `Condition:<name>` flag (the enemy's for an enemy
//! condition), conditioned on `Condition:Combat` or `Condition:Effective`,
//! both on in PoB's main view, and `BuildModList` applies a check only
//! when it is on (`Classes/ConfigTab.lua`). So an unset or `false` check
//! is a settled `false` (none of these is a `defaultState = true` option,
//! which PoB saves only when off: those are `config::DEFAULT_ON`):
//! - `conditionShockedEnemyRecently`: `ShockedEnemyRecently` (1340);
//! - `conditionShatteredEnemyRecently`: `ShatteredEnemyRecently` (1331);
//! - `conditionAtCloseRange`: `AtCloseRange` (1765);
//! - `conditionBlockedRecently`, `conditionBlockedAttackRecently`,
//!   `conditionBlockedSpellRecently`: `BlockedRecently`, set by any of the
//!   three (1410 to 1420; P6-11g), and the last two's own
//!   `BlockedAttackRecently` and `BlockedSpellRecently`;
//! - `conditionEnemyFullLife`: the enemy's `FullLife` (1774);
//! - `conditionEnemyLowLife`: the enemy's `LowLife` (1777; P6-9e, read by
//!   Punishment's hex, `gems::SKILLS`);
//! - `conditionEnemyRareOrUnique`: the enemy's `RareOrUnique` (2057), which
//!   the `Boss`, `Pinnacle`, and `Uber` boss kinds also set (2095, 2129,
//!   2162; the kind as [`super::enemy`] reads it, PoB's default
//!   `Pinnacle` too);
//! - `buffArcaneSurge`: `Condition:ArcaneSurge` (1002), which
//!   `CalcPerform.lua` (686) turns into `AffectedByArcaneSurge` (the buff's
//!   own effect is [`super::buffs`]').
//!
//! A value of another type (PoB would read it otherwise) leaves the
//! condition unsettled, so an effect that needs it is a reason, and
//! [`handles`] leaves the input to `collect`'s `unmodelled-config`.
//!
//! The modifiers that read these conditions are mapping rows with a state
//! (`mapping::STATE_ROWS`). No modifier the engine maps sets one of them
//! (PoB's other setters are unmapped stats). A condition a mapped
//! modifier can also set (being on consecrated ground while stationary)
//! is not settled here: its input stays in `config::NAMED_ONLY`.

use std::collections::BTreeMap;

use loom_domain::{ConfigInputKind, ConfigSet, ConfigValue};

use super::config::last_input as last;
use super::enemy::{self, BossKind};
use crate::condition::ConditionKey;
use crate::input::CalcInput;

/// "if you have Shocked an Enemy Recently".
pub const SHOCKED_ENEMY_RECENTLY: &str = "ShockedEnemyRecently";
/// "if you've Shattered an Enemy Recently".
pub const SHATTERED_ENEMY_RECENTLY: &str = "ShatteredEnemyRecently";
/// "at Close Range".
pub const AT_CLOSE_RANGE: &str = "AtCloseRange";
/// "while stationary" (PoB's `Condition:Stationary`, set when the config's
/// `conditionStationary` count is above 0; P6-10b).
pub const STATIONARY: &str = "Stationary";
/// "if you've stopped taking Damage Over Time Recently" (P6-10b).
pub const STOPPED_TAKING_DOT_RECENTLY: &str = "StoppedTakingDamageOverTimeRecently";
/// "if you've Blocked Recently" (P6-11g).
pub const BLOCKED_RECENTLY: &str = "BlockedRecently";
/// "against Enemies that are on Full Life": the enemy's `FullLife`.
pub const ENEMY_FULL_LIFE: &str = "Enemy:FullLife";
/// The enemy is on low life: the enemy's `LowLife` (P6-9e).
pub const ENEMY_LOW_LIFE: &str = "Enemy:LowLife";
/// "against Unique Enemies": the enemy's `RareOrUnique`.
pub const ENEMY_RARE_OR_UNIQUE: &str = "Enemy:RareOrUnique";
/// "while a Rare or Unique Enemy is Nearby" (P6-10b; PoB's enemy
/// `ActorCondition` on either `NearbyRareOrUniqueEnemy` or `RareOrUnique`,
/// `ModParser.lua` 1776): the enemy's [`ENEMY_RARE_OR_UNIQUE`], or the
/// config's count of nearby rare or unique enemies at least 1
/// (`ConfigOptions.lua` 1231).
pub const ENEMY_RARE_OR_UNIQUE_NEARBY: &str = "Enemy:RareOrUniqueNearby";

/// The config's count of nearby rare or unique enemies.
pub const NEARBY_RARE_OR_UNIQUE_INPUT: &str = "multiplierNearbyRareOrUniqueEnemies";
/// "if you've dealt a Critical Strike Recently" (PoB's `CritRecently`,
/// from `conditionCritRecently`, `ConfigOptions.lua` 1240; close-out).
pub const CRIT_RECENTLY: &str = "CritRecently";
/// "while you have Arcane Surge" (PoB's `AffectedByArcaneSurge`).
pub const ARCANE_SURGE: &str = "AffectedByArcaneSurge";

/// "if you've been Hit Recently" (PoB's `BeenHitRecently`, from
/// `conditionBeenHitRecently` or a `multiplierBeenHitRecently` above 0;
/// P6-10b part 13).
pub const BEEN_HIT_RECENTLY: &str = "BeenHitRecently";
/// "if you haven't been Hit Recently": [`BEEN_HIT_RECENTLY`] negated.
pub const NOT_HIT_RECENTLY: &str = "NotBeenHitRecently";
/// "if you've Killed Recently" (PoB's `KilledRecently`, from
/// `conditionKilledRecently`; P6-11p).
pub const KILLED_RECENTLY: &str = "KilledRecently";
/// "while moving" (PoB's `Moving`, from `conditionMoving`; P6-10b part 13).
pub const MOVING: &str = "Moving";
/// The count of hits taken recently, which sets [`BEEN_HIT_RECENTLY`]
/// above 0 (`ConfigOptions.lua`).
pub const HIT_RECENTLY_COUNT_INPUT: &str = "multiplierBeenHitRecently";
/// The count of enemies killed recently, which sets [`KILLED_RECENTLY`]
/// above 0 (`ConfigOptions.lua`).
pub const KILLED_RECENTLY_COUNT_INPUT: &str = "multiplierKilledRecently";

/// The auras whose `AffectedBy<Name>` condition a row reads (P6-11o),
/// each with its condition: PoB sets it while the aura affects the
/// character (`CalcPerform.lua` 2417 to 2419; a Vaal aura's too, its name
/// without "Vaal "). Settled from the gem table's auras
/// ([`super::gems::affected_by`]).
pub const AFFECTED_BY: [(&str, &str); 7] = [
    ("Anger", "AffectedByAnger"),
    ("Determination", "AffectedByDetermination"),
    ("Grace", "AffectedByGrace"),
    ("Hatred", "AffectedByHatred"),
    ("Precision", "AffectedByPrecision"),
    ("Pride", "AffectedByPride"),
    ("Wrath", "AffectedByWrath"),
];

/// PoB's `Condition:Lifetap`, from its config check `buffLifetap`
/// (`ConfigOptions.lua`; P6-10b part 8: Lifetap's more damage while
/// buffed).
pub const LIFETAP: &str = "Lifetap";

/// PoB's `Condition:ChannellingCyclone`, from its config check
/// `channellingCycloneCheck` (`ConfigOptions.lua` 375; P6-8a: Cyclone's
/// movement speed buff, `poe1::gems`).
pub const CHANNELLING_CYCLONE: &str = "ChannellingCyclone";

/// The check inputs and the condition each sets (module docs), sorted by
/// input (a test checks); an input may set several, and several inputs
/// one: the condition holds when any of its inputs is on (P6-11g).
pub const CHECKS: &[(&str, &str)] = &[
    ("buffArcaneSurge", ARCANE_SURGE),
    ("buffLifetap", LIFETAP),
    ("channellingCycloneCheck", CHANNELLING_CYCLONE),
    ("conditionAtCloseRange", AT_CLOSE_RANGE),
    ("conditionBeenHitRecently", BEEN_HIT_RECENTLY),
    ("conditionBlockedAttackRecently", "BlockedAttackRecently"),
    ("conditionBlockedAttackRecently", BLOCKED_RECENTLY),
    ("conditionBlockedRecently", BLOCKED_RECENTLY),
    ("conditionBlockedSpellRecently", BLOCKED_RECENTLY),
    ("conditionBlockedSpellRecently", "BlockedSpellRecently"),
    ("conditionCritRecently", CRIT_RECENTLY),
    ("conditionEnemyFullLife", ENEMY_FULL_LIFE),
    ("conditionEnemyLowLife", ENEMY_LOW_LIFE),
    ("conditionEnemyRareOrUnique", ENEMY_RARE_OR_UNIQUE),
    ("conditionKilledRecently", KILLED_RECENTLY),
    ("conditionMoving", MOVING),
    ("conditionShatteredEnemyRecently", SHATTERED_ENEMY_RECENTLY),
    ("conditionShockedEnemyRecently", SHOCKED_ENEMY_RECENTLY),
    (
        "conditionStoppedTakingDamageOverTimeRecently",
        STOPPED_TAKING_DOT_RECENTLY,
    ),
];

/// A check's value: `Some(false)` when unset or off, `None` for a value
/// that is not a boolean.
#[must_use]
pub fn check(set: Option<&ConfigSet>, name: &str) -> Option<bool> {
    match set.and_then(|s| last(s, name, ConfigInputKind::Input)) {
        None | Some(ConfigValue::Boolean(false)) => Some(false),
        Some(ConfigValue::Boolean(true)) => Some(true),
        Some(ConfigValue::Number(_) | ConfigValue::String(_)) => None,
    }
}

/// Whether this module settles `input` from `value` (a check as a
/// boolean).
#[must_use]
pub fn handles(input: &str, value: &ConfigValue) -> bool {
    CHECKS.iter().any(|(name, _)| *name == input) && matches!(value, ConfigValue::Boolean(_))
}

/// The conditions the config settles (module docs), into `out`.
pub fn settle(input: &CalcInput<'_>, out: &mut BTreeMap<ConditionKey, bool>) {
    let set = super::collect::active_config(input.build);
    // The stationary count (a number, [`super::buffs::STATIONARY_INPUT`]):
    // above 0 is `Stationary`; a value of another type leaves it unsettled.
    match set.and_then(|s| last(s, super::buffs::STATIONARY_INPUT, ConfigInputKind::Input)) {
        None => {
            out.insert(ConditionKey::new(STATIONARY), false);
        }
        Some(ConfigValue::Number(n)) => {
            out.insert(ConditionKey::new(STATIONARY), n.value().0 > 0.0);
        }
        Some(_) => {}
    }
    let mut seen: BTreeMap<&str, Option<bool>> = BTreeMap::new();
    for &(name, condition) in CHECKS {
        let mut value = check(set, name);
        if condition == ENEMY_RARE_OR_UNIQUE
            && enemy::enemy(input.build, input.data).kind != BossKind::None
        {
            value = value.map(|_| true);
        }
        // An Arcane Surge Support's buff gives Arcane Surge too
        // (`poe1::gems::arcane_surge_support`; close-out).
        if condition == ARCANE_SURGE {
            // Gem levels do not decide which supports apply.
            let levels = super::gem_levels::GemLevels::none();
            if super::gems::arcane_surge_support(input, &levels) {
                value = Some(true);
            }
        }
        // Any of a condition's inputs on sets it; an unreadable input leaves
        // it unsettled unless another sets it (P6-11g).
        let key = ConditionKey::new(condition);
        let seen_before = seen.get(condition).copied().flatten().is_some();
        let merged = match (seen.get(condition).copied(), value) {
            (None, v) => v,
            (Some(Some(true)), _) | (_, Some(true)) => Some(true),
            (Some(None), _) | (_, None) => None,
            (Some(Some(false)), Some(false)) => Some(false),
        };
        seen.insert(condition, merged);
        match merged {
            Some(v) => {
                out.insert(key, v);
            }
            // Only an entry this loop set earlier (P6-7c review).
            None if seen_before => {
                out.remove(&key);
            }
            None => {}
        }
    }
    // A rare or unique enemy nearby (P6-10b): either condition; a count of
    // another type leaves it unsettled unless the enemy is rare or unique.
    let rare = seen.get(ENEMY_RARE_OR_UNIQUE).copied().flatten();
    let nearby =
        match set.and_then(|s| last(s, NEARBY_RARE_OR_UNIQUE_INPUT, ConfigInputKind::Input)) {
            None => Some(false),
            Some(ConfigValue::Number(n)) => Some(n.value().0 >= 1.0),
            Some(_) => None,
        };
    let either = match (rare, nearby) {
        (Some(true), _) | (_, Some(true)) => Some(true),
        (Some(false), Some(false)) => Some(false),
        _ => None,
    };
    if let Some(v) = either {
        out.insert(ConditionKey::new(ENEMY_RARE_OR_UNIQUE_NEARBY), v);
    }
    // The auras affecting the character (P6-11o).
    let levels = super::gem_levels::GemLevels::none();
    let auras = super::gems::affected_by(input, &levels);
    for (name, condition) in AFFECTED_BY {
        out.insert(ConditionKey::new(condition), auras.contains(name));
    }
    // Killed recently: a kill count above 0 sets it too (PoB's
    // `multiplierKilledRecently`, `ConfigOptions.lua`; P6-11p).
    let key = ConditionKey::new(KILLED_RECENTLY);
    let count = match set.and_then(|s| last(s, KILLED_RECENTLY_COUNT_INPUT, ConfigInputKind::Input))
    {
        None => Some(false),
        Some(ConfigValue::Number(n)) => Some(n.value().0 > 0.0),
        Some(_) => None,
    };
    match (out.get(&key).copied(), count) {
        (Some(true), _) | (_, Some(true)) => {
            out.insert(key, true);
        }
        (Some(false), Some(false)) => {}
        _ => {
            out.remove(&key);
        }
    }
    // Hit recently: a count above 0 sets it too; "not hit recently" is its
    // negation, unsettled when it is (P6-10b part 13).
    let key = ConditionKey::new(BEEN_HIT_RECENTLY);
    let count = match set.and_then(|s| last(s, HIT_RECENTLY_COUNT_INPUT, ConfigInputKind::Input)) {
        None => Some(false),
        Some(ConfigValue::Number(n)) => Some(n.value().0 > 0.0),
        Some(_) => None,
    };
    let hit = match (out.get(&key).copied(), count) {
        (Some(true), _) | (_, Some(true)) => Some(true),
        (Some(false), Some(false)) => Some(false),
        _ => None,
    };
    match hit {
        Some(v) => {
            out.insert(key, v);
            out.insert(ConditionKey::new(NOT_HIT_RECENTLY), !v);
        }
        None => {
            out.remove(&key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_checks_should_be_sorted_and_apart_from_the_named_only_inputs() {
        assert!(CHECKS.windows(2).all(|w| w[0] < w[1]));
        for (input, _) in CHECKS {
            assert!(!super::super::config::named_only(input), "{input}");
        }
    }

    #[test]
    fn only_a_checks_boolean_should_be_handled() {
        assert!(handles(
            "conditionAtCloseRange",
            &ConfigValue::Boolean(false)
        ));
        assert!(!handles(
            "conditionAtCloseRange",
            &ConfigValue::String("x".into())
        ));
        assert!(!handles("conditionFullLife", &ConfigValue::Boolean(true)));
        assert!(!handles("conditionStationary", &ConfigValue::Boolean(true)));
    }
}
