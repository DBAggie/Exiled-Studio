//! Active flasks (Phase 6, P6-4b): each active flask's buff and its own
//! lines, scaled by flask effect, as PoB v2.67.2 merges them
//! (`Modules/CalcPerform.lua` 1602 to 1790).
//!
//! What PoB does:
//! - **Which flasks.** A flask slot marked active in the item set
//!   (`CalcSetup.lua` 929), and Mageblood's magic utility flasks
//!   (`CalcPerform.lua` 1543 to 1564; the agent list): the leftmost and
//!   rightmost counts of the items' lines, over the equipped magic flasks
//!   whose base is not a life, mana, or hybrid flask, in slot order
//!   ([`active_flasks`]). An unsettled count is a reason
//!   (`magic-utility-flask-count`), and so is a flask the build marks active
//!   that "Magic Utility Flasks cannot be Used" stops in the game
//!   ([`CANNOT_BE_USED_STAT`]).
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
//! until a row maps it. Not modelled: the `UsingFlask` conditions but
//! `UsingLifeFlask` (left unsettled, so a modifier that reads them is a
//! reason), recovery (P6-10b), flasks applied to minions.
//!
//! **Using a life flask** (P6-11cq; `CalcPerform.lua` 1729 to 1748, 1783):
//! PoB's `UsingLifeFlask` holds when an active flask's base recovers life,
//! its recovery is not all instant, and the character can recover life
//! outside leech. [`using_life_flask`] settles it from the bases alone: the
//! lines that make recovery instant (the flask's own, and PoB's
//! `LifeFlaskInstantRecovery`), Vaal Pact's `CannotRecoverLifeOutsideLeech`,
//! and "Flasks do not apply to you" are unmapped stats, each a reason on
//! every field wherever it appears.

use loom_data::bundle::poe1::Flask;
use loom_domain::{ItemId, LineResolution, Rarity, Resolved, StatId, StatLine, StatValue};

use crate::input::LineValues;

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
/// recovery flask's recovery reads (`CalcPerform.lua` 1627, the
/// `LifeRecovery` and `ManaRecovery` rates, which no slice field reads;
/// P6-11gw): `NoEffect`.
pub const DURATION_STATS: [&str; 2] = ["local_flask_duration_+%", "local_flask_duration_+%_final"];

