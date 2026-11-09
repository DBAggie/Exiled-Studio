//! Forward renderer: stat values to display lines through a [`Scope`].
//!
//! Follows `v2.67.2:src/Modules/StatDescriber.lua` (cited as reference, not
//! as game evidence):
//! - a stat is rendered by the first file of the chain that describes it;
//!   descriptions are written farthest file first, then in file order;
//! - stats of a description that are not given count as 0; a description
//!   is rendered when one of its given stats is not 0;
//! - the first variant whose conditions hold for the **raw** values wins
//!   (`matchLimit`, which tests a range's minimum); `gem_quality` variants
//!   are skipped unless asked for, and so are `table_only` ones (PoE2);
//! - handlers apply in written order; a range is written `(a-b)`, a signed
//!   range `+(a-b)`, or `-(a-b)` when its maximum is negative.
//!
//! Every input stat that is not rendered comes back in
//! [`Rendered::not_rendered`] with a reason (ADR-015).

use std::collections::BTreeMap;

use super::handler::{Handler, LookupHandler, ValueFormat};
use super::scope::{DescRef, Scope};
use super::template::{PlaceholderFormat, Segment, TemplateError};
use super::{Description, Hand};

/// A raw stat value or a range of them (`min == max` for one value). The
/// caller keeps `min <= max`; conditions are tested on `min`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StatValue {
    /// Lower end.
    pub min: i64,
    /// Upper end.
    pub max: i64,
}

impl StatValue {
    /// One value.
    #[must_use]
    pub const fn exact(value: i64) -> Self {
        Self {
            min: value,
            max: value,
        }
    }

    const fn is_zero(self) -> bool {
        self.min == 0 && self.max == 0
    }
}

/// Names rows of other tables for lookup handlers. The host builds it from
/// game data; `None` means the table or the key is not available.
pub trait Lookup {
    /// The display name for `value` under `handler`.
    fn name(&self, handler: LookupHandler, value: i64) -> Option<String>;
}

/// A [`Lookup`] that knows nothing.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoLookup;

impl Lookup for NoLookup {
    fn name(&self, _: LookupHandler, _: i64) -> Option<String> {
        None
    }
}

/// Which flagged variants may be chosen.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VariantFilter {
    /// Allow `gem_quality` variants.
    pub gem_quality: bool,
    /// Allow `table_only` variants.
    pub table_only: bool,
}

/// One rendered line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedLine {
    /// The text, markup removed, trimmed.
    pub text: String,
    /// The description it came from.
    pub at: DescRef,
    /// Index of the chosen variant.
    pub variant: usize,
}

/// Why a stat was not rendered.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum NotRenderedReason {
    /// The value is 0 (nothing to show).
    Zero,
    /// No file of the scope describes the stat.
    NoDescription,
    /// No description, and a file of the scope declares `no_description`.
    DeclaredNoDescription,
    /// No allowed variant's conditions hold.
    NoMatchingVariant,
    /// The chosen variant's text did not parse.
    BadTemplate(String),
    /// The chosen variant has a handler without known semantics.
    UnknownHandler(String),
    /// A placeholder format without known semantics.
    UnsupportedFormat(String),
    /// The lookup has no name for the value.
    MissingLookup {
        /// Handler name.
        handler: &'static str,
        /// Raw value.
        value: i64,
    },
    /// A lookup handler got a range; a range names no single row.
    LookupRange(&'static str),
    /// The chosen variant has a handler with a bad argument.
    MalformedHandler(String),
    /// The chosen variant has a flag without known meaning.
    UnknownFlag(String),
    /// The chosen variant renders to no text (empty, or only an icon).
    EmptyLine,
}

impl NotRenderedReason {
    /// Stable name for counts.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Zero => "zero",
            Self::NoDescription => "no-description",
            Self::DeclaredNoDescription => "declared-no-description",
            Self::NoMatchingVariant => "no-matching-variant",
            Self::BadTemplate(_) => "bad-template",
            Self::UnknownHandler(_) => "unknown-handler",
            Self::UnsupportedFormat(_) => "unsupported-format",
            Self::MissingLookup { .. } => "missing-lookup",
            Self::LookupRange(_) => "lookup-range",
            Self::MalformedHandler(_) => "malformed-handler",
            Self::UnknownFlag(_) => "unknown-flag",
            Self::EmptyLine => "empty-line",
        }
    }
}

