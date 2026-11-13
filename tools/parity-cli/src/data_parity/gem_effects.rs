//! The engine's other-gem tables (`loom_calc::poe1::gems`, P3-12h) against
//! PoB v2.67.2's skill data (the export's format 4 `mods`): table
//! `gem_effects`. Every value compared is in PoB's form; the engine's side
//! is written into the report's `bundle` column (the table is code, not
//! bundle data). Any difference is a defect of the table: no rule covers
//! this table, so `uncategorized` counts it.
//!
//! Per modelled active skill or support (key: its granted effect ID):
//! - `global_stats`: the stats with a `GlobalEffect` mod (`""` for base
//!   mods), sorted, against the table's stats;
//! - `buff`: PoB's `effectType` and buff name (`effectName`, else the
//!   skill's name) of each global mod, against the table's;
//! - `row.<stat>`: each slice row as PoB's mod (name, type, flags read as
//!   attack, spell, cast, hit, projectile, `div` and `mult`, other tags),
//!   against the table's effects in the same form;
//! - `blocked`: PoB's other mods whose name scales buffs or reservation,
//!   sets an ailment's maximum, adds skill mods, or whose `SkillData` key
//!   is not a display key (each a path the module docs exclude): none;
//! - `types`: PoB's types (active skills) or added types (supports) a
//!   modelled skill may not have (reservation for an inert skill, minions,
//!   brands, warcries, triggers, totems, ...): none; `is_trigger`,
//!   `add_minions` (Summon Phantasm's own minion only), and a support
//!   level's `manaReservationPercent`.

use std::collections::{BTreeMap, BTreeSet};

use loom_calc::poe1::gems::{self, BuffStat, Kind};
use loom_calc::{Flags, Op, Quantity};
use serde_json::{Value, json};

use super::compare::Comparison;
use super::pob::{Export, ModRecord, Skill};

/// The table's name.
pub const TABLE: &str = "gem_effects";

/// Mod names no modelled skill may have outside its buff (module docs).
const BLOCKED_NAMES: &[&str] = &[
    "AuraBuffEffect",
    "AuraEffect",
    "AuraEffectOnSelf",
    "BloodMagicReserved",
    "BuffEffect",
    "BuffEffectOnPlayer",
    "BuffEffectOnSelf",
    "ExtraAura",
    "ExtraAuraEffect",
    "ExtraSkillMod",
    "ExtraSupport",
    "HasUncancellableReservation",
    "LifeCostGainAsReservation",
    "LifeReservationEfficiency",
    "LifeReserved",
    "ManaCostGainAsReservation",
    "ManaReservationEfficiency",
    "ManaReserved",
    "ReservationEfficiency",
    "Reserved",
    "SkillAuraEffectOnSelf",
    "SupportManaMultiplier",
];

/// `SkillData` keys that set only the skill's own data (its displays,
/// area, duration, base damage, stun, repeat rate) or what allies and
/// enemies get: `skillData` is per active skill, and PoB's other code
/// reads these keys of the skill being calculated only.
const DISPLAY_KEYS: &[&str] = &[
    "ColdMax",
    "ColdMin",
    "PhysicalMax",
    "PhysicalMin",
    "allowTotemBuff",
    "baseStunDuration",
    "buffAllies",
    "debuff",
    "debuffSecondary",
    "dotIsArea",
    "duration",
    "radius",
    "radiusExtra",
    "radiusLabel",
    "radiusSecondary",
    "radiusSecondaryLabel",
    "radiusTertiary",
    "radiusTertiaryLabel",
    "repeatFrequency",
    "setOffHandPhysicalMax",
    "setOffHandPhysicalMin",
    "showAverage",
    "thisIsNotABuff",
];

