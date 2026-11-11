//! The bundle validator (Phase 2 plan, step 4): a bundle is used only after
//! it passes (gate item 4: invalid bundles are rejected before any signing
//! exists). Five kinds of check, each finding with a stable code (ADR-014):
//!
//! - **structural:** every listed file exists with its SHA-256 and size; no
//!   file is unlisted; `signatures` is empty (formats 1 and 2);
//! - **schema:** the bundle loads ([`Bundle::load`]: format, game, paths,
//!   required files, every file parses);
//! - **referential:** mod stats, quality stats, and the stats of the
//!   rules' compositions are stats; mod classes and base classes are item
//!   classes; base implicits are mods; graft gems are base items; client
//!   texts the rules name exist; in the skill tables (format 2), every
//!   stat, skill type, item class, active skill, granted effect, stat set,
//!   base item, gem effect, gem tag, and cost type a row names exists;
//! - **domain:** IDs are unique per kind, and in the skill tables not empty
//!   (`BUNDLE_EMPTY_ID`); the item rules validate; the
//!   per-level rows of a granted effect or stat set have strictly
//!   ascending levels (`BUNDLE_SKILL_LEVELS`);
//! - **smoke:** every stat-description scope the rules name is one file,
//!   builds, and renders a sample.
//!
//! The `poe1/` section is optional for a PoE1 bundle (an extract without
//! its tables gives none; the PoE1 pipeline then resolves no graft or
//! quality line); a PoE2 bundle must not have it (the loader refuses).
//!
//! Structural findings stop before the schema check (a file whose bytes
//! are not the listed ones is not parsed); a schema finding stops the rest.

use std::collections::BTreeSet;

use serde::Serialize;
use sha2::{Digest, Sha256};

use super::skills::{self, SkillTables};
use super::{Bundle, BundleError, BundleSource, MANIFEST, Manifest, SUPPORTED_FORMATS};
use crate::stat_desc::{StatValue, VariantFilter, render_description};

/// One problem found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    /// Stable code.
    pub code: &'static str,
    /// The file or row the finding is about.
    pub at: String,
    /// What is wrong.
    pub detail: String,
}

/// The validator's result.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Report {
    /// Every finding, in check order.
    pub findings: Vec<Finding>,
}

