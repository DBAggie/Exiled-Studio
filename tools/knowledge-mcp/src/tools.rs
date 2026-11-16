//! The tool list (contract `schemas/knowledge-mcp-tools.v1.json`, rewritten
//! by `LOOM_WRITE_KNOWLEDGE_CONTRACT=1 cargo test -p loom-knowledge-mcp
//! --test contract`) and the calls.

use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::knowledge::Knowledge;
use crate::lookup::{self, GetPrice, SearchMods};

/// The MCP protocol version, the app MCP's (`loom_integrations::mcp`; a test
/// keeps the two equal).
pub const PROTOCOL_VERSION: &str = "2025-06-18";

/// The contract's version.
pub const CONTRACT_VERSION: u32 = 1;

/// A failed call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallError {
    /// Bad arguments (JSON-RPC -32602).
    Params(String),
    /// The tool ran and failed (a result with `isError`).
    Tool(ToolError),
}

/// A tool's failure, with a stable code (ADR-014).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolError {
    /// `KNOWLEDGE_*`, or `PARAMS` for bad arguments.
    pub code: &'static str,
    /// Why, with no file path.
    pub message: String,
}

impl ToolError {
    /// A tool failure.
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    /// Bad arguments.
    pub fn params(message: impl Into<String>) -> Self {
        Self::new("PARAMS", message)
    }
}

impl From<ToolError> for CallError {
    fn from(e: ToolError) -> Self {
        if e.code == "PARAMS" {
            Self::Params(e.message)
        } else {
            Self::Tool(e)
        }
    }
}

fn read_only() -> Value {
    json!({"readOnlyHint": true, "openWorldHint": false})
}

fn name_arg(what: &str) -> Value {
    json!({"type": "object", "required": ["name"], "additionalProperties": false,
        "properties": {"name": {"type": "string", "minLength": 1, "description": what}}})
}

