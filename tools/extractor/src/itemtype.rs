//! Item type files (`*.it`, Spike B, PR B4x): the class hierarchy of item
//! types. Each `BaseItemTypes` row names its parent type in `InheritsFrom`
//! (for example `Metadata/Items/Rings/AbstractRing`); the type's file and
//! the files it `extends` hold the inherited spawn tags (`ring`,
//! `default`), which `BaseItemTypes.TagsKeys` does not repeat. Spike B's
//! Mods filter (PR B4a) needs the full tag set of a base.
//!
//! The extractor writes each file as parsed, without resolving the chain:
//! deriving tags is the consumer's job, as for the other formats. The
//! manifest checks the chain: every `extends` names a file of the index
//! (or `nothing`, the root) and no chain loops.
//!
//! Grammar (UTF-16LE with a byte-order mark; observed on all 413 PoE1 and
//! 277 PoE2 files of the installs, report section 19). Lines are trimmed;
//! blank lines and `//` comment lines are skipped.
//!
//! ```text
//! version 3
//! abstract                         optional
//! extends "Metadata/Items/Item"    "nothing" at the root
//!
//! Base                             a block: the name, then "{" on its own
//! {                                line, or "Name {" / "Name {}" on one line
//!     tag = "ring"                 an entry: key = "text" | bare token
//!     remove_tag = "weapon"
//!     x_size = 1
//! }
//! client { RenderItem { } }        blocks nest (one construct per line)
//! ```
//!
//! Anything else is an error for the whole file, never skipped.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::extract::{ExtractError, TableError};
use crate::install::hex;
use crate::statdesc::{StatDescError, decode_utf16le, plain_relative};

/// Folder of the item type files in the extract output.
pub const ITEM_TYPES_DIR: &str = "item_types";

/// File extension of an item type in the index.
pub const EXTENSION: &str = ".it";

/// The `extends` target of a root type (`Metadata/Items/Item` on both
/// installs). `v2.67.2:src/Export/Scripts/bases.lua` `getBaseItemTags`
/// treats it as the base case too.
pub const ROOT: &str = "nothing";

/// Deepest block nesting read (the installs nest 2 deep). A cap keeps
/// rendering and dropping a hostile file, which recurse per level, off the
/// end of the stack.
pub const MAX_DEPTH: usize = 32;

/// Whether `path` (as listed in the index) is an item type file.
#[must_use]
pub fn is_item_type(path: &str) -> bool {
    path.to_ascii_lowercase().ends_with(EXTENSION)
}

/// Output path of the file at `path` (lowercase index path plus `.json`).
#[must_use]
pub fn output_path(path: &str) -> String {
    format!("{ITEM_TYPES_DIR}/{}.json", path.to_ascii_lowercase())
}

/// The index path an `extends` target names: lowercase plus [`EXTENSION`];
/// `None` for [`ROOT`].
#[must_use]
pub fn target_path(extends: &str) -> Option<String> {
    (extends != ROOT).then(|| format!("{}{EXTENSION}", extends.to_ascii_lowercase()))
}

/// Errors from reading an item type file. Each variant has a stable
/// [`code`](Self::code).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ItemTypeError {
    /// Not UTF-16LE text (the stat-description decoder's error, given an
    /// `EXTRACT_IT_*` code here so the manifest names the file kind).
    #[error(transparent)]
    Text(#[from] StatDescError),
    /// Blocks nested deeper than [`MAX_DEPTH`].
    #[error("line {line}: blocks nested deeper than {MAX_DEPTH}")]
    TooDeep {
        /// 1-based line number.
        line: usize,
    },
    /// A line that is no construct of the grammar, or one in the wrong place.
    #[error("line {line}: unexpected {text:?}")]
    Unexpected {
        /// 1-based line number.
        line: usize,
        /// The trimmed line.
        text: String,
    },
    /// `version`, `abstract`, or `extends` given twice.
    #[error("line {line}: second {what}")]
    Repeated {
        /// 1-based line number.
        line: usize,
        /// The header word.
        what: &'static str,
    },
    /// The file has no `version` or no `extends` line.
    #[error("no {0} line")]
    Missing(&'static str),
    /// The file ends inside a block, or after a block name with no `{`.
    #[error("{0} blocks not closed at the end of the file")]
    Unclosed(usize),
}

impl ItemTypeError {
    /// Stable machine-readable code (ADR-014).
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Text(StatDescError::OddLength { .. }) => "EXTRACT_IT_ODD_LENGTH",
            Self::Text(StatDescError::BadUtf16 { .. }) => "EXTRACT_IT_BAD_UTF16",
            Self::TooDeep { .. } => "EXTRACT_IT_TOO_DEEP",
            Self::Unexpected { .. } => "EXTRACT_IT_UNEXPECTED_LINE",
            Self::Repeated { .. } => "EXTRACT_IT_REPEATED_HEADER",
            Self::Missing(_) => "EXTRACT_IT_MISSING_HEADER",
            Self::Unclosed(_) => "EXTRACT_IT_UNCLOSED_BLOCK",
        }
    }
}

