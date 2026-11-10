//! Reverse matcher (Spike B, PRs B2a and B2b): item lines to stat IDs and
//! raw values through the stat descriptions of one [`Scope`] (ADR-015).
//!
//! 1. [`Index::build`] indexes every variant of the scope's descriptions
//!    (both hands of a handed description) by its **key**: the template
//!    text without markup, each placeholder and each literal number
//!    replaced by a slot ([`token`]). A multi-line template's key is its
//!    lines' keys joined by `\n` (lines trimmed, empty lines dropped, as
//!    the renderer writes them). A placeholder of a stat with a **lookup
//!    handler** is expanded: the variant is indexed once per name the
//!    [`Lookup`] gives for the handler, with the name as literal text, and
//!    the keys that carry the name are kept for the stat. Left out, and
//!    counted in [`IndexStats`]: `gem_quality` and `table_only` variants
//!    (`v2.67.2:src/Modules/StatDescriber.lua` `matchLimit` skips
//!    `gem_quality` for regular mods; `table_only`, a PoE2 flag, is skipped
//!    as the renderer does since PR B1), templates that did not
//!    parse, templates with no text, lookup variants whose handler has no
//!    names in the [`Lookup`], and variants with more than one lookup
//!    placeholder.
//! 2. [`Index::resolve`] normalizes one line, tokenizes it, and looks up its
//!    key; [`Index::resolve_lines`] reads consecutive lines and tries the
//!    longest multi-line key first. Each variant under the key is
//!    **verified**: literal tokens must
//!    be equal; each value token is read with the placeholder's sign rules
//!    and inverted through the stat's numeric handlers
//!    ([`loom_data::stat_desc::invert_shown`]) to raw values; the variant's
//!    conditions must hold for those raw values; stats without a
//!    placeholder take a value their condition allows ([`RawValue::Hidden`]).
//! 3. Variants are grouped by description and hand; the first variant (file
//!    order) that verifies represents it, so variants of one description
//!    sharing a template are not ambiguity; a later variant that verifies
//!    with different values is kept in [`Candidate::alternatives`]. The two
//!    hands of a handed description share their text, so their lines are
//!    ambiguous (main and off hand) until a later step picks the hand.
//!    One description gives
//!    [`Outcome::Resolved`], more give [`Outcome::Ambiguous`] (filtering is
//!    PR B4a), none gives [`Outcome::Unresolved`] with every rejection and
//!    its reason.
//!
//! **Include precedence (hypothesis, see `loom_data::stat_desc::scope`):** a
//! description is *shadowed* when a nearer file, or an earlier description
//! of the same file, describes one of its stats; the renderer would never
//! choose it. A shadowed match is not a candidate; it is listed in
//! [`Resolution::shadowed`] so the lines that depend on the rule can be
//! counted (PR B2b).
//!
//! **Values:** a single number gives [`RawValue::Exact`], the raw values
//! that show as it (one value unless a handler rounds). A range `(a-b)`
//! gives [`RawValue::Ranged`], the hull of both ends' raw values; Spike B
//! does not compute the rolled value (plan, "Values"). Conditions narrow an
//! exact value; a range only has to meet the condition somewhere, so a
//! range that crosses zero (`(-10-10)% increased`) fits a variant whose
//! condition holds on one side. A looked-up name gives [`RawValue::Lookup`],
//! every key with that name that meets the condition.
//!
//! **Composed lines (B5c):** [`compose`] reads lines the client wraps in a
//! `ClientStrings` text, with the stat the wrapper stands for.
//!
//! Every line ends in exactly one [`Outcome`]; nothing is dropped.

pub mod compose;
pub mod token;

use std::collections::BTreeMap;

use loom_data::stat_desc::{
    Condition, DescRef, Description, Hand, Handler, Lookup, LookupHandler, NoLookup,
    NumericHandler, PlaceholderFormat, Scope, Segment, StatValue, Variant, invert_shown,
};

use token::{Atom, SLOT, Sign, Token, collapse, normalize, tokenize};

