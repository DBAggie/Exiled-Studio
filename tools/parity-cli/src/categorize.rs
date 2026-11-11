//! Categories of lines `resolve-corpus` does not resolve (Spike B, PR B4b;
//! plan step 4b). Every such line gets exactly one category, the first
//! whose rule holds, with its evidence; a line no rule explains is
//! `uncategorized`, which fails the gate.
//!
//! The rules compare **word keys**: a text lowercased, every number or
//! range `(a-b)` (and every placeholder of a template) replaced by `#`,
//! every character that is not a letter or `#` turned into a space, runs
//! of `#` joined, spaces collapsed. So `+(17-19)% to Fire Damage over Time
//! Multiplier` and `+{0}% to Fire Damage over Time Multiplier` have the same
//! key. Sources of game text, each recorded in the summary:
//! - every variant line of every stat-description file in the extract
//!   (current game text), by file;
//! - `ClientStrings` texts of the ids in [`CLIENT_STRINGS`] (game text the
//!   client composes around or instead of a stat description);
//! - PoB's item text data at named tags (`--pob <tag>=<dir>`, a `git
//!   archive` of `src/Data`): mod files, enchantments, cluster jewels,
//!   uniques. Legacy evidence only (ADR-005: a cross-check, never an input
//!   to resolution).
//!
//! Rules, first match wins (the plan's list, step 4b, as implemented):
//! 1. `special-case/unknown-base`: the item's base was not found.
//! 2. `parser-gap/ambiguous`: candidates remain after the `Mods` filter.
//! 3. `parser-gap/only-shadowed`: only matches shadowed by include
//!    precedence.
//! 4. `parser-gap/unsupported-handler` (every rejection is a handler
//!    reason) / `parser-gap/near-miss`: a template matched and no variant
//!    verified.
//! 5. From here only `no-template` lines. `special-case/author-note`
//!    (`color-code`): the line starts with a PoB colour escape (`^0`..`^9`,
//!    `^x`); applied only while no stat description or `ClientStrings`
//!    text holds one (`colour_code_in_game_text`). **Excluded**
//!    (maintainer approval 2026-11-10: author notes are left out of the
//!    rate).
//! 6. `parser-gap/split-section` (since PR B5e a safety net: `resolve-corpus`
//!    resolves such lines first, so the corpus no longer reaches it): the
//!    line and the adjacent line of another
//!    section resolve as one multi-line description (PoB's `Implicits:`
//!    count put the two lines of one description in two sections).
//! 7. `special-case/author-note` (`pob-note-spec`): a `Note: ...` line,
//!    PoB's own note field (`Note` is a `ParseRaw` spec key). Excluded.
//! 8. `special-case/item-property`: a spec line `ParseRaw` knows with a
//!    number as its value (`Memory Strands: 63`), or one of the named
//!    `ClientStrings` item-display properties (`Lasts {0} Seconds`,
//!    `Currently has {0} Charges`): item properties pasted from the game,
//!    not modifiers. Since PR B5d `resolve-corpus` resolves these lines as
//!    typed properties first (`properties`), so a line gets here only when
//!    its value or label matches no game data.
//! 9. `parser-gap/client-string`: a line matching a `ClientStrings` text the
//!    client composes a modifier with (`While a Unique Enemy is in your
//!    Presence, {0}`, `Uses level {1} {0}`); the composed remainder is not
//!    resolved, and the evidence says so. Since PR B5c the resolver reads
//!    the [`COMPOSITIONS`] wrappers itself, so a presence line gets here
//!    only when its remainder does not resolve.
//! 10. `parser-gap/near-miss`: the word key equals a variant line of the
//!     item's scope (case, sign, missing value, or one line of a multi-line
//!     variant); a lookup variant's placeholder matches one or more words.
//! 11. `data-gap/other-scope`: the same against another file only.
//! 12. `data-gap/legacy-text`: the word key equals a PoB text line and no
//!     current game text. **Excluded** (maintainer decision 2026-11-10,
//!     B5b). Needs a PoB source, as rule 13.
//! 13. `special-case/author-note` (`no-game-text`): the word key equals no
//!     line of any source above, so it is not modifier text of the game
//!     now or in PoB's data. **Excluded** (as rule 5). Needs at least one
//!     PoB source; without one the line is `uncategorized`.
//! 14. `uncategorized`: any other outcome.

