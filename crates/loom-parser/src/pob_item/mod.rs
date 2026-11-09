//! Reader for one PoB item's text (Spike B, PR B3): the text PoB saves in a
//! build's `<Item>` element, which `v2.67.2:src/Classes/Item.lua`
//! `ItemClass:BuildRaw` writes and `ItemClass:ParseRaw` reads back. Import
//! parsing per ADR-015: every non-blank line of the text ends in exactly one
//! [`LineKind`]; nothing is dropped. The XML around the text is the host's
//! job (`loom-serialization` in Phase 2; the corpus harness in Spike B).
//!
//! What is read, with the `ParseRaw` snippet each rule follows:
//! - Lines: `raw:gmatch("%s*([^\n]*%S)")`, so lines are trimmed and blank
//!   lines are skipped.
//! - Header: an optional `Item Class:` line, then `Rarity: <WORD>`
//!   (`^Rarity: (%a+)`, compared upper-cased), an optional `--------`, the
//!   name line, and for rarities other than normal and magic (and when no
//!   line is `Unidentified`) the next line, which `BuildRaw` writes as the
//!   base (`t_insert(rawLines, self.title)`, then `self.baseName`).
//! - Before `Implicits: n`: spec lines (`^([%a %(%)]+:?): (.+)$` or
//!   `^(Requires %a+) (.+)$`; keys `ParseRaw` knows are listed in
//!   [`KNOWN_SPECS`]); influence lines (`<display> Item`,
//!   `v2.67.2:src/Modules/ItemTools.lua` `influenceInfo.all`); base lines of
//!   unique variants (`{variant:..}<base>`, written by `BuildRaw` from
//!   `self.baseLines`); flag lines. Anything else is
//!   [`LineKind::Unrecognized`].
//! - From `Implicits: n` on, every line is a modifier, except the exact
//!   flag lines `ParseRaw` tests by equality (`Split`, `Mirrored`,
//!   `Corrupted`, `Fractured Item`, `Synthesised Item`; also
//!   `Unidentified`, `Veiled Prefix`, `Veiled Suffix`, and
//!   `Foil Unique (...)`), and influence lines.
//! - Modifier prefixes: every `{key:value}` in the line
//!   (`line:gsub("{(%a*):?([^}]*)}", ...)` removes them wherever they are),
//!   then every ` (word)` of lower-case letters (`line:gsub(" %((%l+)%)",
//!   ...)`, which removes all of them and keeps the word as a flag when it
//!   is a line flag), then a ` - Unscalable Value` suffix.
//! - Section: the first `n` lines go to enchant, scourge, or implicit lists
//!   and the rest to explicit, crucible, or class-requirement lists, by the
//!   `ParseRaw` order of tests ([`Section`]).
//! - Variant selection (`ItemClass:CheckModLineVariant`, without variant
//!   groups): a line with a variant list is selected when it holds the
//!   selected variant or, when the item has it, an alt variant. Defaults as
//!   `ParseRaw` sets them at the end: `self.variant = m_min(#variantList,
//!   self.variant or #variantList)`, the same for alt variants.
//!
//! Not reproduced (none occurs in the maintainer's corpus, 2026-11-09 survey;
//! each is counted by the harness): variant groups and versions (`{group}`,
//! `{version}`, `Selected Variant Group`), `{modGroup}` line joining, the
//! advanced copy format (`{ ... }` lines and `value(min-max)` text), flask
//! and tincture buff lines, mod lines that equal a base name.

#[cfg(test)]
#[expect(
    clippy::indexing_slicing,
    reason = "tests index fixed synthetic data; a bad index should panic"
)]
mod tests;

/// Item rarity, as `Rarity:` names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rarity {
    /// `NORMAL`.
    Normal,
    /// `MAGIC`.
    Magic,
    /// `RARE`.
    Rare,
    /// `UNIQUE`.
    Unique,
    /// `RELIC` (a unique; PoB also sets it for `Foil Unique` lines).
    Relic,
}

impl Rarity {
    /// Parses the word after `Rarity: ` (any case).
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        match word.to_ascii_uppercase().as_str() {
            "NORMAL" => Some(Self::Normal),
            "MAGIC" => Some(Self::Magic),
            "RARE" => Some(Self::Rare),
            "UNIQUE" => Some(Self::Unique),
            "RELIC" => Some(Self::Relic),
            _ => None,
        }
    }

    /// Stable name for reports.
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

    /// Normal and magic items have no separate base line.
    #[must_use]
    pub const fn names_base_in_name(self) -> bool {
        matches!(self, Self::Normal | Self::Magic)
    }
}

