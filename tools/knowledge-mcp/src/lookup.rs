//! The tools' answers from the bundle and the snapshots.

use std::collections::BTreeMap;

use loom_data::bundle::{BaseItem, Bundle, ModEntry};
use loom_data::stat_desc::render::{StatValue, VariantFilter};
use loom_data::stat_desc::scope::Scope;
use loom_parser::resolve::{Candidate, Outcome, RawValue, Unresolved};
use serde_json::{Value, json};

use crate::knowledge::Loaded;
use crate::snapshots::Snapshots;
use crate::tools::ToolError;

/// The most results a search returns.
pub const MAX_LIMIT: usize = 100;

/// The default number of results.
pub const DEFAULT_LIMIT: usize = 25;

/// Scope aliases to the stat-description files tried, in order.
const SCOPE_ALIASES: &[(&str, &[&str])] = &[
    ("passive", &["passive_skill_stat_descriptions"]),
    (
        "gem",
        &[
            "active_skill_gem_stat_descriptions",
            "gem_stat_descriptions",
            "skill_stat_descriptions",
        ],
    ),
];

fn not_found(what: &str, name: &str) -> ToolError {
    ToolError::new(
        "KNOWLEDGE_NOT_FOUND",
        format!("no {what} named {name:?} in this data"),
    )
}

fn eq(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

fn contains(hay: &str, needle: &str) -> bool {
    hay.to_lowercase().contains(&needle.to_lowercase())
}

/// The stat-description file a scope name or alias stands for.
fn scope_root<'b>(bundle: &'b Bundle, scope: Option<&str>) -> Result<&'b str, ToolError> {
    let name = scope.unwrap_or("item");
    let tries: Vec<&str> = if eq(name, "item") {
        vec![bundle.item_rules.default_scope.as_str()]
    } else if let Some((_, files)) = SCOPE_ALIASES.iter().find(|(a, _)| eq(a, name)) {
        files.to_vec()
    } else {
        vec![name]
    };
    tries
        .iter()
        .find_map(|t| bundle.scope_root(t).ok())
        .ok_or_else(|| {
            ToolError::new(
                "KNOWLEDGE_SCOPE",
                format!(
                    "no stat-description scope {name:?}; use item, passive, gem, or one of: {}",
                    scope_names(bundle).join(", ")
                ),
            )
        })
}

/// The stat-description files' names (without folder or extension).
pub fn scope_names(bundle: &Bundle) -> Vec<String> {
    let mut out: Vec<String> = bundle
        .stat_descriptions
        .files()
        .map(|(path, _)| {
            let file = path.rsplit('/').next().unwrap_or(path);
            file.split('.').next().unwrap_or(file).to_owned()
        })
        .collect();
    out.sort();
    out.dedup();
    out
}

/// The game's text for stats, as one scope's descriptions write it (the
/// scope is built once per call: building it is the slow part).
struct Text<'b> {
    bundle: &'b Bundle,
    scope: Option<Scope<'b>>,
}

impl<'b> Text<'b> {
    fn new(bundle: &'b Bundle, root: &str) -> Self {
        Self {
            bundle,
            scope: bundle.stat_descriptions.scope(root).ok(),
        }
    }

    /// A mod's game text, one string per line (none without a scope).
    fn lines(&self, m: &ModEntry) -> Vec<String> {
        let Some(scope) = &self.scope else {
            return Vec::new();
        };
        let map: BTreeMap<String, StatValue> = m
            .stats
            .iter()
            .map(|s| {
                (
                    s.stat.clone(),
                    StatValue {
                        min: s.min,
                        max: s.max,
                    },
                )
            })
            .collect();
        scope
            .render(&map, self.bundle, VariantFilter::default())
            .lines
            .into_iter()
            .map(|l| l.text)
            .collect()
    }

