//! Reverse matcher (Spike B, PR B2a): one item line to stat IDs and raw
//! values through the stat descriptions of one [`Scope`] (ADR-015).
//!
//! 1. [`Index::build`] indexes every variant of the scope's descriptions
//!    (both hands of a handed description) by its **key**: the template
//!    text without markup, each placeholder and each literal number
//!    replaced by a slot ([`token`]). Left out, and counted in
//!    [`IndexStats`]: `gem_quality` and `table_only` variants (PoB
//!    `matchLimit` skips them for items), templates that did not parse,
//!    templates with no text, and, until PR B2b, multi-line templates and
//!    variants with lookup handlers.
//! 2. [`Index::resolve`] normalizes the line, tokenizes it, and looks up its
//!    key. Each variant under the key is **verified**: literal tokens must
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
//! condition holds on one side.
//!
//! Every line ends in exactly one [`Outcome`]; nothing is dropped.

pub mod token;

use std::collections::BTreeMap;

use loom_data::stat_desc::{
    Condition, DescRef, Description, Hand, Handler, NumericHandler, PlaceholderFormat, Scope,
    Segment, StatValue, Variant, invert_shown,
};

use token::{Atom, SLOT, Sign, Token, collapse, normalize, tokenize};

/// Counts of the variants an [`Index`] holds and leaves out.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IndexStats {
    /// Variants indexed (one per hand of a handed description).
    pub indexed: usize,
    /// Distinct keys.
    pub keys: usize,
    /// Left out: flagged `gem_quality`.
    pub gem_quality: usize,
    /// Left out: flagged `table_only`.
    pub table_only: usize,
    /// Left out: the template did not parse.
    pub bad_template: usize,
    /// Left out: the template has no text (a deliberately hidden line).
    pub empty_text: usize,
    /// Left out until PR B2b: the template has a line break.
    pub deferred_multi_line: usize,
    /// Left out until PR B2b: the variant has a lookup handler.
    pub deferred_lookup: usize,
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

/// One indexed variant.
#[derive(Debug, Clone)]
struct Entry<'a> {
    at: DescRef,
    description: &'a Description,
    variant_index: usize,
    variant: &'a Variant,
    slots: Vec<Slot>,
    shadowed: bool,
}

/// Variants of one [`Scope`] by key.
#[derive(Debug, Clone)]
pub struct Index<'a> {
    entries: BTreeMap<String, Vec<Entry<'a>>>,
    stats: IndexStats,
}

/// A raw stat value read from a line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// The outcome for one line and the shadowed matches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolution {
    /// The line after [`normalize`].
    pub line: String,
    /// The result.
    pub outcome: Outcome,
    /// Descriptions that verified but are shadowed under the include
    /// precedence rule, sorted by position.
    pub shadowed: Vec<Candidate>,
}

impl<'a> Index<'a> {
    /// Indexes the variants of every description in `scope`.
    #[must_use]
    pub fn build(scope: &Scope<'a>) -> Self {
        let mut entries: BTreeMap<String, Vec<Entry<'a>>> = BTreeMap::new();
        let mut stats = IndexStats::default();
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
                    let Some((key, slots)) = index_variant(variant, ids.len(), &mut stats) else {
                        continue;
                    };
                    stats.indexed += 1;
                    entries.entry(key).or_default().push(Entry {
                        at,
                        description,
                        variant_index,
                        variant,
                        slots,
                        shadowed,
                    });
                }
            }
        }
        stats.keys = entries.len();
        Self { entries, stats }
    }

    /// What was indexed and left out.
    #[must_use]
    pub const fn stats(&self) -> IndexStats {
        self.stats
    }

    /// Resolves one item line.
    #[must_use]
    pub fn resolve(&self, line: &str) -> Resolution {
        let line = normalize(line);
        let tokenized = tokenize(&line);
        let Some(entries) = self.entries.get(&tokenized.key) else {
            return Resolution {
                line,
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
            outcome,
            shadowed,
        }
    }
}

/// The key and slots of an indexable variant for a hand with `stat_count`
/// stats; `None` (counted) otherwise.
fn index_variant(
    variant: &Variant,
    stat_count: usize,
    stats: &mut IndexStats,
) -> Option<(String, Vec<Slot>)> {
    if variant.gem_quality {
        stats.gem_quality += 1;
        return None;
    }
    if variant.table_only {
        stats.table_only += 1;
        return None;
    }
    let Ok(template) = &variant.template else {
        stats.bad_template += 1;
        return None;
    };
    if template.segments.contains(&Segment::LineBreak) {
        stats.deferred_multi_line += 1;
        return None;
    }
    if variant
        .handlers
        .iter()
        .any(|h| matches!(h, Handler::Lookup { .. }))
    {
        stats.deferred_lookup += 1;
        return None;
    }
    let mut skeleton = String::new();
    let mut placeholders = Vec::new();
    for segment in &template.segments {
        match segment {
            Segment::Text(text) => skeleton.push_str(&text.replace(SLOT, "\u{FFFD}")),
            Segment::Value { index, format, .. } => {
                skeleton.push(SLOT);
                placeholders.push((*index, format.clone()));
            }
            Segment::LineBreak => {}
        }
    }
    let tokenized = tokenize(&collapse(&skeleton));
    if tokenized.key.is_empty() {
        stats.empty_text += 1;
        return None;
    }
    let Some(slots) = slots(tokenized.tokens, placeholders, stat_count) else {
        stats.bad_template += 1;
        return None;
    };
    Some((tokenized.key, slots))
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
