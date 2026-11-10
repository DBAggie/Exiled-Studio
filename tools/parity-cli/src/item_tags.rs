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
}

impl ItemTags {
    /// Tags of `path` (`Metadata/Items/...`, any case); `None` when unknown
    /// (see the module docs).
    pub fn tags(&mut self, read: Read<'_>, path: &str) -> Option<Vec<String>> {
        self.walk(read, &path.to_lowercase(), 0)
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
