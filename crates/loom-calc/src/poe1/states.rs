//! The named conditions the build settles (Phase 3 plan, step 8; P3-8f):
//! what every PoE1 query context knows.
//!
//! **PoB v2.67.2** (`Modules/CalcPerform.lua`, `doActorAttribsConditions`,
//! 135 onward) sets, among others:
//! - `UsingShield` when the off hand ("Weapon 2") holds a shield;
//! - `Using<flag>` for the main hand's weapon type
//!   (`data.weaponTypeInfo`, `Modules/Data.lua` 550: a staff or warstaff
//!   is `Staff`).
//!
//! **The engine** settles only the conditions a mapping row reads
//! ([`USING_STAFF`], [`USING_SHIELD`]), each from the hands of the weapon
//! set in use ([`collect::hands`]): a hand holding an item whose base is
//! not known leaves its condition unsettled, so an effect that needs it is
//! `unmodelled-condition` (a reason), never a guess. Any other condition
//! stays unsettled, as before P3-8f: `OffHandIsEmpty`, `Unarmed`,
//! `UsingMeleeWeapon`, `UsingOneHandedWeapon`, `UsingTwoHandedWeapon`,
//! `DualWielding`, the other weapon types, and Necromantic Aegis's
//! shield; an effect that needs one is a reason.
//!
//! Differences on purpose: PoB also sets `Using<flag>` from the off hand's
//! weapon type (`CalcPerform.lua` 187 to 190), but a staff cannot be held
//! in the off hand. A shield is an armour base with a chance to block
//! (only shields have one). The conditions are settled once per
//! calculation (`Ruleset::states`, P3-8g); every query context comes from
//! `Calc::context`.

use std::collections::{BTreeMap, BTreeSet};

use loom_domain::{LineKind, LineResolution, Resolved};

use super::collect::{self, Held};
use crate::condition::ConditionKey;
use crate::input::CalcInput;

/// The main hand holds a staff or a warstaff (PoB's `UsingStaff`).
pub const USING_STAFF: &str = "UsingStaff";

/// The off hand holds a shield (PoB's `UsingShield`).
pub const USING_SHIELD: &str = "UsingShield";

/// The main hand holds a weapon (P3-8n): PoB gives an attack with it
/// `ModFlag.Weapon` (`CalcActiveSkill.lua` 209: the weapon's type is not
/// `None`), which "with Weapons" modifiers need. Not a PoB condition.
pub const MAIN_HAND_WEAPON: &str = "MainHandWeapon";

/// "if there are no Life Modifiers on Equipped Body Armour" (P3-8n). PoB
/// v2.67.2 reads it as an `ItemCondition` with `searchCond = "life"`
/// (`Data/ModCache.lua` 3483): no mod line of the body armour has the text
/// "life" (`Classes/Item.lua` 284 to 320, `FindModifierSubstring`, with
/// its special and excluded lines). The game's rule is the mods' `life`
/// tag, which the engine reads instead, settling only what the tags
/// decide ([`no_life_modifiers_on_body_armour`]). The two rules agree on
/// the P3-8n fixture and the in-scope builds; they are not proven equal in
/// general (a line with the text "life" on an untagged mod, or a tagged mod
/// whose text lacks it, would differ), so a build where they differ shows
/// in parity and needs its divergence entry then.
pub const NO_LIFE_MODIFIERS_ON_BODY_ARMOUR: &str = "NoLifeModifiersOnBodyArmour";

/// The stat that reads [`NO_LIFE_MODIFIERS_ON_BODY_ARMOUR`].
pub const NO_LIFE_MODIFIERS_STAT: &str = "maximum_life_+%_if_no_life_tags_on_body_armour";

/// The game's implicit tag of a Life Modifier.
const LIFE_TAG: &str = "life";

/// The item classes of PoB's `Staff` weapon type.
const STAFF_CLASSES: [&str; 2] = ["Staff", "Warstaff"];

