//! Readers for the Path of Building files the cross-check compares against:
//! generated data (`src/Data/**`), export specs (`src/Export/{Skills,Bases}`)
//! and the export scripts' file lists (`src/Export/Scripts/*.lua`).
//!
//! PoB's data is Lua, but only a few line shapes carry keys, so each reader
//! matches one narrow pattern instead of parsing Lua. A line that starts like
//! a key but does not complete the pattern is returned in `unparsed`, never
//! skipped (ADR-015).

use serde::Serialize;

/// A line that started like a key but did not parse.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Unparsed {
    /// 1-based line number.
    pub line: usize,
    /// The line as written.
    pub text: String,
}

/// Keys read from one file, in file order.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Keys {
    /// Keys, in file order (a key may repeat).
    pub keys: Vec<String>,
    /// Lines that looked like keys but did not parse.
    pub unparsed: Vec<Unparsed>,
}

impl Keys {
    fn bad(&mut self, line: usize, text: &str) {
        self.unparsed.push(Unparsed {
            line,
            text: text.to_owned(),
        });
    }
}

/// Reads a Lua double-quoted string at the start of `s`. Returns the value
/// and the rest after the closing quote; `None` if `s` does not start with a
/// quote, the string does not end, or it uses an escape other than `\"`,
/// `\\`, `\'`, or `\n`.
fn lua_string(s: &str) -> Option<(String, &str)> {
    let body = s.strip_prefix('"')?;
    let mut value = String::new();
    let mut chars = body.char_indices();
    while let Some((i, c)) = chars.next() {
        match c {
            '"' => return Some((value, body.get(i + 1..)?)),
            '\\' => match chars.next()?.1 {
                '"' => value.push('"'),
                '\\' => value.push('\\'),
                '\'' => value.push('\''),
                'n' => value.push('\n'),
                _ => return None,
            },
            c => value.push(c),
        }
    }
    None
}

/// Lines `<prefix>"<key>"] = {`, the shape of every keyed entry PoB writes
/// (`\t["Id"] = {` in mod files, `itemBases["Name"] = {`, `skills["Id"] = {`).
/// A line that starts with `<prefix>"` but does not end the pattern is unparsed.
fn keyed_lines(text: &str, prefix: &str) -> Keys {
    let mut out = Keys::default();
    for (i, line) in text.lines().enumerate() {
        let Some(rest) = line.strip_prefix(prefix).filter(|r| r.starts_with('"')) else {
            continue;
        };
        match lua_string(rest) {
            Some((key, tail)) if tail.starts_with("] = {") => out.keys.push(key),
            _ => out.bad(i + 1, line),
        }
    }
    out
}

/// Mod IDs in a generated mod file (`src/Data/Mod*.lua`): `\t["Id"] = {`.
#[must_use]
pub fn mod_keys(text: &str) -> Keys {
    keyed_lines(text, "\t[")
}

/// Base names in a generated base file (`src/Data/Bases/*.lua`):
/// `itemBases["Name"] = {`.
#[must_use]
pub fn base_keys(text: &str) -> Keys {
    keyed_lines(text, "itemBases[")
}

/// Skill IDs in a generated skill file (`src/Data/Skills/*.lua`) or written
/// by hand in a skill spec (`src/Export/Skills/*.txt`): `skills["Id"] = {`.
#[must_use]
pub fn skill_keys(text: &str) -> Keys {
    keyed_lines(text, "skills[")
}

