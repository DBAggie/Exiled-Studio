//! The gems the game data offers: the catalog a gem picker lists, and a new
//! gem made from one of its entries (as PoB writes a gem: its gem item, its
//! skill, and its name).

use std::collections::BTreeSet;

use loom_calc::{CalcData, GemEffect};
use loom_data::bundle::Bundle;
use loom_domain::{Gem, GemId, SkillId, SocketGroup, Sourced};
use loom_serialization::dto::app::v1 as api;

/// The level a new gem takes: 20, or the gem's highest level when it has
/// fewer (an Awakened gem's 5, Enlighten's 10).
const NEW_GEM_LEVEL: u32 = 20;

/// Every gem effect a skill gem of the data has, by name; for a support,
/// whether it fits `group` when one is given.
#[must_use]
pub fn catalog(bundle: &Bundle, group: Option<&SocketGroup>) -> Vec<api::GemCatalogEntry> {
    let Some(skills) = bundle.skills.as_ref() else {
        return Vec::new();
    };
    let tag_names = |tags: &[String]| -> Vec<String> {
        tags.iter()
            .filter_map(|id| skills.gem_tags.iter().find(|t| &t.id == id))
            .map(|t| t.name.clone())
            .filter(|n| !n.is_empty())
            .collect()
    };
    let mut seen = BTreeSet::new();
    // A gem's effects by name: a later effect named as an earlier one of the
    // same gem (a Vaal gem's leftover `AltX` and `AltY`) is not a gem a
    // player can choose, so only the first is listed.
    let mut named = BTreeSet::new();
    let mut out: Vec<api::GemCatalogEntry> = skills
        .gems
        .iter()
        .flat_map(|g| g.effects.iter().map(move |e| (g, e)))
        .filter(|(_, id)| seen.insert(id.as_str()))
        .filter_map(|(gem, id)| {
            let effect = bundle.gem_effect(id)?;
            let granted = bundle.granted_effect(effect.granted_effect.as_deref()?)?;
            let name = gem_name(bundle, &gem.base_item, effect)?;
            if !named.insert((gem.base_item.as_str(), name.clone())) {
                return None;
            }
            Some(api::GemCatalogEntry {
                id: effect.id.clone(),
                name,
                support: granted.support,
                colour: granted
                    .attribute
                    .and_then(crate::view::colour)
                    .map(str::to_owned),
                tags: tag_names(&effect.tags),
                fits: group
                    .filter(|_| granted.support)
                    .map(|g| loom_calc::poe1::skill::fits_group(granted, g, bundle)),
            })
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));
    out
}

/// A gem effect's name as players read it: its own, else its gem's base
/// item's (most effects leave the name to the gem item); `None` for a gem
/// players cannot have: an unreleased placeholder (`[DNT]`, "do not
/// translate"), a Path of Exile Royale gem (PoB's `bases.lua` skips base
/// items whose ID contains `Royale`, as the extractor's cross-check
/// records), or no name.
fn gem_name(bundle: &Bundle, base_item: &str, effect: &GemEffect) -> Option<String> {
    if base_item.contains("Royale") {
        return None;
    }
    let name = if effect.name.is_empty() {
        bundle
            .base_items
            .iter()
            .find(|b| b.id == base_item)
            .map(|b| b.name.clone())?
    } else {
        effect.name.clone()
    };
    (!name.is_empty() && !name.starts_with("[DNT]")).then_some(name)
}

/// A new gem of the gem effect `id`, at the default level and quality 0;
/// `None` when no skill gem of the data has that effect.
#[must_use]
pub fn new_gem(bundle: &Bundle, id: &str) -> Option<Gem> {
    let skills = bundle.skills.as_ref()?;
    let gem = skills
        .gems
        .iter()
        .find(|g| g.effects.iter().any(|e| e == id))?;
    let effect = bundle.gem_effect(id)?;
    let granted = effect.granted_effect.as_deref()?;
    let top = bundle
        .granted_effect(granted)?
        .levels
        .iter()
        .map(|l| l.level)
        .max()
        .and_then(|l| u32::try_from(l).ok())
        .unwrap_or(NEW_GEM_LEVEL);
    Some(Gem {
        gem: Some(GemId::new(gem.base_item.clone())),
        skill: Some(SkillId::new(granted)),
        name: Some(gem_name(bundle, &gem.base_item, effect)?),
        level: Sourced::new(top.min(NEW_GEM_LEVEL)),
        quality: Sourced::new(0),
        enabled: true,
        attributes: Vec::new(),
    })
}
