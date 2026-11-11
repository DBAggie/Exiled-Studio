//! The mechanic inventory of a build (Phase 3 plan, step 3): what a
//! calculation of the slice fields would have to model, by source.
//!
//! - **Passives:** the stats of every allocated node and chosen mastery
//!   effect of the active spec (bundle rows), and the keystones.
//! - **Items:** every item equipped in the active item set (the active
//!   weapon set's slots only) and every jewel in an allocated socket: the
//!   stat IDs of resolved lines, typed kinds, and the other lines by
//!   resolution class.
//! - **Skills:** the main skill's supports, and the active skills of the
//!   other enabled groups by class (aura, herald, curse, ...).
//! - **Config:** the input names of the active config set.
//!
//! Only game-data IDs, PoB config names, and counts are kept: no item,
//! gem label, or author text (ADR-007).

use std::collections::{BTreeMap, BTreeSet};

use loom_domain::{Build, ItemId, LineKind, LineResolution, PassiveSpec, Resolved, SocketGroup};
use serde::Serialize;

use super::Tables;
use super::main_skill::{active_groups, attribute, gem_effects};

/// The inventory (module docs).
#[derive(Debug, Default, Clone, Serialize)]
pub struct Inventory {
    /// Allocated nodes.
    pub nodes: usize,
    /// Allocated keystones, by passive ID.
    pub keystones: BTreeSet<String>,
    /// Chosen mastery effects.
    pub mastery_effects: usize,
    /// Stat ID to the number of allocated nodes and mastery effects with it.
    pub passive_stats: BTreeMap<String, usize>,
    /// Items read (equipped and socketed jewels).
    pub items: usize,
    /// Stat ID to the number of resolved item lines with it.
    pub item_stats: BTreeMap<String, usize>,
    /// Typed kind to lines.
    pub typed_kinds: BTreeMap<&'static str, usize>,
    /// Modifier lines by resolution class (`resolved`, `unresolved`, ...).
    pub item_lines: BTreeMap<&'static str, usize>,
    /// Lines the import did not recognize.
    pub unrecognized_lines: usize,
    /// The main skill's supports that apply, by granted effect ID.
    pub supports: BTreeSet<String>,
    /// Supports in the main group that do not apply.
    pub unapplied_supports: BTreeSet<String>,
    /// Other enabled groups' active skills: class to granted effect IDs.
    pub other_skills: BTreeMap<&'static str, BTreeSet<String>>,
    /// Gems of other groups the bundle does not have.
    pub unknown_gems: usize,
    /// Input names of the active config set.
    pub config: BTreeSet<String>,
}

impl Inventory {
    /// The number of distinct things to model: stats (passive and item),
    /// typed kinds, supports, other skills, and config inputs. The skill
    /// choice breaks ties with it (plan step 3).
    #[must_use]
    pub fn size(&self) -> usize {
        let stats: BTreeSet<&String> = self
            .passive_stats
            .keys()
            .chain(self.item_stats.keys())
            .collect();
        stats.len()
            + self.typed_kinds.len()
            + self.supports.len()
            + self.other_skills.values().map(BTreeSet::len).sum::<usize>()
            + self.config.len()
    }
}

/// The active spec: `Tree.activeSpec` (default 1), else the first typed
/// one; `None` when it is not typed.
#[must_use]
pub fn active_spec(build: &Build) -> Option<&PassiveSpec> {
    let p = &build.passives;
    let id = p.active.map_or(1, |s| s.0);
    p.specs
        .typed()
        .find(|s| s.id.0 == id)
        .or_else(|| p.specs.typed().next())
}

/// The class of a skill an other group grants, by its types (first match).
fn class_of(types: &[String]) -> &'static str {
    let has = |t: &str| types.iter().any(|x| x == t);
    if has("Aura") {
        "aura"
    } else if has("Herald") {
        "herald"
    } else if has("AppliesCurse") || has("Hex") || has("Mark") {
        "curse"
    } else if has("Warcry") {
        "warcry"
    } else if has("Guard") {
        "guard"
    } else if has("Banner") {
        "banner"
    } else if has("Minion") || has("CreatesMinion") || has("Golem") {
        "minion"
    } else if has("Buff") {
        "buff"
    } else if has("Movement") || has("Travel") || has("Blink") {
        "movement"
    } else if has("Attack") {
        "attack"
    } else if has("Spell") {
        "spell"
    } else {
        "other"
    }
}

/// A modifier line's resolution class.
const fn resolution_class(r: &LineResolution) -> &'static str {
    match r {
        LineResolution::Pending => "pending",
        LineResolution::Excluded(_) => "excluded",
        LineResolution::Resolved { adapted: None, .. } => "resolved",
        LineResolution::Resolved {
            adapted: Some(_), ..
        } => "resolved-adapted",
        LineResolution::Continues { .. } => "continues",
        LineResolution::Unresolved(_) => "unresolved",
    }
}