use std::collections::{BTreeMap, BTreeSet};

use loom_data::stat_desc::{Handler, Segment, StatDescriptions};
use loom_parser::resolve::compose::Composition;
use loom_serialization::legacy::pob_item;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::extract::{Read, Table};
use crate::render_check;

/// `ClientStrings` ids (exact, or a prefix ending in `*`) and what their
/// lines are. Ids checked on PoE1 26530219.
pub const CLIENT_STRINGS: [(&str, ClientKind); 12] = [
    // The item display's property lines, each id named (a prefix would take
    // ~150 rows, single words and `Requires {0}` among them).
    ("ItemDisplayFlaskBuffDuration1Second", ClientKind::Property),
    ("ItemDisplayFlaskBuffDurationNSeconds", ClientKind::Property),
    (
        "ItemDisplayChargesConsumptionOnUse1Charge",
        ClientKind::Property,
    ),
    (
        "ItemDisplayChargesConsumptionOnUseNCharges",
        ClientKind::Property,
    ),
    ("ItemDisplayCharges1Charge", ClientKind::Property),
    ("ItemDisplayChargesNCharges", ClientKind::Property),
    ("ItemDisplayChargesCurrentOfMax1", ClientKind::Property),
    ("ItemDisplayChargesCurrentOfMaxN", ClientKind::Property),
    ("ItemDisplayHasEffect", ClientKind::Property),
    // `While a Unique Enemy is in your Presence, {0}` and `While a Pinnacle
    // Atlas Boss is in your Presence, {0}`: eldritch implicits with
    // `local_influence_mod_requires_*_presence` (Mods rows
    // `*EldritchImplicitUniquePresence*`, `*PinnaclePresence*`).
    (
        "InfluenceStatConditionPresenceUniqueMonster",
        ClientKind::Modifier,
    ),
    (
        "InfluenceStatConditionPresenceCelestialBoss",
        ClientKind::Modifier,
    ),
    // `Uses level {1} {0}`: the skill a graft's implicit grants.
    ("BrequelGraftSkillString", ClientKind::Modifier),
];

/// `ClientStrings` wrappers the resolver reads through (PR B5c;
/// [`loom_parser::resolve::compose`]), each with the stat the wrapper
/// stands for, value 1. Pairing: every `Mods` row with the stat is an
/// eldritch implicit (`*EldritchImplicitUniquePresence*`,
/// `*PinnaclePresence*`) whose other stat's line the client wraps in the
/// text (PoE1 26530219).
pub const COMPOSITIONS: [(&str, &str); 2] = [
    (
        "InfluenceStatConditionPresenceUniqueMonster",
        "local_influence_mod_requires_unique_monster_presence",
    ),
    (
        "InfluenceStatConditionPresenceCelestialBoss",
        "local_influence_mod_requires_celestial_boss_presence",
    ),
];

/// What a `ClientStrings` line is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientKind {
    /// An item property, not a modifier.
    Property,
    /// A modifier line composed by the client.
    Modifier,
}

/// Categories whose lines are excluded from the rate, each approved by the
/// maintainer on 2026-11-10: `author-note` (B4b) and `legacy-text` (B5b:
/// text a PoB source holds and no current game text does; the game no
/// longer generates it, so the resolver cannot be measured on it).
pub const EXCLUDED: [&str; 2] = ["special-case/author-note", "data-gap/legacy-text"];

/// The word key of a text (module docs).
#[must_use]
pub fn word_key(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut last_hash = false;
    while let Some(c) = chars.next() {
        if c.is_ascii_digit() {
            // A number with an optional decimal part.
            loop {
                match chars.peek().copied() {
                    Some(n) if n.is_ascii_digit() => {
                        chars.next();
                    }
                    Some('.') if chars.clone().nth(1).is_some_and(|d| d.is_ascii_digit()) => {
                        chars.next();
                    }
                    _ => break,
                }
            }
            push_hash(&mut out, &mut last_hash);
        } else if c == '#' {
            push_hash(&mut out, &mut last_hash);
        } else if c.is_alphabetic() {
            out.extend(c.to_lowercase());
            last_hash = false;
        } else if !out.ends_with(' ') && !out.is_empty() {
            out.push(' ');
        }
    }
    out.trim().to_owned()
}

