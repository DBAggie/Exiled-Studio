//! Unique flavour text for the PoE1 bundle (B-2's item card,
//! `docs/ui/b2-data/card-data.md`, Q-B2-1): `FlavourText` to
//! `poe1/unique_flavour.json` (`loom_data::bundle::poe1::UniqueFlavour`).
//!
//! The game links a unique to its text by no key. The join: each
//! `UniqueStashLayout` row names the unique (`WordsKey`, `Words.Text2`) and
//! its art (`ItemVisualIdentityKey`); the `FlavourText` row whose ID is the
//! art's ID, trailing underscores dropped on both (`UniqueJewel165__`), is
//! its text. Left out, as PoB's export does: maps, alternate art, Atlas
//! upgrades, and heist quest items (by the art ID), and rows marked
//! `IsAlternateArt`. Also left out: an upgraded version (`BaseVersion`
//! set) with its base's art, whose text the join would give it (Voidheart
//! would read Blackheart's). Then each `(FlavourText ID, name)` pair of the
//! game's list replaces what the join gives that name; a pair whose ID the
//! table lacks fails the conversion (the list no longer fits the data).

use std::collections::BTreeMap;

use loom_data::bundle::poe1::UniqueFlavour;

use crate::extract::{Read, Table};

/// The art IDs left out, as PoB v2.67.2's export
/// (`src/Export/Scripts/flavourText.lua`).
const SKIPPED_ART: [&str; 4] = ["Map", "AlternateArt", "AtlasUpgrade", "HeistQuest"];

/// An ID with its trailing underscores dropped.
fn normal(id: &str) -> &str {
    id.trim_end_matches('_')
}

/// Row `n` of a key cell, if it is one.
fn key(row: &[serde_json::Value], col: usize) -> Option<usize> {
    Table::int(row, col).and_then(|n| usize::try_from(n).ok())
}

