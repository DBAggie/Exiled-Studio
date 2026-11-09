//! A set of stat-description files and the include chain of one of them.
//!
//! **Precedence (hypothesis, PoB-observed, not game evidence):** a file's
//! own descriptions win over its includes, and nearer includes win over
//! farther ones. `v2.67.2:src/Modules/StatDescriber.lua` `getScope` copies
//! the parent's `scopeList` and puts the file first
//! (`t_insert(scope.scopeList, 1, scope)`); the first scope that has a
//! stat describes it. PoB knows one parent per file; here a file's includes
//! are walked depth-first in written order (every file in both installs
//! has at most one include, 2026-11-09). Spike B counts the lines that
//! depend on this rule (plan step 2b).
//!
//! **Duplicates (hypothesis):** when one file describes a stat in more
//! than one description, the first in file order wins, as PoB's export
//! keeps one description index per stat. Step 2b counts affected lines.

use std::collections::{BTreeMap, BTreeSet};

use super::{Hand, StatDescFile};

/// Files by lowercase index path. Include paths are matched without case,
/// as the extractor does.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StatDescriptions {
    files: BTreeMap<String, StatDescFile>,
}

/// Why a scope cannot be built.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ScopeError {
    /// The root or an included file is not in the set.
    #[error("{0} is not loaded")]
    Missing(String),
    /// A file includes itself through its chain.
    #[error("include cycle through {0}")]
    Cycle(String),
}

impl ScopeError {
    /// Stable error code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Missing(_) => "DATA_STATDESC_SCOPE_MISSING",
            Self::Cycle(_) => "DATA_STATDESC_SCOPE_CYCLE",
        }
    }
}

impl StatDescriptions {
    /// An empty set.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            files: BTreeMap::new(),
        }
    }

    /// Adds a file under its source path; returns the file it replaced.
    pub fn insert(&mut self, file: StatDescFile) -> Option<StatDescFile> {
        self.files.insert(file.source.to_ascii_lowercase(), file)
    }

    /// The file at `path` (any case).
    #[must_use]
    pub fn get(&self, path: &str) -> Option<&StatDescFile> {
        self.files.get(&path.to_ascii_lowercase())
    }

    /// Files by lowercase path.
    pub fn files(&self) -> impl Iterator<Item = (&str, &StatDescFile)> {
        self.files.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// The scope of `root`: the file, then its includes depth-first.
    ///
    /// # Errors
    /// [`ScopeError`] if a file of the chain is missing or the chain has a
    /// cycle.
    pub fn scope(&self, root: &str) -> Result<Scope<'_>, ScopeError> {
        let mut chain = Vec::new();
        let mut seen = BTreeSet::new();
        self.walk(root, &mut chain, &mut seen, &mut Vec::new())?;
        Ok(Scope::new(chain))
    }

    fn walk<'a>(
        &'a self,
        path: &str,
        chain: &mut Vec<&'a StatDescFile>,
        seen: &mut BTreeSet<String>,
        stack: &mut Vec<String>,
    ) -> Result<(), ScopeError> {
        let key = path.to_ascii_lowercase();
        if stack.contains(&key) {
            return Err(ScopeError::Cycle(path.to_owned()));
        }
        let file = self
            .files
            .get(&key)
            .ok_or_else(|| ScopeError::Missing(path.to_owned()))?;
        if !seen.insert(key.clone()) {
            return Ok(());
        }
        chain.push(file);
        stack.push(key);
        for include in &file.includes {
            self.walk(include, chain, seen, stack)?;
        }
        stack.pop();
        Ok(())
    }
}

/// Where a stat is described in a scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DescRef {
    /// Position of the file in the chain (0 = the root file).
    pub depth: usize,
    /// Index of the description in its file.
    pub index: usize,
    /// Which stat list of the description holds the stat.
    pub hand: Hand,
}

/// The include chain of one file with a stat index (first file wins).
#[derive(Debug, Clone)]
pub struct Scope<'a> {
    chain: Vec<&'a StatDescFile>,
    described: BTreeMap<&'a str, DescRef>,
    no_description: BTreeSet<&'a str>,
}

impl<'a> Scope<'a> {
    fn new(chain: Vec<&'a StatDescFile>) -> Self {
        let mut described = BTreeMap::new();
        let mut no_description = BTreeSet::new();
        for (depth, file) in chain.iter().enumerate() {
            for (index, d) in file.descriptions.iter().enumerate() {
                for hand in [Hand::Main, Hand::Off] {
                    for stat in d.stats_of(hand).unwrap_or_default() {
                        described
                            .entry(stat.as_str())
                            .or_insert(DescRef { depth, index, hand });
                    }
                }
            }
            no_description.extend(file.no_description.iter().map(String::as_str));
        }
        Self {
            chain,
            described,
            no_description,
        }
    }

    /// Files in precedence order (root first).
    #[must_use]
    pub fn chain(&self) -> &[&'a StatDescFile] {
        &self.chain
    }

