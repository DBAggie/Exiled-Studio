//! Format text of a variant: markup stripping and placeholder parsing.
//!
//! Markup forms found in the English text of both installs (survey
//! 2026-11-09, PR B1; Spike A report section 12 left them to Spike B):
//! - `<tag>{{text}}` (tag letters and `_`): shown as `text`
//!   (`<enchanted>{{Blessed by Kaom}}`, `<craftingfire>{{Fire}}`);
//! - `[Ref|Text]`: shown as `Text`, which may hold placeholders
//!   (`[ArmourBreak|Break {0}% of Armour]`);
//! - `[Text]`: shown as `Text` (`[DNT]`, `[Attack]`);
//! - `<<Icon>>`: an inline icon, not text; removed.
//!
//! Then `{{` and `}}` are literal braces, `\n` (backslash, `n`) is a line
//! break, and `{[index][:format]}` is a placeholder (`{}` means stat 0).

use std::fmt;

/// A placeholder's number format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaceholderFormat {
    /// No format, or `d`.
    Plain,
    /// `+d` or `+`: the sign is always shown.
    Signed,
    /// Any other format as written (`-d` in PoE2). Kept, not guessed: the
    /// renderer reports it as unsupported.
    Other(String),
}

impl PlaceholderFormat {
    /// Reads a format as written after `:` (`""` when there is none).
    #[must_use]
    pub fn parse(format: &str) -> Self {
        match format {
            "" | "d" => Self::Plain,
            "+d" | "+" => Self::Signed,
            other => Self::Other(other.to_owned()),
        }
    }
}

/// A piece of a parsed format text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    /// Display text without markup.
    Text(String),
    /// A value: 0-based stat index and format.
    Value {
        /// 0-based stat index within the description (per hand).
        index: usize,
        /// Number format.
        format: PlaceholderFormat,
        /// The format as written (for checks against the extract).
        written: String,
    },
    /// A line break (`\n` in the text).
    LineBreak,
}

/// A parsed format text: segments in order, markup removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template {
    /// Segments in written order; adjacent text is merged.
    pub segments: Vec<Segment>,
}

/// Why a format text did not parse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateError {
    /// A `[`, `<tag>{{`, or `<<` without its end.
    UnclosedMarkup,
    /// A `{` without `}`, a lone `}`, or a bad index.
    BadPlaceholder,
    /// A placeholder index is not below the description's stat count.
    PlaceholderOutOfRange,
    /// The placeholders differ from the ones the extractor recorded.
    PlaceholderMismatch,
}

impl TemplateError {
    /// Stable name for reports.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::UnclosedMarkup => "unclosed-markup",
            Self::BadPlaceholder => "bad-placeholder",
            Self::PlaceholderOutOfRange => "placeholder-out-of-range",
            Self::PlaceholderMismatch => "placeholder-mismatch",
        }
    }
}

impl fmt::Display for TemplateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl Template {
    /// Parses format text for a description with `stats` stats.
    ///
    /// # Errors
    /// [`TemplateError`] when markup or a placeholder is malformed or out of
    /// range.
    pub fn parse(text: &str, stats: usize) -> Result<Self, TemplateError> {
        let plain = strip_markup(text)?;
        let segments = segments(&plain)?;
        let out_of_range = segments
            .iter()
            .any(|s| matches!(s, Segment::Value { index, .. } if *index >= stats));
        if out_of_range {
            return Err(TemplateError::PlaceholderOutOfRange);
        }
        Ok(Self { segments })
    }

    /// The value placeholders in order: index and format as written.
    pub fn placeholders(&self) -> impl Iterator<Item = (usize, &str)> {
        self.segments.iter().filter_map(|s| match s {
            Segment::Value { index, written, .. } => Some((*index, written.as_str())),
            Segment::Text(_) | Segment::LineBreak => None,
        })
    }
}

