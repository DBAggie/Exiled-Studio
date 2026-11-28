//! A gem's quality from a socket of its colour (Phase 3 plan, step 12;
//! P3-12i).
//!
//! **The game** (maintainer, in game, 2026-11-13): since 3.29 any gem fits
//! any socket; a gem in a socket of its own colour gains quality, a white
//! socket gives none.
//!
//! **PoB v2.67.2:**
//! - `Classes/SkillsTab.lua` 1210 to 1245 (`UpdateSocketGroups`): for each
//!   socket group of the active skill set in list order, enabled or not,
//!   with a slot, gem `i` of the group sits in socket `offset + i`, where
//!   `offset` counts every gem of the earlier groups of the same slot
//!   (disabled ones and gems PoB does not know too). A gem PoB knows
//!   (`grantedEffect`, else `gemData.grantedEffect`: the gem's own effect)
//!   matches when the slot's item (`itemsTab.slots[slot].selItemId`) has
//!   `colourAlwaysMatches`, or its socket at that index has the colour of
//!   the effect's `color` (1 `R`, 2 `G`, 3 `B`; any other colour, `W`,
//!   and `A` never match).
//! - `Classes/Item.lua` 658 to 666: the `Sockets:` line, one socket per
//!   letter of `RGBWA`, in order. 1482 to 1490: an item without one gets
//!   its base's `socketLimit` sockets of a colour from its base's
//!   requirements (`socketLimit` is PoB's own directive, not game data).
//!   2588 to 2635: "Has # Sockets" (`SocketCount`), "Has # Abyssal
//!   Sockets", "Has no Sockets", and "Cannot have non-Abyssal sockets"
//!   rebuild the list. 2636: "Gems Socketed always have the Quality bonus
//!   from Socket Colour" (`SocketAlwaysMatches`) sets
//!   `colourAlwaysMatches`.
//! - `Modules/CalcSetup.lua` 1578 (supports) and 1654 (active skills): the
//!   instance's quality is the gem's plus [`BONUS`] when it matches
//!   (`data.misc.MatchingSocketQualityBonus`, `Modules/Data.lua` 297).
//!
//! **The engine** does the same with the game's colour (the granted
//! effect's `attribute`, P3-12i's bundle field, equal to PoB's `color` in
//! `data-parity`). What it cannot know is [`SocketMatch::Unknown`]: the
//! gem keeps its own quality and a skill that reads it has a reason
//! (`unmodelled-skill` `socket-colour-quality`): untyped gem content in a
//! group of the slot (the index is not certain), an item that is not
//! typed, an item without a
//! `Sockets` line (PoB's `socketLimit` sockets), more than one such line,
//! a line that rebuilds the sockets ([`REBUILDS`]; but "Has no Sockets",
//! no sockets, and one "Has # Sockets" line with at least # sockets on the
//! `Sockets` line, its first #, as PoB keeps them; open items, S1), or a gem whose colour
//! the bundle does not carry in a red, green, or blue socket. An item
//! without a `Sockets` line can over-report: on a base without PoB's
//! `socketLimit` (a ring, an amulet) PoB has no sockets, so no match.

use loom_domain::{Build, Item, LineKind, LineResolution, Resolved, SocketGroup, StatValue};

use super::collect::active_item_set;
use super::skill;
use crate::input::CalcData;

/// The quality a matching socket adds (PoB's
/// `data.misc.MatchingSocketQualityBonus`).
pub const BONUS: i64 = 10;

/// The stat of "Gems Socketed always have the Quality bonus from Socket
/// Colour" (PoB's `SocketAlwaysMatches`).
pub const ALWAYS_MATCHES: &str = "local_can_socket_gems_ignoring_colour";

/// Stats whose lines make PoB rebuild an item's sockets (`Item.lua` 2588
/// to 2635): the socket order the engine reads is then not PoB's.
pub const REBUILDS: [&str; 4] = [
    "local_cannot_have_non_abyssal_sockets",
    "local_has_X_abyss_sockets",
    "local_has_X_sockets",
    "local_has_no_sockets",
];

/// Whether a gem gains [`BONUS`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketMatch {
    /// No: not in a slot, no item, no socket at its index, or another
    /// colour.
    No,
    /// Yes.
    Yes,
    /// Not known (module docs); the name says why.
    Unknown(&'static str),
}

