//! Stat-description files (Spike A, PR A5): the text files that turn stat IDs
//! and values into the lines shown in game. PoE1 keeps them in
//! `metadata/statdescriptions/*.txt`, PoE2 in `data/statdescriptions/**/*.csd`;
//! both are UTF-16LE with the same grammar (observed on both installs, spike
//! report section 12). Pure: callers pass bytes.
//!
//! Grammar, one construct per non-blank line (indentation is not reliable,
//! so it is ignored):
//! ```text
//! include "Metadata/StatDescriptions/stat_descriptions.txt"
//! no_identifiers | has_identifiers
//! no_description <stat_id>
//! description [<name>]
//!     <n> <stat_id> x n
//!     <m>                                       English variants
//!     <condition> x n [flag ...] "<format>" [handler ...]   (m lines)
//!     lang "<Language>"
//!     <m>
//!     ...                                       (m lines, counted only)
//! handed_description [<name>]
//!     <n> <main-hand stat_id> x n <off-hand stat_id> x n   (may span lines)
//!     ...variants as above, with n conditions
//! ```
//! Conditions: `#` (any), `<v>`, `<a>|<b>` (either side may be `#`), `!<v>`.
//! Flags: [`VARIANT_FLAGS`]. Handlers: `<name> <stat>` (stat is 1-based),
//! `reminderstring <id>`, or one of [`BARE_HANDLERS`]. The format string
//! uses `{<index>[:<format>]}` placeholders (index 0-based, `{}` means
//! stat 0), `{{` and `}}` for literal braces, and `\n` for a line break;
//! markup (`<tag>{...}`, `[Ref|Text]`, `<<Icon>>`) is display text and is
//! kept as written.
//!
//! Only English is parsed (the variant list before the first `lang`); other
//! languages are counted per description. Nothing is dropped silently
//! (ADR-015): a line that does not fit the grammar becomes a [`Finding`]
//! with a [`FindingKind`] and the line text, and the run reports it.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::extract::{ExtractError, TableError};
use crate::install::hex;
use crate::schema::Game;

/// Words allowed between a variant's conditions and its format string.
pub const VARIANT_FLAGS: [&str; 2] = ["gem_quality", "table_only"];

/// Handlers that take no argument.
pub const BARE_HANDLERS: [&str; 2] = ["canonical_line", "markup"];

/// The handler whose argument is a reminder-text ID, not a stat.
pub const REMINDER_HANDLER: &str = "reminderstring";

/// Folder of the output files inside the output directory.
pub const STAT_DESCRIPTIONS_DIR: &str = "stat_descriptions";

/// Keywords that start a top-level construct.
const TOP_KEYWORDS: [&str; 6] = [
    "description",
    "handed_description",
    "include",
    "no_description",
    "no_identifiers",
    "has_identifiers",
];

/// Keyword that starts another language's variant list.
const LANG: &str = "lang";

/// Folder of the game's stat-description files in the index (lowercase).
#[must_use]
pub const fn stat_desc_dir(game: Game) -> &'static str {
    match game {
        Game::Poe1 => "metadata/statdescriptions/",
        Game::Poe2 => "data/statdescriptions/",
    }
}

/// What a file in the stat-description folder holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SourceKind {
    /// Stat descriptions: parsed.
    Descriptions,
    /// A `*_filters` file (PoE1 `skillpopup_stat_filters.txt`): `group`
    /// lists for the skill pop-up, a different format. Hashed and listed,
    /// not parsed.
    Filters,
}

/// The kind of `path` (as listed in the index) if it is a `.txt` or `.csd`
/// file in the game's stat-description folder or below.
#[must_use]
pub fn source_kind(game: Game, path: &str) -> Option<SourceKind> {
    let path = path.to_ascii_lowercase();
    let rest = path.strip_prefix(stat_desc_dir(game))?;
    let stem = rest
        .strip_suffix(".txt")
        .or_else(|| rest.strip_suffix(".csd"))?;
    Some(if stem.ends_with("_filters") {
        SourceKind::Filters
    } else {
        SourceKind::Descriptions
    })
}

/// Errors that stop a whole file from being read. Each variant has a stable
/// [`code`](Self::code). Problems inside the text are [`Finding`]s instead.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StatDescError {
    /// UTF-16 needs an even number of bytes.
    #[error("{len} bytes is not a whole number of UTF-16 units")]
    OddLength {
        /// File size.
        len: usize,
    },
    /// An unpaired surrogate.
    #[error("invalid UTF-16 at byte {offset}")]
    BadUtf16 {
        /// Byte offset of the bad unit.
        offset: usize,
    },
}

impl StatDescError {
    /// Stable machine-readable code (ADR-014).
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::OddLength { .. } => "EXTRACT_STATDESC_ODD_LENGTH",
            Self::BadUtf16 { .. } => "EXTRACT_STATDESC_BAD_UTF16",
        }
    }
}

/// Decodes UTF-16LE text and drops a leading byte-order mark.
///
/// # Errors
/// [`StatDescError`] for an odd byte count or an unpaired surrogate.
pub fn decode_utf16le(bytes: &[u8]) -> Result<String, StatDescError> {
    let (units, odd) = bytes.as_chunks::<2>();
    if !odd.is_empty() {
        return Err(StatDescError::OddLength { len: bytes.len() });
    }
    let mut text = String::with_capacity(units.len());
    let mut unit = 0_usize;
    for c in char::decode_utf16(units.iter().map(|u| u16::from_le_bytes(*u))) {
        let c = c.map_err(|_| StatDescError::BadUtf16 { offset: unit * 2 })?;
        text.push(c);
        unit += c.len_utf16();
    }
    if text.starts_with('\u{feff}') {
        text.drain(..'\u{feff}'.len_utf8());
    }
    Ok(text)
}

/// Why a line was not parsed. The list is fixed; each kind is one category
/// for Spike A gate 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FindingKind {
    /// A top-level line that is no known construct.
    UnknownLine,
    /// A `"` without a closing `"` on the same line.
    UnterminatedString,
    /// `include` without exactly one quoted path.
    BadInclude,
    /// An include names a file that is not in the stat-description folder.
    MissingInclude,
    /// An include names a file that was not parsed (failed or skipped).
    IncludeNotParsed,
    /// An include leads back to the including file.
    IncludeCycle,
    /// `no_description` without exactly one stat ID.
    BadNoDescription,
    /// A `description` line with more than a name after the keyword.
    BadDescriptionHeader,
    /// The stat list is missing, has a bad count, or has too few or too
    /// many IDs.
    BadStatList,
    /// A variant count is missing or not a number, or fewer variant lines
    /// follow than it says.
    BadVariantCount,
    /// `lang` without exactly one quoted language name.
    BadLanguage,
    /// A condition that is not `#`, `<v>`, `<a>|<b>`, or `!<v>`.
    BadCondition,
    /// Fewer or more conditions than the description has stats.
    ConditionCount,
    /// A word between the conditions and the format string that is not in
    /// [`VARIANT_FLAGS`].
    UnknownVariantFlag,
    /// A variant line without a format string.
    MissingString,
    /// A handler without its argument, or with an argument of the wrong
    /// form.
    BadHandler,
    /// A handler's stat number is 0 or past the description's stats.
    HandlerStatOutOfRange,
    /// A `{...}` placeholder that is not `{[<index>][:<format>]}`, or a lone
    /// `{` or `}`.
    BadPlaceholder,
    /// A placeholder index past the description's stats.
    PlaceholderOutOfRange,
}

/// A line that was not parsed (or, for [`FindingKind::MissingInclude`], an
/// include that does not resolve).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    /// Category.
    pub kind: FindingKind,
    /// 1-based line number in the file.
    pub line: usize,
    /// Non-blank lines left out of the parsed output because of this
    /// finding, this line included if it was left out.
    pub skipped: usize,
    /// The line as written, without surrounding whitespace.
    pub text: String,
}