/// Removes markup, keeping the shown text, placeholders, and escapes.
///
/// A `<tag>{{` block ends at the first `}}`, so literal braces cannot
/// appear inside one (none do in either install, 2026-11-09). The same
/// rule bounds the recursion: a block's inner text holds no `}}`, so a
/// block nested in it has no end and fails at the second level.
///
/// # Errors
/// [`TemplateError::UnclosedMarkup`] when a markup form has no end.
pub fn strip_markup(text: &str) -> Result<String, TemplateError> {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        if let Some(after) = rest.strip_prefix("<<") {
            let end = after.find(">>").ok_or(TemplateError::UnclosedMarkup)?;
            rest = after.get(end + 2..).ok_or(TemplateError::UnclosedMarkup)?;
        } else if let Some(after) = tag_open(rest) {
            let end = after.find("}}").ok_or(TemplateError::UnclosedMarkup)?;
            let inner = after.get(..end).ok_or(TemplateError::UnclosedMarkup)?;
            out.push_str(&strip_markup(inner)?);
            rest = after.get(end + 2..).ok_or(TemplateError::UnclosedMarkup)?;
        } else if let Some(after) = rest.strip_prefix('[') {
            let end = after.find(']').ok_or(TemplateError::UnclosedMarkup)?;
            let inner = after.get(..end).ok_or(TemplateError::UnclosedMarkup)?;
            let shown = inner.split_once('|').map_or(inner, |(_, shown)| shown);
            out.push_str(shown);
            rest = after.get(end + 1..).ok_or(TemplateError::UnclosedMarkup)?;
        } else {
            out.push(c);
            rest = rest.get(c.len_utf8()..).unwrap_or_default();
        }
    }
    Ok(out)
}

/// `<tag>{{` at the start of `text`: the text after it.
fn tag_open(text: &str) -> Option<&str> {
    let after_lt = text.strip_prefix('<')?;
    let tag_len = after_lt
        .bytes()
        .take_while(|b| b.is_ascii_alphabetic() || *b == b'_')
        .count();
    if tag_len == 0 {
        return None;
    }
    after_lt.get(tag_len..)?.strip_prefix(">{{")
}

