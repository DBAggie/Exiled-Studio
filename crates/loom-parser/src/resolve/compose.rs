//! Lines the client composes around a described line (Spike B, PR B5c).
//!
//! Some item lines are not one stat description's text: the client wraps a
//! described line in a `ClientStrings` text and the wrapper stands for a
//! stat of the same mod. Example (PoE1 26530219): eldritch implicits with
//! `local_influence_mod_requires_unique_monster_presence` show as
//! `While a Unique Enemy is in your Presence, {0}` (`ClientStrings` id
//! `InfluenceStatConditionPresenceUniqueMonster`) around the line of the
//! mod's other stat. A [`Composition`] is that wrapper with its stat;
//! [`Index::resolve_lines_composed`] resolves the wrapped remainder and adds
//! the stat to every reading.
//!
//! A line is read as composed only when its whole text has no indexed key
//! (so a description whose own text starts like a wrapper always wins) and
//! its remainder resolves (resolved or ambiguous); otherwise it keeps its
//! plain outcome. Nothing is dropped.

use loom_data::stat_desc::StatValue;

use super::token::normalize;
use super::{Candidate, Index, Outcome, RawValue, Resolution, ResolvedStat, Unresolved};

/// A `ClientStrings` wrapper with the stat it stands for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Composition {
    /// The `ClientStrings` id.
    pub id: String,
    /// Normalized text before `{0}`.
    prefix: String,
    /// Normalized text after `{0}`.
    suffix: String,
    /// The stat the wrapper stands for.
    pub stat: String,
    /// Its raw value.
    pub value: i64,
}

impl Composition {
    /// A wrapper from a `ClientStrings` text with exactly one placeholder,
    /// `{0}`, and some text around it; `None` otherwise.
    #[must_use]
    pub fn new(id: &str, text: &str, stat: &str, value: i64) -> Option<Self> {
        let (prefix, suffix) = text.split_once("{0}")?;
        if prefix.contains('{') || suffix.contains('{') {
            return None;
        }
        let (prefix, suffix) = (normalize(prefix), normalize(suffix));
        if prefix.is_empty() && suffix.is_empty() {
            return None;
        }
        Some(Self {
            id: id.to_owned(),
            prefix,
            suffix,
            stat: stat.to_owned(),
            value,
        })
    }

    /// The wrapped remainder of a normalized line, if the line has this
    /// wrapper and a non-empty remainder.
    #[must_use]
    pub fn strip<'l>(&self, line: &'l str) -> Option<&'l str> {
        let rest = line.strip_prefix(self.prefix.as_str())?;
        let rest = rest.strip_suffix(self.suffix.as_str())?.trim();
        (!rest.is_empty()).then_some(rest)
    }

    fn stat(&self) -> ResolvedStat {
        ResolvedStat {
            stat: self.stat.clone(),
            value: RawValue::Exact(StatValue::exact(self.value)),
        }
    }

    fn add_to(&self, c: &mut Candidate) {
        c.stats.push(self.stat());
        for a in &mut c.alternatives {
            a.stats.push(self.stat());
        }
    }
}

/// A resolution, and the index of the [`Composition`] it was read through.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Composed {
    /// The resolution (its `line` is the whole input line).
    pub resolution: Resolution,
    /// The composition, if the line was read through one.
    pub composition: Option<usize>,
}

impl Index<'_> {
    /// [`Index::resolve_lines`], with lines that match a wrapper of
    /// `compositions` read through it (module docs). Every input line is
    /// covered by exactly one resolution; a composed line covers one line.
    #[must_use]
    pub fn resolve_lines_composed(
        &self,
        lines: &[&str],
        compositions: &[Composition],
    ) -> Vec<Composed> {
        let mut out = Vec::new();
        let mut run: Vec<&str> = Vec::new();
        let flush = |run: &mut Vec<&str>, out: &mut Vec<Composed>| {
            if !run.is_empty() {
                out.extend(
                    self.resolve_lines(run)
                        .into_iter()
                        .map(|resolution| Composed {
                            resolution,
                            composition: None,
                        }),
                );
                run.clear();
            }
        };
        for line in lines {
            match self.composed(line, compositions) {
                Some(c) => {
                    flush(&mut run, &mut out);
                    out.push(c);
                }
                None => run.push(line),
            }
        }
        flush(&mut run, &mut out);
        out
    }

    /// The line read through the first composition whose remainder
    /// resolves, when the whole line has no indexed key.
    fn composed(&self, line: &str, compositions: &[Composition]) -> Option<Composed> {
        let whole = self.resolve(line);
        if !matches!(whole.outcome, Outcome::Unresolved(Unresolved::NoTemplate)) {
            return None;
        }
        compositions.iter().enumerate().find_map(|(n, comp)| {
            let rest = comp.strip(&whole.line)?;
            let mut inner = self.resolve(rest);
            match &mut inner.outcome {
                Outcome::Resolved(c) => comp.add_to(c),
                Outcome::Ambiguous(cs) => cs.iter_mut().for_each(|c| comp.add_to(c)),
                Outcome::Unresolved(_) => return None,
            }
            inner.shadowed.iter_mut().for_each(|c| comp.add_to(c));
            inner.line.clone_from(&whole.line);
            Some(Composed {
                resolution: inner,
                composition: Some(n),
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_should_need_one_placeholder_and_some_text() {
        assert!(Composition::new("a", "While x, {0}", "s", 1).is_some());
        assert!(Composition::new("a", "{0}", "s", 1).is_none());
        assert!(Composition::new("a", "no placeholder", "s", 1).is_none());
        assert!(Composition::new("a", "{0} and {1}", "s", 1).is_none());
    }

    #[test]
    fn strip_should_return_the_wrapped_remainder() {
        let c =
            Composition::new("a", "While a Unique Enemy is in your Presence, {0}", "s", 1).unwrap();
        assert_eq!(
            c.strip("While a Unique Enemy is in your Presence, 10% increased Damage"),
            Some("10% increased Damage")
        );
        assert_eq!(c.strip("While a Unique Enemy is in your Presence,"), None);
        assert_eq!(c.strip("10% increased Damage"), None);
    }
}
