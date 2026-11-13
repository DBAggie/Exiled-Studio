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
//!   sets an ailment's maximum, scales or sets a chill (P3-8m: the
//!   guaranteed chill reads the character's chill effect alone), adds
//!   skill mods, or whose `SkillData` key is not a display key (each a
//!   path the module docs exclude): none;
//! - `types`: PoB's types (active skills) or added types (supports) a
//!   modelled skill may not have (minions, brands, warcries, triggers,
//!   totems, ... per kind; the allowances name each exception and why):
//!   none; `is_trigger`, `add_minions` (Summon Phantasm's own minion
//!   only), and a support level's `manaReservationPercent`.
//!
//! Phase 3 close-out: debuffs, curses, and warcries are kinds of their own
//! (`effectType` `Debuff`, `Curse`, `Warcry`); rows gated by a config check
//! (`effectCond`), by Blood Stance, or by Righteous Fire's `StatThreshold`
//! tags are compared with those tags; a gem's own buff and aura effect
//! stats (`gems::OWN_EFFECT_STATS`) are read by the engine, so their mods
//! are not blocked.

use std::collections::{BTreeMap, BTreeSet};

use loom_calc::poe1::Poe1Quantity;
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
    "ChillBase",
    "ChillMinimum",
    "ChillOverride",
    "EnemyChillEffect",
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
    // The skill's own damage over time (Flame Wall, Righteous Fire, Herald
    // of Ash's burn): no slice field reads a skill's dot (close-out).
    "FireDot",
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
    // Righteous Fire's own burn (close-out).
    "RFManaMultiplier",
    "repeatFrequency",
    "setOffHandPhysicalMax",
    "setOffHandPhysicalMin",
    "showAverage",
    "thisIsNotABuff",
    // Convocation's buff on minions only (close-out; the engine gives the
    // character nothing of it).
    "buffMinions",
    "buffNotPlayer",
    // A skill's own secondary duration and how its dot scales (close-out).
    "dotIsSpell",
    "durationSecondary",
    // Herald of Ash's burn (close-out).
    "hoaMoreBurn",
    "hoaOverkillPercent",
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
/// Types a debuff skill may not have (close-out).
const DEBUFF_FORBIDDEN: &[&str] = &[
    "Aura",
    "Brand",
    "Buff",
    "CreatesMinion",
    "HasReservation",
    "Herald",
    "Hex",
    "Mark",
    "Minion",
    "SummonsTotem",
    "Totem",
    "Triggered",
    "Trigger",
    "Warcry",
];
/// Types a curse skill may not have (close-out): a mark, an aura curse, a
/// curse a totem, trap, or minion casts.
const CURSE_FORBIDDEN: &[&str] = &[
    "Aura",
    "Brand",
    "CreatesMinion",
    "HasReservation",
    "Mark",
    "Minion",
    "SummonsTotem",
    "Totem",
    "Triggered",
    "Trigger",
    "Warcry",
];
/// Types a warcry skill may not have (close-out).
const WARCRY_FORBIDDEN: &[&str] = &[
    "Aura",
    "Brand",
    "CreatesMinion",
    "HasReservation",
    "Hex",
    "Mark",
    "Minion",
    "SummonsTotem",
    "Totem",
    "Triggered",
    "Trigger",
];

/// Types a modelled active skill may have after all, with why
/// (close-out): golems, Herald of Purity, and Convocation create or buff
/// minions, whose own skills give the character nothing but Lightning
/// Golem's Wrath (`gems::wrath`); Shockwave Support's attack is triggered
/// and hits on its own.
const ALLOWED_TYPES: &[(&str, &str)] = &[
    ("Convocation", "Minion"),
    ("HeraldOfPurity", "CreatesMinion"),
    ("HeraldOfPurity", "Minion"),
    ("SummonFlameGolem", "CreatesMinion"),
    ("SummonFlameGolem", "Minion"),
    ("SummonIceGolem", "CreatesMinion"),
    ("SummonIceGolem", "Minion"),
    ("SummonLightningGolem", "CreatesMinion"),
    ("SummonLightningGolem", "Minion"),
    ("SummonStoneGolem", "CreatesMinion"),
    ("SummonStoneGolem", "Minion"),
    ("SummonStoneGolemAltY", "CreatesMinion"),
    ("SummonStoneGolemAltY", "Minion"),
    ("SupportBluntWeaponShockwave", "Triggered"),
];

/// Active skills whose minions the engine knows (their skills give the
/// character nothing, or Wrath, which `gems::wrath` models).
const ALLOWED_MINIONS: &[&str] = &[
    "HeraldOfPurity",
    "SummonFlameGolem",
    "SummonIceGolem",
    "SummonLightningGolem",
    "SummonStoneGolem",
    "SummonStoneGolemAltY",
];