/// Counts of the variants an [`Index`] holds and leaves out. Each variant
/// is counted once per hand, under exactly one of `indexed`,
/// `gem_quality`, `table_only`, `bad_template`, `empty_text`,
/// `lookup_unavailable`, `lookup_multi`. (PoB's runtime describer,
/// `v2.67.2:src/Modules/StatDescriber.lua` `matchLimit`, skips only
/// `gem_quality`; its export describer, `Export/statdesc.lua`, only
/// `table_only`.)
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IndexStats {
    /// Variants indexed (one per hand of a handed description).
    pub indexed: usize,
    /// Of `indexed`: templates with more than one line.
    pub multi_line: usize,
    /// Of `indexed`: variants expanded through a lookup.
    pub lookup: usize,
    /// Index entries (a lookup variant has one per distinct name).
    pub entries: usize,
    /// Distinct keys.
    pub keys: usize,
    /// Most lines of one indexed template.
    pub max_lines: usize,
    /// Left out: flagged `gem_quality`.
    pub gem_quality: usize,
    /// Left out: flagged `table_only`.
    pub table_only: usize,
    /// Left out: the template did not parse, or its slots do not fit its
    /// placeholders (for a lookup variant: under any one name; the whole
    /// variant is left out and counted once).
    pub bad_template: usize,
    /// Left out: the template has no text (a deliberately hidden line).
    pub empty_text: usize,
    /// Left out: a lookup placeholder whose handler has no non-blank names
    /// in the [`Lookup`] (its table is not available, or empty).
    pub lookup_unavailable: usize,
    /// Left out: more than one lookup placeholder stat, or more than one
    /// lookup handler on one stat.
    pub lookup_multi: usize,
    /// Not a variant count: lookup keys with a blank name (they write no
    /// text and cannot be matched), over the handlers this index read.
    pub lookup_blank_names: usize,
}

/// What a value token of a template stands for.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Slot {
    /// A placeholder: 0-based stat index (per hand), its format, and a sign
    /// written in the template text right before it (`+{0}`).
    Value {
        stat: usize,
        format: PlaceholderFormat,
        literal_sign: Option<Sign>,
    },
    /// Literal number text of the template (`Level 20`), sign included.
    Literal(String),
}

/// One indexed variant (one name of a lookup variant).
#[derive(Debug, Clone)]
struct Entry<'a> {
    at: DescRef,
    description: &'a Description,
    variant_index: usize,
    variant: &'a Variant,
    slots: Vec<Slot>,
    /// A stat whose placeholder was replaced by a looked-up name: its
    /// index and the keys with that name.
    named: Option<(usize, Vec<i64>)>,
    shadowed: bool,
}

/// Variants of one [`Scope`] by key.
#[derive(Debug, Clone)]
pub struct Index<'a> {
    entries: BTreeMap<String, Vec<Entry<'a>>>,
    stats: IndexStats,
    /// Lowercased key to the one key it folds from; `None` when several
    /// keys fold to it ([`Index::resolve_case_folded`]).
    folded: BTreeMap<String, Option<String>>,
}

/// A raw stat value read from a line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RawValue {
    /// Shown as one number: the raw values that show as it, narrowed by
    /// the variant's condition. A `!v` condition only moves an end, so `v`
    /// can still lie inside the interval.
    Exact(StatValue),
    /// Shown as a range `(a-b)`: from the lowest raw value of `a` to the
    /// highest of `b`.
    Ranged(StatValue),
    /// Not shown in the text.
    Hidden {
        /// The variant's condition on the stat.
        condition: Condition,
        /// A value the condition allows: 0 when it does, otherwise the
        /// bound nearest 0 of a one-sided range, `None` when neither
        /// applies (`!0`).
        assumed: Option<i64>,
    },
    /// Shown as a name through a lookup handler: every key with that name
    /// that meets the variant's condition, sorted.
    Lookup(Vec<i64>),
}

/// A stat ID and its value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedStat {
    /// Stat ID.
    pub stat: String,
    /// Value read from the line.
    pub value: RawValue,
}

/// A description (one hand) whose variant verified against the line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    /// The description and hand.
    pub at: DescRef,
    /// Index of the first variant that verified.
    pub variant: usize,
    /// Every stat of the description's hand, in description order.
    pub stats: Vec<ResolvedStat>,
    /// Later variants of the same description and hand that also verified
    /// with **different** values (another reading of the line), in file
    /// order. Variants that agree with `stats` are not listed.
    pub alternatives: Vec<Alternative>,
}

