//! Normalization and value tokens, shared by item lines and templates.
//!
//! A line and a template are compared by their **key**: the normalized text
//! with every value token replaced by [`SLOT`]. Item lines and template
//! skeletons (template text with each placeholder replaced by [`SLOT`]) go
//! through the same tokenizer, so literal digits in template text
//! (`Level 20`) become tokens exactly as the numbers of a line do, and are
//! checked as literals when a match is verified.
//!
//! A value token is an optional sign and an atom:
//! - a number, `digits` or `digits.digits`;
//! - a range, `(a-b)`, where `a` and `b` are numbers with an optional `-`
//!   (`(-10-10)`, as written for a range that crosses zero);
//! - [`SLOT`] (template skeletons only).
//!
//! A `+` or `-` is the token's sign only when it directly precedes an atom
//! and does not follow a letter, a digit, or another token (`5-10` without
//! parentheses is two numbers and a dash).

/// Stands for a value token in keys and for a placeholder in template
/// skeletons. A private-use character; line text holding it is rewritten
/// to U+FFFD first, so a line can never forge a slot.
pub const SLOT: char = '\u{E000}';

/// Sign written before an atom.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sign {
    /// `+`.
    Plus,
    /// `-`.
    Minus,
}

/// What a value token holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Atom {
    /// A number as written, without the token's sign.
    Number(String),
    /// A range `(min-max)`; each end as written with its own `-`.
    Range {
        /// Lower end as written.
        min: String,
        /// Upper end as written.
        max: String,
    },
    /// A placeholder of a template skeleton.
    Slot,
}

/// One value token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    /// The sign before the atom, if any.
    pub sign: Option<Sign>,
    /// The atom.
    pub atom: Atom,
    /// The token as written (sign included).
    pub text: String,
}

/// A text split into its key and its value tokens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tokenized {
    /// The text with each token replaced by [`SLOT`].
    pub key: String,
    /// Tokens in order (one per [`SLOT`] in `key`).
    pub tokens: Vec<Token>,
}

/// Normalizes an item line: [`SLOT`] rewritten to U+FFFD, surrounding
/// whitespace removed, inner whitespace runs collapsed to one space. Case
/// and punctuation are kept (the game's text is exact).
#[must_use]
pub fn normalize(line: &str) -> String {
    collapse(&line.replace(SLOT, "\u{FFFD}"))
}

/// Trims and collapses whitespace runs to one space.
pub(crate) fn collapse(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Splits normalized text into its key and tokens.
#[must_use]
pub fn tokenize(text: &str) -> Tokenized {
    let mut key = String::with_capacity(text.len());
    let mut tokens = Vec::new();
    let mut prev: Option<char> = None;
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        let sign_allowed = prev.is_none_or(|p| !p.is_alphanumeric() && p != SLOT);
        let sign = match c {
            '+' if sign_allowed => Some(Sign::Plus),
            '-' if sign_allowed => Some(Sign::Minus),
            _ => None,
        };
        let after_sign = if sign.is_some() {
            rest.get(1..).unwrap_or_default()
        } else {
            rest
        };
        if let Some((len, atom)) = atom(after_sign) {
            let sign_len = usize::from(sign.is_some());
            let written = rest.get(..sign_len + len).unwrap_or_default();
            tokens.push(Token {
                sign,
                atom,
                text: written.to_owned(),
            });
            key.push(SLOT);
            prev = Some(SLOT);
            rest = after_sign.get(len..).unwrap_or_default();
            continue;
        }
        key.push(c);
        prev = Some(c);
        rest = rest.get(c.len_utf8()..).unwrap_or_default();
    }
    Tokenized { key, tokens }
}

/// An atom at the start of `text`: its length in bytes and the atom.
fn atom(text: &str) -> Option<(usize, Atom)> {
    if text.starts_with(SLOT) {
        return Some((SLOT.len_utf8(), Atom::Slot));
    }
    if let Some(len) = number(text) {
        return Some((len, Atom::Number(text.get(..len)?.to_owned())));
    }
    range(text)
}

/// `digits` or `digits.digits` at the start of `text`: its length.
fn number(text: &str) -> Option<usize> {
    let digits = |s: &str| s.bytes().take_while(u8::is_ascii_digit).count();
    let whole = digits(text);
    if whole == 0 {
        return None;
    }
    let fraction = text
        .get(whole..)
        .and_then(|s| s.strip_prefix('.'))
        .map_or(0, digits);
    Some(if fraction > 0 {
        whole + 1 + fraction
    } else {
        whole
    })
}