/// An input stat that was not rendered.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NotRendered {
    /// Stat ID.
    pub stat: String,
    /// Why.
    pub reason: NotRenderedReason,
}

/// Output of [`Scope::render`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Rendered {
    /// Lines in display order.
    pub lines: Vec<RenderedLine>,
    /// Every input stat not rendered, sorted by stat.
    pub not_rendered: Vec<NotRendered>,
}

impl Scope<'_> {
    /// Renders `stats` (stat ID to value).
    #[must_use]
    pub fn render(
        &self,
        stats: &BTreeMap<String, StatValue>,
        lookup: &dyn Lookup,
        filter: VariantFilter,
    ) -> Rendered {
        let mut out = Rendered::default();
        // Farthest file first, then file order, main hand first.
        let mut groups: BTreeMap<(std::cmp::Reverse<usize>, usize, Hand), Vec<&str>> =
            BTreeMap::new();
        for (stat, value) in stats {
            let reason = if value.is_zero() {
                NotRenderedReason::Zero
            } else if let Some(at) = self.find(stat) {
                groups
                    .entry((std::cmp::Reverse(at.depth), at.index, at.hand))
                    .or_default()
                    .push(stat);
                continue;
            } else if self.declares_no_description(stat) {
                NotRenderedReason::DeclaredNoDescription
            } else {
                NotRenderedReason::NoDescription
            };
            out.not_rendered.push(NotRendered {
                stat: stat.clone(),
                reason,
            });
        }
        for ((std::cmp::Reverse(depth), index, hand), given) in groups {
            let at = DescRef { depth, index, hand };
            let Some((_, description)) = self.description(at) else {
                continue;
            };
            let ids = description.stats_of(hand).unwrap_or_default();
            let values: Vec<StatValue> = ids
                .iter()
                .map(|id| stats.get(id).copied().unwrap_or(StatValue::exact(0)))
                .collect();
            match render_description(description, hand, &values, lookup, filter) {
                Ok((variant, lines)) => {
                    out.lines.extend(lines.into_iter().map(|text| RenderedLine {
                        text,
                        at,
                        variant,
                    }))
                }
                Err(reason) => out
                    .not_rendered
                    .extend(given.into_iter().map(|stat| NotRendered {
                        stat: stat.to_owned(),
                        reason: reason.clone(),
                    })),
            }
        }
        out.not_rendered.sort();
        out
    }
}

/// A value on its way to text.
#[derive(Debug, Clone)]
struct Shown {
    min: f64,
    max: f64,
    format: ValueFormat,
    name: Option<String>,
}

