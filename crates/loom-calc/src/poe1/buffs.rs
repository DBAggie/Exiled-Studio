//! Buffs the config turns on (Phase 3 plan, step 12; P3-12f): Onslaught.
//!
//! What PoB v2.67.2 does:
//! - `buffOnslaught` (`Modules/ConfigOptions.lua` 1010 to 1012), a check,
//!   sets `Condition:Onslaught` in combat; `calcs.initModDB`
//!   (`Modules/CalcSetup.lua` 97) turns it into the `Onslaught` flag;
//! - `calcs.perform` (`Modules/CalcPerform.lua` 682 to 711): with the
//!   flag, the effect is `floor(20 x (1 + inc / 100))`, `inc` the
//!   `OnslaughtEffect` and `BuffEffectOnSelf` INC sums (and a Silver
//!   Flask's effect when one is active), given as increased attack speed,
//!   increased cast speed, and increased movement speed (not a slice
//!   field).
//!
//! What the engine does: with the check on, `Speed` INC 20 for attacks
//! and for casts. Everything else that changes it is a modifier, so a
//! stat ([`DEPENDS_ON`], which must stay reasons: a test), or an active
//! flask, which is a reason on its own (`collect`). Other sources of
//! Onslaught are modifiers or other gems, reasons too.

use loom_domain::{ConfigInputKind, ConfigSet, ConfigValue};

use crate::condition::Flags;
use crate::ingest::Ingest;
use crate::quantity::{Op, Quantity};
use crate::source::Source;
use crate::state::UnsupportedReason;
use crate::store::{Conditions, Effect};

/// The check that grants Onslaught.
pub const ONSLAUGHT_INPUT: &str = "buffOnslaught";

/// Onslaught's increased attack and cast speed without modifiers (PoB's
/// constant, `CalcPerform.lua` 704).
pub const ONSLAUGHT_SPEED: f64 = 20.0;

/// The stats that change Onslaught's effect (PoB's `OnslaughtEffect` and
/// `BuffEffectOnSelf`, `ModParser.lua` 478, 682, 683), sorted: they must
/// stay reasons while the engine adds the constant effect.
pub const DEPENDS_ON: &[&str] = &[
    "buff_effect_on_self_+%",
    "map_monster_and_player_onslaught_effect_+%",
    "onslaught_effect_+%",
];

/// Whether this module handles `input`.
#[must_use]
pub fn is_buff_input(input: &str) -> bool {
    input == ONSLAUGHT_INPUT
}

/// Adds the buffs the active config set turns on (module docs). A value
/// that is not a boolean is a reason under its input (PoB reads any value
/// other than `false` as on).
pub fn buffs(set: Option<&ConfigSet>, ingest: &mut Ingest<'_>) {
    let Some(set) = set else { return };
    // The last input counts; a check's placeholder is not applied.
    let value = set
        .inputs
        .typed()
        .filter(|i| i.kind == ConfigInputKind::Input && i.name == ONSLAUGHT_INPUT)
        .last()
        .map(|i| &i.value);
    match value {
        None | Some(ConfigValue::Boolean(false)) => {}
        Some(ConfigValue::Boolean(true)) => {
            for flags in [Flags::ATTACK, Flags::CAST] {
                ingest.effect(Effect {
                    quantity: Quantity::Speed,
                    op: Op::Increased,
                    value: ONSLAUGHT_SPEED,
                    conditions: Conditions::none().with_all(flags),
                    source: Source::Config {
                        input: ONSLAUGHT_INPUT.to_owned(),
                    },
                });
            }
        }
        Some(ConfigValue::Number(_) | ConfigValue::String(_)) => {
            ingest.reason(UnsupportedReason::UnmodelledConfig {
                input: ONSLAUGHT_INPUT.to_owned(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest::{StatMap, StatMapping};
    use crate::poe1::ArmourSlot;
    use crate::poe1::mapping::{
        HIT_ROWS, LOCAL_ROWS, NO_EFFECT, PieceMapping, Poe1Mapping, ROWS, SkillMapping,
        WEAPON_ROWS, WeaponMapping,
    };
    use loom_domain::StatId;

    /// No mapped or `NoEffect` stat names Onslaught, and the stats that
    /// change its effect stay `Unmapped` through every mapping.
    #[test]
    fn no_mapped_stat_should_grant_or_change_onslaught() {
        assert!(DEPENDS_ON.windows(2).all(|w| w[0] < w[1]));
        let names = |id: &str| id.contains("onslaught");
        let ids = HIT_ROWS
            .iter()
            .map(|r| r.0)
            .chain(ROWS.iter().map(|r| r.0))
            .chain(LOCAL_ROWS.iter().map(|r| r.0))
            .chain(WEAPON_ROWS.iter().map(|r| r.0))
            .chain(NO_EFFECT.iter().map(|r| r.0));
        for id in ids {
            assert!(!names(id), "{id}: revisit poe1::buffs");
        }
        let piece = PieceMapping {
            slot: ArmourSlot::Helmet,
        };
        let mappings: [&dyn StatMapping; 4] = [&Poe1Mapping, &SkillMapping, &piece, &WeaponMapping];
        for stat in DEPENDS_ON {
            for m in mappings {
                assert_eq!(m.map(&StatId::new(*stat)), StatMap::Unmapped, "{stat}");
            }
        }
    }

    #[test]
    fn the_inputs_should_be_apart_from_the_other_tables() {
        assert!(!super::super::config::named_only(ONSLAUGHT_INPUT));
        assert!(!super::super::charges::is_charge_input(ONSLAUGHT_INPUT));
    }
}