impl Report {
    /// Whether the bundle passed.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.findings.is_empty()
    }

    /// The distinct codes found, sorted.
    #[must_use]
    pub fn codes(&self) -> Vec<&'static str> {
        let set: BTreeSet<&'static str> = self.findings.iter().map(|f| f.code).collect();
        set.into_iter().collect()
    }

    fn add(&mut self, code: &'static str, at: impl Into<String>, detail: impl Into<String>) {
        self.findings.push(Finding {
            code,
            at: at.into(),
            detail: detail.into(),
        });
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Validates the bundle `source` holds (module docs).
#[must_use]
pub fn validate(source: &dyn BundleSource) -> Report {
    let mut report = Report::default();
    if !structural(source, &mut report) {
        return report;
    }
    let bundle = match Bundle::load(source) {
        Ok(b) => b,
        Err(e) => {
            report.add(e.code(), at_of(&e), e.to_string());
            return report;
        }
    };
    referential(&bundle, &mut report);
    domain(&bundle, &mut report);
    smoke(&bundle, &mut report);
    report
}

fn at_of(e: &BundleError) -> String {
    match e {
        BundleError::Json { path, .. } | BundleError::StatDescription { path, .. } => path.clone(),
        BundleError::MissingFile(p)
        | BundleError::BadPath(p)
        | BundleError::DuplicatePath(p)
        | BundleError::GameSection(p, _)
        | BundleError::FormatFile(p, _) => p.clone(),
        _ => MANIFEST.to_owned(),
    }
}

/// Hashes, sizes, unlisted files, signatures; `false` when the schema check
/// must not run.
fn structural(source: &dyn BundleSource, report: &mut Report) -> bool {
    let bytes = match source.read(MANIFEST) {
        Ok(b) => b,
        Err(e) => {
            report.add(e.code(), MANIFEST, e.to_string());
            return false;
        }
    };
    // An unknown format is the loader's finding; this check needs a known one.
    let manifest: Manifest = match serde_json::from_slice(&bytes) {
        Ok(m) => m,
        Err(_) => return true,
    };
    if !SUPPORTED_FORMATS.contains(&manifest.format) {
        return true;
    }
    let before = report.findings.len();
    if !manifest.signatures.is_empty() {
        report.add(
            "BUNDLE_SIGNATURES",
            MANIFEST,
            "signatures are reserved and empty in formats 1 and 2",
        );
    }
    for f in &manifest.files {
        match source.read(&f.path) {
            Err(e) => report.add(e.code(), &f.path, e.to_string()),
            Ok(b) => {
                if b.len() as u64 != f.size {
                    report.add(
                        "BUNDLE_SIZE_MISMATCH",
                        &f.path,
                        format!("listed {} bytes, found {}", f.size, b.len()),
                    );
                }
                let sha = hex(&Sha256::digest(&b));
                if sha != f.sha256 {
                    report.add("BUNDLE_HASH_MISMATCH", &f.path, format!("SHA-256 {sha}"));
                }
            }
        }
    }
    match source.list() {
        Err(e) => report.add(e.code(), "", e.to_string()),
        Ok(paths) => {
            for p in paths {
                if p != MANIFEST && manifest.files.iter().all(|f| f.path != p) {
                    report.add("BUNDLE_UNLISTED_FILE", &p, "not in the manifest");
                }
            }
        }
    }
    report.findings.len() == before
}

fn referential(b: &Bundle, report: &mut Report) {
    let stats: BTreeSet<&str> = b.stats.iter().map(|s| s.id.as_str()).collect();
    let classes: BTreeSet<&str> = b.item_classes.iter().map(String::as_str).collect();
    let mods: BTreeSet<&str> = b.mods.iter().map(|m| m.id.as_str()).collect();
    let bases: BTreeSet<&str> = b.base_items.iter().map(|x| x.id.as_str()).collect();
    let mut dangling = |at: String, what: &str, id: &str| {
        report.add(
            "BUNDLE_DANGLING_REF",
            at,
            format!("{what} {id:?} does not exist"),
        );
    };
    for m in &b.mods {
        for s in &m.stats {
            if !stats.contains(s.stat.as_str()) {
                dangling(format!("mods:{}", m.id), "stat", &s.stat);
            }
        }
        for c in m
            .craft_classes
            .iter()
            .chain(m.essence_classes.iter().flatten())
        {
            if !classes.contains(c.as_str()) {
                dangling(format!("mods:{}", m.id), "item class", c);
            }
        }
    }
    for x in &b.base_items {
        if let Some(c) = &x.item_class
            && !classes.contains(c.as_str())
        {
            dangling(format!("base_items:{}", x.id), "item class", c);
        }
        for i in &x.implicits {
            if !mods.contains(i.as_str()) {
                dangling(format!("base_items:{}", x.id), "mod", i);
            }
        }
    }
    for (id, stat) in &b.item_rules.text.compositions {
        if !stats.contains(stat.as_str()) {
            dangling(format!("item_rules:{id}"), "stat", stat);
        }
    }
    if let Some(p) = &b.poe1 {
        for q in p.quality_types.iter().flatten() {
            for (s, _) in &q.stats {
                if !stats.contains(s.as_str()) {
                    dangling(format!("poe1/quality_types:{}", q.id), "stat", s);
                }
            }
        }
        for g in p.grafts.iter().flat_map(|g| &g.bases) {
            if !bases.contains(g.gem.as_str()) {
                dangling(format!("poe1/grafts:{}", g.graft), "gem", &g.gem);
            }
        }
    }
    if let Some(sk) = &b.skills {
        skill_refs(sk, &stats, &classes, &bases, report);
    }
    let texts: BTreeSet<&str> = b.client_texts.iter().map(|t| t.id.as_str()).collect();
    let t = &b.item_rules.text;
    let named = t
        .compositions
        .iter()
        .map(|(id, _)| id)
        .chain(&t.property_ids)
        .chain(&t.label_ids)
        .chain(&t.granted_skill_wrapper);
    for id in named {
        if !texts.contains(id.as_str()) {
            report.add(
                "BUNDLE_MISSING_CLIENT_TEXT",
                "data/client_texts.json",
                format!("the item rules name {id:?}"),
            );
        }
    }
}

/// The skill tables' references (module docs).
fn skill_refs(
    sk: &SkillTables,
    stats: &BTreeSet<&str>,
    classes: &BTreeSet<&str>,
    bases: &BTreeSet<&str>,
    report: &mut Report,
) {
    fn ids<'a>(it: impl Iterator<Item = &'a str>) -> BTreeSet<&'a str> {
        it.collect()
    }
    let types = ids(sk.skill_types.iter().map(|t| t.id.as_str()));
    let actives = ids(sk.active_skills.iter().map(|a| a.id.as_str()));
    let effects = ids(sk.granted_effects.iter().map(|e| e.id.as_str()));
    let sets = ids(sk.stat_sets.iter().map(|s| s.id.as_str()));
    let gem_effects = ids(sk.gem_effects.iter().map(|e| e.id.as_str()));
    let tags = ids(sk.gem_tags.iter().map(|t| t.id.as_str()));
    let costs = ids(sk.cost_types.iter().map(String::as_str));
    let mut check = |known: &BTreeSet<&str>, at: &dyn Fn() -> String, what: &str, id: &str| {
        if !known.contains(id) {
            report.add(
                "BUNDLE_DANGLING_REF",
                at(),
                format!("{what} {id:?} does not exist"),
            );
        }
    };
    for t in &sk.skill_types {
        let at = || format!("skill_types:{}", t.id);
        if let Some(s) = &t.flag_stat {
            check(stats, &at, "stat", s);
        }
    }
    for a in &sk.active_skills {
        let at = || format!("active_skills:{}", a.id);
        for t in a.types.iter().chain(&a.minion_types) {
            check(&types, &at, "skill type", t);
        }
        for c in &a.weapon_classes {
            check(classes, &at, "item class", c);
        }
    }
    for e in &sk.granted_effects {
        let at = || format!("granted_effects:{}", e.id);
        if let Some(a) = &e.active_skill {
            check(&actives, &at, "active skill", a);
        }
        for t in e
            .allowed_types
            .iter()
            .chain(&e.added_types)
            .chain(&e.excluded_types)
            .chain(&e.added_minion_types)
        {
            check(&types, &at, "skill type", t);
        }
        for s in &e.stat_sets {
            check(&sets, &at, "stat set", s);
        }
        for q in &e.quality {
            check(stats, &at, "stat", &q.stat);
        }
        for c in e.levels.iter().flat_map(|l| &l.costs) {
            check(&costs, &at, "cost type", &c.resource);
        }
    }
    for set in &sk.stat_sets {
        let at = || format!("stat_sets:{}", set.id);
        let set_stats = set
            .implicit_stats
            .iter()
            .chain(set.constant_stats.iter().map(|c| &c.stat));
        let level_stats = set.levels.iter().flat_map(|l| {
            l.float_stats
                .iter()
                .map(|f| &f.stat)
                .chain(l.stats.iter().map(|s| &s.stat))
                .chain(&l.flags)
        });
        for s in set_stats.chain(level_stats) {
            check(stats, &at, "stat", s);
        }
        for e in set.levels.iter().flat_map(|l| &l.granted_effects) {
            check(&effects, &at, "granted effect", e);
        }
    }
    for g in &sk.gems {
        let at = || format!("gems:{}", g.base_item);
        check(bases, &at, "base item", &g.base_item);
        for e in &g.effects {
            check(&gem_effects, &at, "gem effect", e);
        }
    }
    for e in &sk.gem_effects {
        let at = || format!("gem_effects:{}", e.id);
        for g in e.granted_effect.iter().chain(&e.additional_granted_effects) {
            check(&effects, &at, "granted effect", g);
        }
        for t in &e.tags {
            check(&tags, &at, "gem tag", t);
        }
    }
}

