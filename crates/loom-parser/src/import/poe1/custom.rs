//! Custom modifier lines that no stat description reads (ADR-015
//! Amendment 2; Phase 3 close-out).
//!
//! A config set's custom lines are the player's own text: PoB v2.67.2
//! parses each with its `ModParser` and applies the line only when the
//! whole of it parses (`Classes/ConfigTab.lua` 1115 to 1116: `mods and not
//! extra`); any other line does nothing. The resolver reads such a line
//! through the game's stat descriptions first ([`super::build`]). A line
//! with no template is then tried here, in order:
//!
//! 1. **A curated pattern** ([`PATTERNS`]): a reading of PoB's parser that
//!    has no game stat description ("20% more Damage"), as a
//!    [`TypedResolution::CustomPattern`] the calculation maps. Each
//!    pattern names the `ModParser` mod it stands for and is checked
//!    against PoB's own parse of every fixture line by a maintainer-run
//!    test (`docs/phase-3/custom-lines.md`).
//! 2. **A note** ([`is_note`]): a label or a comment ("Perfect Forest
//!    Warrior", "=== AG item mods ==="), as a
//!    [`TypedResolution::CustomNote`] with no effect.
//!
//! Anything else stays unresolved, a reason.

use loom_domain::TypedResolution;

/// One curated pattern: its name, the text around the number (lower case,
/// as PoB's parser compares), whether the number may carry a sign, and
/// whether a "less" form negates it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pattern {
    /// The name the calculation maps (`more-damage`).
    pub name: &'static str,
    /// The text after the number, for the "more" (or only) form.
    pub more: &'static str,
    /// The text after the number for the "less" form, if any.
    pub less: Option<&'static str>,
    /// Whether a leading `+` or `-` is allowed.
    pub signed: bool,
}

/// The curated patterns, sorted by name. PoB's mod for each
/// (`Modules/ModParser.lua` at v2.67.2, from its parse of the line):
/// - `base-critical-strike-chance`: "+#% to critical strike chance",
///   `CritChance` BASE (a base crit chance added to the skill's; the game's
///   own line is local to a weapon);
/// - `more-block-chance`: "#% more/less block chance", `BlockChance` MORE;
/// - `more-damage`: "#% more/less damage", `Damage` MORE;
/// - `more-spell-block-chance`: "#% more/less spell block chance",
///   `SpellBlockChance` MORE.
pub const PATTERNS: &[Pattern] = &[
    Pattern {
        name: "base-critical-strike-chance",
        more: "% to critical strike chance",
        less: None,
        signed: true,
    },
    Pattern {
        name: "more-block-chance",
        more: "% more block chance",
        less: Some("% less block chance"),
        signed: false,
    },
    Pattern {
        name: "more-damage",
        more: "% more damage",
        less: Some("% less damage"),
        signed: false,
    },
    Pattern {
        name: "more-spell-block-chance",
        more: "% more spell block chance",
        less: Some("% less spell block chance"),
        signed: false,
    },
];

/// The pattern `text` matches, with its value (negated for a "less"
/// form), as a typed resolution.
#[must_use]
pub fn pattern(text: &str) -> Option<TypedResolution> {
    let lower = text.to_ascii_lowercase();
    let (number, rest) = split_number(&lower)?;
    for p in PATTERNS {
        let signed_ok = p.signed || !(number.starts_with('+') || number.starts_with('-'));
        if !signed_ok {
            continue;
        }
        let negate = if rest == p.more {
            false
        } else if p.less == Some(rest) {
            true
        } else {
            continue;
        };
        let plain = number.trim_start_matches('+');
        let value = if negate {
            format!("-{plain}")
        } else {
            plain.to_owned()
        };
        return Some(TypedResolution::CustomPattern {
            pattern: p.name.to_owned(),
            value,
        });
    }
    None
}

/// The leading number of `text` (an optional sign, digits, an optional
/// decimal part) and the rest.
fn split_number(text: &str) -> Option<(&str, &str)> {
    let bytes = text.as_bytes();
    let mut i = 0;
    if matches!(bytes.first(), Some(b'+' | b'-')) {
        i = 1;
    }
    let digits = i;
    while bytes.get(i).is_some_and(u8::is_ascii_digit) {
        i += 1;
    }
    if i == digits {
        return None;
    }
    if bytes.get(i) == Some(&b'.') {
        let point = i;
        i += 1;
        while bytes.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        if i == point + 1 {
            return None;
        }
    }
    Some(text.split_at(i))
}