/// Another reading of a line by a later variant of the same description.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alternative {
    /// Variant index.
    pub variant: usize,
    /// Every stat of the description's hand, in description order.
    pub stats: Vec<ResolvedStat>,
}

/// Why a variant with the line's key did not verify.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RejectReason {
    /// A literal number of the template differs from the line's.
    LiteralMismatch {
        /// The template's text.
        expected: String,
        /// The line's text.
        found: String,
    },
    /// The token's sign is not one the placeholder format writes.
    SignMismatch {
        /// Stat ID.
        stat: String,
    },
    /// No raw value shows as the number through the stat's handlers.
    ValueNotShown {
        /// Stat ID.
        stat: String,
        /// The number as read (sign applied).
        shown: String,
    },
    /// The same stat is shown twice with values that do not overlap.
    InconsistentValues {
        /// Stat ID.
        stat: String,
    },
    /// The raw value does not meet the variant's condition.
    ConditionFails {
        /// Stat ID.
        stat: String,
    },
    /// The variant has a handler without known semantics.
    UnknownHandler(String),
    /// The variant has a handler with a bad argument.
    MalformedHandler(String),
    /// The variant has a flag without known meaning.
    UnknownFlag(String),
    /// A placeholder format without known semantics.
    UnsupportedFormat(String),
}

impl RejectReason {
    /// Stable name for reports.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::LiteralMismatch { .. } => "literal-mismatch",
            Self::SignMismatch { .. } => "sign-mismatch",
            Self::ValueNotShown { .. } => "value-not-shown",
            Self::InconsistentValues { .. } => "inconsistent-values",
            Self::ConditionFails { .. } => "condition-fails",
            Self::UnknownHandler(_) => "unknown-handler",
            Self::MalformedHandler(_) => "malformed-handler",
            Self::UnknownFlag(_) => "unknown-flag",
            Self::UnsupportedFormat(_) => "unsupported-format",
        }
    }
}

/// A variant with the line's key that did not verify.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rejected {
    /// The description and hand.
    pub at: DescRef,
    /// Variant index.
    pub variant: usize,
    /// Why.
    pub reason: RejectReason,
}

/// Why a line did not resolve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unresolved {
    /// No indexed variant has the line's key.
    NoTemplate,
    /// Variants have the key, and none verified; every rejection in index
    /// order.
    NoVariantFits(Vec<Rejected>),
    /// Only shadowed descriptions verified (see [`Resolution::shadowed`]).
    OnlyShadowed,
}

impl Unresolved {
    /// Stable name for reports.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::NoTemplate => "no-template",
            Self::NoVariantFits(_) => "no-variant-fits",
            Self::OnlyShadowed => "only-shadowed",
        }
    }
}

/// The result for one line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// One description.
    Resolved(Candidate),
    /// More than one description, sorted by position.
    Ambiguous(Vec<Candidate>),
    /// None.
    Unresolved(Unresolved),
}

/// The outcome for one line, or consecutive lines of a multi-line
/// template, and the shadowed matches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolution {
    /// The line after [`normalize`]; several lines are joined by `\n`.
    pub line: String,
    /// How many input lines this resolution covers.
    pub lines: usize,
    /// The result.
    pub outcome: Outcome,
    /// Descriptions that verified but are shadowed under the include
    /// precedence rule, sorted by position.
    pub shadowed: Vec<Candidate>,
}

impl<'a> Index<'a> {
    /// Indexes the variants of every description in `scope` without lookup
    /// tables (lookup variants are left out and counted).
    #[must_use]
    pub fn build(scope: &Scope<'a>) -> Self {
        Self::build_with(scope, &NoLookup)
    }