/// Which modifier list `ParseRaw` puts a line in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Section {
    /// `{enchant}`, or `{crafted}` while the implicit count is not reached.
    Enchant,
    /// `{scourge}`.
    Scourge,
    /// Text containing `Requires Class` (not counted as an implicit).
    ClassRequirement,
    /// `{implicit}`, or not crafted while the implicit count is not reached.
    Implicit,
    /// `{crucible}` after the implicits.
    Crucible,
    /// Every other line.
    Explicit,
}

impl Section {
    /// Stable name for reports.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Enchant => "enchant",
            Self::Scourge => "scourge",
            Self::ClassRequirement => "class-requirement",
            Self::Implicit => "implicit",
            Self::Crucible => "crucible",
            Self::Explicit => "explicit",
        }
    }
}

/// `ParseRaw`'s `lineFlags` (`v2.67.2:src/Classes/Item.lua`).
pub const LINE_FLAGS: [&str; 17] = [
    "crafted",
    "crucible",
    "custom",
    "disabled",
    "eater",
    "enchant",
    "exarch",
    "fractured",
    "implicit",
    "scourge",
    "synthesis",
    "mutated",
    "unscalable",
    "prefix",
    "suffix",
    "unveiled",
    "vestigial",
];

/// Prefix keys with a value that `ParseRaw` reads.
pub const VALUE_PREFIXES: [&str; 7] = [
    "variant",
    "version",
    "group",
    "tags",
    "modGroup",
    "range",
    "corruptedRange",
];

/// Spec keys `ParseRaw` handles (`specName == ...`), plus `Implicits`.
pub const KNOWN_SPECS: [&str; 59] = [
    "Unique ID",
    "Item Level",
    "Memory Strands",
    "Requires Class",
    "Quality",
    "Sockets",
    "Radius",
    "Limited to",
    "Version",
    "Variant",
    "Talisman Tier",
    "Armour",
    "Evasion Rating",
    "Evasion",
    "Energy Shield",
    "Ward",
    "ArmourBasePercentile",
    "EvasionBasePercentile",
    "EnergyShieldBasePercentile",
    "WardBasePercentile",
    "Requires Level",
    "Level",
    "LevelReq",
    "Has Alt Variant",
    "Has Alt Variant Two",
    "Has Alt Variant Three",
    "Has Alt Variant Four",
    "Has Alt Variant Five",
    "Selected Version",
    "Selected Variant Group",
    "Selected Variant",
    "Selected Alt Variant",
    "Selected Alt Variant Two",
    "Selected Alt Variant Three",
    "Selected Alt Variant Four",
    "Selected Alt Variant Five",
    "Allow Duplicate Variants",
    "Has Variants",
    "Selected Variants",
    "League",
    "Crafted",
    "Scourge",
    "Crucible",
    "Implicit",
    "Prefix",
    "Suffix",
    "Implicits",
    "Unreleased",
    "Upgrade",
    "Source",
    "Cluster Jewel Skill",
    "Cluster Jewel Node Count",
    "Catalyst",
    "CatalystQuality",
    "Intangibility",
    "Note",
    "Str",
    "Dex",
    "Int",
];

/// Influence displays (`v2.67.2:src/Modules/ItemTools.lua`
/// `influenceInfo.all`); the line is `<display> Item`.
pub const INFLUENCES: [&str; 8] = [
    "Shaper",
    "Elder",
    "Warlord",
    "Hunter",
    "Crusader",
    "Redeemer",
    "Searing Exarch",
    "Eater of Worlds",
];

/// Lines `ParseRaw` compares by equality and that `BuildRaw` writes as item
/// flags, plus `Unidentified` and the veiled lines (Spike B plan, step 3).
pub const FLAG_LINES: [&str; 8] = [
    "Split",
    "Mirrored",
    "Corrupted",
    "Fractured Item",
    "Synthesised Item",
    "Unidentified",
    "Veiled Prefix",
    "Veiled Suffix",
];

/// One `{key:value}` prefix (or a ` (word)` flag, with `value` `None`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prefix {
    /// The key: the letters after `{` (`%a*`), or the word in ` (word)`.
    pub key: String,
    /// The text after the key and an optional `:`, if not empty.
    pub value: Option<String>,
}