/// Types an inert skill may not have; a buff skill may reserve
/// (`HasReservation`) and be a buff, aura, or herald. A totem's or a
/// minion's type is allowed: PoB counts them only into named multipliers
/// and conditions (`TotemsSummoned`, minion counts,
/// `HaveDamageableMinion`: `CalcPerform.lua` 1334 to 1430), which only
/// modifiers read, and no mapping row reads a named condition (its row
/// types hold flags only).
const INERT_FORBIDDEN: &[&str] = &[
    "Aura",
    "Brand",
    "Buff",
    "HasReservation",
    "Herald",
    "Hex",
    "Mark",
    "Triggered",
    "Trigger",
    "Warcry",
];
const BUFF_FORBIDDEN: &[&str] = &[
    "Brand",
    "CreatesMinion",
    "Hex",
    "Mark",
    "Minion",
    "ReservationBecomesCost",
    "SummonsTotem",
    "Totem",
    "Triggered",
    "Trigger",
    "Warcry",
];
/// Types a modelled support may not add.
const SUPPORT_FORBIDDEN: &[&str] = &[
    "Aura",
    "Buff",
    "HasReservation",
    "Herald",
    "ReservationBecomesCost",
    "SummonsTotem",
    "Totem",
    "Trigger",
    "Triggered",
];

/// Types a modelled support may add after all: Momentum makes its skill a
/// buff (its own movement-speed buff, `NoSlice`).
const ALLOWED_ADD_TYPES: &[(&str, &str)] = &[("SupportMomentum", "Buff")];

/// Supports that add a minion to the skill they support, and the minion:
/// its own skills grant the character nothing (Summon Phantasm: the
/// phantasm attacks; PoB's minion code counts it for modifiers that read
/// minion counts, which are reasons).
const ALLOWED_ADD_MINIONS: &[(&str, &str)] = &[("SupportSummonPhantasm", "SummonedPhantasm")];

fn global(m: &ModRecord) -> Option<&BTreeMap<String, Value>> {
    m.global_effect()
}

fn ailment_max(name: &str) -> bool {
    name.ends_with("Max")
        && [
            "Shock", "Chill", "Scorch", "Brittle", "Sap", "Freeze", "Ignite", "Bleed", "Poison",
        ]
        .iter()
        .any(|a| name == format!("{a}Max"))
}

/// A mod in a canonical form both sides share.
fn pob_form(m: &ModRecord, div: Option<f64>, mult: Option<f64>) -> String {
    let flags = m.flags.unwrap_or(0);
    let kw = m.keyword_flags.unwrap_or(0);
    let mut f = Vec::new();
    if flags & 1 != 0 || kw & 0x1_0000 != 0 {
        f.push("attack");
    }
    if flags & 2 != 0 || kw & 0x2_0000 != 0 {
        f.push("spell");
    }
    if flags & 4 != 0 {
        f.push("hit");
    }
    if flags & 16 != 0 {
        f.push("cast");
    }
    if flags & 1024 != 0 {
        f.push("projectile");
    }
    let rest = flags & !(1 | 2 | 4 | 16 | 1024);
    let rest_kw = kw & !(0x1_0000 | 0x2_0000);
    let tags: Vec<String> = m
        .tags
        .iter()
        .filter(|t| t.get("type").and_then(Value::as_str) != Some("GlobalEffect"))
        .map(|t| serde_json::to_string(t).unwrap_or_default())
        .collect();
    format!(
        "{} {} [{}]{}{}{}{}",
        m.name,
        m.kind,
        f.join(","),
        if rest != 0 || rest_kw != 0 {
            format!(" other-flags {rest}/{rest_kw}")
        } else {
            String::new()
        },
        div.map_or(String::new(), |d| format!(" div {d}")),
        mult.map_or(String::new(), |d| format!(" mult {d}")),
        if tags.is_empty() {
            String::new()
        } else {
            format!(" tags {}", tags.join(";"))
        }
    )
}

/// PoB's mod name of one of our quantities, for the rows the tables use.
fn pob_name(q: Quantity) -> String {
    match q {
        Quantity::Accuracy => "Accuracy".into(),
        Quantity::CritChance => "CritChance".into(),
        Quantity::Speed => "Speed".into(),
        Quantity::EnergyShield => "EnergyShield".into(),
        Quantity::Damage => "Damage".into(),
        Quantity::AddedDamage(t, b) => {
            let t = t.name();
            let mut c = t.chars();
            let t = c.next().map_or(String::new(), |f| {
                f.to_uppercase().collect::<String>() + c.as_str()
            });
            format!("{t}{}", if b.name() == "min" { "Min" } else { "Max" })
        }
        other => format!("<{other}>"),
    }
}

