//! Tags of an item type (Spike B, PR B4a), from the extractor's
//! `item_types/<lowercase path>.it.json` files (PR B4x, Spike A report
//! section 19).
//!
//! The tags of a path are its parent's tags (the `extends` chain up to
//! `nothing`, parent first) with the file's `Base` block entries applied in
//! file order: `tag` appends, `remove_tag` removes the first occurrence
//! (PoB's `table.remove`). A base's tags are the tags of its
//! `BaseItemTypes.InheritsFrom` path plus its `TagsKeys`.
//!
//! A path whose file (or a parent's) is missing or unreadable, or whose chain
//! loops or is deeper than [`MAX_DEPTH`], has unknown tags: the caller counts
//! it and skips the spawn-tag tie-break, the run does not stop.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::extract::Read;

/// Deepest `extends` chain followed (observed: 4).
pub const MAX_DEPTH: usize = 32;

#[derive(Deserialize)]
struct ItemTypeFile {
    extends: String,
    #[serde(default)]
    blocks: Vec<Block>,
}

#[derive(Deserialize)]
struct Block {
    name: String,
    #[serde(default)]
    entries: Vec<Entry>,
}

#[derive(Deserialize)]
struct Entry {
    key: String,
    #[serde(default)]
    value: Option<serde_json::Value>,
}

/// Tags of item-type paths, read once per path.
#[derive(Default)]
pub struct ItemTags {
    cache: BTreeMap<String, Option<Vec<String>>>,
    sockets: BTreeMap<String, Option<String>>,
}

/// The game's item level for a socket count that never rolls.
const NEVER: u32 = 9999;

/// The socket counts a `socket_info` value lets roll, with the item level
/// each needs (`count:item level:weight` entries; weight above 0 and item
/// level below [`NEVER`]), by count; `None` when an entry does not read.
pub fn socket_levels(info: &str) -> Option<Vec<(u32, u32)>> {
    let mut out: BTreeMap<u32, u32> = BTreeMap::new();
    for entry in info.split_whitespace() {
        let mut parts = entry.split(':').map(str::parse::<u32>);
        let (Some(Ok(count)), Some(Ok(level)), Some(Ok(weight)), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return None;
        };
        if weight > 0 && level < NEVER {
            let at = out.entry(count).or_insert(level);
            *at = (*at).min(level);
        }
    }
    Some(out.into_iter().collect())
}

impl ItemTags {
    /// Tags of `path` (`Metadata/Items/...`, any case); `None` when unknown
    /// (see the module docs).
    pub fn tags(&mut self, read: Read<'_>, path: &str) -> Option<Vec<String>> {
        self.walk(read, &path.to_lowercase(), 0)
    }

    /// The `socket_info` of `path`'s `Sockets` block, from the nearest file
    /// of its chain that sets it (B-2 item evidence, E9); `None` when no
    /// file does, or the chain does not read.
    pub fn socket_info(&mut self, read: Read<'_>, path: &str) -> Option<String> {
        self.socket_walk(read, &path.to_lowercase(), 0)
    }

    fn socket_walk(&mut self, read: Read<'_>, path: &str, depth: usize) -> Option<String> {
        if let Some(known) = self.sockets.get(path) {
            return known.clone();
        }
        self.sockets.insert(path.to_owned(), None);
        let info = if depth > MAX_DEPTH {
            None
        } else {
            read(&format!("item_types/{path}.it.json"))
                .ok()
                .and_then(|bytes| serde_json::from_slice::<ItemTypeFile>(&bytes).ok())
                .and_then(|file| {
                    let own = file
                        .blocks
                        .iter()
                        .filter(|b| b.name == "Sockets")
                        .flat_map(|b| &b.entries)
                        .filter(|e| e.key == "socket_info")
                        .filter_map(|e| e.value.as_ref()?.as_str().map(str::to_owned))
                        .next_back();
                    own.or_else(|| {
                        (file.extends != "nothing")
                            .then(|| {
                                self.socket_walk(read, &file.extends.to_lowercase(), depth + 1)
                            })
                            .flatten()
                    })
                })
        };
        self.sockets.insert(path.to_owned(), info.clone());
        info
    }

    fn walk(&mut self, read: Read<'_>, path: &str, depth: usize) -> Option<Vec<String>> {
        if let Some(known) = self.cache.get(path) {
            return known.clone();
        }
        // Mark the path as in progress: a loop back to it reads `None`.
        self.cache.insert(path.to_owned(), None);
        let tags = if depth > MAX_DEPTH {
            None
        } else {
            self.read_one(read, path, depth)
        };
        self.cache.insert(path.to_owned(), tags.clone());
        tags
    }