/// A skill's buff identities besides its kind's, with why: Flesh and
/// Stone's Blood Stance mod is an `AuraDebuff` on the enemy and its Sand
/// Stance mod a `Buff` on the character (the engine scales the Blood
/// Stance row as an aura's, which PoB's `AuraDebuff` scale also reads).
const EXTRA_BUFFS: &[(&str, &str)] = &[
    ("FleshAndStone", "AuraDebuff Flesh and Stone"),
    ("FleshAndStone", "Buff Flesh and Stone"),
    // A warcry's `UsesWarcryPower` flag is a `Buff` of its name.
    ("IntimidatingCry", "Buff Intimidating Cry"),
    ("IntimidatingCry", "Warcry Intimidating Cry"),
    ("RallyingCry", "Buff Rallying Cry"),
    ("RallyingCry", "Warcry Rallying Cry"),
];

/// Support mods outside the buff the engine models, with why: Guardian's
/// Blessing's reservation of 0 for an aura (`gems::NO_RESERVATION_SUPPORTS`).
const ALLOWED_BLOCKED: &[(&str, &str)] = &[
    // Automation's trigger of the skill it supports, whose own kind says
    // what it does.
    ("SupportAutomation", "triggered"),
    ("SupportAutomation", "triggeredByAutomation"),
    ("SupportGuardiansBlessing", "lifeReservationFlat"),
    ("SupportGuardiansBlessing", "lifeReservationPercent"),
    ("SupportGuardiansBlessing", "manaReservationFlat"),
    ("SupportGuardiansBlessing", "manaReservationPercent"),
];

/// Types a modelled support may not add.
const SUPPORT_FORBIDDEN: &[&str] = &[
    "AppliesCurse",
    "Aura",
    "Brand",
    "Buff",
    "CreatesMinion",
    "HasReservation",
    "Herald",
    "Hex",
    "Mark",
    "Minion",
    "ReservationBecomesCost",
    "SummonsTotem",
    "Totem",
    "Trigger",
    "Triggered",
    "Warcry",
];

/// PoB mod names a `NoSlice` buff stat may give: each reaches no slice
/// field (regeneration, recharge, movement, degeneration, leech, freeze,
/// damage taken by you).
const NO_SLICE_NAMES: &[&str] = &[
    "AreaOfEffect",
    "ArcaneSurgeManaRegen",
    "Condition:HaveColdGolem",
    "Condition:HaveFireGolem",
    "Condition:HaveLightningGolem",
    "Condition:HavePhysicalGolem",
    "DamageTakenWhenHit",
    "Dummy",
    "FireDegen",
    "EnergyShieldRecharge",
    "FireDamageTakenWhenHit",
    "FreezeImmune",
    "LifeRegen",
    "ManaRegen",
    "MovementSpeed",
    "PhysicalDamageLifeLeech",
    "PhysicalDamageTakenWhenHit",
    "PhysicalDegen",
    "RallyingCryAllyDamageBonusPer5Power",
    "SelfElementalAilmentDuration",
    "SelfFreezeChance",
    "SelfFreezeDuration",
    "UsesWarcryPower",
    "takenFromStoneGolemBeforeYou",
];

/// The `GlobalEffect` tag's fields a slice row may have: any other
/// (`effectCond`, `modCond`, `applyNotPlayer`, a stack variable, ...)
/// makes PoB apply the mod only sometimes, which the engine does not model.
const PLAIN_GLOBAL_FIELDS: &[&str] = &["effectName", "effectType", "type"];

/// Types a modelled support may add after all: Momentum makes its skill a
/// buff (its own movement-speed buff, `NoSlice`); Summon Phantasm makes it
/// create its phantasm (`ALLOWED_ADD_MINIONS`; a minion type only counts
/// into named conditions, as an inert skill's may).
const ALLOWED_ADD_TYPES: &[(&str, &str)] = &[
    // Automation triggers the skill it supports (close-out): the skill's
    // own kind says what it does.
    ("SupportAutomation", "Triggered"),
    ("SupportMomentum", "Buff"),
    ("SupportSummonPhantasm", "CreatesMinion"),
];

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

