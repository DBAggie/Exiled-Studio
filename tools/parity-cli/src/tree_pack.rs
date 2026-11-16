//! `tree-pack`: adds an older passive tree version to a bundle as a tree
//! pack (`loom_data::bundle::tree_packs`; passive tree plan, T4), converted
//! from GGG's passive tree export at the commit pinned for that version in
//! `tools/tree/manifest.json` (`versions`; ADR-005 Amendment 2).
//! Maintainer-run: the export is fetched by `tools/tree/acquire.{ps1,sh}
//! <version>` into `local/tree-export-<version>/`.
//!
//! The export writes each passive's lines as text; each is read as stat
//! IDs through the bundle's passive stat descriptions (ADR-015), and a
//! line that does not read to one settled value is kept as text in the
//! pack's `unresolved` list (the calculation gives its node a reason),
//! never guessed. The rest maps field by field:
//! - a node keeps the bundle's passive ID when the bundle has the same
//!   node number, else `export<node>`; its `expansion` flag is the
//!   bundle's for the same node, else derived (a cluster socket, a
//!   position proxy, or a node off the graph);
//! - `ascendancyName` names the bundle's ascendancy by ID (an alternate
//!   or bloodline one the bundle does not list: none, as the bundle's own
//!   passives have it), and
//!   `classStartIndex` the bundle's class by number;
//! - a mastery keeps the bundle's group ID when the bundle's node is the
//!   same mastery, and each effect the bundle's effect ID by hash, else
//!   `export<node>` and `export<hash>`;
//! - the groups are the export's, in their numbers' order; links are the
//!   export's `out` lists (the root's are the class starts).
//!
//! A line in an older wording is read in its current one when GGG's data
//! shows the rewording ([`rewordings`]) and the review in
//! `tools/tree/rewordings.json` (beside the manifest) accepts it.
//!
//! Checks, each refusing the run: the export's checkout is the pinned
//! commit; its orbits are the bundle's; the bundle validates before and
//! the written one after; at most [`MAX_UNREAD_PERCENT`] of the export's
//! lines did not read (unless `--accept-unread`).
//!
//! `docs/tree/tree-packs.md` has the rules and the results.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use loom_data::bundle::Bundle;
use loom_data::bundle::passives::{
    MasteryEffect, MasteryGroup, Passive, PassiveTree, StatAmount, TreeConnection, TreeGroup,
    TreeNode,
};
use loom_data::bundle::tree_layout::ExportFile;
use loom_data::bundle::tree_packs::{self, PackLine, PackSource, TreePack};
use loom_data::stat_desc::Scope;
use loom_data::stat_desc::render::{NotRenderedReason, StatValue, VariantFilter};
use loom_parser::resolve::{Candidate, Index, Outcome, RawValue};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::tree_layout::{hex, read, write_copy};

/// The stat descriptions the tree's lines are written with.
const PASSIVE_DESCRIPTIONS: &str = "passive_skill_stat_descriptions";

#[derive(Deserialize)]
struct Pins {
    export: Repository,
    #[serde(default)]
    versions: BTreeMap<String, Pinned>,
}

#[derive(Deserialize)]
struct Repository {
    repository: String,
}

#[derive(Deserialize)]
struct Pinned {
    tag: String,
    commit: String,
}

/// What the run did.
#[derive(Serialize)]
pub struct Summary {
    version: String,
    tag: String,
    commit: String,
    passives: usize,
    mastery_effects: usize,
    groups: usize,
    unresolved: usize,
    /// The export's node and mastery effect lines (each effect once).
    lines: usize,
    /// Whether `unresolved` is over [`MAX_UNREAD_PERCENT`] of `lines`,
    /// written only with `--accept-unread`.
    over_threshold: bool,
    /// The rewordings the lines were read with (old template to
    /// current; `tools/tree/rewordings.json`, accepted).
    reworded: BTreeMap<String, String>,
    /// Learned rewordings the table neither accepts nor rejects: their
    /// lines stay unread until the table is reviewed.
    unreviewed: BTreeMap<String, String>,
    pack_sha256: String,
    manifest_sha256: String,
}

/// The share of an export's lines that may not read, in percent: a
/// version over it ships without its pack (passive tree plan, T4
/// fallback) unless the maintainer accepts it (`--accept-unread`).
pub const MAX_UNREAD_PERCENT: usize = 1;

/// The export's node lines and its mastery effects' lines, each effect
/// once (as the pack holds them).
fn line_count(data: &Value) -> usize {
    let count = |v: Option<&Value>| {
        v.and_then(Value::as_array).map_or(0, |a| {
            a.iter()
                .filter_map(Value::as_str)
                .flat_map(str::lines)
                .filter(|l| !l.trim().is_empty())
                .count()
        })
    };
    let mut effects = BTreeSet::new();
    let mut lines = 0;
    if let Some(nodes) = data.get("nodes").and_then(Value::as_object) {
        for (id, node) in nodes {
            if id == "root" {
                continue;
            }
            lines += count(node.get("stats"));
            for e in node
                .get("masteryEffects")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                if effects.insert(e.get("effect").and_then(Value::as_i64)) {
                    lines += count(e.get("stats"));
                }
            }
        }
    }
    lines
}