/// A modifier line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Modifier {
    /// The text without prefixes, flag words, and the unscalable suffix.
    pub text: String,
    /// Prefixes in line order, then ` (word)` flags.
    pub prefixes: Vec<Prefix>,
    /// The list `ParseRaw` puts the line in.
    pub section: Section,
    /// Whether the item's variant selection includes the line.
    pub selected: bool,
}

impl Modifier {
    /// Whether a prefix with this key is present.
    #[must_use]
    pub fn has(&self, key: &str) -> bool {
        self.prefixes.iter().any(|p| p.key == key)
    }

    /// The value of the first prefix with this key.
    #[must_use]
    pub fn value(&self, key: &str) -> Option<&str> {
        self.prefixes
            .iter()
            .find(|p| p.key == key)
            .and_then(|p| p.value.as_deref())
    }

    /// Prefix keys that are neither line flags nor value prefixes.
    #[must_use]
    pub fn unknown_prefixes(&self) -> Vec<&str> {
        self.prefixes
            .iter()
            .map(|p| p.key.as_str())
            .filter(|k| !is_known_prefix(k))
            .collect()
    }
}

/// Whether `key` is a line flag or a value prefix `ParseRaw` reads.
#[must_use]
pub fn is_known_prefix(key: &str) -> bool {
    !key.is_empty() && (LINE_FLAGS.contains(&key) || VALUE_PREFIXES.contains(&key))
}

/// What a line is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineKind {
    /// `Item Class: ...`.
    ItemClass,
    /// `Rarity: ...` with a known rarity.
    Rarity,
    /// `--------` separator.
    Separator,
    /// The name line.
    Name,
    /// The base line of a rare or unique item.
    Base,
    /// A spec line before the implicits.
    Spec {
        /// The key.
        key: String,
        /// Whether `ParseRaw` handles the key ([`KNOWN_SPECS`]).
        known: bool,
    },
    /// A unique variant's base line (`{variant:1,2}<base>`).
    VariantBase {
        /// Base text without prefixes.
        base: String,
        /// Whether the selection includes it.
        selected: bool,
    },
    /// `<influence> Item`.
    Influence,
    /// One of [`FLAG_LINES`] or `Foil Unique (...)`.
    Flag,
    /// A modifier line.
    Modifier(Modifier),
    /// None of the above (before the implicits).
    Unrecognized,
}

impl LineKind {
    /// Stable name for reports.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::ItemClass => "item-class",
            Self::Rarity => "rarity",
            Self::Separator => "separator",
            Self::Name => "name",
            Self::Base => "base",
            Self::Spec { known: true, .. } => "spec",
            Self::Spec { known: false, .. } => "spec-unknown",
            Self::VariantBase { .. } => "variant-base",
            Self::Influence => "influence",
            Self::Flag => "flag",
            Self::Modifier(_) => "modifier",
            Self::Unrecognized => "unrecognized",
        }
    }
}

/// One non-blank line of the item text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// The trimmed line as written.
    pub raw: String,
    /// What it is.
    pub kind: LineKind,
}

/// Variant selection of an item, as written and as `ParseRaw` defaults it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Variants {
    /// Number of `Variant:` lines.
    pub count: u32,
    /// `Selected Variant:` as written.
    pub selected: Option<u32>,
    /// `Has Alt Variant` (index 0) to `Has Alt Variant Five` (index 4):
    /// whether the item has it, and `Selected Alt Variant ...` as written.
    pub alt: [(bool, Option<u32>); 5],
}

impl Variants {
    /// `ParseRaw`'s default: `m_min(#variantList, v or #variantList)`;
    /// `None` without variants.
    fn effective(&self, written: Option<u32>) -> Option<u32> {
        (self.count > 0).then(|| written.unwrap_or(self.count).min(self.count))
    }

    /// `CheckModLineVariant` without variant groups.
    #[must_use]
    pub fn includes(&self, list: Option<&[u32]>) -> bool {
        let Some(list) = list else {
            return true;
        };
        let main = self.effective(self.selected);
        main.is_some_and(|v| list.contains(&v))
            || self
                .alt
                .iter()
                .any(|(has, sel)| *has && self.effective(*sel).is_some_and(|v| list.contains(&v)))
    }
}

/// One item read from its text.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Item {
    /// Rarity; `None` without a `Rarity:` line or with an unknown word.
    pub rarity: Option<Rarity>,
    /// The name line.
    pub name: Option<String>,
    /// The base line of a rare or unique item, as written.
    pub base: Option<String>,
    /// `Implicits: n`, if present and a number.
    pub implicits: Option<usize>,
    /// Variant selection.
    pub variants: Variants,
    /// Every non-blank line, in order.
    pub lines: Vec<Line>,
}