/// A `key = value` line. Fields in alphabetical order (stable JSON).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Entry {
    /// The key.
    pub key: String,
    /// Whether the value was written in double quotes (removed here).
    pub quoted: bool,
    /// The value as written, without the quotes.
    pub value: String,
}

/// A named block. Fields in alphabetical order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Block {
    /// Nested blocks, in file order.
    pub blocks: Vec<Block>,
    /// Entries, in file order.
    pub entries: Vec<Entry>,
    /// The block name (`Base`, `Mods`, ...).
    pub name: String,
}

/// A parsed item type file. Fields in alphabetical order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ItemType {
    /// Whether the file has an `abstract` line.
    #[serde(rename = "abstract")]
    pub is_abstract: bool,
    /// Top-level blocks, in file order.
    pub blocks: Vec<Block>,
    /// `//` comment lines skipped.
    pub comments: usize,
    /// The parent type as written (`Metadata/Items/Item`), or [`ROOT`].
    pub extends: String,
    /// The `version` number.
    pub version: u32,
}

impl ItemType {
    /// Values of every entry named `key` in a top-level block named
    /// `block`, in file order.
    pub fn values<'a>(&'a self, block: &'a str, key: &'a str) -> impl Iterator<Item = &'a str> {
        self.blocks
            .iter()
            .filter(move |b| b.name == block)
            .flat_map(|b| &b.entries)
            .filter(move |e| e.key == key)
            .map(|e| e.value.as_str())
    }
}

/// Whether `s` is a name: ASCII letters, digits, `_`.
fn is_name(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// Whether `s` is a bare value: ASCII letters, digits, `_`, `.`, `+`, `-`.
fn is_bare(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'+' | b'-'))
}

/// `"text"` without inner quotes, as the text.
fn quoted(s: &str) -> Option<&str> {
    s.strip_prefix('"')?
        .strip_suffix('"')
        .filter(|t| !t.contains('"'))
}