/// A line that did not read, and why.
type Unread = (String, &'static str);

/// `line` as a template and its numbers: each number (a sign, digits, an
/// optional fraction) becomes `#`.
fn template(line: &str) -> (String, Vec<&str>) {
    let bytes = line.as_bytes();
    let mut text = String::new();
    let mut numbers = Vec::new();
    let mut i = 0;
    let mut last = 0;
    while i < bytes.len() {
        let signed = matches!(bytes[i], b'+' | b'-')
            && bytes.get(i + 1).is_some_and(u8::is_ascii_digit)
            && (i == 0 || !bytes[i - 1].is_ascii_alphanumeric());
        if signed || bytes[i].is_ascii_digit() {
            let start = i;
            i += usize::from(signed);
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if bytes.get(i) == Some(&b'.') && bytes.get(i + 1).is_some_and(u8::is_ascii_digit) {
                i += 1;
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
            }
            text.push_str(&line[last..start]);
            text.push('#');
            numbers.push(&line[start..i]);
            last = i;
        } else {
            i += 1;
        }
    }
    text.push_str(&line[last..]);
    (text, numbers)
}

/// `template` with its `#`s replaced by `numbers` in order.
fn fill(template: &str, numbers: &[&str]) -> String {
    let mut out = String::new();
    let mut numbers = numbers.iter();
    for c in template.chars() {
        if c == '#'
            && let Some(n) = numbers.next()
        {
            out.push_str(n);
        } else {
            out.push(c);
        }
    }
    out
}

/// The rewordings GGG's own data shows between the export's lines and
/// the bundle's (old template to current template): a node or effect both
/// have, whose lines differ in exactly one line each way with the same
/// numbers, as "15% increased Global Accuracy Rating" on a node the
/// bundle prints "15% increased Accuracy Rating". A template two pairs
/// map differently is left out.
fn rewordings(pairs: &[(Vec<String>, Vec<String>)]) -> BTreeMap<String, String> {
    let mut found: BTreeMap<String, String> = BTreeMap::new();
    let mut conflicts = BTreeSet::new();
    for (old, current) in pairs {
        let mut only_old: Vec<&String> = Vec::new();
        let mut rest: Vec<&String> = current.iter().collect();
        for line in old {
            if let Some(i) = rest.iter().position(|c| *c == line) {
                rest.swap_remove(i);
            } else {
                only_old.push(line);
            }
        }
        let ([was], [now]) = (only_old.as_slice(), rest.as_slice()) else {
            continue;
        };
        let ((was, a), (now, b)) = (template(was), template(now));
        if a != b || was == now {
            continue;
        }
        if found.get(&was).is_some_and(|n| *n != now) {
            conflicts.insert(was.clone());
        }
        found.insert(was, now);
    }
    for c in conflicts {
        found.remove(&c);
    }
    found
}

/// Reads the export's lines as stat IDs (module docs).
struct Reader<'a> {
    bundle: &'a Bundle,
    scope: &'a Scope<'a>,
    index: &'a Index<'a>,
    /// The stats the bundle's passives, mastery effects, and templates
    /// use: of several readings of a line, the one of these.
    used: BTreeSet<&'a str>,
    /// Old wordings to current ones ([`rewordings`]): a line that does
    /// not read is read again in its current wording.
    reworded: BTreeMap<String, String>,
}

