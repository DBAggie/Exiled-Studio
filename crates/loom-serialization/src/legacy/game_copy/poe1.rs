//! PoE1's item copy → PoB item text (`v2.67.2` of `PathOfBuilding`).
//!
//! The copy is cut into sections at `--------`. `ParseRaw` walks the copy in
//! stages and finds base, flask, and tincture lines by its item data; the
//! converter has no item data, so it tells the sections apart by what they
//! hold, in the copy's order:
//! - **Header:** `Item Class:`, `Rarity:`, the name, and for rare and unique
//!   items the base (`ParseRaw`'s header). A rarity other than normal, magic,
//!   rare, or unique (gems, currency, cards) is refused: [`CopyError`].
//! - **Properties,** every section up to the one holding `Item Level:`:
//!   - spec lines `ParseRaw` reads are kept, their `(augmented)` mark and
//!     `+`/`%` signs removed so the reader's number parse takes them:
//!     `Quality`, `Armour`, `Evasion Rating`, `Energy Shield`, `Ward`,
//!     `Sockets`, `Item Level`, `Radius`, `Limited to` (`Intangibility` is
//!     kept with its `%`, as `BuildRaw` writes it);
//!   - `Quality (<descriptor> Modifiers)` becomes `Catalyst:` and
//!     `CatalystQuality:` (`ParseRaw`'s `catalystDescriptorList`);
//!   - the weapon and block values `ParseRaw` sets `hidden_specs` for are
//!     dropped ([`Dropped::ShownValue`]): the calculation computes them;
//!   - any other property line (the item class word, a flask's effect and
//!     charges) is dropped as [`Dropped::Property`]. `ParseRaw` drops a
//!     flask's lines by its base data; the converter drops the section's
//!     non-spec lines.
//! - **Requirements:** `ParseRaw` reads `Level:` as `importedLevelReq` and
//!   `Str`, `Dex`, `Int` as the requirements, and uses them only for an item
//!   without sockets (the copy's totals include socketed gems'). The
//!   converter writes them as `LevelReq`, `Str`, `Dex`, `Int` only then.
//! - **Modifier sections:** a `{ ... }` header (the advanced copy) sets the
//!   prefixes of the lines after it, as `ParseRaw` does: the header's words
//!   before `Modifier` that are line flags (`Foulborn` read as `Mutated`),
//!   then `{tags:...}` from the header's tags, lower-cased without spaces.
//!   Reminder lines (`^%(%a+` or `^%(%d+%%? of `, to a line ending `)`) are
//!   dropped. Values follow `ParseRaw`: `v(base)` becomes `v`; a named
//!   prefix or suffix keeps its rolled values (`ParseRaw` records the affix
//!   and its roll separately); any other `v(min-max)` becomes `(min-max)`
//!   with `{range:r}` when every value of the line rolled the same fraction
//!   and the fraction lies in `[0, 1]`, else the rolled value; enum ranges
//!   without digits are removed.
//! - Lines ending ` (enchant)` and, in the plain copy, ` (implicit)` are
//!   kept: the reader reads the flag words.
//! - `Corrupted`, `Fractured Item`, and the other flag lines, and
//!   `<Influence> Item`, wherever they are, are kept and written before
//!   `Implicits:`.
//! - **Text after the modifiers:** a section starting with a usage hint
//!   (`Place into`, `Right click`, `Travel to`) is dropped
//!   ([`Dropped::UsageHint`]); any other is flavour text, returned in
//!   [`Copy::flavour`] and dropped from the item text ([`Dropped::Flavour`]:
//!   the card reads unique flavour from the data).
//! - The plain copy (Ctrl+C, no headers): the first section after
//!   `Item Level:` that is not enchants or implicits holds the explicit
//!   modifiers, and later text is flavour, as `ParseRaw`'s stages read it.
//!
//! Each line is first cleaned as `ParseRaw` cleans the whole copy:
//! `sanitiseText` (dashes and the minus sign become `-`, `ä` and `ö` lose
//! their marks, `<...>` is removed) and `escapeGGGString`.
//!
//! Differs from PoB: a named affix PoB cannot find in its data gets `{custom}`
//! there and its range form; the converter, without data, keeps its values
//! (the game wrote them). PoB records each named affix (`Prefix:` and
//! `Suffix:` lines) for its crafting window; the converter does not.

