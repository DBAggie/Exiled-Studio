//! Items, item sets, and slots.
//!
//! An item keeps every source line it was read with, each typed by kind
//! (Phase 2 plan, decision 3), so "source lines in = source lines out" and
//! an exporter can write the item back from the model. Only modifier lines
//! get a [`LineResolution`]. The fields here are what the import pipeline
//! reads (decision 4); names and values are game data, kept as text.

mod resolution;

pub use resolution::{
    Adapted, Exclusion, LineResolution, PropertyValue, Resolved, StatLine, StatValue,
    TypedResolution, UnresolvedReason,
};

use crate::error::DomainError;
use crate::id::{InfluenceId, ItemId, ItemSetId, SlotId};
use crate::opaque::{Attributes, Ordered};

/// The build's items and item sets.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Items {
    /// The active item set, if set.
    pub active_set: Option<ItemSetId>,
    /// Items and sets in source order, with untyped content in place.
    pub entries: Ordered<ItemsEntry>,
    /// Attributes not typed.
    pub attributes: Attributes,
}

impl Items {
    /// The items, in order.
    pub fn items(&self) -> impl Iterator<Item = &Item> {
        self.entries.typed().filter_map(|e| match e {
            ItemsEntry::Item(i) => Some(i),
            ItemsEntry::Set(_) => None,
        })
    }

    /// The item sets, in order.
    pub fn sets(&self) -> impl Iterator<Item = &ItemSet> {
        self.entries.typed().filter_map(|e| match e {
            ItemsEntry::Item(_) => None,
            ItemsEntry::Set(s) => Some(s),
        })
    }

    /// The item with this ID.
    #[must_use]
    pub fn item(&self, id: ItemId) -> Option<&Item> {
        self.items().find(|i| i.id == id)
    }
}

/// An item or an item set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemsEntry {
    /// An item.
    Item(Item),
    /// An item set.
    Set(ItemSet),
}

/// An item set: which item is in which slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemSet {
    /// The set's ID within the build.
    pub id: ItemSetId,
    /// The title, if any.
    pub title: Option<String>,
    /// Slots, with untyped content in place.
    pub slots: Ordered<SlotEntry>,
    /// Attributes not typed.
    pub attributes: Attributes,
}

/// One slot of an item set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotEntry {
    /// The slot.
    pub slot: SlotId,
    /// The item in it; `None` when empty.
    pub item: Option<ItemId>,
    /// Attributes not typed.
    pub attributes: Attributes,
}

/// Where an item's typed body sits among its untyped children (other
/// texts, per-line ranges, ...).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemPart {
    /// The text the item was read from ([`Item::body`]).
    Body,
}

/// One item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    /// The item's ID within the build.
    pub id: ItemId,
    /// The item as read from its text.
    pub body: ItemBody,
    /// The item's children: where the body sits among untyped content.
    pub layout: Ordered<ItemPart>,
    /// Attributes not typed.
    pub attributes: Attributes,
}

/// Item rarity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rarity {
    /// Normal.
    Normal,
    /// Magic.
    Magic,
    /// Rare.
    Rare,
    /// Unique.
    Unique,
    /// Relic: a unique-tier rarity the game data has for some uniques.
    Relic,
}

impl Rarity {
    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Magic => "magic",
            Self::Rare => "rare",
            Self::Unique => "unique",
            Self::Relic => "relic",
        }
    }

    /// Unique or relic.
    #[must_use]
    pub const fn is_unique(self) -> bool {
        matches!(self, Self::Unique | Self::Relic)
    }
}

/// Which modifier list a line belongs to. Sections all games share are
/// variants; the rest are data names the import adapter gives
/// ([`ModSection::Other`]), and game-specific rules about them are bundle
/// data, not code (Phase 2 plan, decision 5).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ModSection {
    /// Enchantments.
    Enchant,
    /// Implicit modifiers.
    Implicit,
    /// Explicit modifiers.
    Explicit,
    /// Another section, by name. PoB's scourge, crucible, and
    /// class-requirement lists map here (`scourge`, `crucible`,
    /// `class-requirement`); a class-requirement line is
    /// [`Exclusion::Section`].
    Other(String),
}

impl ModSection {
    /// Stable name.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Enchant => "enchant",
            Self::Implicit => "implicit",
            Self::Explicit => "explicit",
            Self::Other(name) => name,
        }
    }
}

/// A prefix or flag word on a modifier line (`{crafted}`,
/// `{variant:1,2}`, `(fractured)`), as written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prefix {
    /// The key.
    pub key: String,
    /// The value, if any.
    pub value: Option<String>,
}

impl Prefix {
    /// A prefix.
    #[must_use]
    pub fn new(key: impl Into<String>, value: Option<&str>) -> Self {
        Self {
            key: key.into(),
            value: value.map(str::to_owned),
        }
    }
}

/// A modifier line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModifierLine {
    /// The text without prefixes.
    pub text: String,
    /// Prefixes and flag words, in line order.
    pub prefixes: Vec<Prefix>,
    /// The section.
    pub section: ModSection,
    /// Whether the item's variant selection includes the line.
    pub selected: bool,
    /// What the line resolved to.
    pub resolution: LineResolution,
}

impl ModifierLine {
    /// Whether a prefix with this key is present.
    #[must_use]
    pub fn has(&self, key: &str) -> bool {
        self.prefixes.iter().any(|p| p.key == key)
    }
}

/// A header line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderLine {
    /// The item class.
    ItemClass,
    /// The rarity.
    Rarity,
    /// A separator.
    Separator,
    /// The name.
    Name,
    /// The base of a rare or unique item.
    Base,
}