impl Item {
    /// The modifier lines.
    pub fn modifiers(&self) -> impl Iterator<Item = &Modifier> {
        self.lines.iter().filter_map(|l| match &l.kind {
            LineKind::Modifier(m) => Some(m),
            _ => None,
        })
    }
}

/// The key of a spec-shaped line: `^([%a %(%)]+:?): (.+)$` or
/// `^(Requires %a+) (.+)$`.
#[must_use]
pub fn spec_key(line: &str) -> Option<&str> {
    if let Some((key, value)) = line.split_once(": ") {
        let body = key.strip_suffix(':').unwrap_or(key);
        if !body.is_empty()
            && body
                .chars()
                .all(|c| c.is_ascii_alphabetic() || c == ' ' || c == '(' || c == ')')
            && !value.is_empty()
        {
            return Some(key);
        }
    }
    let rest = line.strip_prefix("Requires ")?;
    let (word, value) = rest.split_once(' ')?;
    (!word.is_empty() && word.chars().all(|c| c.is_ascii_alphabetic()) && !value.is_empty())
        .then(|| line.get(..9 + word.len()))
        .flatten()
}

fn spec_value<'l>(line: &'l str, key: &str) -> &'l str {
    line.get(key.len()..)
        .map(|r| r.strip_prefix(": ").unwrap_or(r).trim_start())
        .unwrap_or("")
}

/// Lua `tonumber` on a whole spec value, for integers.
fn number(value: &str) -> Option<u32> {
    value.trim().parse().ok()
}

fn is_influence(line: &str) -> bool {
    line.strip_suffix(" Item")
        .is_some_and(|d| INFLUENCES.contains(&d))
}

fn is_flag(line: &str) -> bool {
    FLAG_LINES.contains(&line) || line.contains("Foil Unique")
}

/// Removes every `{key:value}` group and returns the rest and the prefixes
/// (`line:gsub("{(%a*):?([^}]*)}", ...)`; a `{` without a later `}` stays).
#[must_use]
pub fn strip_prefixes(line: &str) -> (String, Vec<Prefix>) {
    let mut text = String::with_capacity(line.len());
    let mut prefixes = Vec::new();
    let mut rest = line;
    while let Some(open) = rest.find('{') {
        let (before, from) = rest.split_at(open);
        let Some(close) = from.find('}') else {
            break;
        };
        text.push_str(before);
        let inner = from.get(1..close).unwrap_or("");
        let key_len = inner
            .find(|c: char| !c.is_ascii_alphabetic())
            .unwrap_or(inner.len());
        let (key, tail) = inner.split_at(key_len);
        let value = tail.strip_prefix(':').unwrap_or(tail);
        prefixes.push(Prefix {
            key: key.to_owned(),
            value: (!value.is_empty()).then(|| value.to_owned()),
        });
        rest = from.get(close + 1..).unwrap_or("");
    }
    text.push_str(rest);
    (text, prefixes)
}

/// Removes every ` (word)` of lower-case ASCII letters
/// (`line:gsub(" %((%l+)%)", ...)`), returning the words.
fn strip_flag_words(line: &str) -> (String, Vec<String>) {
    let mut text = String::with_capacity(line.len());
    let mut words = Vec::new();
    let mut rest = line;
    while let Some(at) = rest.find(" (") {
        let (before, from) = rest.split_at(at);
        let after = from.get(2..).unwrap_or("");
        let len = after
            .find(|c: char| !c.is_ascii_lowercase())
            .unwrap_or(after.len());
        if len > 0 && after.get(len..).is_some_and(|r| r.starts_with(')')) {
            text.push_str(before);
            words.push(after.get(..len).unwrap_or("").to_owned());
            rest = after.get(len + 1..).unwrap_or("");
        } else {
            text.push_str(before);
            text.push_str(" (");
            rest = after;
        }
    }
    text.push_str(rest);
    (text, words)
}

/// The numbers in a `{variant:1,2}` value (`val:gmatch("%d+")`).
#[must_use]
pub fn variant_list(value: &str) -> Vec<u32> {
    value
        .split(|c: char| !c.is_ascii_digit())
        .filter_map(|d| d.parse().ok())
        .collect()
}

/// Counts toward `Implicits: n` while reading modifiers.
#[derive(Default)]
struct Counts {
    enchant: usize,
    scourge: usize,
    implicit: usize,
}

