//! The passive tree's sprites (ADR-005 Amendment 2; passive tree plan,
//! T-a): GGG's sprite sheets from the passive tree export, which the tree
//! screen draws when a bundle has them (our own shapes otherwise).
//!
//! | Path | Content |
//! |---|---|
//! | `data/passive_tree_sprites.json` | the [`TreeSprites`] index: the sheets' sprites by category, each node's art, with the export they came from |
//! | `trees/assets/*` | the export's sheets, unmodified, one zoom level |
//!
//! Optional, listed when present. The sheets are copied byte for byte from
//! the export (`assets/`) at the commit pinned in `tools/tree/manifest.json`
//! by `loom-parity-cli tree-layout`; they are never committed to the
//! repository and ship only inside bundles. The index is ours: the export's
//! `sprites` entries for the one zoom level copied, and each node's icons
//! from its `nodes` (the export's node numbers are the game's).
//!
//! A sprite drawn at the index's [`TreeSprites::zoom`] covers its pixel size
//! divided by the zoom in tree units (the export's convention: a sheet of
//! zoom `z` is drawn with one tree unit as `z` pixels).

use serde::{Deserialize, Serialize};

use super::tree_layout::TreeExport;

/// The index file.
pub const TREE_SPRITES: &str = "data/passive_tree_sprites.json";

/// The folder of the sheets.
pub const SPRITES_DIR: &str = "trees/assets/";

/// The sprites of one export category (`normalActive`, `frame`, ...) at the
/// index's zoom.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpriteCategory {
    /// The export's category name.
    pub name: String,
    /// Its sheet's file name in [`SPRITES_DIR`].
    pub file: String,
    /// The sheet's width in pixels.
    pub width: u32,
    /// The sheet's height in pixels.
    pub height: u32,
    /// Its sprites, by name.
    pub sprites: Vec<Sprite>,
}

/// One sprite on a sheet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sprite {
    /// Its name (an icon path, or a frame's name).
    pub name: String,
    /// Left, in pixels.
    pub x: u32,
    /// Top, in pixels.
    pub y: u32,
    /// Width, in pixels.
    pub w: u32,
    /// Height, in pixels.
    pub h: u32,
}

/// A node's art, as the export names it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeArt {
    /// The node.
    pub node: u32,
    /// Its icon (a passive's; a mastery's icon on the tree).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// A mastery's icon while its group has no allocated node.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inactive_icon: Option<String>,
    /// A mastery's icon once it is allocated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_icon: Option<String>,
}

/// The index.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeSprites {
    /// The export's zoom level the sheets are for.
    pub zoom: f64,
    /// The categories, by name.
    pub categories: Vec<SpriteCategory>,
    /// The nodes' art, by node.
    pub nodes: Vec<NodeArt>,
    /// Where they came from (the sheets read are in its files).
    pub source: TreeExport,
}

impl TreeSprites {
    /// A category by name.
    #[must_use]
    pub fn category(&self, name: &str) -> Option<&SpriteCategory> {
        self.categories.iter().find(|c| c.name == name)
    }

    /// The bundle paths of the sheets, each once, in order.
    #[must_use]
    pub fn sheet_paths(&self) -> Vec<String> {
        let mut out: Vec<String> = self
            .categories
            .iter()
            .map(|c| format!("{SPRITES_DIR}{}", c.file))
            .collect();
        out.sort();
        out.dedup();
        out
    }

    /// What is wrong with the index, if anything: a zoom that is not
    /// positive, a sheet name that is not a plain file name, a sprite
    /// outside its sheet, a node icon no category has, or rows out of
    /// order.
    #[must_use]
    pub fn problems(&self) -> Vec<String> {
        let mut out = Vec::new();
        if !(self.zoom.is_finite() && self.zoom > 0.0) {
            out.push(format!("zoom {}", self.zoom));
        }
        if !ascending(self.categories.iter().map(|c| c.name.as_str())) {
            out.push("categories are not in name order, each once".to_owned());
        }
        for c in &self.categories {
            if c.file.is_empty() || c.file.contains(['/', '\\']) || c.file.starts_with('.') {
                out.push(format!("{}: file {:?}", c.name, c.file));
            }
            if !ascending(c.sprites.iter().map(|s| s.name.as_str())) {
                out.push(format!(
                    "{}: sprites are not in name order, each once",
                    c.name
                ));
            }
            for s in &c.sprites {
                if s.w == 0
                    || s.h == 0
                    || u64::from(s.x) + u64::from(s.w) > u64::from(c.width)
                    || u64::from(s.y) + u64::from(s.h) > u64::from(c.height)
                {
                    out.push(format!("{}: {} is outside the sheet", c.name, s.name));
                }
            }
        }
        if !ascending(self.nodes.iter().map(|n| n.node)) {
            out.push("nodes are not in order, each once".to_owned());
        }
        let known = |name: &str| {
            self.categories.iter().any(|c| {
                c.sprites
                    .binary_search_by(|s| s.name.as_str().cmp(name))
                    .is_ok()
            })
        };
        for n in &self.nodes {
            for icon in [&n.icon, &n.inactive_icon, &n.active_icon]
                .into_iter()
                .flatten()
            {
                if !known(icon) {
                    out.push(format!("node {}: no sprite {icon}", n.node));
                }
            }
        }
        out
    }
}

/// Whether `items` is strictly ascending (sorted, each once).
fn ascending<T: PartialOrd>(items: impl Iterator<Item = T>) -> bool {
    let mut last: Option<T> = None;
    for item in items {
        if last.as_ref().is_some_and(|l| *l >= item) {
            return false;
        }
        last = Some(item);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sprites() -> TreeSprites {
        TreeSprites {
            zoom: 0.5,
            categories: vec![SpriteCategory {
                name: "normalActive".to_owned(),
                file: "skills-4.jpg".to_owned(),
                width: 100,
                height: 50,
                sprites: vec![Sprite {
                    name: "Art/a.png".to_owned(),
                    x: 50,
                    y: 0,
                    w: 50,
                    h: 50,
                }],
            }],
            nodes: vec![NodeArt {
                node: 7,
                icon: Some("Art/a.png".to_owned()),
                inactive_icon: None,
                active_icon: None,
            }],
            source: TreeExport {
                repository: "r".to_owned(),
                tag: "t".to_owned(),
                commit: "c".to_owned(),
                files: Vec::new(),
                angle_rule: String::new(),
            },
        }
    }

    #[test]
    fn a_sound_index_should_have_no_problems() {
        let s = sprites();
        assert!(s.problems().is_empty(), "{:?}", s.problems());
        assert_eq!(s.sheet_paths(), vec!["trees/assets/skills-4.jpg"]);
        assert!(s.category("normalActive").is_some());
    }

    #[test]
    fn problems_should_name_a_sprite_outside_its_sheet_an_unknown_icon_and_a_bad_file() {
        let mut s = sprites();
        for c in &mut s.categories {
            c.file = "../x.jpg".to_owned();
            for sprite in &mut c.sprites {
                sprite.x = 51;
            }
        }
        for n in &mut s.nodes {
            n.icon = Some("Art/b.png".to_owned());
        }
        let p = s.problems();
        assert_eq!(p.len(), 3, "{p:?}");
    }
}