use std::fmt::Write as _;

use crate::legacy::pob_item::{FLAG_LINES, INFLUENCES, LINE_FLAGS};

/// Why a copy cannot be read as an item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CopyError {
    /// No non-blank line.
    Empty,
    /// No `Rarity:` line in the header: not a game copy.
    NoRarity,
    /// A rarity that is not an item's (`Gem`, `Currency`, `Divination Card`).
    NotAnItem(String),
}

impl std::fmt::Display for CopyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => f.write_str("the text is empty"),
            Self::NoRarity => f.write_str("the text has no Rarity line"),
            Self::NotAnItem(r) => write!(f, "a {r} is not an item"),
        }
    }
}

impl std::error::Error for CopyError {}

/// The rule that dropped a line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Dropped {
    /// `--------`.
    Separator,
    /// A weapon or block value the calculation computes.
    ShownValue,
    /// Another property line (item class word, a flask's effect or charges).
    Property,
    /// `Requirements:` and its lines, or the requirements of a socketed item.
    Requirements,
    /// A `{ ... }` modifier header.
    ModHeader,
    /// Reminder text in parentheses.
    Reminder,
    /// Flavour text.
    Flavour,
    /// The usage hint (`Place into ...`, `Right click to ...`).
    UsageHint,
}

impl Dropped {
    /// Stable name for reports.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Separator => "separator",
            Self::ShownValue => "shown-value",
            Self::Property => "property",
            Self::Requirements => "requirements",
            Self::ModHeader => "mod-header",
            Self::Reminder => "reminder",
            Self::Flavour => "flavour",
            Self::UsageHint => "usage-hint",
        }
    }
}

/// What became of one line of the copy.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// Written to the item text as it is (trimmed).
    Kept,
    /// Written to the item text as this (one or more lines).
    Rewritten(String),
    /// Not written, by this rule.
    Dropped(Dropped),
}

/// One non-blank line of the copy and its outcome.
#[derive(Debug, Clone, PartialEq)]
pub struct CopyLine {
    /// 1-based line number in the copy.
    pub number: usize,
    /// The line, trimmed.
    pub raw: String,
    /// What became of it.
    pub outcome: Outcome,
}

/// A `{ ... }` modifier header of the advanced copy and the lines under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModHeader {
    /// The words before `Modifier` (`Prefix`, `Master Crafted Suffix`,
    /// `Searing Exarch Implicit`, `Unique`).
    pub kind: String,
    /// The affix name in quotes, if any.
    pub name: Option<String>,
    /// `(Tier: n)`.
    pub tier: Option<u32>,
    /// `(Rank: n)` (crafted modifiers).
    pub rank: Option<u32>,
    /// The tags as the game names them.
    pub tags: Vec<String>,
    /// The modifier lines under the header, as written to the item text
    /// without their prefixes.
    pub lines: Vec<String>,
}

/// A converted copy.
#[derive(Debug, Clone, PartialEq)]
pub struct Copy {
    /// PoB item text.
    pub text: String,
    /// Every non-blank line of the copy, in order.
    pub lines: Vec<CopyLine>,
    /// The flavour text's lines, as the copy wrote them.
    pub flavour: Vec<String>,
    /// The modifier headers, in order (advanced copy only).
    pub headers: Vec<ModHeader>,
}

impl Copy {
    /// How many lines each rule dropped.
    #[must_use]
    pub fn dropped(&self) -> std::collections::BTreeMap<&'static str, usize> {
        let mut counts = std::collections::BTreeMap::new();
        for l in &self.lines {
            if let Outcome::Dropped(d) = l.outcome {
                *counts.entry(d.name()).or_insert(0) += 1;
            }
        }
        counts
    }
}