impl Reader<'_> {
    /// `stats` as the game prints them, if every one prints (a zero or a
    /// stat declared without a description prints nothing).
    fn printed(&self, stats: &[StatAmount]) -> Option<Vec<String>> {
        let input: BTreeMap<String, StatValue> = stats
            .iter()
            .map(|s| (s.stat.clone(), StatValue::exact(s.value)))
            .collect();
        let r = self
            .scope
            .render(&input, self.bundle, VariantFilter::default());
        let silent = r.not_rendered.iter().all(|n| {
            matches!(
                n.reason,
                NotRenderedReason::Zero | NotRenderedReason::DeclaredNoDescription
            )
        });
        silent.then(|| r.lines.into_iter().map(|l| l.text).collect())
    }

    /// The lines of `texts` (a node's or an effect's entries) and of
    /// `known` as printed, when there are both, for [`rewordings`].
    fn pair(
        &self,
        texts: &[String],
        known: Option<&[StatAmount]>,
    ) -> Option<(Vec<String>, Vec<String>)> {
        let split = |t: &[String]| -> Vec<String> {
            t.iter()
                .flat_map(|t| t.lines())
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .map(str::to_owned)
                .collect()
        };
        let printed = self.printed(known?)?;
        Some((split(texts), split(&printed)))
    }

    /// The stats of `texts` (a node's or an effect's entries): `known`
    /// (the bundle's stats for the same node or effect) when they print
    /// as these lines, else each line read alone; with the lines that did
    /// not read.
    fn read(
        &self,
        texts: &[String],
        known: Option<&[StatAmount]>,
    ) -> (Vec<StatAmount>, Vec<Unread>) {
        let lines: Vec<&str> = texts
            .iter()
            .flat_map(|t| t.lines())
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .collect();
        if let Some(known) = known
            && let Some(printed) = self.printed(known)
        {
            let mut a: Vec<&str> = printed
                .iter()
                .flat_map(|t| t.lines())
                .map(str::trim)
                .collect();
            let mut b = lines.clone();
            a.sort_unstable();
            b.sort_unstable();
            if a == b {
                return (known.to_vec(), Vec::new());
            }
        }
        let own: BTreeSet<&str> = known
            .into_iter()
            .flatten()
            .map(|s| s.stat.as_str())
            .collect();
        let mut stats = Vec::new();
        let mut unread = Vec::new();
        for text in texts {
            for r in self.stats_of(text, &own) {
                match r {
                    Ok(s) => stats.extend(s),
                    Err(e) => unread.push(e),
                }
            }
        }
        (stats, unread)
    }

    /// Reads `text` (one export entry: lines joined by `\n`): each line
    /// must resolve to one reading with every shown value settled; a
    /// hidden value counts at the value its condition allows, left out
    /// when it is 0. Of several readings, the one whose stats are all
    /// `own` (the bundle's stats for the same node or effect), else the
    /// one whose stats the bundle's passives use. A line with no reading
    /// is read again in its current wording when [`rewordings`] has one.
    fn stats_of(&self, text: &str, own: &BTreeSet<&str>) -> Vec<Result<Vec<StatAmount>, Unread>> {
        let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
        // A line that does not read alone, in its current wording when the
        // review accepts one (a line of a longer description too).
        let current: Vec<String> = lines
            .iter()
            .map(|l| {
                let (was, numbers) = template(l.trim());
                match self.reworded.get(&was) {
                    Some(now)
                        if matches!(
                            self.index.resolve_lines(&[*l]).first(),
                            Some(r) if r.lines == 1 && matches!(r.outcome, Outcome::Unresolved(_))
                        ) =>
                    {
                        fill(now, &numbers)
                    }
                    _ => (*l).to_owned(),
                }
            })
            .collect();
        let read: Vec<&str> = current.iter().map(String::as_str).collect();
        let mut out = Vec::new();
        let mut at = 0;
        for r in self.index.resolve_lines(&read) {
            let shown = lines[at..at + r.lines].join(
                "
",
            );
            at += r.lines;
            let candidates = match r.outcome {
                Outcome::Resolved(c) => vec![c],
                Outcome::Ambiguous(cs) => cs,
                Outcome::Unresolved(_) => {
                    out.push(Err((shown, "no-template")));
                    continue;
                }
            };
            let all_in = |c: &Candidate, set: &BTreeSet<&str>| {
                c.stats.iter().all(|s| set.contains(s.stat.as_str()))
            };
            let picked: Vec<&Candidate> = if candidates.len() == 1 {
                candidates.iter().collect()
            } else {
                let mine: Vec<&Candidate> = candidates.iter().filter(|c| all_in(c, own)).collect();
                if mine.len() == 1 {
                    mine
                } else {
                    candidates
                        .iter()
                        .filter(|c| all_in(c, &self.used))
                        .collect()
                }
            };
            let [c] = picked.as_slice() else {
                out.push(Err((shown, "ambiguous")));
                continue;
            };
            if !c.alternatives.is_empty() {
                out.push(Err((shown, "ambiguous")));
                continue;
            }
            let read = c
                .stats
                .iter()
                .filter_map(|s| {
                    let value = match &s.value {
                        RawValue::Exact(v) if v.min == v.max => Some(v.min),
                        RawValue::Hidden {
                            assumed: Some(0), ..
                        } => return None,
                        RawValue::Hidden {
                            assumed: Some(v), ..
                        } => Some(*v),
                        RawValue::Lookup(keys) if keys.len() == 1 => Some(keys[0]),
                        _ => None,
                    };
                    Some(value.map(|value| StatAmount {
                        stat: s.stat.clone(),
                        value,
                    }))
                })
                .collect::<Option<Vec<_>>>()
                .ok_or((shown, "unsettled"));
            out.push(read);
        }
        out
    }
}

/// The export's line for the passive points a node grants.
fn points_line(n: i64) -> String {
    if n == 1 {
        "Grants 1 Passive Skill Point".to_owned()
    } else {
        format!("Grants {n} Passive Skill Points")
    }
}

/// A node's lines but the one for the passive points it grants: those
/// are its `skill_points`, not a stat.
fn node_lines(n: &Value) -> Vec<String> {
    let points = int(&n["grantedPassivePoints"]).unwrap_or(0);
    texts(&n["stats"])
        .into_iter()
        .filter(|t| !(points > 0 && *t == points_line(points)))
        .collect()
}

fn int(v: &Value) -> Option<i64> {
    v.as_i64()
        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
}

