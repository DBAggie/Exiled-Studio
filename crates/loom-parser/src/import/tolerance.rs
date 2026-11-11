//! Import tolerances (Phase 2 plan, step 2d; lifted from Spike B's
//! `resolve_corpus.rs`, PR B5e; ADR-015 Amendment 1, `adapted` lines).
//!
//! A tolerance is tried only when exact matching finds no template, and a
//! line it resolves carries the tolerance's name ([`loom_domain::Adapted`]):
//! - **case-folded:** the text differs from the game's in letter case only
//!   (hand-edited text); refused when the text folds to more than one game
//!   key ([`Index::resolve_case_folded`]);
//! - **split-section:** an import format put the two lines of one
//!   description in two sections; the line and an adjacent line of another
//!   section resolve together as one two-line description.

use loom_domain::{ItemBody, LineKind, LineResolution, ModifierLine};

use crate::resolve::{Index, Outcome, Resolution, Unresolved};

/// The case-folded reading of one line, when exact matching gave
/// `resolution` for it alone with no template and the line was not read
/// through a composed wrapper (`composed`); `None` otherwise, or when the
/// folded text does not resolve.
#[must_use]
pub fn case_folded(
    index: &Index<'_>,
    text: &str,
    resolution: &Resolution,
    composed: bool,
) -> Option<Resolution> {
    if composed
        || resolution.lines != 1
        || !matches!(
            resolution.outcome,
            Outcome::Unresolved(Unresolved::NoTemplate)
        )
    {
        return None;
    }
    index
        .resolve_case_folded(text)
        .filter(|r| !matches!(r.outcome, Outcome::Unresolved(_)))
}

/// A modifier line resolution has not decided yet (the pipeline's input).
fn pending(body: &ItemBody, n: usize) -> Option<&ModifierLine> {
    match body.lines.get(n).map(|l| &l.kind) {
        Some(LineKind::Modifier(m)) if m.resolution == LineResolution::Pending => Some(m),
        _ => None,
    }
}