/// Whether the text looks like a game copy rather than PoB's item text:
/// a `Rarity:` line in its first two lines and a `--------` separator.
#[must_use]
pub fn is_game_copy(text: &str) -> bool {
    let mut lines = text.lines().map(str::trim).filter(|l| !l.is_empty());
    let first_two: Vec<&str> = lines.by_ref().take(2).collect();
    first_two.iter().any(|l| l.starts_with("Rarity: ")) && text.contains("--------")
}

const SEPARATOR: &str = "--------";

/// `ParseRaw`'s `catalystList` and `catalystDescriptorList`.
const CATALYSTS: [(&str, &str); 12] = [
    ("Attack", "Abrasive"),
    ("Speed", "Accelerating"),
    ("Suffix", "Dextral"),
    ("Life and Mana", "Fertile"),
    ("Caster", "Imbued"),
    ("Attribute", "Intrinsic"),
    ("Physical and Chaos", "Noxious"),
    ("Resistance", "Prismatic"),
    ("Prefix", "Sinistral"),
    ("Defence", "Tempering"),
    ("Elemental", "Turbulent"),
    ("Critical", "Unstable"),
];

/// Spec keys whose value the converter keeps as a number.
const NUMBER_SPECS: [&str; 7] = [
    "Quality",
    "Armour",
    "Evasion Rating",
    "Energy Shield",
    "Ward",
    "Item Level",
    "Limited to",
];

/// The values `ParseRaw` marks `hidden_specs` for (weapon and block values).
const SHOWN_VALUES: [&str; 10] = [
    "Physical Damage",
    "Elemental Damage",
    "Chaos Damage",
    "Critical Strike Chance",
    "Critical Strike Range",
    "Attacks per Second",
    "Weapon Range",
    "Chance to Block",
    "Block chance",
    "Attack Time",
];

