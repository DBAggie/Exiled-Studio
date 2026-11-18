//! Active flasks (Phase 6, P6-4b): each active flask's buff and its own
//! lines, scaled by flask effect, as PoB v2.67.2 merges them
//! (`Modules/CalcPerform.lua` 1602 to 1790).
//!
//! What PoB does:
//! - **Which flasks.** A flask slot marked active in the item set
//!   (`CalcSetup.lua` 929). Mageblood's magic utility flasks
//!   (`CalcPerform.lua` 1543 to 1564) come from unmapped stats, so reasons.
//! - **The effect.** `inc` is the player's `FlaskEffect` INC sum, plus the
//!   flask's own `FlaskEffect` and `LocalEffect` INC (`Classes/Item.lua`
//!   2400), plus `MagicUtilityFlaskEffect` INC for a magic flask that
//!   recovers neither life nor mana (1667 to 1671). The scale is `1 + inc /
//!   100`, applied with `ScaleAddList` (`Classes/ModStore.lua`;
//!   [`skill_modifiers::scale`]) to the base's buff (the base data's buff
//!   lines, `Data/Bases/flask.lua`, from the game's `Flasks` row) and to the
//!   flask's other lines.
//! - **Merging** (`mergeBuff`, 44 to 66): the buffs of flasks of one base
//!   merge keeping each mod's numerically largest value. Here each stat of
//!   the base's buff is applied once per base, at the scale that gives its
//!   mods the largest value: the largest scale for a positive mod, the
//!   smallest for a negative one (a stat whose mods differ in sign is a
//!   reason). A flask's own lines merge by the unique's name or by the
//!   mods; two active flasks with one key are a reason (PoB keys a magic
//!   flask by its mods in order, so the engine's sorted lines are the more
//!   cautious key).
//! - **Silver Flask's Onslaught** (658 to 678): with a Silver Flask active,
//!   Onslaught's speed is `floor(20 x (1 + inc / 100))`, `inc` the largest
//!   of the Silver flasks' (without the magic part for a flask that is not
//!   magic). [`super::buffs`] adds the constant 20 for the flag
//!   (`silver_flask_display_onslaught`, a mapping row); this module adds
//!   the difference, for attacks and casts.
//!
//! **The flask's lines** go through [`FlaskMapping`]: its own effect (a
//! scale, [`LOCAL_EFFECT_STAT`]), its charges and a utility flask's
//! duration (read into the item's flask data alone, which no calc output
//! reads: `NoEffect`), and every other local stat unmapped, so a reason,
//! until a row maps it. Not modelled: the `UsingFlask` conditions (left
//! unsettled, so a modifier that reads them is a reason), recovery (P6-10b),
//! flasks applied to minions.

use loom_data::bundle::poe1::Flask;
use loom_domain::{ItemId, LineResolution, Rarity, Resolved, StatId, StatLine, StatValue};

use super::{Poe1Quantity, collect, mapping::Poe1Mapping, skill_modifiers};
use crate::condition::Flags;
use crate::ingest::{EffectTemplate, Ingest, StatMap, StatMapping};
use crate::input::CalcInput;
use crate::quantity::{Op, Quantity};
use crate::source::Source;
use crate::state::UnsupportedReason;
use crate::store::{Conditions, Effect};

/// A flask's own increased effect (PoB's local `FlaskEffect`).
pub const LOCAL_EFFECT_STAT: &str = "local_flask_effect_+%";

/// The stat of Silver Flask's buff that grants Onslaught.
pub const SILVER_STAT: &str = "silver_flask_display_onslaught";

/// Onslaught's increased speed (`CalcPerform.lua` 678).
const ONSLAUGHT_SPEED: f64 = 20.0;

/// A flask's charge stats: PoB reads them into the item's flask data
/// (`Classes/Item.lua` 2397 to 2399), which only Iron Flask's unbroken ward
/// condition and a life flask's charge multiplier read (`CalcPerform.lua`
/// 1753; `CalcSetup.lua` 938), each through modifiers: `NoEffect`.
pub const CHARGE_STATS: [&str; 3] = [
    "local_charges_added_+%",
    "local_charges_used_+%",
    "local_extra_max_charges",
];

