//! `calc-survey` (Phase 3 plan, step 3): which builds are in the calc
//! slice's scope, decided from game data and the build, never from the
//! engine (decision 3), and what a calculation of them would need.
//!
//! Per build: import (PoB XML) and resolution with the bundle, the main
//! skill as PoB finds it ([`main_skill`]), the scope rules ([`rules`]):
//! archetype rules on the main skill, the tree-version rule, unknown
//! passives, and the excluded mechanics the scope file lists; and the
//! mechanic inventory ([`inventory`]).
//!
//! Across builds: the candidate skills of each kind (spell, attack) with
//! their in-scope builds, ranked by in-scope builds, then by the smaller
//! mean inventory (plan step 3, Q2); the chosen skills of the scope file
//! with their in-scope builds; and how many builds each stat, support,
//! other skill, and config input reaches.
//!
//! **Inputs:** the bundle (format 2, PoE1, of the scope file's Steam
//! build), the scope file (`docs/phase-3/calc-scope-poe1.json`), and the
//! data-parity rules file (`docs/phase-3/data-parity-rules-poe1.json`) for
//! the passives a `not-carried` rule names.
//!
//! **Scrubbing (ADR-007):** builds are named by path, or by SHA-256 with
//! `--anonymous`; the outputs hold game-data IDs, PoB config names, and
//! counts, never item, label, or author text. Identical files count once.

pub mod inventory;
pub mod main_skill;
pub mod rules;

#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, BTreeSet};

use loom_data::bundle::passives::{MasteryEffect, Passive};
use loom_data::bundle::skills::{ActiveSkill, GemEffect, GrantedEffect, SkillGem};
use loom_data::bundle::{BaseItem, Bundle};
use loom_domain::{Build, Game};
use loom_parser::import::ItemData as _;
use loom_serialization::legacy::pob_item;
use serde::Serialize;
use sha2::{Digest, Sha256};

use self::inventory::{Inventory, active_spec};
use self::main_skill::MainSkill;
use self::rules::{Archetype, Exclusion, Kind, Reason, Scope};
use crate::resolve_corpus::{BundleInputs, Tool};

/// Summary format.
pub const FORMAT: u32 = 1;

/// The bundle's tables the survey reads, indexed.
pub struct Tables<'b> {
    /// The bundle (the engine's skill rule reads it through `CalcData`).
    pub bundle: &'b Bundle,
    /// Granted effects by ID.
    pub granted: BTreeMap<&'b str, &'b GrantedEffect>,
    /// Active skills by ID.
    pub active: BTreeMap<&'b str, &'b ActiveSkill>,
    /// Gems by base item ID.
    pub gems: BTreeMap<&'b str, &'b SkillGem>,
    /// Gem effects by ID.
    pub gem_effects: BTreeMap<&'b str, &'b GemEffect>,
    /// Passives by node.
    pub passives: BTreeMap<i64, &'b Passive>,
    /// Mastery effects by hash.
    pub masteries: BTreeMap<i64, &'b MasteryEffect>,
    /// Base item rows by name.
    pub bases: BTreeMap<&'b str, Vec<&'b BaseItem>>,
}

impl<'b> Tables<'b> {
    /// Indexes a format-2 bundle; an error names what is missing.
    pub fn new(bundle: &'b Bundle) -> Result<Self, String> {
        let skills = bundle
            .skills
            .as_ref()
            .ok_or("the bundle has no skill tables (format 1)")?;
        let passives = bundle
            .passives
            .as_ref()
            .ok_or("the bundle has no passive tables (format 1)")?;
        let mut bases: BTreeMap<&str, Vec<&BaseItem>> = BTreeMap::new();
        for b in &bundle.base_items {
            bases.entry(b.name.as_str()).or_default().push(b);
        }
        Ok(Self {
            bundle,
            granted: skills
                .granted_effects
                .iter()
                .map(|g| (g.id.as_str(), g))
                .collect(),
            active: skills
                .active_skills
                .iter()
                .map(|a| (a.id.as_str(), a))
                .collect(),
            gems: skills
                .gems
                .iter()
                .map(|g| (g.base_item.as_str(), g))
                .collect(),
            gem_effects: skills
                .gem_effects
                .iter()
                .map(|g| (g.id.as_str(), g))
                .collect(),
            passives: passives.passives.iter().map(|p| (p.node, p)).collect(),
            masteries: passives
                .mastery_effects
                .iter()
                .map(|m| (m.hash, m))
                .collect(),
            bases,
        })
    }
}