    /// Indexes the variants of every description in `scope`, expanding
    /// lookup placeholders with the names `lookup` gives.
    #[must_use]
    pub fn build_with(scope: &Scope<'a>, lookup: &dyn Lookup) -> Self {
        let mut entries: BTreeMap<String, Vec<Entry<'a>>> = BTreeMap::new();
        let mut stats = IndexStats::default();
        let mut names = NameCache::default();
        for (main, description) in scope.descriptions() {
            for hand in [Hand::Main, Hand::Off] {
                let Some(ids) = description.stats_of(hand) else {
                    continue;
                };
                let at = DescRef { hand, ..main };
                // Depth and index only: a stat in both hands of one
                // description is found under the main hand and is not
                // shadowed in the off hand.
                let shadowed = ids.iter().any(|id| {
                    scope
                        .find(id)
                        .is_none_or(|f| (f.depth, f.index) != (at.depth, at.index))
                });
                for (variant_index, variant) in description.variants.iter().enumerate() {
                    let indexed = index_variant(variant, ids.len(), lookup, &mut names, &mut stats);
                    for one in indexed {
                        stats.entries += 1;
                        stats.max_lines = stats.max_lines.max(one.lines);
                        entries.entry(one.key).or_default().push(Entry {
                            at,
                            description,
                            variant_index,
                            variant,
                            slots: one.slots,
                            named: one.named,
                            shadowed,
                        });
                    }
                }
            }
        }
        stats.keys = entries.len();
        let mut folded: BTreeMap<String, Option<String>> = BTreeMap::new();
        for key in entries.keys() {
            folded
                .entry(key.to_lowercase())
                .and_modify(|k| *k = None)
                .or_insert_with(|| Some(key.clone()));
        }
        Self {
            entries,
            stats,
            folded,
        }
    }

    /// What was indexed and left out.
    #[must_use]
    pub const fn stats(&self) -> IndexStats {
        self.stats
    }

    /// Resolves one item line. Use [`Index::resolve_lines`] for lines that
    /// may belong to a multi-line template.
    #[must_use]
    pub fn resolve(&self, line: &str) -> Resolution {
        self.resolve_normalized(normalize(line), 1)
    }

    /// Resolves consecutive item lines. At each line, the longest run of
    /// lines (up to the longest indexed template) whose joined key is
    /// indexed is taken as one multi-line template: a key that spans lines
    /// is strong evidence that they belong together, so it is kept even
    /// when it does not verify, unless every line of the run resolves
    /// alone (resolved or ambiguous), in which case the single-line
    /// readings are kept. Otherwise the line is resolved alone. Every input
    /// line is covered by exactly one [`Resolution`] (see
    /// [`Resolution::lines`]).
    #[must_use]
    pub fn resolve_lines(&self, lines: &[&str]) -> Vec<Resolution> {
        let normalized: Vec<String> = lines.iter().map(|l| normalize(l)).collect();
        let mut out = Vec::new();
        let mut at = 0;
        while let Some(first) = normalized.get(at) {
            let longest = self.stats.max_lines.min(normalized.len() - at);
            let multi = (2..=longest).rev().find_map(|n| {
                let run = normalized.get(at..at + n)?;
                let joined = run.join("\n");
                self.entries
                    .contains_key(&tokenize(&joined).key)
                    .then(|| (run, self.resolve_normalized(joined, n)))
            });
            match multi {
                Some((run, multi)) if matches!(multi.outcome, Outcome::Unresolved(_)) => {
                    let singles: Vec<Resolution> = run
                        .iter()
                        .map(|l| self.resolve_normalized(l.clone(), 1))
                        .collect();
                    let all_read = singles
                        .iter()
                        .all(|r| !matches!(r.outcome, Outcome::Unresolved(_)));
                    at += multi.lines;
                    if all_read {
                        out.extend(singles);
                    } else {
                        out.push(multi);
                    }
                }
                Some((_, multi)) => {
                    at += multi.lines;
                    out.push(multi);
                }
                None => {
                    out.push(self.resolve_normalized(first.clone(), 1));
                    at += 1;
                }
            }
        }
        out
    }

    /// Resolves one line whose key differs from exactly one indexed key in
    /// letter case only (`+132 to Maximum Life` for `+{0} to maximum
    /// Life`): an import tolerance for text written by hand (PoB's own
    /// parser ignores case), never used by [`Index::resolve`]. `None` when
    /// the line's key is indexed as it is, when no key folds to it, or when
    /// several do.
    #[must_use]
    pub fn resolve_case_folded(&self, line: &str) -> Option<Resolution> {
        let line = normalize(line);
        let tokenized = tokenize(&line);
        if self.entries.contains_key(&tokenized.key) {
            return None;
        }
        let key = self.folded.get(&tokenized.key.to_lowercase())?.as_ref()?;
        Some(self.resolve_key(line, 1, &tokenized, key))
    }