/// Splits markup-free text into segments.
fn segments(text: &str) -> Result<Vec<Segment>, TemplateError> {
    let mut out: Vec<Segment> = Vec::new();
    let mut buf = String::new();
    let mut rest = text;
    let flush = |buf: &mut String, out: &mut Vec<Segment>| {
        if !buf.is_empty() {
            out.push(Segment::Text(std::mem::take(buf)));
        }
    };
    while let Some(c) = rest.chars().next() {
        if let Some(after) = rest.strip_prefix("\\n") {
            flush(&mut buf, &mut out);
            out.push(Segment::LineBreak);
            rest = after;
        } else if let Some(after) = rest.strip_prefix("{{") {
            buf.push('{');
            rest = after;
        } else if let Some(after) = rest.strip_prefix("}}") {
            buf.push('}');
            rest = after;
        } else if c == '}' {
            return Err(TemplateError::BadPlaceholder);
        } else if let Some(after) = rest.strip_prefix('{') {
            let end = after.find('}').ok_or(TemplateError::BadPlaceholder)?;
            let spec = after.get(..end).ok_or(TemplateError::BadPlaceholder)?;
            let (index, written) = spec.split_once(':').unwrap_or((spec, ""));
            let index = if index.is_empty() {
                0
            } else if index.bytes().all(|b| b.is_ascii_digit()) {
                index.parse().map_err(|_| TemplateError::BadPlaceholder)?
            } else {
                return Err(TemplateError::BadPlaceholder);
            };
            if written.contains('{') {
                return Err(TemplateError::BadPlaceholder);
            }
            flush(&mut buf, &mut out);
            out.push(Segment::Value {
                index,
                format: PlaceholderFormat::parse(written),
                written: written.to_owned(),
            });
            rest = after.get(end + 1..).ok_or(TemplateError::BadPlaceholder)?;
        } else {
            buf.push(c);
            rest = rest.get(c.len_utf8()..).unwrap_or_default();
        }
    }
    flush(&mut buf, &mut out);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(s: &str) -> Segment {
        Segment::Text(s.to_owned())
    }

    fn value(index: usize, written: &str) -> Segment {
        Segment::Value {
            index,
            format: PlaceholderFormat::parse(written),
            written: written.to_owned(),
        }
    }

    #[test]
    fn strip_markup_should_keep_the_shown_text_of_each_form() {
        let stripped = strip_markup(
            "<<Icon>> [Ref|Shown] and [Plain], <enchanted>{{Blessed by [K|Kaom]}} {0}",
        );
        assert_eq!(
            stripped.as_deref(),
            Ok(" Shown and Plain, Blessed by Kaom {0}")
        );
    }

    #[test]
    fn strip_markup_should_keep_placeholders_inside_reference_text() {
        assert_eq!(
            strip_markup("Skills [ArmourBreak|Break {0}% of Armour]").as_deref(),
            Ok("Skills Break {0}% of Armour")
        );
    }

    #[test]
    fn strip_markup_should_fail_deep_nesting_without_deep_recursion() {
        // Would overflow the stack if the recursion followed the nesting.
        let deep = format!("{}x{}", "<a>{{".repeat(100_000), "}}".repeat(100_000));
        assert_eq!(strip_markup(&deep), Err(TemplateError::UnclosedMarkup));
    }

    #[test]
    fn strip_markup_should_leave_a_lone_angle_bracket_as_text() {
        assert_eq!(strip_markup("a < b > c").as_deref(), Ok("a < b > c"));
    }

    #[test]
    fn strip_markup_should_reject_each_unclosed_form() {
        for bad in ["[Ref|Text", "<tag>{{text", "<<Icon"] {
            assert_eq!(
                strip_markup(bad),
                Err(TemplateError::UnclosedMarkup),
                "{bad}"
            );
        }
    }

    #[test]
    fn parse_should_split_values_breaks_and_literal_braces() {
        let t = Template::parse("{0:+d}% of {{x}}\\nper {1} and {}", 2).unwrap();
        assert_eq!(
            t.segments,
            vec![
                value(0, "+d"),
                text("% of {x}"),
                Segment::LineBreak,
                text("per "),
                value(1, ""),
                text(" and "),
                value(0, ""),
            ]
        );
    }

    #[test]
    fn parse_should_read_every_placeholder_format() {
        let t = Template::parse("{0}{0:d}{0:+d}{0:+}{0:-d}", 1).unwrap();
        let formats: Vec<_> = t
            .segments
            .iter()
            .filter_map(|s| match s {
                Segment::Value { format, .. } => Some(format.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(
            formats,
            vec![
                PlaceholderFormat::Plain,
                PlaceholderFormat::Plain,
                PlaceholderFormat::Signed,
                PlaceholderFormat::Signed,
                PlaceholderFormat::Other("-d".to_owned()),
            ]
        );
    }

    #[test]
    fn parse_should_reject_bad_placeholders() {
        for bad in ["{0", "x}", "{a}", "{0:{}"] {
            assert_eq!(
                Template::parse(bad, 1),
                Err(TemplateError::BadPlaceholder),
                "{bad}"
            );
        }
    }

    #[test]
    fn parse_should_reject_an_index_beyond_the_stat_count() {
        assert_eq!(
            Template::parse("{1}", 1),
            Err(TemplateError::PlaceholderOutOfRange)
        );
    }

    #[test]
    fn parse_should_not_panic_on_any_prefix_of_marked_up_text() {
        let s = "<<I>> [A|é {0:+d}] <t>{{x}} {{}}\\n{1}";
        for (i, _) in s.char_indices() {
            let _ = Template::parse(s.get(..i).unwrap(), 2);
        }
    }
}