/// The main skill, as the summary shows it.
#[derive(Debug, Clone, Serialize)]
pub struct SkillRow {
    /// The main group's position in the active skill set.
    pub group: usize,
    /// Granted effect ID.
    pub skill: String,
    /// The active skill's display name (game data), if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// `spell` or `attack`, when it is one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<Kind>,
    /// Types after the supports.
    pub types: BTreeSet<String>,
    /// Whether an item grants it.
    #[serde(skip_serializing_if = "core::ops::Not::not")]
    pub item_granted: bool,
}

/// One build's survey (summary row).
#[derive(Debug, Clone, Serialize)]
pub struct BuildRow {
    /// The path as given, or `sha256:<hex>` with `--anonymous`.
    pub name: String,
    /// The active spec's tree version, if read.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tree_version: Option<String>,
    /// The main skill, if found.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub main_skill: Option<SkillRow>,
    /// Whether every rule passed.
    pub in_scope: bool,
    /// Whether the main skill passes the archetype rules (tree and
    /// exclusions aside).
    pub archetype_ok: bool,
    /// Why it is out of scope, each `<group>:<name>`.
    pub reasons: Vec<String>,
    /// The state a calculation would give (`in-scope`, `out-of-scope`,
    /// `indeterminate`; the worst of the reasons).
    pub state: &'static str,
    /// [`Inventory::size`].
    pub inventory_size: usize,
}

/// One build's inventory (report).
#[derive(Debug, Clone, Serialize)]
pub struct BuildInventory {
    /// As in [`BuildRow::name`].
    pub name: String,
    /// The inventory.
    pub inventory: Inventory,
}

/// A candidate skill of a kind.
#[derive(Debug, Clone, Serialize)]
pub struct Candidate {
    /// Granted effect ID.
    pub skill: String,
    /// Display name, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Builds whose main skill it is and that pass the archetype rules.
    pub archetype_builds: usize,
    /// Of those, the in-scope builds.
    pub in_scope: Vec<String>,
    /// Mean inventory size of the in-scope builds (rounded down), or of
    /// the archetype builds when none is in scope.
    pub mean_inventory: usize,
}

/// A chosen skill and its in-scope builds.
#[derive(Debug, Clone, Serialize)]
pub struct ChosenRow {
    /// Granted effect ID.
    pub skill: String,
    /// In-scope builds.
    pub in_scope: Vec<String>,
    /// Whether they reach the scope file's minimum.
    pub meets_minimum: bool,
}

/// How many builds reach each item of the inventory.
#[derive(Debug, Default, Clone, Serialize)]
pub struct Reach {
    /// Builds counted.
    pub builds: usize,
    /// Stat ID (passive or item) to builds.
    pub stats: BTreeMap<String, usize>,
    /// Typed kind to builds.
    pub typed_kinds: BTreeMap<String, usize>,
    /// Support to builds.
    pub supports: BTreeMap<String, usize>,
    /// `<class>:<skill>` of other groups to builds.
    pub other_skills: BTreeMap<String, usize>,
    /// Keystone to builds.
    pub keystones: BTreeMap<String, usize>,
    /// Config input to builds.
    pub config: BTreeMap<String, usize>,
    /// Builds with an unresolved or unrecognized item line.
    pub builds_with_unresolved_lines: usize,
}