/// A flask's duration stats: read into the item's flask data, which only a
/// recovery flask's recovery reads (`CalcPerform.lua` 1627). `NoEffect`
/// on a utility flask; unmapped (a reason) on a recovery flask until
/// recovery is computed (P6-10b).
pub const DURATION_STATS: [&str; 2] = ["local_flask_duration_+%", "local_flask_duration_+%_final"];

/// A flask's own lines that reach no slice field (P6-11i), sorted, each as
/// PoB parses its text (`local/p6/parse2`): Bleeding and Corrupted Blood
/// immunity on use, consecrated ground on use, charges when hit, and use
/// on full charges PoB does not parse at all; the effect of curses on you
/// during the effect (`CurseEffectOnSelf` INC, `UsingFlask`) no slice
/// field reads. Movement speed during the effect is an alias since P6-10b.
pub const NO_EFFECT_LOCAL: [&str; 5] = [
    "local_consecrate_ground_on_flask_use_radius",
    "local_flask_bleeding_immunity_if_bleeding_and_remove_corrupted_blood_s",
    "local_flask_gain_X_charges_when_hit",
    "local_flask_use_on_full_charges",
    "local_self_curse_effect_+%_during_flask_effect",
];

/// A flask's own lines PoB reads as the global stat's mod: the line's
/// "during Effect" is PoB's `Condition:UsingFlask` tag (`ModParser.lua`
/// 1737, 1738), on while the flask is active, so the mod is the global
/// one's. Sorted by the local stat (a test).
pub const ALIASES: &[(&str, &str)] = &[
    // "#% increased Armour during Effect".
    (
        "local_flask_armour_+%_while_healing",
        "physical_damage_reduction_rating_+%",
    ),
    // "#% increased Attack Speed during Effect".
    (
        "local_flask_attack_speed_+%_while_healing",
        "attack_speed_+%",
    ),
    // "#% increased Cast Speed during Effect".
    (
        "local_flask_cast_speed_+%_while_healing",
        "base_cast_speed_+%",
    ),
    // "#% increased Critical Strike Chance during Effect".
    (
        "local_flask_critical_strike_chance_+%_during_flask_effect",
        "critical_strike_chance_+%",
    ),
    // "#% increased Evasion Rating during Effect".
    ("local_flask_evasion_+%_while_healing", "evasion_rating_+%"),
    // "#% increased Movement Speed during Effect" (P6-10b).
    (
        "local_flask_movement_speed_+%_while_healing",
        "base_movement_velocity_+%",
    ),
    // "#% additional Elemental Resistances during Effect" (PoB: "additional"
    // is BASE).
    (
        "local_flask_resistances_+%_while_healing",
        "base_resist_all_elements_%",
    ),
    // "+#% to all maximum Elemental Resistances during Effect" (a
    // unique's).
    (
        "local_unique_flask_additional_maximum_all_elemental_resistances_%_while_healing",
        "additional_maximum_all_elemental_resistances_%",
    ),
    // "+#% to Elemental Resistances during Effect" (a unique's).
    (
        "local_unique_flask_resist_all_elements_%_during_flask_effect",
        "base_resist_all_elements_%",
    ),
];

/// The mapping of a flask's own lines (module docs). Not modelled: a
/// pasted base buff line on the item, which PoB drops (`Classes/Item.lua`
/// 515, `flaskBuffLines`); neither PoB's saved items nor the game's copies
/// carry one.
#[derive(Debug, Clone, Copy)]
pub struct FlaskMapping {
    /// Whether the flask recovers life or mana.
    pub recovery: bool,
}

impl StatMapping for FlaskMapping {
    fn map(&self, stat: &StatId) -> StatMap {
        let id = stat.as_str();
        if id == LOCAL_EFFECT_STAT {
            return StatMap::Effects(vec![EffectTemplate {
                quantity: Quantity::Poe1(Poe1Quantity::LocalFlaskEffect),
                op: Op::Increased,
                scale: 1.0,
                divisor: 1.0,
                conditions: Conditions::none(),
            }]);
        }
        if let Ok(i) = ALIASES.binary_search_by(|(local, _)| (*local).cmp(id)) {
            return Poe1Mapping.map(&StatId::new(ALIASES[i].1));
        }
        if CHARGE_STATS.contains(&id)
            || NO_EFFECT_LOCAL.contains(&id)
            || (!self.recovery && DURATION_STATS.contains(&id))
        {
            return StatMap::NoEffect;
        }
        if id.starts_with("local_") || id.starts_with("old_do_not_use_local_") {
            return StatMap::Unmapped;
        }
        Poe1Mapping.map(stat)
    }