impl SocketMatch {
    /// The quality it adds: [`BONUS`] for [`SocketMatch::Yes`], else 0.
    #[must_use]
    pub const fn bonus(self) -> i64 {
        match self {
            Self::Yes => BONUS,
            Self::No | Self::Unknown(_) => 0,
        }
    }
}

/// An item's sockets as PoB reads them, or why they are not known.
enum Sockets {
    /// Every gem matches (`colourAlwaysMatches`).
    Always,
    /// The colours, in order (`R`, `G`, `B`, `W`, `A`).
    Colours(Vec<char>),
    /// No item: nothing matches.
    None,
    /// Not known.
    Unknown(&'static str),
}

/// The stat IDs of an item's lines that are part of it (resolved, not an
/// unselected variant).
fn stats(item: &Item) -> impl Iterator<Item = &str> {
    item.body.lines.iter().flat_map(|l| {
        let stats = match &l.kind {
            LineKind::Modifier(m) => match &m.resolution {
                LineResolution::Resolved {
                    result: Resolved::Stats(stats),
                    ..
                } => stats.as_slice(),
                _ => &[],
            },
            _ => &[],
        };
        stats.iter().map(|s| s.stat.as_str())
    })
}

/// The sockets of the item in `slot` of the active item set (module docs).
fn sockets(build: &Build, slot: &str) -> Sockets {
    let Some(set) = active_item_set(build) else {
        return Sockets::None;
    };
    // The typed slots, as `collect` reads the items in use: untyped content
    // of a set is also PoB's jewel-socket entries (`SocketIdURL`). PoB
    // keeps the last entry of a slot name (`selItemId` set per entry).
    let Some(id) = set
        .slots
        .typed()
        .filter(|e| e.slot.as_str() == slot)
        .last()
        .and_then(|e| e.item)
    else {
        return Sockets::None;
    };
    // PoB writes `itemId="0"` for an empty slot.
    if id.0 == 0 {
        return Sockets::None;
    }
    let Some(item) = build.items.item(id) else {
        return Sockets::Unknown("untyped-item");
    };
    if stats(item).any(|s| s == ALWAYS_MATCHES) {
        return Sockets::Always;
    }
    // "Has no Sockets" removes them all (`Item.lua` 2591 to 2595: PoB checks
    // it first).
    if stats(item).any(|s| s == NO_SOCKETS) {
        return Sockets::None;
    }
    // "Has # Sockets" and "Has # Abyssal Sockets" (open items, S1; 2596 to
    // 2634). PoB reads the count first: it keeps the line's first #
    // sockets in order and fills a missing one with the item's default
    // colour, which the engine does not read. Else, with abyssal sockets,
    // it keeps the line's other sockets in order and appends the abyssal
    // ones, so a line whose abyssal sockets are its last ones, as many as
    // the line says, is kept as it is. Any other case, or another
    // rebuilding line, is not known.
    let count = socket_count(item, SOCKET_COUNT);
    let abyssal = socket_count(item, ABYSSAL_SOCKET_COUNT);
    let other = stats(item)
        .any(|s| REBUILDS.contains(&s) && s != SOCKET_COUNT && s != ABYSSAL_SOCKET_COUNT);
    if other || count == Some(None) || (count.is_none() && abyssal == Some(None)) {
        return Sockets::Unknown("socket-count-line");
    }
    let mut lines = item.body.lines.iter().filter_map(|l| match &l.kind {
        LineKind::Spec { key, value, .. } if key == "Sockets" => Some(value.as_str()),
        _ => None,
    });
    match (lines.next(), lines.next()) {
        (None, _) => Sockets::Unknown("no-sockets-line"),
        (Some(_), Some(_)) => Sockets::Unknown("several-sockets-lines"),
        (Some(line), None) => {
            let mut colours: Vec<char> = line.chars().filter(|c| "RGBWA".contains(*c)).collect();
            match (count.flatten(), abyssal.flatten()) {
                (Some(n), _) if n > colours.len() => Sockets::Unknown("socket-count-line"),
                (Some(n), _) => {
                    colours.truncate(n);
                    Sockets::Colours(colours)
                }
                (None, Some(k)) => {
                    let tail = colours.iter().rev().take_while(|c| **c == 'A').count();
                    let all = colours.iter().filter(|c| **c == 'A').count();
                    if tail == k && all == k {
                        Sockets::Colours(colours)
                    } else {
                        Sockets::Unknown("socket-count-line")
                    }
                }
                (None, None) => Sockets::Colours(colours),
            }
        }
    }
}

/// "Has no Sockets" (PoB's `NoSockets`).
const NO_SOCKETS: &str = "local_has_no_sockets";
/// "Has # Sockets" (PoB's `SocketCount` BASE).
const SOCKET_COUNT: &str = "local_has_X_sockets";
/// "Has # Abyssal Sockets" (PoB's `AbyssalSocketCount` BASE).
const ABYSSAL_SOCKET_COUNT: &str = "local_has_X_abyss_sockets";

/// The count of the item's one line of `stat` ([`SOCKET_COUNT`] or
/// [`ABYSSAL_SOCKET_COUNT`]): `None` without the line, `Some(None)` when it
/// is not one count above 0 (several lines, a range, a hidden value without
/// a clear count).
fn socket_count(item: &Item, stat: &str) -> Option<Option<usize>> {
    let mut values = item.body.lines.iter().flat_map(|l| match &l.kind {
        LineKind::Modifier(m) => match &m.resolution {
            LineResolution::Resolved {
                result: Resolved::Stats(stats),
                ..
            } => stats
                .iter()
                .filter(|s| s.stat.as_str() == stat)
                .map(|s| s.value.clone())
                .collect::<Vec<_>>(),
            _ => Vec::new(),
        },
        _ => Vec::new(),
    });
    let first = values.next()?;
    if values.next().is_some() {
        return Some(None);
    }
    Some(match first {
        StatValue::Exact { min, max } if min == max && min > 0 => usize::try_from(min).ok(),
        // "Has 1 Socket": no number shown, the description's one value.
        StatValue::Hidden { assumed: Some(n) } if n > 0 => usize::try_from(n).ok(),
        _ => None,
    })
}

/// The number of sockets of `colour` (`R`, `G`, `B`, or `W`) of the item
/// in `slot` of the active item set (P6-11go; PoB's
/// `Multiplier:<Colour>SocketIn<slot>`, `Item.lua` 2207 to 2219): 0 with
/// no item; `None` when the sockets are not known, or every colour matches
/// (`colourAlwaysMatches`, whose sockets the engine does not read).
#[must_use]
pub fn colour_count(build: &Build, slot: &str, colour: char) -> Option<u32> {
    match sockets(build, slot) {
        Sockets::None => Some(0),
        Sockets::Colours(c) => u32::try_from(c.iter().filter(|x| **x == colour).count()).ok(),
        Sockets::Always | Sockets::Unknown(_) => None,
    }
}

/// Whether the item in `slot` (a slot's name) of the active item set is
/// equipped and holds no socketed gem (the agent list; PoB's
/// `Multiplier:SocketedGemsIn<slot>` at most 0 with `Condition:Using<slot>`,
/// `CalcSetup.lua` 1206 to 1244): the count is `min(gem sockets, gems)`, the
/// gem sockets the item's red, green, blue, and white ones, the gems the
/// enabled known gems of the enabled groups in the slot that no item
/// grants. With no gem the count is 0 whatever the sockets (PoB fills an
/// item without a `Sockets` line with its base's socket limit, which the
/// engine does not read). `Some(false)` with no item; `None` when the
/// sockets (with gems in the slot) or a gem are not known.
#[must_use]
pub fn no_socketed_gems(build: &Build, data: &dyn CalcData, slot: &str) -> Option<bool> {
    let Some(set) = active_item_set(build) else {
        return Some(false);
    };
    let id = set
        .slots
        .typed()
        .filter(|e| e.slot.as_str() == slot)
        .last()
        .and_then(|e| e.item);
    let Some(id) = id.filter(|id| id.0 != 0) else {
        return Some(false);
    };
    build.items.item(id)?;
    let groups = match skill::active_groups(build) {
        Ok(groups) => groups,
        // No skill set: no gem anywhere.
        Err(skill::SkillProblem::NoSkillSet) => Vec::new(),
        Err(_) => return None,
    };
    let mut gems = 0usize;
    for group in groups {
        if !group.enabled
            || group.slot.as_ref().map(|s| s.as_str()) != Some(slot)
            || skill::attribute(&group.attributes, "source").is_some()
        {
            continue;
        }
        if group.gems.opaque().next().is_some() {
            return None;
        }
        gems += group
            .gems
            .typed()
            .filter(|g| g.enabled && skill::gem_effects(g, data).is_some())
            .count();
    }
    if gems == 0 {
        return Some(true);
    }
    let sockets = match sockets(build, slot) {
        Sockets::Colours(c) => c.iter().filter(|x| "RGBW".contains(**x)).count(),
        Sockets::None => 0,
        Sockets::Always | Sockets::Unknown(_) => return None,
    };
    Some(sockets.min(gems) == 0)
}

/// The socket colour of a granted effect's `attribute` (PoB's `colours`).
const fn colour(attribute: i64) -> Option<char> {
    match attribute {
        1 => Some('R'),
        2 => Some('G'),
        3 => Some('B'),
        _ => None,
    }
}

/// For each of `groups` (the active skill set's, in order) and each of its
/// typed gems, whether the gem gains [`BONUS`] (module docs).
#[must_use]
pub fn socket_matches(
    build: &Build,
    groups: &[&SocketGroup],
    data: &dyn CalcData,
) -> Vec<Vec<SocketMatch>> {
    // Per slot: the gems counted so far, or why the count is not known.
    let mut offsets: Vec<(&str, Result<usize, &'static str>)> = Vec::new();
    let mut out = Vec::with_capacity(groups.len());
    for group in groups {
        let gems: Vec<_> = group.gems.typed().collect();
        let Some(slot) = group.slot.as_ref().map(|s| s.as_str()) else {
            out.push(vec![SocketMatch::No; gems.len()]);
            continue;
        };
        let at = match offsets.iter().position(|(s, _)| *s == slot) {
            Some(n) => n,
            None => {
                offsets.push((slot, Ok(0)));
                offsets.len() - 1
            }
        };
        let offset = offsets[at].1;
        offsets[at].1 = if group.gems.opaque().next().is_some() {
            Err("untyped-gem")
        } else {
            offset.map(|o| o + gems.len())
        };
        let sockets = sockets(build, slot);
        let matches = gems
            .iter()
            .enumerate()
            .map(|(i, gem)| {
                // A gem PoB does not know never matches.
                let Some(effect) = skill::gem_effects(gem, data).and_then(|e| e.first().copied())
                else {
                    return SocketMatch::No;
                };
                match &sockets {
                    Sockets::Always => SocketMatch::Yes,
                    Sockets::None => SocketMatch::No,
                    Sockets::Unknown(why) => SocketMatch::Unknown(why),
                    Sockets::Colours(colours) => {
                        let index = match offset {
                            Ok(o) => o + i,
                            Err(why) => return SocketMatch::Unknown(why),
                        };
                        if group.gems.opaque().next().is_some() {
                            return SocketMatch::Unknown("untyped-gem");
                        }
                        // No socket, or a white or abyssal one: no match
                        // whatever the gem's colour.
                        let Some(&socket) = colours.get(index).filter(|c| "RGB".contains(**c))
                        else {
                            return SocketMatch::No;
                        };
                        let Some(attribute) = effect.attribute else {
                            return SocketMatch::Unknown("no-colour");
                        };
                        if colour(attribute) == Some(socket) {
                            SocketMatch::Yes
                        } else {
                            SocketMatch::No
                        }
                    }
                }
            })
            .collect();
        out.push(matches);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_should_be_pobs() {
        assert_eq!(
            [1, 2, 3, 4, 0].map(colour),
            [Some('R'), Some('G'), Some('B'), None, None]
        );
        assert_eq!(SocketMatch::Yes.bonus(), 10);
        assert_eq!(SocketMatch::No.bonus(), 0);
        assert_eq!(SocketMatch::Unknown("x").bonus(), 0);
    }

    /// The always-matching line acts here only, and every line that
    /// rebuilds the sockets has no effect of its own on a slice field.
    #[test]
    fn socket_lines_should_act_only_here() {
        use super::super::mapping::NO_EFFECT;
        let no_effect = |id: &str| NO_EFFECT.iter().any(|(s, c)| *s == id && *c == "sockets");
        assert!(no_effect(ALWAYS_MATCHES));
        assert!(no_effect("local_has_X_abyss_sockets"));
        let mapping = super::super::mapping::Poe1Mapping;
        for id in REBUILDS.iter().filter(|id| !no_effect(id)) {
            assert_eq!(
                crate::ingest::StatMapping::map(&mapping, &loom_domain::StatId::new(*id)),
                crate::ingest::StatMap::Unmapped,
                "{id}"
            );
        }
    }
}