    /// Resolves normalized text covering `lines` input lines.
    fn resolve_normalized(&self, line: String, lines: usize) -> Resolution {
        let tokenized = tokenize(&line);
        let key = tokenized.key.clone();
        self.resolve_key(line, lines, &tokenized, &key)
    }

    /// Resolves tokenized text against the entries of `key`.
    fn resolve_key(
        &self,
        line: String,
        lines: usize,
        tokenized: &token::Tokenized,
        key: &str,
    ) -> Resolution {
        let Some(entries) = self.entries.get(key) else {
            return Resolution {
                line,
                lines,
                outcome: Outcome::Unresolved(Unresolved::NoTemplate),
                shadowed: Vec::new(),
            };
        };
        let mut found: BTreeMap<DescRef, (bool, Candidate)> = BTreeMap::new();
        let mut rejected = Vec::new();
        for entry in entries {
            let verified = verify(entry, &tokenized.tokens);
            if let Some((_, first)) = found.get_mut(&entry.at) {
                // A later variant of a description already read: kept only
                // as another reading when its values differ.
                if let Ok(stats) = verified
                    && stats != first.stats
                    && first.alternatives.iter().all(|a| a.stats != stats)
                {
                    first.alternatives.push(Alternative {
                        variant: entry.variant_index,
                        stats,
                    });
                }
                continue;
            }
            match verified {
                Ok(stats) => {
                    let candidate = Candidate {
                        at: entry.at,
                        variant: entry.variant_index,
                        stats,
                        alternatives: Vec::new(),
                    };
                    found.insert(entry.at, (entry.shadowed, candidate));
                }
                Err(reason) => rejected.push(Rejected {
                    at: entry.at,
                    variant: entry.variant_index,
                    reason,
                }),
            }
        }
        let (shadowed, mut candidates): (Vec<_>, Vec<_>) =
            found.into_values().partition(|(shadowed, _)| *shadowed);
        let shadowed: Vec<Candidate> = shadowed.into_iter().map(|(_, c)| c).collect();
        let outcome = match candidates.len() {
            0 if shadowed.is_empty() => Outcome::Unresolved(Unresolved::NoVariantFits(rejected)),
            0 => Outcome::Unresolved(Unresolved::OnlyShadowed),
            1 => match candidates.pop() {
                Some((_, candidate)) => Outcome::Resolved(candidate),
                None => Outcome::Unresolved(Unresolved::NoVariantFits(rejected)),
            },
            _ => Outcome::Ambiguous(candidates.into_iter().map(|(_, c)| c).collect()),
        };
        Resolution {
            line,
            lines,
            outcome,
            shadowed,
        }
    }
}

/// Names per lookup handler, each with the keys that carry it, read once
/// per [`Index::build_with`].
#[derive(Default)]
struct NameCache(BTreeMap<LookupHandler, BTreeMap<String, Vec<i64>>>);

impl NameCache {
    fn get(
        &mut self,
        handler: LookupHandler,
        lookup: &dyn Lookup,
        stats: &mut IndexStats,
    ) -> &BTreeMap<String, Vec<i64>> {
        self.0.entry(handler).or_insert_with(|| {
            let mut names: BTreeMap<String, Vec<i64>> = BTreeMap::new();
            for (key, name) in lookup.entries(handler) {
                // Shown text is trimmed by the renderer, line by line (a
                // name can span lines: `tree_expansion_jewel_passive` names
                // a passive with several stats); a blank name writes no
                // text and cannot be matched (counted).
                let shown = name
                    .split('\n')
                    .map(collapse)
                    .filter(|l| !l.is_empty())
                    .collect::<Vec<_>>()
                    .join("\n");
                if shown.is_empty() {
                    stats.lookup_blank_names += 1;
                } else {
                    names.entry(shown).or_default().push(key);
                }
            }
            for keys in names.values_mut() {
                keys.sort_unstable();
                keys.dedup();
            }
            names
        })
    }
}

/// One index entry of a variant.
struct Indexed {
    key: String,
    slots: Vec<Slot>,
    named: Option<(usize, Vec<i64>)>,
    lines: usize,
}