    // An item line's unsettled value of a stat another actor or field
    // alone reads (P6-10b: regeneration) is that one's reason.
    fn elsewhere(&self, quantity: Quantity) -> bool {
        Poe1Mapping.elsewhere(quantity)
    }
}

/// The prefix of the consume stats a flask line's description pairs with
/// its own effect, duration, and charges ("#% increased effect" is
/// `local_flask_effect_+%` with `local_flask_consume_flask_effect_+%_when_used`).
pub const CONSUME_PREFIX: &str = "local_flask_consume_";

/// `resolution` without its consume stats left at 0: the description's
/// variant the line matched pins them to 0 (a `[0, 0]` condition, so the
/// resolver's assumed 0 is the value), and a stat at 0 does nothing in
/// game. A consume stat with another value stays, unmapped (a reason).
fn without_consume(resolution: &LineResolution) -> std::borrow::Cow<'_, LineResolution> {
    let LineResolution::Resolved {
        result: Resolved::Stats(stats),
        adapted,
    } = resolution
    else {
        return std::borrow::Cow::Borrowed(resolution);
    };
    let zero = |s: &StatLine| {
        s.stat.as_str().starts_with(CONSUME_PREFIX)
            && matches!(
                s.value,
                StatValue::Hidden { assumed: Some(0) } | StatValue::Exact { min: 0, max: 0 }
            )
    };
    if !stats.iter().any(zero) {
        return std::borrow::Cow::Borrowed(resolution);
    }
    std::borrow::Cow::Owned(LineResolution::Resolved {
        result: Resolved::Stats(stats.iter().filter(|s| !zero(s)).cloned().collect()),
        adapted: *adapted,
    })
}

/// The unconditional `Increased` sum of `quantity` among `effects`, and
/// whether a conditional one was left out.
fn increased(effects: &[Effect], quantity: Quantity) -> (f64, bool) {
    let mut sum = 0.0;
    let mut conditional = false;
    for e in effects
        .iter()
        .filter(|e| e.quantity == quantity && e.op == Op::Increased)
    {
        if e.conditions == Conditions::none() {
            sum += e.value;
        } else {
            conditional = true;
        }
    }
    (sum, conditional)
}

/// The flask of `group` whose scale gives `line`'s mods their largest
/// value: the largest scale when every mod is positive (or zero), the
/// smallest when every one is negative; `None` when they differ in sign
/// and the flasks' scales differ.
fn merge_pick<'a, 'd>(
    line: &StatLine,
    value: i64,
    group: &[&'a Active<'d>],
) -> Option<&'a Active<'d>> {
    let largest = group
        .iter()
        .copied()
        .max_by(|x, y| x.scale.total_cmp(&y.scale))?;
    let smallest = group
        .iter()
        .copied()
        .min_by(|x, y| x.scale.total_cmp(&y.scale))?;
    if largest.scale == smallest.scale {
        return Some(largest);
    }
    #[expect(clippy::cast_precision_loss, reason = "a small stat value")]
    let v = value as f64;
    let signs: Vec<bool> = match Poe1Mapping.map(&line.stat) {
        StatMap::Effects(t) => t
            .iter()
            .filter(|e| e.op != Op::Flag)
            .map(|e| v * e.scale / e.divisor >= 0.0)
            .collect(),
        StatMap::NoEffect | StatMap::Unmapped => Vec::new(),
    };
    if signs.iter().all(|s| *s) {
        Some(largest)
    } else if signs.iter().all(|s| !*s) {
        Some(smallest)
    } else {
        None
    }
}

/// One active flask, after its lines are in.
struct Active<'d> {
    item: ItemId,
    slot: String,
    base: &'d Flask,
    scale: f64,
}