/// PoB's tag for a `notMinionStat` mod: the actor has no parent.
fn is_not_minion(t: &BTreeMap<String, Value>) -> bool {
    t.get("type").and_then(Value::as_str) == Some("ActorCondition")
        && t.get("actor").and_then(Value::as_str) == Some("parent")
        && t.get("neg") == Some(&Value::Bool(true))
        && t.len() == 3
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
    let mut tags: Vec<String> = m
        .tags
        .iter()
        .filter(|t| t.get("type").and_then(Value::as_str) != Some("GlobalEffect"))
        // `notMinionStat`'s tag (`ActorCondition` `parent`, negated): the
        // mod is not a minion's; every row here is the character's.
        .filter(|t| !is_not_minion(t))
        .map(|t| serde_json::to_string(t).unwrap_or_default())
        .collect();
    if let Some(g) = m.global_effect() {
        let extra: BTreeMap<&String, &Value> = g
            .iter()
            .filter(|(k, _)| !PLAIN_GLOBAL_FIELDS.contains(&k.as_str()))
            .collect();
        if !extra.is_empty() {
            tags.push(format!(
                "global {}",
                serde_json::to_string(&extra).unwrap_or_default()
            ));
        }
    }
    if let Some(v) = &m.value {
        tags.push(format!("value {v}"));
    }
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
        Quantity::Armour => "Armour".into(),
        Quantity::Evasion => "Evasion".into(),
        Quantity::Poe1(Poe1Quantity::EnemyResistance(t)) => format!("{}Resist", title(t.name())),
        Quantity::Poe1(Poe1Quantity::SkillExposure(t)) => format!("{}Exposure", title(t.name())),
        Quantity::Poe1(Poe1Quantity::GainAs(a, b)) => {
            format!("{}DamageGainAs{}", title(a.name()), title(b.name()))
        }
        Quantity::Poe1(Poe1Quantity::EnemyTypeDamageTaken(t)) => {
            format!("{}DamageTaken", title(t.name()))
        }
        Quantity::Poe1(Poe1Quantity::ArcaneSurgeCastSpeed) => "ArcaneSurgeCastSpeed".into(),
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

/// A word with its first letter in upper case.
fn title(word: &str) -> String {
    let mut c = word.chars();
    c.next().map_or(String::new(), |f| {
        f.to_uppercase().collect::<String>() + c.as_str()
    })
}

/// Rows PoB writes as one mod (close-out): the three elemental
/// resistances as `ElementalResist`, energy shield, armour, and evasion
/// increased as `Defences`.
fn collapse(effects: &[(Quantity, Op, f64, Flags, Flags)]) -> Option<String> {
    use loom_calc::DamageType as D;
    let res = |t| Quantity::Poe1(Poe1Quantity::EnemyResistance(t));
    let same = |qs: [Quantity; 3]| {
        effects.len() == 3
            && effects.iter().zip(qs).all(|(e, q)| e.0 == q)
            && effects.iter().all(|e| {
                e.1 == effects[0].1 && e.2 == 1.0 && e.3 == Flags::NONE && e.4 == Flags::NONE
            })
    };
    let kind = effects.first().map(|e| op_name(e.1))?;
    if same([res(D::Fire), res(D::Cold), res(D::Lightning)]) {
        return Some(format!("ElementalResist {kind} []"));
    }
    if same([Quantity::EnergyShield, Quantity::Armour, Quantity::Evasion]) {
        return Some(format!("Defences {kind} []"));
    }
    None
}

fn op_name(op: Op) -> &'static str {
    match op {
        Op::Base => "BASE",
        Op::Increased => "INC",
        Op::More => "MORE",
        Op::Flag => "FLAG",
        Op::Override => "OVERRIDE",
    }
}

/// One row of a table (`poe1::mapping::HitEffect`).
type Row = (Quantity, Op, f64, Flags, Flags);

/// Our rows of a stat as PoB's mods, with the tags its variant stands for.
fn variant_form(b: &BuffStat) -> Vec<String> {
    let (rows, suffix): (&[Row], String) = match b {
        BuffStat::Rows(r) => (r, String::new()),
        BuffStat::WhenInput {
            condition, rows, ..
        } => (
            rows,
            format!(" tags global {}", json!({ "effectCond": condition })),
        ),
        BuffStat::BloodStance(rows) => (
            rows,
            format!(" tags global {}", json!({ "effectCond": "BloodStance" })),
        ),
        BuffStat::Threshold(rows) => (
            rows,
            format!(
                " tags {};{}",
                json!({ "stat": "LifeUnreserved", "threshold": 2, "type": "StatThreshold" }),
                json!({ "stat": "Mana", "threshold": 1, "type": "StatThreshold" })
            ),
        ),
        BuffStat::NoSlice(_) | BuffStat::Gated(_) => return Vec::new(),
    };
    if let Some(one) = collapse(rows) {
        return vec![one + &suffix];
    }
    our_form(rows)
        .into_iter()
        .zip(rows)
        .map(|(f, row)| {
            // Arcane Surge's cast speed: PoB's MAX mod, `unscalable`; the
            // engine keeps the largest (`poe1::buffs`).
            if row.0 == Quantity::Poe1(Poe1Quantity::ArcaneSurgeCastSpeed) {
                return f.replacen(" BASE ", " MAX ", 1)
                    + &format!(" tags global {}", json!({ "unscalable": true }));
            }
            f + &suffix
        })
        .collect()
}