/// Renders one description of one hand with values in its stat order
/// (missing values count as 0). Returns the chosen variant and its lines.
///
/// # Errors
/// The [`NotRenderedReason`] when no line can be written.
pub fn render_description(
    description: &Description,
    hand: Hand,
    values: &[StatValue],
    lookup: &dyn Lookup,
    filter: VariantFilter,
) -> Result<(usize, Vec<String>), NotRenderedReason> {
    let count = description.stats_of(hand).map_or(0, <[String]>::len);
    let raw: Vec<StatValue> = (0..count)
        .map(|i| values.get(i).copied().unwrap_or(StatValue::exact(0)))
        .collect();
    let (index, variant) = description
        .variants
        .iter()
        .enumerate()
        .find(|(_, v)| {
            (filter.gem_quality || !v.gem_quality)
                && (filter.table_only || !v.table_only)
                && v.conditions
                    .iter()
                    .zip(&raw)
                    .all(|(c, value)| c.holds(value.min))
        })
        .ok_or(NotRenderedReason::NoMatchingVariant)?;
    if let Some(flag) = variant.unknown_flags.first() {
        return Err(NotRenderedReason::UnknownFlag(flag.clone()));
    }
    let template = variant
        .template
        .as_ref()
        .map_err(|e: &TemplateError| NotRenderedReason::BadTemplate(e.name().to_owned()))?;
    #[expect(
        clippy::cast_precision_loss,
        reason = "stat values are game integers far below 2^53"
    )]
    let mut shown: Vec<Shown> = raw
        .iter()
        .map(|v| Shown {
            min: v.min as f64,
            max: v.max as f64,
            format: ValueFormat::Integer,
            name: None,
        })
        .collect();
    for handler in &variant.handlers {
        match handler {
            Handler::Numeric { handler, stat } => {
                if let Some(s) = shown.get_mut(*stat) {
                    let (a, b) = (handler.apply(s.min), handler.apply(s.max));
                    (s.min, s.max) = if a <= b { (a, b) } else { (b, a) };
                    if let Some(format) = handler.format {
                        s.format = format;
                    }
                }
            }
            Handler::Lookup { handler, stat } => {
                let (Some(s), Some(value)) = (shown.get_mut(*stat), raw.get(*stat)) else {
                    continue;
                };
                if value.min != value.max {
                    return Err(NotRenderedReason::LookupRange(handler.name()));
                }
                s.name = Some(lookup.name(*handler, value.min).ok_or(
                    NotRenderedReason::MissingLookup {
                        handler: handler.name(),
                        value: value.min,
                    },
                )?);
            }
            Handler::Unknown { name, .. } => {
                return Err(NotRenderedReason::UnknownHandler(name.clone()));
            }
            Handler::Malformed { name } => {
                return Err(NotRenderedReason::MalformedHandler(name.clone()));
            }
            Handler::Reminder(_)
            | Handler::CanonicalLine
            | Handler::Markup
            | Handler::CanonicalStat { .. } => {}
        }
    }
    let mut lines = vec![String::new()];
    for segment in &template.segments {
        match segment {
            Segment::Text(text) => push(&mut lines, text),
            Segment::LineBreak => lines.push(String::new()),
            Segment::Value { index, format, .. } => {
                let s = shown.get(*index).ok_or(NotRenderedReason::BadTemplate(
                    TemplateError::PlaceholderOutOfRange.name().to_owned(),
                ))?;
                let text = match (&s.name, format) {
                    (Some(name), _) => name.clone(),
                    (None, PlaceholderFormat::Other(f)) => {
                        return Err(NotRenderedReason::UnsupportedFormat(f.clone()));
                    }
                    (None, PlaceholderFormat::Plain) => value_text(s, false),
                    (None, PlaceholderFormat::Signed) => value_text(s, true),
                };
                push(&mut lines, &text);
            }
        }
    }
    let lines = lines
        .into_iter()
        .map(|l| l.trim().to_owned())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>();
    if lines.is_empty() {
        return Err(NotRenderedReason::EmptyLine);
    }
    Ok((index, lines))
}

fn push(lines: &mut [String], text: &str) {
    if let Some(last) = lines.last_mut() {
        last.push_str(text);
    }
}

fn value_text(s: &Shown, signed: bool) -> String {
    let (min, max) = (
        format_number(s.min, s.format),
        format_number(s.max, s.format),
    );
    // PoB compares the numbers (`v.min == v.max`), not the written text.
    #[expect(clippy::float_cmp, reason = "reproduces PoB's exact comparison")]
    let one_value = s.min == s.max;
    if one_value {
        return if signed && !min.starts_with('-') {
            format!("+{min}")
        } else {
            min
        };
    }
    if !signed {
        return format!("({min}-{max})");
    }
    if s.max < 0.0 {
        format!(
            "-({}-{})",
            format_number(-s.min, s.format),
            format_number(-s.max, s.format)
        )
    } else {
        format!("+({min}-{max})")
    }
}

/// Writes a number like C `printf` with the given format.
#[must_use]
pub fn format_number(x: f64, format: ValueFormat) -> String {
    // -0 prints as 0.
    let x = if x == 0.0 { 0.0 } else { x };
    match format {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "C %d truncates; values are game integers after handlers"
        )]
        ValueFormat::Integer => format!("{}", x.trunc() as i64),
        ValueFormat::Fixed2 => format!("{x:.2}"),
        ValueFormat::General => format_general(x),
    }
}

/// C `%g` (precision 6): shortest of fixed or scientific, zeros trimmed.
fn format_general(x: f64) -> String {
    if x == 0.0 || !x.is_finite() {
        return format!("{x}");
    }
    let sci = format!("{x:.5e}");
    let exp: i32 = sci
        .split_once('e')
        .and_then(|(_, e)| e.parse().ok())
        .unwrap_or(0);
    if (-4..6).contains(&exp) {
        let decimals = usize::try_from(5 - exp).unwrap_or(0);
        trim_zeros(format!("{x:.decimals$}"))
    } else {
        let (mantissa, _) = sci.split_once('e').unwrap_or((&sci, ""));
        let sign = if exp < 0 { '-' } else { '+' };
        format!("{}e{sign}{:02}", trim_zeros(mantissa.to_owned()), exp.abs())
    }
}