/// A number with an optional `-`: its length.
fn signed_number(text: &str) -> Option<usize> {
    match text.strip_prefix('-') {
        Some(rest) => number(rest).map(|n| n + 1),
        None => number(text),
    }
}

/// `(a-b)` at the start of `text`.
fn range(text: &str) -> Option<(usize, Atom)> {
    let inner = text.strip_prefix('(')?;
    let min_len = signed_number(inner)?;
    let min = inner.get(..min_len)?;
    let after_min = inner.get(min_len..)?.strip_prefix('-')?;
    let max_len = signed_number(after_min)?;
    let max = after_min.get(..max_len)?;
    after_min.get(max_len..)?.strip_prefix(')')?;
    let len = 1 + min_len + 1 + max_len + 1;
    Some((
        len,
        Atom::Range {
            min: min.to_owned(),
            max: max.to_owned(),
        },
    ))
}

#[cfg(test)]
#[expect(
    clippy::indexing_slicing,
    reason = "tests index fixed synthetic data; a bad index should panic"
)]
mod tests {
    use super::*;

    fn key(text: &str) -> String {
        tokenize(text).key.replace(SLOT, "#")
    }

    #[test]
    fn normalize_should_trim_and_collapse_whitespace_and_keep_case() {
        assert_eq!(
            normalize("  +10 to\tmaximum   Life \n"),
            "+10 to maximum Life"
        );
    }

    #[test]
    fn normalize_should_rewrite_a_slot_character_in_line_text() {
        let line = format!("{SLOT}% increased");
        assert_eq!(tokenize(&normalize(&line)).tokens.len(), 0);
    }

    #[test]
    fn tokenize_should_read_signs_numbers_decimals_and_ranges() {
        let t = tokenize("+10 to -5 and 1.5 or (10-20) and -(10-5) ok");
        assert_eq!(t.key.replace(SLOT, "#"), "# to # and # or # and # ok");
        let atoms: Vec<_> = t.tokens.iter().map(|t| (t.sign, t.atom.clone())).collect();
        assert_eq!(
            atoms,
            vec![
                (Some(Sign::Plus), Atom::Number("10".to_owned())),
                (Some(Sign::Minus), Atom::Number("5".to_owned())),
                (None, Atom::Number("1.5".to_owned())),
                (
                    None,
                    Atom::Range {
                        min: "10".to_owned(),
                        max: "20".to_owned()
                    }
                ),
                (
                    Some(Sign::Minus),
                    Atom::Range {
                        min: "10".to_owned(),
                        max: "5".to_owned()
                    }
                ),
            ]
        );
        assert_eq!(t.tokens[4].text, "-(10-5)");
    }

    #[test]
    fn tokenize_should_read_a_range_that_crosses_zero() {
        let t = tokenize("(-10-10)% increased");
        assert_eq!(
            t.tokens[0].atom,
            Atom::Range {
                min: "-10".to_owned(),
                max: "10".to_owned()
            }
        );
        assert_eq!(
            tokenize("(-20--10)").tokens[0].atom,
            Atom::Range {
                min: "-20".to_owned(),
                max: "-10".to_owned()
            }
        );
    }

    #[test]
    fn tokenize_should_not_take_a_dash_after_a_word_or_number_as_a_sign() {
        assert_eq!(key("5-10"), "#-#");
        assert_eq!(key("Off-5"), "Off-#");
        assert_eq!(tokenize("Off-5").tokens[0].sign, None);
    }

    #[test]
    fn tokenize_should_keep_a_sign_without_an_atom_and_a_trailing_dot_as_text() {
        assert_eq!(key("+ X - Y"), "+ X - Y");
        assert_eq!(key("by 5."), "by #.");
        assert_eq!(key("(5-x)"), "(#-x)");
    }

    #[test]
    fn tokenize_should_read_slots_and_signs_before_them() {
        let skeleton = format!("+{SLOT} to {SLOT}% and Level 20");
        let t = tokenize(&skeleton);
        assert_eq!(key(&skeleton), "# to #% and Level #");
        assert_eq!(t.tokens[0].sign, Some(Sign::Plus));
        assert_eq!(t.tokens[1].atom, Atom::Slot);
        assert_eq!(t.tokens[2].atom, Atom::Number("20".to_owned()));
    }

    #[test]
    fn tokenize_should_give_line_and_skeleton_the_same_key() {
        let line = tokenize("Adds (5-7) to (10-12) Fire Damage");
        let skeleton = tokenize(&format!("Adds {SLOT} to {SLOT} Fire Damage"));
        assert_eq!(line.key, skeleton.key);
        assert_eq!(line.tokens.len(), 2);
    }
}
