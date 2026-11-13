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
//! a line that rebuilds the sockets ([`REBUILDS`]), or a gem whose colour
//! the bundle does not carry.

use loom_domain::{Build, Item, LineKind, LineResolution, Resolved, SocketGroup};

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
    if stats(item).any(|s| REBUILDS.contains(&s)) {
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
            Sockets::Colours(line.chars().filter(|c| "RGBWA".contains(*c)).collect())
        }
    }
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
                        let Some(attribute) = effect.attribute else {
                            return SocketMatch::Unknown("no-colour");
                        };
                        match (colours.get(index), colour(attribute)) {
                            (Some(socket), Some(gem)) if *socket == gem => SocketMatch::Yes,
                            _ => SocketMatch::No,
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
