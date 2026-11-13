//! Item edits (Phase 4 plan, decision 6): line operations on one item,
//! applied to its text and read back through the PoB item reader, then
//! resolved, so an edited line gets the same reading as an imported one
//! and lines that do not resolve are kept as warnings (ADR-015).

use std::collections::BTreeMap;

use loom_domain::{
    Build, Entry, Item, ItemId, ItemLine, ItemPart, ItemsEntry, LineKind, ModSection, OpaqueNode,
};
use loom_serialization::dto::app::v1::{ItemEdit, LineOp};
use loom_serialization::legacy::{pob_item, pob_xml};

use crate::data::GameData;
use crate::error::{AppError, Code};
use loom_pipeline::RangeEnds;

/// A build and its range ends after an edit.
pub struct Edited {
    /// The build.
    pub build: Build,
    /// Its range ends.
    pub ends: RangeEnds,
}

/// What the edit does to one original line.
enum Fate {
    Keep,
    Remove,
    Text(String),
}

/// Applies `edit` to `build`, resolving the edited item with `data`.
///
/// # Errors
/// `APP_EDIT_ITEM`, `APP_STALE_EDIT` (a named line's text is not the one
/// the edit expects, or a named line has no expectation), `APP_EDIT_LINE`,
/// `APP_RANGE`.
pub fn apply(
    build: &Build,
    ends: &RangeEnds,
    edit: &ItemEdit,
    data: &GameData,
) -> Result<Edited, AppError> {
    let id = ItemId(edit.item);
    let item = build
        .items
        .item(id)
        .ok_or_else(|| AppError::new(Code::EditItem, format!("no item {}", edit.item)))?;
    let lines = &item.body.lines;
    let expect: BTreeMap<u32, &str> = edit
        .expect
        .iter()
        .map(|e| (e.line, e.text.as_str()))
        .collect();
    let mut fates: Vec<Fate> = lines.iter().map(|_| Fate::Keep).collect();
    let mut added = Vec::new();
    for op in &edit.ops {
        let line = match op {
            LineOp::Replace { line, .. }
            | LineOp::Remove { line }
            | LineOp::SetRange { line, .. } => Some(*line),
            LineOp::Add { .. } => None,
        };
        if let Some(n) = line {
            let source = usize::try_from(n)
                .ok()
                .and_then(|i| lines.get(i))
                .ok_or_else(|| AppError::new(Code::EditLine, format!("no line {n}")))?;
            if expect.get(&n) != Some(&source.source.as_str()) {
                return Err(AppError::new(
                    Code::StaleEdit,
                    format!("line {n} is not the line the edit was made for"),
                ));
            }
            if !editable(&source.kind) {
                return Err(AppError::new(
                    Code::EditLine,
                    format!("line {n} is a {} line", source.kind.name()),
                ));
            }
        }
        let index = line.and_then(|n| usize::try_from(n).ok());
        match op {
            LineOp::Replace { text, .. } => {
                let text = one_line(text)?;
                if let Some(f) = index.and_then(|i| fates.get_mut(i)) {
                    *f = Fate::Text(text);
                }
            }
            LineOp::Remove { .. } => {
                if let Some(f) = index.and_then(|i| fates.get_mut(i)) {
                    *f = Fate::Remove;
                }
            }
            LineOp::SetRange { value, .. } => {
                let current = index
                    .and_then(|i| match fates.get(i) {
                        Some(Fate::Text(t)) => Some(t.clone()),
                        _ => lines.get(i).map(|l| l.source.clone()),
                    })
                    .unwrap_or_default();
                let text = set_range(&current, *value)?;
                if let Some(f) = index.and_then(|i| fates.get_mut(i)) {
                    *f = Fate::Text(text);
                }
            }
            LineOp::Add { text } => added.push(one_line(text)?),
        }
    }

    // Implicit lines removed lower the item's `Implicits:` count.
    let removed_implicits = lines
        .iter()
        .zip(&fates)
        .filter(|(l, f)| {
            matches!(f, Fate::Remove)
                && matches!(&l.kind, LineKind::Modifier(m) if m.section == ModSection::Implicit)
        })
        .count();
    let mut sources: Vec<String> = Vec::new();
    for (line, fate) in lines.iter().zip(fates) {
        match fate {
            Fate::Remove => {}
            Fate::Text(t) => sources.push(t),
            Fate::Keep => {
                if removed_implicits > 0
                    && matches!(&line.kind, LineKind::Spec { key, .. } if key == "Implicits")
                {
                    let count = item.body.implicits.unwrap_or(0);
                    sources.push(format!(
                        "Implicits: {}",
                        count.saturating_sub(removed_implicits)
                    ));
                } else {
                    sources.push(line.source.clone());
                }
            }
        }
    }
    sources.extend(added);

    // The text through the export's own writer (lines joined; no spacing).
    let shell = Item {
        id: item.id,
        body: loom_domain::ItemBody {
            lines: sources
                .into_iter()
                .map(|source| ItemLine {
                    source,
                    kind: LineKind::Unrecognized,
                })
                .collect(),
            ..loom_domain::ItemBody::default()
        },
        layout: loom_domain::Ordered::new(),
        text_spacing: Vec::new(),
        attributes: Vec::new(),
    };
    let text = pob_xml::item_text(&shell);
    let mut body = pob_item::to_domain(&pob_item::read(&text));
    let item_ends = loom_pipeline::resolve_item(data.resolver, data.data, &mut body);
    let mut body = Some(body);

    let mut out = build.clone();
    for entry in out.items.entries.typed_mut() {
        if let ItemsEntry::Item(it) = entry
            && it.id == id
            && let Some(body) = body.take()
        {
            it.body = body;
            // Lines were added, removed, or changed: the text is written
            // one line per line (domain rule on `text_spacing`).
            it.text_spacing = Vec::new();
            // PoB's `<ModRange>` entries name lines by position; after an
            // edit they would name the wrong ones, so they go (PoB then
            // reads each line's own `{range:}`).
            it.layout = std::mem::take(&mut it.layout)
                .entries()
                .iter()
                .filter(|e| !matches!(e, Entry::Opaque(OpaqueNode::Element { name, .. }) if name == "ModRange"))
                .cloned()
                .collect::<loom_domain::Ordered<ItemPart>>();
        }
    }
    let ends = loom_pipeline::with_item_ends(ends, id, item_ends);
    Ok(Edited { build: out, ends })
}