    fn render(&self, stats: &[(String, i64, i64)]) -> Value {
        let Some(scope) = &self.scope else {
            return json!({"lines": [], "not_rendered": stats.iter().map(|s| &s.0).collect::<Vec<_>>()});
        };
        let map: BTreeMap<String, StatValue> = stats
            .iter()
            .map(|(id, min, max)| {
                (
                    id.clone(),
                    StatValue {
                        min: *min,
                        max: *max,
                    },
                )
            })
            .collect();
        let rendered = scope.render(&map, self.bundle, VariantFilter::default());
        json!({
            "lines": rendered.lines.iter().map(|l| &l.text).collect::<Vec<_>>(),
            "not_rendered": rendered.not_rendered.iter().map(|n| &n.stat).collect::<Vec<_>>(),
        })
    }
}

fn mod_json(text: &Text<'_>, m: &ModEntry) -> Value {
    let stats: Vec<(String, i64, i64)> = m
        .stats
        .iter()
        .map(|s| (s.stat.clone(), s.min, s.max))
        .collect();
    json!({
        "id": m.id,
        "name": m.name,
        "level": m.level,
        "domain": m.domain,
        "generation": m.generation,
        "mod_type": m.mod_type,
        "stats": m.stats,
        "text": text.render(&stats),
        "spawn_tags": m.spawn.iter().filter(|w| w.weight > 0).map(|w| &w.tag).collect::<Vec<_>>(),
        "implicit_tags": m.implicit_tags,
    })
}

/// `search_mods`.
pub fn search_mods(data: &Loaded, args: &SearchMods) -> Result<Value, ToolError> {
    if args.text.is_none() && args.stat.is_none() && args.tag.is_none() {
        return Err(ToolError::params("give at least one of text, stat, tag"));
    }
    let bundle = data.bundle;
    let text = Text::new(bundle, scope_root(bundle, None)?);
    let limit = args.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let needle = args.text.as_deref().map(str::to_lowercase);
    // Rendering every mod's text is the slow part: it is done once per
    // process, and only when a call filters on text.
    let lines = needle.as_ref().map(|_| {
        data.mod_text(|| {
            bundle
                .mods
                .iter()
                .map(|m| text.lines(m).join("\n").to_lowercase())
                .collect()
        })
    });
    let mut total = 0usize;
    let mut out = Vec::new();
    for (i, m) in bundle.mods.iter().enumerate() {
        if args.domain.is_some_and(|d| d != m.domain)
            || args.generation.is_some_and(|g| g != m.generation)
            || args
                .stat
                .as_deref()
                .is_some_and(|s| !m.stats.iter().any(|x| contains(&x.stat, s)))
            || args.tag.as_deref().is_some_and(|t| {
                !m.spawn.iter().any(|w| w.weight > 0 && eq(&w.tag, t))
                    && !m.implicit_tags.iter().any(|x| eq(x, t))
            })
        {
            continue;
        }
        if let Some(needle) = needle.as_deref() {
            let in_lines = lines
                .and_then(|l| l.get(i))
                .is_some_and(|l| l.contains(needle));
            if !(m.id.to_lowercase().contains(needle)
                || m.name.to_lowercase().contains(needle)
                || in_lines)
            {
                continue;
            }
        }
        total += 1;
        if out.len() < limit {
            out.push(mod_json(&text, m));
        }
    }
    Ok(json!({"total": total, "mods": out}))
}

/// `search_mods` arguments.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchMods {
    /// Text in the mod's ID, affix name, or game text.
    pub text: Option<String>,
    /// Part of a stat ID.
    pub stat: Option<String>,
    /// A spawn or implicit tag.
    pub tag: Option<String>,
    /// The mod domain.
    pub domain: Option<i64>,
    /// The generation type.
    pub generation: Option<i64>,
    /// At most this many.
    pub limit: Option<usize>,
}