/// The skill tables' domain checks: IDs not empty and unique per file,
/// ascending levels.
fn skill_domain(sk: &SkillTables, report: &mut Report) {
    let files: [(&str, Vec<&str>); 8] = [
        (
            skills::SKILL_TYPES,
            sk.skill_types.iter().map(|t| t.id.as_str()).collect(),
        ),
        (
            skills::COST_TYPES,
            sk.cost_types.iter().map(String::as_str).collect(),
        ),
        (
            skills::ACTIVE_SKILLS,
            sk.active_skills.iter().map(|a| a.id.as_str()).collect(),
        ),
        (
            skills::GRANTED_EFFECTS,
            sk.granted_effects.iter().map(|e| e.id.as_str()).collect(),
        ),
        (
            skills::STAT_SETS,
            sk.stat_sets.iter().map(|s| s.id.as_str()).collect(),
        ),
        (
            skills::GEMS,
            sk.gems.iter().map(|g| g.base_item.as_str()).collect(),
        ),
        (
            skills::GEM_EFFECTS,
            sk.gem_effects.iter().map(|e| e.id.as_str()).collect(),
        ),
        (
            skills::GEM_TAGS,
            sk.gem_tags.iter().map(|t| t.id.as_str()).collect(),
        ),
    ];
    for (file, ids) in files {
        let empty = ids.iter().filter(|id| id.is_empty()).count();
        if empty > 0 {
            report.add("BUNDLE_EMPTY_ID", file, format!("{empty} rows have no ID"));
        }
        duplicates(report, file, ids.into_iter());
    }
    let ascending = |levels: Vec<i64>| levels.iter().zip(levels.iter().skip(1)).all(|(a, b)| a < b);
    for e in &sk.granted_effects {
        if !ascending(e.levels.iter().map(|l| l.level).collect()) {
            report.add(
                "BUNDLE_SKILL_LEVELS",
                format!("granted_effects:{}", e.id),
                "levels are not strictly ascending",
            );
        }
    }
    for s in &sk.stat_sets {
        if !ascending(s.levels.iter().map(|l| l.level).collect()) {
            report.add(
                "BUNDLE_SKILL_LEVELS",
                format!("stat_sets:{}", s.id),
                "levels are not strictly ascending",
            );
        }
    }
}