    fn read_one(&mut self, read: Read<'_>, path: &str, depth: usize) -> Option<Vec<String>> {
        let bytes = read(&format!("item_types/{path}.it.json")).ok()?;
        let file: ItemTypeFile = serde_json::from_slice(&bytes).ok()?;
        let mut tags = if file.extends == "nothing" {
            Vec::new()
        } else {
            self.walk(read, &file.extends.to_lowercase(), depth + 1)?
        };
        for entry in file
            .blocks
            .iter()
            .filter(|b| b.name == "Base")
            .flat_map(|b| &b.entries)
        {
            let Some(value) = entry.value.as_ref().and_then(serde_json::Value::as_str) else {
                continue;
            };
            match entry.key.as_str() {
                "tag" => tags.push(value.to_owned()),
                "remove_tag" => {
                    if let Some(at) = tags.iter().position(|t| t == value) {
                        tags.remove(at);
                    }
                }
                _ => {}
            }
        }
        Some(tags)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn socket_levels_should_keep_the_counts_that_roll() {
        // The game's body armour and shield values (card-data.md).
        assert_eq!(
            socket_levels("1:1:50 2:1:120 3:2:100 4:25:30 5:35:5 6:50:1"),
            Some(vec![(1, 1), (2, 1), (3, 2), (4, 25), (5, 35), (6, 50)])
        );
        assert_eq!(
            socket_levels("1:1:50 2:1:120 3:2:100 4:9999:30 5:9999:20 6:9999:5"),
            Some(vec![(1, 1), (2, 1), (3, 2)])
        );
        assert_eq!(socket_levels("0:1:1 1:9999:100"), Some(vec![(0, 1)]));
        assert_eq!(socket_levels("1:1"), None);
    }

    #[test]
    fn socket_info_should_come_from_the_nearest_file_that_sets_it() {
        let files: BTreeMap<String, String> = [
            (
                "item_types/a.it.json",
                r#"{"extends": "nothing", "blocks": [{"name": "Sockets", "entries": [{"key": "socket_info", "value": "1:1:1 2:1:1"}]}]}"#,
            ),
            ("item_types/b.it.json", r#"{"extends": "A", "blocks": []}"#),
            (
                "item_types/c.it.json",
                r#"{"extends": "B", "blocks": [{"name": "Sockets", "entries": [{"key": "socket_info", "value": "1:1:1"}]}]}"#,
            ),
            ("item_types/d.it.json", r#"{"extends": "nothing", "blocks": []}"#),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect();
        let read = |p: &str| -> Result<Vec<u8>, String> {
            files
                .get(p)
                .map(|s| s.as_bytes().to_vec())
                .ok_or_else(|| p.to_owned())
        };
        let mut t = ItemTags::default();
        assert_eq!(t.socket_info(&read, "B").as_deref(), Some("1:1:1 2:1:1"));
        assert_eq!(t.socket_info(&read, "C").as_deref(), Some("1:1:1"));
        assert_eq!(t.socket_info(&read, "D"), None);
        assert_eq!(t.socket_info(&read, "E"), None);
    }

    fn file(extends: &str, entries: &[(&str, &str)]) -> String {
        let entries: Vec<String> = entries
            .iter()
            .map(|(k, v)| format!(r#"{{"key": "{k}", "quoted": true, "value": "{v}"}}"#))
            .collect();
        format!(
            r#"{{"abstract": true, "comments": 0, "extends": "{extends}", "version": 3,
              "blocks": [{{"name": "Base", "entries": [{}], "blocks": []}},
                         {{"name": "Other", "entries": [{{"key": "tag", "value": "not_base"}}]}}]}}"#,
            entries.join(", ")
        )
    }

    fn read(path: &str) -> Result<Vec<u8>, String> {
        let text = match path {
            "item_types/metadata/items/item.it.json" => file("nothing", &[("tag", "default")]),
            "item_types/metadata/items/rings/abstractring.it.json" => file(
                "Metadata/Items/Item",
                &[("tag", "ring"), ("tag", "x"), ("tag", "x")],
            ),
            "item_types/metadata/items/rings/ring1.it.json" => file(
                "Metadata/Items/Rings/AbstractRing",
                &[("remove_tag", "x"), ("tag", "ring_kind")],
            ),
            "item_types/metadata/items/orphan.it.json" => file("Metadata/Items/Missing", &[]),
            "item_types/metadata/items/loop_a.it.json" => file("Metadata/Items/Loop_B", &[]),
            "item_types/metadata/items/loop_b.it.json" => file("Metadata/Items/Loop_A", &[]),
            _ => return Err(format!("{path}: not found")),
        };
        Ok(text.into_bytes())
    }

    #[test]
    fn tags_should_apply_the_chain_parent_first_and_remove_the_first_occurrence() {
        let tags = ItemTags::default().tags(&read, "Metadata/Items/Rings/Ring1");
        assert_eq!(
            tags,
            Some(
                ["default", "ring", "x", "ring_kind"]
                    .map(str::to_owned)
                    .to_vec()
            )
        );
    }

    #[test]
    fn tags_should_be_unknown_when_a_parent_file_is_missing() {
        assert_eq!(
            ItemTags::default().tags(&read, "Metadata/Items/Orphan"),
            None
        );
    }

    #[test]
    fn tags_should_be_unknown_when_the_chain_loops() {
        assert_eq!(
            ItemTags::default().tags(&read, "Metadata/Items/Loop_A"),
            None
        );
    }
}