/// A flask's own lines that reach no slice field (P6-11i), sorted, each as
/// PoB parses its text (`local/p6/parse2`): Bleeding and Corrupted Blood
/// immunity on use, consecrated ground on use, charges when hit, and use
/// on full charges PoB does not parse at all; the effect of curses on you
/// during the effect (`CurseEffectOnSelf` INC, `UsingFlask`) no slice
/// field reads. Movement speed during the effect is an alias since P6-10b.
pub const NO_EFFECT_LOCAL: [&str; 16] = [
    // P6-11dz: avoid shock during the effect (`AvoidShock` BASE,
    // `UsingFlask`): no slice field.
    "local_avoid_shock_%_during_flask_effect",
    "local_consecrate_ground_on_flask_use_radius",
    // P6-11dp: area of effect during the effect (`AreaOfEffect` INC,
    // `UsingFlask`): area, no slice field.
    "local_flask_area_of_effect_+%_during_flask_effect",
    // P6-11gp: avoid stun during the effect (`AvoidStun` BASE,
    // `UsingFlask`): no slice field.
    "local_flask_avoid_stun_chance_%_during_flask_effect",
    "local_flask_bleeding_immunity_if_bleeding_and_remove_corrupted_blood_s",
    "local_flask_gain_X_charges_when_hit",
    // Mageblood (the agent list): immunity to bleeding during the effect
    // (`BleedImmune` FLAG, `UsingFlask`): no slice field, as the helmet's
    // immunity (`mapping::NO_EFFECT`).
    "local_flask_immune_to_bleeding_and_corrupted_blood_during_flask_effect",
    // Immunity to shock during the effect (`ShockImmune` FLAG,
    // `UsingFlask`): PoB reads it only for a shock on you (`CalcPerform.lua`
    // 853), which only the `conditionShocked` and `conditionOnShockedGround`
    // checks give, each a reason, as for the shock effect below.
    "local_flask_immune_to_shock_during_flask_effect",
    // P6-11gp: life leeched from damage taken during the effect
    // (`DamageTakenWhenHitLifeLeech` BASE, `UsingFlask`): no module of PoB
    // reads the mod.
    "local_flask_life_leech_on_damage_taken_%_permyriad_during_flask_effect",
    // P6-11dr: energy shield recovered on kill during the effect
    // (`EnergyShieldOnKill` BASE, `UsingFlask`): no slice field.
    "local_flask_recover_%_maximum_energy_shield_on_kill_during_flask_effect",
    // Close-out M1: mana recovered on kill during the effect (`ManaOnKill`
    // BASE, `UsingFlask`): no slice field.
    "local_flask_recover_%_maximum_mana_on_kill_during_flask_effect",
    // P6-11dz: "Removes Curses on use": no slice field.
    "local_flask_remove_curses_on_use",
    "local_flask_use_on_full_charges",
    "local_self_curse_effect_+%_during_flask_effect",
    // P6-11gp: the effect of shock on you during the effect
    // (`SelfShockEffect` INC, `UsingFlask`): PoB reads it only for a shock
    // on you, which only its `conditionShocked` and `conditionOnShockedGround`
    // checks give (`CalcPerform.lua` 849), each a reason (no engine module
    // reads them).
    "local_self_shock_effect_+%_during_flask_effect",
    // P6-11ea: smoke ground on use: no slice field.
    "local_smoke_ground_on_flask_use_radius",
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
    // "#% chance to Freeze, Shock and Ignite during Effect" (the agent list;
    // `EnemyFreezeChance`, `EnemyShockChance`, `EnemyIgniteChance` BASE,
    // `UsingFlask`).
    (
        "local_flask_chance_to_freeze_shock_ignite_%_while_healing",
        "chance_to_freeze_shock_ignite_%",
    ),
    // "#% increased Critical Strike Chance during Effect".
    (
        "local_flask_critical_strike_chance_+%_during_flask_effect",
        "critical_strike_chance_+%",
    ),
    // "#% increased Evasion Rating during Effect".
    ("local_flask_evasion_+%_while_healing", "evasion_rating_+%"),
    // "Regenerate #% of Life per second during Effect" (P6-11gp;
    // `LifeRegenPercent` BASE, `UsingFlask`).
    (
        "local_flask_life_regeneration_per_minute_%_during_flask_effect",
        "life_regeneration_rate_per_minute_%",
    ),
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
    // "+#% Chance to Block Attack Damage during Effect" (close-out M1;
    // `BlockChance` BASE, `UsingFlask`).
    (
        "local_unique_flask_block_%_while_healing",
        "additional_block_%",
    ),
    // "#% increased Damage Over Time during Effect" (a unique's; the old
    // "during Flask Effect" is a reviewed rewording; PoB: `Damage` INC with
    // `ModFlag.Dot`, `UsingFlask`).
    (
        "local_unique_flask_damage_over_time_+%_during_flask_effect",
        "damage_over_time_+%",
    ),
    // "Gain #% of Elemental Damage as Extra Chaos Damage during effect"
    // (close-out M1).
    (
        "local_unique_flask_elemental_damage_%_to_add_as_chaos_while_healing",
        "elemental_damage_%_to_add_as_chaos",
    ),
    // "#% of Chaos Damage Leeched as Life during Effect" (a unique's; the
    // old "during Flask effect" is a reviewed rewording; PoB:
    // `ChaosDamageLifeLeech` BASE, `UsingFlask`).
    (
        "local_unique_flask_life_leech_from_chaos_damage_permyriad_while_healing",
        "base_life_leech_from_chaos_damage_permyriad",
    ),
    // "Gain #% of Physical Damage as Extra Chaos Damage during effect"
    // (close-out M1).
    (
        "local_unique_flask_physical_damage_%_to_add_as_chaos_while_healing",
        "physical_damage_%_to_add_as_chaos",
    ),
    // "+#% to Elemental Resistances during Effect" (a unique's).
    (
        "local_unique_flask_resist_all_elements_%_during_flask_effect",
        "base_resist_all_elements_%",
    ),
    // "+#% Chance to Block Spell Damage during Effect" (close-out M1;
    // `SpellBlockChance` BASE, `UsingFlask`).
    (
        "local_unique_flask_spell_block_%_while_healing",
        "additional_spell_block_%",
    ),
];