/// What a line is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineKind {
    /// A header line.
    Header(HeaderLine),
    /// A spec line (`Key: value`).
    Spec {
        /// The key.
        key: String,
        /// The value, as written.
        value: String,
        /// Whether the import adapter knows the key; an unknown one is
        /// quarantined.
        known: bool,
    },
    /// A base line of one variant.
    VariantBase {
        /// The base text without prefixes.
        base: String,
        /// Whether the variant selection includes it.
        selected: bool,
    },
    /// An influence line.
    Influence(InfluenceId),
    /// An item flag line (`Corrupted`, `Mirrored`, ...).
    Flag,
    /// A modifier line.
    Modifier(ModifierLine),
    /// A line the import adapter did not recognize; quarantined.
    Unrecognized,
}

impl LineKind {
    /// Stable name.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Header(HeaderLine::ItemClass) => "item-class",
            Self::Header(HeaderLine::Rarity) => "rarity",
            Self::Header(HeaderLine::Separator) => "separator",
            Self::Header(HeaderLine::Name) => "name",
            Self::Header(HeaderLine::Base) => "base",
            Self::Spec { known: true, .. } => "spec",
            Self::Spec { known: false, .. } => "spec-unknown",
            Self::VariantBase { .. } => "variant-base",
            Self::Influence(_) => "influence",
            Self::Flag => "flag",
            Self::Modifier(_) => "modifier",
            Self::Unrecognized => "unrecognized",
        }
    }
}

/// One source line of an item.
///
/// `source` is the authority for export (Phase 2 plan, decision 7); the
/// kind's fields (a modifier's text, prefixes, and selection, a spec's
/// key and value) are the import adapter's reading of it. An edit
/// replaces the whole line, so the two cannot drift.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemLine {
    /// The line as written (trimmed).
    pub source: String,
    /// What it is.
    pub kind: LineKind,
}

/// An alternate variant the item has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AltVariant {
    /// Which alternate (0 for the first).
    pub index: u8,
    /// The selected variant, as written.
    pub selected: Option<u32>,
}

/// An item's variant selection, as written.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VariantSelection {
    /// Number of variants.
    pub count: u32,
    /// The selected variant, as written.
    pub selected: Option<u32>,
    /// Alternate variants the item has.
    pub alternates: Vec<AltVariant>,
}

/// An item as read from its text.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ItemBody {
    /// The item class, as written, if any.
    pub item_class: Option<String>,
    /// Rarity, if read.
    pub rarity: Option<Rarity>,
    /// The name, if any.
    pub name: Option<String>,
    /// The base name of a rare or unique item, as written.
    pub base: Option<String>,
    /// The implicit count, if written.
    pub implicits: Option<usize>,
    /// Variant selection.
    pub variants: VariantSelection,
    /// Every source line, in order.
    pub lines: Vec<ItemLine>,
}

impl ItemBody {
    /// The modifier lines with their indices.
    pub fn modifiers(&self) -> impl Iterator<Item = (usize, &ModifierLine)> {
        self.lines
            .iter()
            .enumerate()
            .filter_map(|(i, l)| match &l.kind {
                LineKind::Modifier(m) => Some((i, m)),
                _ => None,
            })
    }

    /// The influences, in line order.
    pub fn influences(&self) -> impl Iterator<Item = &InfluenceId> {
        self.lines.iter().filter_map(|l| match &l.kind {
            LineKind::Influence(id) => Some(id),
            _ => None,
        })
    }

    /// The flag lines' text, in line order.
    pub fn flags(&self) -> impl Iterator<Item = &str> {
        self.lines
            .iter()
            .filter(|l| l.kind == LineKind::Flag)
            .map(|l| l.source.as_str())
    }

    /// Checks the continuation links of multi-line resolutions:
    /// - each [`LineResolution::Continues`] points to a resolved modifier
    ///   line, and follows it, or another continuation of it, directly
    ///   among the modifier lines (a description over several lines, or a
    ///   split-section pair);
    /// - a line resolved with [`Adapted::SplitSection`] is continued by
    ///   exactly one line.
    ///
    /// # Errors
    /// [`DomainError::BrokenContinuation`] for the first bad link.
    pub fn check_links(&self) -> Result<(), DomainError> {
        let mut continued = vec![0_usize; self.lines.len()];
        // The resolved line the previous modifier line belongs to.
        let mut previous: Option<usize> = None;
        for (index, m) in self.modifiers() {
            previous = match m.resolution {
                LineResolution::Continues { line } => {
                    let first_resolved = matches!(
                        self.lines.get(line).map(|l| &l.kind),
                        Some(LineKind::Modifier(ModifierLine {
                            resolution: LineResolution::Resolved { .. },
                            ..
                        }))
                    );
                    let count = continued
                        .get_mut(line)
                        .filter(|_| first_resolved && previous == Some(line));
                    match count {
                        Some(c) => *c += 1,
                        None => return Err(DomainError::BrokenContinuation { line: index }),
                    }
                    Some(line)
                }
                LineResolution::Resolved { .. } => Some(index),
                _ => None,
            };
        }
        for (index, m) in self.modifiers() {
            let split = matches!(
                m.resolution,
                LineResolution::Resolved {
                    adapted: Some(Adapted::SplitSection),
                    ..
                }
            );
            if split && continued.get(index).copied() != Some(1) {
                return Err(DomainError::BrokenContinuation { line: index });
            }
        }
        Ok(())
    }
}