fn flag(node: &Value, name: &str) -> bool {
    node[name].as_bool().unwrap_or(false)
}

fn texts(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|s| s.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// The reviewed rewordings (`tools/tree/rewordings.json`).
#[derive(Debug, Default, Deserialize)]
pub struct Reviewed {
    #[serde(default)]
    accepted: Vec<Rewording>,
    #[serde(default)]
    rejected: Vec<Rewording>,
}

#[derive(Debug, Deserialize)]
struct Rewording {
    old: String,
    current: String,
}

impl Reviewed {
    fn has(list: &[Rewording], old: &str, current: &str) -> bool {
        list.iter().any(|r| r.old == old && r.current == current)
    }
}

/// A converted pack, with the rewordings its lines were read with and the
/// learned ones the review has not seen.
pub struct Converted {
    pub pack: TreePack,
    pub reworded: BTreeMap<String, String>,
    pub unreviewed: BTreeMap<String, String>,
}

/// Converts the export `data` into `version`'s pack, its lines read
/// through `bundle`'s passive stat descriptions (module docs), with the
/// learned rewordings `reviewed` accepts.
///
/// # Errors
/// When the export lacks a field the pack needs, or the bundle has no
/// passive tables or passive stat descriptions.
pub fn convert(
    data: &Value,
    bundle: &Bundle,
    version: &str,
    source: PackSource,
    reviewed: &Reviewed,
) -> Result<Converted, String> {
    let own = bundle
        .passives
        .as_ref()
        .ok_or("the bundle has no passive tables")?;
    let root = bundle
        .scope_root(PASSIVE_DESCRIPTIONS)
        .map_err(|e| format!("{PASSIVE_DESCRIPTIONS}: {e:?}"))?;
    let scope = bundle
        .stat_descriptions
        .scope(root)
        .map_err(|e| format!("{PASSIVE_DESCRIPTIONS}: {e:?}"))?;
    let index = Index::build_with(&scope, bundle);
    let mut reader = Reader {
        bundle,
        scope: &scope,
        index: &index,
        used: own
            .passives
            .iter()
            .flat_map(|p| &p.stats)
            .chain(own.mastery_effects.iter().flat_map(|e| &e.stats))
            .chain(own.object_templates.iter().flat_map(|t| &t.stats))
            .map(|s| s.stat.as_str())
            .collect(),
        reworded: BTreeMap::new(),
    };
    let known_effects: BTreeMap<i64, &[StatAmount]> = own
        .mastery_effects
        .iter()
        .map(|e| (e.hash, e.stats.as_slice()))
        .collect();
    let rows: BTreeMap<i64, &Passive> = own.passives.iter().map(|p| (p.node, p)).collect();
    let nodes = data["nodes"].as_object().ok_or("data.json: no nodes")?;
    // The rewordings first, from every node and effect the bundle has.
    let mut pairs = Vec::new();
    for (key, n) in nodes {
        if key == "root" {
            continue;
        }
        let row = int(&n["skill"]).and_then(|node| rows.get(&node));
        pairs.extend(reader.pair(&node_lines(n), row.map(|r| r.stats.as_slice())));
        for e in n["masteryEffects"].as_array().into_iter().flatten() {
            let known = int(&e["effect"]).and_then(|h| known_effects.get(&h).copied());
            pairs.extend(reader.pair(&texts(&e["stats"]), known));
        }
    }
    let (accepted, unaccepted): (BTreeMap<_, _>, BTreeMap<_, _>) = rewordings(&pairs)
        .into_iter()
        .partition(|(o, c)| Reviewed::has(&reviewed.accepted, o, c));
    let unreviewed = unaccepted
        .into_iter()
        .filter(|(o, c)| !Reviewed::has(&reviewed.rejected, o, c))
        .collect();
    reader.reworded = accepted;
    let ascendancies: BTreeSet<&str> = own.ascendancies.iter().map(|a| a.id.as_str()).collect();
    let characters: BTreeMap<i64, &str> = own
        .characters
        .iter()
        .map(|c| (c.class_number, c.id.as_str()))
        .collect();
    let effect_ids: BTreeMap<i64, &str> = own
        .mastery_effects
        .iter()
        .map(|e| (e.hash, e.id.as_str()))
        .collect();
    let groups_of: BTreeMap<&str, Vec<&str>> = own
        .mastery_groups
        .iter()
        .map(|g| {
            (
                g.id.as_str(),
                g.effects.iter().map(String::as_str).collect(),
            )
        })
        .collect();

    let mut passives = Vec::new();
    let mut unresolved = Vec::new();
    let mut effects: BTreeMap<i64, MasteryEffect> = BTreeMap::new();
    let mut mastery_groups: Vec<MasteryGroup> = Vec::new();
    for (key, n) in nodes {
        if key == "root" {
            continue;
        }
        let node = int(&n["skill"]).ok_or_else(|| format!("node {key}: no skill"))?;
        let row = rows.get(&node).copied();
        // The points a node grants are its `skill_points`, not a stat: the
        // export prints them as a line ("Grants 1 Passive Skill Point").
        let skill_points = int(&n["grantedPassivePoints"]).unwrap_or(0);
        let (stats, unread) = reader.read(&node_lines(n), row.map(|r| r.stats.as_slice()));
        unresolved.extend(unread.into_iter().map(|(text, reason)| PackLine {
            node,
            mastery_effect: false,
            text,
            reason: reason.to_owned(),
        }));
        // The bundle's ascendancy by ID (the export's `ascendancyName` is
        // the ID: Raider is shown as Warden); an alternate or bloodline
        // ascendancy the game's tables do not list has none, as in the
        // bundle's own passives.
        let ascendancy = n["ascendancyName"]
            .as_str()
            .filter(|a| ascendancies.contains(a))
            .map(str::to_owned);
        let characters = match int(&n["classStartIndex"]) {
            Some(c) => vec![
                characters
                    .get(&c)
                    .ok_or_else(|| format!("node {node}: class {c} is not the bundle's"))?
                    .to_string(),
            ],
            None => Vec::new(),
        };
        let mastery_group = if flag(n, "isMastery") {
            let mut hashes = Vec::new();
            for e in n["masteryEffects"].as_array().into_iter().flatten() {
                let hash = int(&e["effect"]).ok_or_else(|| format!("node {node}: effect"))?;
                hashes.push(hash);
                if effects.contains_key(&hash) {
                    continue;
                }
                let (stats, unread) =
                    reader.read(&texts(&e["stats"]), known_effects.get(&hash).copied());
                unresolved.extend(unread.into_iter().map(|(text, reason)| PackLine {
                    node: hash,
                    mastery_effect: true,
                    text,
                    reason: reason.to_owned(),
                }));
                effects.insert(
                    hash,
                    MasteryEffect {
                        id: effect_ids
                            .get(&hash)
                            .map_or_else(|| format!("export{hash}"), |id| (*id).to_owned()),
                        hash,
                        stats,
                    },
                );
            }
            let ids: Vec<String> = hashes.iter().map(|h| effects[h].id.clone()).collect();
            // The bundle's group ID when its node is this mastery and the
            // ID is free or holds the same effects; else one of its own.
            let id = row
                .and_then(|r| r.mastery_group.as_deref())
                .filter(|g| groups_of.contains_key(g))
                .filter(|g| {
                    mastery_groups
                        .iter()
                        .find(|m| m.id == *g)
                        .is_none_or(|m| m.effects == ids)
                })
                .map_or_else(|| format!("export{node}"), str::to_owned);
            if !mastery_groups.iter().any(|m| m.id == id) {
                mastery_groups.push(MasteryGroup {
                    id: id.clone(),
                    effects: ids,
                });
            }
            Some(id)
        } else {
            None
        };
        let expansion = row.map_or_else(
            || n.get("expansionJewel").is_some() || flag(n, "isProxy") || n.get("group").is_none(),
            |r| r.expansion,
        );
        passives.push(Passive {
            id: row.map_or_else(|| format!("export{node}"), |r| r.id.clone()),
            node,
            name: n["name"].as_str().unwrap_or_default().to_owned(),
            stats,
            keystone: flag(n, "isKeystone"),
            notable: flag(n, "isNotable"),
            jewel_socket: flag(n, "isJewelSocket"),
            expansion,
            anoint_only: flag(n, "isBlighted"),
            multiple_choice: flag(n, "isMultipleChoice"),
            multiple_choice_option: flag(n, "isMultipleChoiceOption"),
            skill_points,
            mastery_group,
            ascendancy,
            ascendancy_start: flag(n, "isAscendancyStart"),
            characters,
        });
    }
    passives.sort_by_key(|p| p.node);
    let known: BTreeSet<i64> = passives.iter().map(|p| p.node).collect();

    let groups = data["groups"].as_object().ok_or("data.json: no groups")?;
    let mut numbered: Vec<(i64, &Value)> = groups
        .iter()
        .map(|(k, g)| Ok((k.parse::<i64>().map_err(|_| format!("group {k}"))?, g)))
        .collect::<Result<_, String>>()?;
    numbered.sort_by_key(|(k, _)| *k);
    let links = |n: &Value| -> Vec<TreeConnection> {
        texts(&n["out"])
            .iter()
            .filter_map(|o| o.parse::<i64>().ok())
            .filter(|o| known.contains(o))
            .map(|node| TreeConnection { node, orbit: None })
            .collect()
    };
    let mut tree_groups = Vec::new();
    for (k, g) in numbered {
        let mut members = Vec::new();
        for m in texts(&g["nodes"]) {
            let n = &nodes
                .get(&m)
                .ok_or_else(|| format!("group {k}: node {m}"))?;
            let Some(node) = int(&n["skill"]) else {
                continue;
            };
            members.push(TreeNode {
                node,
                orbit: int(&n["orbit"]).unwrap_or(0),
                orbit_index: int(&n["orbitIndex"]).unwrap_or(0),
                connections: links(n),
            });
        }
        #[expect(
            clippy::cast_possible_truncation,
            reason = "the export's coordinates fit f32, as the graph file stores them"
        )]
        let (x, y) = (
            g["x"].as_f64().unwrap_or(0.0) as f32,
            g["y"].as_f64().unwrap_or(0.0) as f32,
        );
        tree_groups.push(TreeGroup {
            x,
            y,
            flags: 0,
            nodes: members,
        });
    }
    let slots: Vec<i64> = serde_json::from_value(data["constants"]["skillsPerOrbit"].clone())
        .map_err(|e| format!("data.json constants.skillsPerOrbit: {e}"))?;
    let roots = nodes
        .get("root")
        .map(|r| {
            texts(&r["out"])
                .iter()
                .filter_map(|o| o.parse::<i64>().ok())
                .filter(|o| known.contains(o))
                .collect()
        })
        .unwrap_or_default();
    let pack = TreePack {
        version: version.to_owned(),
        tree: PassiveTree {
            tree: own.tree.tree.clone(),
            graph: format!("export {}", source.tag),
            orbit_slots: slots,
            roots,
            groups: tree_groups,
        },
        source,
        passives,
        mastery_groups,
        mastery_effects: effects.into_values().collect(),
        unresolved,
    };
    Ok(Converted {
        pack,
        reworded: reader.reworded,
        unreviewed,
    })
}