impl Reach {
    fn add(&mut self, inv: &Inventory) {
        self.builds += 1;
        let stats: BTreeSet<&String> = inv
            .passive_stats
            .keys()
            .chain(inv.item_stats.keys())
            .collect();
        for s in stats {
            *self.stats.entry(s.clone()).or_default() += 1;
        }
        for k in inv.typed_kinds.keys() {
            *self.typed_kinds.entry((*k).to_owned()).or_default() += 1;
        }
        for s in &inv.supports {
            *self.supports.entry(s.clone()).or_default() += 1;
        }
        for (class, skills) in &inv.other_skills {
            for s in skills {
                *self.other_skills.entry(format!("{class}:{s}")).or_default() += 1;
            }
        }
        for k in &inv.keystones {
            *self.keystones.entry(k.clone()).or_default() += 1;
        }
        for c in &inv.config {
            *self.config.entry(c.clone()).or_default() += 1;
        }
        if inv.unrecognized_lines > 0 || inv.item_lines.contains_key("unresolved") {
            self.builds_with_unresolved_lines += 1;
        }
    }
}

/// Recorded inputs.
#[derive(Debug, Clone, Serialize)]
pub struct Inputs {
    /// The tool.
    pub tool: Tool,
    /// The bundle.
    pub bundle: BundleInputs,
    /// SHA-256 of the scope file.
    pub scope_sha256: String,
    /// SHA-256 of the data-parity rules file.
    pub data_rules_sha256: String,
    /// Whether builds are named by content hash.
    pub anonymous: bool,
}

/// The summary.
#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    /// [`FORMAT`].
    pub format: u32,
    /// Recorded inputs.
    pub inputs: Inputs,
    /// The scope file as read.
    pub scope: Scope,
    /// Distinct builds.
    pub builds: usize,
    /// Files skipped as identical to an earlier one.
    pub duplicates: usize,
    /// Builds in scope (any main skill of a kind).
    pub in_scope: usize,
    /// Reason to builds it puts out of scope.
    pub reasons: BTreeMap<String, usize>,
    /// State to builds.
    pub states: BTreeMap<&'static str, usize>,
    /// Kind to ranked candidates (plan step 3).
    pub candidates: BTreeMap<&'static str, Vec<Candidate>>,
    /// Kind to the chosen skill, when the scope file names one.
    pub chosen: BTreeMap<&'static str, ChosenRow>,
    /// Whether every chosen skill meets the minimum (true when none is
    /// chosen yet).
    pub pass: bool,
    /// Every build.
    pub per_build: Vec<BuildRow>,
}

/// The report: inventories and reach.
#[derive(Debug, Clone, Serialize)]
pub struct Report {
    /// Reach over the in-scope builds of the chosen skills (every in-scope
    /// build while none is chosen).
    pub reach_in_scope: Reach,
    /// Reach over every build whose main skill passes the archetype rules.
    pub reach_archetype: Reach,
    /// Every build's inventory; empty with `--anonymous` (ADR-007).
    pub inventories: Vec<BuildInventory>,
}

/// One input file.
pub struct Input {
    /// The path as given (`/` separators).
    pub path: String,
    /// The bytes.
    pub bytes: Vec<u8>,
}

/// What [`run`] needs.
pub struct Request<'a, 'r, 'd, 'b> {
    /// The pipeline, to resolve item lines.
    pub resolver: &'r loom_parser::import::Resolver<'d, 'b>,
    /// Its data (for base names).
    pub data: &'d loom_parser::import::BundleData<'b>,
    /// The indexed tables.
    pub tables: &'a Tables<'b>,
    /// The scope file.
    pub scope: &'a Scope,
    /// Passives a `not-carried` rule names, by node.
    pub not_carried: &'a BTreeSet<i64>,
    /// Name builds by content hash.
    pub anonymous: bool,
    /// Recorded inputs.
    pub inputs: Inputs,
}