/// Lines an edit may replace or remove: modifier lines and lines the
/// reader did not recognize (warnings the user can fix).
fn editable(kind: &LineKind) -> bool {
    matches!(
        kind,
        LineKind::Modifier(_) | LineKind::Unrecognized | LineKind::Spec { known: false, .. }
    )
}

fn one_line(text: &str) -> Result<String, AppError> {
    let t = text.trim();
    if t.is_empty() || t.contains(['\n', '\r']) {
        return Err(AppError::new(
            Code::EditLine,
            "a line must be one non-empty line",
        ));
    }
    Ok(t.to_owned())
}

/// Writes `value` as the line's `{range:r}` prefix (PoB writes six
/// decimals at most), replacing one that is there.
fn set_range(source: &str, value: f64) -> Result<String, AppError> {
    if !(0.0..=1.0).contains(&value) {
        return Err(AppError::new(Code::Range, "a roll is from 0 to 1"));
    }
    if !has_range(source) {
        return Err(AppError::new(Code::Range, "the line has no range"));
    }
    let mut r = format!("{value:.6}");
    while r.ends_with('0') {
        r.pop();
    }
    if r.ends_with('.') {
        r.pop();
    }
    let prefix = format!("{{range:{r}}}");
    if let Some(start) = source.find("{range:")
        && let Some(len) = source.get(start..).and_then(|s| s.find('}'))
    {
        let mut out = String::with_capacity(source.len());
        out.push_str(source.get(..start).unwrap_or_default());
        out.push_str(&prefix);
        out.push_str(source.get(start + len + 1..).unwrap_or_default());
        return Ok(out);
    }
    Ok(format!("{prefix}{source}"))
}

/// A `(a-b)` range in the text.
fn has_range(text: &str) -> bool {
    let b = text.as_bytes();
    b.windows(2).enumerate().any(|(i, w)| {
        w[0] == b'('
            && (w[1].is_ascii_digit() || w[1] == b'-')
            && b.get(i + 1..)
                .is_some_and(|rest| rest.iter().take_while(|c| **c != b')').any(|c| *c == b'-'))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_range_should_write_or_replace_the_prefix() {
        assert_eq!(
            set_range("+(10-20) to maximum Life", 0.25).unwrap(),
            "{range:0.25}+(10-20) to maximum Life"
        );
        assert_eq!(
            set_range("{crafted}{range:0.5}+(10-20) to maximum Life", 1.0).unwrap(),
            "{crafted}{range:1}+(10-20) to maximum Life"
        );
        assert_eq!(
            set_range("+15 to maximum Life", 0.5).unwrap_err().code,
            "APP_RANGE"
        );
        assert_eq!(set_range("+(1-2) to x", 1.5).unwrap_err().code, "APP_RANGE");
    }

    #[test]
    fn one_line_should_refuse_empty_and_multi_line_text() {
        assert!(one_line("  ").is_err());
        assert!(one_line("a\nb").is_err());
        assert_eq!(one_line(" +1 to x ").unwrap(), "+1 to x");
    }
}