fn our_form(effects: &[(Quantity, Op, f64, Flags, Flags)]) -> Vec<String> {
    effects
        .iter()
        .map(|&(q, op, divisor, all, any)| {
            let kind = match op {
                Op::Base => "BASE",
                Op::Increased => "INC",
                Op::More => "MORE",
                Op::Flag => "FLAG",
                Op::Override => "OVERRIDE",
            };
            let mut f = Vec::new();
            if all.contains(Flags::ATTACK) {
                f.push("attack");
            }
            if all.contains(Flags::SPELL) {
                f.push("spell");
            }
            if all.contains(Flags::HIT) {
                f.push("hit");
            }
            if all.contains(Flags::CAST) {
                f.push("cast");
            }
            if all.contains(Flags::PROJECTILE) {
                f.push("projectile");
            }
            let other = !any.is_empty()
                || !(Flags::ATTACK | Flags::SPELL | Flags::HIT | Flags::CAST | Flags::PROJECTILE)
                    .contains(all);
            format!(
                "{} {kind} [{}]{}{}",
                pob_name(q),
                f.join(","),
                if other { " other-flags" } else { "" },
                if divisor == 1.0 {
                    String::new()
                } else {
                    format!(" div {divisor}")
                }
            )
        })
        .collect()
}

fn strings(items: impl IntoIterator<Item = String>) -> Value {
    Value::Array(items.into_iter().map(Value::String).collect())
}

fn check_one(
    out: &mut Comparison,
    id: &str,
    skill: &Skill,
    stats: &[(&str, BuffStat)],
    buff: Option<(&str, &str)>,
    forbidden: &[&str],
    support: bool,
) {
    // Global stats.
    let pob_global: BTreeSet<String> = skill
        .mods
        .iter()
        .filter(|s| s.mods.iter().any(|m| global(m).is_some()))
        .map(|s| s.stat.clone())
        .collect();
    let ours: BTreeSet<String> = stats.iter().map(|(s, _)| (*s).to_owned()).collect();
    out.check(
        TABLE,
        id,
        "global_stats",
        strings(ours),
        strings(pob_global),
    );
    // Buff identity.
    if let Some((name, effect)) = buff {
        let mut pob: BTreeSet<String> = BTreeSet::new();
        for s in &skill.mods {
            for m in &s.mods {
                if let Some(g) = global(m) {
                    let t = g.get("effectType").and_then(Value::as_str).unwrap_or("");
                    let n = g
                        .get("effectName")
                        .and_then(Value::as_str)
                        .or(skill.name.as_deref())
                        .unwrap_or("");
                    pob.insert(format!("{t} {n}"));
                }
            }
        }
        out.check(
            TABLE,
            id,
            "buff",
            strings([format!("{effect} {name}")]),
            strings(pob),
        );
    }
    // Rows.
    for (stat, b) in stats {
        let BuffStat::Rows(rows) = b else { continue };
        let pob: Vec<String> = skill
            .mods
            .iter()
            .filter(|s| s.stat == *stat)
            .flat_map(|s| s.mods.iter().map(move |m| pob_form(m, s.div, s.mult)))
            .collect();
        out.check(
            TABLE,
            id,
            format!("row.{stat}"),
            strings(our_form(rows)),
            strings(pob),
        );
    }
    // Blocked mods outside the buff.
    let mut blocked = Vec::new();
    for s in &skill.mods {
        for m in &s.mods {
            if global(m).is_some() {
                continue;
            }
            let key = m
                .value_fields
                .as_ref()
                .and_then(|v| v.get("key"))
                .and_then(Value::as_str);
            let bad = BLOCKED_NAMES.contains(&m.name.as_str())
                || ailment_max(&m.name)
                || (m.name == "SkillData" && !key.is_some_and(|k| DISPLAY_KEYS.contains(&k)));
            if bad {
                blocked.push(format!(
                    "{}:{} {}{}",
                    s.stat,
                    m.name,
                    m.kind,
                    key.map_or(String::new(), |k| format!(" {k}"))
                ));
            }
        }
    }
    out.check(TABLE, id, "blocked", strings(Vec::new()), strings(blocked));
    // Types.
    let types = if support {
        &skill.add_types
    } else {
        &skill.types
    };
    let bad: Vec<String> = types
        .iter()
        .filter(|t| forbidden.contains(&t.as_str()))
        .filter(|t| !ALLOWED_ADD_TYPES.contains(&(id, t.as_str())))
        .cloned()
        .collect();
    out.check(TABLE, id, "types", strings(Vec::new()), strings(bad));
    out.check(
        TABLE,
        id,
        "is_trigger",
        json!(false),
        json!(skill.is_trigger),
    );
    let allowed: Vec<String> = ALLOWED_ADD_MINIONS
        .iter()
        .filter(|(s, _)| *s == id)
        .map(|(_, m)| (*m).to_owned())
        .collect();
    out.check(
        TABLE,
        id,
        "add_minions",
        strings(allowed),
        strings(skill.add_minions.clone()),
    );
    if !support {
        out.check(
            TABLE,
            id,
            "minions",
            strings(Vec::new()),
            strings(skill.minions.clone()),
        );
    } else {
        let percent: Vec<String> = skill
            .levels
            .iter()
            .filter(|(_, l)| {
                l.mana_reservation_percent.is_some() || l.life_reservation_percent.is_some()
            })
            .map(|(k, _)| k.clone())
            .collect();
        out.check(
            TABLE,
            id,
            "level_reservation",
            strings(Vec::new()),
            strings(percent),
        );
    }
}