/// The node numbers of passives that a `not-carried` rule of the
/// data-parity rules file names (table `passives`), after checking the
/// file is for the scope's Steam build.
pub fn not_carried_passives(rules: &[u8], steam_build: u64) -> Result<BTreeSet<i64>, String> {
    let v: serde_json::Value =
        serde_json::from_slice(rules).map_err(|e| format!("data-parity rules: {e}"))?;
    let build = v.get("steam_build").and_then(serde_json::Value::as_u64);
    if build != Some(steam_build) {
        return Err(format!(
            "data-parity rules: Steam build {build:?}, the scope file's is {steam_build}"
        ));
    }
    let mut out = BTreeSet::new();
    for rule in v
        .get("rules")
        .and_then(serde_json::Value::as_array)
        .ok_or("data-parity rules: no rules")?
    {
        let field = |k: &str| rule.get(k).and_then(serde_json::Value::as_str);
        if field("category") != Some("not-carried") || field("table") != Some("passives") {
            continue;
        }
        for key in rule
            .get("keys")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
        {
            let node = key
                .as_str()
                .and_then(|k| k.parse::<i64>().ok())
                .ok_or_else(|| format!("data-parity rules: key {key} is not a node number"))?;
            out.insert(node);
        }
    }
    Ok(out)
}