fn trim_zeros(s: String) -> String {
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_owned()
    } else {
        s
    }
}

#[cfg(test)]
#[expect(
    clippy::indexing_slicing,
    reason = "tests index fixed synthetic data; a bad index should panic"
)]
mod tests {
    use super::*;
    use crate::stat_desc::Condition;
    use crate::stat_desc::scope::tests::{desc, file, variant};
    use crate::stat_desc::{StatDescFile, StatDescriptions};

    fn stats(pairs: &[(&str, i64)]) -> BTreeMap<String, StatValue> {
        pairs
            .iter()
            .map(|(s, v)| ((*s).to_owned(), StatValue::exact(*v)))
            .collect()
    }

    fn one_file(descriptions: Vec<Description>) -> StatDescriptions {
        let mut set = StatDescriptions::new();
        set.insert(file("a.txt", &[], descriptions));
        set
    }

    fn texts(set: &StatDescriptions, values: &BTreeMap<String, StatValue>) -> Vec<String> {
        let scope = set.scope("a.txt").unwrap();
        let r = scope.render(values, &NoLookup, VariantFilter::default());
        r.lines.into_iter().map(|l| l.text).collect()
    }

    fn handled(stats: &[&str], text: &str, handlers: &[(&str, Option<usize>)]) -> Description {
        let mut d = desc(stats, text);
        d.variants[0].handlers = handlers
            .iter()
            .map(|(n, s)| Handler::classify(n, *s, Some("R"), stats.len()).unwrap())
            .collect();
        d
    }

    /// Increased/reduced pair: `1|#` then `#|-1` with `negate`.
    fn increased_reduced() -> Description {
        let mut d = desc(&["s"], "{0}% increased X");
        d.variants[0].conditions = vec![Condition::Range {
            min: Some(1),
            max: None,
        }];
        let mut reduced = variant(1, "{0}% reduced X");
        reduced.conditions = vec![Condition::Range {
            min: None,
            max: Some(-1),
        }];
        reduced.handlers = vec![Handler::classify("negate", Some(1), None, 1).unwrap()];
        d.variants.push(reduced);
        d
    }

    #[test]
    fn render_should_choose_the_variant_whose_condition_holds() {
        let set = one_file(vec![increased_reduced()]);
        assert_eq!(texts(&set, &stats(&[("s", 10)])), vec!["10% increased X"]);
        assert_eq!(texts(&set, &stats(&[("s", -10)])), vec!["10% reduced X"]);
    }

    #[test]
    fn render_should_report_values_no_condition_accepts() {
        let set = one_file(vec![increased_reduced()]);
        let scope = set.scope("a.txt").unwrap();
        let mut d = increased_reduced();
        d.variants.truncate(1);
        let r = render_description(
            &d,
            Hand::Main,
            &[StatValue::exact(-1)],
            &NoLookup,
            VariantFilter::default(),
        );
        assert_eq!(r, Err(NotRenderedReason::NoMatchingVariant));
        assert!(scope.find("s").is_some());
    }

    #[test]
    fn render_should_skip_gem_quality_variants_unless_asked() {
        let mut d = desc(&["s"], "quality {0}");
        d.variants[0].gem_quality = true;
        d.variants.push(variant(1, "plain {0}"));
        let plain = render_description(
            &d,
            Hand::Main,
            &[StatValue::exact(1)],
            &NoLookup,
            VariantFilter::default(),
        );
        let quality = VariantFilter {
            gem_quality: true,
            table_only: false,
        };
        let gem = render_description(&d, Hand::Main, &[StatValue::exact(1)], &NoLookup, quality);
        assert_eq!((plain.unwrap().0, gem.unwrap().0), (1, 0));
    }

    #[test]
    fn render_should_fill_missing_stats_with_zero() {
        let mut d = desc(&["a", "b"], "{0} and {1}");
        d.variants[0].conditions[1] = Condition::Range {
            min: Some(0),
            max: Some(0),
        };
        let set = one_file(vec![d]);
        assert_eq!(texts(&set, &stats(&[("a", 3)])), vec!["3 and 0"]);
    }