/// A variant condition on one stat's value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Condition {
    /// `min <= value <= max`; `None` is open (`#`). `#` alone is both open,
    /// `<v>` is `v|v`.
    Range {
        /// Upper bound.
        max: Option<i64>,
        /// Lower bound.
        min: Option<i64>,
    },
    /// `!<v>`: value is not `v`.
    Not {
        /// The excluded value.
        value: i64,
    },
}

/// A handler after the format string. JSON leaves out the absent argument.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Handler {
    /// Handler name as written.
    pub name: String,
    /// Reminder-text ID ([`REMINDER_HANDLER`] only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reminder: Option<String>,
    /// 1-based stat number the handler applies to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stat: Option<usize>,
}

/// A `{...}` placeholder in a format string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Placeholder {
    /// Format after `:` (`""`, `d`, `+d`, `-d`, or `+`); `""` also when
    /// there is no `:`.
    pub format: String,
    /// 0-based stat index as written; `None` for `{}` and `{:...}`, which
    /// mean stat 0.
    pub index: Option<usize>,
}

/// One English variant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Variant {
    /// One condition per stat (per hand for handed descriptions).
    pub conditions: Vec<Condition>,
    /// Words from [`VARIANT_FLAGS`].
    pub flags: Vec<String>,
    /// Handlers in written order.
    pub handlers: Vec<Handler>,
    /// 1-based line number.
    pub line: usize,
    /// Placeholders in written order.
    pub placeholders: Vec<Placeholder>,
    /// Format string as written (escapes and markup kept).
    pub text: String,
}

/// Variant lists of one non-English language.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct LanguageCount {
    /// Descriptions with a list in this language.
    pub descriptions: usize,
    /// Variant lines in this language.
    pub variants: usize,
}

/// One `description` or `handed_description` block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Description {
    /// `true` for `handed_description`.
    pub handed: bool,
    /// 1-based line number of the keyword.
    pub line: usize,
    /// Name after the keyword, if any.
    pub name: Option<String>,
    /// Off-hand stat IDs of a handed description; `None` otherwise.
    pub off_hand_stats: Option<Vec<String>>,
    /// Stat IDs (main hand for a handed description).
    pub stats: Vec<String>,
    /// English variants that parsed.
    pub variants: Vec<Variant>,
}

/// A parsed file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StatDescFile {
    /// Description blocks in file order.
    pub descriptions: Vec<Description>,
    /// `has_identifiers` / `no_identifiers` lines, as written.
    pub file_flags: Vec<String>,
    /// Lines that were not parsed, in line order.
    pub findings: Vec<Finding>,
    /// Included paths as written, with their line numbers.
    pub includes: Vec<(usize, String)>,
    /// Non-English variant lists, by language.
    pub languages: BTreeMap<String, LanguageCount>,
    /// `no_description` stat IDs in file order.
    pub no_description: Vec<String>,
}

/// A token of a line: a bare word or a quoted string (without its quotes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Token<'a> {
    Word(&'a str),
    Quoted(&'a str),
}

/// Splits a line into tokens; `None` if a quote is not closed. Quoted
/// strings hold no escaped quotes in either game (checked on both installs).
fn tokenize(line: &str) -> Option<Vec<Token<'_>>> {
    let mut tokens = Vec::new();
    let mut rest = line;
    loop {
        rest = rest.trim_start();
        if rest.is_empty() {
            return Some(tokens);
        }
        if let Some(after) = rest.strip_prefix('"') {
            let end = after.find('"')?;
            tokens.push(Token::Quoted(after.get(..end)?));
            rest = after.get(end + 1..)?;
        } else {
            let end = rest
                .find(|c: char| c.is_whitespace() || c == '"')
                .unwrap_or(rest.len());
            tokens.push(Token::Word(rest.get(..end)?));
            rest = rest.get(end..)?;
        }
    }
}

/// A non-blank line.
#[derive(Debug, Clone)]
struct Line<'a> {
    number: usize,
    text: &'a str,
    /// `None` if a quote is not closed.
    tokens: Option<Vec<Token<'a>>>,
}

impl Line<'_> {
    fn first_word(&self) -> Option<&str> {
        match self.tokens.as_deref()?.first()? {
            Token::Word(w) => Some(w),
            Token::Quoted(_) => None,
        }
    }

    /// Starts a top-level construct.
    fn is_top(&self) -> bool {
        self.first_word().is_some_and(|w| TOP_KEYWORDS.contains(&w))
    }

    /// Ends a variant list: a top-level construct or another language.
    fn ends_list(&self) -> bool {
        self.is_top() || self.first_word() == Some(LANG)
    }

    /// All tokens, if they are all words.
    fn words(&self) -> Option<Vec<&str>> {
        self.tokens
            .as_deref()?
            .iter()
            .map(|t| match t {
                Token::Word(w) => Some(*w),
                Token::Quoted(_) => None,
            })
            .collect()
    }

    /// The line's only token as a number.
    fn count(&self) -> Option<usize> {
        match self.tokens.as_deref()? {
            [Token::Word(w)] => unsigned(w),
            _ => None,
        }
    }
}

/// Parses a decoded stat-description file. Never fails: lines that do not
/// fit the grammar become findings.
#[must_use]
pub fn parse(text: &str) -> StatDescFile {
    let lines: Vec<Line<'_>> = text
        .split('\n')
        .enumerate()
        .filter_map(|(i, raw)| {
            let text = raw.trim();
            (!text.is_empty()).then(|| Line {
                number: i + 1,
                text,
                tokens: tokenize(text),
            })
        })
        .collect();
    let mut parser = Parser {
        lines: &lines,
        pos: 0,
        out: StatDescFile::default(),
    };
    parser.run();
    let mut out = parser.out;
    out.findings.sort_by_key(|f| f.line);
    out
}

struct Parser<'l, 'a> {
    lines: &'l [Line<'a>],
    pos: usize,
    out: StatDescFile,
}

impl<'l, 'a> Parser<'l, 'a> {
    fn peek(&self) -> Option<&'l Line<'a>> {
        self.lines.get(self.pos)
    }

    fn next_line(&mut self) -> Option<&'l Line<'a>> {
        let line = self.peek()?;
        self.pos += 1;
        Some(line)
    }

    fn find(&mut self, kind: FindingKind, line: &Line<'_>, skipped: usize) {
        self.out.findings.push(Finding {
            kind,
            line: line.number,
            skipped,
            text: line.text.to_owned(),
        });
    }

    /// Records `kind` at `at` and skips the rest of the block: every line
    /// from the current position up to the next top-level construct.
    fn bad_block(&mut self, kind: FindingKind, at: &Line<'_>) {
        let start = self.pos;
        while self.peek().is_some_and(|l| !l.is_top()) {
            self.pos += 1;
        }
        self.find(kind, at, self.pos - start);
    }

    fn run(&mut self) {
        while let Some(line) = self.next_line() {
            let Some(tokens) = line.tokens.as_deref() else {
                self.find(FindingKind::UnterminatedString, line, 1);
                continue;
            };
            match tokens {
                [Token::Word("include"), Token::Quoted(path)] => {
                    self.out.includes.push((line.number, (*path).to_owned()));
                }
                [Token::Word("include"), ..] => self.find(FindingKind::BadInclude, line, 1),
                [Token::Word("no_description"), Token::Word(id)] => {
                    self.out.no_description.push((*id).to_owned());
                }
                [Token::Word("no_description"), ..] => {
                    self.find(FindingKind::BadNoDescription, line, 1);
                }
                [Token::Word(flag @ ("no_identifiers" | "has_identifiers"))] => {
                    self.out.file_flags.push((*flag).to_owned());
                }
                [
                    Token::Word(kw @ ("description" | "handed_description")),
                    rest @ ..,
                ] => {
                    let name = match rest {
                        [] => None,
                        [Token::Word(name)] => Some((*name).to_owned()),
                        _ => {
                            self.bad_block(FindingKind::BadDescriptionHeader, line);
                            continue;
                        }
                    };
                    self.description(line, *kw == "handed_description", name);
                }
                _ => self.find(FindingKind::UnknownLine, line, 1),
            }
        }
    }

