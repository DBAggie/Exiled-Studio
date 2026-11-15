//! Projections of the domain onto the API DTOs (ADR-004: queries return
//! typed projections; the UI never reads the domain).

use std::collections::HashMap;

use loom_calc::CalcResult;
use loom_calc::poe1::collect::{InputUse, ItemPlace};
use loom_data::bundle::Bundle;
use loom_domain::{
    Build, ConfigInputKind, ConfigValue, DataVersion, Item, ItemId, ItemLine, League, LineKind,
    LineResolution, QuarantineReason, Resolved, StatValue, UnresolvedReason,
};
use loom_parser::import::{BundleData, ItemData as _};
use loom_serialization::dto::app::v1 as api;
use loom_serialization::legacy::pob_item;

use crate::loadout;

/// The data version's view.
#[must_use]
pub fn data_version(v: &DataVersion) -> api::DataVersionView {
    api::DataVersionView {
        game: v.game.name().to_owned(),
        schema_version: v.schema_version,
        steam_build: v.steam_build,
    }
}

/// The slot each item is in, in the active item set (else the first).
fn slot_of(build: &Build, id: ItemId) -> Option<String> {
    let set = build
        .items
        .sets()
        .find(|s| Some(s.id) == build.items.active_set)
        .or_else(|| build.items.sets().next())?;
    set.slots
        .typed()
        .find(|s| s.item == Some(id))
        .map(|s| s.slot.as_str().to_owned())
}