/// `ParseRaw`'s choice of list (its `if modLine.enchant or ...` chain).
fn section(m: &Modifier, implicits: usize, counts: &mut Counts) -> Section {
    let enchant = m.has("enchant");
    let crafted = m.has("crafted") || enchant;
    let implicit = m.has("implicit") || enchant;
    let section = if enchant || (crafted && counts.enchant + counts.implicit < implicits) {
        Section::Enchant
    } else if m.has("scourge") {
        Section::Scourge
    } else if m.text.contains("Requires Class") {
        Section::ClassRequirement
    } else if implicit
        || (!crafted && counts.enchant + counts.scourge + counts.implicit < implicits)
    {
        Section::Implicit
    } else if m.has("crucible") {
        Section::Crucible
    } else {
        Section::Explicit
    };
    match section {
        Section::Enchant => counts.enchant += 1,
        Section::Scourge => counts.scourge += 1,
        Section::Implicit => counts.implicit += 1,
        _ => {}
    }
    section
}

/// Reads a modifier line's prefixes and text (section and selection are
/// set by the caller).
#[must_use]
pub fn read_modifier(line: &str) -> Modifier {
    let (text, mut prefixes) = strip_prefixes(line);
    let (text, words) = strip_flag_words(&text);
    prefixes.extend(words.into_iter().map(|key| Prefix { key, value: None }));
    let text = text
        .strip_suffix(" - Unscalable Value")
        .map_or(text.clone(), |t| {
            prefixes.push(Prefix {
                key: "unscalable".to_owned(),
                value: None,
            });
            t.to_owned()
        });
    Modifier {
        text,
        prefixes,
        section: Section::Explicit,
        selected: true,
    }
}

/// Reads one item's text. Never fails: what it cannot place is
/// [`LineKind::Unrecognized`].
#[must_use]
pub fn read(text: &str) -> Item {
    let raw: Vec<&str> = text
        .split('\n')
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    let mut item = Item::default();
    let unidentified = raw.contains(&"Unidentified");
    let mut kinds: Vec<Option<LineKind>> = vec![None; raw.len()];
    let mut at = 0;
    let set = |kinds: &mut Vec<Option<LineKind>>, i: usize, kind: LineKind| {
        if let Some(slot) = kinds.get_mut(i) {
            *slot = Some(kind);
        }
    };

    // Header.
    if raw.get(at).is_some_and(|l| l.starts_with("Item Class:")) {
        set(&mut kinds, at, LineKind::ItemClass);
        at += 1;
    }
    if let Some(word) = raw.get(at).and_then(|l| l.strip_prefix("Rarity: ")) {
        let word: String = word.chars().take_while(char::is_ascii_alphabetic).collect();
        if let Some(rarity) = Rarity::parse(&word) {
            item.rarity = Some(rarity);
            set(&mut kinds, at, LineKind::Rarity);
            at += 1;
        }
    }
    if raw.get(at) == Some(&"--------") {
        set(&mut kinds, at, LineKind::Separator);
        at += 1;
    }
    if let Some(name) = raw.get(at) {
        item.name = Some((*name).to_owned());
        set(&mut kinds, at, LineKind::Name);
        at += 1;
        // `ParseRaw`'s default rarity (no `Rarity:` line) is UNIQUE.
        let rarity = item.rarity.unwrap_or(Rarity::Unique);
        if !rarity.names_base_in_name()
            && !unidentified
            && let Some(base) = raw.get(at)
        {
            item.base = Some((*base).to_owned());
            set(&mut kinds, at, LineKind::Base);
            at += 1;
        }
    }

    // Spec lines up to `Implicits:`.
    let mut implicits_at = None;
    for (i, line) in raw.iter().enumerate().skip(at) {
        let line = *line;
        if let Some(key) = spec_key(line) {
            let value = spec_value(line, key);
            match key {
                "Implicits" => {
                    item.implicits = value.trim().parse().ok();
                    set(&mut kinds, i, spec_kind(key));
                    implicits_at = Some(i);
                    break;
                }
                "Variant" => item.variants.count += 1,
                "Selected Variant" => item.variants.selected = number(value),
                _ => {
                    for (n, suffix) in ["", " Two", " Three", " Four", " Five"].iter().enumerate() {
                        if let Some(slot) = item.variants.alt.get_mut(n) {
                            if key == format!("Has Alt Variant{suffix}") {
                                slot.0 = true;
                            } else if key == format!("Selected Alt Variant{suffix}") {
                                slot.1 = number(value);
                            }
                        }
                    }
                }
            }
            set(&mut kinds, i, spec_kind(key));
        } else if is_influence(line) {
            set(&mut kinds, i, LineKind::Influence);
        } else if is_flag(line) {
            set(&mut kinds, i, LineKind::Flag);
        } else {
            let (base, prefixes) = strip_prefixes(line);
            let variant = prefixes.iter().find(|p| p.key == "variant");
            let kind = match variant {
                Some(_) if prefixes.iter().all(|p| p.key == "variant") && !base.is_empty() => {
                    // Selection is set below, once every spec is read.
                    LineKind::VariantBase {
                        base: base.trim().to_owned(),
                        selected: false,
                    }
                }
                _ => LineKind::Unrecognized,
            };
            set(&mut kinds, i, kind);
        }
    }

    // Modifier lines.
    if let Some(start) = implicits_at {
        let implicits = item.implicits.unwrap_or(0);
        let mut counts = Counts::default();
        for (i, line) in raw.iter().enumerate().skip(start + 1) {
            let line = *line;
            let kind = if is_flag(line) {
                LineKind::Flag
            } else if is_influence(line) {
                LineKind::Influence
            } else {
                let mut m = read_modifier(line);
                m.section = section(&m, implicits, &mut counts);
                let list = m.value("variant").map(variant_list);
                m.selected = item.variants.includes(list.as_deref());
                LineKind::Modifier(m)
            };
            set(&mut kinds, i, kind);
        }
    }

    // Variant base selection, now that the selection is known.
    for (i, line) in raw.iter().enumerate() {
        if let Some(Some(LineKind::VariantBase { selected, .. })) = kinds.get_mut(i) {
            let (_, prefixes) = strip_prefixes(line);
            let list = prefixes
                .iter()
                .find(|p| p.key == "variant")
                .and_then(|p| p.value.as_deref())
                .map(variant_list);
            *selected = item.variants.includes(list.as_deref());
        }
    }

    item.lines = raw
        .iter()
        .zip(kinds)
        .map(|(raw, kind)| Line {
            raw: (*raw).to_owned(),
            kind: kind.unwrap_or(LineKind::Unrecognized),
        })
        .collect();
    item
}