    /// Parses a description block after its keyword line `header`.
    fn description(&mut self, header: &Line<'_>, handed: bool, name: Option<String>) {
        let Some((count, ids)) = self.stat_list(header, handed) else {
            return;
        };
        let (stats, off_hand_stats) = if handed {
            let mut main = ids;
            let off = main.split_off(count);
            (main, Some(off))
        } else {
            (ids, None)
        };
        // A bad English count keeps the description, with no variants, so
        // its stats stay visible; the finding says why.
        let variants = self.variant_list(header, count, true);
        let english_ok = variants.is_some();
        self.out.descriptions.push(Description {
            handed,
            line: header.number,
            name,
            off_hand_stats,
            stats,
            variants: variants.unwrap_or_default(),
        });
        if !english_ok {
            return;
        }
        while let Some(line) = self.peek().filter(|l| l.first_word() == Some(LANG)) {
            let Some([Token::Word(_), Token::Quoted(language)]) = line.tokens.as_deref() else {
                self.bad_block(FindingKind::BadLanguage, line);
                return;
            };
            self.pos += 1;
            let before = self.pos;
            if self.variant_list(line, count, false).is_none() {
                // The `lang` line was consumed; count it as skipped too.
                if let Some(finding) = self.out.findings.last_mut() {
                    finding.skipped += 1;
                }
                return;
            }
            // The count line plus the variant lines.
            let variants = self.pos - before - 1;
            let entry = self
                .out
                .languages
                .entry((*language).to_owned())
                .or_default();
            entry.descriptions += 1;
            entry.variants += variants;
        }
    }

    /// Reads `<n> <id> x n` (`x 2n` if `handed`), which may continue on
    /// lines that start with a non-number. Returns `n` and the IDs.
    fn stat_list(&mut self, header: &Line<'_>, handed: bool) -> Option<(usize, Vec<String>)> {
        let first = match self.peek() {
            Some(line) if !line.is_top() => line,
            _ => {
                self.find(FindingKind::BadStatList, header, 0);
                return None;
            }
        };
        let parsed = first.words().and_then(|words| {
            let (count, ids) = words.split_first()?;
            let count = unsigned(count).filter(|&n| n > 0)?;
            let need = if handed { count.checked_mul(2)? } else { count };
            Some((count, need, ids.to_vec()))
        });
        let Some((count, need, mut ids)) = parsed else {
            self.bad_block(FindingKind::BadStatList, first);
            return None;
        };
        let start = self.pos;
        self.pos += 1;
        while ids.len() < need {
            let more = self
                .peek()
                .filter(|l| !l.ends_list())
                .and_then(|l| l.words())
                .filter(|w| w.first().is_some_and(|w| signed(w).is_none()));
            let Some(words) = more else {
                break;
            };
            self.pos += 1;
            ids.extend(words);
        }
        if ids.len() != need {
            // Count the stat-list lines read so far as skipped too.
            self.pos = start;
            self.bad_block(FindingKind::BadStatList, first);
            return None;
        }
        Some((count, ids.into_iter().map(str::to_owned).collect()))
    }

    /// Reads `<m>` and `m` variant lines for a description with `stats`
    /// stats (per hand). English lines are parsed; other languages' lines
    /// are only consumed. `None` if the count line is bad (the rest of the
    /// block is skipped).
    fn variant_list(&mut self, at: &Line<'_>, stats: usize, english: bool) -> Option<Vec<Variant>> {
        let Some((count_line, count)) = self
            .peek()
            .filter(|l| !l.ends_list())
            .and_then(|l| Some((l, l.count()?)))
        else {
            let at = self.peek().filter(|l| !l.is_top()).unwrap_or(at);
            self.bad_block(FindingKind::BadVariantCount, at);
            return None;
        };
        self.pos += 1;
        let mut variants = Vec::new();
        for _ in 0..count {
            let Some(line) = self.peek().filter(|l| !l.ends_list()) else {
                self.find(FindingKind::BadVariantCount, count_line, 0);
                break;
            };
            self.pos += 1;
            if english {
                match parse_variant(line, stats) {
                    Ok(variant) => variants.push(variant),
                    Err(kind) => self.find(kind, line, 1),
                }
            }
        }
        Some(variants)
    }
}

/// A decimal of digits only (no sign), if it fits a `usize`.
fn unsigned(s: &str) -> Option<usize> {
    let digits = !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    digits.then(|| s.parse().ok()).flatten()
}

/// A decimal with an optional `-` (no `+`), if it fits an `i64`.
fn signed(s: &str) -> Option<i64> {
    let magnitude = s.strip_prefix('-').unwrap_or(s);
    let digits = !magnitude.is_empty() && magnitude.bytes().all(|b| b.is_ascii_digit());
    digits.then(|| s.parse().ok()).flatten()
}

/// Parses one condition token. A range with `min > max` is kept as
/// written (none in either install).
fn condition(word: &str) -> Option<Condition> {
    let bound = |s: &str| -> Option<Option<i64>> {
        if s == "#" {
            Some(None)
        } else {
            signed(s).map(Some)
        }
    };
    if let Some(value) = word.strip_prefix('!') {
        return signed(value).map(|value| Condition::Not { value });
    }
    let (min, max) = match word.split_once('|') {
        Some((min, max)) => (bound(min)?, bound(max)?),
        None => {
            let v = bound(word)?;
            (v, v)
        }
    };
    Some(Condition::Range { max, min })
}

/// Parses an English variant line of a description with `stats` stats.
fn parse_variant(line: &Line<'_>, stats: usize) -> Result<Variant, FindingKind> {
    let tokens = line
        .tokens
        .as_deref()
        .ok_or(FindingKind::UnterminatedString)?;
    let mut tokens = tokens.iter();
    let mut conditions = Vec::with_capacity(stats);
    for _ in 0..stats {
        match tokens.next() {
            Some(Token::Word(w)) => match condition(w) {
                Some(c) => conditions.push(c),
                None if VARIANT_FLAGS.contains(w) => return Err(FindingKind::ConditionCount),
                None => return Err(FindingKind::BadCondition),
            },
            Some(Token::Quoted(_)) | None => return Err(FindingKind::ConditionCount),
        }
    }
    let mut flags = Vec::new();
    let text = loop {
        match tokens.next() {
            Some(Token::Word(w)) if condition(w).is_some() => {
                return Err(FindingKind::ConditionCount);
            }
            Some(Token::Word(w)) if VARIANT_FLAGS.contains(w) => flags.push((*w).to_owned()),
            Some(Token::Word(_)) => return Err(FindingKind::UnknownVariantFlag),
            Some(Token::Quoted(text)) => break *text,
            None => return Err(FindingKind::MissingString),
        }
    };
    let mut handlers = Vec::new();
    while let Some(token) = tokens.next() {
        let Token::Word(name) = token else {
            return Err(FindingKind::BadHandler);
        };
        let (reminder, stat) = if BARE_HANDLERS.contains(name) {
            (None, None)
        } else if *name == REMINDER_HANDLER {
            match tokens.next() {
                Some(Token::Word(id)) => (Some((*id).to_owned()), None),
                _ => return Err(FindingKind::BadHandler),
            }
        } else {
            let stat: usize = match tokens.next() {
                Some(Token::Word(v)) => unsigned(v).ok_or(FindingKind::BadHandler)?,
                _ => return Err(FindingKind::BadHandler),
            };
            if stat == 0 || stat > stats {
                return Err(FindingKind::HandlerStatOutOfRange);
            }
            (None, Some(stat))
        };
        handlers.push(Handler {
            name: (*name).to_owned(),
            reminder,
            stat,
        });
    }
    Ok(Variant {
        conditions,
        flags,
        handlers,
        line: line.number,
        placeholders: placeholders(text, stats)?,
        text: text.to_owned(),
    })
}

/// Placeholder formats seen in English text on both installs.
const PLACEHOLDER_FORMATS: [&str; 5] = ["", "d", "+d", "-d", "+"];