/// What a block line opens: a block whose `{` follows on the next line, an
/// open block, or an empty one.
enum Opens<'l> {
    Pending(&'l str),
    Open(&'l str),
    Empty(&'l str),
}

fn block_line(line: &str) -> Option<Opens<'_>> {
    if is_name(line) {
        return Some(Opens::Pending(line));
    }
    if let Some(name) = line.strip_suffix("{}") {
        let name = name.trim_end();
        return is_name(name).then_some(Opens::Empty(name));
    }
    let name = line.strip_suffix('{')?.trim_end();
    is_name(name).then_some(Opens::Open(name))
}

fn entry_line(line: &str) -> Option<Entry> {
    let (key, value) = line.split_once('=')?;
    let (key, value) = (key.trim(), value.trim());
    if !is_name(key) {
        return None;
    }
    let (quoted, value) = match quoted(value) {
        Some(text) => (true, text),
        None if is_bare(value) => (false, value),
        None => return None,
    };
    Some(Entry {
        key: key.to_owned(),
        quoted,
        value: value.to_owned(),
    })
}

/// Header lines and blocks read so far.
#[derive(Default)]
struct State {
    version: Option<u32>,
    is_abstract: bool,
    extends: Option<String>,
    top: Vec<Block>,
    /// Open blocks, innermost last.
    open: Vec<Block>,
    /// A block name whose `{` is expected on the next line.
    pending: Option<String>,
    comments: usize,
}

impl State {
    fn close(&mut self, block: Block) {
        match self.open.last_mut() {
            Some(parent) => parent.blocks.push(block),
            None => self.top.push(block),
        }
    }

    fn header(&mut self, n: usize, line: &str) -> Result<bool, ItemTypeError> {
        let repeated = |what| ItemTypeError::Repeated { line: n, what };
        if let Some(v) = line.strip_prefix("version ") {
            let Ok(v) = v.trim().parse() else {
                return Ok(false);
            };
            if self.version.replace(v).is_some() {
                return Err(repeated("version"));
            }
        } else if line == "abstract" {
            if self.is_abstract {
                return Err(repeated("abstract"));
            }
            self.is_abstract = true;
        } else if let Some(target) = line.strip_prefix("extends ").and_then(|t| quoted(t.trim())) {
            if self.extends.replace(target.to_owned()).is_some() {
                return Err(repeated("extends"));
            }
        } else {
            return Ok(false);
        }
        Ok(true)
    }

    fn push(&mut self, n: usize, block: Block) -> Result<(), ItemTypeError> {
        if self.open.len() >= MAX_DEPTH {
            return Err(ItemTypeError::TooDeep { line: n });
        }
        self.open.push(block);
        Ok(())
    }

    fn line(&mut self, n: usize, line: &str) -> Result<(), ItemTypeError> {
        let unexpected = || ItemTypeError::Unexpected {
            line: n,
            text: line.to_owned(),
        };
        if let Some(name) = self.pending.take() {
            if line != "{" {
                return Err(unexpected());
            }
            return self.push(
                n,
                Block {
                    blocks: Vec::new(),
                    entries: Vec::new(),
                    name,
                },
            );
        }
        if line == "}" {
            let block = self.open.pop().ok_or_else(unexpected)?;
            self.close(block);
            return Ok(());
        }
        if self.open.is_empty() && self.header(n, line)? {
            return Ok(());
        }
        if let Some(opens) = block_line(line) {
            let new = |name: &str| Block {
                blocks: Vec::new(),
                entries: Vec::new(),
                name: name.to_owned(),
            };
            match opens {
                Opens::Pending(name) => self.pending = Some(name.to_owned()),
                Opens::Open(name) => self.push(n, new(name))?,
                Opens::Empty(name) => self.close(new(name)),
            }
            return Ok(());
        }
        match (self.open.last_mut(), entry_line(line)) {
            (Some(block), Some(entry)) => {
                block.entries.push(entry);
                Ok(())
            }
            _ => Err(unexpected()),
        }
    }
}

/// Parses an item type file.
///
/// # Errors
/// [`ItemTypeError`] if the bytes are not UTF-16LE, a line is no construct
/// of the grammar, a header line repeats or is missing, or a block is not
/// closed.
pub fn parse(bytes: &[u8]) -> Result<ItemType, ItemTypeError> {
    let text = decode_utf16le(bytes)?;
    let mut state = State::default();
    for (i, raw) in text.split('\n').enumerate() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with("//") {
            state.comments += 1;
            continue;
        }
        state.line(i + 1, line)?;
    }
    let unclosed = state.open.len() + usize::from(state.pending.is_some());
    if unclosed > 0 {
        return Err(ItemTypeError::Unclosed(unclosed));
    }
    Ok(ItemType {
        is_abstract: state.is_abstract,
        blocks: state.top,
        comments: state.comments,
        extends: state.extends.ok_or(ItemTypeError::Missing("extends"))?,
        version: state.version.ok_or(ItemTypeError::Missing("version"))?,
    })
}

/// Renders a file as JSON: header fields, then one top-level block per line.
fn render(item: &ItemType) -> Result<Vec<u8>, ExtractError> {
    let mut out = format!("{{\n\"abstract\": {},\n\"blocks\": [", item.is_abstract);
    for (i, block) in item.blocks.iter().enumerate() {
        out.push_str(if i == 0 { "\n" } else { ",\n" });
        out.push_str(&serde_json::to_string(block)?);
    }
    out.push_str(if item.blocks.is_empty() {
        "],\n"
    } else {
        "\n],\n"
    });
    out.push_str(&format!(
        "\"comments\": {},\n\"extends\": {},\n\"version\": {}\n}}\n",
        item.comments,
        serde_json::to_string(&item.extends)?,
        item.version
    ));
    Ok(out.into_bytes())
}