fn duplicates<'a>(report: &mut Report, file: &str, ids: impl Iterator<Item = &'a str>) {
    let mut seen = BTreeSet::new();
    for id in ids {
        if !seen.insert(id) {
            report.add(
                "BUNDLE_DUPLICATE_ID",
                file,
                format!("{id:?} is on two rows"),
            );
        }
    }
}

fn domain(b: &Bundle, report: &mut Report) {
    duplicates(report, super::STATS, b.stats.iter().map(|s| s.id.as_str()));
    duplicates(report, super::MODS, b.mods.iter().map(|m| m.id.as_str()));
    duplicates(
        report,
        super::BASE_ITEMS,
        b.base_items.iter().map(|x| x.id.as_str()),
    );
    duplicates(
        report,
        super::ITEM_CLASSES,
        b.item_classes.iter().map(String::as_str),
    );
    if let Err(e) = b.item_rules.mods.validate() {
        report.add("BUNDLE_ITEM_RULES", super::ITEM_RULES, e.code());
    }
    if let Some(sk) = &b.skills {
        skill_domain(sk, report);
    }
}

fn smoke(b: &Bundle, report: &mut Report) {
    for scope in b.item_rules.scopes() {
        let root = match b.scope_root(scope) {
            Ok(root) => root,
            Err(e) => {
                report.add("BUNDLE_SCOPE", scope, e.to_string());
                continue;
            }
        };
        match b.stat_descriptions.scope(root) {
            Ok(s) => {
                // A sample renders: the first description that renders
                // with every stat at 1.
                let renders = s.descriptions().any(|(at, d)| {
                    let values = vec![StatValue::exact(1); d.stats.len()];
                    render_description(d, at.hand, &values, b, VariantFilter::default())
                        .is_ok_and(|(_, lines)| !lines.is_empty())
                });
                if !renders {
                    report.add("BUNDLE_SCOPE", scope, "no description renders");
                }
            }
            Err(e) => report.add("BUNDLE_SCOPE", scope, e.to_string()),
        }
    }
}