/// Stat IDs in a converted stat-description file
/// (`src/Data/StatDescriptions/**/*.lua`): the `[n]="stat_id"` lines inside
/// each `stats={ ... }` block.
#[must_use]
pub fn stat_description_stats(text: &str) -> Keys {
    let mut out = Keys::default();
    let mut in_stats = false;
    for (i, line) in text.lines().enumerate() {
        let t = line.trim();
        if !in_stats {
            in_stats = t == "stats={";
            continue;
        }
        if t == "}" || t == "}," {
            in_stats = false;
            continue;
        }
        let stat = t
            .strip_prefix('[')
            .and_then(|r| r.split_once("]="))
            .filter(|(n, _)| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
            .and_then(|(_, r)| lua_string(r))
            .filter(|(_, tail)| tail.is_empty() || *tail == ",");
        match stat {
            Some((stat, _)) => out.keys.push(stat),
            None => out.bad(i + 1, line),
        }
    }
    if in_stats {
        out.bad(text.lines().count(), "end of file inside stats={");
    }
    out
}

/// Passive node IDs (`PassiveSkillGraphId`) in a tree file
/// (`src/TreeData/<version>/tree.lua`): `["skill"]= <n>` (PoE1) or
/// `skill=<n>` (PoE2), one per line.
#[must_use]
pub fn tree_nodes(text: &str) -> Keys {
    let mut out = Keys::default();
    for (i, line) in text.lines().enumerate() {
        let t = line.trim();
        let Some(rest) = t
            .strip_prefix("[\"skill\"]=")
            .or_else(|| t.strip_prefix("skill="))
        else {
            continue;
        };
        let n = rest.trim();
        let n = n.strip_suffix(',').unwrap_or(n);
        match n.parse::<u32>() {
            Ok(n) => out.keys.push(n.to_string()),
            Err(_) => out.bad(i + 1, line),
        }
    }
    out
}

/// A skill spec (`src/Export/Skills/<name>.txt`).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SkillSpec {
    /// Granted-effect IDs named by `#skill <Id> [display name]`.
    pub listed: Vec<String>,
    /// Skills written out by hand (`skills["Id"] = {`), not generated.
    pub hand_written: Vec<String>,
    /// `#skill` lines without an ID, and hand-written lines that did not parse.
    pub unparsed: Vec<Unparsed>,
}

/// Reads a skill spec.
#[must_use]
pub fn skill_spec(text: &str) -> SkillSpec {
    let hand = skill_keys(text);
    let mut spec = SkillSpec {
        hand_written: hand.keys,
        unparsed: hand.unparsed,
        ..SkillSpec::default()
    };
    for (i, line) in text.lines().enumerate() {
        // `#skillEnd` and other `#skill...` directives are not `#skill`.
        let Some(rest) = line
            .strip_prefix("#skill")
            .filter(|r| r.is_empty() || r.starts_with(char::is_whitespace))
        else {
            continue;
        };
        {
            match rest.split_whitespace().next() {
                Some(id) => spec.listed.push(id.to_owned()),
                _ => spec.unparsed.push(Unparsed {
                    line: i + 1,
                    text: line.to_owned(),
                }),
            }
        }
    }
    spec.unparsed.sort();
    spec
}

/// One `#baseMatch` directive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseMatch {
    /// PoB's column name (`Id` when omitted).
    pub column: String,
    /// The value, matched as a Lua pattern against the column.
    pub value: String,
}

/// A base spec (`src/Export/Bases/<name>.txt`).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct BaseSpec {
    /// `#base <Id> [display name]` directives.
    pub bases: Vec<(String, Option<String>)>,
    /// `#baseMatch [<column>] <value>` directives.
    pub matches: Vec<BaseMatch>,
    /// `#base` and `#baseMatch` lines with the wrong number of words.
    pub unparsed: Vec<Unparsed>,
}

/// Reads a base spec.
#[must_use]
pub fn base_spec(text: &str) -> BaseSpec {
    let mut spec = BaseSpec::default();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim_end();
        let directive = |name: &str| {
            line.strip_prefix(name)
                .filter(|r| r.is_empty() || r.starts_with(' '))
        };
        if let Some(rest) = directive("#baseMatch") {
            let words: Vec<&str> = rest.split_whitespace().collect();
            match words.as_slice() {
                [value] => spec.matches.push(BaseMatch {
                    column: "Id".to_owned(),
                    value: (*value).to_owned(),
                }),
                [column, value] => spec.matches.push(BaseMatch {
                    column: (*column).to_owned(),
                    value: (*value).to_owned(),
                }),
                _ => spec.unparsed.push(Unparsed {
                    line: i + 1,
                    text: line.to_owned(),
                }),
            }
        } else if let Some(rest) = directive("#base") {
            let rest = rest.trim_start();
            match rest.split_once(' ') {
                _ if rest.is_empty() => spec.unparsed.push(Unparsed {
                    line: i + 1,
                    text: line.to_owned(),
                }),
                Some((id, name)) => spec
                    .bases
                    .push((id.to_owned(), Some(name.trim().to_owned()))),
                None => spec.bases.push((rest.to_owned(), None)),
            }
        }
    }
    spec
}