/// Appends `#`, joining it with a `#` written just before (a range or two
/// adjacent values are one value).
fn push_hash(out: &mut String, last_hash: &mut bool) {
    if *last_hash {
        // `# #` (a range `(a-b)`) or `##`: keep one.
        while out.ends_with(' ') {
            out.pop();
        }
        return;
    }
    if !out.is_empty() && !out.ends_with(' ') {
        out.push(' ');
    }
    out.push('#');
    out.push(' ');
    *last_hash = true;
}

/// Every line of a template text, as written, with placeholders `#`.
fn template_lines(segments: &[Segment]) -> Vec<String> {
    let mut lines = vec![String::new()];
    for s in segments {
        match s {
            Segment::Text(t) => {
                if let Some(l) = lines.last_mut() {
                    l.push_str(t);
                }
            }
            Segment::Value { .. } => {
                if let Some(l) = lines.last_mut() {
                    l.push_str(" # ");
                }
            }
            Segment::LineBreak => lines.push(String::new()),
        }
    }
    lines
}

/// `{0}`, `{1:+d}`, ... to `#` in a `ClientStrings` text.
fn client_template(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some((before, after)) = rest.split_once('{') {
        out.push_str(before);
        match after.split_once('}') {
            Some((_, tail)) => {
                out.push_str(" # ");
                rest = tail;
            }
            None => {
                rest = after;
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Whether `words` match `pattern`, where a `#` in the pattern matches one
/// or more words.
fn glob(pattern: &[&str], words: &[&str]) -> bool {
    match pattern.split_first() {
        None => words.is_empty(),
        Some((&"#", rest)) => {
            (1..=words.len()).any(|n| words.get(n..).is_some_and(|w| glob(rest, w)))
        }
        Some((p, rest)) => words
            .split_first()
            .is_some_and(|(w, ws)| w == p && glob(rest, ws)),
    }
}

/// One PoB text source, as recorded in the summary.
#[derive(Debug, Clone, Default, Serialize)]
pub struct PobSource {
    /// The tag the maintainer named (`--pob <tag>=<dir>`).
    pub tag: String,
    /// Files read, with the text lines read from each.
    pub files: BTreeMap<String, usize>,
    /// Distinct word keys.
    pub keys: usize,
    /// SHA-256 over every file read (name, NUL, content, NUL; sorted by
    /// name), hex.
    pub sha256: String,
}

/// PoB text lines of one source, by word key.
#[derive(Debug, Default)]
pub struct PobText {
    /// Recorded source.
    pub source: PobSource,
    keys: BTreeSet<String>,
}

impl PobText {
    /// Reads PoB's item text files at one tag. `files` are paths relative
    /// to `src/Data` with their content: `Mod*.lua` (but
    /// [`render_check::NON_MOD_ID_FILES`]) through the mod entry reader;
    /// `Enchantment*.lua` and `ClusterJewels.lua` as quoted value strings;
    /// `Uniques/**/*.lua` as the lines of each `[[ ... ]]` block with `{...}`
    /// prefixes removed. Other files are ignored.
    #[must_use]
    pub fn read(tag: &str, mut files: Vec<(String, String)>) -> Self {
        files.sort();
        let mut hash = Sha256::new();
        let mut out = Self::default();
        out.source.tag = tag.to_owned();
        for (name, text) in &files {
            let text = text.strip_prefix('\u{feff}').unwrap_or(text);
            let file = name.rsplit('/').next().unwrap_or(name);
            let lines: Vec<String> = if name.starts_with("Uniques/") {
                unique_lines(text)
            } else if file.starts_with("Mod") && !render_check::NON_MOD_ID_FILES.contains(&file) {
                let mut pob = render_check::Pob::default();
                pob.add_mod_file(file, text);
                pob.texts.into_values().flatten().flatten().collect()
            } else if file.starts_with("Enchantment") || file == "ClusterJewels.lua" {
                quoted_values(text)
            } else {
                continue;
            };
            hash.update(name.as_bytes());
            hash.update([0]);
            hash.update(text.as_bytes());
            hash.update([0]);
            out.source.files.insert(name.clone(), lines.len());
            for line in lines {
                for part in line.split("\\n") {
                    let key = word_key(part);
                    if !key.is_empty() {
                        out.keys.insert(key);
                    }
                }
            }
        }
        out.source.keys = out.keys.len();
        out.source.sha256 = hash.finalize().iter().map(|b| format!("{b:02x}")).collect();
        out
    }
}

/// Lines of every `[[ ... ]]` block, `{...}` prefixes removed.
fn unique_lines(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some((_, after)) = rest.split_once("[[") {
        let Some((block, tail)) = after.split_once("]]") else {
            break;
        };
        // The first two lines are the name and the base; spec lines
        // (`Requires Level`, `Variant:`, `Implicits:`) are not mod text.
        for line in block
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .skip(2)
        {
            if pob_item::spec_key(line).is_some_and(|k| pob_item::KNOWN_SPECS.contains(&k)) {
                continue;
            }
            let mut line = line;
            while let Some(after) = line.strip_prefix('{') {
                match after.split_once('}') {
                    Some((_, t)) => line = t,
                    None => break,
                }
            }
            if !line.is_empty() {
                out.push(line.to_owned());
            }
        }
        rest = tail;
    }
    out
}

/// Quoted strings that are values: not a `["key"]` and not `name = "..."`.
fn quoted_values(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        let mut rest = line;
        let mut before = String::new();
        while let Some((pre, after)) = rest.split_once('"') {
            before.push_str(pre);
            let Some((value, tail)) = after.split_once('"') else {
                break;
            };
            let prev = before.trim_end();
            if !(prev.ends_with('[') || prev.ends_with('=')) {
                out.push(value.to_owned());
            }
            before.push('"');
            before.push_str(value);
            before.push('"');
            rest = tail;
        }
    }
    out
}

/// One `ClientStrings` rule matched against lines.
#[derive(Debug, Clone)]
struct ClientRule {
    id: String,
    kind: ClientKind,
    words: Vec<String>,
}

/// Every source of game text, by word key.
#[derive(Debug, Default)]
pub struct GameText {
    /// Word key to file source to the first variant line.
    descriptions: BTreeMap<String, BTreeMap<String, usize>>,
    /// Lines of variants with a lookup handler, whose placeholder shows a
    /// name, not a number: `#` matches one or more words (`glob`).
    wild: Vec<(Vec<String>, String, usize)>,
    client: Vec<ClientRule>,
    pob: Vec<PobText>,
    /// Whether a description or client text holds `^` and a digit or `x`.
    pub colour_code_in_game_text: bool,
    /// The [`COMPOSITIONS`] whose `ClientStrings` text was found and has
    /// one `{0}`, in that order.
    pub compositions: Vec<Composition>,
    /// [`COMPOSITIONS`] ids not read (no `ClientStrings` row, or a text
    /// without exactly one `{0}`), recorded rather than skipped silently.
    pub compositions_missing: Vec<String>,
}

impl GameText {
    /// Indexes the extract's descriptions, `ClientStrings`, and PoB sources.
    ///
    /// # Errors
    /// A message when `ClientStrings` or one of its columns is missing.
    pub fn load(set: &StatDescriptions, read: Read<'_>, pob: Vec<PobText>) -> Result<Self, String> {
        let mut out = Self {
            pob,
            ..Self::default()
        };
        for (_, file) in set.files() {
            for d in &file.descriptions {
                for v in &d.variants {
                    out.colour_code_in_game_text |= has_colour_code(&v.text);
                    let Ok(t) = &v.template else {
                        // B1 loads both extracts with 0 template errors; a
                        // skipped variant could push its text to `no-game-text`.
                        return Err(format!(
                            "{}:{}: template does not parse",
                            file.source, v.line
                        ));
                    };
                    let lookup = v
                        .handlers
                        .iter()
                        .any(|h| matches!(h, Handler::Lookup { .. }));
                    for line in template_lines(&t.segments) {
                        let key = word_key(&line);
                        if key.is_empty() {
                            continue;
                        }
                        // A pattern needs a placeholder and a literal word
                        // (`{0}` alone would match every line).
                        if lookup
                            && key.split(' ').any(|w| w == "#")
                            && key.split(' ').any(|w| w != "#")
                        {
                            out.wild.push((
                                key.split(' ').map(str::to_owned).collect(),
                                file.source.clone(),
                                v.line,
                            ));
                        }
                        out.descriptions
                            .entry(key)
                            .or_default()
                            .entry(file.source.clone())
                            .or_insert(v.line);
                    }
                }
            }
        }
        let table = Table::require(read, "ClientStrings")?;
        let (id, text) = (table.column("Id")?, table.column("Text")?);
        for row in &table.rows {
            let (Some(i), Some(t)) = (Table::text(row, id), Table::text(row, text)) else {
                continue;
            };
            out.colour_code_in_game_text |= has_colour_code(t);
            if let Some((_, stat)) = COMPOSITIONS.iter().find(|(id, _)| *id == i)
                && let Some(c) = Composition::new(i, t, stat, 1)
            {
                out.compositions.push(c);
            }
            let Some(kind) = client_kind(i) else {
                continue;
            };
            let key = word_key(&client_template(t));
            let words: Vec<String> = key.split(' ').map(str::to_owned).collect();
            if words.iter().any(|w| w != "#") {
                out.client.push(ClientRule {
                    id: i.to_owned(),
                    kind,
                    words,
                });
            }
        }
        out.compositions
            .sort_by_key(|c| COMPOSITIONS.iter().position(|(id, _)| *id == c.id));
        out.compositions_missing = COMPOSITIONS
            .iter()
            .filter(|(id, _)| out.compositions.iter().all(|c| c.id != *id))
            .map(|(id, _)| (*id).to_owned())
            .collect();
        // A stale pairing (a stat renamed by a patch) fails the run instead
        // of writing a stat ID the game no longer has.
        if let Some(stats) = Table::read(read, "Stats")? {
            let id = stats.column("Id")?;
            for c in &out.compositions {
                if !stats
                    .rows
                    .iter()
                    .any(|r| Table::text(r, id) == Some(&c.stat))
                {
                    return Err(format!("{}: stat {} is not in Stats", c.id, c.stat));
                }
            }
        }
        Ok(out)
    }

    /// The PoB sources, for the summary.
    #[must_use]
    pub fn pob_sources(&self) -> Vec<PobSource> {
        self.pob.iter().map(|p| p.source.clone()).collect()
    }

    /// Files whose variant lines match `key`: exactly, or through a lookup
    /// variant's name placeholder.
    fn description_files(&self, key: &str) -> BTreeMap<String, usize> {
        let mut out = self.descriptions.get(key).cloned().unwrap_or_default();
        let words: Vec<&str> = key.split(' ').collect();
        for (pattern, file, line) in &self.wild {
            let pattern: Vec<&str> = pattern.iter().map(String::as_str).collect();
            if glob(&pattern, &words) {
                out.entry(file.clone()).or_insert(*line);
            }
        }
        out
    }

    fn client_match(&self, key: &str) -> Option<&ClientRule> {
        let words: Vec<&str> = key.split(' ').collect();
        self.client.iter().find(|r| {
            let pattern: Vec<&str> = r.words.iter().map(String::as_str).collect();
            glob(&pattern, &words)
        })
    }
}

fn client_kind(id: &str) -> Option<ClientKind> {
    CLIENT_STRINGS.iter().find_map(|(rule, kind)| {
        let hit = match rule.strip_suffix('*') {
            Some(prefix) => id.starts_with(prefix),
            None => id == *rule,
        };
        hit.then_some(*kind)
    })
}

/// `^` followed by a digit or `x` (a PoB colour escape).
fn has_colour_code(text: &str) -> bool {
    text.as_bytes()
        .windows(2)
        .any(|w| matches!(w, [b'^', d] if d.is_ascii_digit() || *d == b'x'))
}

/// What the categorizer knows about one line that is not resolved.
#[derive(Debug, Clone, Copy)]
pub struct LineFacts<'a> {
    /// `resolve-corpus` outcome (`ambiguous`, `unresolved/...`,
    /// `unknown-base/...`, ...).
    pub outcome: &'a str,
    /// The line's text.
    pub text: &'a str,
    /// Rejection reasons (`no-variant-fits`), or the filter decision and
    /// notes (`ambiguous`).
    pub detail: &'a [String],
    /// Sources of the files in the item's scope chain.
    pub scope: &'a BTreeSet<String>,
    /// The adjacent line of another section it resolves with, if any.
    pub split_with: Option<usize>,
}

/// A category with its evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Category {
    /// Category name.
    pub name: &'static str,
    /// Why it applies.
    pub evidence: String,
}

impl Category {
    fn new(name: &'static str, evidence: impl Into<String>) -> Self {
        Self {
            name,
            evidence: evidence.into(),
        }
    }

    /// Whether lines of this category are excluded from the rate.
    #[must_use]
    pub fn excluded(&self) -> bool {
        EXCLUDED.contains(&self.name)
    }
}

/// Rejection reasons that mean the resolver could not use a handler.
const HANDLER_REASONS: [&str; 4] = [
    "unknown-handler",
    "malformed-handler",
    "unsupported-format",
    "unknown-flag",
];

/// The category of a line that is not resolved (module docs).
#[must_use]
pub fn categorize(game: &GameText, line: LineFacts<'_>) -> Category {
    let text = line.text;
    if let Some(reason) = line.outcome.strip_prefix("unknown-base/") {
        return Category::new("special-case/unknown-base", reason);
    }
    match line.outcome {
        "ambiguous" => return Category::new("parser-gap/ambiguous", line.detail.join(" ")),
        "unresolved/only-shadowed" => {
            return Category::new("parser-gap/only-shadowed", "every match is shadowed");
        }
        "unresolved/no-variant-fits" => {
            let reasons = line.detail.join(" ");
            let handler_only = !line.detail.is_empty()
                && line
                    .detail
                    .iter()
                    .all(|r| HANDLER_REASONS.contains(&r.as_str()));
            let name = if handler_only {
                "parser-gap/unsupported-handler"
            } else {
                "parser-gap/near-miss"
            };
            return Category::new(name, reasons);
        }
        "unresolved/no-template" => {}
        _ => return Category::new("uncategorized", line.outcome),
    }
    let mut chars = text.chars();
    let colour =
        chars.next() == Some('^') && chars.next().is_some_and(|c| c.is_ascii_digit() || c == 'x');
    // The rule's evidence is that no game text holds a colour escape.
    if colour && !game.colour_code_in_game_text {
        return Category::new("special-case/author-note", "color-code");
    }
    if let Some(n) = line.split_with {
        return Category::new(
            "parser-gap/split-section",
            format!("resolves with item line {n}"),
        );
    }
    if pob_item::spec_key(text) == Some("Note") {
        return Category::new("special-case/author-note", "pob-note-spec");
    }
    if let Some(key) = pob_item::spec_key(text).filter(|k| {
        (pob_item::KNOWN_SPECS.contains(k)
            || (k.starts_with("Quality (") && k.ends_with(" Modifiers)")))
            && text.split_once(": ").is_some_and(|(_, v)| numeric_value(v))
    }) {
        return Category::new("special-case/item-property", format!("pob-spec:{key}"));
    }
    let key = word_key(text);
    if let Some(rule) = game.client_match(&key) {
        return match rule.kind {
            ClientKind::Property => Category::new(
                "special-case/item-property",
                format!("ClientStrings:{}", rule.id),
            ),
            ClientKind::Modifier => Category::new(
                "parser-gap/client-string",
                format!(
                    "ClientStrings:{} (the composed remainder is not resolved)",
                    rule.id
                ),
            ),
        };
    }
    let files = game.description_files(&key);
    if !files.is_empty() {
        if let Some((file, at)) = files.iter().find(|(f, _)| line.scope.contains(*f)) {
            return Category::new("parser-gap/near-miss", format!("{file}:{at}"));
        }
        let others: Vec<&str> = files.keys().map(String::as_str).collect();
        return Category::new("data-gap/other-scope", others.join(" "));
    }
    let tags: Vec<&str> = game
        .pob
        .iter()
        .filter(|p| p.keys.contains(&key))
        .map(|p| p.source.tag.as_str())
        .collect();
    if !tags.is_empty() {
        return Category::new("data-gap/legacy-text", format!("pob {}", tags.join(" ")));
    }
    if game.pob.is_empty() {
        return Category::new("uncategorized", "no PoB source to test legacy text");
    }
    Category::new("special-case/author-note", "no-game-text")
}

/// A spec value that is a number: `63`, `+20%`, `-1.5`.
fn numeric_value(v: &str) -> bool {
    let v = v.trim();
    let v = v.strip_prefix(['+', '-']).unwrap_or(v);
    let v = v.strip_suffix('%').unwrap_or(v);
    let (whole, frac) = match v.split_once('.') {
        Some((w, f)) => (w, Some(f)),
        None => (v, None),
    };
    !whole.is_empty()
        && whole.bytes().all(|b| b.is_ascii_digit())
        && frac.is_none_or(|f| !f.is_empty() && f.bytes().all(|b| b.is_ascii_digit()))
}

#[cfg(test)]
mod tests;
