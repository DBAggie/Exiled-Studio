//! `<Items>` (P2-5b): items, item sets, and slots.
//!
//! - **Item:** `id`; the body is the **last** non-blank text child, as PoB
//!   reads it (`v2.67.2:src/Classes/ItemsTab.lua` calls `ParseRaw` for each
//!   text child and the last one wins; Spike B's rule), read with
//!   [`pob_item::read`](crate::legacy::pob_item::read) and
//!   [`to_domain`](crate::legacy::pob_item::to_domain). Every other child
//!   (other texts, `<ModRange>`, anything else) stays in place around
//!   [`ItemPart::Body`]. The body's whitespace is kept as
//!   [`Item::text_spacing`]; the export writes the text from the lines'
//!   sources (decision 6). An item whose text the spacing cannot rebuild is
//!   kept whole.
//! - **Item set:** `id`, `title`; `<Slot>` with `name` and `itemId`
//!   (`"0"`: empty); other children (`<SocketIdURL>`) in place.

use loom_domain::{
    Item, ItemId, ItemPart, ItemSet, ItemSetId, Items, ItemsEntry, Ordered, SlotEntry, SlotId,
};
use roxmltree::Node;

use super::read::{children, opaque, required_u32, rest, text};
use crate::legacy::pob_item;

pub(super) fn items_of(n: Node<'_, '_>) -> Option<Items> {
    let mut entries = Ordered::new();
    for c in children(n) {
        let typed = match (c.tag_name().name(), c.is_element()) {
            ("Item", true) => item(c).map(ItemsEntry::Item),
            ("ItemSet", true) => item_set(c).map(ItemsEntry::Set),
            _ => None,
        };
        match typed {
            Some(e) => entries.push(e),
            None => entries.push_opaque(opaque(c)?),
        }
    }
    Some(Items {
        active_set: super::read::number_u32(n, "activeItemSet")?.map(|s| ItemSetId(*s.value())),
        entries,
        attributes: rest(n, &["activeItemSet"]),
    })
}

/// The whitespace before, between, and after `lines` in `text`; `None`
/// when the text is not exactly those lines with whitespace around them.
fn spacing(text: &str, lines: &[&str]) -> Option<Vec<String>> {
    let mut out = Vec::with_capacity(lines.len() + 1);
    let mut at = 0;
    for line in lines {
        let found = at + text.get(at..)?.find(line)?;
        let gap = text.get(at..found)?;
        if !gap.trim().is_empty() {
            return None;
        }
        out.push(gap.to_owned());
        at = found + line.len();
    }
    let tail = text.get(at..)?;
    if !tail.trim().is_empty() {
        return None;
    }
    out.push(tail.to_owned());
    Some(out)
}

fn item(n: Node<'_, '_>) -> Option<Item> {
    let id = ItemId(*required_u32(n, "id")?.value());
    // Every child kept (an item is mixed content); the body is the last
    // text child that is not blank.
    let kids: Vec<Node<'_, '_>> = n.children().collect();
    let body_at = kids
        .iter()
        .rposition(|c| c.is_text() && c.text().is_some_and(|t| !t.trim().is_empty()));
    let mut layout = Ordered::new();
    let mut item = Item {
        id,
        body: loom_domain::ItemBody::default(),
        layout: Ordered::new(),
        text_spacing: Vec::new(),
        attributes: rest(n, &["id"]),
    };
    for (k, c) in kids.iter().enumerate() {
        if Some(k) == body_at {
            let raw = c.text().unwrap_or("");
            let read = pob_item::read(raw);
            let lines: Vec<&str> = read.lines.iter().map(|l| l.raw.as_str()).collect();
            item.text_spacing = spacing(raw, &lines)?;
            item.body = pob_item::to_domain(&read);
            layout.push(ItemPart::Body);
        } else if c.is_text() {
            layout.push_opaque(loom_domain::OpaqueNode::Text(
                c.text().unwrap_or("").to_owned(),
            ));
        } else if let Some(o) = opaque(*c) {
            layout.push_opaque(o);
        }
    }
    item.layout = layout;
    Some(item)
}

fn item_set(n: Node<'_, '_>) -> Option<ItemSet> {
    let mut slots = Ordered::new();
    for c in children(n) {
        let typed =
            (c.tag_name().name() == "Slot" && c.is_element() && children(c).next().is_none())
                .then(|| {
                    let item = *required_u32(c, "itemId")?.value();
                    Some(SlotEntry {
                        slot: SlotId::new(text(c, "name")?),
                        item: (item != 0).then_some(ItemId(item)),
                        attributes: rest(c, &["name", "itemId"]),
                    })
                })
                .flatten();
        match typed {
            Some(s) => slots.push(s),
            None => slots.push_opaque(opaque(c)?),
        }
    }
    Some(ItemSet {
        id: ItemSetId(*required_u32(n, "id")?.value()),
        title: text(n, "title"),
        slots,
        attributes: rest(n, &["id", "title"]),
    })
}