const USAGE_HINTS: [&str; 4] = ["Place into", "Right click", "Travel to", "Use this"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Part {
    Spec,
    Requirement,
    Flag,
    Enchant,
    Implicit,
    Explicit,
}

/// Converts one game copy.
///
/// # Errors
/// [`CopyError`] when the text is not an item's copy.
pub fn convert(text: &str) -> Result<Copy, CopyError> {
    let numbered: Vec<(usize, String)> = text
        .split('\n')
        .enumerate()
        .map(|(i, l)| (i + 1, l.trim().to_owned()))
        .filter(|(_, l)| !l.is_empty())
        .collect();
    if numbered.is_empty() {
        return Err(CopyError::Empty);
    }
    let mut sections: Vec<Vec<(usize, String)>> = vec![Vec::new()];
    let mut separators = Vec::new();
    for (n, l) in numbered {
        if l == SEPARATOR {
            separators.push((n, l));
            sections.push(Vec::new());
        } else if let Some(last) = sections.last_mut() {
            last.push((n, l));
        }
    }
    let mut c = Converter::default();
    for (n, l) in separators {
        c.drop(n, &l, Dropped::Separator);
    }
    let mut rest = sections.into_iter();
    c.header(rest.next().unwrap_or_default())?;
    let rest: Vec<_> = rest.filter(|s| !s.is_empty()).collect();
    c.has_sockets = rest.iter().flatten().any(|(_, l)| {
        l.strip_prefix("Sockets:")
            .is_some_and(|s| !s.trim().is_empty())
    });
    c.advanced = rest.iter().flatten().any(|(_, l)| is_header(l));
    let mut properties = true;
    for section in &rest {
        c.section(section, &mut properties);
    }
    Ok(c.finish())
}

#[derive(Default)]
struct Converter {
    head: Vec<String>,
    out: Vec<(Part, String)>,
    lines: Vec<CopyLine>,
    flavour: Vec<String>,
    headers: Vec<ModHeader>,
    has_sockets: bool,
    advanced: bool,
    seen_mods: bool,
    unique: bool,
}

impl Converter {
    fn drop(&mut self, number: usize, raw: &str, rule: Dropped) {
        self.lines.push(CopyLine {
            number,
            raw: raw.to_owned(),
            outcome: Outcome::Dropped(rule),
        });
    }

    fn write(&mut self, number: usize, raw: &str, part: Part, text: String) {
        let outcome = if text == raw {
            Outcome::Kept
        } else {
            Outcome::Rewritten(text.clone())
        };
        self.lines.push(CopyLine {
            number,
            raw: raw.to_owned(),
            outcome,
        });
        match part {
            Part::Spec | Part::Requirement => self.head.push(text),
            _ => self.out.push((part, text)),
        }
    }

    fn header(&mut self, section: Vec<(usize, String)>) -> Result<(), CopyError> {
        let mut iter = section.into_iter().peekable();
        let mut first = Vec::new();
        if let Some((n, l)) = iter.next_if(|(_, l)| l.starts_with("Item Class:")) {
            first.push((n, l));
        }
        let Some((n, rarity_line)) = iter.next_if(|(_, l)| l.starts_with("Rarity: ")) else {
            return Err(if first.is_empty() && iter.peek().is_none() {
                CopyError::Empty
            } else {
                CopyError::NoRarity
            });
        };
        let word = rarity_line.trim_start_matches("Rarity: ").trim().to_owned();
        let upper = word.to_ascii_uppercase();
        if !matches!(upper.as_str(), "NORMAL" | "MAGIC" | "RARE" | "UNIQUE") {
            return Err(CopyError::NotAnItem(word));
        }
        self.unique = upper == "UNIQUE";
        for (n, l) in first {
            self.write(n, &l.clone(), Part::Spec, l);
        }
        self.write(n, &rarity_line, Part::Spec, format!("Rarity: {upper}"));
        for (n, l) in iter {
            // The name, then the base: `ParseRaw` reads both from here.
            let clean = clean(&l);
            self.write(n, &l, Part::Spec, clean);
        }
        Ok(())
    }

    fn section(&mut self, section: &[(usize, String)], properties: &mut bool) {
        let first = section.first().map_or("", |(_, l)| l.as_str());
        if first == "Requirements:" {
            self.requirements(section);
            return;
        }
        if section.iter().all(|(_, l)| is_flag(l)) {
            for (n, l) in section {
                self.write(*n, l, Part::Flag, l.clone());
            }
            return;
        }
        let headed = section.iter().any(|(_, l)| is_header(l));
        let enchants = !headed && marked_all(section, " (enchant)");
        let implicits = !headed && marked_all(section, " (implicit)");
        if *properties && !headed && !enchants {
            if section.iter().any(|(_, l)| l.starts_with("Item Level:")) {
                *properties = false;
            }
            for (n, l) in section {
                self.property(*n, l);
            }
            return;
        }
        *properties = false;
        if headed {
            self.seen_mods = true;
            self.mods(section, Part::Explicit);
        } else if enchants {
            self.mods(section, Part::Enchant);
        } else if implicits {
            self.mods(section, Part::Implicit);
        } else if USAGE_HINTS.iter().any(|h| first.starts_with(h)) {
            for (n, l) in section {
                self.drop(*n, l, Dropped::UsageHint);
            }
        } else if !self.advanced && !self.seen_mods {
            self.seen_mods = true;
            self.mods(section, Part::Explicit);
        } else {
            for (n, l) in section {
                if is_flag(l) {
                    self.write(*n, l, Part::Flag, l.clone());
                } else {
                    self.flavour.push(l.clone());
                    self.drop(*n, l, Dropped::Flavour);
                }
            }
        }
    }

    fn property(&mut self, n: usize, raw: &str) {
        if is_flag(raw) {
            self.write(n, raw, Part::Flag, raw.to_owned());
            return;
        }
        let line = strip_marks(raw);
        let Some((key, value)) = line.split_once(": ") else {
            self.drop(n, raw, Dropped::Property);
            return;
        };
        if SHOWN_VALUES.contains(&key) {
            self.drop(n, raw, Dropped::ShownValue);
        } else if let Some(descriptor) = key
            .strip_prefix("Quality (")
            .and_then(|k| k.strip_suffix(" Modifiers)"))
        {
            match CATALYSTS.iter().find(|(d, _)| *d == descriptor) {
                Some((_, catalyst)) => {
                    let quality = leading_number(value).unwrap_or_default();
                    self.write(
                        n,
                        raw,
                        Part::Spec,
                        format!("Catalyst: {catalyst}\nCatalystQuality: {quality}"),
                    );
                }
                None => self.write(n, raw, Part::Spec, line.clone()),
            }
        } else if NUMBER_SPECS.contains(&key) {
            let text = match leading_number(value) {
                Some(v) => format!("{key}: {v}"),
                None => line.clone(),
            };
            self.write(n, raw, Part::Spec, text);
        } else if matches!(key, "Sockets" | "Radius" | "Intangibility") {
            self.write(n, raw, Part::Spec, line.clone());
        } else {
            self.drop(n, raw, Dropped::Property);
        }
    }

    fn requirements(&mut self, section: &[(usize, String)]) {
        for (n, raw) in section {
            let line = strip_marks(raw);
            let spec = line.split_once(": ").and_then(|(k, v)| {
                let key = match k {
                    "Level" => "LevelReq",
                    "Str" | "Strength" => "Str",
                    "Dex" | "Dexterity" => "Dex",
                    "Int" | "Intelligence" => "Int",
                    _ => return None,
                };
                leading_number(v).map(|v| format!("{key}: {v}"))
            });
            match spec {
                Some(text) if !self.has_sockets => self.write(*n, raw, Part::Requirement, text),
                _ => self.drop(*n, raw, Dropped::Requirements),
            }
        }
    }

    fn mods(&mut self, section: &[(usize, String)], part: Part) {
        let mut prefix = String::new();
        let mut named = false;
        let mut current = part;
        let mut reminder = false;
        let mut under_header = false;
        for (n, raw) in section {
            if reminder {
                reminder = !raw.ends_with(')');
                self.drop(*n, raw, Dropped::Reminder);
                continue;
            }
            if is_flag(raw) {
                self.write(*n, raw, Part::Flag, raw.clone());
                continue;
            }
            let line = clean(raw);
            if is_header(&line) {
                let header = read_header(&line);
                prefix = header.prefix.clone();
                named = header.named;
                current = if header.flags.iter().any(|f| f == "implicit") {
                    Part::Implicit
                } else {
                    part
                };
                self.headers.push(header.info);
                under_header = true;
                self.drop(*n, raw, Dropped::ModHeader);
                continue;
            }
            if is_reminder(&line) {
                reminder = !line.ends_with(')');
                self.drop(*n, raw, Dropped::Reminder);
                continue;
            }
            let text = values(&line, named);
            if under_header && let Some(h) = self.headers.last_mut() {
                h.lines.push(text.clone());
            }
            let part = if line.ends_with(" (enchant)") {
                Part::Enchant
            } else if line.ends_with(" (implicit)") {
                Part::Implicit
            } else {
                current
            };
            self.write(*n, raw, part, format!("{prefix}{text}"));
        }
    }

    fn finish(self) -> Copy {
        let mut text = String::new();
        for l in &self.head {
            text.push_str(l);
            text.push('\n');
        }
        for (part, l) in &self.out {
            if *part == Part::Flag {
                text.push_str(l);
                text.push('\n');
            }
        }
        let implicits = self
            .out
            .iter()
            .filter(|(p, _)| matches!(p, Part::Enchant | Part::Implicit))
            .count();
        let _ = writeln!(text, "Implicits: {implicits}");
        for wanted in [Part::Enchant, Part::Implicit, Part::Explicit] {
            for (part, l) in &self.out {
                if *part == wanted {
                    text.push_str(l);
                    text.push('\n');
                }
            }
        }
        let mut lines = self.lines;
        lines.sort_by_key(|l| l.number);
        Copy {
            text,
            lines,
            flavour: self.flavour,
            headers: self.headers,
        }
    }
}

fn is_header(line: &str) -> bool {
    line.starts_with("{ ") && line.ends_with('}')
}

fn is_flag(line: &str) -> bool {
    FLAG_LINES.contains(&line)
        || line.contains("Foil Unique")
        || line
            .strip_suffix(" Item")
            .is_some_and(|d| INFLUENCES.contains(&d))
}

/// `ParseRaw`'s reminder test: `^%(%a+` or `^%(%d+%%? of `.
fn is_reminder(line: &str) -> bool {
    let Some(rest) = line.strip_prefix('(') else {
        return false;
    };
    if rest.starts_with(|c: char| c.is_ascii_alphabetic()) {
        return true;
    }
    let digits = rest.trim_start_matches(|c: char| c.is_ascii_digit());
    digits.len() < rest.len() && digits.trim_start_matches('%').starts_with(" of ")
}

/// Whether every line that is not a reminder ends with `mark`.
fn marked_all(section: &[(usize, String)], mark: &str) -> bool {
    let mut any = false;
    for (_, l) in section {
        if l.starts_with('(') {
            continue;
        }
        if !l.ends_with(mark) {
            return false;
        }
        any = true;
    }
    any
}

/// Removes the copy's ` (augmented)` and ` (unmet)` marks.
fn strip_marks(line: &str) -> String {
    line.replace(" (augmented)", "").replace(" (unmet)", "")
}

/// The first number in a value (`specToNumber`), without its sign.
fn leading_number(value: &str) -> Option<String> {
    let start = value.find(|c: char| c.is_ascii_digit())?;
    let tail = value.get(start..)?;
    let end = tail
        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
        .unwrap_or(tail.len());
    let number = tail.get(..end)?.trim_end_matches('.');
    Some(number.to_owned())
}

/// `sanitiseText` and `escapeGGGString` on one line.
fn clean(line: &str) -> String {
    let mut out = line.to_owned();
    if !line.is_ascii() || line.contains('<') {
        out = remove_angle_groups(&out);
        out = out
            .chars()
            .map(|c| match c {
                '\u{2010}'..='\u{2015}' | '\u{2212}' => '-',
                '\u{e4}' => 'a',
                '\u{f6}' => 'o',
                c => c,
            })
            .collect();
    }
    escape_ggg(&out)
}

/// Lua `gsub("%b<>", "")`.
fn remove_angle_groups(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut depth = 0usize;
    let mut held = String::new();
    for c in line.chars() {
        match c {
            '<' => {
                depth += 1;
                held.push(c);
            }
            '>' if depth > 0 => {
                depth -= 1;
                held.push(c);
                if depth == 0 {
                    held.clear();
                }
            }
            c if depth > 0 => held.push(c),
            c => out.push(c),
        }
    }
    out.push_str(&held);
    out
}

/// `escapeGGGString`: `[a|b]` becomes `b`, `[a]` becomes `a`.
fn escape_ggg(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(open) = rest.find('[') {
        let (before, from) = rest.split_at(open);
        out.push_str(before);
        match from.find(']') {
            Some(close) => {
                let inner = from.get(1..close).unwrap_or("");
                out.push_str(inner.rsplit('|').next().unwrap_or(inner));
                rest = from.get(close + 1..).unwrap_or("");
            }
            None => {
                out.push_str(from);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

struct Header {
    prefix: String,
    flags: Vec<String>,
    named: bool,
    info: ModHeader,
}

/// `ParseRaw`'s advanced copy header (`^{ (.-) %- (.-)  %- (%d*).*}$`, then
/// `^{ (.-) %- (.-) }$`, then `^{ (.-) }$`).
fn read_header(line: &str) -> Header {
    let inner = line
        .strip_prefix("{ ")
        .and_then(|l| l.strip_suffix('}'))
        .unwrap_or(line)
        .trim_end();
    let (full, tags) = match inner.split_once(" - ") {
        Some((full, rest)) => (full, rest.split("  - ").next().unwrap_or(rest).trim()),
        None => (inner, ""),
    };
    let name = full
        .find("Modifier \"")
        .and_then(|at| full.get(at + "Modifier \"".len()..))
        .and_then(|r| r.rfind('"').and_then(|end| r.get(..end)))
        .filter(|n| !n.is_empty())
        .map(str::to_owned);
    let kind = full
        .rfind("Modifier")
        .map_or(full, |at| full.get(..at).unwrap_or(full));
    let flags: Vec<String> = kind
        .replace("Foulborn", "Mutated")
        .split(|c: char| !c.is_ascii_alphabetic())
        .map(str::to_ascii_lowercase)
        .filter(|w| LINE_FLAGS.contains(&w.as_str()))
        .collect();
    let mut prefix: String = flags.iter().map(|f| format!("{{{f}}}")).collect();
    if !tags.is_empty() {
        let compact: String = tags
            .to_ascii_lowercase()
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        let _ = write!(prefix, "{{tags:{compact}}}");
    }
    let number_after = |label: &str| {
        full.find(label)
            .and_then(|at| full.get(at + label.len()..))
            .and_then(|r| r.split(')').next())
            .and_then(|v| v.trim().parse().ok())
    };
    let kind = kind.trim().to_owned();
    let named = name.is_some() && (kind.contains("Prefix") || kind.contains("Suffix"));
    Header {
        prefix,
        flags,
        named,
        info: ModHeader {
            kind,
            name,
            tier: number_after("(Tier:"),
            rank: number_after("(Rank:"),
            tags: tags
                .split(',')
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .map(str::to_owned)
                .collect(),
            lines: Vec::new(),
        },
    }
}

/// A Lua `%-?%d+%.?%d*` number starting at `at`; returns its end.
fn number_end(s: &[u8], at: usize) -> Option<usize> {
    let mut i = at;
    if s.get(i) == Some(&b'-') {
        i += 1;
    }
    let digits = i;
    while s.get(i).is_some_and(u8::is_ascii_digit) {
        i += 1;
    }
    if i == digits {
        return None;
    }
    if s.get(i) == Some(&b'.') {
        i += 1;
        while s.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
    }
    Some(i)
}

/// A rolled value: `value(min-max)` or `value(base)`.
struct Roll {
    start: usize,
    end: usize,
    value: String,
    range: Option<(String, String)>,
}

/// Every `value(min-max)` (with `ranged`) or `value(base)` in the line,
/// leftmost first, as Lua's `gmatch` finds them.
fn rolls(line: &str, ranged: bool) -> Vec<Roll> {
    let s = line.as_bytes();
    let mut found = Vec::new();
    let mut at = 0;
    while at < s.len() {
        if let Some(roll) = roll_at(line, s, at, ranged) {
            at = roll.end;
            found.push(roll);
        } else {
            at += 1;
        }
    }
    found
}

fn roll_at(line: &str, s: &[u8], at: usize, ranged: bool) -> Option<Roll> {
    let value_end = number_end(s, at)?;
    if s.get(value_end) != Some(&b'(') {
        return None;
    }
    let first_end = number_end(s, value_end + 1)?;
    let (range, close) = if ranged {
        if s.get(first_end) != Some(&b'-') {
            return None;
        }
        let second_end = number_end(s, first_end + 1)?;
        let min = line.get(value_end + 1..first_end)?.to_owned();
        let max = line.get(first_end + 1..second_end)?.to_owned();
        (Some((min, max)), second_end)
    } else {
        (None, first_end)
    };
    if s.get(close) != Some(&b')') {
        return None;
    }
    Some(Roll {
        start: at,
        end: close + 1,
        value: line.get(at..value_end)?.to_owned(),
        range,
    })
}

/// Replaces each roll by what `f` returns.
fn replace_rolls(line: &str, rolls: &[Roll], mut f: impl FnMut(&Roll) -> String) -> String {
    let mut out = String::with_capacity(line.len());
    let mut last = 0;
    for r in rolls {
        out.push_str(line.get(last..r.start).unwrap_or(""));
        out.push_str(&f(r));
        last = r.end;
    }
    out.push_str(line.get(last..).unwrap_or(""));
    out
}

/// Lua's `round(x, 6)` printed as Lua prints it.
fn fraction(x: f64) -> String {
    let r = (x * 1e6).round() / 1e6;
    format!("{r}")
}

/// `ParseRaw`'s value handling for one modifier line (see the module docs).
fn values(line: &str, named: bool) -> String {
    let (body, unscalable) = match line.strip_suffix(" - Unscalable Value") {
        Some(b) => (b, true),
        None => (line, false),
    };
    // `current(base)` for fixed values.
    let fixed = rolls(body, false);
    let mut text = replace_rolls(body, &fixed, |r| r.value.clone());
    let ranged = rolls(&text, true);
    if named {
        text = replace_rolls(&text, &ranged, |r| r.value.clone());
    } else {
        text = remove_enum_ranges(&text);
        let ranged = rolls(&text, true);
        let mut fractions = Vec::new();
        let mut best_delta = -1.0_f64;
        let mut best = -1.0_f64;
        let mut keep_values = Vec::new();
        for r in &ranged {
            let (min, max) = r.range.clone().unwrap_or_default();
            let (lo, hi): (f64, f64) = (min.parse().unwrap_or(0.0), max.parse().unwrap_or(0.0));
            let (lo, hi, min, max) = if lo > hi {
                (hi, lo, max, min)
            } else {
                (lo, hi, min, max)
            };
            let value: f64 = r.value.parse().unwrap_or(0.0);
            let delta = hi - lo;
            let roll = if delta > 0.0 {
                ((value - lo) / delta * 1e6).round() / 1e6
            } else {
                0.5
            };
            fractions.push(roll);
            if delta > best_delta {
                best = roll;
                best_delta = delta;
            }
            keep_values.push(if (0.0..=1.0).contains(&best) {
                format!("{}({min}-{max})", if value < 0.0 { "+" } else { "" })
            } else {
                r.value.clone()
            });
        }
        let independent = fractions.windows(2).any(|w| w.first() != w.get(1));
        if independent {
            text = replace_rolls(&text, &ranged, |r| r.value.clone());
        } else if !ranged.is_empty() {
            let mut i = 0;
            let replaced = replace_rolls(&text, &ranged, |_| {
                let s = keep_values.get(i).cloned().unwrap_or_default();
                i += 1;
                s
            });
            text = if (0.0..=1.0).contains(&best) {
                format!("{{range:{}}}{replaced}", fraction(best))
            } else {
                replaced
            };
        }
    }
    if unscalable {
        text.push_str(" - Unscalable Value");
    }
    text
}

/// `line:gsub("(%s*)(%b())", ...)` removing a balanced group with a `-` and
/// no digit, with the space before it.
fn remove_enum_ranges(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::with_capacity(line.len());
    let mut i = 0;
    while i < chars.len() {
        let space_start = i;
        let mut j = i;
        while chars.get(j).is_some_and(|c| c.is_whitespace()) {
            j += 1;
        }
        if chars.get(j) == Some(&'(') {
            let mut depth = 0;
            let mut k = j;
            let mut close = None;
            while let Some(c) = chars.get(k) {
                if *c == '(' {
                    depth += 1;
                } else if *c == ')' {
                    depth -= 1;
                    if depth == 0 {
                        close = Some(k);
                        break;
                    }
                }
                k += 1;
            }
            if let Some(close) = close {
                let group: String = chars.get(j..=close).unwrap_or(&[]).iter().collect();
                if group.contains('-') && !group.chars().any(|c| c.is_ascii_digit()) {
                    i = close + 1;
                    continue;
                }
                let kept: String = chars
                    .get(space_start..=close)
                    .unwrap_or(&[])
                    .iter()
                    .collect();
                out.push_str(&kept);
                i = close + 1;
                continue;
            }
        }
        if let Some(c) = chars.get(space_start) {
            out.push(*c);
        }
        i = space_start + 1;
    }
    out
}

#[cfg(test)]
mod tests;
