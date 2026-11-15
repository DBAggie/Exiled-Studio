//! The passive tree's layout constants (ADR-005 Amendment 2; passive tree
//! plan, T-b): what places a tree node on screen, which the game files our
//! extractor reads do not give.
//!
//! | Path | Content |
//! |---|---|
//! | `data/passive_tree_layout.json` | the [`TreeLayout`]: each orbit's radius and its slots' angles, with the export they came from |
//!
//! Optional, listed when present: a bundle without it has a tree that
//! connects and calculates but is not drawn. A node's position is its
//! group's centre plus its orbit's radius in the direction of its slot's
//! angle, clockwise from up: `(x + r sin θ, y − r cos θ)` ([`TreeLayout::offset`]).
//!
//! The constants come from GGG's passive tree export
//! (`grindinggear/skilltree-export`) at the commit pinned in
//! `tools/tree/manifest.json`: `orbitRadii` and `skillsPerOrbit` from its
//! `data.json`, the 16-slot orbits' angles from its README's table (3.17.0
//! "Notable Changes"). The 40-slot orbit's angles are not documented by
//! GGG; they follow the documented table's rule (the multiples of 360°
//! divided by the slots less four, and the four diagonals), recorded as
//! the angle rule in [`TreeExport::angle_rule`] and cross-checked against
//! PoB (`tree-layout`'s evidence, `docs/tree/layout.md`).

use serde::{Deserialize, Serialize};

/// The layout file.
pub const TREE_LAYOUT: &str = "data/passive_tree_layout.json";

/// Where the constants came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeExport {
    /// The export's repository.
    pub repository: String,
    /// The tag read.
    pub tag: String,
    /// The commit read (the pinned one).
    pub commit: String,
    /// The files read, with their SHA-256.
    pub files: Vec<ExportFile>,
    /// How the angles not in the export's documentation were made.
    pub angle_rule: String,
}

/// A file of the export that was read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportFile {
    /// Its path in the export.
    pub path: String,
    /// SHA-256 of its bytes, lowercase hex.
    pub sha256: String,
}

/// The tree's layout constants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeLayout {
    /// Each orbit's radius, from the group's centre.
    pub orbit_radii: Vec<f64>,
    /// Each orbit's slot angles in degrees, clockwise from up, one per
    /// slot.
    pub orbit_angles: Vec<Vec<f64>>,
    /// Where they came from.
    pub source: TreeExport,
}

impl TreeLayout {
    /// The offset of slot `index` on `orbit` from its group's centre;
    /// `None` when the layout has no such slot.
    #[must_use]
    pub fn offset(&self, orbit: usize, index: usize) -> Option<(f64, f64)> {
        let r = *self.orbit_radii.get(orbit)?;
        let angle = self.orbit_angles.get(orbit)?.get(index)?.to_radians();
        Some((r * angle.sin(), -r * angle.cos()))
    }

    /// Whether the layout fits the tree's slots per orbit: one radius and
    /// one angle per slot for every orbit, each angle in `[0, 360)`.
    #[must_use]
    pub fn fits(&self, orbit_slots: &[i64]) -> bool {
        self.orbit_radii.len() == orbit_slots.len()
            && self.orbit_angles.len() == orbit_slots.len()
            && self
                .orbit_angles
                .iter()
                .zip(orbit_slots)
                .all(|(angles, slots)| {
                    usize::try_from(*slots).is_ok_and(|s| s == angles.len())
                        && angles.iter().all(|a| (0.0..360.0).contains(a))
                })
    }
}

/// The angles of an orbit with `slots` slots: evenly spaced, except the
/// 16- and 40-slot orbits, which take the multiples of `360 / (slots − 4)`
/// and the four diagonals (45°, 135°, 225°, 315°), sorted. For 16 slots
/// this is the export's documented table; for 40 it is the same rule.
#[must_use]
pub fn orbit_angles(slots: usize) -> Vec<f64> {
    if slots == 16 || slots == 40 {
        let step = 360.0 / (slots - 4) as f64;
        let mut out: Vec<f64> = (0..slots - 4).map(|k| k as f64 * step).collect();
        out.extend([45.0, 135.0, 225.0, 315.0]);
        out.sort_by(f64::total_cmp);
        return out;
    }
    (0..slots)
        .map(|k| k as f64 * 360.0 / slots.max(1) as f64)
        .collect()
}

/// The rule [`orbit_angles`] follows, as recorded in [`TreeExport::angle_rule`].
pub const ANGLE_RULE: &str = "16- and 40-slot orbits: the multiples of 360/(slots-4) and the four diagonals, sorted (16: the export README's 3.17.0 table; 40: the same rule); other orbits evenly spaced";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_16_slot_angles_should_be_the_exports_documented_table() {
        let table = [
            0.0, 30.0, 45.0, 60.0, 90.0, 120.0, 135.0, 150.0, 180.0, 210.0, 225.0, 240.0, 270.0,
            300.0, 315.0, 330.0,
        ];
        assert_eq!(orbit_angles(16), table);
    }

    #[test]
    fn the_40_slot_angles_should_be_every_ten_degrees_and_the_diagonals() {
        let a = orbit_angles(40);
        assert_eq!(a.len(), 40);
        assert_eq!(
            a.get(..7),
            Some(&[0.0, 10.0, 20.0, 30.0, 40.0, 45.0, 50.0][..])
        );
        assert_eq!(a.last(), Some(&350.0));
    }

    #[test]
    fn even_orbits_and_offsets_should_place_clockwise_from_up() {
        assert_eq!(orbit_angles(6), [0.0, 60.0, 120.0, 180.0, 240.0, 300.0]);
        let layout = TreeLayout {
            orbit_radii: vec![0.0, 82.0],
            orbit_angles: vec![orbit_angles(1), orbit_angles(6)],
            source: TreeExport {
                repository: String::new(),
                tag: String::new(),
                commit: String::new(),
                files: Vec::new(),
                angle_rule: ANGLE_RULE.to_owned(),
            },
        };
        assert!(layout.fits(&[1, 6]));
        assert!(!layout.fits(&[1, 16]));
        let (x, y) = layout.offset(1, 0).unwrap_or_default();
        assert!(
            x.abs() < 1e-9 && (y + 82.0).abs() < 1e-9,
            "slot 0 is straight up"
        );
        let (x, y) = layout.offset(1, 3).unwrap_or_default();
        assert!(
            x.abs() < 1e-9 && (y - 82.0).abs() < 1e-9,
            "slot 3 is straight down"
        );
        assert_eq!(layout.offset(2, 0), None);
    }
}