    /// Where `stat` is described, if anywhere in the chain.
    #[must_use]
    pub fn find(&self, stat: &str) -> Option<DescRef> {
        self.described.get(stat).copied()
    }

    /// The file and description a [`DescRef`] points at.
    #[must_use]
    pub fn description(&self, at: DescRef) -> Option<(&'a StatDescFile, &'a super::Description)> {
        let file = *self.chain.get(at.depth)?;
        Some((file, file.descriptions.get(at.index)?))
    }

    /// Whether a file of the chain declares `stat` as having no description.
    #[must_use]
    pub fn declares_no_description(&self, stat: &str) -> bool {
        self.no_description.contains(stat)
    }

    /// Every description of the chain with where it is, root file first
    /// (main hand only; see [`Hand`] for handed descriptions).
    pub fn descriptions(&self) -> impl Iterator<Item = (DescRef, &'a super::Description)> + '_ {
        self.chain.iter().enumerate().flat_map(|(depth, file)| {
            file.descriptions.iter().enumerate().map(move |(index, d)| {
                (
                    DescRef {
                        depth,
                        index,
                        hand: Hand::Main,
                    },
                    d,
                )
            })
        })
    }

    /// Number of described stats.
    #[must_use]
    pub fn described_stats(&self) -> usize {
        self.described.len()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::stat_desc::{Condition, Description, Template, Variant};

    pub(crate) fn variant(stats: usize, text: &str) -> Variant {
        Variant {
            conditions: vec![
                Condition::Range {
                    min: None,
                    max: None
                };
                stats
            ],
            gem_quality: false,
            table_only: false,
            unknown_flags: Vec::new(),
            handlers: Vec::new(),
            line: 1,
            text: text.to_owned(),
            template: Template::parse(text, stats),
        }
    }

    pub(crate) fn desc(stats: &[&str], text: &str) -> Description {
        Description {
            name: None,
            line: 1,
            stats: stats.iter().map(|s| (*s).to_owned()).collect(),
            off_hand_stats: None,
            variants: vec![variant(stats.len(), text)],
        }
    }

    pub(crate) fn file(
        source: &str,
        includes: &[&str],
        descriptions: Vec<Description>,
    ) -> StatDescFile {
        StatDescFile {
            source: source.to_owned(),
            includes: includes.iter().map(|s| (*s).to_owned()).collect(),
            descriptions,
            no_description: Vec::new(),
        }
    }

    fn set(files: Vec<StatDescFile>) -> StatDescriptions {
        let mut set = StatDescriptions::new();
        for f in files {
            set.insert(f);
        }
        set
    }

    #[test]
    fn scope_should_prefer_the_own_file_then_nearer_includes() {
        let set = set(vec![
            file(
                "dir/root.txt",
                &["Dir/Mid.txt"],
                vec![desc(&["a"], "root a")],
            ),
            file(
                "dir/mid.txt",
                &["DIR/BASE.txt"],
                vec![desc(&["a", "b"], "mid")],
            ),
            file(
                "dir/base.txt",
                &[],
                vec![desc(&["b"], "base b"), desc(&["c"], "base c")],
            ),
        ]);
        let scope = set.scope("Dir/Root.txt").unwrap();
        let depth = |s| scope.find(s).map(|r| r.depth);
        assert_eq!(
            (depth("a"), depth("b"), depth("c"), depth("d")),
            (Some(0), Some(1), Some(2), None)
        );
    }

    #[test]
    fn scope_should_use_the_first_description_of_a_stat_within_one_file() {
        let set = set(vec![file(
            "a.txt",
            &[],
            vec![desc(&["s"], "first"), desc(&["s", "t"], "second")],
        )]);
        let scope = set.scope("a.txt").unwrap();
        let index = |s| scope.find(s).map(|r| r.index);
        assert_eq!((index("s"), index("t")), (Some(0), Some(1)));
        assert_eq!(scope.descriptions().count(), 2);
    }

    #[test]
    fn scope_should_report_a_missing_include() {
        let set = set(vec![file("a.txt", &["b.txt"], Vec::new())]);
        assert_eq!(
            set.scope("a.txt").unwrap_err(),
            ScopeError::Missing("b.txt".to_owned())
        );
    }

    #[test]
    fn scope_should_report_a_cycle() {
        let set = set(vec![
            file("a.txt", &["b.txt"], Vec::new()),
            file("b.txt", &["A.txt"], Vec::new()),
        ]);
        assert_eq!(
            set.scope("a.txt").unwrap_err().code(),
            "DATA_STATDESC_SCOPE_CYCLE"
        );
    }

    #[test]
    fn scope_should_index_off_hand_stats_of_handed_descriptions() {
        let mut d = desc(&["main_min", "main_max"], "{0} to {1}");
        d.off_hand_stats = Some(vec!["off_min".to_owned(), "off_max".to_owned()]);
        let set = set(vec![file("a.txt", &[], vec![d])]);
        let scope = set.scope("a.txt").unwrap();
        assert_eq!(scope.find("off_max").map(|r| r.hand), Some(Hand::Off));
    }
}