/// The entries of a variant for a hand with `stat_count` stats: one, one
/// per looked-up name, or none (counted).
fn index_variant(
    variant: &Variant,
    stat_count: usize,
    lookup: &dyn Lookup,
    names: &mut NameCache,
    stats: &mut IndexStats,
) -> Vec<Indexed> {
    if variant.gem_quality {
        stats.gem_quality += 1;
        return Vec::new();
    }
    if variant.table_only {
        stats.table_only += 1;
        return Vec::new();
    }
    let Ok(template) = &variant.template else {
        stats.bad_template += 1;
        return Vec::new();
    };
    // Lookup stats that have a placeholder; a lookup stat without one is
    // hidden like any other stat.
    let mut looked_up: Vec<(usize, LookupHandler)> = variant
        .handlers
        .iter()
        .filter_map(|h| match h {
            Handler::Lookup { handler, stat } => Some((*stat, *handler)),
            _ => None,
        })
        .filter(|(stat, _)| {
            template
                .segments
                .iter()
                .any(|s| matches!(s, Segment::Value { index, .. } if index == stat))
        })
        .collect();
    looked_up.sort_unstable();
    looked_up.dedup();
    let mut out = Vec::new();
    match looked_up.as_slice() {
        [] => match index_skeleton(template, None, stat_count, stats) {
            Some(one) => out.push(one),
            None => return Vec::new(),
        },
        [(stat, handler)] => {
            let named = names.get(*handler, lookup, stats);
            if named.is_empty() {
                stats.lookup_unavailable += 1;
                return Vec::new();
            }
            for (name, keys) in named {
                match index_skeleton(template, Some((*stat, name, keys)), stat_count, stats) {
                    Some(one) => out.push(one),
                    // Counted once for the variant, not per name.
                    None => return Vec::new(),
                }
            }
            stats.lookup += 1;
        }
        _ => {
            stats.lookup_multi += 1;
            return Vec::new();
        }
    }
    stats.indexed += 1;
    if out.iter().any(|one| one.lines > 1) {
        stats.multi_line += 1;
    }
    out
}

/// The entry of a parsed template, with a looked-up stat's placeholders
/// replaced by `name`; `None` (counted) when it has no text or its slots
/// do not fit.
fn index_skeleton(
    template: &loom_data::stat_desc::Template,
    name: Option<(usize, &String, &Vec<i64>)>,
    stat_count: usize,
    stats: &mut IndexStats,
) -> Option<Indexed> {
    let mut skeleton = String::new();
    let mut placeholders = Vec::new();
    for segment in &template.segments {
        match segment {
            Segment::Text(text) => skeleton.push_str(&text.replace(SLOT, "\u{FFFD}")),
            Segment::Value { index, .. } if name.is_some_and(|(stat, ..)| stat == *index) => {
                if let Some((_, shown, _)) = name {
                    skeleton.push_str(&shown.replace(SLOT, "\u{FFFD}"));
                }
            }
            Segment::Value { index, format, .. } => {
                skeleton.push(SLOT);
                placeholders.push((*index, format.clone()));
            }
            Segment::LineBreak => skeleton.push('\n'),
        }
    }
    // As the renderer writes lines: each trimmed, empty ones dropped.
    let lines: Vec<String> = skeleton
        .split('\n')
        .map(collapse)
        .filter(|l| !l.is_empty())
        .collect();
    let tokenized = tokenize(&lines.join("\n"));
    if tokenized.key.is_empty() {
        stats.empty_text += 1;
        return None;
    }
    let Some(slots) = slots(tokenized.tokens, placeholders, stat_count) else {
        stats.bad_template += 1;
        return None;
    };
    Some(Indexed {
        key: tokenized.key,
        slots,
        named: name.map(|(stat, _, keys)| (stat, keys.clone())),
        lines: lines.len(),
    })
}

/// Pairs the skeleton's tokens with the placeholders. Each SLOT came from
/// one placeholder, in order (whitespace collapsing keeps them). `None` if
/// the counts differ or an index is not below `stat_count`; the loader
/// already rejects both, and they are checked again so nothing is lost
/// silently.
fn slots(
    tokens: Vec<Token>,
    placeholders: Vec<(usize, PlaceholderFormat)>,
    stat_count: usize,
) -> Option<Vec<Slot>> {
    let mut placeholders = placeholders.into_iter();
    let mut out = Vec::with_capacity(tokens.len());
    for token in tokens {
        out.push(match token.atom {
            Atom::Slot => {
                let (stat, format) = placeholders.next()?;
                if stat >= stat_count {
                    return None;
                }
                Slot::Value {
                    stat,
                    format,
                    literal_sign: token.sign,
                }
            }
            Atom::Number(_) | Atom::Range { .. } => Slot::Literal(token.text),
        });
    }
    placeholders.next().is_none().then_some(out)
}