    #[test]
    fn render_should_write_signed_and_ranged_values() {
        let set = one_file(vec![desc(&["s"], "{0:+d} to X")]);
        assert_eq!(texts(&set, &stats(&[("s", 5)])), vec!["+5 to X"]);
        assert_eq!(texts(&set, &stats(&[("s", -5)])), vec!["-5 to X"]);
        let mut range = BTreeMap::new();
        range.insert("s".to_owned(), StatValue { min: 5, max: 10 });
        assert_eq!(texts(&set, &range), vec!["+(5-10) to X"]);
        range.insert("s".to_owned(), StatValue { min: -10, max: -5 });
        assert_eq!(texts(&set, &range), vec!["-(10-5) to X"]);
    }

    #[test]
    fn render_should_write_a_plain_range_in_parentheses() {
        let set = one_file(vec![desc(&["s"], "{0}% more")]);
        let mut range = BTreeMap::new();
        range.insert("s".to_owned(), StatValue { min: 10, max: 20 });
        assert_eq!(texts(&set, &range), vec!["(10-20)% more"]);
    }

    #[test]
    fn render_should_apply_numeric_handlers_with_their_format() {
        let cases = [
            ("divide_by_one_hundred", 150, "1.5"),
            ("divide_by_one_hundred", 100, "1"),
            ("milliseconds_to_seconds_2dp_if_required", 1234, "1.23"),
            ("per_minute_to_per_second", 100, "1.7"),
            ("divide_by_two_0dp", 5, "2"),
            ("30%_of_value", 15, "4"),
            ("deciseconds_to_seconds", 15, "1.50"),
            ("negate_and_double", -3, "6"),
            ("divide_by_three", 10, "3.33333"),
        ];
        for (name, value, expected) in cases {
            let set = one_file(vec![handled(&["s"], "{0}", &[(name, Some(1))])]);
            assert_eq!(
                texts(&set, &stats(&[("s", value)])),
                vec![expected],
                "{name}"
            );
        }
    }

    #[test]
    fn render_should_swap_a_range_that_a_negating_handler_reverses() {
        let set = one_file(vec![handled(&["s"], "{0}", &[("negate", Some(1))])]);
        let mut range = BTreeMap::new();
        range.insert("s".to_owned(), StatValue { min: -20, max: -10 });
        assert_eq!(texts(&set, &range), vec!["(10-20)"]);
    }

    #[test]
    fn render_should_split_lines_and_strip_markup() {
        let set = one_file(vec![desc(
            &["s"],
            "<<I>> [A|Gain] {0}\\n<tag>{{Second}} line",
        )]);
        assert_eq!(
            texts(&set, &stats(&[("s", 2)])),
            vec!["Gain 2", "Second line"]
        );
    }

    #[test]
    fn render_should_name_lookup_values_through_the_host() {
        struct Names;
        impl Lookup for Names {
            fn name(&self, handler: LookupHandler, value: i64) -> Option<String> {
                (handler == LookupHandler::PassiveHash && value == 7).then(|| "Whispers".to_owned())
            }
        }
        let d = handled(&["s"], "Allocates {0}", &[("passive_hash", Some(1))]);
        let ok = render_description(
            &d,
            Hand::Main,
            &[StatValue::exact(7)],
            &Names,
            VariantFilter::default(),
        );
        assert_eq!(ok.unwrap().1, vec!["Allocates Whispers"]);
        let missing = render_description(
            &d,
            Hand::Main,
            &[StatValue::exact(8)],
            &Names,
            VariantFilter::default(),
        );
        assert_eq!(
            missing,
            Err(NotRenderedReason::MissingLookup {
                handler: "passive_hash",
                value: 8
            })
        );
        let range = render_description(
            &d,
            Hand::Main,
            &[StatValue { min: 7, max: 8 }],
            &Names,
            VariantFilter::default(),
        );
        assert_eq!(range, Err(NotRenderedReason::LookupRange("passive_hash")));
    }

    #[test]
    fn render_should_report_unknown_handlers_and_formats() {
        let d = handled(&["s"], "{0}", &[("old_leech_percent", Some(1))]);
        let r = render_description(
            &d,
            Hand::Main,
            &[StatValue::exact(1)],
            &NoLookup,
            VariantFilter::default(),
        );
        assert_eq!(
            r,
            Err(NotRenderedReason::UnknownHandler(
                "old_leech_percent".to_owned()
            ))
        );
        let d = desc(&["s"], "{0:-d}%");
        let r = render_description(
            &d,
            Hand::Main,
            &[StatValue::exact(1)],
            &NoLookup,
            VariantFilter::default(),
        );
        assert_eq!(
            r,
            Err(NotRenderedReason::UnsupportedFormat("-d".to_owned()))
        );
    }