/// Patterns an export script excludes mod IDs by: the string in every
/// `not mod.Id:match("<pattern>")`, in script order, each once.
#[must_use]
pub fn excluded_id_patterns(script: &str) -> Vec<String> {
    const MARKER: &str = "not mod.Id:match(";
    let mut out: Vec<String> = Vec::new();
    let mut rest = script;
    while let Some(at) = rest.find(MARKER) {
        rest = rest.get(at + MARKER.len()..).unwrap_or("");
        if let Some((pattern, tail)) = lua_string(rest) {
            if !out.contains(&pattern) {
                out.push(pattern);
            }
            rest = tail;
        }
    }
    out
}

/// The quoted strings in the first call `<function>(...)` in a script, for
/// example the files `loadStatFile("a.txt", "b.txt")` loads; `None` if there
/// is no such call.
#[must_use]
pub fn call_args(script: &str, function: &str) -> Option<Vec<String>> {
    let marker = format!("{function}(");
    let start = script.find(&marker)? + marker.len();
    let args = script.get(start..)?;
    let args = args.get(..args.find(')')?)?;
    let mut out = Vec::new();
    let mut rest = args;
    while let Some(q) = rest.find('"') {
        let (value, tail) = lua_string(rest.get(q..)?)?;
        out.push(value);
        rest = tail;
    }
    Some(out)
}

/// PoB's `escapeGGGString` (src/Modules/Common.lua), which statdesc.lua
/// applies to every description text: `<tag>{text}` becomes `text`, then
/// `[Ref]` becomes `Ref`, then `[Ref|Text]` becomes `Text` (each a global
/// `gsub`, left to right).
#[must_use]
pub fn escape_ggg_string(text: &str) -> String {
    // <[^>]+>{([^}]+)}
    let step1 = replace_all(text, |rest| {
        let tag_end = rest.strip_prefix('<')?.find('>')?;
        (tag_end > 0).then_some(())?;
        let after = rest.get(tag_end + 2..)?.strip_prefix('{')?;
        let close = after.find('}')?;
        (close > 0).then_some(())?;
        Some((after.get(..close)?.to_owned(), tag_end + 2 + 1 + close + 1))
    });
    // %[([^|%]]+)%]
    let step2 = replace_all(&step1, |rest| {
        let body = rest.strip_prefix('[')?;
        let end = body.find(['|', ']'])?;
        (end > 0 && body.get(end..)?.starts_with(']')).then_some(())?;
        Some((body.get(..end)?.to_owned(), end + 2))
    });
    // %[[^|]+|([^|]+)%]
    replace_all(&step2, |rest| {
        let body = rest.strip_prefix('[')?;
        let bar = body.find('|')?;
        (bar > 0).then_some(())?;
        let tail = body.get(bar + 1..)?;
        let run = tail.find('|').unwrap_or(tail.len());
        let close = tail.get(..run)?.rfind(']')?;
        (close > 0).then_some(())?;
        Some((tail.get(..close)?.to_owned(), 1 + bar + 1 + close + 1))
    })
}

/// Scans `text` left to right; where `at` matches (returning the
/// replacement and the matched byte length), replaces it and continues
/// after the match, like a Lua `gsub`.
fn replace_all(text: &str, at: impl Fn(&str) -> Option<(String, usize)>) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        if let Some((replacement, len)) = at(rest) {
            out.push_str(&replacement);
            rest = rest.get(len..).unwrap_or("");
        } else {
            out.push(c);
            rest = rest.get(c.len_utf8()..).unwrap_or("");
        }
    }
    out
}