/// The conditions the build settles (module docs).
#[must_use]
pub fn states(input: &CalcInput<'_>) -> BTreeMap<ConditionKey, bool> {
    let hands = collect::hands(input);
    let mut out = BTreeMap::new();
    let staff = match &hands.main_hand {
        Held::Weapon(class) => Some(STAFF_CLASSES.contains(&class.as_str())),
        Held::Empty | Held::Shield | Held::Other => Some(false),
        Held::Unknown => None,
    };
    let shield = match &hands.off_hand {
        Held::Shield => Some(true),
        Held::Empty | Held::Weapon(_) | Held::Other => Some(false),
        Held::Unknown => None,
    };
    let weapon = match &hands.main_hand {
        Held::Weapon(_) => Some(true),
        Held::Empty | Held::Shield | Held::Other => Some(false),
        Held::Unknown => None,
    };
    for (key, value) in [
        (USING_STAFF, staff),
        (USING_SHIELD, shield),
        (MAIN_HAND_WEAPON, weapon),
    ] {
        if let Some(v) = value {
            out.insert(ConditionKey::new(key), v);
        }
    }
    // The config's condition checks (P3-8n).
    super::conditions::settle(input, &mut out);
    if reads_life_modifiers(input)
        && let Some(v) = no_life_modifiers_on_body_armour(input)
    {
        out.insert(ConditionKey::new(NO_LIFE_MODIFIERS_ON_BODY_ARMOUR), v);
    }
    out
}

/// Whether an allocated node or a chosen mastery effect carries
/// [`NO_LIFE_MODIFIERS_STAT`] (it is a Life Mastery's): only then is the
/// condition settled (it reads every mod of the data). From another source
/// the stat meets it unsettled, a reason.
fn reads_life_modifiers(input: &CalcInput<'_>) -> bool {
    let has = |stats: Option<&[crate::input::StatAmount]>| {
        stats.is_some_and(|stats| stats.iter().any(|s| s.stat == NO_LIFE_MODIFIERS_STAT))
    };
    collect::active_spec(input.build).is_some_and(|spec| {
        spec.nodes
            .value()
            .iter()
            .any(|n| has(input.data.passive_stats(n.0)))
            || spec
                .mastery_effects
                .value()
                .iter()
                .any(|m| has(input.data.mastery_effect_stats(m.effect)))
    })
}

/// Whether the equipped body armour has no Life Modifier (P3-8n): a
/// modifier is one whose mod has the game's `life` implicit tag. The
/// item's lines carry their stats, not their mods, so:
/// - a line whose stat is only on mods with the tag is a Life Modifier:
///   `false`;
/// - an affix PoB's `Prefix:`/`Suffix:` record names that has the tag, with
///   all its stats among the item's lines, is a Life Modifier: `false`
///   (a crafted rare's "+# to maximum Life" is on untagged mods too, so
///   only the record settles it);
/// - when no mod with the tag has every one of its stats among the item's
///   lines, none of its mods can have the tag: `true`;
/// - otherwise not settled (`None`), as for an empty slot, a line that is
///   not resolved to stats, or data without implicit tags.
fn no_life_modifiers_on_body_armour(input: &CalcInput<'_>) -> Option<bool> {
    let build = input.build;
    let set = collect::active_item_set(build)?;
    let id = set
        .slots
        .typed()
        .find(|e| e.slot.as_str() == "Body Armour")
        .and_then(|e| e.item)
        .filter(|id| id.0 != 0)?;
    let item = build.items.item(id)?;
    let mut stats = BTreeSet::new();
    let mut named = BTreeSet::new();
    for line in &item.body.lines {
        match &line.kind {
            LineKind::Spec { key, value, .. } if AFFIX_KEYS.contains(&key.as_str()) => {
                if let Some(id) = affix_mod(value) {
                    named.insert(id.to_owned());
                }
            }
            LineKind::Modifier(m) => match &m.resolution {
                LineResolution::Resolved {
                    result: Resolved::Stats(lines),
                    ..
                } => stats.extend(lines.iter().map(|l| l.stat.as_str().to_owned())),
                LineResolution::Excluded(loom_domain::Exclusion::NotSelected)
                | LineResolution::Continues { .. } => {}
                _ => return None,
            },
            LineKind::Unrecognized | LineKind::Spec { known: false, .. } => return None,
            _ => {}
        }
    }
    tagless(&stats, &named, &input.data.mods_by_tag(LIFE_TAG)?)
}