/// Runs the command (module docs).
///
/// # Errors
/// Every problem: the run writes nothing then.
pub fn run(
    pins: &Path,
    version: &str,
    export: &Path,
    bundle: &Path,
    out: &Path,
    accept_unread: bool,
) -> Result<Summary, String> {
    if tree_packs::version_of(&tree_packs::path(version)) != Some(version) {
        return Err(format!("{version}: not a tree version (3_28)"));
    }
    let pins_path = pins;
    let pins: Pins = serde_json::from_slice(&read(pins)?).map_err(|e| format!("pins: {e}"))?;
    let pin = pins
        .versions
        .get(version)
        .ok_or_else(|| format!("no pin for {version} in the manifest's versions"))?;
    let head = String::from_utf8_lossy(&read(&export.join(".git").join("HEAD"))?)
        .trim()
        .to_owned();
    if head != pin.commit {
        return Err(format!(
            "the export is at {head}, the pin is {} ({}): run tools/tree/acquire {version}",
            pin.commit, pin.tag
        ));
    }
    let bytes = read(&export.join("data.json"))?;
    let data: Value = serde_json::from_slice(&bytes).map_err(|e| format!("data.json: {e}"))?;
    let source = crate::DirSource(bundle);
    if !loom_data::bundle::validate::validate(&source).is_valid() {
        return Err(format!(
            "{}: the bundle does not validate",
            bundle.display()
        ));
    }
    let loaded = Bundle::load(&source).map_err(|e| format!("{}: {e}", bundle.display()))?;
    // The reviewed rewordings beside the manifest.
    let table = pins_path.with_file_name("rewordings.json");
    let reviewed: Reviewed =
        serde_json::from_slice(&read(&table)?).map_err(|e| format!("{}: {e}", table.display()))?;
    let Converted {
        pack,
        reworded,
        unreviewed,
    } = convert(
        &data,
        &loaded,
        version,
        PackSource {
            repository: pins.export.repository,
            tag: pin.tag.clone(),
            commit: pin.commit.clone(),
            file: ExportFile {
                path: "data.json".to_owned(),
                sha256: hex(&Sha256::digest(&bytes)),
            },
        },
        &reviewed,
    )?;
    let own_slots = &loaded
        .passives
        .as_ref()
        .ok_or("the bundle has no passive tables")?
        .tree
        .orbit_slots;
    if pack.tree.orbit_slots != *own_slots {
        return Err(format!(
            "the export's orbits {:?} differ from the bundle's {own_slots:?}",
            pack.tree.orbit_slots
        ));
    }
    let lines = line_count(&data);
    let over_threshold = pack.unresolved.len() * 100 > lines * MAX_UNREAD_PERCENT;
    if over_threshold && !accept_unread {
        return Err(format!(
            "{version}: {} of {lines} lines did not read, over {MAX_UNREAD_PERCENT}%: \
             the version ships without its pack unless the maintainer accepts it \
             (--accept-unread)",
            pack.unresolved.len()
        ));
    }
    let mut json = serde_json::to_vec_pretty(&pack).map_err(|e| e.to_string())?;
    json.push(b'\n');
    let added = BTreeMap::from([(tree_packs::path(version), json.clone())]);
    let manifest = write_copy(bundle, loaded.manifest.clone(), &added, |_| false, out)?;
    Ok(Summary {
        version: version.to_owned(),
        tag: pin.tag.clone(),
        commit: pin.commit.clone(),
        passives: pack.passives.len(),
        mastery_effects: pack.mastery_effects.len(),
        groups: pack.tree.groups.len(),
        unresolved: pack.unresolved.len(),
        lines,
        over_threshold,
        reworded,
        unreviewed,
        pack_sha256: hex(&Sha256::digest(&json)),
        manifest_sha256: hex(&Sha256::digest(&manifest)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bundle() -> Bundle {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../crates/loom-data/tests/fixtures/bundle-poe1-tree");
        Bundle::load(&crate::DirSource(&dir)).unwrap()
    }

    fn source() -> PackSource {
        PackSource {
            repository: "synthetic".to_owned(),
            tag: "synthetic".to_owned(),
            commit: "0".repeat(40),
            file: ExportFile {
                path: "data.json".to_owned(),
                sha256: "0".repeat(64),
            },
        }
    }

    /// A made-up export in the export's shape: a class start, a node the
    /// bundle has unchanged, one changed with a line the descriptions do
    /// not have, a new node of an ascendancy the bundle does not list, a
    /// node granting a passive point, and a mastery with one known and
    /// one new effect.
    fn export() -> Value {
        serde_json::json!({
            "constants": {"skillsPerOrbit": [1, 6, 16, 16]},
            "groups": {
                "10": {"x": 5.0, "y": 6.0, "nodes": []},
                "2": {"x": 3.0, "y": 4.0, "nodes": ["10000", "10001", "77777", "10029", "11000"]},
                "1": {"x": 1.0, "y": 2.0, "nodes": ["1000"]}
            },
            "nodes": {
                "root": {"out": ["1000", "99"], "in": []},
                "1000": {"skill": 1000, "name": "Brute Start", "classStartIndex": 1, "stats": [],
                         "group": 1, "orbit": 0, "orbitIndex": 0, "out": ["10000"], "in": []},
                "10000": {"skill": 10000, "name": "Test Life", "stats": ["+5 to Test Life"],
                          "group": 2, "orbit": 1, "orbitIndex": 0, "out": ["10001"], "in": []},
                "10001": {"skill": 10001, "name": "Test Life", "stats": ["+9 to Test Life\nOld wording"],
                          "group": 2, "orbit": 1, "orbitIndex": 1, "out": [], "in": []},
                "77777": {"skill": 77777, "name": "New Node", "ascendancyName": "Warlock",
                          "stats": ["+3 to Test Mastery"], "group": 2, "orbit": 1, "orbitIndex": 2,
                          "out": [], "in": []},
                "11000": {"skill": 11000, "name": "Point", "grantedPassivePoints": 1,
                          "stats": ["Grants 1 Passive Skill Point"], "group": 2, "orbit": 1,
                          "orbitIndex": 3, "out": [], "in": []},
                "10029": {"skill": 10029, "name": "Test Life Mastery", "isMastery": true, "stats": [],
                          "masteryEffects": [
                              {"effect": 4242, "stats": ["+5 to Test Mastery"]},
                              {"effect": 9999, "stats": ["+2 to Test Life"]}
                          ],
                          "group": 2, "orbit": 0, "orbitIndex": 0, "out": [], "in": ["10000"]}
            }
        })
    }

    #[test]
    fn template_should_take_signed_and_decimal_numbers_and_fill_them_back() {
        let (t, n) = template("+20 to Dexterity, 0.5% of Mana and 3-4 Fire");
        assert_eq!(t, "# to Dexterity, #% of Mana and #-# Fire");
        assert_eq!(n, ["+20", "0.5", "3", "4"]);
        assert_eq!(fill(&t, &n), "+20 to Dexterity, 0.5% of Mana and 3-4 Fire");
    }

    #[test]
    fn rewordings_should_pair_one_changed_line_with_equal_numbers() {
        let s = |v: &[&str]| v.iter().map(|x| (*x).to_owned()).collect::<Vec<_>>();
        let pairs = vec![
            // Reordered, one line reworded: paired.
            (
                s(&[
                    "15% increased Global Accuracy Rating",
                    "5% increased Movement Speed",
                ]),
                s(&[
                    "5% increased Movement Speed",
                    "15% increased Accuracy Rating",
                ]),
            ),
            // The numbers changed: not a rewording.
            (
                s(&["10% increased Old Thing"]),
                s(&["12% increased New Thing"]),
            ),
            // Two lines changed: not paired.
            (s(&["1 A", "2 B"]), s(&["1 C", "2 D"])),
            // A template mapped two ways: left out.
            (s(&["3 X"]), s(&["3 Y"])),
            (s(&["4 X"]), s(&["4 Z"])),
        ];
        let r = rewordings(&pairs);
        assert_eq!(
            r,
            BTreeMap::from([(
                "#% increased Global Accuracy Rating".to_owned(),
                "#% increased Accuracy Rating".to_owned()
            )])
        );
    }

    #[test]
    fn line_count_should_count_each_mastery_effect_once() {
        let data = serde_json::json!({"nodes": {
            "root": {"stats": ["Not a line"]},
            "1": {"stats": ["+1 to Test Life", "A\nB"]},
            "2": {"masteryEffects": [{"effect": 9, "stats": ["E"]}]},
            "3": {"masteryEffects": [{"effect": 9, "stats": ["E"]}, {"effect": 8, "stats": ["F\n"]}]}
        }});
        assert_eq!(line_count(&data), 5);
    }

    #[test]
    fn convert_should_read_old_wordings_only_as_the_review_accepts() {
        // Node 10000 in old wording, which the bundle prints "+5 to Test
        // Life": the pair is learned; the new node 77777 has the same old
        // wording.
        let mut data = export();
        data["nodes"]["10000"]["stats"] = serde_json::json!(["+5 to Old Test Life"]);
        data["nodes"]["77777"]["stats"] = serde_json::json!(["+8 to Old Test Life"]);
        let pair = ("# to Old Test Life".to_owned(), "# to Test Life".to_owned());
        let bundle = bundle();
        let life = |c: &Converted| {
            c.pack
                .passives
                .iter()
                .find(|p| p.node == 77777)
                .unwrap()
                .stats
                .iter()
                .map(|s| (s.stat.clone(), s.value))
                .collect::<Vec<_>>()
        };

        // Not reviewed: listed, the line kept unread.
        let c = convert(&data, &bundle, "3_28", source(), &Reviewed::default()).unwrap();
        assert_eq!(c.unreviewed, BTreeMap::from([pair.clone()]));
        assert!(c.reworded.is_empty());
        assert!(life(&c).is_empty());
        assert!(
            c.pack
                .unresolved
                .iter()
                .any(|u| u.text == "+8 to Old Test Life")
        );

        // Accepted: read in its current wording.
        let accepted = Reviewed {
            accepted: vec![Rewording {
                old: pair.0.clone(),
                current: pair.1.clone(),
            }],
            rejected: Vec::new(),
        };
        let c = convert(&data, &bundle, "3_28", source(), &accepted).unwrap();
        assert_eq!(c.reworded, BTreeMap::from([pair.clone()]));
        assert_eq!(life(&c), [("test_passive_life".to_owned(), 8)]);
        assert!(
            c.pack
                .unresolved
                .iter()
                .all(|u| !u.text.contains("Old Test"))
        );

        // Rejected: neither applied nor listed.
        let rejected = Reviewed {
            accepted: Vec::new(),
            rejected: accepted.accepted,
        };
        let c = convert(&data, &bundle, "3_28", source(), &rejected).unwrap();
        assert!(c.unreviewed.is_empty() && c.reworded.is_empty());
        assert!(life(&c).is_empty());
    }

    #[test]
    fn convert_should_read_each_line_as_stat_ids_or_keep_it() {
        let bundle = bundle();
        let pack = convert(&export(), &bundle, "3_28", source(), &Reviewed::default())
            .unwrap()
            .pack;
        let by: BTreeMap<i64, &Passive> = pack.passives.iter().map(|p| (p.node, p)).collect();
        let stat = |n: i64| -> Vec<(&str, i64)> {
            by[&n]
                .stats
                .iter()
                .map(|s| (s.stat.as_str(), s.value))
                .collect()
        };
        // Unchanged: the bundle's row; changed: read anew, the line the
        // descriptions lack kept.
        assert_eq!(by[&10000].id, "t10000");
        assert_eq!(stat(10000), [("test_passive_life", 5)]);
        assert_eq!(stat(10001), [("test_passive_life", 9)]);
        assert_eq!(
            pack.unresolved,
            [PackLine {
                node: 10001,
                mastery_effect: false,
                text: "Old wording".to_owned(),
                reason: "no-template".to_owned(),
            }]
        );
        // New: its own ID, no ascendancy the bundle lacks.
        assert_eq!(by[&77777].id, "export77777");
        assert_eq!(by[&77777].ascendancy, None);
        assert_eq!(stat(77777), [("test_mastery_stat", 3)]);
        // The class start by number; the point line is the field.
        assert_eq!(by[&1000].characters, ["Test/Chars/Str"]);
        assert_eq!((by[&11000].skill_points, stat(11000)), (1, vec![]));
        // The mastery keeps the bundle's group with the export's effects.
        assert_eq!(by[&10029].mastery_group.as_deref(), Some("TestLifeMastery"));
        assert_eq!(
            pack.mastery_groups[0].effects,
            ["TestLifeMastery1", "export9999"]
        );
        let effects: Vec<(i64, &str, i64)> = pack
            .mastery_effects
            .iter()
            .map(|e| (e.hash, e.stats[0].stat.as_str(), e.stats[0].value))
            .collect();
        assert_eq!(
            effects,
            [
                (4242, "test_mastery_stat", 5),
                (9999, "test_passive_life", 2)
            ]
        );
        // Groups in their numbers' order; links and roots only to nodes it
        // has.
        let centres: Vec<(f32, f32)> = pack.tree.groups.iter().map(|g| (g.x, g.y)).collect();
        assert_eq!(centres, [(1.0, 2.0), (3.0, 4.0), (5.0, 6.0)]);
        assert_eq!(pack.tree.roots, [1000]);
        assert_eq!(pack.tree.groups[0].nodes[0].connections[0].node, 10000);
        assert_eq!(pack.tree.orbit_slots, [1, 6, 16, 16]);
    }
}