/// `get_gem`.
pub fn get_gem(data: &Loaded, name: &str, levels: &[i64]) -> Result<Value, ToolError> {
    let bundle = data.bundle;
    let skills = bundle
        .skills
        .as_ref()
        .ok_or_else(|| ToolError::new("KNOWLEDGE_NO_TABLE", "this bundle has no skill tables"))?;
    let effect = skills
        .gem_effects
        .iter()
        .find(|e| eq(&e.name, name) || eq(&e.id, name))
        .ok_or_else(|| {
            let close: Vec<&str> = skills
                .gem_effects
                .iter()
                .filter(|e| contains(&e.name, name))
                .take(10)
                .map(|e| e.name.as_str())
                .collect();
            ToolError::new(
                "KNOWLEDGE_NOT_FOUND",
                format!("no gem named {name:?}; close: {}", close.join(", ")),
            )
        })?;
    let gem = skills.gems.iter().find(|g| g.effects.contains(&effect.id));
    let tag_names: Vec<&str> = effect
        .tags
        .iter()
        .map(|t| {
            skills
                .gem_tags
                .iter()
                .find(|g| &g.id == t)
                .filter(|g| !g.name.is_empty())
                .map_or(t.as_str(), |g| g.name.as_str())
        })
        .collect();
    let granted = effect
        .granted_effect
        .as_deref()
        .and_then(|id| skills.granted_effects.iter().find(|g| g.id == id));
    let pick = |level: i64| levels.is_empty() || levels.contains(&level);
    let granted_json = granted.map(|g| {
        let stat_sets: Vec<Value> = g
            .stat_sets
            .iter()
            .filter_map(|id| skills.stat_sets.iter().find(|s| &s.id == id))
            .map(|s| {
                json!({
                    "id": s.id,
                    "constant_stats": s.constant_stats,
                    "implicit_stats": s.implicit_stats,
                    "levels": s.levels.iter().filter(|l| pick(l.level)).collect::<Vec<_>>(),
                })
            })
            .collect();
        let active = g
            .active_skill
            .as_deref()
            .and_then(|id| skills.active_skills.iter().find(|a| a.id == id));
        json!({
            "id": g.id,
            "support": g.support,
            "cast_time_ms": g.cast_time,
            "active_skill": active,
            "allowed_types": g.allowed_types,
            "added_types": g.added_types,
            "excluded_types": g.excluded_types,
            "quality": g.quality,
            "level_count": g.levels.len(),
            "levels": g.levels.iter().filter(|l| pick(l.level)).collect::<Vec<_>>(),
            "stat_sets": stat_sets,
        })
    });
    Ok(json!({
        "name": effect.name,
        "id": effect.id,
        "tags": tag_names,
        "gem": gem,
        "granted_effect": granted_json,
        "levels_shown": if levels.is_empty() { json!("all") } else { json!(levels) },
    }))
}

/// `get_unique`.
pub fn get_unique(data: &Loaded, name: &str) -> Result<Value, ToolError> {
    let flavour = data
        .bundle
        .poe1
        .as_ref()
        .and_then(|p| p.unique_flavour.as_ref())
        .and_then(|u| u.iter().find(|f| eq(&f.name, name)))
        .ok_or_else(|| not_found("unique", name))?;
    Ok(json!({
        "name": flavour.name,
        "id": flavour.id,
        "flavour_text": flavour.text,
        "mods": null,
        "note": "The game's client data does not list a unique's mods, so this data has none. Read the item's lines from the item itself (the app MCP's get_item, or resolve_stat on its text).",
    }))
}

fn base_json(bundle: &Bundle, text: &Text<'_>, b: &BaseItem) -> Value {
    let implicits: Vec<Value> = b
        .implicits
        .iter()
        .filter_map(|id| bundle.mods.iter().find(|m| &m.id == id))
        .map(|m| mod_json(text, m))
        .collect();
    let props = bundle.properties.as_ref();
    let caps = bundle
        .poe1
        .as_ref()
        .and_then(|p| p.socket_caps.as_ref())
        .and_then(|c| c.iter().find(|c| c.base == b.id));
    json!({
        "id": b.id,
        "name": b.name,
        "item_class": b.item_class,
        "tags": b.tags,
        "drop_level": b.drop_level,
        "required_level": bundle.required_level(b),
        "requirements": b.requirements,
        "implicits": implicits,
        "weapon": props.and_then(|p| p.weapon_types.iter().find(|w| w.base_item == b.id)),
        "armour": props.and_then(|p| p.armour_types.iter().find(|a| a.base_item == b.id)),
        "shield": props.and_then(|p| p.shield_types.iter().find(|s| s.base_item == b.id)),
        "socket_caps": caps.map(|c| &c.levels),
    })
}