/// Outcome of one item type file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ItemTypeStatus {
    /// Parsed.
    Ok,
    /// Did not parse: see `error`. No output file.
    Failed,
}

/// One item type file in the manifest. Fields in alphabetical order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ItemTypeEntry {
    /// Whether the file is `abstract`.
    #[serde(rename = "abstract")]
    pub is_abstract: Option<bool>,
    /// Top-level blocks.
    pub blocks: Option<usize>,
    /// Why the file did not parse.
    pub error: Option<TableError>,
    /// The parent type as written.
    pub extends: Option<String>,
    /// Output file path.
    pub output: Option<String>,
    /// Path in the index.
    pub source: String,
    /// SHA-256 of the file as read from its (decoded) bundle.
    pub source_sha256: String,
    /// Outcome.
    pub status: ItemTypeStatus,
}

/// The item type section of the manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ItemTypeManifest {
    /// Files on an `extends` chain that comes back to itself.
    pub cycles: BTreeSet<String>,
    /// Files whose `extends` names no parsed file of the index (and is not
    /// [`ROOT`]), with the target as written.
    pub dangling_extends: BTreeMap<String, String>,
    /// Output folder.
    pub dir: &'static str,
    /// Every `*.it` file in the index, by lowercase index path.
    pub files: BTreeMap<String, ItemTypeEntry>,
}

impl ItemTypeManifest {
    /// Problems that make the run fail: no item type file at all, a file
    /// that did not parse, a dangling `extends`, or a cycle.
    #[must_use]
    pub fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if self.files.is_empty() {
            problems.push(format!("no {EXTENSION} files in the index"));
        }
        for (path, file) in &self.files {
            if file.status == ItemTypeStatus::Failed {
                problems.push(format!("{path}: Failed"));
            }
        }
        for (path, target) in &self.dangling_extends {
            problems.push(format!(
                "{path}: extends {target:?}, which is not in the index"
            ));
        }
        for path in &self.cycles {
            problems.push(format!("{path}: on an extends cycle"));
        }
        problems
    }
}

/// The `extends` chain checks over parsed files (`path -> extends`). A
/// target is dangling only when no file of the index has its path (`known`,
/// failed files included), so a failed parent is reported once, as failed,
/// not again for each child.
fn chain_checks(
    extends: &BTreeMap<String, String>,
    known: &BTreeSet<String>,
) -> (BTreeMap<String, String>, BTreeSet<String>) {
    let mut dangling = BTreeMap::new();
    let mut cycles = BTreeSet::new();
    for (path, target) in extends {
        if target_path(target).is_some_and(|t| !known.contains(&t)) {
            dangling.insert(path.clone(), target.clone());
        }
        // Walk up from `path`; coming back to it means it is on a cycle. A
        // chain that loops above `path` stops when a file repeats.
        let mut seen = BTreeSet::new();
        let mut at = path.clone();
        while let Some(next) = extends.get(&at).and_then(|t| target_path(t)) {
            if next == *path {
                cycles.insert(path.clone());
                break;
            }
            if !seen.insert(next.clone()) {
                break;
            }
            at = next;
        }
    }
    (dangling, cycles)
}

#[derive(Debug, Clone)]
struct Added {
    source: String,
    sha256: [u8; 32],
    result: Result<(ItemType, Vec<u8>), TableError>,
}

/// Collects the item type files of one game in any order.
#[derive(Debug, Clone, Default)]
pub struct Collection {
    added: BTreeMap<String, Added>,
}