/// The markers a comment starts with in the players' blocks.
pub const COMMENT_MARKERS: &[&str] = &["//", "--", "=", "~"];

/// Words of a modifier's sentence: PoB applies numberless lines such as
/// "Cannot be Stunned", "You have Onslaught", or "Enemies are Intimidated"
/// (`ModParser.lua`'s flag and special forms), which no stat description
/// may read. A numberless line with one of these words is never a note: it
/// stays unresolved (a reason), so a line PoB applies cannot be hidden as a
/// label. Compared per word, without case.
pub const MODIFIER_WORDS: &[&str] = &[
    "always", "are", "can", "cannot", "deal", "deals", "enemies", "enemy", "gain", "gains",
    "grant", "grants", "has", "have", "immune", "is", "nearby", "never", "per", "recently", "take",
    "takes", "when", "while", "you", "your",
];

/// Whether `text`, a custom line no description or pattern reads, is a
/// note: it starts with a comment marker, or it holds no digit and none of
/// [`MODIFIER_WORDS`] (a label: a skill, a spectre, a heading). A line
/// naming a keystone is not one: PoB reads a keystone's name as the
/// keystone (`keystones` tells the caller's names, compared without case).
#[must_use]
pub fn is_note(text: &str, is_keystone: impl Fn(&str) -> bool) -> bool {
    if is_keystone(text) {
        return false;
    }
    if COMMENT_MARKERS.iter().any(|m| text.starts_with(m)) {
        return true;
    }
    let sentence = text
        .split(|c: char| !c.is_ascii_alphabetic() && c != '\'')
        .any(|w| MODIFIER_WORDS.iter().any(|m| w.eq_ignore_ascii_case(m)));
    !sentence && !text.bytes().any(|b| b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(name: &str, value: &str) -> Option<TypedResolution> {
        Some(TypedResolution::CustomPattern {
            pattern: name.to_owned(),
            value: value.to_owned(),
        })
    }

    #[test]
    fn the_patterns_should_be_sorted() {
        let names: Vec<&str> = PATTERNS.iter().map(|p| p.name).collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(names, sorted);
    }

    #[test]
    fn a_pattern_should_read_the_value_and_its_less_form() {
        assert_eq!(pattern("20% more damage"), p("more-damage", "20"));
        assert_eq!(pattern("20% More Damage"), p("more-damage", "20"));
        assert_eq!(pattern("15% less damage"), p("more-damage", "-15"));
        assert_eq!(
            pattern("24% more block chance"),
            p("more-block-chance", "24")
        );
        assert_eq!(
            pattern("24% more spell block chance"),
            p("more-spell-block-chance", "24")
        );
        assert_eq!(
            pattern("+1.5% to critical strike chance"),
            p("base-critical-strike-chance", "1.5")
        );
        assert_eq!(
            pattern("-1% to critical strike chance"),
            p("base-critical-strike-chance", "-1")
        );
    }

    #[test]
    fn a_line_beside_a_pattern_should_not_match() {
        assert_eq!(pattern("+20% more damage"), None);
        assert_eq!(pattern("20% more damage with hits"), None);
        assert_eq!(pattern("20%more damage"), None);
        assert_eq!(pattern("1.% more damage"), None);
        assert_eq!(pattern("more damage"), None);
    }

    #[test]
    fn a_label_or_a_comment_should_be_a_note() {
        let none = |_: &str| false;
        assert!(is_note("Perfect Forest Warrior", none));
        assert!(is_note("Ice Nova Buffs", none));
        assert!(is_note("=== AG item mods ===", none));
        assert!(is_note("--Follow Through (above 5 chains)", none));
        assert!(is_note("~Add 30 Rage to the Config", none));
        assert!(!is_note("Gain 30 Rage", none));
        assert!(!is_note("-5% to Fire Resistance", none));
        assert!(!is_note("Iron Reflexes", |t| t.eq_ignore_ascii_case("iron reflexes")));
        // Numberless lines PoB applies as flags are sentences, never notes.
        assert!(!is_note("Cannot be Stunned", none));
        assert!(!is_note("You have Onslaught", none));
        assert!(!is_note("Enemies are Intimidated", none));
        assert!(!is_note("Your hits can't be Evaded", none));
        assert!(!is_note("Immune to Freeze", none));
        // A word inside another is not one ("Primal", "Cane").
        assert!(is_note("Primal Crushclaw Average Damage", none));
        assert!(is_note("CANE", none));
    }
}