/// Flask lines with a row of their own in the character's mapping
/// (P6-11bz), sorted.
pub const FLASK_ROWS: &[&str] = &[
    // P6-11ca.
    "local_flask_critical_strike_chance_against_enemies_on_consecrated_ground_%_during_effect",
    "local_flask_during_effect_enemy_damage_taken_+%_on_consecrated_ground",
    "local_flask_enemies_ignited_during_flask_effect_damage_taken_+%",
    // P6-11gp: "#% increased Mana Cost of Skills during Effect".
    "local_flask_skill_mana_cost_+%_during_flask_effect",
    // P6-11ce.
    "local_life_loss_%_to_prevent_during_flask_effect_to_lose_over_time",
    // Close-out M1: "#% increased Critical Strike Chance against Enemies on
    // Consecrated Ground during Effect" and Lion's Roar's "#% more Melee
    // Physical Damage during effect".
    "local_unique_flask_critical_strike_chance_+%_vs_enemies_on_consecrated_ground_during_flask_effect",
    "local_unique_lions_roar_melee_physical_damage_+%_final_during_flask_effect",
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
            || DURATION_STATS.contains(&id)
            || id == INSTANT_STAT
            || id == LOW_LIFE_INSTANT_STAT
            || id == PARTIAL_INSTANT_STAT
        {
            return StatMap::NoEffect;
        }
        // A flask line with a row of its own (P6-11bz): the enemy's damage
        // taken while ignited or on consecrated ground.
        if FLASK_ROWS.contains(&id) {
            return Poe1Mapping.map(stat);
        }
        if id.starts_with("local_") || id.starts_with("old_do_not_use_local_") {
            // A flask line the game's mapping reads as `NoEffect` (P6-11m:
            // the amount and rate recovered, charges on crit, taunt, the
            // ignited enemies' lines) is `NoEffect` here too; any other
            // local line is a reason.
            return match Poe1Mapping.map(stat) {
                StatMap::NoEffect => StatMap::NoEffect,
                _ => StatMap::Unmapped,
            };
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

/// Mageblood's "Leftmost # Magic Utility Flasks constantly apply their
/// Flask Effects to you" (the agent list; PoB's
/// `LeftActiveMagicUtilityFlasks` BASE, `CalcPerform.lua` 1543 to 1564).
pub const ALWAYS_APPLY_STAT: &str = "num_magic_utility_flasks_always_apply";

/// Its rightmost form (PoB's `RightActiveMagicUtilityFlasks` BASE).
pub const ALWAYS_APPLY_RIGHTMOST_STAT: &str = "num_magic_utility_flasks_always_apply_rightmost";

/// Mageblood's "Magic Utility Flasks cannot be Used". PoB parses no mod
/// for it, so a magic utility flask the build marks active applies there
/// even when Mageblood does not apply it; in the game it cannot be used.
/// The engine applies it as PoB does and marks it a reason
/// (`magic-utility-flask-cannot-be-used`).
pub const CANNOT_BE_USED_STAT: &str = "magic_utility_flasks_cannot_be_used";

/// "Can't use Flask in Fifth Slot" (the agent list, 2026-11-28; PoB's
/// `CanNotUseItem` with `DisablesItem` `Flask 5`, `CalcSetup.lua` 845 to
/// 877): the flask there is not used, unless the config's "Don't disable
/// items" ([`IGNORE_DISABLERS_INPUT`]) is on. PoB keeps a Tincture there;
/// the engine models no tincture.
pub const FIFTH_SLOT_STAT: &str = "cannot_use_flask_in_fifth_slot";

/// PoB's "Don't disable items" (`ConfigOptions.lua` 248).
pub const IGNORE_DISABLERS_INPUT: &str = "ignoreItemDisablers";

/// The active flasks (module docs, "Which flasks").
struct ActiveFlasks {
    /// Their items and slots, in slot order.
    slots: Vec<(ItemId, String)>,
    /// An item whose Mageblood count is not settled (a ranged roll the
    /// host did not settle), with its slot: then which flasks apply is
    /// not known.
    unsettled: Option<(ItemId, String)>,
    /// Magic utility flasks the build marks active that "cannot be Used"
    /// makes unusable in the game (PoB applies them; [`CANNOT_BE_USED_STAT`]).
    cannot_use: Vec<(ItemId, String)>,
}

/// The total of `stat` on the selected lines of the items in the active
/// item set's slots, with the host's rolled values; `Err` with the item
/// and slot of a value that is not settled.
fn slot_item_stat(input: &CalcInput<'_>, stat: &str) -> Result<i64, (ItemId, String)> {
    let mut total = 0;
    for (id, place) in collect::items_in_use(input.build) {
        let collect::ItemPlace::Slot(slot) = place else {
            continue;
        };
        let Some(item) = input.build.items.item(id) else {
            continue;
        };
        for (line, m) in item.body.modifiers() {
            if !m.selected {
                continue;
            }
            let LineResolution::Resolved {
                result: Resolved::Stats(stats),
                ..
            } = &m.resolution
            else {
                continue;
            };
            let rolled = input.options.item_lines.get(&id).and_then(|l| l.get(&line));
            for (j, s) in stats.iter().enumerate() {
                if s.stat.as_str() != stat {
                    continue;
                }
                let value = match (&s.value, rolled) {
                    (StatValue::Exact { min, max }, _) if min == max => Some(*min),
                    (StatValue::Ranged { min, max }, Some(LineValues::Rolled(values))) => values
                        .get(j)
                        .copied()
                        .flatten()
                        .filter(|v| (*min..=*max).contains(v)),
                    _ => None,
                };
                match value {
                    Some(v) => total += v,
                    None => return Err((id, slot.clone())),
                }
            }
        }
    }
    Ok(total)
}

/// Whether the build has `stat` on a selected line of an item in a slot.
fn has_slot_item_stat(input: &CalcInput<'_>, stat: &str) -> bool {
    collect::items_in_use(input.build)
        .into_iter()
        .any(|(id, place)| {
            matches!(place, collect::ItemPlace::Slot(_))
                && input.build.items.item(id).is_some_and(|item| {
                    item.body.modifiers().any(|(_, m)| {
                        m.selected
                            && matches!(&m.resolution, LineResolution::Resolved {
                            result: Resolved::Stats(stats),
                            ..
                        } if stats.iter().any(|s| s.stat.as_str() == stat))
                    })
                })
        })
}

/// Whether item `id` is a magic utility flask as PoB tells it
/// (`CalcPerform.lua` 1550, 1551): magic, its base not a life, mana, or
/// hybrid flask.
fn is_magic_utility(input: &CalcInput<'_>, id: ItemId) -> bool {
    let Some(item) = input.build.items.item(id) else {
        return false;
    };
    let base = input
        .options
        .item_bases
        .get(&id)
        .map(|b| b.name.as_str())
        .or(item.body.base.as_deref())
        .unwrap_or_default();
    item.body.rarity == Some(Rarity::Magic)
        && !["Life Flask", "Mana Flask", "Hybrid Flask"]
            .iter()
            .any(|kind| base.contains(kind))
}

/// The active flasks (module docs, "Which flasks"): the flask slots the
/// build marks active, and Mageblood's magic utility flasks, the leftmost
/// and rightmost counted in slot order (PoB's `orderedSlots`, `Flask 1` to
/// `Flask 5`).
fn active_flasks(input: &CalcInput<'_>) -> ActiveFlasks {
    let mut out = ActiveFlasks {
        slots: Vec::new(),
        unsettled: None,
        cannot_use: Vec::new(),
    };
    let Some(set) = collect::active_item_set(input.build) else {
        return out;
    };
    let second = set
        .attributes
        .iter()
        .any(|a| a.name == "useSecondWeaponSet" && a.value == "true");
    // Every equipped flask, in slot order, with whether the build marks it
    // active.
    let mut flasks: Vec<(ItemId, String, bool)> = Vec::new();
    for entry in set.slots.typed() {
        let slot = entry.slot.as_str();
        let Some(id) = entry.item else { continue };
        if id.0 == 0 || !collect::slot_in_use(slot, second) || !collect::is_flask_slot(slot) {
            continue;
        }
        let active = entry
            .attributes
            .iter()
            .any(|a| a.name == "active" && a.value == "true");
        flasks.push((id, slot.to_owned(), active));
    }
    flasks.sort_by(|a, b| a.1.cmp(&b.1));
    let (left, right) = match (
        slot_item_stat(input, ALWAYS_APPLY_STAT),
        slot_item_stat(input, ALWAYS_APPLY_RIGHTMOST_STAT),
    ) {
        (Ok(l), Ok(r)) => (l, r),
        (Err(at), _) | (_, Err(at)) => {
            out.unsettled = Some(at);
            (0, 0)
        }
    };
    let magic: Vec<usize> = (0..flasks.len())
        .filter(|&i| is_magic_utility(input, flasks[i].0))
        .collect();
    let left = usize::try_from(left).unwrap_or(0).min(magic.len());
    let right = usize::try_from(right).unwrap_or(0).min(magic.len());
    let constant: Vec<usize> = magic[..left]
        .iter()
        .chain(&magic[magic.len() - right..])
        .copied()
        .collect();
    let cannot_be_used = has_slot_item_stat(input, CANNOT_BE_USED_STAT);
    let ignore = collect::active_config(input.build).is_some_and(|set| {
        matches!(
            super::config::last_input(
                set,
                IGNORE_DISABLERS_INPUT,
                loom_domain::ConfigInputKind::Input
            ),
            Some(loom_domain::ConfigValue::Boolean(true))
        )
    });
    let fifth_disabled = !ignore && has_slot_item_stat(input, FIFTH_SLOT_STAT);
    for (i, (id, slot, active)) in flasks.into_iter().enumerate() {
        if fifth_disabled && slot == "Flask 5" {
            continue;
        }
        let applies = constant.contains(&i);
        if active && !applies && cannot_be_used && magic.contains(&i) {
            out.cannot_use.push((id, slot.clone()));
        }
        if active || applies {
            out.slots.push((id, slot));
        }
    }
    out
}

/// The flask base of item `id`: the base name the import adapter found
/// (`CalcOptions::item_bases`, as an armour piece's), else the one the item
/// writes.
fn base_of<'d>(
    input: &CalcInput<'_>,
    bases: &'d [Flask],
    id: ItemId,
    written: Option<&str>,
) -> Option<&'d Flask> {
    let named = |name: &str| bases.iter().find(|f| f.name == name);
    match input.options.item_bases.get(&id) {
        Some(b) => named(&b.name).or_else(|| b.fallback.as_deref().and_then(named)),
        None => written.and_then(named),
    }
}