/// One single-character class of a Lua pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
enum CharClass {
    /// `.`
    Any,
    /// A literal character (also `%` followed by a non-letter).
    Char(char),
    /// `%a`, `%d`, ... (an upper-case letter negates).
    Named(char),
    /// `[...]` or `[^...]`.
    Set { negated: bool, items: Vec<SetItem> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SetItem {
    Char(char),
    Range(char, char),
    Named(char),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Repeat {
    One,
    /// `?`
    Optional,
    /// `*` (longest)
    Many,
    /// `+` (longest)
    OneOrMore,
    /// `-` (shortest)
    Few,
}

fn named_class(letter: char, c: char) -> Option<bool> {
    let hit = match letter.to_ascii_lowercase() {
        'a' => c.is_ascii_alphabetic(),
        'c' => c.is_ascii_control(),
        'd' => c.is_ascii_digit(),
        'l' => c.is_ascii_lowercase(),
        'p' => c.is_ascii_punctuation(),
        's' => c.is_ascii_whitespace(),
        'u' => c.is_ascii_uppercase(),
        'w' => c.is_ascii_alphanumeric(),
        'x' => c.is_ascii_hexdigit(),
        _ => return None,
    };
    Some(hit != letter.is_ascii_uppercase())
}

impl CharClass {
    fn matches(&self, c: char) -> bool {
        match self {
            Self::Any => true,
            Self::Char(x) => *x == c,
            Self::Named(letter) => named_class(*letter, c).unwrap_or(false),
            Self::Set { negated, items } => {
                let hit = items.iter().any(|item| match item {
                    SetItem::Char(x) => *x == c,
                    SetItem::Range(lo, hi) => (*lo..=*hi).contains(&c),
                    SetItem::Named(letter) => named_class(*letter, c).unwrap_or(false),
                });
                hit != *negated
            }
        }
    }
}

/// A Lua pattern as PoB's export scripts use it (`string.match`/`find`):
/// single-character classes (`.`, `%a`/`%d`/... and their negations,
/// `%` escapes, `[...]` sets with ranges), the repeats `?`, `*`, `+`, `-`,
/// and the anchors `^` and `$`. Captures, `%b`, `%f`, and back-references
/// are rejected, so an unsupported pattern fails instead of matching wrongly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LuaPattern {
    items: Vec<(CharClass, Repeat)>,
    anchored_start: bool,
    anchored_end: bool,
}

impl LuaPattern {
    /// Parses a pattern.
    ///
    /// # Errors
    /// A description of the unsupported or malformed part.
    pub fn parse(pattern: &str) -> Result<Self, String> {
        let (anchored_start, body) = match pattern.strip_prefix('^') {
            Some(rest) => (true, rest),
            None => (false, pattern),
        };
        let mut chars: Vec<char> = body.chars().collect();
        let anchored_end = chars.last() == Some(&'$')
            && chars.len().checked_sub(2).and_then(|i| chars.get(i)) != Some(&'%');
        if anchored_end {
            chars.pop();
        }
        let mut items = Vec::new();
        let mut i = 0;
        let next = |i: &mut usize| -> Option<char> {
            let c = chars.get(*i).copied();
            *i += 1;
            c
        };
        while let Some(c) = next(&mut i) {
            let class = match c {
                '.' => CharClass::Any,
                '(' | ')' => return Err(format!("captures are not supported in `{pattern}`")),
                '%' => match next(&mut i) {
                    Some(l @ ('b' | 'f')) => {
                        return Err(format!("`%{l}` is not supported in `{pattern}`"));
                    }
                    Some(l) if l.is_ascii_digit() => {
                        return Err(format!("back-references are not supported in `{pattern}`"));
                    }
                    Some(l) if l.is_ascii_alphabetic() => {
                        named_class(l, 'a')
                            .ok_or(format!("unknown class `%{l}` in `{pattern}`"))?;
                        CharClass::Named(l)
                    }
                    Some(l) => CharClass::Char(l),
                    None => return Err(format!("`{pattern}` ends with `%`")),
                },
                '[' => {
                    let negated = chars.get(i) == Some(&'^');
                    if negated {
                        i += 1;
                    }
                    let mut set = Vec::new();
                    let mut first = true;
                    loop {
                        let c = next(&mut i).ok_or(format!("unclosed `[` in `{pattern}`"))?;
                        match c {
                            ']' if !first => break,
                            '%' => {
                                let l = next(&mut i).ok_or(format!("`{pattern}` ends with `%`"))?;
                                if l.is_ascii_alphabetic() {
                                    named_class(l, 'a')
                                        .ok_or(format!("unknown class `%{l}` in `{pattern}`"))?;
                                    set.push(SetItem::Named(l));
                                } else {
                                    set.push(SetItem::Char(l));
                                }
                            }
                            c if chars.get(i) == Some(&'-')
                                && chars.get(i + 1).is_some_and(|h| *h != ']') =>
                            {
                                let hi = chars.get(i + 1).copied().unwrap_or(c);
                                i += 2;
                                set.push(SetItem::Range(c, hi));
                            }
                            c => set.push(SetItem::Char(c)),
                        }
                        first = false;
                    }
                    CharClass::Set {
                        negated,
                        items: set,
                    }
                }
                c => CharClass::Char(c),
            };
            let repeat = match chars.get(i) {
                Some('?') => Repeat::Optional,
                Some('*') => Repeat::Many,
                Some('+') => Repeat::OneOrMore,
                Some('-') => Repeat::Few,
                _ => Repeat::One,
            };
            if repeat != Repeat::One {
                i += 1;
            }
            items.push((class, repeat));
        }
        Ok(Self {
            items,
            anchored_start,
            anchored_end,
        })
    }

    /// Whether the pattern matches anywhere in `subject` (Lua `string.find`).
    #[must_use]
    pub fn find(&self, subject: &str) -> bool {
        let s: Vec<char> = subject.chars().collect();
        if self.anchored_start {
            return self.match_here(&self.items, &s);
        }
        (0..=s.len()).any(|start| {
            s.get(start..)
                .is_some_and(|rest| self.match_here(&self.items, rest))
        })
    }

    fn match_here(&self, items: &[(CharClass, Repeat)], s: &[char]) -> bool {
        let Some(((class, repeat), rest)) = items.split_first() else {
            return !self.anchored_end || s.is_empty();
        };
        let after = |n: usize| s.get(n..).is_some_and(|tail| self.match_here(rest, tail));
        let first = s.first().is_some_and(|c| class.matches(*c));
        match repeat {
            Repeat::One => first && after(1),
            Repeat::Optional => (first && after(1)) || after(0),
            Repeat::Many | Repeat::OneOrMore | Repeat::Few => {
                let max = s.iter().take_while(|c| class.matches(**c)).count();
                let min = usize::from(*repeat == Repeat::OneOrMore);
                if *repeat == Repeat::Few {
                    (min..=max).any(after)
                } else {
                    (min..=max).rev().any(after)
                }
            }
        }
    }
}

/// The quoted strings of the first `{ ... }` list after `marker` in an
/// export script, for example the spec files `skills.lua` processes
/// (`for _, name in pairs({"act_str", ...})`) or `bases.lua`'s
/// `local itemTypes = { ... }`. `None` if the marker or the closing brace is
/// missing, so a changed script fails loudly instead of matching nothing.
#[must_use]
pub fn quoted_list(text: &str, marker: &str) -> Option<Vec<String>> {
    let start = text.find(marker)?;
    let after = text.get(start + marker.len()..)?;
    let open = after.find('{')?;
    let list = after.get(open + 1..)?;
    let close = list.find('}')?;
    let list = list.get(..close)?;
    let mut out = Vec::new();
    let mut rest = list;
    while let Some(q) = rest.find('"') {
        let (value, tail) = lua_string(rest.get(q..)?)?;
        out.push(value);
        rest = tail;
    }
    Some(out)
}

#[cfg(test)]
#[expect(clippy::indexing_slicing, reason = "tests index known-length results")]
mod tests {
    use super::*;

    #[test]
    fn mod_keys_should_read_entries_and_report_broken_ones() {
        let text = "return {\n\t[\"Strength1\"] = { type = \"Suffix\" },\n\t[\"Al's \\\"x\\\"\"] = { },\n\t[\"Broken\" = { },\n\t[1] = 2,\n}";
        let keys = mod_keys(text);
        assert_eq!(keys.keys, ["Strength1", "Al's \"x\""]);
        assert_eq!(
            keys.unparsed,
            [Unparsed {
                line: 4,
                text: "\t[\"Broken\" = { },".to_owned()
            }]
        );
    }

    #[test]
    fn lua_string_should_reject_unknown_escapes_and_unterminated_strings() {
        assert_eq!(lua_string("\"a\\tb\""), None);
        assert_eq!(lua_string("\"abc"), None);
        assert_eq!(lua_string("abc\""), None);
        assert_eq!(lua_string("\"\" x"), Some((String::new(), " x")));
        assert_eq!(lua_string("\"a\\nb\""), Some(("a\nb".to_owned(), "")));
    }

    #[test]
    fn base_and_skill_keys_should_match_their_prefix_only() {
        let text = "itemBases[\"Rusted Sword\"] = {\n\titemBases[\"Indented\"] = {\nskills[\"Fireball\"] = {";
        assert_eq!(base_keys(text).keys, ["Rusted Sword"]);
        assert_eq!(skill_keys(text).keys, ["Fireball"]);
    }

    #[test]
    fn stat_description_stats_should_read_stats_blocks_only() {
        let text = "return {\n\t[1]={\n\t\t[1]={\n\t\t\t[1]={\n\t\t\t\ttext=\"x\"\n\t\t\t}\n\t\t},\n\t\tstats={\n\t\t\t[1]=\"a_stat\",\n\t\t\t[2]=\"b_stat\"\n\t\t}\n\t},\n\t[2]={\n\t\tname=\"n\",\n\t\tstats={\n\t\t\t[1]=\"0\"\n\t\t}\n\t}\n}";
        let keys = stat_description_stats(text);
        assert_eq!(keys.keys, ["a_stat", "b_stat", "0"]);
        assert!(keys.unparsed.is_empty());
    }

    #[test]
    fn stat_description_stats_should_report_odd_lines_and_unclosed_blocks() {
        let keys = stat_description_stats("stats={\n[x]=\"a\"\n[1]=\"b\" junk\n[1]=\"c\",\n");
        assert_eq!(keys.keys, ["c"]);
        assert_eq!(keys.unparsed.len(), 3);
        assert_eq!(keys.unparsed[2].text, "end of file inside stats={");
    }

    #[test]
    fn tree_nodes_should_read_both_games_shapes() {
        let keys =
            tree_nodes("\t\t[\"skill\"]= 28609,\n\t\tskill=4\n\tskillsPerOrbit= {\n\tskill=x\n");
        assert_eq!(keys.keys, ["28609", "4"]);
        assert_eq!(keys.unparsed.len(), 1);
    }

    #[test]
    fn skill_spec_should_split_listed_and_hand_written_skills() {
        let spec = skill_spec(
            "#skill Fireball\n#flags spell\n#mods\n#skill Arc Arc Display\n#skill\nskills[\"Manual\"] = {\n#skillEnd\n",
        );
        assert_eq!(spec.listed, ["Fireball", "Arc"]);
        assert_eq!(spec.hand_written, ["Manual"]);
        assert_eq!(spec.unparsed.len(), 1);
        assert_eq!(spec.unparsed[0].line, 5);
    }

    #[test]
    fn base_spec_should_read_bases_and_matches() {
        let spec = base_spec(
            "#type Ring\n#base Metadata/Items/Rings/Ring12 Two-Stone Ring (Fire/Lightning)\n#base Metadata/Items/Rings/Ring1\n#baseMatch Metadata/Items/Rings/Ring\n#baseMatch BaseType Metadata/Items/Rings/AbstractRing\n#baseMatch a b c\n#base \n",
        );
        assert_eq!(
            spec.bases,
            [
                (
                    "Metadata/Items/Rings/Ring12".to_owned(),
                    Some("Two-Stone Ring (Fire/Lightning)".to_owned())
                ),
                ("Metadata/Items/Rings/Ring1".to_owned(), None),
            ]
        );
        assert_eq!(
            spec.matches,
            [
                BaseMatch {
                    column: "Id".to_owned(),
                    value: "Metadata/Items/Rings/Ring".to_owned()
                },
                BaseMatch {
                    column: "BaseType".to_owned(),
                    value: "Metadata/Items/Rings/AbstractRing".to_owned()
                },
            ]
        );
        assert_eq!(spec.unparsed.len(), 2);
    }

    #[test]
    fn escape_ggg_string_should_strip_markup_like_pob() {
        assert_eq!(
            escape_ggg_string("[DNT] Minions are Aggressive"),
            "DNT Minions are Aggressive"
        );
        assert_eq!(escape_ggg_string("Gain [Onslaught]"), "Gain Onslaught");
        assert_eq!(
            escape_ggg_string("Deal [Hit|Hits] and [A|B]"),
            "Deal Hits and B"
        );
        assert_eq!(escape_ggg_string("<unique>{Name} here"), "Name here");
        // As in Lua: `[^|]+` in the third pattern runs across `] x [`.
        assert_eq!(escape_ggg_string("{0}% [] x [|y]"), "{0}% y");
        assert_eq!(escape_ggg_string("[] and [|y"), "[] and [|y");
        assert_eq!(escape_ggg_string("plain"), "plain");
    }

    #[test]
    fn call_args_should_read_the_first_call() {
        let script = "if not loadStatFile then
	dofile(\"statdesc.lua\")
end
loadStatFile(\"a.txt\", \"b.txt\")
loadStatFile(\"c.txt\")";
        assert_eq!(
            call_args(script, "loadStatFile"),
            Some(vec!["a.txt".to_owned(), "b.txt".to_owned()])
        );
        assert_eq!(call_args(script, "missing"), None);
    }

    #[test]
    fn excluded_id_patterns_should_read_each_pattern_once() {
        let script = "and not mod.Id:match(\"Royale\")
and not mod.Id:match(\"^Synthesis\") and not mod.Id:match(\"Royale\")
mod.Id:match(\"Kept\")";
        assert_eq!(excluded_id_patterns(script), ["Royale", "^Synthesis"]);
    }

    #[test]
    fn lua_pattern_should_match_like_string_find() {
        let find = |p: &str, s: &str| LuaPattern::parse(p).unwrap().find(s);
        assert!(find(
            "Wands/Wand[MEK]?%d+",
            "Metadata/Items/Weapons/Wands/WandM12"
        ));
        assert!(find(
            "Wands/Wand[MEK]?%d+",
            "Metadata/Items/Weapons/Wands/Wand3"
        ));
        assert!(!find(
            "Wands/Wand[MEK]?%d+",
            "Metadata/Items/Weapons/Wands/WandX3"
        ));
        assert!(find("Rings/Ring", "Metadata/Items/Rings/Ring12"));
        assert!(find("^Meta", "Metadata"));
        assert!(!find("^data", "Metadata"));
        assert!(find("Ring%d$", "Ring1"));
        assert!(!find("Ring%d$", "Ring12"));
        assert!(find("a.c", "xabcx"));
        assert!(find("[^%d]x", "ax"));
        assert!(!find("^[^%d]x", "1x"));
        assert!(find("[a-c]+z", "bbaz"));
        assert!(find("a%-b", "a-b"));
        assert!(find("a-b", "aaab"));
        assert!(find("%D", "1a"));
        assert!(find("x*$", ""));
    }

    #[test]
    fn lua_pattern_should_reject_unsupported_parts() {
        for bad in ["(a)", "%b()", "%f[%w]", "%1", "%", "[abc", "%q"] {
            assert!(LuaPattern::parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn quoted_list_should_read_one_line_and_multi_line_lists() {
        let skills = "x\nfor _, name in pairs({\"act_str\",\"glove\"}) do\n";
        assert_eq!(
            quoted_list(skills, "for _, name in pairs("),
            Some(vec!["act_str".to_owned(), "glove".to_owned()])
        );
        let bases = "local itemTypes = {\n\t\"axe\",\n\t\"bow\",\n}\n";
        assert_eq!(
            quoted_list(bases, "local itemTypes ="),
            Some(vec!["axe".to_owned(), "bow".to_owned()])
        );
        assert_eq!(quoted_list(bases, "missing"), None);
        assert_eq!(
            quoted_list("local itemTypes = { \"a\"", "local itemTypes ="),
            None
        );
    }
}