fn spec_kind(key: &str) -> LineKind {
    LineKind::Spec {
        key: key.to_owned(),
        known: KNOWN_SPECS.contains(&key) || key == "Class:",
    }
}

/// The base name a base line names: `ParseRaw` removes `Superior `,
/// `Synthesised `, and `Vestigial ` at the start
/// (`line:gsub("^Superior ", ""):gsub("^Synthesised ", ""):gsub("^Vestigial ", "")`).
#[must_use]
pub fn base_name(line: &str) -> &str {
    let line = line.strip_prefix("Superior ").unwrap_or(line);
    let line = line.strip_prefix("Synthesised ").unwrap_or(line);
    line.strip_prefix("Vestigial ").unwrap_or(line)
}

/// Result of [`magic_base`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BaseMatch {
    /// One base.
    One(String),
    /// No base name is in the name.
    None,
    /// Several different base names of the longest length are in the name.
    Tie(Vec<String>),
}

/// The base of a normal or magic item, whose name line holds the base
/// between affixes (Spike B plan, "Base of an item"): after [`base_name`],
/// the name itself if it is a base, else the longest base name contained in
/// it. `ParseRaw` keeps the first longest match in `pairs` order, which is
/// not deterministic; a tie here is [`BaseMatch::Tie`].
#[must_use]
pub fn magic_base<'n>(name: &str, bases: impl IntoIterator<Item = &'n str>) -> BaseMatch {
    let name = base_name(name);
    let mut best: Vec<&str> = Vec::new();
    for base in bases {
        if base.is_empty() {
            continue;
        }
        if base == name {
            return BaseMatch::One(base.to_owned());
        }
        if !name.contains(base) {
            continue;
        }
        match best.first().map(|b| b.len()) {
            Some(len) if base.len() < len => {}
            Some(len) if base.len() == len => {
                if !best.contains(&base) {
                    best.push(base);
                }
            }
            _ => best = vec![base],
        }
    }
    match best.as_slice() {
        [] => BaseMatch::None,
        [one] => BaseMatch::One((*one).to_owned()),
        many => {
            let mut names: Vec<String> = many.iter().map(|b| (*b).to_owned()).collect();
            names.sort();
            BaseMatch::Tie(names)
        }
    }
}