/// The placeholders of a format string for a description with `stats`
/// stats.
fn placeholders(text: &str, stats: usize) -> Result<Vec<Placeholder>, FindingKind> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find(['{', '}']) {
        let (brace, after) = rest
            .get(at..)
            .and_then(|s| Some((s.chars().next()?, s.get(1..)?)))
            .ok_or(FindingKind::BadPlaceholder)?;
        // `{{` and `}}` are literal braces.
        if let Some(after) = after.strip_prefix(brace) {
            rest = after;
            continue;
        }
        if brace == '}' {
            return Err(FindingKind::BadPlaceholder);
        }
        let end = after.find(['{', '}']).ok_or(FindingKind::BadPlaceholder)?;
        let spec = after.get(..end).ok_or(FindingKind::BadPlaceholder)?;
        if !after.get(end..).is_some_and(|s| s.starts_with('}')) {
            return Err(FindingKind::BadPlaceholder);
        }
        let (index, format) = spec.split_once(':').unwrap_or((spec, ""));
        if !PLACEHOLDER_FORMATS.contains(&format) {
            return Err(FindingKind::BadPlaceholder);
        }
        let index: Option<usize> = if index.is_empty() {
            None
        } else if index.bytes().all(|b| b.is_ascii_digit()) {
            Some(index.parse().map_err(|_| FindingKind::BadPlaceholder)?)
        } else {
            return Err(FindingKind::BadPlaceholder);
        };
        if index.unwrap_or(0) >= stats {
            return Err(FindingKind::PlaceholderOutOfRange);
        }
        out.push(Placeholder {
            format: format.to_owned(),
            index,
        });
        rest = after.get(end + 1..).ok_or(FindingKind::BadPlaceholder)?;
    }
    Ok(out)
}

/// One added file.
#[derive(Debug, Clone)]
struct Added {
    source: String,
    kind: SourceKind,
    sha256: [u8; 32],
    /// `None` for files that are not parsed.
    parsed: Option<Result<Rendered, TableError>>,
}

/// A parsed and rendered file.
#[derive(Debug, Clone)]
struct Rendered {
    json: Vec<u8>,
    file: StatDescFile,
}

/// Collects the stat-description files of one game in any order.
#[derive(Debug, Clone)]
pub struct Collection {
    game: Game,
    /// By lowercase index path.
    added: BTreeMap<String, Added>,
}

impl Collection {
    /// An empty collection for `game`.
    #[must_use]
    pub const fn new(game: Game) -> Self {
        Self {
            game,
            added: BTreeMap::new(),
        }
    }

    /// Whether `path` is a file of this collection.
    #[must_use]
    pub fn wants(&self, path: &str) -> bool {
        source_kind(self.game, path).is_some()
    }

    /// Decodes, parses, and renders the file at `path` (as listed in the
    /// index). A file that does not decode is recorded, not returned.
    ///
    /// # Errors
    /// [`ExtractError`] if `path` is not a stat-description file, was added
    /// before, or JSON cannot be written.
    pub fn add(&mut self, path: &str, bytes: &[u8]) -> Result<(), ExtractError> {
        let kind = source_kind(self.game, path).ok_or_else(|| ExtractError::UnplannedFile {
            path: path.to_owned(),
        })?;
        let key = path.to_ascii_lowercase();
        if self.added.contains_key(&key) {
            return Err(ExtractError::DuplicateFile {
                path: path.to_owned(),
            });
        }
        let below = key.strip_prefix(stat_desc_dir(self.game)).unwrap_or(&key);
        let parsed = match kind {
            // The path becomes an output path: refuse anything but plain
            // relative parts.
            _ if !plain_relative(below) => Some(Err(TableError {
                code: "EXTRACT_STATDESC_BAD_PATH",
                message: format!("{path} is not a plain relative path"),
            })),
            SourceKind::Filters => None,
            SourceKind::Descriptions => Some(match decode_utf16le(bytes) {
                Ok(text) => {
                    let file = parse(&text);
                    Ok(Rendered {
                        json: render(path, &file)?,
                        file,
                    })
                }
                Err(e) => Err(TableError {
                    code: e.code(),
                    message: e.to_string(),
                }),
            }),
        };
        self.added.insert(
            key,
            Added {
                source: path.to_owned(),
                kind,
                sha256: Sha256::digest(bytes).into(),
                parsed,
            },
        );
        Ok(())
    }

    /// Resolves includes (missing, not parsed, cycles) and builds the output files (by path inside the
    /// output directory) and the manifest section.
    #[must_use]
    pub fn finish(self) -> (BTreeMap<String, Vec<u8>>, StatDescManifest) {
        let dir = stat_desc_dir(self.game);
        let present: BTreeSet<&str> = self.added.keys().map(String::as_str).collect();
        // Include edges of parsed files, by lowercase path.
        let graph: BTreeMap<&str, Vec<String>> = self
            .added
            .iter()
            .filter_map(|(key, added)| {
                let file = &added.parsed.as_ref()?.as_ref().ok()?.file;
                let targets = file
                    .includes
                    .iter()
                    .map(|(_, p)| p.to_ascii_lowercase())
                    .collect();
                Some((key.as_str(), targets))
            })
            .collect();
        let mut files = BTreeMap::new();
        let mut manifest = StatDescManifest {
            dir,
            files: BTreeMap::new(),
            findings: Vec::new(),
            findings_by_kind: BTreeMap::new(),
            handlers: BTreeMap::new(),
            variant_flags: BTreeMap::new(),
        };
        for (key, added) in &self.added {
            let mut entry = StatDescEntry {
                descriptions: 0,
                english_variants: 0,
                error: None,
                findings: 0,
                includes: Vec::new(),
                kind: added.kind,
                languages: BTreeMap::new(),
                no_description: 0,
                output: None,
                source: added.source.clone(),
                source_sha256: hex(&added.sha256),
                status: FileStatus::Skipped,
            };
            match &added.parsed {
                None => {}
                Some(Err(error)) => {
                    entry.status = FileStatus::Failed;
                    entry.error = Some(error.clone());
                }
                Some(Ok(rendered)) => {
                    let file = &rendered.file;
                    let mut findings = file.findings.clone();
                    for (line, path) in &file.includes {
                        let target = path.to_ascii_lowercase();
                        let kind = if !present.contains(target.as_str()) {
                            FindingKind::MissingInclude
                        } else if !graph.contains_key(target.as_str()) {
                            FindingKind::IncludeNotParsed
                        } else if reaches(&graph, &target, key) {
                            FindingKind::IncludeCycle
                        } else {
                            continue;
                        };
                        findings.push(Finding {
                            kind,
                            line: *line,
                            skipped: 0,
                            text: format!("include \"{path}\""),
                        });
                    }
                    findings.sort_by_key(|f| f.line);
                    let variants = file.descriptions.iter().flat_map(|d| &d.variants);
                    for variant in variants.clone() {
                        for handler in &variant.handlers {
                            *manifest.handlers.entry(handler.name.clone()).or_default() += 1;
                        }
                        for flag in &variant.flags {
                            *manifest.variant_flags.entry(flag.clone()).or_default() += 1;
                        }
                    }
                    let output = format!(
                        "{STAT_DESCRIPTIONS_DIR}/{}.json",
                        key.strip_prefix(dir).unwrap_or(key)
                    );
                    entry.status = FileStatus::Ok;
                    entry.descriptions = file.descriptions.len();
                    entry.english_variants = variants.count();
                    entry.findings = findings.len();
                    entry.includes = file.includes.iter().map(|(_, p)| p.clone()).collect();
                    entry.languages = file.languages.clone();
                    entry.no_description = file.no_description.len();
                    entry.output = Some(output.clone());
                    files.insert(output, rendered.json.clone());
                    for finding in findings {
                        *manifest.findings_by_kind.entry(finding.kind).or_default() += 1;
                        manifest.findings.push(FindingOut {
                            file: added.source.clone(),
                            kind: finding.kind,
                            line: finding.line,
                            skipped: finding.skipped,
                            text: finding.text,
                        });
                    }
                }
            }
            manifest.files.insert(added.source.clone(), entry);
        }
        (files, manifest)
    }
}