    #[test]
    fn render_should_report_every_stat_it_did_not_render() {
        let mut f: StatDescFile = file("a.txt", &[], vec![desc(&["shown"], "{0}")]);
        f.no_description.push("hidden".to_owned());
        let mut set = StatDescriptions::new();
        set.insert(f);
        let scope = set.scope("a.txt").unwrap();
        let r = scope.render(
            &stats(&[("shown", 1), ("hidden", 1), ("zero", 0), ("nowhere", 2)]),
            &NoLookup,
            VariantFilter::default(),
        );
        let reasons: Vec<_> = r
            .not_rendered
            .iter()
            .map(|n| (n.stat.as_str(), n.reason.name()))
            .collect();
        assert_eq!(
            reasons,
            vec![
                ("hidden", "declared-no-description"),
                ("nowhere", "no-description"),
                ("zero", "zero")
            ]
        );
        assert_eq!(r.lines.len(), 1);
    }

    #[test]
    fn render_should_write_farther_files_first_and_use_the_nearest_description() {
        let mut set = StatDescriptions::new();
        set.insert(file(
            "a.txt",
            &["b.txt"],
            vec![desc(&["near"], "near {0}"), desc(&["both"], "own {0}")],
        ));
        set.insert(file(
            "b.txt",
            &[],
            vec![desc(&["both"], "base {0}"), desc(&["far"], "far {0}")],
        ));
        let scope = set.scope("a.txt").unwrap();
        let r = scope.render(
            &stats(&[("near", 1), ("both", 2), ("far", 3)]),
            &NoLookup,
            VariantFilter::default(),
        );
        let lines: Vec<_> = r.lines.into_iter().map(|l| l.text).collect();
        assert_eq!(lines, vec!["far 3", "near 1", "own 2"]);
    }

    #[test]
    fn render_should_render_each_hand_of_a_handed_description() {
        let mut d = desc(&["main_min", "main_max"], "{0} to {1} Damage");
        d.off_hand_stats = Some(vec!["off_min".to_owned(), "off_max".to_owned()]);
        let set = one_file(vec![d]);
        let lines = texts(
            &set,
            &stats(&[
                ("main_min", 1),
                ("main_max", 2),
                ("off_min", 3),
                ("off_max", 4),
            ]),
        );
        assert_eq!(lines, vec!["1 to 2 Damage", "3 to 4 Damage"]);
    }

    #[test]
    fn render_should_write_a_range_that_formats_equal_as_a_range_like_pob() {
        // divide_by_two_0dp: 20 -> 10, 21 -> 10.5; `%d` writes both as 10,
        // and PoB still writes a range because the numbers differ.
        let set = one_file(vec![handled(
            &["s"],
            "{0}",
            &[("divide_by_two_0dp", Some(1))],
        )]);
        let mut range = BTreeMap::new();
        range.insert("s".to_owned(), StatValue { min: 20, max: 21 });
        assert_eq!(texts(&set, &range), vec!["(10-10)"]);
    }

    #[test]
    fn render_should_report_a_variant_that_renders_no_text() {
        let d = desc(&["s"], "<<IconOnly>>");
        let r = render_description(
            &d,
            Hand::Main,
            &[StatValue::exact(1)],
            &NoLookup,
            VariantFilter::default(),
        );
        assert_eq!(r, Err(NotRenderedReason::EmptyLine));
    }

    #[test]
    fn render_should_report_a_chosen_variant_with_an_unknown_flag() {
        let mut d = desc(&["s"], "{0}");
        d.variants[0].unknown_flags.push("new_flag".to_owned());
        let r = render_description(
            &d,
            Hand::Main,
            &[StatValue::exact(1)],
            &NoLookup,
            VariantFilter::default(),
        );
        assert_eq!(
            r,
            Err(NotRenderedReason::UnknownFlag("new_flag".to_owned()))
        );
    }

    #[test]
    fn render_should_report_a_malformed_handler() {
        let mut d = desc(&["s"], "{0}");
        d.variants[0].handlers.push(Handler::Malformed {
            name: "negate".to_owned(),
        });
        let r = render_description(
            &d,
            Hand::Main,
            &[StatValue::exact(1)],
            &NoLookup,
            VariantFilter::default(),
        );
        assert_eq!(
            r,
            Err(NotRenderedReason::MalformedHandler("negate".to_owned()))
        );
    }