/// `get_base`: every row with the name (several rows may share one).
pub fn get_base(data: &Loaded, name: &str) -> Result<Value, ToolError> {
    let bundle = data.bundle;
    let text = Text::new(bundle, scope_root(bundle, None)?);
    let rows: Vec<Value> = bundle
        .base_items
        .iter()
        .filter(|b| eq(&b.name, name) || eq(&b.id, name))
        .map(|b| base_json(bundle, &text, b))
        .collect();
    if rows.is_empty() {
        return Err(not_found("base item", name));
    }
    Ok(json!({"bases": rows}))
}

/// `get_passive`: by name (every node with it) or by node ID.
pub fn get_passive(
    data: &Loaded,
    name: Option<&str>,
    node: Option<i64>,
) -> Result<Value, ToolError> {
    let bundle = data.bundle;
    let tables = bundle
        .passives
        .as_ref()
        .ok_or_else(|| ToolError::new("KNOWLEDGE_NO_TABLE", "this bundle has no passive tables"))?;
    let root = scope_root(bundle, Some("passive")).or_else(|_| scope_root(bundle, None))?;
    let text = Text::new(bundle, root);
    let flavour = bundle.poe1.as_ref().and_then(|p| p.tree_flavour.as_ref());
    let rows: Vec<Value> = tables
        .passives
        .iter()
        .filter(|p| match (name, node) {
            (_, Some(n)) => p.node == n,
            (Some(n), None) => eq(&p.name, n) || eq(&p.id, n),
            (None, None) => false,
        })
        .map(|p| {
            let stats: Vec<(String, i64, i64)> = p
                .stats
                .iter()
                .map(|s| (s.stat.clone(), s.value, s.value))
                .collect();
            let kind = if p.keystone {
                "keystone"
            } else if p.notable {
                "notable"
            } else if p.jewel_socket {
                "jewel-socket"
            } else if p.mastery_group.is_some() {
                "mastery"
            } else if p.ascendancy_start {
                "ascendancy-start"
            } else {
                "small"
            };
            let mastery: Vec<Value> = p
                .mastery_group
                .as_deref()
                .and_then(|g| tables.mastery_groups.iter().find(|m| m.id == g))
                .map(|g| {
                    g.effects
                        .iter()
                        .filter_map(|e| tables.mastery_effects.iter().find(|m| &m.id == e))
                        .map(|e| {
                            let s: Vec<(String, i64, i64)> = e
                                .stats
                                .iter()
                                .map(|s| (s.stat.clone(), s.value, s.value))
                                .collect();
                            json!({"id": e.id, "hash": e.hash, "stats": e.stats, "text": text.render(&s)})
                        })
                        .collect()
                })
                .unwrap_or_default();
            json!({
                "id": p.id,
                "node": p.node,
                "name": p.name,
                "kind": kind,
                "ascendancy": p.ascendancy,
                "stats": p.stats,
                "text": text.render(&stats),
                "mastery_effects": mastery,
                "flavour_text": flavour.and_then(|f| f.passives.iter().find(|x| x.node == p.node)).map(|x| &x.text),
            })
        })
        .collect();
    if rows.is_empty() {
        let what = name.map_or_else(
            || node.map(|n| n.to_string()).unwrap_or_default(),
            str::to_owned,
        );
        return Err(not_found("passive", &what));
    }
    Ok(json!({"passives": rows}))
}

fn raw_value(v: &RawValue) -> Value {
    match v {
        RawValue::Exact(s) => json!({"min": s.min, "max": s.max}),
        RawValue::Ranged(s, ends) => json!({
            "min": s.min,
            "max": s.max,
            "range": ends.map(|e| json!({"from": e.from, "to": e.to})),
        }),
        RawValue::Hidden { assumed, .. } => json!({"hidden": true, "assumed": assumed}),
        RawValue::Lookup(keys) => json!({"lookup_keys": keys}),
    }
}