/// A stat's raw values as read from the line so far.
#[derive(Debug, Clone, Copy)]
struct Read {
    value: StatValue,
    ranged: bool,
}

/// Verifies one variant against the line's tokens (same key, so the same
/// number of tokens); returns every stat of the hand with its value.
fn verify(entry: &Entry<'_>, tokens: &[Token]) -> Result<Vec<ResolvedStat>, RejectReason> {
    let variant = entry.variant;
    if let Some(flag) = variant.unknown_flags.first() {
        return Err(RejectReason::UnknownFlag(flag.clone()));
    }
    for handler in &variant.handlers {
        match handler {
            Handler::Unknown { name, .. } => {
                return Err(RejectReason::UnknownHandler(name.clone()));
            }
            Handler::Malformed { name } => {
                return Err(RejectReason::MalformedHandler(name.clone()));
            }
            Handler::Numeric { .. }
            | Handler::Lookup { .. }
            | Handler::Reminder(_)
            | Handler::CanonicalLine
            | Handler::Markup
            | Handler::CanonicalStat { .. } => {}
        }
    }
    let ids = entry
        .description
        .stats_of(entry.at.hand)
        .unwrap_or_default();
    let id = |stat: usize| ids.get(stat).cloned().unwrap_or_default();
    let mut read: Vec<Option<Read>> = vec![None; ids.len()];
    for (slot, token) in entry.slots.iter().zip(tokens) {
        match slot {
            Slot::Literal(expected) => {
                if *expected != token.text {
                    return Err(RejectReason::LiteralMismatch {
                        expected: expected.clone(),
                        found: token.text.clone(),
                    });
                }
            }
            Slot::Value {
                stat,
                format,
                literal_sign,
            } => {
                if let PlaceholderFormat::Other(written) = format {
                    return Err(RejectReason::UnsupportedFormat(written.clone()));
                }
                let (numbers, ranged) = shown_numbers(token, format, *literal_sign)
                    .ok_or_else(|| RejectReason::SignMismatch { stat: id(*stat) })?;
                let chain: Vec<&NumericHandler> = variant
                    .handlers
                    .iter()
                    .filter_map(|h| match h {
                        Handler::Numeric { handler, stat: s } if s == stat => Some(*handler),
                        _ => None,
                    })
                    .collect();
                let mut value: Option<StatValue> = None;
                for shown in &numbers {
                    let raw =
                        invert_shown(&chain, shown).ok_or_else(|| RejectReason::ValueNotShown {
                            stat: id(*stat),
                            shown: shown.clone(),
                        })?;
                    value = Some(value.map_or(raw, |v| hull(v, raw)));
                }
                let Some(value) = value else { continue };
                let merged = match read.get(*stat).copied().flatten() {
                    None => Some(Read { value, ranged }),
                    Some(before) => intersect(before.value, value).map(|value| Read {
                        value,
                        ranged: before.ranged || ranged,
                    }),
                };
                let merged =
                    merged.ok_or_else(|| RejectReason::InconsistentValues { stat: id(*stat) })?;
                if let Some(cell) = read.get_mut(*stat) {
                    *cell = Some(merged);
                }
            }
        }
    }
    let mut out = Vec::with_capacity(ids.len());
    for (i, stat) in ids.iter().enumerate() {
        let condition = variant
            .conditions
            .get(i)
            .copied()
            .unwrap_or(Condition::Range {
                min: None,
                max: None,
            });
        let fails = || RejectReason::ConditionFails { stat: stat.clone() };
        if let Some((_, keys)) = entry.named.as_ref().filter(|(s, _)| *s == i) {
            let keys: Vec<i64> = keys
                .iter()
                .copied()
                .filter(|k| condition.holds(*k))
                .collect();
            if keys.is_empty() {
                return Err(fails());
            }
            out.push(ResolvedStat {
                stat: stat.clone(),
                value: RawValue::Lookup(keys),
            });
            continue;
        }
        let value = match read.get(i).copied().flatten() {
            Some(Read {
                value,
                ranged: true,
            }) => meets(condition, value)
                .then_some(RawValue::Ranged(value))
                .ok_or_else(fails)?,
            Some(Read {
                value,
                ranged: false,
            }) => RawValue::Exact(narrow(condition, value).ok_or_else(fails)?),
            None => hidden(condition).ok_or_else(fails)?,
        };
        out.push(ResolvedStat {
            stat: stat.clone(),
            value,
        });
    }
    Ok(out)
}

