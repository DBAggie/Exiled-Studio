//! Monster object templates (`Metadata/Monsters/**.ot` but the monster base,
//! [`is_monster_template`]; Phase 6 P6-7b1):
//! the `.ot` stats of a minion, which PoB v2.67.2 exports into each
//! minion's `modList` (`src/Export/Scripts/minions.lua` `getOTStats`, 41
//! to 73).
//!
//! These files hold script blocks (`on_death = { RemoveEffects(); }`), a
//! nested `client` section, and several `extends` lines, which the item
//! type grammar ([`crate::itemtype`]) rejects. They are not parsed as a
//! grammar: each file is read line by line as PoB's reader reads it, and
//! what that reader keeps is written in file order ([`Line`]):
//! - every line that matches `extends "(.+)"` (anywhere in the line; the
//!   capture is greedy, up to the line's last quote) gives an `extends`
//!   target, kept as written, `Metadata/Monsters/Monster` and `nothing`
//!   included;
//! - a line that starts with `Stats` opens the stats block, and the next
//!   line that starts with `}` closes it;
//! - inside it, a line with `=` and without `//` loses all whitespace
//!   (Lua's `%s`) and splits at its first `=` into a key (possibly empty)
//!   and a value (not empty), kept as text.
//!
//! Lines are split at `\r` and `\n`, and empty lines are skipped, as Lua's
//! `[^\r\n]+`. Resolving the chains (PoB stops at `Metadata/Monsters/Monster`
//! and `nothing`, reads a superclass where its `extends` line stands, and
//! keeps repeats) and reading values as numbers (`tonumber`) are the
//! consumer's: the PoE1 bundle's `minions.json` (`tools/parity-cli`).
//!
//! A file is `failed` only when it is not UTF-16LE text or its path is not
//! a plain relative path. An `extends` target outside the folder is kept as
//! written; the consumer checks the chains it needs.

use std::collections::BTreeMap;

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::extract::{ExtractError, TableError};
use crate::install::hex;
use crate::statdesc::{StatDescError, decode_utf16le, plain_relative};

/// Output folder of the monster templates.
pub const MONSTER_TEMPLATES_DIR: &str = "monster_templates";

/// The folder read (lowercase index path).
pub const MONSTERS_DIR: &str = "metadata/monsters/";

/// The extension of an object template.
pub const EXTENSION: &str = ".ot";

/// Whether `path` (as listed in the index) is a monster template: an `.ot`
/// file anywhere under `Metadata/Monsters/`, except the monster base
/// (`Metadata/Monsters/Monster.ot`, [`crate::itemtype::MONSTER_TEMPLATE`]),
/// which the object templates read (its default monster stats) and PoB's
/// reader never reads (it stops there).
#[must_use]
pub fn is_monster_template(path: &str) -> bool {
    let path = path.to_ascii_lowercase();
    if path == crate::itemtype::MONSTER_TEMPLATE {
        return false;
    }
    path.strip_prefix(MONSTERS_DIR)
        .and_then(|rest| rest.strip_suffix(EXTENSION))
        .is_some_and(|rest| !rest.is_empty() && !rest.ends_with('/'))
}

/// What PoB's reader keeps from one line (module docs).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Line {
    /// An `extends` target, as written.
    Extends(String),
    /// A stat line of the stats block, without whitespace.
    Stat {
        /// The text before the first `=` (possibly empty).
        key: String,
        /// The text after it (not empty).
        value: String,
    },
}

/// Reads a file as PoB's `getOTStats` does (module docs).
///
/// # Errors
/// [`StatDescError`] if the bytes are not UTF-16LE text.
pub fn read(bytes: &[u8]) -> Result<Vec<Line>, StatDescError> {
    let text = decode_utf16le(bytes)?;
    let mut out = Vec::new();
    let mut in_stats = false;
    for line in text.split(['\r', '\n']).filter(|l| !l.is_empty()) {
        if let Some(target) = extends_target(line) {
            out.push(Line::Extends(target.to_owned()));
        }
        if line.starts_with("Stats") {
            in_stats = true;
        } else if in_stats && line.starts_with('}') {
            in_stats = false;
        } else if in_stats && line.contains('=') && !line.contains("//") {
            let squeezed: String = line.chars().filter(|&c| !lua_space(c)).collect();
            if let Some((key, value)) = squeezed.split_once('=')
                && !value.is_empty()
            {
                out.push(Line::Stat {
                    key: key.to_owned(),
                    value: value.to_owned(),
                });
            }
        }
    }
    Ok(out)
}

/// Lua's `%s` in the C locale: space, `\t`, `\n`, `\v`, `\f`, `\r`.
const fn lua_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\u{b}' | '\u{c}' | '\r')
}

