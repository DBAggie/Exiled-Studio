//! Granted-skill lines (Phase 2 plan, step 2d; lifted from Spike B's
//! `grafts.rs`, PR B5g; ADR-015 Amendment 1, kind `granted-skill`).
//!
//! Some bases grant a skill, and their implicit shows it through a
//! client-text wrapper with the level in `{1}` and the skill's name in
//! `{0}` ([`TextRules::granted_skill_wrapper`]). No stat stands for the
//! line: the skill comes from the base's data row and the level range from
//! the game data. A line resolves when the item's base grants a skill, the
//! line matches the wrapper with a whole-number level in the range, and
//! the name equals the base's skill. A range level or another name does
//! not resolve.
//!
//! [`TextRules::granted_skill_wrapper`]: super::TextRules::granted_skill_wrapper

use std::collections::BTreeMap;

/// The granted skill of one base.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Graft {
    /// The data row that grants the skill.
    pub id: String,
    /// The granted skill gem's base item name.
    pub skill: String,
}

/// The wrapper split around `{1}` (level) and `{0}` (name).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Wrapper {
    prefix: String,
    middle: String,
    suffix: String,
}

impl Wrapper {
    /// `<prefix>{1}<middle>{0}<suffix>`; `None` for any other shape.
    fn new(text: &str) -> Option<Self> {
        let (prefix, rest) = text.split_once("{1}")?;
        let (middle, suffix) = rest.split_once("{0}")?;
        if [prefix, middle, suffix].iter().any(|p| p.contains('{')) {
            return None;
        }
        Some(Self {
            prefix: prefix.to_owned(),
            middle: middle.to_owned(),
            suffix: suffix.to_owned(),
        })
    }

    /// `(level, name)` of a line, when the level is a whole number.
    fn read<'l>(&self, line: &'l str) -> Option<(u32, &'l str)> {
        let rest = line.trim().strip_prefix(self.prefix.as_str())?;
        let (level, name) = rest.split_once(self.middle.as_str())?;
        let name = name.strip_suffix(self.suffix.as_str())?;
        if level.is_empty() || !level.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let level = level.parse().ok()?;
        (!name.is_empty()).then_some((level, name))
    }
}

/// Bases that grant a skill, the wrapper, and the allowed levels.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Grafts {
    by_base: BTreeMap<String, Graft>,
    wrapper: Option<Wrapper>,
    levels: Option<(u32, u32)>,
    conflicts: Vec<String>,
}

/// A resolved granted-skill line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantedSkill {
    /// The base's graft.
    pub graft: Graft,
    /// The skill level shown.
    pub level: u32,
}

impl Grafts {
    /// From the game data:
    /// - `bases`: `(base name, graft)` for each base row that grants a
    ///   skill; a base name whose rows name different grafts is dropped
    ///   and listed in [`Grafts::conflicts`]; the graft names the skill by
    ///   its gem's base name (the domain's gem ID is looked up from the
    ///   data when the line is written to the build, plan step 6);
    /// - `wrapper`: the wrapper text, if the data has it (a text of
    ///   another shape is not read);
    /// - `levels`: every allowed skill level; a level between the lowest
    ///   and the highest is allowed even if no row names it.
    #[must_use]
    pub fn new(
        bases: impl IntoIterator<Item = (String, Graft)>,
        wrapper: Option<&str>,
        levels: &[u32],
    ) -> Self {
        let mut out = Self::default();
        for (name, graft) in bases {
            match out.by_base.get(&name) {
                Some(old) if *old != graft => out.conflicts.push(name),
                _ => {
                    out.by_base.insert(name, graft);
                }
            }
        }
        out.conflicts.sort();
        out.conflicts.dedup();
        for name in &out.conflicts {
            out.by_base.remove(name);
        }
        out.wrapper = wrapper.and_then(Wrapper::new);
        out.levels = levels
            .iter()
            .min()
            .copied()
            .zip(levels.iter().max().copied());
        out
    }

    /// Whether the wrapper was read.
    #[must_use]
    pub fn has_wrapper(&self) -> bool {
        self.wrapper.is_some()
    }

    /// The allowed level range.
    #[must_use]
    pub fn levels(&self) -> Option<(u32, u32)> {
        self.levels
    }

    /// Base names dropped because their rows name different grafts.
    #[must_use]
    pub fn conflicts(&self) -> &[String] {
        &self.conflicts
    }

    /// Number of bases that grant a skill.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_base.len()
    }

    /// Whether no base grants a skill.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_base.is_empty()
    }

    /// The granted skill a line of an item with base `base` shows (module
    /// docs).
    #[must_use]
    pub fn resolve(&self, base: &str, line: &str) -> Option<GrantedSkill> {
        let graft = self.by_base.get(base)?;
        let (level, name) = self.wrapper.as_ref()?.read(line)?;
        let (lo, hi) = self.levels?;
        (name == graft.skill && (lo..=hi).contains(&level)).then(|| GrantedSkill {
            graft: graft.clone(),
            level,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grafts() -> Grafts {
        Grafts::new(
            [(
                "Sky Graft".to_owned(),
                Graft {
                    id: "Graft1".to_owned(),
                    skill: "Sky Bolt".to_owned(),
                },
            )],
            Some("Uses level {1} {0}"),
            &[1, 30],
        )
    }

    #[test]
    fn resolve_should_read_the_level_and_the_base_skill() {
        assert_eq!(
            grafts()
                .resolve("Sky Graft", "Uses level 20 Sky Bolt")
                .map(|g| (g.graft.id, g.level)),
            Some(("Graft1".to_owned(), 20))
        );
    }

    #[test]
    fn resolve_should_refuse_another_skill_a_range_or_a_level_out_of_range() {
        let g = grafts();
        assert_eq!(
            [
                g.resolve("Sky Graft", "Uses level 20 Sea Bolt"),
                g.resolve("Sky Graft", "Uses level (1-30) Sky Bolt"),
                g.resolve("Sky Graft", "Uses level 31 Sky Bolt"),
                g.resolve("Other", "Uses level 20 Sky Bolt"),
            ],
            [None, None, None, None]
        );
    }

    #[test]
    fn new_should_drop_a_base_whose_rows_name_different_grafts() {
        let graft = |id: &str| Graft {
            id: id.to_owned(),
            skill: "S".to_owned(),
        };
        let g = Grafts::new(
            [("B".to_owned(), graft("1")), ("B".to_owned(), graft("2"))],
            Some("{1} {0}"),
            &[1],
        );
        assert_eq!((g.len(), g.conflicts()), (0, &["B".to_owned()][..]));
    }

    #[test]
    fn new_should_not_read_a_wrapper_of_another_shape() {
        assert!(!Grafts::new([], Some("Uses {0} at {1}"), &[]).has_wrapper());
    }
}