/// PoB's spec keys that name an item's crafted affixes (`Prefix: {range:r}
/// <mod id>`, `Suffix: ...`; `Classes/Item.lua` `ParseRaw`).
const AFFIX_KEYS: [&str; 2] = ["Prefix", "Suffix"];

/// The mod ID a `Prefix:` or `Suffix:` value names: the value without its
/// `{range:r}` prefix; `None` for PoB's empty affix ("None") or no ID.
fn affix_mod(value: &str) -> Option<&str> {
    let v = value.trim();
    let v = match v.strip_prefix('{') {
        Some(rest) => rest.split_once('}')?.1.trim(),
        None => v,
    };
    (!v.is_empty() && v != "None").then_some(v)
}

/// Whether no mod of an item with lines of `stats` can carry the tag of
/// `mods` ([`no_life_modifiers_on_body_armour`]'s rule): `Some(false)` when
/// a stat is only on tagged mods, or when an affix the item names (`named`,
/// PoB's `Prefix:`/`Suffix:` record of the crafted mod) is a tagged mod
/// whose stats are all among `stats` (the item has that mod's lines);
/// `Some(true)` when no tagged mod has all its stats among `stats`, else
/// `None`.
fn tagless(
    stats: &BTreeSet<String>,
    named: &BTreeSet<String>,
    mods: &crate::input::ModsByTag,
) -> Option<bool> {
    let present = |m: &crate::input::TaggedMod| m.stats.iter().all(|s| stats.contains(s));
    if mods
        .tagged
        .iter()
        .any(|m| named.contains(&m.id) && present(m))
    {
        return Some(false);
    }
    let tagged_only = stats
        .iter()
        .any(|s| !mods.untagged.contains(s) && mods.tagged.iter().any(|m| m.stats.contains(s)));
    if tagged_only {
        return Some(false);
    }
    if mods.tagged.iter().any(present) {
        return None;
    }
    Some(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{ModsByTag, TaggedMod};

    fn set(ids: &[&str]) -> BTreeSet<String> {
        ids.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn a_body_armours_life_modifiers_should_be_settled_only_when_certain() {
        let tagged = |id: &str, stats: &[&str]| TaggedMod {
            id: id.to_owned(),
            stats: stats.iter().map(|s| (*s).to_owned()).collect(),
        };
        let mods = ModsByTag {
            tagged: vec![
                tagged("IncreasedLife6", &["base_maximum_life"]),
                tagged("Hybrid", &["armour", "base_maximum_life"]),
                tagged("Regen", &["life_regeneration"]),
            ],
            untagged: set(&["armour", "fire_resistance", "base_maximum_life"]),
        };
        let none = BTreeSet::new();
        // No tagged mod has all its stats here: no Life Modifier.
        assert_eq!(
            tagless(&set(&["armour", "fire_resistance"]), &none, &mods),
            Some(true)
        );
        // A stat only tagged mods carry: a Life Modifier.
        assert_eq!(
            tagless(&set(&["armour", "life_regeneration"]), &none, &mods),
            Some(false)
        );
        // A tagged mod's stats are all here, but an untagged mod also
        // carries them: not settled.
        assert_eq!(tagless(&set(&["base_maximum_life"]), &none, &mods), None);
        // PoB's affix record names the tagged mod and its lines are here:
        // a Life Modifier.
        let named = set(&["IncreasedLife6"]);
        assert_eq!(
            tagless(&set(&["base_maximum_life"]), &named, &mods),
            Some(false)
        );
        // A named tagged mod whose lines are not on the item settles
        // nothing (the record and the lines disagree).
        assert_eq!(
            tagless(&set(&["fire_resistance"]), &named, &mods),
            Some(true)
        );
        // No line: no Life Modifier.
        assert_eq!(tagless(&BTreeSet::new(), &none, &mods), Some(true));
    }

    #[test]
    fn an_affix_record_should_name_its_mod() {
        assert_eq!(
            affix_mod("{range:0.042}IncreasedLife6"),
            Some("IncreasedLife6")
        );
        assert_eq!(affix_mod(" IncreasedLife6 "), Some("IncreasedLife6"));
        assert_eq!(affix_mod("None"), None);
        assert_eq!(affix_mod("{range:0.5}"), None);
        assert_eq!(affix_mod("{range:0.5"), None);
    }
}