/// "Instant Recovery" (P6-11gw; PoB's `FlaskInstantRecovery` BASE 100, the
/// flask's `instantPerc`): a flask with it recovers all at once, so it
/// sets no `Using` condition (`CalcPerform.lua` 1733 to 1748). The line
/// itself reaches no slice field (its recovery is the `LifeRecovery` rate).
pub const INSTANT_STAT: &str = "local_flask_recovers_instantly";

/// "Instant Recovery when on Low Life" (the agent list; PoB's
/// `FlaskLowLifeInstantRecovery` BASE 100, the flask's
/// `instantLowLifePerc`, added to its instant share while
/// `Condition:LowLife`, `CalcPerform.lua` 1603 to 1607): with low life
/// settled true the flask recovers all at once, so sets no `Using`
/// condition; unsettled, those conditions stay unsettled. The line itself
/// reaches no slice field.
pub const LOW_LIFE_INSTANT_STAT: &str = "local_flask_recover_instantly_when_on_low_life";

/// "#% of Recovery applied Instantly" (the agent list, 2026-11-28; PoB's
/// `FlaskInstantRecovery` BASE, added to the flask's `instantPerc`): the
/// flask recovers all at once when its instant share reaches 100
/// (`CalcPerform.lua` 1603 to 1614). The line itself reaches no slice
/// field.
pub const PARTIAL_INSTANT_STAT: &str = "local_flask_recovery_amount_%_to_recover_instantly";

