//! Named conditions the config settles (Phase 3 plan, step 8; P3-8n):
//! PoB's condition checks that only modifiers read, settled from the
//! active config set as [`super::states`] settles the weapon conditions.
//!
//! **PoB v2.67.2** (`Modules/ConfigOptions.lua`): each check option's
//! `apply` adds a `Condition:<name>` flag (the enemy's for an enemy
//! condition), conditioned on `Condition:Combat` or `Condition:Effective`,
//! both on in PoB's main view, and `BuildModList` applies a check only
//! when it is on (`Classes/ConfigTab.lua`). So an unset or `false` check
//! is a settled `false`:
//! - `conditionShockedEnemyRecently`: `ShockedEnemyRecently` (1340);
//! - `conditionShatteredEnemyRecently`: `ShatteredEnemyRecently` (1331);
//! - `conditionAtCloseRange`: `AtCloseRange` (1765);
//! - `conditionEnemyFullLife`: the enemy's `FullLife` (1774);
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
/// "against Enemies that are on Full Life": the enemy's `FullLife`.
pub const ENEMY_FULL_LIFE: &str = "Enemy:FullLife";
/// "against Unique Enemies": the enemy's `RareOrUnique`.
pub const ENEMY_RARE_OR_UNIQUE: &str = "Enemy:RareOrUnique";
/// "while you have Arcane Surge" (PoB's `AffectedByArcaneSurge`).
pub const ARCANE_SURGE: &str = "AffectedByArcaneSurge";

/// The check inputs and the condition each sets (module docs), sorted by
/// input (a test checks).
pub const CHECKS: &[(&str, &str)] = &[
    ("buffArcaneSurge", ARCANE_SURGE),
    ("conditionAtCloseRange", AT_CLOSE_RANGE),
    ("conditionEnemyFullLife", ENEMY_FULL_LIFE),
    ("conditionEnemyRareOrUnique", ENEMY_RARE_OR_UNIQUE),
    ("conditionShatteredEnemyRecently", SHATTERED_ENEMY_RECENTLY),
    ("conditionShockedEnemyRecently", SHOCKED_ENEMY_RECENTLY),
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
    for &(name, condition) in CHECKS {
        let mut value = check(set, name);
        if condition == ENEMY_RARE_OR_UNIQUE
            && enemy::enemy(input.build, input.data).kind != BossKind::None
        {
            value = value.map(|_| true);
        }
        if let Some(v) = value {
            out.insert(ConditionKey::new(condition), v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_checks_should_be_sorted_and_apart_from_the_named_only_inputs() {
        assert!(CHECKS.windows(2).all(|w| w[0].0 < w[1].0));
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