/// An item's sockets as linked groups (`R-R-G B` → `RRG`, `B`), from
/// its `Sockets` line; empty without one.
fn socket_groups(item: &Item) -> Vec<String> {
    item.body
        .lines
        .iter()
        .find_map(|l| match &l.kind {
            LineKind::Spec { key, value, .. } if key == "Sockets" => Some(value.as_str()),
            _ => None,
        })
        .map(|v| {
            v.split_whitespace()
                .map(|g| {
                    g.chars()
                        .filter(|c| "RGBWA".contains(*c))
                        .collect::<String>()
                })
                .filter(|g| !g.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// The socket groups of the item in `slot` of the active item set.
fn sockets_in(build: &Build, slot: &str) -> Option<Vec<String>> {
    let set = loom_calc::poe1::collect::active_item_set(build)?;
    let id = set
        .slots
        .typed()
        .filter(|e| e.slot.as_str() == slot)
        .last()
        .and_then(|e| e.item)?;
    build.items.item(id).map(socket_groups)
}

/// A name made from the build: main skill, ascendancy (else class), and
/// level, as players name builds ("Smite Inquisitor 92"); empty when it
/// has none of them (the UI names it then).
fn derived_name(build: &Build, skill: Option<&str>) -> String {
    let named = |s: &str| !s.is_empty() && s != "None";
    let who = build
        .character
        .ascendancy
        .as_ref()
        .map(|a| a.as_str())
        .filter(|a| named(a))
        .or_else(|| build.character.class.as_ref().map(|c| c.as_str()))
        .filter(|c| named(c));
    let level = build
        .character
        .level
        .as_ref()
        .map(|l| l.value().to_string());
    [skill.map(str::to_owned), who.map(str::to_owned), level]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ")
}

/// The main skill: in the build's main socket group, the active skill its
/// `mainActiveSkill` names, counting the gems that are not supports (UX
/// walkthrough, UX-11; a gem the data does not know counts as active).
/// The group's gem names when no gem's name is known that way.
fn main_skill(build: &Build, groups: &[api::SkillGroupView]) -> Option<String> {
    let index = usize::try_from(*build.character.main_socket_group.as_ref()?.value()).ok()?;
    let set = build
        .skills
        .sets
        .typed()
        .find(|s| Some(s.id) == build.skills.active)
        .or_else(|| build.skills.sets.typed().next())?;
    let group = set.groups.typed().nth(index.checked_sub(1)?)?;
    let view = groups.get(index - 1)?;
    view.gems
        .get(main_active_index(group, &view.gems)?)?
        .name
        .clone()
}

fn warning_code(line: &ItemLine) -> Option<&'static str> {
    let reason = match &line.kind {
        LineKind::Unrecognized => QuarantineReason::UnrecognizedLine,
        LineKind::Spec { known: false, .. } => QuarantineReason::UnknownSpec,
        LineKind::Modifier(m) => match &m.resolution {
            LineResolution::Pending => QuarantineReason::Pending,
            LineResolution::Unresolved(r) => QuarantineReason::Unresolved(r.clone()),
            _ => return None,
        },
        _ => return None,
    };
    Some(reason.code())
}

fn value_text(v: &StatValue) -> String {
    match v {
        StatValue::Exact { min, max } | StatValue::Ranged { min, max } if min == max => {
            min.to_string()
        }
        StatValue::Exact { min, max } | StatValue::Ranged { min, max } => format!("{min} to {max}"),
        StatValue::Hidden { assumed: Some(a) } => format!("hidden ({a})"),
        StatValue::Hidden { assumed: None } => "hidden".to_owned(),
        StatValue::Lookup(values) => values
            .iter()
            .map(i64::to_string)
            .collect::<Vec<_>>()
            .join(", "),
    }
}

/// The numbers in a line's text, outside `{...}` prefixes.
fn numbers(text: &str) -> Vec<api::NumberSpan> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    let mut depth = 0_u32;
    while let Some(&c) = chars.get(i) {
        match c {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            _ if depth == 0 && c.is_ascii_digit() => {
                let start = i;
                while chars
                    .get(i)
                    .is_some_and(|c| c.is_ascii_digit() || *c == '.')
                {
                    i += 1;
                }
                // A trailing dot ends a sentence, not a number.
                let mut end = i;
                if chars.get(end - 1) == Some(&'.') {
                    end -= 1;
                }
                let (Ok(s), Ok(e)) = (u32::try_from(start), u32::try_from(end)) else {
                    break;
                };
                out.push(api::NumberSpan {
                    start: s,
                    end: e,
                    text: chars.get(start..end).unwrap_or_default().iter().collect(),
                });
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    out
}

/// One line's view.
#[must_use]
pub fn line_view(index: usize, line: &ItemLine) -> api::LineView {
    let mut view = api::LineView {
        index: u32::try_from(index).unwrap_or(u32::MAX),
        text: line.source.clone(),
        kind: line.kind.name().to_owned(),
        section: None,
        status: api::LineStatus::Other,
        code: warning_code(line).map(str::to_owned),
        stats: Vec::new(),
        typed: None,
        adapted: None,
        numbers: numbers(&line.source),
        shown: None,
        ranged: None,
        variants: Vec::new(),
        marks: Vec::new(),
    };
    if view.code.is_some() {
        view.status = api::LineStatus::Warning;
    }
    if let LineKind::Modifier(m) = &line.kind {
        view.section = Some(m.section.name().to_owned());
        view.shown = pob_item::shown_text(m).or_else(|| Some(m.text.clone()));
        view.ranged = pob_item::ranged_text(m);
        view.variants = m
            .prefixes
            .iter()
            .filter(|p| p.key == "variant")
            .filter_map(|p| p.value.as_deref())
            .flat_map(|v| v.split(','))
            .filter_map(|n| n.trim().parse().ok())
            .collect();
        view.marks = m
            .prefixes
            .iter()
            .filter(|p| MARKS.contains(&p.key.as_str()))
            .map(|p| p.key.clone())
            .collect();
        match &m.resolution {
            LineResolution::Resolved { result, adapted } => {
                view.status = if adapted.is_some() {
                    api::LineStatus::Adapted
                } else {
                    api::LineStatus::Matched
                };
                view.adapted = adapted.map(|a| a.name().to_owned());
                match result {
                    Resolved::Stats(stats) => {
                        view.stats = stats
                            .iter()
                            .map(|s| api::StatView {
                                id: s.stat.as_str().to_owned(),
                                value: value_text(&s.value),
                            })
                            .collect();
                    }
                    Resolved::Typed(t) => view.typed = Some(t.kind().to_owned()),
                }
            }
            LineResolution::Continues { .. } => view.status = api::LineStatus::Continues,
            LineResolution::Excluded(_) => view.status = api::LineStatus::Excluded,
            LineResolution::Pending | LineResolution::Unresolved(_) => {}
        }
        if let LineResolution::Unresolved(UnresolvedReason::Ambiguous(options)) = &m.resolution {
            view.stats = options
                .iter()
                .flatten()
                .map(|s| api::StatView {
                    id: s.as_str().to_owned(),
                    value: "?".to_owned(),
                })
                .collect();
        }
    }
    view
}

/// One item's view.
#[must_use]
pub fn item_view(build: &Build, item: &Item, data: Option<&BundleData<'_>>) -> api::ItemView {
    api::ItemView {
        id: item.id.0,
        name: item.body.name.clone(),
        base: item.body.base.clone(),
        rarity: item.body.rarity.map(|r| r.name().to_owned()),
        slot: slot_of(build, item.id),
        lines: item
            .body
            .lines
            .iter()
            .enumerate()
            .map(|(i, l)| line_view(i, l))
            .collect(),
        card: card(item, data),
    }
}

/// [`item_view`] with the card's values ([`crate::card_values`]), which
/// need the item's range ends and the game data.
pub(crate) fn item_view_of(
    state: &crate::State,
    item: &Item,
    data: Option<&crate::GameData>,
) -> api::ItemView {
    let mut view = item_view(&state.build, item, data.map(|d| d.data));
    view.card.values = data.and_then(|d| crate::card_values::values(item, &state.ends, d));
    view
}

/// The item's affixes, named and inferred, with tiers
/// ([`loom_parser::import::affixes`]).
fn affixes(item: &Item, data: &BundleData<'_>) -> Vec<api::AffixView> {
    use loom_parser::import::affixes::{AffixKind, item_affixes};
    use loom_parser::import::{BaseQuery, find_base};
    let names = pob_item::item_base(&item.body, data.base_names()).ok();
    let base = names.as_ref().and_then(|(name, fallback)| {
        find_base(
            data,
            BaseQuery {
                name,
                fallback: fallback.as_deref(),
            },
        )
        .ok()
    });
    item_affixes(&item.body, data, base.map(|b| b.rows))
        .into_iter()
        .map(|a| api::AffixView {
            kind: match a.kind {
                AffixKind::Implicit => api::AffixKind::Implicit,
                AffixKind::Prefix => api::AffixKind::Prefix,
                AffixKind::Suffix => api::AffixKind::Suffix,
            },
            name: a.name,
            tier: a.tier,
            crafted: a.crafted,
            inferred: a.inferred,
            tags: a.tags,
            lines: a
                .lines
                .into_iter()
                .filter_map(|i| u32::try_from(i).ok())
                .collect(),
        })
        .collect()
}

/// The item's requirements: the level as written (`LevelReq`), else the
/// base's drop level; the base's attribute requirements (data schema 4).
fn requirements(
    item: &Item,
    written: Option<&str>,
    bundle: Option<&Bundle>,
) -> Option<api::Requirements> {
    let base = bundle.and_then(|b| {
        if b.manifest.data_schema < loom_data::bundle::AFFIX_SCHEMA {
            return None;
        }
        let (name, fallback) =
            pob_item::item_base(&item.body, b.base_items.iter().map(|x| x.name.as_str())).ok()?;
        let find = |n: &str| b.base_items.iter().find(|x| x.name == n);
        find(&name).or_else(|| fallback.as_deref().and_then(find))
    });
    // The base's level by PoB's rule (`Bundle::required_level`): a jewel
    // or a ring without implicits requires none, as the game shows.
    let level = written
        .and_then(|w| w.trim().parse::<u32>().ok())
        .or_else(|| {
            bundle
                .zip(base)
                .and_then(|(b, base)| b.required_level(base))
                .and_then(|l| u32::try_from(l).ok())
        });
    let attrs = base.and_then(|b| b.requirements).unwrap_or_default();
    let attr = |v: i64| u32::try_from(v).unwrap_or(0);
    let out = api::Requirements {
        level,
        strength: attr(attrs.strength),
        dexterity: attr(attrs.dexterity),
        intelligence: attr(attrs.intelligence),
    };
    (out.level.is_some() || out.strength + out.dexterity + out.intelligence > 0).then_some(out)
}

/// An item's requirements from its own `LevelReq` (or `Requires Level`)
/// line and its base ([`requirements`]).
fn item_requirements(item: &Item, bundle: Option<&Bundle>) -> Option<api::Requirements> {
    let written = item.body.lines.iter().find_map(|l| match &l.kind {
        LineKind::Spec { key, value, .. } if key == "LevelReq" || key == "Requires Level" => {
            Some(value.trim().to_owned())
        }
        _ => None,
    });
    requirements(item, written.as_deref(), bundle)
}

/// The base values an item card shows, by spec key, in this order.
const CARD_PROPERTIES: [&str; 8] = [
    "Armour",
    "Evasion Rating",
    "Evasion",
    "Energy Shield",
    "Ward",
    "Radius",
    "Limited to",
    "Requires Class",
];

/// A unique's flavour text (B-2), from the game data's rows for its name:
/// none when it has none, or several texts (variants with their own).
fn flavour(item: &Item, bundle: &loom_data::bundle::Bundle) -> Vec<api::FlavourLine> {
    let unique = item
        .body
        .rarity
        .is_some_and(|r| matches!(r.name(), "unique" | "relic"));
    let (true, Some(name), Some(poe1)) = (unique, item.body.name.as_deref(), &bundle.poe1) else {
        return Vec::new();
    };
    // A Foulborn unique reads its plain version's text, as PoB v2.67.2's
    // tooltip does (`ItemsTab.lua`, "Show flavour text").
    let name = name.trim();
    let name = match name.strip_prefix("Foulborn ") {
        Some(plain) if poe1.flavour_of(name).next().is_none() => plain,
        _ => name,
    };
    // Distinct texts: variants that print the same text still show it.
    let texts: std::collections::BTreeSet<&str> =
        poe1.flavour_of(name).map(|r| r.text.as_str()).collect();
    match (texts.first(), texts.len()) {
        (Some(text), 1) => flavour_lines(text),
        _ => Vec::new(),
    }
}

/// The game's flavour text as lines: `<<...>>` glyph runs (a script the
/// app has no font for) dropped, a `<default>{...}` line marked for the
/// default colour, other tags and braces removed; blank lines kept as one
/// gap between text, none at the ends.
fn flavour_lines(text: &str) -> Vec<api::FlavourLine> {
    let mut out: Vec<api::FlavourLine> = Vec::new();
    for raw in text.lines() {
        let (mut line, mut default_colour) = (String::new(), false);
        let mut rest = raw;
        while let Some(c) = rest.chars().next() {
            if let Some(after) = rest.strip_prefix("<<") {
                rest = after.find(">>").map_or("", |end| &after[end + 2..]);
            } else if let Some(after) = rest.strip_prefix('<') {
                let end = after.find('>').unwrap_or(after.len());
                default_colour |= &after[..end] == "default";
                rest = after.get(end + 1..).unwrap_or("");
            } else {
                if c != '{' && c != '}' {
                    line.push(c);
                }
                rest = &rest[c.len_utf8()..];
            }
        }
        let line = line.trim();
        if line.is_empty() {
            if out.last().is_some_and(|l| !l.text.is_empty()) {
                out.push(api::FlavourLine {
                    text: String::new(),
                    default_colour: false,
                });
            }
        } else if !(raw.contains("<<") && line.chars().all(|c| !c.is_alphanumeric())) {
            out.push(api::FlavourLine {
                text: line.to_owned(),
                default_colour,
            });
        }
    }
    while out.last().is_some_and(|l| l.text.is_empty()) {
        out.pop();
    }
    out
}

/// An item's card (UX-08): the header and spec lines a player reads.
fn card(item: &Item, data: Option<&BundleData<'_>>) -> api::ItemCard {
    let bundle = data.map(BundleData::bundle);
    let spec = |name: &str| {
        item.body.lines.iter().find_map(|l| match &l.kind {
            LineKind::Spec { key, value, .. } if key == name => Some(value.trim().to_owned()),
            _ => None,
        })
    };
    let specs = || {
        item.body.lines.iter().filter_map(|l| match &l.kind {
            LineKind::Spec { key, value, .. } => Some((key.as_str(), value.trim())),
            _ => None,
        })
    };
    api::ItemCard {
        item_class: item.body.item_class.clone(),
        quality: spec("Quality"),
        item_level: spec("Item Level"),
        level_req: spec("LevelReq").or_else(|| spec("Requires Level")),
        sockets: socket_groups(item),
        properties: CARD_PROPERTIES
            .iter()
            .flat_map(|name| {
                specs()
                    .filter(move |(k, _)| k == name)
                    .map(|(k, v)| api::CardProperty {
                        name: k.to_owned(),
                        value: v.to_owned(),
                    })
            })
            .collect(),
        influences: item
            .body
            .influences()
            .map(|i| i.as_str().to_owned())
            .collect(),
        flags: item
            .body
            .lines
            .iter()
            .filter(|l| matches!(l.kind, LineKind::Flag))
            .map(|l| l.source.trim().to_owned())
            .collect(),
        variant_eras: pob_item::variant_eras(&variant_names(item))
            .into_iter()
            .map(era)
            .collect(),
        current_variant: item
            .body
            .variants
            .selected
            .and_then(|v| pob_item::current_variant(&variant_names(item), v)),
        affixes: data.map(|d| affixes(item, d)).unwrap_or_default(),
        requirements: item_requirements(item, bundle),
        flavour: bundle.map(|b| flavour(item, b)).unwrap_or_default(),
        values: None,
        variants: variant_names(item),
        selected_variant: item.body.variants.selected,
    }
}

/// The prefixes a line view names as marks.
/// PoB's item set attribute for the weapon swap.
pub(crate) const SECOND_WEAPON_SET: &str = "useSecondWeaponSet";

const MARKS: [&str; 6] = [
    "crafted",
    "fractured",
    "crucible",
    "scourge",
    "synthesis",
    "mutated",
];

/// A gem's colour name from the game's `Attribute` (as
/// `loom_calc::poe1::sockets` reads it).
pub(crate) fn colour(attribute: i64) -> Option<&'static str> {
    match attribute {
        1 => Some("red"),
        2 => Some("green"),
        3 => Some("blue"),
        4 => Some("white"),
        _ => None,
    }
}

fn gem_view(gem: &loom_domain::Gem, bundle: Option<&Bundle>) -> api::GemView {
    let effect = bundle
        .and_then(|b| loom_calc::poe1::skill::gem_effects(gem, b))
        .and_then(|e| e.first().copied());
    api::GemView {
        name: gem.name.clone(),
        level: *gem.level.value(),
        quality: *gem.quality.value(),
        enabled: gem.enabled,
        support: effect.map(|e| e.support),
        colour: effect
            .and_then(|e| e.attribute)
            .and_then(colour)
            .map(str::to_owned),
        main: false,
        supports: Vec::new(),
        socket: None,
    }
}

/// The active skill set's socket groups, in the calculation's order
/// (its `gem` sources index the typed groups and gems).
fn skill_groups(build: &Build, bundle: Option<&Bundle>) -> Vec<api::SkillGroupView> {
    let Some(set) = build
        .skills
        .sets
        .typed()
        .find(|s| Some(s.id) == build.skills.active)
        .or_else(|| build.skills.sets.typed().next())
    else {
        return Vec::new();
    };
    let main = build
        .character
        .main_socket_group
        .as_ref()
        .map(|m| *m.value());
    // Per slot: the sockets its earlier groups' gems have filled.
    let mut filled: HashMap<String, usize> = HashMap::new();
    set.groups
        .typed()
        .enumerate()
        .map(|(i, g)| {
            let is_main = u32::try_from(i + 1).ok() == main;
            let mut gems: Vec<api::GemView> =
                g.gems.typed().map(|gem| gem_view(gem, bundle)).collect();
            if let Some(b) = bundle {
                let links = loom_calc::poe1::skill::group_links(g, b);
                for (gem, links) in gems.iter_mut().zip(links) {
                    gem.supports = links
                        .into_iter()
                        .filter_map(|a| u32::try_from(a).ok())
                        .collect();
                }
            }
            if let Some(slot) = g.slot.as_ref().map(|s| s.as_str()) {
                let count = sockets_in(build, slot).map_or(0, |s| s.concat().len());
                let start = filled.get(slot).copied().unwrap_or(0);
                for (k, gem) in gems.iter_mut().enumerate() {
                    gem.socket = (start + k < count)
                        .then(|| u32::try_from(start + k).ok())
                        .flatten();
                }
                filled.insert(slot.to_owned(), start + gems.len());
            }
            if is_main {
                let chosen = main_active_index(g, &gems);
                if let Some(gem) = chosen.and_then(|c| gems.get_mut(c)) {
                    gem.main = true;
                }
            }
            api::SkillGroupView {
                label: g.label.clone(),
                slot: g.slot.as_ref().map(|s| s.as_str().to_owned()),
                enabled: g.enabled,
                main: is_main,
                gems,
            }
        })
        .collect()
}

/// The index (among all its gems) of the active skill a group's
/// `mainActiveSkill` names: the n-th gem that is not a support (a gem
/// the data does not know counts as active); the first when out of range.
fn main_active_index(group: &loom_domain::SocketGroup, gems: &[api::GemView]) -> Option<usize> {
    let actives: Vec<usize> = gems
        .iter()
        .enumerate()
        .filter(|(_, g)| g.support != Some(true))
        .map(|(i, _)| i)
        .collect();
    let n = group
        .main_active_skill
        .as_ref()
        .and_then(|m| usize::try_from(*m.value()).ok())
        .and_then(|m| m.checked_sub(1))
        .unwrap_or(0);
    actives.get(n).or_else(|| actives.first()).copied()
}

fn set_view(id: u32, title: Option<&String>, active: bool) -> api::SetView {
    api::SetView {
        id,
        title: title.cloned(),
        active,
    }
}

fn loadout_views(build: &Build) -> Vec<api::LoadoutView> {
    loadout::loadouts(build)
        .iter()
        .map(|l| api::LoadoutView {
            spec: l.spec.0,
            title: l.title.clone(),
            item_set: l.item_set.map(|s| s.0),
            skill_set: l.skill_set.map(|s| s.0),
            config_set: l.config_set.map(|s| s.0),
            active: loadout::is_active(build, l),
        })
        .collect()
}

/// The names of the active spec's allocated nodes, from the bundle.
fn passive_names(build: &Build, bundle: Option<&Bundle>) -> Vec<api::NodeName> {
    let (Some(spec), Some(tables)) = (
        loom_calc::poe1::collect::active_spec(build),
        bundle.and_then(|b| b.passives.as_ref()),
    ) else {
        return Vec::new();
    };
    let names: HashMap<i64, &str> = tables
        .passives
        .iter()
        .map(|p| (p.node, p.name.as_str()))
        .collect();
    spec.nodes
        .value()
        .iter()
        .filter_map(|n| {
            let name = names.get(&i64::from(n.0))?;
            Some(api::NodeName {
                node: n.0,
                name: (*name).to_owned(),
            })
        })
        .collect()
}

/// Whether a custom modifier line is pending or did not resolve (as
/// [`Build::completeness`] counts them).
fn custom_open(build: &Build) -> bool {
    build.config.custom_lines().any(|l| {
        matches!(
            l.resolution,
            LineResolution::Pending | LineResolution::Unresolved(_)
        )
    })
}

/// The build's ascendancy; PoB writes "no ascendancy" as `None`, which is
/// no name.
fn ascendancy(build: &Build) -> Option<String> {
    build
        .character
        .ascendancy
        .as_ref()
        .map(|a| a.as_str().to_owned())
        .filter(|a| a != "None")
}

/// What the recent list shows of a build (UX-20): class, ascendancy,
/// level, main skill, as [`build_view`] names them.
#[must_use]
pub fn recent_summary(build: &Build, bundle: Option<&Bundle>) -> loom_persistence::recent::Summary {
    let groups = skill_groups(build, bundle);
    loom_persistence::recent::Summary {
        class: build
            .character
            .class
            .as_ref()
            .map(|c| c.as_str().to_owned()),
        ascendancy: ascendancy(build),
        level: build.character.level.as_ref().map(|l| *l.value()),
        skill: main_skill(build, &groups),
    }
}

/// The build's view. `bundle` names the passive nodes, when loaded.
#[must_use]
pub fn build_view(build: &Build, name: String, bundle: Option<&Bundle>) -> api::BuildView {
    let in_use = loom_calc::poe1::collect::items_in_use(build);
    let used = |id: ItemId| in_use.iter().any(|(i, _)| *i == id);
    let mut counts = api::LineCounts {
        modifiers: 0,
        matched: 0,
        adapted: 0,
        warnings: 0,
        excluded: 0,
    };
    let mut items = Vec::new();
    let mut warnings = Vec::new();
    for item in build.items.items() {
        let mut item_warnings = 0;
        for (i, line) in item.body.lines.iter().enumerate() {
            let view = line_view(i, line);
            if let LineKind::Modifier(_) = line.kind {
                counts.modifiers += 1;
            }
            match view.status {
                api::LineStatus::Matched => counts.matched += 1,
                api::LineStatus::Adapted => counts.adapted += 1,
                api::LineStatus::Excluded => counts.excluded += 1,
                api::LineStatus::Warning => {
                    counts.warnings += 1;
                    item_warnings += 1;
                }
                api::LineStatus::Continues | api::LineStatus::Other => {}
            }
            if let Some(code) = view.code {
                warnings.push(api::WarningView {
                    item: item.id.0,
                    item_name: item.body.name.clone(),
                    line: view.index,
                    text: view.text,
                    code,
                    in_use: used(item.id),
                });
            }
        }
        items.push(api::ItemSummary {
            id: item.id.0,
            item_class: bundle.and_then(|b| item_class(b, item)),
            name: item.body.name.clone(),
            base: item.body.base.clone(),
            rarity: item.body.rarity.map(|r| r.name().to_owned()),
            slot: slot_of(build, item.id),
            warnings: item_warnings,
            in_use: used(item.id),
            sockets: socket_groups(item),
            sets: build
                .items
                .sets()
                .filter(|s| s.slots.typed().any(|e| e.item == Some(item.id)))
                .map(|s| s.id.0)
                .collect(),
            tree_socket: in_use.iter().find_map(|(i, place)| match place {
                ItemPlace::TreeSocket(node) if *i == item.id => Some(node.0),
                _ => None,
            }),
            legacy: is_legacy(item),
            requirements: item_requirements(item, bundle),
        });
    }
    let active = loom_calc::poe1::collect::active_item_set(build).map(|s| s.id);
    let item_sets = build
        .items
        .sets()
        .map(|s| set_view(s.id.0, s.title.as_ref(), Some(s.id) == active))
        .collect();
    let skill_set = loadout::active_skill_set(build);
    let groups = skill_groups(build, bundle);
    let config_set = loadout::active_config_set(build);
    let spec = loom_calc::poe1::collect::active_spec(build).map(|s| s.id);
    let skill = main_skill(build, &groups);
    // A build with no file name yet is named from itself (UX-15).
    let name = if name.is_empty() {
        derived_name(build, skill.as_deref())
    } else {
        name
    };
    api::BuildView {
        name,
        game: build.game().name().to_owned(),
        class: build
            .character
            .class
            .as_ref()
            .map(|c| c.as_str().to_owned()),
        ascendancy: ascendancy(build),
        level: build.character.level.as_ref().map(|l| *l.value()),
        main_skill: skill,
        data_version: build.data.as_ref().map(data_version),
        completeness: build.completeness().name().to_owned(),
        in_use_completeness: if warnings.iter().any(|w| w.in_use) || custom_open(build) {
            "partial"
        } else {
            "complete"
        }
        .to_owned(),
        lines: counts,
        items,
        warnings,
        item_sets,
        second_weapon_set: loom_calc::poe1::collect::active_item_set(build).is_some_and(|s| {
            s.attributes
                .iter()
                .any(|a| a.name == SECOND_WEAPON_SET && a.value == "true")
        }),
        skill_sets: build
            .skills
            .sets
            .typed()
            .map(|s| set_view(s.id.0, s.title.as_ref(), Some(s.id) == skill_set))
            .collect(),
        specs: build
            .passives
            .specs
            .typed()
            .map(|s| set_view(s.id.0, s.title.as_ref(), Some(s.id) == spec))
            .collect(),
        config_sets: build
            .config
            .sets
            .typed()
            .map(|s| set_view(s.id.0, s.title.as_ref(), Some(s.id) == config_set))
            .collect(),
        loadouts: loadout_views(build),
        skill_groups: groups,
        passive_names: passive_names(build, bundle),
        config: config_view(build),
        league: match build.league {
            Some(League::Standard) => api::League::Standard,
            Some(League::Challenge) | None => api::League::Challenge,
        },
        tree: None,
    }
}

/// The item's variants' names, in order (its `Variant` lines).
fn variant_names(item: &Item) -> Vec<String> {
    item.body
        .lines
        .iter()
        .filter_map(|l| match &l.kind {
            LineKind::Spec { key, value, .. } if key == "Variant" => Some(value.trim().to_owned()),
            _ => None,
        })
        .collect()
}

/// The item's base's item class in `bundle`, if the base is known.
pub(crate) fn item_class(bundle: &Bundle, item: &Item) -> Option<String> {
    let (name, fallback) = pob_item::item_base(
        &item.body,
        bundle.base_items.iter().map(|b| b.name.as_str()),
    )
    .ok()?;
    let class = |n: &str| {
        bundle
            .base_items
            .iter()
            .find(|b| b.name == n)
            .and_then(|b| b.item_class.clone())
    };
    class(&name).or_else(|| fallback.as_deref().and_then(class))
}

/// Whether the item's selected variant is a legacy version.
fn is_legacy(item: &Item) -> bool {
    let eras = pob_item::variant_eras(&variant_names(item));
    item.body
        .variants
        .selected
        .and_then(|v| usize::try_from(v).ok()?.checked_sub(1))
        .and_then(|i| eras.get(i))
        .is_some_and(|e| *e == pob_item::VariantEra::Legacy)
}

fn era(e: pob_item::VariantEra) -> api::VariantEra {
    match e {
        pob_item::VariantEra::Legacy => api::VariantEra::Legacy,
        pob_item::VariantEra::Current => api::VariantEra::Current,
        pob_item::VariantEra::Choice => api::VariantEra::Choice,
    }
}

/// One custom modifier line's view.
fn custom_line_view(text: &str, resolution: &LineResolution) -> api::CustomLineView {
    let (status, typed) = match resolution {
        LineResolution::Resolved { result, adapted } => (
            if adapted.is_some() {
                api::LineStatus::Adapted
            } else {
                api::LineStatus::Matched
            },
            match result {
                Resolved::Typed(t) => Some(t.kind().to_owned()),
                Resolved::Stats(_) => None,
            },
        ),
        LineResolution::Continues { .. } => (api::LineStatus::Continues, None),
        LineResolution::Excluded(_) => (api::LineStatus::Excluded, None),
        LineResolution::Pending | LineResolution::Unresolved(_) => (api::LineStatus::Warning, None),
    };
    api::CustomLineView {
        text: text.to_owned(),
        status,
        typed,
    }
}

/// The active config set's inputs, each with what it does to the computed
/// fields, and its custom modifier blocks.
fn config_view(build: &Build) -> Option<api::ConfigView> {
    let set = loom_calc::poe1::collect::active_config(build)?;
    Some(api::ConfigView {
        set: set.id.0,
        inputs: set
            .inputs
            .typed()
            .map(|i| api::ConfigInputView {
                name: i.name.clone(),
                value: match &i.value {
                    ConfigValue::Boolean(b) => api::ConfigValueView::Boolean(*b),
                    ConfigValue::Number(n) => api::ConfigValueView::Number(n.value().0),
                    ConfigValue::String(t) => api::ConfigValueView::Text(t.clone()),
                },
                placeholder: i.kind == ConfigInputKind::Placeholder,
                effect: match loom_calc::poe1::collect::input_use(i) {
                    InputUse::Applied => api::InputEffect::Applied,
                    InputUse::NoEffect => api::InputEffect::NoEffect,
                    InputUse::Unmodelled => api::InputEffect::Unmodelled,
                },
            })
            .collect(),
        blocks: set
            .custom_modifiers
            .iter()
            .map(|b| api::CustomBlockView {
                title: b.title.clone(),
                enabled: b.enabled,
                text: b.text.clone(),
                lines: b
                    .lines
                    .iter()
                    .map(|l| custom_line_view(&l.text, &l.resolution))
                    .collect(),
            })
            .collect(),
    })
}

/// Each field of a calc result as `(name, finite value, state)`.
#[must_use]
pub fn fields(result: &CalcResult) -> Vec<(String, Option<f64>, String)> {
    result
        .fields
        .iter()
        .map(|(field, f)| {
            let value = f.value.filter(|v| v.is_finite());
            (field.to_string(), value, f.state.kind().name().to_owned())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_should_skip_prefixes_and_sentence_dots() {
        let n = numbers("{range:0.5}+(10-20.5) to maximum Life.");
        let texts: Vec<&str> = n.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(texts, ["10", "20.5"]);
        assert_eq!(n[0].start, 13);
    }

    #[test]
    fn flavour_lines_should_drop_glyphs_mark_default_text_and_keep_one_gap() {
        let lines = |text: &str| {
            flavour_lines(text)
                .into_iter()
                .map(|l| (l.text, l.default_colour))
                .collect::<Vec<_>>()
        };
        let line = |t: &str, d: bool| (t.to_owned(), d);
        assert_eq!(
            lines("The warrior who\nfears will fall."),
            [
                line("The warrior who", false),
                line("fears will fall.", false)
            ]
        );
        assert_eq!(
            lines("Great works take time.\n\n\n<default>{This item can be transformed}\n"),
            [
                line("Great works take time.", false),
                line("", false),
                line("This item can be transformed", true),
            ]
        );
        assert!(lines("<<HBGAa>><<HBG01>>\n<<HBGAb>>").is_empty());
        assert_eq!(lines("<size:31>{Big} words"), [line("Big words", false)]);
        // An unclosed tag drops the rest of its line; text outside ASCII
        // is kept whole.
        assert_eq!(lines("Kept <unclosed"), [line("Kept", false)]);
        assert_eq!(lines("Kept <<glyphs"), [line("Kept", false)]);
        assert_eq!(
            lines("Çà et là — «déjà»\n<default>{Ünïcode}"),
            [line("Çà et là — «déjà»", false), line("Ünïcode", true)]
        );
    }
}