/// The tool list (`tools/list`'s `tools`), the committed contract.
#[must_use]
pub fn list() -> Value {
    let provenance = "Every result carries `provenance`: game, patch, Steam build, the bundle's manifest SHA-256 and index SHA-256, and whether it is signed by a trusted key.";
    json!([
        {
            "name": "get_snapshot_info",
            "title": "What data is loaded",
            "description": format!("The game, the loaded bundle's provenance (or why none is loaded), the stat-description scopes, and every economy snapshot (league, poe.ninja league ID, as_of, patch, verification, status) with the files skipped and why. Call first. {provenance} Read-only."),
            "inputSchema": {"type": "object", "properties": {}, "additionalProperties": false},
            "annotations": read_only()
        },
        {
            "name": "search_mods",
            "title": "Search mods",
            "description": format!("Mod rows matching every filter given: `text` in the mod ID, affix name, or game text; part of a `stat` ID; a spawn or implicit `tag`; `domain`; `generation`. Each mod: ID, affix name, level, domain, generation, stats with ranges (stat IDs, ADR-015), the game text, spawn tags. At most `limit` (default 25, at most 100), with the total. {provenance} Read-only."),
            "inputSchema": {"type": "object", "additionalProperties": false, "properties": {
                "text": {"type": "string", "minLength": 1},
                "stat": {"type": "string", "minLength": 1},
                "tag": {"type": "string", "minLength": 1},
                "domain": {"type": "integer"},
                "generation": {"type": "integer"},
                "limit": {"type": "integer", "minimum": 1, "maximum": 100}
            }},
            "annotations": read_only()
        },
        {
            "name": "get_gem",
            "title": "Get a gem",
            "description": format!("A skill or support gem by name: tags, attribute split, its granted effect (support or not, cast time, skill types, quality stats) and, per level asked (`levels`, default [1, 20]; [] for all), costs, cooldown, reservation, and stat-set values (stat IDs). {provenance} Read-only."),
            "inputSchema": {"type": "object", "required": ["name"], "additionalProperties": false, "properties": {
                "name": {"type": "string", "minLength": 1, "description": "The gem's name, any case."},
                "levels": {"type": "array", "items": {"type": "integer", "minimum": 1}, "maxItems": 40}
            }},
            "annotations": read_only()
        },
        {
            "name": "get_unique",
            "title": "Get a unique",
            "description": format!("A unique item by name: its ID and flavour text. The game's client data does not list a unique's mods, so `mods` is null; read the item's own lines instead. {provenance} Read-only."),
            "inputSchema": name_arg("The unique's name, any case."),
            "annotations": read_only()
        },
        {
            "name": "get_base",
            "title": "Get an item base",
            "description": format!("Every base item row with the name: class, tags, drop and required level, attribute requirements, implicits (stat IDs and game text), weapon, armour, or shield properties, and socket caps by item level. {provenance} Read-only."),
            "inputSchema": name_arg("The base's name (or ID), any case."),
            "annotations": read_only()
        },
        {
            "name": "get_passive",
            "title": "Get a passive",
            "description": format!("Passive nodes by `name` (every node with it) or by `node` ID, one of the two: kind (small, notable, keystone, mastery, jewel socket, ascendancy start), ascendancy, stats (stat IDs) and game text, a mastery's effects, flavour text. {provenance} Read-only."),
            "inputSchema": {"type": "object", "additionalProperties": false, "properties": {
                "name": {"type": "string", "minLength": 1},
                "node": {"type": "integer"}
            }},
            "annotations": read_only()
        },
        {
            "name": "resolve_stat",
            "title": "Resolve text to stat IDs",
            "description": format!("Item, passive, or gem text to (stat ID, value) through the stat descriptions (Spike B's resolver): each line or multi-line template, its outcome (resolved, ambiguous, unresolved, with the reason), every candidate's stats, and shadowed matches. Unresolved text is returned as unresolved, never dropped (ADR-015). `scope`: item (default), passive, gem, or a stat-description file name. {provenance} Read-only."),
            "inputSchema": {"type": "object", "required": ["text"], "additionalProperties": false, "properties": {
                "text": {"type": "string", "minLength": 1, "maxLength": 20000},
                "scope": {"type": "string", "minLength": 1}
            }},
            "annotations": read_only()
        },
        {
            "name": "get_price",
            "title": "Get a price",
            "description": "An item's or currency's price from the newest economy snapshot of the league (`league`: name or poe.ninja ID; default the newest current league), optionally narrowed by `variant` and `links`: chaos and divine values, volume or listings, with the snapshot's league, as_of, fetched time, patch, verification, and `signed: false`. Refuses another game's snapshots, never falls back to another league, and refuses when the loaded bundle's patch is unknown (not in the patch table); a patch mismatch is a warning. Only listed fields of a snapshot row are returned. State `as_of` and the league when quoting a price. Read-only; no network.",
            "inputSchema": {"type": "object", "required": ["name"], "additionalProperties": false, "properties": {
                "name": {"type": "string", "minLength": 1},
                "league": {"type": "string", "minLength": 1},
                "variant": {"type": "string", "minLength": 1},
                "links": {"type": "integer", "minimum": 0, "maximum": 6}
            }},
            "annotations": read_only()
        }
    ])
}