/// Adds the active flasks' buffs and lines (module docs). Runs after every
/// source of flask effect is in (passives, items, config).
pub fn flasks(input: &CalcInput<'_>, ingest: &mut Ingest<'_>) {
    let build = input.build;
    let Some(set) = collect::active_item_set(build) else {
        return;
    };
    let second = set
        .attributes
        .iter()
        .any(|a| a.name == "useSecondWeaponSet" && a.value == "true");
    let mut slots = Vec::new();
    for entry in set.slots.typed() {
        let slot = entry.slot.as_str();
        let Some(id) = entry.item else { continue };
        if id.0 == 0
            || !collect::slot_in_use(slot, second)
            || !collect::is_flask_slot(slot)
            || !entry
                .attributes
                .iter()
                .any(|a| a.name == "active" && a.value == "true")
        {
            continue;
        }
        slots.push((id, slot.to_owned()));
    }
    if slots.is_empty() {
        return;
    }
    // Data without the flask table (a bundle from before P6-4b): each
    // active flask stays the reason it was (P6-4b review), so such a build
    // keeps its other values.
    let Some(bases) = input.data.poe1().and_then(super::Poe1Data::flasks) else {
        for (id, slot) in slots {
            ingest.reason(UnsupportedReason::UnmodelledItem {
                item: id,
                slot,
                what: "active-flask",
            });
        }
        return;
    };
    let (global, conditional) =
        increased(ingest.effects(), Quantity::Poe1(Poe1Quantity::FlaskEffect));
    let (magic, magic_conditional) = increased(
        ingest.effects(),
        Quantity::Poe1(Poe1Quantity::MagicUtilityFlaskEffect),
    );
    let mut active: Vec<Active<'_>> = Vec::new();
    let mut keys: Vec<Vec<String>> = Vec::new();
    for (id, slot) in slots {
        let unmodelled = |what| UnsupportedReason::UnmodelledItem {
            item: id,
            slot: slot.clone(),
            what,
        };
        let Some(item) = build.items.item(id) else {
            continue;
        };
        // The base name the import adapter found (`CalcOptions::item_bases`,
        // as an armour piece's), else the one the item writes.
        let named = |name: &str| bases.iter().find(|f| f.name == name);
        let found = match input.options.item_bases.get(&id) {
            Some(b) => named(&b.name).or_else(|| b.fallback.as_deref().and_then(named)),
            None => item.body.base.as_deref().and_then(named),
        };
        let Some(base) = found else {
            ingest.reason(unmodelled("flask-base"));
            continue;
        };
        if conditional {
            ingest.reason(unmodelled("flask-effect-condition"));
        }
        let recovery = base.life > 0 || base.mana > 0;
        let is_magic = item.body.rarity == Some(Rarity::Magic) && !recovery;
        if is_magic && magic_conditional {
            ingest.reason(unmodelled("flask-effect-condition"));
        }
        // The lines, then their key for the merge.
        let start = ingest.effects().len();
        let mapping = FlaskMapping { recovery };
        let mut key = Vec::new();
        for (line, m) in item.body.modifiers() {
            ingest.item_line_with(&mapping, id, line, &without_consume(&m.resolution));
            key.push(m.text.clone());
        }
        if matches!(item.body.rarity, Some(Rarity::Unique | Rarity::Relic)) {
            key = vec![item.body.name.clone().unwrap_or_default()];
        } else {
            key.sort();
        }
        if !key.is_empty() && !key.iter().all(String::is_empty) {
            if keys.contains(&key) {
                ingest.reason(unmodelled("flask-merge"));
            }
            keys.push(key);
        }
        let (local, _) = increased(
            &ingest.effects()[start..],
            Quantity::Poe1(Poe1Quantity::LocalFlaskEffect),
        );
        let inc = global + local + if is_magic { magic } else { 0.0 };
        let scale = 1.0 + inc / 100.0;
        ingest.rescale_from(start, &|q, op, v| skill_modifiers::scale(q, op, v, scale));
        active.push(Active {
            item: id,
            slot: slot.clone(),
            base,
            scale,
        });
    }
    // Each stat of the base's buff once per base, at the scale that gives
    // its mods the largest value (`mergeBuff`, module docs).
    let mut done: Vec<&str> = Vec::new();
    for a in &active {
        if done.contains(&a.base.name.as_str()) {
            continue;
        }
        done.push(&a.base.name);
        let group: Vec<&Active<'_>> = active
            .iter()
            .filter(|b| b.base.name == a.base.name)
            .collect();
        for (stat, value) in &a.base.stats {
            let line = StatLine {
                stat: StatId::new(stat.as_str()),
                value: StatValue::Exact {
                    min: *value,
                    max: *value,
                },
            };
            let Some(pick) = merge_pick(&line, *value, &group) else {
                ingest.reason(UnsupportedReason::UnmodelledItem {
                    item: a.item,
                    slot: a.slot.clone(),
                    what: "flask-merge",
                });
                continue;
            };
            let scale = pick.scale;
            ingest.stats_scaled(
                &Source::ItemBase { item: pick.item },
                std::slice::from_ref(&line),
                &|q, op, v| skill_modifiers::scale(q, op, v, scale),
            );
        }
    }
    // Silver Flask's Onslaught (module docs): the difference from the
    // constant [`super::buffs`] adds. PoB floors one sum, `20 x (1 + flask
    // inc + Onslaught effect inc)`, and gives it to movement speed too; the
    // split equals it while the Onslaught effect stats stay unmapped
    // (`buffs::DEPENDS_ON`, a test). Mapping them must compute the extra as
    // one floor here.
    let silver = active
        .iter()
        .filter(|a| a.base.stats.iter().any(|(s, _)| s == SILVER_STAT))
        .max_by(|x, y| x.scale.total_cmp(&y.scale));
    if let Some(s) = silver {
        let extra = crate::math::floor(ONSLAUGHT_SPEED * s.scale) - ONSLAUGHT_SPEED;
        if extra != 0.0 {
            for flags in [Flags::ATTACK, Flags::CAST] {
                ingest.effect(Effect {
                    quantity: Quantity::Speed,
                    op: Op::Increased,
                    value: extra,
                    conditions: Conditions::none().with_all(flags),
                    source: Source::ItemBase { item: s.item },
                });
            }
            // Its movement speed too (P6-10b).
            ingest.effect(Effect {
                quantity: Quantity::Poe1(Poe1Quantity::MovementSpeed),
                op: Op::Increased,
                value: extra,
                conditions: Conditions::none(),
                source: Source::ItemBase { item: s.item },
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_flask_mapping_should_read_effect_charges_and_duration() {
        let utility = FlaskMapping { recovery: false };
        let life = FlaskMapping { recovery: true };
        let map = |m: &FlaskMapping, s: &str| m.map(&StatId::new(s));
        assert!(matches!(
            map(&utility, LOCAL_EFFECT_STAT),
            StatMap::Effects(_)
        ));
        for s in CHARGE_STATS {
            assert_eq!(map(&utility, s), StatMap::NoEffect);
            assert_eq!(map(&life, s), StatMap::NoEffect);
        }
        for s in DURATION_STATS {
            assert_eq!(map(&utility, s), StatMap::NoEffect);
            assert_eq!(map(&life, s), StatMap::Unmapped);
        }
        // A local line no row reads stays a reason (recovery: P6-10b).
        assert_eq!(
            map(&life, "local_flask_amount_to_recover_+%"),
            StatMap::Unmapped
        );
        // Each alias reads as its global stat, and that stat is mapped.
        assert!(ALIASES.windows(2).all(|w| w[0].0 < w[1].0));
        for (local, global) in ALIASES {
            let theirs = Poe1Mapping.map(&StatId::new(*global));
            assert!(matches!(theirs, StatMap::Effects(_)), "{global}");
            assert_eq!(map(&utility, local), theirs, "{local}");
        }
    }

    /// P6-11i: the no-effect local lines are sorted and read as no effect.
    #[test]
    fn no_effect_local_lines_should_have_no_effect() {
        assert!(NO_EFFECT_LOCAL.windows(2).all(|w| w[0] < w[1]));
        for id in NO_EFFECT_LOCAL {
            for recovery in [false, true] {
                assert_eq!(
                    FlaskMapping { recovery }.map(&StatId::new(id)),
                    StatMap::NoEffect,
                    "{id}"
                );
            }
        }
    }
}
