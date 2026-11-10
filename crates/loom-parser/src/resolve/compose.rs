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
//! A line is read as composed only when [`Index::resolve_lines`] left it as
//! one line with no indexed key (`NoTemplate`): a description whose own
//! text starts like a wrapper always wins, a whole line that has a key but
//! fails verification is not composed, and a line of a multi-line template
//! is never taken out of it. The remainder must resolve (resolved or
//! ambiguous); otherwise the line keeps its plain outcome. Nothing is
//! dropped. Wrappers are tried in slice order; callers keep their texts
//! disjoint (no wrapper's line also matches another).

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

    /// Adds the wrapper's stat to a reading, unless the reading already has
    /// it.
    fn add_to(&self, c: &mut Candidate) {
        let add = |stats: &mut Vec<ResolvedStat>| {
            if stats.iter().all(|s| s.stat != self.stat) {
                stats.push(self.stat());
            }
        };
        add(&mut c.stats);
        for a in &mut c.alternatives {
            add(&mut a.stats);
        }
    }
}

/// A resolution, and the index of the [`Composition`] it was read through.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Composed {
    /// The resolution (its `line` is the whole input line).
    pub resolution: Resolution,
    /// Index into the `compositions` slice passed in, if the line was read
    /// through one.
    pub composition: Option<usize>,
}

impl Index<'_> {
    /// [`Index::resolve_lines`], then every single-line `NoTemplate`
    /// resolution that matches a wrapper of `compositions` is read through
    /// it (module docs). Runs are resolved as they are, so a multi-line
    /// template is never split; each input line is still covered by exactly
    /// one resolution.
    #[must_use]
    pub fn resolve_lines_composed(
        &self,
        lines: &[&str],
        compositions: &[Composition],
    ) -> Vec<Composed> {
        self.resolve_lines(lines)
            .into_iter()
            .map(|resolution| {
                let plain = resolution.lines == 1
                    && matches!(
                        resolution.outcome,
                        Outcome::Unresolved(Unresolved::NoTemplate)
                    );
                plain
                    .then(|| self.composed(&resolution, compositions))
                    .flatten()
                    .unwrap_or(Composed {
                        resolution,
                        composition: None,
                    })
            })
            .collect()
    }

    /// The line read through the first composition whose remainder
    /// resolves (resolved or ambiguous).
    fn composed(&self, whole: &Resolution, compositions: &[Composition]) -> Option<Composed> {
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