/// The capture of Lua's `line:match('extends "(.+)"')`: from the first
/// `extends "`, at least one character up to the line's last quote.
fn extends_target(line: &str) -> Option<&str> {
    const OPEN: &str = "extends \"";
    let start = line.find(OPEN)? + OPEN.len();
    let rest = &line[start..];
    let end = rest.rfind('"')?;
    (end > 0).then(|| &rest[..end])
}

/// Outcome of one file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MonsterTemplateStatus {
    /// Read; the output file holds its lines.
    Ok,
    /// Not text, or a bad path: see `error`. No output file.
    Failed,
}

/// One monster template in the manifest. Fields in alphabetical order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MonsterTemplateEntry {
    /// Why the file was not read.
    pub error: Option<TableError>,
    /// `extends` lines kept.
    pub extends: Option<usize>,
    /// Output file path.
    pub output: Option<String>,
    /// Path in the index.
    pub source: String,
    /// SHA-256 of the file as read from its (decoded) bundle.
    pub source_sha256: String,
    /// Stat lines kept.
    pub stats: Option<usize>,
    /// Outcome.
    pub status: MonsterTemplateStatus,
}

/// The monster template section of the manifest (format 6).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct MonsterTemplateManifest {
    /// Output folder.
    pub dir: &'static str,
    /// Every monster template in the index, by lowercase index path.
    pub files: BTreeMap<String, MonsterTemplateEntry>,
}

impl MonsterTemplateManifest {
    /// Problems that make the run fail: no monster template at all, or a
    /// file that was not read.
    #[must_use]
    pub fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if self.files.is_empty() {
            problems.push(format!("no {EXTENSION} files in {MONSTERS_DIR}"));
        }
        for (path, file) in &self.files {
            if file.status == MonsterTemplateStatus::Failed {
                problems.push(format!("{path}: Failed"));
            }
        }
        problems
    }
}

/// A file read: its source path, hash, and `(extends, stats, json)` or why
/// it was not read.
type Added = (
    String,
    [u8; 32],
    Result<(usize, usize, Vec<u8>), TableError>,
);

/// Collects the monster templates of one game in any order.
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

    /// Reads the file at `path` (as listed in the index). A file that is not
    /// read is recorded, not returned.
    ///
    /// # Errors
    /// [`ExtractError`] if `path` is not a monster template, was added
    /// before, or JSON cannot be written.
    pub fn add(&mut self, path: &str, bytes: &[u8]) -> Result<(), ExtractError> {
        if !is_monster_template(path) {
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
            match read(bytes) {
                Ok(lines) => {
                    let extends = lines
                        .iter()
                        .filter(|l| matches!(l, Line::Extends(_)))
                        .count();
                    Ok((extends, lines.len() - extends, render(&lines)?))
                }
                Err(e) => Err(TableError {
                    code: match e {
                        StatDescError::OddLength { .. } => "EXTRACT_OT_ODD_LENGTH",
                        _ => "EXTRACT_OT_BAD_UTF16",
                    },
                    message: e.to_string(),
                }),
            }
        } else {
            Err(TableError {
                code: "EXTRACT_OT_BAD_PATH",
                message: format!("{path} is not a plain relative path"),
            })
        };
        self.added
            .insert(key, (path.to_owned(), Sha256::digest(bytes).into(), result));
        Ok(())
    }

    /// Builds the output files (by path inside the output directory) and the
    /// manifest section.
    #[must_use]
    pub fn finish(self) -> (BTreeMap<String, Vec<u8>>, MonsterTemplateManifest) {
        let mut files = BTreeMap::new();
        let mut entries = BTreeMap::new();
        for (key, (source, sha256, result)) in self.added {
            let mut entry = MonsterTemplateEntry {
                error: None,
                extends: None,
                output: None,
                source,
                source_sha256: hex(&sha256),
                stats: None,
                status: MonsterTemplateStatus::Failed,
            };
            match result {
                Ok((extends, stats, json)) => {
                    let out = format!("{MONSTER_TEMPLATES_DIR}/{key}.json");
                    entry.extends = Some(extends);
                    entry.stats = Some(stats);
                    entry.output = Some(out.clone());
                    entry.status = MonsterTemplateStatus::Ok;
                    files.insert(out, json);
                }
                Err(error) => entry.error = Some(error),
            }
            entries.insert(key, entry);
        }
        (
            files,
            MonsterTemplateManifest {
                dir: MONSTER_TEMPLATES_DIR,
                files: entries,
            },
        )
    }
}