/// The number texts a token shows for a placeholder (sign applied), and
/// whether it is a range. `None` when the sign is not one the format
/// writes: a plain placeholder writes `-` for negative values only, a
/// signed one always writes `+` or `-`, and a sign written in the template
/// before a plain placeholder must be on the line as is.
fn shown_numbers(
    token: &Token,
    format: &PlaceholderFormat,
    literal_sign: Option<Sign>,
) -> Option<(Vec<String>, bool)> {
    let signed = matches!(format, PlaceholderFormat::Signed);
    // How the sign before the atom applies: `Some(true)` negates.
    let negate = match (literal_sign, signed, token.sign) {
        (Some(literal), false, Some(sign)) if literal == sign => false,
        (None, false, None) | (None, true, Some(Sign::Plus)) => false,
        (None, false, Some(Sign::Minus)) | (None, true, Some(Sign::Minus)) => true,
        _ => return None,
    };
    let apply = |n: &str| {
        if negate {
            n.strip_prefix('-')
                .map_or_else(|| format!("-{n}"), str::to_owned)
        } else {
            n.to_owned()
        }
    };
    match &token.atom {
        Atom::Number(n) => Some((vec![apply(n)], false)),
        // The renderer writes a plain range without an outer sign; a signed
        // one as `+(a-b)`, or `-(|a|-|b|)` when both ends are negative.
        Atom::Range { min, max } => {
            let outer_sign_ok = signed || literal_sign.is_some() || token.sign.is_none();
            outer_sign_ok.then(|| (vec![apply(min), apply(max)], true))
        }
        Atom::Slot => None,
    }
}

const fn hull(a: StatValue, b: StatValue) -> StatValue {
    StatValue {
        min: if a.min < b.min { a.min } else { b.min },
        max: if a.max > b.max { a.max } else { b.max },
    }
}

fn intersect(a: StatValue, b: StatValue) -> Option<StatValue> {
    let v = StatValue {
        min: a.min.max(b.min),
        max: a.max.min(b.max),
    };
    (v.min <= v.max).then_some(v)
}

/// `value` narrowed to the raw values that meet `condition`.
fn narrow(condition: Condition, value: StatValue) -> Option<StatValue> {
    match condition {
        Condition::Range { min, max } => intersect(
            value,
            StatValue {
                min: min.unwrap_or(i64::MIN),
                max: max.unwrap_or(i64::MAX),
            },
        ),
        // An excluded value inside the interval cannot be cut out; it only
        // moves an end.
        Condition::Not(v) => {
            let min = if value.min == v {
                v.saturating_add(1)
            } else {
                value.min
            };
            let max = if value.max == v {
                v.saturating_sub(1)
            } else {
                value.max
            };
            (min <= max).then_some(StatValue { min, max })
        }
    }
}

/// Whether some raw value of `value` meets `condition`.
fn meets(condition: Condition, value: StatValue) -> bool {
    narrow(condition, value).is_some()
}

/// The value of a stat without a placeholder; `None` if no value meets the
/// condition.
fn hidden(condition: Condition) -> Option<RawValue> {
    let assumed = match condition {
        _ if condition.holds(0) => Some(0),
        Condition::Range {
            min: Some(min),
            max,
        } if min > 0 => {
            if max.is_some_and(|max| max < min) {
                return None;
            }
            Some(min)
        }
        Condition::Range {
            min,
            max: Some(max),
        } if max < 0 => {
            if min.is_some_and(|min| min > max) {
                return None;
            }
            Some(max)
        }
        Condition::Range { .. } | Condition::Not(_) => None,
    };
    Some(RawValue::Hidden { condition, assumed })
}

#[cfg(test)]
mod tests;