/// Whether a slot is in use for the item set's weapon set (PoB's
/// `slot.weaponSet`): `... Swap` slots belong to set 2, the other weapon
/// slots to set 1, every other slot to both.
fn slot_in_use(slot: &str, second_set: bool) -> bool {
    if slot.contains("Swap") {
        second_set
    } else if slot.starts_with("Weapon") {
        !second_set
    } else {
        true
    }
}

/// The items to read: the active item set's slots in use, then jewels in
/// allocated sockets of `spec`; each item once.
fn items_in_use(build: &Build, spec: Option<&PassiveSpec>) -> Vec<ItemId> {
    let mut out = Vec::new();
    let items = &build.items;
    let set = items
        .active_set
        .and_then(|id| items.sets().find(|s| s.id == id))
        .or_else(|| items.sets().next());
    if let Some(set) = set {
        let second = attribute(&set.attributes, "useSecondWeaponSet") == Some("true");
        for entry in set.slots.typed() {
            if let Some(id) = entry.item
                && slot_in_use(entry.slot.as_str(), second)
                && !out.contains(&id)
            {
                out.push(id);
            }
        }
    }
    if let Some(spec) = spec {
        let allocated: BTreeSet<u32> = spec.nodes.value().iter().map(|n| n.0).collect();
        for socket in spec.jewel_sockets.typed() {
            if let Some(id) = socket.item
                && allocated.contains(&socket.node.0)
                && !out.contains(&id)
            {
                out.push(id);
            }
        }
    }
    out
}

/// The build's inventory (module docs). `main_group` is the main group's
/// position (1-based), left out of the other skills.
#[must_use]
pub fn inventory(build: &Build, t: &Tables<'_>, main_group: Option<usize>) -> Inventory {
    let mut inv = Inventory::default();
    let spec = active_spec(build);
    if let Some(spec) = spec {
        for node in spec.nodes.value() {
            inv.nodes += 1;
            if let Some(p) = t.passives.get(&i64::from(node.0)) {
                if p.keystone {
                    inv.keystones.insert(p.id.clone());
                }
                for s in &p.stats {
                    *inv.passive_stats.entry(s.stat.clone()).or_default() += 1;
                }
            }
        }
        for m in spec.mastery_effects.value() {
            inv.mastery_effects += 1;
            if let Some(e) = t.masteries.get(&i64::from(m.effect)) {
                for s in &e.stats {
                    *inv.passive_stats.entry(s.stat.clone()).or_default() += 1;
                }
            }
        }
    }
    for id in items_in_use(build, spec) {
        let Some(item) = build.items.item(id) else {
            continue;
        };
        inv.items += 1;
        for line in &item.body.lines {
            match &line.kind {
                LineKind::Unrecognized => inv.unrecognized_lines += 1,
                LineKind::Modifier(m) => {
                    *inv.item_lines
                        .entry(resolution_class(&m.resolution))
                        .or_default() += 1;
                    if let LineResolution::Resolved { result, .. } = &m.resolution {
                        match result {
                            Resolved::Stats(stats) => {
                                for s in stats {
                                    *inv.item_stats
                                        .entry(s.stat.as_str().to_owned())
                                        .or_default() += 1;
                                }
                            }
                            Resolved::Typed(typed) => {
                                *inv.typed_kinds.entry(typed.kind()).or_default() += 1;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    other_skills(build, t, main_group, &mut inv);
    let config = &build.config;
    let set = config
        .active
        .and_then(|id| config.sets.typed().find(|s| s.id == id))
        .or_else(|| config.sets.typed().next());
    if let Some(set) = set {
        inv.config = set.inputs.typed().map(|i| i.name.clone()).collect();
    }
    inv
}

/// The active skills of enabled groups other than the main one, by class.
fn other_skills(build: &Build, t: &Tables<'_>, main_group: Option<usize>, inv: &mut Inventory) {
    let Ok(groups) = active_groups(build) else {
        return;
    };
    let second = build
        .items
        .active_set
        .and_then(|id| build.items.sets().find(|s| s.id == id))
        .is_some_and(|s| attribute(&s.attributes, "useSecondWeaponSet") == Some("true"));
    for (n, group) in groups.iter().enumerate() {
        if Some(n + 1) == main_group || !group_enabled(group, second) {
            continue;
        }
        for gem in group.gems.typed().filter(|g| g.enabled) {
            let Some(effects) = gem_effects(gem, t) else {
                inv.unknown_gems += 1;
                continue;
            };
            for e in effects.into_iter().filter(|e| !e.support) {
                let types = e
                    .active_skill
                    .as_deref()
                    .and_then(|a| t.active.get(a))
                    .map_or(&[][..], |a| a.types.as_slice());
                inv.other_skills
                    .entry(class_of(types))
                    .or_default()
                    .insert(e.id.clone());
            }
        }
    }
}

/// PoB: a group counts when it is enabled and its slot belongs to the
/// active weapon set.
fn group_enabled(group: &SocketGroup, second_set: bool) -> bool {
    group.enabled
        && group
            .slot
            .as_ref()
            .is_none_or(|s| slot_in_use(s.as_str(), second_set))
}