/// Compares every table entry the bundle has (`effects`: its granted
/// effect IDs; an entry the bundle lacks can act on no build) with PoB's
/// export (module docs).
pub fn compare_gem_effects(out: &mut Comparison, export: &Export, effects: &BTreeSet<&str>) {
    for (id, kind) in gems::SKILLS {
        if !effects.contains(id) {
            continue;
        }
        let Some(skill) = export.skills.get(*id) else {
            out.check(TABLE, id, "row", json!("present"), Value::Null);
            continue;
        };
        match kind {
            Kind::Inert => check_one(out, id, skill, &[], None, INERT_FORBIDDEN, false),
            Kind::Buff {
                name,
                effect,
                stats,
            } => check_one(
                out,
                id,
                skill,
                stats,
                Some((name, effect.pob())),
                BUFF_FORBIDDEN,
                false,
            ),
        }
    }
    for (id, stats) in gems::SUPPORTS {
        if !effects.contains(id) {
            continue;
        }
        let Some(skill) = export.skills.get(*id) else {
            out.check(TABLE, id, "row", json!("present"), Value::Null);
            continue;
        };
        check_one(out, id, skill, stats, None, SUPPORT_FORBIDDEN, true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// PoB's Precision as the export writes it (the parts the check reads).
    fn precision() -> Value {
        json!({
            "name": "Precision",
            "stats": ["accuracy_rating", "skill_buff_grants_critical_strike_chance_+%"],
            "types": ["Aura", "Buff", "HasReservation", "Spell"],
            "constant_stats": {}, "quality_stats": {}, "levels": {},
            "minions": {}, "add_types": {}, "add_minions": {}, "is_trigger": false,
            "mods": [
                {"stat": "accuracy_rating", "own": true, "mods": [
                    {"name": "Accuracy", "type": "BASE", "flags": 0, "keywordFlags": 0,
                     "tags": [{"type": "GlobalEffect", "effectType": "Aura"}]}]},
                {"stat": "skill_buff_grants_critical_strike_chance_+%", "own": true, "mods": [
                    {"name": "CritChance", "type": "INC", "flags": 0, "keywordFlags": 0,
                     "tags": [{"type": "GlobalEffect", "effectType": "Aura"}]}]},
                {"stat": "", "own": true, "mods": [
                    {"name": "SkillData", "type": "LIST", "value_fields": {"key": "radius", "value": 40}, "tags": {}}]}
            ]
        })
    }

    fn export(skill: Value) -> Export {
        let doc = json!({
            "format": super::super::pob::FORMAT,
            "pob": {"version": "2.67.2"},
            "tree_version": "3_29",
            "skill_types": {},
            "skills": {"Precision": skill},
            "gems": {}, "bases": {}, "monster_tables": {}, "game_constants": {},
            "character_constants": {}, "monster_constants": {}, "bosses": {}, "pantheons": {},
            "tree": {"classes": {}, "nodes": {}}
        });
        super::super::pob::parse(&serde_json::to_vec(&doc).unwrap()).unwrap()
    }

    fn run(skill: Value) -> Comparison {
        let mut out = Comparison::default();
        compare_gem_effects(&mut out, &export(skill), &BTreeSet::from(["Precision"]));
        out
    }

    fn fields(out: &Comparison) -> Vec<String> {
        out.differences.iter().map(|d| d.field.clone()).collect()
    }

    #[test]
    fn the_table_should_equal_pobs_precision() {
        let out = run(precision());
        assert_eq!(out.differences, []);
        assert!(out.tables[TABLE].compared > 5);
    }

    #[test]
    fn a_changed_row_a_new_global_stat_or_a_blocked_mod_should_differ() {
        let mut s = precision();
        s["mods"][0]["mods"][0]["type"] = json!("INC");
        assert_eq!(fields(&run(s)), ["row.accuracy_rating"]);

        let mut s = precision();
        s["mods"][0]["mods"][0]["tags"]
            .as_array_mut()
            .unwrap()
            .push(json!({"type": "Condition", "var": "x"}));
        assert_eq!(fields(&run(s)), ["row.accuracy_rating"], "another tag");

        let mut s = precision();
        s["mods"][2]["mods"]
            .as_array_mut()
            .unwrap()
            .push(json!({"name": "AuraEffect", "type": "INC", "tags": {}}));
        assert_eq!(fields(&run(s)), ["blocked"]);

        let mut s = precision();
        s["mods"][2]["mods"][0]["value_fields"]["key"] = json!("manaReservationFlat");
        assert_eq!(
            fields(&run(s)),
            ["blocked"],
            "a SkillData key that is not local"
        );

        let mut s = precision();
        s["mods"][2]["mods"]
            .as_array_mut()
            .unwrap()
            .push(json!({"name": "Life", "type": "BASE", "tags": [{"type": "GlobalEffect", "effectType": "Buff"}]}));
        assert_eq!(
            fields(&run(s)),
            ["global_stats", "buff"],
            "a global base mod"
        );

        let mut s = precision();
        s["types"].as_array_mut().unwrap().push(json!("Warcry"));
        s["minions"] = json!(["X"]);
        assert_eq!(fields(&run(s)), ["types", "minions"]);
    }

    #[test]
    fn an_entry_the_bundle_lacks_should_not_be_compared() {
        let mut out = Comparison::default();
        compare_gem_effects(&mut out, &export(precision()), &BTreeSet::new());
        assert!(out.tables.is_empty());
        let mut out = Comparison::default();
        let doc = export(precision());
        let mut empty = doc;
        empty.skills.clear();
        compare_gem_effects(&mut out, &empty, &BTreeSet::from(["Precision"]));
        assert_eq!(fields(&out), ["row"], "a modelled skill PoB lacks");
    }

    #[test]
    fn our_form_should_name_pobs_mods() {
        assert_eq!(
            pob_name(Quantity::AddedDamage(
                loom_calc::DamageType::Cold,
                loom_calc::Bound::Min
            )),
            "ColdMin"
        );
        assert_eq!(
            our_form(&[(
                Quantity::Speed,
                Op::Increased,
                1.0,
                Flags::CAST,
                Flags::NONE
            )]),
            ["Speed INC [cast]"]
        );
        assert!(ailment_max("ShockMax"));
        assert!(!ailment_max("ColdMax"));
    }
}