fn candidate(c: &Candidate) -> Value {
    json!({
        "stats": c.stats.iter().map(|s| json!({"stat": s.stat, "value": raw_value(&s.value)})).collect::<Vec<_>>(),
        "alternatives": c.alternatives.len(),
    })
}

/// `resolve_stat`: each line (or multi-line template) of `text`.
pub fn resolve_stat(data: &Loaded, text: &str, scope: Option<&str>) -> Result<Value, ToolError> {
    let root = scope_root(data.bundle, scope)?.to_owned();
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    if lines.is_empty() {
        return Err(ToolError::params("text has no lines"));
    }
    let out = data
        .with_index(&root, |index| {
            index
                .resolve_lines(&lines)
                .iter()
                .map(|r| {
                    let (outcome, candidates, reason) = match &r.outcome {
                        Outcome::Resolved(c) => ("resolved", vec![candidate(c)], None),
                        Outcome::Ambiguous(cs) => {
                            ("ambiguous", cs.iter().map(candidate).collect(), None)
                        }
                        Outcome::Unresolved(u) => ("unresolved", vec![], Some(u.name())),
                    };
                    let rejected = match &r.outcome {
                        Outcome::Unresolved(Unresolved::NoVariantFits(rs)) => {
                            rs.iter().map(|x| x.reason.name()).collect::<Vec<_>>()
                        }
                        _ => vec![],
                    };
                    json!({
                        "line": r.line,
                        "lines": r.lines,
                        "outcome": outcome,
                        "candidates": candidates,
                        "unresolved_reason": reason,
                        "rejected": rejected,
                        "shadowed": r.shadowed.iter().map(candidate).collect::<Vec<_>>(),
                    })
                })
                .collect::<Vec<_>>()
        })
        .ok_or_else(|| {
            ToolError::new("KNOWLEDGE_SCOPE", format!("scope {root:?} does not build"))
        })?;
    Ok(json!({"scope": root, "resolutions": out}))
}

/// `get_price` arguments.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GetPrice {
    /// The item's or currency's name.
    pub name: String,
    /// The league's name or poe.ninja ID; the newest current snapshot's when
    /// absent.
    pub league: Option<String>,
    /// poe.ninja's variant text.
    pub variant: Option<String>,
    /// Linked sockets.
    pub links: Option<i64>,
}

/// The fields of a snapshot's item row a price answer carries; the rest of
/// the (unsigned) row is never passed on.
const PRICE_FIELDS: [&str; 13] = [
    "name",
    "baseType",
    "itemClass",
    "variant",
    "links",
    "levelRequired",
    "gemLevel",
    "gemQuality",
    "corrupted",
    "mapTier",
    "chaosValue",
    "divineValue",
    "listingCount",
];

/// The longest string taken from a snapshot row.
const MAX_ROW_STRING: usize = 200;

/// A snapshot value as passed on: strings cut to [`MAX_ROW_STRING`]
/// characters, numbers and booleans as they are, anything else dropped.
fn row_value(v: Option<&Value>) -> Value {
    match v {
        Some(Value::String(s)) => Value::String(s.chars().take(MAX_ROW_STRING).collect()),
        Some(v @ (Value::Number(_) | Value::Bool(_))) => v.clone(),
        _ => Value::Null,
    }
}

