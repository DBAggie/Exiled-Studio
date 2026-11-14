//! A build's stages: PoB's loadouts. Each passive spec is a stage; the
//! item, skill, and config sets linked to it switch with it, so one choice
//! ("Early maps", "Endgame") sets the gear, the gems, the tree, and the
//! configuration together (maintainer, 2026-11-14).
//!
//! PoB links sets by the `{…}` at the end of their titles: a spec titled
//! `Early Maps {5}` is stage `5`, and a set titled `Early Campaign {1,2,3,4}`
//! belongs to stages 1 to 4. A spec without a link matches sets by their
//! title (colour codes and links left out). A kind with one set only uses
//! it for every stage. A stage a kind has no set for leaves that kind as it
//! is when chosen.

use loom_domain::{Build, ConfigSetId, ItemSetId, SkillSetId, SpecId};

/// One stage: a spec and the sets linked to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loadout {
    /// The passive spec.
    pub spec: SpecId,
    /// Its title, without colour codes and links.
    pub title: String,
    /// The linked item set.
    pub item_set: Option<ItemSetId>,
    /// The linked skill set.
    pub skill_set: Option<SkillSetId>,
    /// The linked config set.
    pub config_set: Option<ConfigSetId>,
}

/// The links at the end of a title: `Early Maps {5}` → `["5"]`.
fn links(title: &str) -> Option<Vec<&str>> {
    let title = title.trim_end();
    let body = title.strip_suffix('}')?;
    let open = body.rfind('{')?;
    let ids: Vec<&str> = body[open + 1..]
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    (!ids.is_empty()).then_some(ids)
}

/// A title without PoB's colour codes (`^4`, `^xRRGGBB`) and links.
#[must_use]
pub fn clean_title(title: &str) -> String {
    let mut out = String::new();
    let mut chars = title.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '^' {
            match chars.peek() {
                Some(d) if d.is_ascii_digit() => {
                    chars.next();
                    continue;
                }
                Some('x') => {
                    let rest: String = chars.clone().skip(1).take(6).collect();
                    if rest.len() == 6 && rest.chars().all(|h| h.is_ascii_hexdigit()) {
                        for _ in 0..7 {
                            chars.next();
                        }
                        continue;
                    }
                }
                _ => {}
            }
        }
        out.push(c);
    }
    let trimmed = out.trim_end();
    let without = match (trimmed.strip_suffix('}'), trimmed.rfind('{')) {
        (Some(_), Some(open)) => &trimmed[..open],
        _ => trimmed,
    };
    without.trim().to_owned()
}

/// The set of one kind linked to a spec (module docs).
fn linked<T: Copy>(spec_title: &str, sets: &[(T, Option<&str>)]) -> Option<T> {
    let found = match links(spec_title).and_then(|l| l.first().copied()) {
        Some(id) => sets
            .iter()
            .find(|(_, title)| title.and_then(links).is_some_and(|l| l.contains(&id))),
        None => {
            let want = clean_title(spec_title);
            sets.iter()
                .find(|(_, title)| title.is_some_and(|t| clean_title(t) == want))
        }
    };
    match (found, sets) {
        (Some((id, _)), _) => Some(*id),
        (None, [(only, _)]) => Some(*only),
        (None, _) => None,
    }
}

/// The build's stages, in spec order.
#[must_use]
pub fn loadouts(build: &Build) -> Vec<Loadout> {
    let items: Vec<_> = build
        .items
        .sets()
        .map(|s| (s.id, s.title.as_deref()))
        .collect();
    let skills: Vec<_> = build
        .skills
        .sets
        .typed()
        .map(|s| (s.id, s.title.as_deref()))
        .collect();
    let configs: Vec<_> = build
        .config
        .sets
        .typed()
        .map(|s| (s.id, s.title.as_deref()))
        .collect();
    build
        .passives
        .specs
        .typed()
        .map(|spec| {
            let title = spec.title.as_deref().unwrap_or_default();
            let clean = clean_title(title);
            Loadout {
                spec: spec.id,
                title: if clean.is_empty() {
                    format!("Tree {}", spec.id.0)
                } else {
                    clean
                },
                item_set: linked(title, &items),
                skill_set: linked(title, &skills),
                config_set: linked(title, &configs),
            }
        })
        .collect()
}

/// The active skill set's ID (the first when none is named).
#[must_use]
pub fn active_skill_set(build: &Build) -> Option<SkillSetId> {
    build
        .skills
        .sets
        .typed()
        .find(|s| Some(s.id) == build.skills.active)
        .or_else(|| build.skills.sets.typed().next())
        .map(|s| s.id)
}

/// The active config set's ID (the first when none is named).
#[must_use]
pub fn active_config_set(build: &Build) -> Option<ConfigSetId> {
    build
        .config
        .sets
        .typed()
        .find(|s| Some(s.id) == build.config.active)
        .or_else(|| build.config.sets.typed().next())
        .map(|s| s.id)
}

/// Whether the build uses the stage: its spec and every linked set.
#[must_use]
pub fn is_active(build: &Build, l: &Loadout) -> bool {
    let spec = loom_calc::poe1::collect::active_spec(build).map(|s| s.id);
    let items = loom_calc::poe1::collect::active_item_set(build).map(|s| s.id);
    spec == Some(l.spec)
        && l.item_set.is_none_or(|s| items == Some(s))
        && l.skill_set
            .is_none_or(|s| active_skill_set(build) == Some(s))
        && l.config_set
            .is_none_or(|s| active_config_set(build) == Some(s))
}

/// Makes the stage active: its spec and its linked sets.
pub fn apply(build: &mut Build, l: &Loadout) {
    build.passives.active = Some(l.spec);
    if let Some(s) = l.item_set {
        build.items.active_set = Some(s);
    }
    if let Some(s) = l.skill_set {
        build.skills.active = Some(s);
    }
    if let Some(s) = l.config_set {
        build.config.active = Some(s);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles_should_lose_colour_codes_and_links() {
        assert_eq!(clean_title("^4Endgame {3,4,5}"), "Endgame");
        assert_eq!(clean_title("^xFF0000Red Maps{6}"), "Red Maps");
        assert_eq!(clean_title("Default"), "Default");
        assert_eq!(clean_title("2^3 {1}"), "2");
        assert_eq!(links("Cluster Endgame {7,8}"), Some(vec!["7", "8"]));
        assert_eq!(links("Default"), None);
    }

    #[test]
    fn sets_should_link_by_id_then_title_then_a_single_set() {
        let sets = [
            (1, Some("Early Campaign {1,2,3,4}")),
            (2, Some("Early Maps {5}")),
            (3, Some("Endgame")),
        ];
        assert_eq!(linked("12-27 Sunder {2}", &sets), Some(1));
        assert_eq!(linked("Early Maps {5}", &sets), Some(2));
        assert_eq!(linked("^2Endgame", &sets), Some(3));
        assert_eq!(linked("Other {9}", &sets), None);
        assert_eq!(linked("Other {9}", &[(4, Some("Default"))]), Some(4));
    }
}