fn params<T: DeserializeOwned>(args: &Value) -> Result<T, CallError> {
    let args = if args.is_null() {
        json!({})
    } else {
        args.clone()
    };
    serde_json::from_value(args).map_err(|e| CallError::Params(e.to_string()))
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Name {
    name: String,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Gem {
    name: String,
    levels: Option<Vec<i64>>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Passive {
    name: Option<String>,
    node: Option<i64>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Resolve {
    text: String,
    scope: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Empty {}

/// Calls tool `name` with `args`.
///
/// # Errors
/// [`CallError`].
pub fn call(k: &Knowledge, name: &str, args: &Value) -> Result<Value, CallError> {
    let game = k.game.name();
    if name == "get_snapshot_info" {
        let _: Empty = params(args)?;
        return Ok(snapshot_info(k));
    }
    if name == "get_price" {
        let a: GetPrice = params(args)?;
        if a.links.is_some_and(|l| !(0..=6).contains(&l)) {
            return Err(CallError::Params("links: 0 to 6".to_owned()));
        }
        let patch = k.data.as_ref().ok().map(|d| d.provenance.patch.as_str());
        let mut out = lookup::get_price(game, patch, &k.snapshots, &a)?;
        if let (Some(o), Ok(d)) = (out.as_object_mut(), k.data.as_ref()) {
            o.insert("provenance".to_owned(), json!(d.provenance));
        }
        return Ok(out);
    }
    let known = [
        "search_mods",
        "get_gem",
        "get_unique",
        "get_base",
        "get_passive",
        "resolve_stat",
    ];
    if !known.contains(&name) {
        return Err(CallError::Params(format!("unknown tool: {name}")));
    }
    let data = k.data.as_ref().map_err(|e| {
        CallError::Tool(ToolError::new(
            e.code(),
            format!("{e}; game data lookups are unavailable"),
        ))
    })?;
    let result = match name {
        "search_mods" => {
            let a: SearchMods = params(args)?;
            if a.limit
                .is_some_and(|l| !(1..=lookup::MAX_LIMIT).contains(&l))
            {
                return Err(CallError::Params(format!(
                    "limit: 1 to {}",
                    lookup::MAX_LIMIT
                )));
            }
            lookup::search_mods(data, &a)?
        }
        "get_gem" => {
            let a: Gem = params(args)?;
            let levels = a.levels.unwrap_or_else(|| vec![1, 20]);
            if levels.len() > 40 || levels.iter().any(|l| *l < 1) {
                return Err(CallError::Params(
                    "levels: at most 40, each at least 1".to_owned(),
                ));
            }
            lookup::get_gem(data, &a.name, &levels)?
        }
        "get_unique" => lookup::get_unique(data, &params::<Name>(args)?.name)?,
        "get_base" => lookup::get_base(data, &params::<Name>(args)?.name)?,
        "get_passive" => {
            let a: Passive = params(args)?;
            if a.name.is_some() == a.node.is_some() {
                return Err(CallError::Params("give name or node, not both".to_owned()));
            }
            lookup::get_passive(data, a.name.as_deref(), a.node)?
        }
        _ => {
            let a: Resolve = params(args)?;
            if a.text.chars().count() > 20_000 {
                return Err(CallError::Params(
                    "text: at most 20000 characters".to_owned(),
                ));
            }
            lookup::resolve_stat(data, &a.text, a.scope.as_deref())?
        }
    };
    Ok(json!({"provenance": data.provenance, "result": result}))
}

fn snapshot_info(k: &Knowledge) -> Value {
    let (provenance, scopes, error) = match &k.data {
        Ok(d) => (
            json!(d.provenance),
            json!(lookup::scope_names(d.bundle)),
            Value::Null,
        ),
        Err(e) => (
            Value::Null,
            json!([]),
            json!({"code": e.code(), "message": e.to_string()}),
        ),
    };
    let snapshots: Vec<Value> = k
        .snapshots
        .items
        .iter()
        .map(|s| {
            json!({
                "file": s.file,
                "game": s.header.game,
                "league": s.header.league,
                "league_id": s.league_id,
                "as_of": s.header.as_of,
                "fetched_utc": s.fetched_utc,
                "patch": s.header.patch,
                "verification": s.header.verification,
                "status": s.header.status,
            })
        })
        .collect();
    json!({
        "game": k.game.name(),
        "contract_version": CONTRACT_VERSION,
        "provenance": provenance,
        "bundle_error": error,
        "scopes": scopes,
        "snapshots": snapshots,
        "snapshots_skipped": k.snapshots.skipped,
    })
}