/// Whether `to` is `from` or reachable from it through includes.
fn reaches(graph: &BTreeMap<&str, Vec<String>>, from: &str, to: &str) -> bool {
    let mut seen = BTreeSet::new();
    let mut todo = vec![from];
    while let Some(node) = todo.pop() {
        if node == to {
            return true;
        }
        if seen.insert(node)
            && let Some(targets) = graph.get(node)
        {
            todo.extend(targets.iter().map(String::as_str));
        }
    }
    false
}

/// Characters that could take an output path out of its folder on Windows.
const PATH_BREAKERS: [char; 3] = ['\\', ':', '\0'];

/// Whether `path` is `/`-separated parts that are not empty, `.`, or `..`
/// and hold none of [`PATH_BREAKERS`] (so it stays inside the output folder).
pub(crate) fn plain_relative(path: &str) -> bool {
    path.split('/').all(|part| {
        !part.is_empty() && part != "." && part != ".." && !part.contains(PATH_BREAKERS)
    })
}

/// Renders a parsed file as JSON: one description per line.
fn render(source: &str, file: &StatDescFile) -> Result<Vec<u8>, ExtractError> {
    let includes: Vec<&str> = file.includes.iter().map(|(_, p)| p.as_str()).collect();
    let mut out = String::from("{\n\"descriptions\": [");
    for (i, description) in file.descriptions.iter().enumerate() {
        out.push_str(if i == 0 { "\n" } else { ",\n" });
        out.push_str(&serde_json::to_string(description)?);
    }
    out.push_str(if file.descriptions.is_empty() {
        "],\n"
    } else {
        "\n],\n"
    });
    for (name, json) in [
        ("file_flags", serde_json::to_string(&file.file_flags)?),
        ("includes", serde_json::to_string(&includes)?),
        ("languages", serde_json::to_string(&file.languages)?),
        (
            "no_description",
            serde_json::to_string(&file.no_description)?,
        ),
    ] {
        out.push('"');
        out.push_str(name);
        out.push_str("\": ");
        out.push_str(&json);
        out.push_str(",\n");
    }
    out.push_str("\"source\": ");
    out.push_str(&serde_json::to_string(source)?);
    out.push_str("\n}\n");
    Ok(out.into_bytes())
}

/// Outcome for one stat-description file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FileStatus {
    /// Parsed; lines that did not fit are in `findings`.
    Ok,
    /// Not UTF-16: see `error`. No output file.
    Failed,
    /// Not a stat-description file ([`SourceKind::Filters`]): hashed only.
    Skipped,
}

/// The stat-description section of the manifest. Fields in alphabetical
/// order (see [`crate::extract::Manifest`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StatDescManifest {
    /// Folder read in the index.
    pub dir: &'static str,
    /// Every file in the folder, by index path.
    pub files: BTreeMap<String, StatDescEntry>,
    /// Every line not parsed, by file and line.
    pub findings: Vec<FindingOut>,
    /// Finding counts by kind.
    pub findings_by_kind: BTreeMap<FindingKind, usize>,
    /// Handler use counts in parsed English variants, by name.
    pub handlers: BTreeMap<String, usize>,
    /// Variant flag use counts in parsed English variants.
    pub variant_flags: BTreeMap<String, usize>,
}

impl StatDescManifest {
    /// Problems that make the run fail: no stat-description file at all,
    /// or a file that did not decode.
    #[must_use]
    pub fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if !self
            .files
            .values()
            .any(|f| f.kind == SourceKind::Descriptions)
        {
            problems.push(format!("no stat-description files in {}", self.dir));
        }
        for (path, file) in &self.files {
            if file.status == FileStatus::Failed {
                problems.push(format!("{path}: Failed"));
            }
        }
        problems
    }
}

/// One stat-description file in the manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StatDescEntry {
    /// Description blocks parsed.
    pub descriptions: usize,
    /// English variants parsed.
    pub english_variants: usize,
    /// Why the file did not decode.
    pub error: Option<TableError>,
    /// Findings in this file.
    pub findings: usize,
    /// Included paths as written.
    pub includes: Vec<String>,
    /// What the file holds.
    pub kind: SourceKind,
    /// Non-English variant lists, by language.
    pub languages: BTreeMap<String, LanguageCount>,
    /// `no_description` lines.
    pub no_description: usize,
    /// Output file path.
    pub output: Option<String>,
    /// Path in the index.
    pub source: String,
    /// SHA-256 of the file as read from its (decoded) bundle.
    pub source_sha256: String,
    /// Outcome.
    pub status: FileStatus,
}

/// A finding with its file, for the manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FindingOut {
    /// Path in the index.
    pub file: String,
    /// Category.
    pub kind: FindingKind,
    /// 1-based line number.
    pub line: usize,
    /// See [`Finding::skipped`].
    pub skipped: usize,
    /// The line as written.
    pub text: String,
}

#[cfg(test)]
#[expect(
    clippy::indexing_slicing,
    reason = "tests index fixed synthetic data; a bad index should panic"
)]
mod tests {
    use super::*;

    fn utf16le(text: &str) -> Vec<u8> {
        text.encode_utf16().flat_map(u16::to_le_bytes).collect()
    }

    const fn range(min: Option<i64>, max: Option<i64>) -> Condition {
        Condition::Range { max, min }
    }

    fn handler(name: &str, stat: Option<usize>, reminder: Option<&str>) -> Handler {
        Handler {
            name: name.to_owned(),
            reminder: reminder.map(str::to_owned),
            stat,
        }
    }

    fn placeholder(index: Option<usize>, format: &str) -> Placeholder {
        Placeholder {
            format: format.to_owned(),
            index,
        }
    }

    /// (kind, line, skipped) of each finding.
    fn findings(file: &StatDescFile) -> Vec<(FindingKind, usize, usize)> {
        file.findings
            .iter()
            .map(|f| (f.kind, f.line, f.skipped))
            .collect()
    }

    #[test]
    fn decode_should_drop_bom_and_keep_surrogate_pairs() {
        assert_eq!(decode_utf16le(&utf16le("\u{feff}ab")).unwrap(), "ab");
        assert_eq!(decode_utf16le(&utf16le("ab")).unwrap(), "ab");
        // Only a leading BOM is dropped.
        assert_eq!(decode_utf16le(&utf16le("a\u{feff}")).unwrap(), "a\u{feff}");
        assert_eq!(
            decode_utf16le(&utf16le("x\u{1f600}")).unwrap(),
            "x\u{1f600}"
        );
        assert_eq!(decode_utf16le(&[]).unwrap(), "");
    }

    #[test]
    fn decode_should_reject_odd_length_and_unpaired_surrogates() {
        let err = decode_utf16le(b"abc").unwrap_err();
        assert_eq!(err, StatDescError::OddLength { len: 3 });
        assert_eq!(err.code(), "EXTRACT_STATDESC_ODD_LENGTH");
        // "a", "\u{1f600}" (2 units), then a lone high surrogate at byte 6.
        let mut bytes = utf16le("a\u{1f600}");
        bytes.extend(0xd800_u16.to_le_bytes());
        bytes.extend(utf16le("b"));
        let err = decode_utf16le(&bytes).unwrap_err();
        assert_eq!(err, StatDescError::BadUtf16 { offset: 6 });
        assert_eq!(err.code(), "EXTRACT_STATDESC_BAD_UTF16");
    }

    #[test]
    fn tokenize_should_split_words_and_quoted_strings() {
        use Token::{Quoted, Word};
        assert_eq!(
            tokenize("  1|#  \"a  b\"\tnegate 1"),
            Some(vec![Word("1|#"), Quoted("a  b"), Word("negate"), Word("1")])
        );
        assert_eq!(
            tokenize("x\"y\"z \"\""),
            Some(vec![Word("x"), Quoted("y"), Word("z"), Quoted("")])
        );
        assert_eq!(tokenize("   "), Some(vec![]));
        assert_eq!(tokenize("# \"open"), None);
    }