/// The scope verdict of a resolved build (module docs). `main` is the
/// main skill's result.
#[must_use]
pub fn verdict(
    build: &Build,
    main: &Result<MainSkill<'_>, main_skill::Problem>,
    t: &Tables<'_>,
    scope: &Scope,
    not_carried: &BTreeSet<i64>,
    base_names: &[&str],
) -> Vec<Reason> {
    let mut reasons = Vec::new();
    if build.game() != Game::Poe1 {
        reasons.push(Reason::NotPoe1);
        return reasons;
    }
    match main {
        Ok(skill) => {
            for rule in Archetype::ALL {
                if rule.applies(skill) {
                    reasons.push(Reason::Archetype(rule));
                }
            }
        }
        Err(p) => reasons.push(Reason::MainSkill(p.name())),
    }
    let Some(spec) = active_spec(build) else {
        reasons.push(Reason::UntypedSpec);
        return reasons;
    };
    if spec
        .tree_version
        .as_ref()
        .map(loom_domain::TreeVersion::as_str)
        != Some(scope.tree_version.as_str())
    {
        reasons.push(Reason::TreeVersion);
    }
    let allocated: BTreeSet<i64> = spec.nodes.value().iter().map(|n| i64::from(n.0)).collect();
    if allocated.iter().any(|n| !t.passives.contains_key(n))
        || spec
            .mastery_effects
            .value()
            .iter()
            .any(|m| !t.masteries.contains_key(&i64::from(m.effect)))
    {
        reasons.push(Reason::UnknownPassive);
    }
    if scope.excludes(Exclusion::NotCarriedPassive)
        && allocated.iter().any(|n| not_carried.contains(n))
    {
        reasons.push(Reason::Excluded(Exclusion::NotCarriedPassive));
    }
    let mut jewels = BTreeSet::new();
    // A socket entry the import kept untyped may hold a jewel.
    let mut unread = spec.jewel_sockets.opaque().next().is_some();
    for socket in spec.jewel_sockets.typed() {
        let Some(id) = socket.item else { continue };
        if !allocated.contains(&i64::from(socket.node.0)) {
            continue;
        }
        // An item the survey cannot name a base row for could be either
        // jewel: never treated as neither (ADR-015).
        let rows = build
            .items
            .item(id)
            .and_then(|item| pob_item::item_base(&item.body, base_names.iter().copied()).ok())
            .and_then(|(name, _)| t.bases.get(name.as_str()));
        let Some(rows) = rows else {
            unread = true;
            continue;
        };
        for row in rows {
            if row.id == rules::TIMELESS_JEWEL_BASE {
                jewels.insert(Exclusion::TimelessJewel);
            }
            if row
                .tags
                .iter()
                .flatten()
                .any(|tag| tag.starts_with(rules::CLUSTER_JEWEL_TAG))
            {
                jewels.insert(Exclusion::ClusterJewel);
            }
        }
    }
    for e in jewels {
        if scope.excludes(e) {
            reasons.push(Reason::Excluded(e));
        }
    }
    if unread
        && (scope.excludes(Exclusion::TimelessJewel) || scope.excludes(Exclusion::ClusterJewel))
    {
        reasons.push(Reason::UnreadJewel);
    }
    reasons.sort();
    reasons.dedup();
    reasons
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// One build's survey: its row and inventory.
struct Surveyed {
    row: BuildRow,
    inventory: Inventory,
}

fn survey_one(name: String, bytes: &[u8], req: &Request<'_, '_, '_, '_>) -> Surveyed {
    let mut row = BuildRow {
        name,
        tree_version: None,
        main_skill: None,
        in_scope: false,
        archetype_ok: false,
        reasons: Vec::new(),
        state: "out-of-scope",
        inventory_size: 0,
    };
    let fail = |mut row: BuildRow, reason: Reason| {
        row.reasons = vec![reason.name()];
        Surveyed {
            row,
            inventory: Inventory::default(),
        }
    };
    let Ok(text) = core::str::from_utf8(bytes) else {
        return fail(row, Reason::Import("NOT_UTF8"));
    };
    let mut build = match loom_serialization::legacy::pob_xml::import(text) {
        Ok(b) => b,
        Err(e) => return fail(row, Reason::Import(e.code())),
    };
    if build.game() == Game::Poe1 {
        let resolved = req.resolver.resolve_build(&mut build, |body| {
            pob_item::item_base(body, req.data.base_names())
                .map(|(name, fallback)| loom_parser::import::ItemBaseName { name, fallback })
                .map_err(|e| e.name().to_owned())
        });
        if let Err(e) = resolved {
            return fail(row, Reason::Resolve(e.code()));
        }
    }
    let base_names: Vec<&str> = req.data.base_names().collect();
    let main = main_skill::main_skill(&build, req.tables);
    let reasons = verdict(
        &build,
        &main,
        req.tables,
        req.scope,
        req.not_carried,
        &base_names,
    );
    row.tree_version = active_spec(&build)
        .and_then(|s| s.tree_version.as_ref())
        .map(|v| v.as_str().to_owned());
    let mut inventory =
        inventory::inventory(&build, req.tables, main.as_ref().ok().map(|m| m.group));
    if let Ok(m) = &main {
        inventory.supports = m.supports.iter().map(|s| s.effect.id.clone()).collect();
        inventory.unapplied_supports = m.unapplied.iter().map(|s| s.effect.id.clone()).collect();
        row.main_skill = Some(SkillRow {
            group: m.group,
            skill: m.effect().id.clone(),
            name: m.active.map(|a| a.name.clone()),
            kind: Kind::of(&m.types),
            types: m.types.clone(),
            item_granted: m.item_granted,
        });
    }
    row.archetype_ok = main.is_ok()
        && !reasons
            .iter()
            .any(|r| matches!(r, Reason::Archetype(_) | Reason::NotPoe1));
    row.in_scope = main.is_ok() && reasons.is_empty();
    row.state = if row.in_scope {
        "in-scope"
    } else if reasons.iter().any(|r| r.state() == "indeterminate") {
        "indeterminate"
    } else {
        "out-of-scope"
    };
    row.reasons = reasons.iter().map(Reason::name).collect();
    row.inventory_size = inventory.size();
    Surveyed { row, inventory }
}

/// Runs the survey (module docs).
#[must_use]
pub fn run(inputs: &[Input], req: &Request<'_, '_, '_, '_>) -> (Summary, Report) {
    let mut seen = BTreeSet::new();
    let mut duplicates = 0;
    let mut surveyed = Vec::new();
    for input in inputs {
        let hash = hex(&Sha256::digest(&input.bytes));
        if !seen.insert(hash.clone()) {
            duplicates += 1;
            continue;
        }
        let name = if req.anonymous {
            format!("sha256:{hash}")
        } else {
            input.path.clone()
        };
        surveyed.push(survey_one(name, &input.bytes, req));
    }
    summarize(surveyed, duplicates, req)
}

fn summarize(
    surveyed: Vec<Surveyed>,
    duplicates: usize,
    req: &Request<'_, '_, '_, '_>,
) -> (Summary, Report) {
    let mut reasons: BTreeMap<String, usize> = BTreeMap::new();
    let mut states: BTreeMap<&'static str, usize> = BTreeMap::new();
    // Kind to skill to (name, archetype builds, in-scope names, sizes).
    type Tally = (Option<String>, usize, Vec<String>, Vec<usize>, Vec<usize>);
    let mut by_skill: BTreeMap<Kind, BTreeMap<String, Tally>> = BTreeMap::new();
    let mut reach_archetype = Reach::default();
    for s in &surveyed {
        for r in &s.row.reasons {
            *reasons.entry(r.clone()).or_default() += 1;
        }
        *states.entry(s.row.state).or_default() += 1;
        let Some(skill) = &s.row.main_skill else {
            continue;
        };
        let (Some(kind), true) = (skill.kind, s.row.archetype_ok) else {
            continue;
        };
        reach_archetype.add(&s.inventory);
        let tally = by_skill
            .entry(kind)
            .or_default()
            .entry(skill.skill.clone())
            .or_insert_with(|| (skill.name.clone(), 0, Vec::new(), Vec::new(), Vec::new()));
        tally.1 += 1;
        tally.4.push(s.row.inventory_size);
        if s.row.in_scope {
            tally.2.push(s.row.name.clone());
            tally.3.push(s.row.inventory_size);
        }
    }
    let mean = |v: &[usize]| v.iter().sum::<usize>().checked_div(v.len()).unwrap_or(0);
    let mut candidates = BTreeMap::new();
    for (kind, skills) in by_skill {
        let mut list: Vec<Candidate> = skills
            .into_iter()
            .map(
                |(skill, (name, archetype_builds, in_scope, sizes, all))| Candidate {
                    skill,
                    name,
                    archetype_builds,
                    mean_inventory: if in_scope.is_empty() {
                        mean(&all)
                    } else {
                        mean(&sizes)
                    },
                    in_scope,
                },
            )
            .collect();
        list.sort_by(|a, b| {
            b.in_scope
                .len()
                .cmp(&a.in_scope.len())
                .then(a.mean_inventory.cmp(&b.mean_inventory))
                .then(b.archetype_builds.cmp(&a.archetype_builds))
                .then(a.skill.cmp(&b.skill))
        });
        candidates.insert(kind.name(), list);
    }
    let mut chosen = BTreeMap::new();
    let mut reach_in_scope = Reach::default();
    let any_chosen = req.scope.skills.spell.is_some() || req.scope.skills.attack.is_some();
    for kind in [Kind::Spell, Kind::Attack] {
        let Some(id) = req.scope.skills.of(kind) else {
            continue;
        };
        let in_scope: Vec<String> = surveyed
            .iter()
            .filter(|s| s.row.in_scope)
            .filter(|s| s.row.main_skill.as_ref().is_some_and(|m| m.skill == id))
            .map(|s| s.row.name.clone())
            .collect();
        chosen.insert(
            kind.name(),
            ChosenRow {
                skill: id.to_owned(),
                meets_minimum: in_scope.len() >= req.scope.minimum_per_skill,
                in_scope,
            },
        );
    }
    for s in &surveyed {
        let counted = s.row.in_scope
            && (!any_chosen
                || s.row.main_skill.as_ref().is_some_and(|m| {
                    m.kind
                        .and_then(|k| req.scope.skills.of(k))
                        .is_some_and(|id| id == m.skill)
                }));
        if counted {
            reach_in_scope.add(&s.inventory);
        }
    }
    let pass = chosen.values().all(|c| c.meets_minimum);
    let (rows, inventories): (Vec<BuildRow>, Vec<BuildInventory>) = surveyed
        .into_iter()
        .map(|s| {
            let inv = BuildInventory {
                name: s.row.name.clone(),
                inventory: s.inventory,
            };
            (s.row, inv)
        })
        .unzip();
    let summary = Summary {
        format: FORMAT,
        inputs: req.inputs.clone(),
        scope: req.scope.clone(),
        builds: rows.len(),
        duplicates,
        in_scope: rows.iter().filter(|r| r.in_scope).count(),
        reasons,
        states,
        candidates,
        chosen,
        pass,
        per_build: rows,
    };
    // ADR-007: a corpus build's full inventory works as a fingerprint of a
    // real build, so `--anonymous` keeps only the aggregates.
    let report = Report {
        reach_in_scope,
        reach_archetype,
        inventories: if req.anonymous {
            Vec::new()
        } else {
            inventories
        },
    };
    (summary, report)
}