/// Renders a file's lines as JSON, one line per line: `{"lines": [...]}`.
fn render(lines: &[Line]) -> Result<Vec<u8>, ExtractError> {
    let mut out = String::from("{\n\"lines\": [");
    for (i, line) in lines.iter().enumerate() {
        out.push_str(if i == 0 { "\n" } else { ",\n" });
        out.push_str(&serde_json::to_string(line)?);
    }
    out.push_str(if lines.is_empty() {
        "]\n}\n"
    } else {
        "\n]\n}\n"
    });
    Ok(out.into_bytes())
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
        let mut out = vec![0xff, 0xfe];
        for u in text.encode_utf16() {
            out.extend_from_slice(&u.to_le_bytes());
        }
        out
    }

    fn stat(key: &str, value: &str) -> Line {
        Line::Stat {
            key: key.to_owned(),
            value: value.to_owned(),
        }
    }

    #[test]
    fn monster_templates_should_be_every_ot_file_under_monsters() {
        for (path, wanted) in [
            ("metadata/monsters/monster.ot", false),
            ("Metadata/Monsters/Monster.ot", false),
            ("Metadata/Monsters/Zombies/Zombie.ot", true),
            ("metadata/monsters/a/b/c/d.ot", true),
            ("metadata/monsters/.ot", false),
            ("metadata/monsters/zombie/.ot", false),
            ("metadata/monstersx/zombie.ot", false),
            ("metadata/monsters/zombie.it", false),
            ("metadata/characters/character.ot", false),
        ] {
            assert_eq!(is_monster_template(path), wanted, "{path}");
        }
    }

    /// The reader keeps what PoB's `getOTStats` keeps, in file order.
    #[test]
    fn read_should_keep_what_pobs_reader_keeps() {
        let text = "version 3\r\nabstract\r\nextends \"Metadata/Monsters/Monster\"\r\n\
            extends \"Metadata/Monsters/BossLifeBar\"\r\n\r\nStats\r\n{\r\n\
            \tset_item_drop_slots = 0\r\n\tset_base_mana_cost_-% = -5\r\n\
            \t// commented = 1\r\n\tlevel = 3 // trailing\r\n\tflag = true\r\n\
            \t = 7\r\n\tnovalue =\r\n\tno_equals\r\n}\r\nActor\r\n{\r\n\
            \tbasic_action = \"Emerge\"\r\n}\r\nclient\r\n{\r\n\tStats\r\n\t{\r\n\
            \t\tinner = 1\r\n\t}\r\n}\r\n";
        assert_eq!(
            read(&utf16(text)).unwrap(),
            [
                Line::Extends("Metadata/Monsters/Monster".to_owned()),
                Line::Extends("Metadata/Monsters/BossLifeBar".to_owned()),
                stat("set_item_drop_slots", "0"),
                stat("set_base_mana_cost_-%", "-5"),
                stat("flag", "true"),
                stat("", "7"),
            ]
        );
    }

    /// A `Stats` block stays open until a line starts with `}`, as PoB's
    /// `^}`: an indented brace does not close it.
    #[test]
    fn read_should_close_the_stats_block_only_at_a_line_starting_with_a_brace() {
        let text = "Stats\n{\n\ta = 1\n\t}\n\tb = 2\n}\nc = 3\n";
        assert_eq!(
            read(&utf16(text)).unwrap(),
            [stat("a", "1"), stat("b", "2")]
        );
    }

    #[test]
    fn extends_target_should_match_luas_greedy_capture() {
        assert_eq!(extends_target("extends \"A/B\""), Some("A/B"));
        assert_eq!(
            extends_target("\textends \"A\" // \"x\""),
            Some("A\" // \"x")
        );
        assert_eq!(extends_target("extends \"\""), None);
        assert_eq!(extends_target("extends A"), None);
    }

    #[test]
    fn finish_should_write_lines_and_record_failures() {
        let mut c = Collection::new();
        c.add(
            "Metadata/Monsters/Zombie.ot",
            &utf16("extends \"nothing\"\nStats\n{\n\tx = 1\n}\n"),
        )
        .unwrap();
        c.add("metadata/monsters/bad.ot", &[0xff, 0xfe, 0x00])
            .unwrap();
        assert!(c.add("metadata/monsters/zombie.ot", &[]).is_err());
        assert!(c.add("metadata/items/x.ot", &[]).is_err());
        let (files, manifest) = c.finish();
        assert_eq!(
            String::from_utf8(files["monster_templates/metadata/monsters/zombie.ot.json"].clone())
                .unwrap(),
            "{\n\"lines\": [\n{\"extends\":\"nothing\"},\n{\"stat\":{\"key\":\"x\",\"value\":\"1\"}}\n]\n}\n"
        );
        let ok = &manifest.files["metadata/monsters/zombie.ot"];
        assert_eq!((ok.extends, ok.stats), (Some(1), Some(1)));
        assert_eq!(manifest.problems(), ["metadata/monsters/bad.ot: Failed"]);
        assert_eq!(
            manifest.files["metadata/monsters/bad.ot"]
                .error
                .as_ref()
                .map(|e| e.code),
            Some("EXTRACT_OT_ODD_LENGTH")
        );
    }

    #[test]
    fn problems_should_flag_an_empty_collection() {
        let (_, manifest) = Collection::new().finish();
        assert_eq!(manifest.problems(), ["no .ot files in metadata/monsters/"]);
    }
}