/// Whether the item `id` recovers all at once: its instant share, 100 for
/// [`INSTANT_STAT`] and the value of [`PARTIAL_INSTANT_STAT`] on its
/// selected lines, plus 100 for [`LOW_LIFE_INSTANT_STAT`] with `low_life`
/// (the settled low life), at least 100. `None` when that depends on an
/// unsettled low life or on a value whose range straddles 100.
fn all_instant(input: &CalcInput<'_>, id: ItemId, low_life: Option<bool>) -> Option<bool> {
    let Some(item) = input.build.items.item(id) else {
        return Some(false);
    };
    let (mut low, mut high, mut on_low_life) = (0_i64, 0_i64, false);
    for l in &item.body.lines {
        let loom_domain::LineKind::Modifier(m) = &l.kind else {
            continue;
        };
        if !m.selected {
            continue;
        }
        let LineResolution::Resolved {
            result: Resolved::Stats(stats),
            ..
        } = &m.resolution
        else {
            continue;
        };
        for st in stats {
            match st.stat.as_str() {
                INSTANT_STAT => {
                    low += 100;
                    high += 100;
                }
                PARTIAL_INSTANT_STAT => {
                    let (a, b) = match &st.value {
                        StatValue::Exact { min, max } | StatValue::Ranged { min, max, .. } => {
                            (*min, *max)
                        }
                        // A value not shown: neither bound known.
                        StatValue::Hidden { .. } | StatValue::Lookup(_) => (0, 100),
                    };
                    low += a;
                    high += b;
                }
                LOW_LIFE_INSTANT_STAT => on_low_life = true,
                _ => {}
            }
        }
    }
    if on_low_life {
        match low_life {
            Some(true) => return Some(true),
            Some(false) => {}
            None if low >= 100 => return Some(true),
            None => return None,
        }
    }
    if low >= 100 {
        Some(true)
    } else if high < 100 {
        Some(false)
    } else {
        None
    }
}