    #[test]
    fn condition_should_parse_every_form() {
        for (word, expected) in [
            ("#", Some(range(None, None))),
            ("5", Some(range(Some(5), Some(5)))),
            ("-1", Some(range(Some(-1), Some(-1)))),
            ("1|#", Some(range(Some(1), None))),
            ("#|-1", Some(range(None, Some(-1)))),
            ("-99|-1", Some(range(Some(-99), Some(-1)))),
            ("#|#", Some(range(None, None))),
            ("2147483646|#", Some(range(Some(2_147_483_646), None))),
            ("!0", Some(Condition::Not { value: 0 })),
            ("!-3", Some(Condition::Not { value: -3 })),
            ("1|1|#", None),
            ("1|", None),
            ("|1", None),
            ("!", None),
            ("!#", None),
            ("+5", None),
            ("!+1", None),
            ("1|+2", None),
            ("-", None),
            ("a", None),
            ("table_only", None),
        ] {
            assert_eq!(condition(word), expected, "{word}");
        }
    }

    #[test]
    fn parse_should_read_top_level_constructs() {
        let file = parse(
            "include \"Metadata/StatDescriptions/a.txt\"\r\n\
             \r\n\
             has_identifiers\t\t\r\n\
             no_identifiers\n\
             no_description some_stat\n",
        );
        assert_eq!(
            file.includes,
            [(1, "Metadata/StatDescriptions/a.txt".to_owned())]
        );
        assert_eq!(file.file_flags, ["has_identifiers", "no_identifiers"]);
        assert_eq!(file.no_description, ["some_stat"]);
        assert!(file.descriptions.is_empty());
        assert_eq!(findings(&file), []);
    }

    #[test]
    fn parse_should_read_a_description_with_variants_and_languages() {
        let file = parse(concat!(
            "description named_one\n",
            "\t2 stat_a stat_b\n",
            "\t3\n",
            "\t\t1|# 0 \"{0}% more, {1:+d} extra\\nsecond line\" negate 2 reminderstring ReminderTextX canonical_line\n",
            "\t\t#|-1 # table_only gem_quality \"{}% less\" negate_and_double 1 markup\n",
            "\t\t!0 -5 \"<enchanted>{{{0}% [Ref|Text]}}\"\n",
            "\tlang \"French\"\n",
            "\t1\n",
            "\t\t1|1|# # \"not parsed {9}\" bogus\n",
            "\tlang \"German\"\n",
            "\t2\n",
            "\t\t# # \"x\"\n",
            "\t\t# # \"y\"\n",
            "description\n",
            "\t1 stat_c\n",
            "\t1\n",
            "\t\t# \"c\"\n",
            "\tlang \"French\"\n",
            "\t1\n",
            "\t\t# \"c fr\"\n",
        ));
        assert_eq!(findings(&file), []);
        assert_eq!(file.descriptions.len(), 2);
        let d = &file.descriptions[0];
        assert_eq!(
            (d.handed, d.line, d.name.as_deref()),
            (false, 1, Some("named_one"))
        );
        assert_eq!(d.stats, ["stat_a", "stat_b"]);
        assert_eq!(d.off_hand_stats, None);
        assert_eq!(d.variants.len(), 3);

        let v = &d.variants[0];
        assert_eq!(v.line, 4);
        assert_eq!(
            v.conditions,
            [range(Some(1), None), range(Some(0), Some(0))]
        );
        assert!(v.flags.is_empty());
        assert_eq!(v.text, "{0}% more, {1:+d} extra\\nsecond line");
        assert_eq!(
            v.handlers,
            [
                handler("negate", Some(2), None),
                handler("reminderstring", None, Some("ReminderTextX")),
                handler("canonical_line", None, None),
            ]
        );
        assert_eq!(
            v.placeholders,
            [placeholder(Some(0), ""), placeholder(Some(1), "+d")]
        );

        let v = &d.variants[1];
        assert_eq!(v.conditions, [range(None, Some(-1)), range(None, None)]);
        assert_eq!(v.flags, ["table_only", "gem_quality"]);
        assert_eq!(
            v.handlers,
            [
                handler("negate_and_double", Some(1), None),
                handler("markup", None, None),
            ]
        );
        assert_eq!(v.placeholders, [placeholder(None, "")]);

        let v = &d.variants[2];
        assert_eq!(
            v.conditions,
            [Condition::Not { value: 0 }, range(Some(-5), Some(-5))]
        );
        assert_eq!(v.placeholders, [placeholder(Some(0), "")]);

        assert_eq!(file.descriptions[1].stats, ["stat_c"]);
        assert_eq!(file.descriptions[1].name, None);
        // Other languages are counted, not parsed (French has a bad line).
        assert_eq!(
            file.languages,
            BTreeMap::from([
                (
                    "French".to_owned(),
                    LanguageCount {
                        descriptions: 2,
                        variants: 2
                    }
                ),
                (
                    "German".to_owned(),
                    LanguageCount {
                        descriptions: 1,
                        variants: 2
                    }
                ),
            ])
        );
    }

    #[test]
    fn parse_should_read_handed_description_across_lines() {
        let file = parse(concat!(
            "handed_description weapon_range\n",
            "\t2\n",
            "main_min main_max\n",
            "\t\toff_min off_max\n",
            "\t2\n",
            "\t\t0 0 \"None\"\n",
            "\t\t# # \"{0} to {1}\"\n",
        ));
        assert_eq!(findings(&file), []);
        let d = &file.descriptions[0];
        assert!(d.handed);
        assert_eq!(d.name.as_deref(), Some("weapon_range"));
        assert_eq!(d.stats, ["main_min", "main_max"]);
        assert_eq!(
            d.off_hand_stats.as_deref(),
            Some(&["off_min".to_owned(), "off_max".to_owned()][..])
        );
        assert_eq!(d.variants.len(), 2);
        assert_eq!(d.variants[1].conditions.len(), 2);
    }

    #[test]
    fn placeholders_should_read_formats_and_literal_braces() {
        for (text, stats, expected) in [
            ("plain", 1, vec![]),
            ("{}", 1, vec![placeholder(None, "")]),
            ("{:+d}", 1, vec![placeholder(None, "+d")]),
            ("{:d}", 1, vec![placeholder(None, "d")]),
            ("{0:+}", 1, vec![placeholder(Some(0), "+")]),
            ("{0:}", 1, vec![placeholder(Some(0), "")]),
            ("{0:-d}", 1, vec![placeholder(Some(0), "-d")]),
            (
                "{2:+d} {1}",
                3,
                vec![placeholder(Some(2), "+d"), placeholder(Some(1), "")],
            ),
            ("{{literal}} <t>{{x}}", 1, vec![]),
            ("<t>{{{0}%}}", 1, vec![placeholder(Some(0), "")]),
            ("[Ref|Text] <<Icon>> {0}", 1, vec![placeholder(Some(0), "")]),
        ] {
            assert_eq!(placeholders(text, stats), Ok(expected), "{text}");
        }
    }

    #[test]
    fn placeholders_should_reject_bad_and_out_of_range_forms() {
        for (text, stats, kind) in [
            ("{0%}", 1, FindingKind::BadPlaceholder),
            ("{0 }", 1, FindingKind::BadPlaceholder),
            ("{0+d}", 1, FindingKind::BadPlaceholder),
            ("{0:x}", 1, FindingKind::BadPlaceholder),
            ("{a}", 1, FindingKind::BadPlaceholder),
            ("{-1}", 1, FindingKind::BadPlaceholder),
            ("{0", 1, FindingKind::BadPlaceholder),
            ("{0{1}", 2, FindingKind::BadPlaceholder),
            ("a } b", 1, FindingKind::BadPlaceholder),
            ("{99999999999999999999999}", 1, FindingKind::BadPlaceholder),
            ("{1}", 1, FindingKind::PlaceholderOutOfRange),
            ("{}", 0, FindingKind::PlaceholderOutOfRange),
        ] {
            assert_eq!(placeholders(text, stats), Err(kind), "{text}");
        }
    }

    /// A description of one stat with `line` as its only English variant.
    fn one_variant(line: &str) -> String {
        format!("description\n1 s\n1\n{line}\n")
    }