impl Collection {
    /// An empty collection.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            added: BTreeMap::new(),
        }
    }

    /// Parses and renders the file at `path` (as listed in the index). A
    /// file that does not parse is recorded, not returned.
    ///
    /// # Errors
    /// [`ExtractError`] if `path` is not an item type file, was added
    /// before, or JSON cannot be written.
    pub fn add(&mut self, path: &str, bytes: &[u8]) -> Result<(), ExtractError> {
        if !is_item_type(path) {
            return Err(ExtractError::UnplannedFile {
                path: path.to_owned(),
            });
        }
        let key = path.to_ascii_lowercase();
        if self.added.contains_key(&key) {
            return Err(ExtractError::DuplicateFile {
                path: path.to_owned(),
            });
        }
        let result = if plain_relative(&key) {
            match parse(bytes) {
                Ok(item) => {
                    let json = render(&item)?;
                    Ok((item, json))
                }
                Err(e) => Err(TableError {
                    code: e.code(),
                    message: e.to_string(),
                }),
            }
        } else {
            Err(TableError {
                code: "EXTRACT_IT_BAD_PATH",
                message: format!("{path} is not a plain relative path"),
            })
        };
        self.added.insert(
            key,
            Added {
                source: path.to_owned(),
                sha256: Sha256::digest(bytes).into(),
                result,
            },
        );
        Ok(())
    }

    /// Builds the output files (by path inside the output directory) and the
    /// manifest section.
    #[must_use]
    pub fn finish(self) -> (BTreeMap<String, Vec<u8>>, ItemTypeManifest) {
        let mut files = BTreeMap::new();
        let mut entries = BTreeMap::new();
        let mut extends = BTreeMap::new();
        for (key, added) in self.added {
            let mut entry = ItemTypeEntry {
                is_abstract: None,
                blocks: None,
                error: None,
                extends: None,
                output: None,
                source: added.source,
                source_sha256: hex(&added.sha256),
                status: ItemTypeStatus::Failed,
            };
            match added.result {
                Ok((item, json)) => {
                    let out = output_path(&key);
                    entry.is_abstract = Some(item.is_abstract);
                    entry.blocks = Some(item.blocks.len());
                    entry.extends = Some(item.extends.clone());
                    entry.output = Some(out.clone());
                    entry.status = ItemTypeStatus::Ok;
                    extends.insert(key.clone(), item.extends);
                    files.insert(out, json);
                }
                Err(error) => entry.error = Some(error),
            }
            entries.insert(key, entry);
        }
        let known: BTreeSet<String> = entries.keys().cloned().collect();
        let (dangling_extends, cycles) = chain_checks(&extends, &known);
        (
            files,
            ItemTypeManifest {
                cycles,
                dangling_extends,
                dir: ITEM_TYPES_DIR,
                files: entries,
            },
        )
    }
}

#[cfg(test)]
#[expect(
    clippy::indexing_slicing,
    reason = "tests index fixed synthetic data; a bad index should panic"
)]
mod tests {
    use super::*;

    /// UTF-16LE with a byte-order mark, as the game stores the files.
    fn utf16(text: &str) -> Vec<u8> {
        std::iter::once(0xfeff_u16)
            .chain(text.encode_utf16())
            .flat_map(u16::to_le_bytes)
            .collect()
    }

    const RING: &str = "version 3\r\nabstract\r\nextends \"Metadata/Items/Equipment\"\r\n\r\n\
        Base\r\n{\r\n\tx_size = 1\r\n\ttag = \"ring\"\r\n}\r\n\r\nMods\r\n{\r\n\
        \tinventory_type = \"Ring\"\r\n}\r\n";

    #[test]
    fn parse_should_read_headers_blocks_and_entries() {
        let item = parse(&utf16(RING)).unwrap();
        assert!(item.is_abstract);
        assert_eq!(item.version, 3);
        assert_eq!(item.extends, "Metadata/Items/Equipment");
        assert_eq!(item.blocks.len(), 2);
        assert_eq!(
            item.blocks[0].entries,
            [
                Entry {
                    key: "x_size".into(),
                    quoted: false,
                    value: "1".into()
                },
                Entry {
                    key: "tag".into(),
                    quoted: true,
                    value: "ring".into()
                },
            ]
        );
        assert_eq!(item.values("Base", "tag").collect::<Vec<_>>(), ["ring"]);
        assert_eq!(item.values("Mods", "tag").count(), 0);
    }