/// The active flasks that do not recover all at once (P6-11gw): the ones
/// that set PoB's `Using` conditions; `None` when one depends on an
/// unsettled low life.
fn using_slots(input: &CalcInput<'_>, low_life: Option<bool>) -> Option<Vec<(ItemId, String)>> {
    let active = active_flasks(input);
    // Which flasks Mageblood applies is not known.
    if active.unsettled.is_some() {
        return None;
    }
    let mut out = Vec::new();
    for (id, slot) in active.slots {
        if !all_instant(input, id, low_life)? {
            out.push((id, slot));
        }
    }
    Some(out)
}

/// Whether the character is using a flask (PoB's `UsingFlask`; P6-11dz,
/// `CalcPerform.lua` 1735): an active flask whose recovery is not all
/// instant ([`INSTANT_STAT`], P6-11gw; [`LOW_LIFE_INSTANT_STAT`] by
/// `low_life`, the settled low life). `None` when that depends on an
/// unsettled low life. PoB's global `LifeFlaskInstantRecovery` lines are
/// unmapped stats, a reason wherever they appear.
#[must_use]
pub fn using_flask(input: &CalcInput<'_>, low_life: Option<bool>) -> Option<bool> {
    Some(!using_slots(input, low_life)?.is_empty())
}

/// Whether the character is using a life flask (PoB's `UsingLifeFlask`;
/// P6-11cq, module docs): `Some(false)` with no active flask, `Some(true)`
/// when an active flask's base recovers life, `None` when a flask's base
/// is not known or its instant recovery depends on an unsettled low life
/// (`low_life`).
#[must_use]
pub fn using_life_flask(input: &CalcInput<'_>, low_life: Option<bool>) -> Option<bool> {
    let slots = using_slots(input, low_life)?;
    if slots.is_empty() {
        return Some(false);
    }
    let bases = input.data.poe1().and_then(super::Poe1Data::flasks)?;
    let mut life = false;
    for (id, _) in slots {
        let item = input.build.items.item(id)?;
        let base = base_of(input, bases, id, item.body.base.as_deref())?;
        life |= base.life > 0;
    }
    Some(life)
}

/// Adds the active flasks' buffs and lines (module docs). Runs after every
/// source of flask effect is in (passives, items, config).
pub fn flasks(input: &CalcInput<'_>, ingest: &mut Ingest<'_>) {
    let build = input.build;
    let ActiveFlasks {
        slots,
        unsettled,
        cannot_use,
    } = active_flasks(input);
    if let Some((item, slot)) = unsettled {
        ingest.reason(UnsupportedReason::UnmodelledItem {
            item,
            slot,
            what: "magic-utility-flask-count",
        });
    }
    for (item, slot) in cannot_use {
        ingest.reason(UnsupportedReason::UnmodelledItem {
            item,
            slot,
            what: "magic-utility-flask-cannot-be-used",
        });
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
        let Some(base) = base_of(input, bases, id, item.body.base.as_deref()) else {
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
        // P6-11gw: a recovery flask's duration and instant recovery reach
        // no slice field either.
        for s in DURATION_STATS.into_iter().chain([INSTANT_STAT]) {
            assert_eq!(map(&utility, s), StatMap::NoEffect);
            assert_eq!(map(&life, s), StatMap::NoEffect);
        }
        // A local line no row reads stays a reason; one the game's
        // mapping reads as `NoEffect` (P6-11m) is `NoEffect` (P6-11n).
        assert_eq!(
            map(
                &life,
                "local_unique_flask_nearby_enemies_cursed_with_level_x_vulnerability_during_flask_effect"
            ),
            StatMap::Unmapped
        );
        // Its own rows (P6-11bz).
        assert!(FLASK_ROWS.windows(2).all(|w| w[0] < w[1]));
        for s in FLASK_ROWS {
            assert!(matches!(map(&life, s), StatMap::Effects(_)), "{s}");
        }
        assert_eq!(
            map(&life, "local_flask_amount_to_recover_+%"),
            StatMap::NoEffect
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