    #[test]
    fn parse_should_report_each_bad_variant_line() {
        use FindingKind as K;
        for (line, kind) in [
            ("1|1|# \"a\"", K::BadCondition),
            ("x \"a\"", K::BadCondition),
            ("# # \"a\"", K::ConditionCount),
            ("\"a\"", K::ConditionCount),
            ("table_only \"a\"", K::ConditionCount),
            ("#", K::MissingString),
            ("# foo \"a\"", K::UnknownVariantFlag),
            ("# gem_quality", K::MissingString),
            ("# \"open", K::UnterminatedString),
            ("# \"a\" negate", K::BadHandler),
            ("# \"a\" negate x", K::BadHandler),
            ("# \"a\" negate -1", K::BadHandler),
            ("# \"a\" reminderstring", K::BadHandler),
            ("# \"a\" reminderstring \"q\"", K::BadHandler),
            ("# \"a\" \"b\"", K::BadHandler),
            ("# \"a\" negate 0", K::HandlerStatOutOfRange),
            ("# \"a\" negate 2", K::HandlerStatOutOfRange),
            ("# \"{0%}\"", K::BadPlaceholder),
            ("# \"{1}\"", K::PlaceholderOutOfRange),
        ] {
            let file = parse(&one_variant(line));
            assert_eq!(findings(&file), [(kind, 4, 1)], "{line}");
            assert_eq!(file.findings[0].text, line);
            // The description stays, without the bad variant.
            assert_eq!(file.descriptions.len(), 1, "{line}");
            assert!(file.descriptions[0].variants.is_empty(), "{line}");
        }
    }

    #[test]
    fn parse_should_report_bad_top_level_lines() {
        use FindingKind as K;
        for (text, expected) in [
            ("bogus line\n", vec![(K::UnknownLine, 1, 1)]),
            ("\"quoted\"\n", vec![(K::UnknownLine, 1, 1)]),
            ("include \"open\n", vec![(K::UnterminatedString, 1, 1)]),
            ("include a.txt\n", vec![(K::BadInclude, 1, 1)]),
            ("include \"a\" \"b\"\n", vec![(K::BadInclude, 1, 1)]),
            ("no_description\n", vec![(K::BadNoDescription, 1, 1)]),
            ("no_description a b\n", vec![(K::BadNoDescription, 1, 1)]),
            ("no_identifiers x\n", vec![(K::UnknownLine, 1, 1)]),
            // The header is consumed; the three block lines are skipped.
            (
                "description a b\n1 s\n1\n# \"a\"\nno_identifiers\n",
                vec![(K::BadDescriptionHeader, 1, 3)],
            ),
        ] {
            assert_eq!(findings(&parse(text)), expected, "{text}");
        }
    }

    #[test]
    fn parse_should_report_bad_blocks_and_resume_at_next_description() {
        use FindingKind as K;
        let next = "description\n1 ok\n1\n# \"ok\"\n";
        for (block, expected) in [
            // Missing stat list: nothing to skip.
            ("description\n", vec![(K::BadStatList, 1, 0)]),
            // Count not a number, zero, or too many IDs: the list line and
            // the rest of the block are skipped.
            (
                "description\nx s\n1\n# \"a\"\n",
                vec![(K::BadStatList, 2, 3)],
            ),
            ("description\n0\n1\n# \"a\"\n", vec![(K::BadStatList, 2, 3)]),
            (
                "description\n1 a b\n1\n# \"a\"\n",
                vec![(K::BadStatList, 2, 3)],
            ),
            (
                "description\n1 \"a\"\n1\n# \"a\"\n",
                vec![(K::BadStatList, 2, 3)],
            ),
            // Too few IDs: a number line does not continue the list.
            (
                "description\n2 a\n1\n# # \"a\"\n",
                vec![(K::BadStatList, 2, 3)],
            ),
            // Handed with too few off-hand IDs.
            (
                "handed_description\n1\nmain\n1\n# \"a\"\n",
                vec![(K::BadStatList, 2, 4)],
            ),
            // Missing or bad variant count.
            ("description\n1 s\n", vec![(K::BadVariantCount, 1, 0)]),
            (
                "description\n1 s\n# \"a\"\n",
                vec![(K::BadVariantCount, 3, 1)],
            ),
            (
                "description\n1 s\nlang \"French\"\n1\n# \"a\"\n",
                vec![(K::BadVariantCount, 3, 3)],
            ),
            // Fewer English lines than counted: the one found is kept.
            (
                "description\n1 s\n2\n# \"a\"\n",
                vec![(K::BadVariantCount, 3, 0)],
            ),
            // Fewer French lines than counted.
            (
                "description\n1 s\n1\n# \"a\"\nlang \"French\"\n3\n# \"b\"\n",
                vec![(K::BadVariantCount, 6, 0)],
            ),
            // Bad lang line, then a bad count in another language.
            (
                "description\n1 s\n1\n# \"a\"\nlang French\n1\n# \"b\"\n",
                vec![(K::BadLanguage, 5, 3)],
            ),
            (
                "description\n1 s\n1\n# \"a\"\nlang \"French\"\nx\n# \"b\"\n",
                vec![(K::BadVariantCount, 6, 3)],
            ),
            // More English lines than counted: the extra one is unknown.
            (
                "description\n1 s\n1\n# \"a\"\n# \"b\"\n",
                vec![(K::UnknownLine, 5, 1)],
            ),
        ] {
            let file = parse(&format!("{block}{next}"));
            assert_eq!(findings(&file), expected, "{block}");
            let last = file.descriptions.last().unwrap();
            assert_eq!(last.stats, ["ok"], "{block}");
            assert_eq!(last.variants.len(), 1, "{block}");
        }
    }

    #[test]
    fn parse_should_not_panic_on_any_prefix_of_a_file() {
        let text = concat!(
            "include \"x\"\nno_description a\n",
            "handed_description h\n2\nm1 m2\no1 o2\n2\n0 0 \"n\"\n# # \"{0} {1:+d}\" negate 1\n",
            "lang \"Fran\u{e7}ais\"\n1\n# # \"{0}\"\n",
            "description\n1 s\n1\n!0 table_only \"<t>{{{0}}}\" reminderstring R markup\n",
        );
        let full = parse(text);
        assert_eq!(findings(&full), []);
        for (end, _) in text.char_indices() {
            let _ = parse(&text[..end]);
        }
    }

    #[test]
    fn decode_should_not_panic_on_truncated_or_corrupt_bytes() {
        let bytes = utf16le("\u{feff}description\n1 s\n1\n# \"\u{1f600}\"\n");
        for end in 0..bytes.len() {
            if let Ok(text) = decode_utf16le(&bytes[..end]) {
                let _ = parse(&text);
            }
        }
        for at in 0..bytes.len() {
            for value in [0x00, 0x0a, 0x22, 0x7b, 0xd8, 0xdc, 0xff] {
                let mut corrupt = bytes.clone();
                corrupt[at] = value;
                if let Ok(text) = decode_utf16le(&corrupt) {
                    let _ = parse(&text);
                }
            }
        }
    }

    #[test]
    fn source_kind_should_match_the_game_folder_and_extensions() {
        use SourceKind::{Descriptions, Filters};
        for (game, path, expected) in [
            (
                Game::Poe1,
                "metadata/statdescriptions/stat_descriptions.txt",
                Some(Descriptions),
            ),
            (
                Game::Poe1,
                "Metadata/StatDescriptions/Gem_Stat_Descriptions.TXT",
                Some(Descriptions),
            ),
            (
                Game::Poe1,
                "metadata/statdescriptions/skillpopup_stat_filters.txt",
                Some(Filters),
            ),
            (
                Game::Poe1,
                "data/statdescriptions/stat_descriptions.csd",
                None,
            ),
            (Game::Poe1, "metadata/statdescriptions/notes.dat", None),
            (
                Game::Poe2,
                "data/statdescriptions/stat_descriptions.csd",
                Some(Descriptions),
            ),
            (
                Game::Poe2,
                "data/statdescriptions/specific_skill_stat_descriptions/a/statset_0.csd",
                Some(Descriptions),
            ),
            (
                Game::Poe2,
                "metadata/statdescriptions/stat_descriptions.txt",
                None,
            ),
            (Game::Poe2, "data/balance/stats.datc64", None),
        ] {
            assert_eq!(source_kind(game, path), expected, "{path}");
        }
    }