    #[test]
    fn parse_should_read_one_line_blocks_nesting_and_comments() {
        let text = "version 3\nextends \"nothing\"\nLocalStats {}\nclient\n{\n\tRenderItem\n\t{\n\t}\n\
            \tInner {\n\t\ton = true\n\t}\n}\nQuest\n{\n\t//remove later\n\tflag = \"a b\"\n}\n";
        let item = parse(&utf16(text)).unwrap();
        assert!(!item.is_abstract);
        assert_eq!(item.extends, ROOT);
        assert_eq!(item.comments, 1);
        let names: Vec<&str> = item.blocks.iter().map(|b| b.name.as_str()).collect();
        assert_eq!(names, ["LocalStats", "client", "Quest"]);
        let client = &item.blocks[1];
        assert_eq!(client.blocks.len(), 2);
        assert_eq!(client.blocks[1].entries[0].value, "true");
        assert_eq!(item.values("Quest", "flag").collect::<Vec<_>>(), ["a b"]);
    }

    #[test]
    fn parse_should_reject_what_the_grammar_does_not_have() {
        let err = |text: &str| parse(&utf16(text)).unwrap_err().code();
        let head = "version 3\nextends \"nothing\"\n";
        assert_eq!(
            err(&format!("{head}tag = \"x\"\n")),
            "EXTRACT_IT_UNEXPECTED_LINE"
        );
        assert_eq!(
            err(&format!("{head}Base\n{{\n\tbad line\n}}\n")),
            "EXTRACT_IT_UNEXPECTED_LINE"
        );
        assert_eq!(
            err(&format!("{head}Base\n{{\n\tk = \"a\"b\"\n}}\n")),
            "EXTRACT_IT_UNEXPECTED_LINE"
        );
        assert_eq!(
            err(&format!("{head}Base\nx = 1\n")),
            "EXTRACT_IT_UNEXPECTED_LINE"
        );
        assert_eq!(err(&format!("{head}}}\n")), "EXTRACT_IT_UNEXPECTED_LINE");
        assert_eq!(
            err(&format!("{head}Base\n{{\n")),
            "EXTRACT_IT_UNCLOSED_BLOCK"
        );
        assert_eq!(err(&format!("{head}Base\n")), "EXTRACT_IT_UNCLOSED_BLOCK");
        assert_eq!(
            err(&format!("{head}version 3\n")),
            "EXTRACT_IT_REPEATED_HEADER"
        );
        assert_eq!(
            err(&format!("{head}extends \"x\"\n")),
            "EXTRACT_IT_REPEATED_HEADER"
        );
        assert_eq!(err("version 3\n"), "EXTRACT_IT_MISSING_HEADER");
        assert_eq!(err("extends \"nothing\"\n"), "EXTRACT_IT_MISSING_HEADER");
        // Header words are only headers at the top level.
        assert_eq!(
            err(&format!("{head}Base\n{{\nabstract\n}}\n")),
            "EXTRACT_IT_UNEXPECTED_LINE"
        );
        assert_eq!(parse(b"abc").unwrap_err().code(), "EXTRACT_IT_ODD_LENGTH");
        assert_eq!(
            parse(&[0x00, 0xd8]).unwrap_err().code(),
            "EXTRACT_IT_BAD_UTF16"
        );
        let deep = |n: usize| format!("{head}{}{}", "X {\n".repeat(n), "}\n".repeat(n));
        assert!(parse(&utf16(&deep(MAX_DEPTH))).is_ok());
        assert_eq!(err(&deep(MAX_DEPTH + 1)), "EXTRACT_IT_TOO_DEEP");
        let pending = format!("{head}{}X\n{{\n", "X {\n".repeat(MAX_DEPTH));
        assert_eq!(err(&pending), "EXTRACT_IT_TOO_DEEP");
    }

    #[test]
    fn finish_should_report_a_failed_parent_once() {
        let mut c = Collection::new();
        c.add("metadata/items/parent.it", b"odd").unwrap();
        c.add("metadata/items/child.it", &file("Metadata/Items/Parent"))
            .unwrap();
        let (_, m) = c.finish();
        assert!(m.dangling_extends.is_empty());
        assert_eq!(m.problems(), ["metadata/items/parent.it: Failed"]);
    }

