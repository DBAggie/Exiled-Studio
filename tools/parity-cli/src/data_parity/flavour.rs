//! `flavour_text` (B-2's item card, `docs/ui/b2-data/card-data.md`):
//! the bundle's unique flavour text (`poe1/unique_flavour.json`) against
//! PoB's (`data.flavourText`, export format 8).
//!
//! A row's key is the unique's name and the `FlavourText` ID, trailing
//! underscores dropped (PoB's exporter drops them). PoB keeps each text
//! as the lines its exporter cleaned (`v2.67.2:src/Export/Scripts/
//! flavourText.lua`, `cleanAndSplit`); the bundle keeps the game's text,
//! so the bundle's is cleaned the same way here ([`pob_lines`]) and the
//! lines are compared. A row on one side only: PoB's only is a
//! difference, the bundle's only a bundle-only row.

use std::collections::BTreeMap;

use loom_data::bundle::Bundle;
use serde_json::json;

use super::compare::Comparison;
use super::pob::Export;

/// The table's name.
const T: &str = "flavour_text";

/// A row's key: `name|ID`, the ID without trailing underscores.
fn key(name: &str, id: &str) -> String {
    format!("{name}|{}", id.trim_end_matches('_'))
}

/// `text` as PoB's exporter cleans it (`cleanAndSplit`): `\r\n` made
/// `\n`; `<default>` made a line break and PoB's grey colour code `^8`;
/// each line trimmed, its `{...}` braces removed (contents kept) and its
/// `<<...>>` runs removed, trimmed again, and kept if not empty; a blank
/// line before each `^8` line (unless the last line is blank).
pub fn pob_lines(text: &str) -> Vec<String> {
    let text = text.replace("\r\n", "\n").replace("<default>", "\n^8");
    let mut lines: Vec<String> = Vec::new();
    for line in text.split('\n').map(str::trim).filter(|l| !l.is_empty()) {
        let line = strip(&strip(line, "{", "}", true), "<<", ">>", false);
        let line = line.trim();
        if line.starts_with("^8") && lines.last().is_none_or(|l| !l.is_empty()) {
            lines.push(String::new());
        }
        if !line.is_empty() {
            lines.push(line.to_owned());
        }
    }
    lines
}

/// `line` with each `open...close` run (the shortest, as Lua's `.-`)
/// replaced by its contents (`keep`) or removed; an `open` with no
/// `close` after it is left.
fn strip(line: &str, open: &str, close: &str, keep: bool) -> String {
    let mut out = String::new();
    let mut rest = line;
    while let Some(start) = rest.find(open) {
        let after = &rest[start + open.len()..];
        let Some(end) = after.find(close) else {
            break;
        };
        out.push_str(&rest[..start]);
        if keep {
            out.push_str(&after[..end]);
        }
        rest = &after[end + close.len()..];
    }
    out.push_str(rest);
    out
}

/// Compares the table (module docs).
pub fn compare_flavour_text(out: &mut Comparison, bundle: &Bundle, export: &Export) {
    let ours: BTreeMap<String, Vec<String>> = bundle
        .poe1
        .as_ref()
        .and_then(|p| p.unique_flavour.as_deref())
        .unwrap_or_default()
        .iter()
        .map(|r| (key(&r.name, &r.id), pob_lines(&r.text)))
        .collect();
    let mut pob: BTreeMap<String, &Vec<String>> = BTreeMap::new();
    for e in &export.flavour_text {
        // PoB lists a forced name before the join's; its lookup keeps the
        // first entry of a name and ID.
        pob.entry(key(&e.name, &e.id)).or_insert(&e.text);
    }
    out.bundle_only(T, ours.keys().filter(|k| !pob.contains_key(*k)).cloned());
    for (k, text) in pob {
        match ours.get(&k) {
            Some(lines) => out.check(T, &k, "text", json!(lines), json!(text)),
            None => out.pob_only(T, &k),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pob_lines_should_clean_as_pobs_exporter() {
        assert_eq!(
            pob_lines("The warrior who\r\nfears will fall."),
            ["The warrior who", "fears will fall."]
        );
        assert_eq!(
            pob_lines("Great works take time.\r\n\r\n<default>{This item can be transformed}"),
            [
                "Great works take time.",
                "",
                "^8This item can be transformed"
            ]
        );
        assert!(pob_lines("<<HBGAa>><<HBG01>>\r\n<<HBGAb>>").is_empty());
        assert_eq!(pob_lines("  {a} b {c"), ["a b {c"]);
        assert_eq!(
            key("Grand Spectrum", "UniqueJewel165__"),
            "Grand Spectrum|UniqueJewel165"
        );
    }
}