    /// Every PoB-cited handler against `applySpecial` computed by hand
    /// (`v2.67.2:src/Modules/StatDescriber.lua`): value in, text out.
    #[test]
    fn render_should_match_hand_computed_pob_output_for_every_cited_handler() {
        let cases = [
            ("30%_of_value", 15, "4"),
            ("60%_of_value", 15, "9"),
            ("deciseconds_to_seconds", 15, "1.50"),
            ("divide_by_fifteen_0dp", 40, "2"),
            ("divide_by_five", 7, "1.4"),
            ("divide_by_four", 5, "1.25"),
            ("divide_by_one_hundred", 150, "1.5"),
            ("divide_by_one_hundred_1dp", 125, "1.3"),
            ("divide_by_one_hundred_2dp", 1234, "12.34"),
            ("divide_by_one_hundred_2dp_if_required", 1200, "12"),
            ("divide_by_one_hundred_and_negate", 125, "-1.3"),
            ("divide_by_one_thousand", 1500, "1.5"),
            ("divide_by_six", 9, "1.5"),
            ("divide_by_ten_0dp", 25, "2"),
            ("divide_by_ten_1dp", 15, "1.5"),
            ("divide_by_ten_1dp_if_required", 20, "2"),
            ("divide_by_twelve", 18, "1.5"),
            ("divide_by_twenty", 30, "1.5"),
            ("divide_by_twenty_then_double_0dp", 30, "4"),
            ("divide_by_two_0dp", 5, "2"),
            ("double", 4, "8"),
            ("invert_chance", 30, "70"),
            ("locations_to_metres", 15, "1.5"),
            ("milliseconds_to_seconds", 1250, "1.25"),
            ("milliseconds_to_seconds_0dp", 1900, "1"),
            ("milliseconds_to_seconds_1dp", 1250, "1.3"),
            ("milliseconds_to_seconds_2dp", 1234, "1.23"),
            ("milliseconds_to_seconds_2dp_if_required", 1500, "1.5"),
            ("multiplicative_damage_modifier", -20, "80"),
            ("multiplicative_permyriad_damage_modifier", 250, "102.5"),
            ("multiply_by_four", 3, "12"),
            ("negate", -5, "5"),
            ("negate_and_double", -3, "6"),
            ("per_minute_to_per_second", 100, "1.7"),
            ("per_minute_to_per_second_0dp", 90, "1"),
            ("per_minute_to_per_second_1dp", 100, "1.7"),
            ("per_minute_to_per_second_2dp", 100, "1.67"),
            ("per_minute_to_per_second_2dp_if_required", 120, "2"),
            ("permyriad_per_minute_to_%_per_second", 9000, "1.5"),
            ("plus_two_hundred", 5, "205"),
            ("times_one_point_five", 3, "4"),
            ("times_twenty", 3, "60"),
        ];
        let cited = crate::stat_desc::handler::NUMERIC_HANDLERS
            .iter()
            .filter(|h| h.basis == crate::stat_desc::Basis::PobCited)
            .count();
        assert_eq!(cases.len(), cited, "one case per PoB-cited handler");
        for (name, value, expected) in cases {
            let d = handled(&["s"], "{0}", &[(name, Some(1))]);
            let r = render_description(
                &d,
                Hand::Main,
                &[StatValue::exact(value)],
                &NoLookup,
                VariantFilter::default(),
            );
            assert_eq!(r.map(|(_, l)| l), Ok(vec![expected.to_owned()]), "{name}");
        }
    }

    #[test]
    fn format_general_should_match_c_percent_g() {
        let cases = [
            (0.5, "0.5"),
            (1.0, "1"),
            (-0.25, "-0.25"),
            (123_456.0, "123456"),
            (1_234_567.0, "1.23457e+06"),
            (0.000_01, "1e-05"),
            (2.0 / 3.0, "0.666667"),
        ];
        for (x, expected) in cases {
            assert_eq!(format_number(x, ValueFormat::General), expected, "{x}");
        }
    }

    #[test]
    fn format_number_should_print_negative_zero_as_zero() {
        assert_eq!(format_number(-0.0, ValueFormat::Integer), "0");
        assert_eq!(format_number(-0.0, ValueFormat::General), "0");
    }
}