/// An item's or currency's price (`get_price`): refuses another game's or
/// another league's prices; a patch mismatch is a warning (ADR-018,
/// "Refusals").
///
/// # Errors
/// `KNOWLEDGE_NO_SNAPSHOT` without a snapshot of the game and league;
/// `KNOWLEDGE_PATCH_UNKNOWN` when the loaded bundle's patch is not in the
/// patch table (ADR-018, "Patch label").
pub fn get_price(
    game: &str,
    data_patch: Option<&str>,
    snapshots: &Snapshots,
    args: &GetPrice,
) -> Result<Value, ToolError> {
    if data_patch == Some(crate::patches::UNKNOWN) {
        return Err(ToolError::new(
            "KNOWLEDGE_PATCH_UNKNOWN",
            "the loaded bundle's Steam build is not in the patch table, so a price cannot be matched to its patch",
        ));
    }
    let snap = snapshots
        .newest(game, args.league.as_deref())
        .ok_or_else(|| {
            let known: Vec<&str> = snapshots
                .items
                .iter()
                .filter(|s| s.header.game == game)
                .map(|s| s.header.league.as_str())
                .collect();
            ToolError::new(
                "KNOWLEDGE_NO_SNAPSHOT",
                format!(
                    "no {game} snapshot for {}; leagues with snapshots: {}",
                    args.league.as_deref().unwrap_or("the current league"),
                    if known.is_empty() {
                        "none".to_owned()
                    } else {
                        known.join(", ")
                    }
                ),
            )
        })?;
    let body = &snap.body;
    let chaos_per_divine = body
        .pointer("/exchange/Currency/rates/divine")
        .and_then(Value::as_f64)
        .filter(|r| *r > 0.0)
        .map(|r| 1.0 / r);
    let mut matches = Vec::new();
    if let Some(groups) = body.get("exchange").and_then(Value::as_object) {
        for (kind, group) in groups {
            for line in group
                .get("lines")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let hit = line
                    .get("name")
                    .and_then(Value::as_str)
                    .is_some_and(|n| eq(n, &args.name))
                    || line
                        .get("id")
                        .and_then(Value::as_str)
                        .is_some_and(|n| eq(n, &args.name));
                if hit && matches.len() < MAX_LIMIT {
                    let chaos = line.get("value").and_then(Value::as_f64);
                    matches.push(json!({
                        "kind": row_value(Some(&json!(kind))),
                        "name": row_value(line.get("name")),
                        "chaos": chaos,
                        "divine": chaos.zip(chaos_per_divine).map(|(c, d)| c / d),
                        "volume": row_value(line.get("volume")),
                    }));
                }
            }
        }
    }
    if let Some(groups) = body.get("items").and_then(Value::as_object) {
        for (kind, rows) in groups {
            for row in rows.as_array().into_iter().flatten() {
                let name_hit = row
                    .get("name")
                    .and_then(Value::as_str)
                    .is_some_and(|n| eq(n, &args.name))
                    || row
                        .get("baseType")
                        .and_then(Value::as_str)
                        .is_some_and(|n| eq(n, &args.name));
                let variant_ok = args.variant.as_deref().is_none_or(|v| {
                    row.get("variant")
                        .and_then(Value::as_str)
                        .is_some_and(|x| eq(x, v))
                });
                let links_ok = args
                    .links
                    .is_none_or(|l| row.get("links").and_then(Value::as_i64).unwrap_or(0) == l);
                if name_hit && variant_ok && links_ok && matches.len() < MAX_LIMIT {
                    let mut m = serde_json::Map::new();
                    m.insert("kind".to_owned(), row_value(Some(&json!(kind))));
                    for f in PRICE_FIELDS {
                        if let Some(v) = row.get(f) {
                            m.insert(f.to_owned(), row_value(Some(v)));
                        }
                    }
                    matches.push(Value::Object(m));
                }
            }
        }
    }
    let mut warnings = Vec::new();
    match data_patch {
        Some(p) if p != snap.header.patch => warnings.push(format!(
            "the snapshot was taken on patch {}, the loaded data is {p}",
            snap.header.patch
        )),
        Some(_) => {}
        None => warnings.push(
            "no game data bundle is loaded, so the snapshot's patch was not checked".to_owned(),
        ),
    }
    if snap.header.status == "historical" {
        warnings.push("historical snapshot: the league has ended".to_owned());
    }
    if matches.is_empty() {
        warnings.push(format!("no price for {:?} in this snapshot", args.name));
    }
    Ok(json!({
        "snapshot": {
            "game": snap.header.game,
            "league": snap.header.league,
            "league_id": snap.league_id,
            "as_of": snap.header.as_of,
            "fetched_utc": snap.fetched_utc,
            "patch": snap.header.patch,
            "verification": snap.header.verification,
            "status": snap.header.status,
            "signed": false,
            "source": "poe.ninja economy overviews, through the knowledge pack's reviewed refresh",
        },
        "chaos_per_divine": chaos_per_divine,
        "matches": matches,
        "warnings": warnings,
    }))
}