    #[test]
    fn parse_should_not_panic_on_any_prefix() {
        let bytes = utf16(RING);
        for n in 0..=bytes.len() {
            let _ = parse(&bytes[..n]);
        }
    }

    #[test]
    fn render_should_write_headers_and_one_block_per_line() {
        let json = render(&parse(&utf16(RING)).unwrap()).unwrap();
        let text = String::from_utf8(json).unwrap();
        assert_eq!(
            text,
            "{\n\"abstract\": true,\n\"blocks\": [\n\
             {\"blocks\":[],\"entries\":[{\"key\":\"x_size\",\"quoted\":false,\"value\":\"1\"},\
             {\"key\":\"tag\",\"quoted\":true,\"value\":\"ring\"}],\"name\":\"Base\"},\n\
             {\"blocks\":[],\"entries\":[{\"key\":\"inventory_type\",\"quoted\":true,\"value\":\"Ring\"}],\"name\":\"Mods\"}\n\
             ],\n\"comments\": 0,\n\"extends\": \"Metadata/Items/Equipment\",\n\"version\": 3\n}\n"
        );
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["blocks"][0]["entries"][1]["value"], "ring");
    }

    fn file(extends: &str) -> Vec<u8> {
        utf16(&format!("version 3\nextends \"{extends}\"\n"))
    }

    #[test]
    fn finish_should_check_the_extends_chain() {
        let mut c = Collection::new();
        c.add("Metadata/Items/Item.it", &file(ROOT)).unwrap();
        c.add(
            "metadata/items/rings/abstractring.it",
            &file("Metadata/Items/Item"),
        )
        .unwrap();
        c.add("metadata/items/lost.it", &file("Metadata/Items/Gone"))
            .unwrap();
        c.add("metadata/items/a.it", &file("Metadata/Items/B"))
            .unwrap();
        c.add("metadata/items/b.it", &file("Metadata/Items/A"))
            .unwrap();
        c.add("metadata/items/c.it", &file("Metadata/Items/A"))
            .unwrap();
        c.add("metadata/items/bad.it", b"odd").unwrap();
        let (files, m) = c.finish();
        assert_eq!(
            m.dangling_extends,
            BTreeMap::from([(
                "metadata/items/lost.it".to_owned(),
                "Metadata/Items/Gone".to_owned()
            )])
        );
        // c leads into the a-b loop but is not on it.
        assert_eq!(
            m.cycles,
            BTreeSet::from([
                "metadata/items/a.it".to_owned(),
                "metadata/items/b.it".to_owned()
            ])
        );
        assert_eq!(
            m.files["metadata/items/bad.it"].status,
            ItemTypeStatus::Failed
        );
        assert!(!files.contains_key("item_types/metadata/items/bad.it.json"));
        assert_eq!(
            m.files["metadata/items/item.it"].output.as_deref(),
            Some("item_types/metadata/items/item.it.json")
        );
        assert!(files.contains_key("item_types/metadata/items/rings/abstractring.it.json"));
        assert_eq!(
            m.problems(),
            [
                "metadata/items/bad.it: Failed",
                "metadata/items/lost.it: extends \"Metadata/Items/Gone\", which is not in the index",
                "metadata/items/a.it: on an extends cycle",
                "metadata/items/b.it: on an extends cycle",
            ]
        );
    }

    #[test]
    fn collection_should_refuse_other_files_and_duplicates() {
        let mut c = Collection::new();
        assert_eq!(
            c.add("metadata/items/x.ot", &file(ROOT))
                .unwrap_err()
                .code(),
            "EXTRACT_UNPLANNED_FILE"
        );
        c.add("metadata/items/x.it", &file(ROOT)).unwrap();
        assert_eq!(
            c.add("Metadata/Items/X.it", &file(ROOT))
                .unwrap_err()
                .code(),
            "EXTRACT_DUPLICATE_FILE"
        );
        c.add("metadata/../x.it", &file(ROOT)).unwrap();
        let (_, m) = c.finish();
        assert_eq!(
            m.files["metadata/../x.it"].error.as_ref().map(|e| e.code),
            Some("EXTRACT_IT_BAD_PATH")
        );
        assert_eq!(
            Collection::new().finish().1.problems(),
            ["no .it files in the index"]
        );
    }
}