fn our_form(effects: &[(Quantity, Op, f64, Flags, Flags)]) -> Vec<String> {
    effects
        .iter()
        .map(|&(q, op, divisor, all, any)| {
            // A damage type's more or increased (Herald of Purity's
            // `PhysicalDamage` MORE): the type is the name.
            let typed = [
                (Flags::PHYSICAL, "Physical"),
                (Flags::FIRE, "Fire"),
                (Flags::COLD, "Cold"),
                (Flags::LIGHTNING, "Lightning"),
                (Flags::CHAOS, "Chaos"),
            ]
            .into_iter()
            .find(|(f, _)| q == Quantity::Damage && all.contains(*f));
            let (name, all) = match typed {
                Some((f, t)) => (
                    format!("{t}Damage"),
                    if all == f { Flags::NONE } else { all },
                ),
                None => (pob_name(q), all),
            };
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
                name,
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
    let own_effect: Vec<&str> = gems::OWN_EFFECT_STATS.iter().map(|(s, _)| *s).collect();
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
            {
                let extra: Vec<String> = EXTRA_BUFFS
                    .iter()
                    .filter(|(s, _)| *s == id)
                    .map(|(_, b)| (*b).to_owned())
                    .collect();
                if extra.is_empty() {
                    strings([format!("{effect} {name}")])
                } else {
                    strings(extra)
                }
            },
            strings(pob),
        );
    }
    // `NoSlice` and `Gated` stats: what PoB's mods are, and when.
    for (stat, b) in stats {
        let mods = || {
            skill
                .mods
                .iter()
                .filter(|s| s.stat == *stat)
                .flat_map(|s| s.mods.iter())
        };
        match b {
            BuffStat::NoSlice(_) => {
                // Only the global mods form the buff; a skill's other mods
                // act on itself (close-out: base mods share the stat "").
                let bad: Vec<String> = mods()
                    .filter(|m| global(m).is_some())
                    .filter(|m| !NO_SLICE_NAMES.contains(&m.name.as_str()))
                    .map(|m| m.name.clone())
                    .collect();
                out.check(
                    TABLE,
                    id,
                    format!("no_slice.{stat}"),
                    strings(Vec::new()),
                    strings(bad),
                );
            }
            BuffStat::Gated(cond)
            | BuffStat::WhenInput {
                condition: cond, ..
            } => {
                let conds: BTreeSet<String> = mods()
                    .map(|m| {
                        global(m)
                            .and_then(|g| g.get("effectCond"))
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_owned()
                    })
                    .collect();
                out.check(
                    TABLE,
                    id,
                    format!("gated.{stat}"),
                    strings([(*cond).to_owned()]),
                    strings(conds),
                );
            }
            BuffStat::Rows(_) | BuffStat::BloodStance(_) | BuffStat::Threshold(_) => {}
        }
    }
    // Rows.
    for (stat, b) in stats {
        let ours = variant_form(b);
        if ours.is_empty() {
            continue;
        }
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
            strings(ours),
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
            // A gem's own buff or aura effect stat is read by the engine.
            if own_effect.contains(&s.stat.as_str()) {
                continue;
            }
            if key.is_some_and(|k| ALLOWED_BLOCKED.contains(&(id, k))) {
                continue;
            }
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
        .filter(|t| !ALLOWED_TYPES.contains(&(id, t.as_str())))
        .cloned()
        .collect();
    out.check(TABLE, id, "types", strings(Vec::new()), strings(bad));
    out.check(
        TABLE,
        id,
        "is_trigger",
        json!(ALLOWED_ADD_TYPES.contains(&(id, "Triggered")) && support),
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
        let minions = if ALLOWED_MINIONS.contains(&id) {
            Vec::new()
        } else {
            skill.minions.clone()
        };
        out.check(TABLE, id, "minions", strings(Vec::new()), strings(minions));
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
        let forbidden = match kind {
            Kind::Inert => INERT_FORBIDDEN,
            Kind::Buff { .. } => BUFF_FORBIDDEN,
            Kind::Debuff { .. } => DEBUFF_FORBIDDEN,
            Kind::Curse { .. } => CURSE_FORBIDDEN,
            Kind::Warcry { .. } => WARCRY_FORBIDDEN,
        };
        let buff = kind.buff().map(|(effect, name)| (name, effect));
        check_one(out, id, skill, kind.stats(), buff, forbidden, false);
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