    #[test]
    fn collection_should_skip_filters_and_report_missing_includes() {
        let mut c = Collection::new(Game::Poe2);
        let main = "data/statdescriptions/stat_descriptions.csd";
        let skill = "data/statdescriptions/specific_skill_stat_descriptions/a/statset_0.csd";
        let filters = "data/statdescriptions/x_filters.csd";
        c.add(main, &utf16le("description\n1 s\n1\n# \"{0}\" negate 1\n"))
            .unwrap();
        c.add(
            skill,
            &utf16le(concat!(
                "include \"Data/StatDescriptions/stat_descriptions.csd\"\n",
                "include \"Data/StatDescriptions/absent.csd\"\n",
                "description\n1 s\n1\n# gem_quality \"{0}\" negate 1\n",
            )),
        )
        .unwrap();
        c.add(filters, b"not utf-16").unwrap();

        let err = c
            .add("DATA/StatDescriptions/stat_descriptions.csd", &[])
            .unwrap_err();
        assert_eq!(err.code(), "EXTRACT_DUPLICATE_FILE");
        let err = c.add("data/balance/stats.datc64", &[]).unwrap_err();
        assert_eq!(err.code(), "EXTRACT_UNPLANNED_FILE");

        let (files, m) = c.finish();
        assert_eq!(
            files.keys().collect::<Vec<_>>(),
            [
                "stat_descriptions/specific_skill_stat_descriptions/a/statset_0.csd.json",
                "stat_descriptions/stat_descriptions.csd.json",
            ]
        );
        assert_eq!(m.files[filters].status, FileStatus::Skipped);
        assert_eq!(m.files[filters].kind, SourceKind::Filters);
        assert_eq!(m.files[filters].output, None);
        let entry = &m.files[skill];
        assert_eq!(entry.status, FileStatus::Ok);
        assert_eq!(
            (entry.descriptions, entry.english_variants, entry.findings),
            (1, 1, 1)
        );
        assert_eq!(entry.includes.len(), 2);
        assert_eq!(
            m.findings,
            [FindingOut {
                file: skill.to_owned(),
                kind: FindingKind::MissingInclude,
                line: 2,
                skipped: 0,
                text: "include \"Data/StatDescriptions/absent.csd\"".to_owned(),
            }]
        );
        assert_eq!(
            m.findings_by_kind,
            BTreeMap::from([(FindingKind::MissingInclude, 1)])
        );
        assert_eq!(m.handlers, BTreeMap::from([("negate".to_owned(), 2)]));
        assert_eq!(
            m.variant_flags,
            BTreeMap::from([("gem_quality".to_owned(), 1)])
        );
        assert_eq!(m.problems(), Vec::<String>::new());
    }

    #[test]
    fn render_should_write_one_description_per_line() {
        let text = "include \"i\"\nno_description n\ndescription\n1 s\n1\n# \"{0}\"\n\
                    lang \"French\"\n1\n# \"x\"\n";
        let json = render("data/statdescriptions/a.csd", &parse(text)).unwrap();
        let expected = concat!(
            "{\n\"descriptions\": [\n",
            r#"{"handed":false,"line":3,"name":null,"off_hand_stats":null,"stats":["s"],"#,
            r#""variants":[{"conditions":[{"kind":"range","max":null,"min":null}],"#,
            r#""flags":[],"handlers":[],"line":6,"placeholders":[{"format":"","index":0}],"#,
            r#""text":"{0}"}]}"#,
            "\n],\n",
            "\"file_flags\": [],\n",
            "\"includes\": [\"i\"],\n",
            "\"languages\": {\"French\":{\"descriptions\":1,\"variants\":1}},\n",
            "\"no_description\": [\"n\"],\n",
            "\"source\": \"data/statdescriptions/a.csd\"\n}\n",
        );
        assert_eq!(String::from_utf8(json).unwrap(), expected);
        let empty = String::from_utf8(render("x", &parse("")).unwrap()).unwrap();
        assert!(empty.starts_with("{\n\"descriptions\": [],\n"));
    }

    #[test]
    fn parse_should_keep_a_description_whose_english_count_is_bad() {
        use FindingKind as K;
        for (text, expected) in [
            ("description x\n2 a b\n", vec![(K::BadVariantCount, 1, 0)]),
            (
                "description x\n2 a b\n+1\n# # \"a\"\n",
                vec![(K::BadVariantCount, 3, 2)],
            ),
        ] {
            let file = parse(text);
            assert_eq!(findings(&file), expected, "{text}");
            let d = &file.descriptions[0];
            assert_eq!((d.name.as_deref(), d.line), (Some("x"), 1), "{text}");
            assert_eq!(d.stats, ["a", "b"], "{text}");
            assert!(d.variants.is_empty(), "{text}");
        }
    }

    #[test]
    fn parse_should_reject_plus_signs_in_counts_and_handler_stats() {
        use FindingKind as K;
        let file = parse("description\n+1 s\n1\n# \"a\"\n");
        assert_eq!(findings(&file), [(K::BadStatList, 2, 3)]);
        let file = parse(&one_variant("# \"a\" negate +1"));
        assert_eq!(findings(&file), [(K::BadHandler, 4, 1)]);
        let file = parse(&one_variant("+1 \"a\""));
        assert_eq!(findings(&file), [(K::BadCondition, 4, 1)]);
    }

    #[test]
    fn collection_should_fail_paths_that_leave_the_output_folder() {
        for bad in [
            "data/statdescriptions/../x.csd",
            "data/statdescriptions/a//x.csd",
            "data/statdescriptions/./x.csd",
            "data/statdescriptions/a\\x.csd",
            "data/statdescriptions/c:x.csd",
        ] {
            let mut c = Collection::new(Game::Poe2);
            c.add(bad, &utf16le("no_description s\n")).unwrap();
            let (files, m) = c.finish();
            assert!(files.is_empty(), "{bad}");
            let entry = &m.files[bad];
            assert_eq!(entry.status, FileStatus::Failed, "{bad}");
            assert_eq!(
                entry.error.as_ref().map(|e| e.code),
                Some("EXTRACT_STATDESC_BAD_PATH"),
                "{bad}"
            );
            assert_eq!(m.problems(), [format!("{bad}: Failed")], "{bad}");
        }
    }

    #[test]
    fn collection_should_report_includes_of_unparsed_files_and_cycles() {
        let dir = "metadata/statdescriptions/";
        let mut c = Collection::new(Game::Poe1);
        let include = |name: &str| format!("include \"Metadata/StatDescriptions/{name}\"\n");
        // a -> b -> c -> a is a cycle; d -> a is not part of it;
        // e includes a filter file and a file that is not UTF-16.
        for (name, text) in [
            ("a.txt", include("B.txt")),
            ("b.txt", include("c.txt")),
            ("c.txt", include("a.txt")),
            ("d.txt", include("a.txt")),
            ("e.txt", include("x_filters.txt") + &include("bad.txt")),
        ] {
            c.add(&format!("{dir}{name}"), &utf16le(&text)).unwrap();
        }
        c.add(&format!("{dir}x_filters.txt"), b"").unwrap();
        c.add(&format!("{dir}bad.txt"), b"odd").unwrap();
        let (_, m) = c.finish();
        let found: Vec<(&str, FindingKind, usize)> = m
            .findings
            .iter()
            .map(|f| (f.file.strip_prefix(dir).unwrap(), f.kind, f.line))
            .collect();
        assert_eq!(
            found,
            [
                ("a.txt", FindingKind::IncludeCycle, 1),
                ("b.txt", FindingKind::IncludeCycle, 1),
                ("c.txt", FindingKind::IncludeCycle, 1),
                ("e.txt", FindingKind::IncludeNotParsed, 1),
                ("e.txt", FindingKind::IncludeNotParsed, 2),
            ]
        );
    }
}