/// The flavour text rows, sorted by name and ID; `None` when the extract
/// has no `UniqueStashLayout`.
///
/// # Errors
/// A table or column missing, a key out of range, or a pair whose ID
/// `FlavourText` lacks.
pub fn rows(read: Read<'_>, pairs: &[(&str, &str)]) -> Result<Option<Vec<UniqueFlavour>>, String> {
    let Some(layout) = Table::read(read, "UniqueStashLayout")? else {
        return Ok(None);
    };
    let words = Table::require(read, "Words")?;
    let art = Table::require(read, "ItemVisualIdentity")?;
    let flavour = Table::require(read, "FlavourText")?;
    let (l_word, l_art) = (
        layout.column("WordsKey")?,
        layout.column("ItemVisualIdentityKey")?,
    );
    let l_base = layout.column("BaseVersion")?;
    let l_alt = layout.column("IsAlternateArt")?;
    let (w_text2, w_text) = (words.column("Text2")?, words.column("Text")?);
    let a_id = art.column("Id")?;
    let (f_id, f_text) = (flavour.column("Id")?, flavour.column("Text")?);

    // Normalized ID -> (ID, text); the first row wins.
    let mut texts: BTreeMap<&str, (&str, &str)> = BTreeMap::new();
    for r in &flavour.rows {
        if let (Some(id), Some(text)) = (Table::text(r, f_id), Table::text(r, f_text)) {
            texts.entry(normal(id)).or_insert((id, text));
        }
    }
    let art_of = |row: &[serde_json::Value]| -> Result<&str, String> {
        key(row, l_art)
            .and_then(|n| art.rows.get(n))
            .and_then(|a| Table::text(a, a_id))
            .ok_or_else(|| {
                "UniqueStashLayout: a row's art is not an ItemVisualIdentity row".to_owned()
            })
    };
    let mut by_name: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    for r in &layout.rows {
        let id = art_of(r)?;
        if SKIPPED_ART.iter().any(|s| id.contains(s))
            || r.get(l_alt).and_then(serde_json::Value::as_bool) == Some(true)
        {
            continue;
        }
        if let Some(base) = key(r, l_base) {
            let base = layout
                .rows
                .get(base)
                .ok_or("UniqueStashLayout: a BaseVersion is not a row")?;
            if art_of(base)? == id {
                continue;
            }
        }
        let word = key(r, l_word)
            .and_then(|n| words.rows.get(n))
            .ok_or("UniqueStashLayout: a row's name is not a Words row")?;
        let name = Table::text(word, w_text2)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .or_else(|| Table::text(word, w_text).map(str::trim))
            .unwrap_or_default();
        if let (false, Some((fid, text))) = (name.is_empty(), texts.get(normal(id))) {
            by_name
                .entry(name.to_owned())
                .or_default()
                .insert((*fid).to_owned(), (*text).to_owned());
        }
    }
    // The list's pairs replace the join's rows of their names.
    let mut listed: BTreeMap<&str, BTreeMap<String, String>> = BTreeMap::new();
    for (fid, name) in pairs {
        let (id, text) = texts
            .get(normal(fid))
            .ok_or_else(|| format!("flavour pair ({fid}, {name}): FlavourText has no such ID"))?;
        listed
            .entry(name)
            .or_default()
            .insert((*id).to_owned(), (*text).to_owned());
    }
    for (name, rows) in listed {
        by_name.insert(name.to_owned(), rows);
    }
    Ok(Some(
        by_name
            .into_iter()
            .flat_map(|(name, rows)| {
                rows.into_iter().map(move |(id, text)| UniqueFlavour {
                    name: name.clone(),
                    id,
                    text: text.replace("\r\n", "\n"),
                })
            })
            .collect(),
    ))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::{Value, json};

    use super::*;

    fn table(name: &str, columns: &[&str], rows: &Value) -> (String, Vec<u8>) {
        let columns: Vec<Value> = columns
            .iter()
            .map(|c| json!({"name": c, "references": null}))
            .collect();
        let body = json!({"table": name, "columns": columns, "rows": rows});
        (format!("tables/{name}.json"), body.to_string().into_bytes())
    }

    fn files() -> BTreeMap<String, Vec<u8>> {
        [
            table(
                "Words",
                &["Wordlist", "Text", "Text2"],
                &json!([
                    [6, "Kaom's Heart", "Kaom's Heart"],
                    [6, "Blackheart", "Blackheart"],
                    [6, "Voidheart", ""],
                    [6, "Grand Spectrum", "Grand Spectrum"],
                    [6, "Goldrim", "Goldrim"]
                ]),
            ),
            table(
                "ItemVisualIdentity",
                &["Id"],
                &json!([
                    ["UniqueBodyStr1"],
                    ["UniqueRing1"],
                    ["UniqueJewel75"],
                    ["UniqueJewel165__"],
                    ["AlternateArtUniqueGoldrim"]
                ]),
            ),
            table(
                "FlavourText",
                &["Id", "Unknown", "Text"],
                &json!([
                    ["UniqueBodyStr1", 1, "The warrior who\r\nfears will fall."],
                    ["UniqueRing1", 2, "Blackheart's text"],
                    ["UniqueJewel75", 3, "Thoughts"],
                    ["UniqueJewel165", 4, "A wellspring"],
                    ["AlternateArtUniqueGoldrim", 5, "Not shown"],
                    ["FatedUnique2", 6, "Voidheart's text"]
                ]),
            ),
            table(
                "UniqueStashLayout",
                &[
                    "WordsKey",
                    "ItemVisualIdentityKey",
                    "BaseVersion",
                    "IsAlternateArt",
                ],
                &json!([
                    [0, 0, null, false],
                    [1, 1, null, false],
                    [2, 1, 1, false],
                    [3, 2, null, false],
                    [3, 3, null, false],
                    [4, 4, null, true]
                ]),
            ),
        ]
        .into_iter()
        .collect()
    }

    fn run(
        files: &BTreeMap<String, Vec<u8>>,
        pairs: &[(&str, &str)],
    ) -> Result<Option<Vec<UniqueFlavour>>, String> {
        let read = |p: &str| files.get(p).cloned().ok_or_else(|| format!("{p}: missing"));
        rows(&read, pairs)
    }

    fn row(name: &str, id: &str, text: &str) -> UniqueFlavour {
        UniqueFlavour {
            name: name.to_owned(),
            id: id.to_owned(),
            text: text.to_owned(),
        }
    }

    #[test]
    fn rows_should_join_by_art_and_skip_shared_art_upgrades_and_alternate_art() {
        let r = run(&files(), &[]).unwrap().unwrap();
        assert_eq!(
            r,
            [
                row("Blackheart", "UniqueRing1", "Blackheart's text"),
                row("Grand Spectrum", "UniqueJewel165", "A wellspring"),
                row("Grand Spectrum", "UniqueJewel75", "Thoughts"),
                row(
                    "Kaom's Heart",
                    "UniqueBodyStr1",
                    "The warrior who\nfears will fall."
                ),
            ]
        );
    }

    #[test]
    fn rows_should_take_the_lists_pairs_over_the_join() {
        let r = run(
            &files(),
            &[
                ("FatedUnique2", "Voidheart"),
                ("UniqueRing1", "Kaom's Heart"),
            ],
        )
        .unwrap()
        .unwrap();
        assert!(r.contains(&row("Voidheart", "FatedUnique2", "Voidheart's text")));
        let kaom: Vec<_> = r.iter().filter(|x| x.name == "Kaom's Heart").collect();
        assert_eq!(
            kaom,
            [&row("Kaom's Heart", "UniqueRing1", "Blackheart's text")]
        );
    }

    #[test]
    fn rows_should_be_none_without_the_layout_and_fail_on_an_unknown_pair() {
        assert_eq!(run(&BTreeMap::new(), &[]).unwrap(), None);
        let e = run(&files(), &[("FatedUnique99", "X")]).unwrap_err();
        assert!(e.contains("FatedUnique99"), "{e}");
    }
}