/// The split-section reading of line `n` of `body` (a pending modifier
/// line that has no template): the adjacent pending modifier line of
/// another section, and the two-line description the pair resolves to.
/// The line before is tried first; when both neighbours would complete a
/// pair, the earlier one wins.
///
/// Pass the body as read, before any resolution is written into it: a
/// neighbour counts only while it is [`LineResolution::Pending`], as the
/// import adapter left it (excluded lines never pair).
#[must_use]
pub fn split_section(index: &Index<'_>, body: &ItemBody, n: usize) -> Option<(usize, Resolution)> {
    let this = pending(body, n)?;
    let neighbours = [n.checked_sub(1), n.checked_add(1)];
    for other in neighbours.into_iter().flatten() {
        let Some(m) = pending(body, other).filter(|m| m.section != this.section) else {
            continue;
        };
        let pair = if other < n {
            [m.text.as_str(), this.text.as_str()]
        } else {
            [this.text.as_str(), m.text.as_str()]
        };
        let joined = index
            .resolve_lines(&pair)
            .into_iter()
            .next()
            .filter(|r| r.lines == 2 && matches!(r.outcome, Outcome::Resolved(_)));
        if let Some(r) = joined {
            return Some((other, r));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use loom_data::stat_desc::{
        Condition, Description, StatDescFile, StatDescriptions, Template, Variant,
    };
    use loom_domain::{ItemLine, ModSection, ModifierLine};

    use super::*;

    const ANY: Condition = Condition::Range {
        min: None,
        max: None,
    };

    fn desc(stats: &[&str], text: &str) -> Description {
        let n = stats.len();
        Description {
            name: None,
            line: 1,
            stats: stats.iter().map(|s| (*s).to_owned()).collect(),
            off_hand_stats: None,
            variants: vec![Variant {
                conditions: vec![ANY; n],
                gem_quality: false,
                table_only: false,
                unknown_flags: Vec::new(),
                handlers: Vec::new(),
                line: 1,
                text: text.to_owned(),
                template: Template::parse(text, n),
            }],
        }
    }

    fn set() -> StatDescriptions {
        let mut set = StatDescriptions::new();
        set.insert(StatDescFile {
            source: "a.txt".to_owned(),
            includes: Vec::new(),
            descriptions: vec![
                desc(&["life"], "+{0} to maximum Life"),
                desc(
                    &["first", "second"],
                    "{0} first half
{1} second half",
                ),
            ],
            no_description: Vec::new(),
        });
        set
    }

    fn modifier(text: &str, section: ModSection) -> ItemLine {
        ItemLine {
            source: text.to_owned(),
            kind: LineKind::Modifier(ModifierLine {
                text: text.to_owned(),
                prefixes: Vec::new(),
                section,
                selected: true,
                resolution: LineResolution::Pending,
            }),
        }
    }

    fn body(lines: Vec<ItemLine>) -> ItemBody {
        ItemBody {
            lines,
            ..ItemBody::default()
        }
    }

    #[test]
    fn case_folded_should_read_text_that_differs_in_case_only() {
        let set = set();
        let scope = set.scope("a.txt").unwrap();
        let index = Index::build(&scope);
        let line = "+12 to Maximum Life";
        let exact = index.resolve(line);
        let folded = case_folded(&index, line, &exact, false);
        assert!(matches!(
            folded.map(|r| r.outcome),
            Some(Outcome::Resolved(_))
        ));
    }

    #[test]
    fn case_folded_should_not_run_for_a_composed_line() {
        let set = set();
        let scope = set.scope("a.txt").unwrap();
        let index = Index::build(&scope);
        let line = "+12 to Maximum Life";
        let exact = index.resolve(line);
        assert_eq!(case_folded(&index, line, &exact, true), None);
    }

    #[test]
    fn split_section_should_pair_a_line_with_its_neighbour_of_another_section() {
        let set = set();
        let scope = set.scope("a.txt").unwrap();
        let index = Index::build(&scope);
        let b = body(vec![
            modifier("3 first half", ModSection::Implicit),
            modifier("4 second half", ModSection::Explicit),
        ]);
        assert_eq!(
            split_section(&index, &b, 1).map(|(other, r)| (other, r.lines)),
            Some((0, 2))
        );
    }

    #[test]
    fn split_section_should_prefer_the_earlier_neighbour() {
        let set = set();
        let scope = set.scope("a.txt").unwrap();
        let index = Index::build(&scope);
        // Line 1 pairs with line 0 (before) and with line 2 (after).
        let b = body(vec![
            modifier("3 first half", ModSection::Implicit),
            modifier("4 second half", ModSection::Explicit),
            modifier("5 first half", ModSection::Enchant),
        ]);
        assert_eq!(
            split_section(&index, &b, 1).map(|(other, _)| other),
            Some(0)
        );
    }

    #[test]
    fn split_section_should_not_pair_with_an_excluded_neighbour() {
        let set = set();
        let scope = set.scope("a.txt").unwrap();
        let index = Index::build(&scope);
        let mut first = modifier("3 first half", ModSection::Implicit);
        if let LineKind::Modifier(m) = &mut first.kind {
            m.resolution = LineResolution::Excluded(loom_domain::Exclusion::Custom);
        }
        let b = body(vec![first, modifier("4 second half", ModSection::Explicit)]);
        assert_eq!(split_section(&index, &b, 1), None);
    }

    #[test]
    fn case_folded_should_not_run_for_a_line_that_resolved() {
        let set = set();
        let scope = set.scope("a.txt").unwrap();
        let index = Index::build(&scope);
        let line = "+12 to maximum Life";
        let exact = index.resolve(line);
        assert_eq!(case_folded(&index, line, &exact, false), None);
    }

    #[test]
    fn split_section_should_not_pair_lines_of_one_section() {
        let set = set();
        let scope = set.scope("a.txt").unwrap();
        let index = Index::build(&scope);
        let b = body(vec![
            modifier("3 first half", ModSection::Explicit),
            modifier("4 second half", ModSection::Explicit),
        ]);
        assert_eq!(split_section(&index, &b, 1), None);
    }
}
