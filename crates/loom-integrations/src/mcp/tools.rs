//! The MCP tools, contract v1 (ADR-017; Phase 4 plan, decision 11). Each
//! result is an application API v1 DTO (`schemas/app-api.v1.schema.json`)
//! or, for the breakdown, a slice of the calc result document v1, so the
//! tools version with the API. No tool takes a file path, saves, or
//! exports (ADR-010), and no result carries a file path.

use loom_app::{App, AppError, api};
use serde_json::{Value, json};

/// The protocol version this contract speaks.
pub const PROTOCOL_VERSION: &str = "2025-06-18";

/// The contract's version.
pub const CONTRACT_VERSION: u32 = 1;

fn item_edit_schema() -> Value {
    json!({
        "type": "object",
        "description": "app-api.v1 ItemEdit: line operations on one item. `expect` gives the current text of every line an operation names (from get_item); the edit is refused as stale (APP_STALE_EDIT) when a line changed.",
        "required": ["item", "ops", "expect"],
        "additionalProperties": false,
        "properties": {
            "item": {"type": "integer", "minimum": 0, "description": "The item's ID (list_items)."},
            "ops": {
                "type": "array",
                "items": {
                    "oneOf": [
                        {"type": "object", "required": ["replace"], "additionalProperties": false, "properties": {"replace": {"type": "object", "required": ["line", "text"], "additionalProperties": false, "properties": {"line": {"type": "integer", "minimum": 0}, "text": {"type": "string"}}}}},
                        {"type": "object", "required": ["remove"], "additionalProperties": false, "properties": {"remove": {"type": "object", "required": ["line"], "additionalProperties": false, "properties": {"line": {"type": "integer", "minimum": 0}}}}},
                        {"type": "object", "required": ["set-range"], "additionalProperties": false, "properties": {"set-range": {"type": "object", "required": ["line", "value"], "additionalProperties": false, "properties": {"line": {"type": "integer", "minimum": 0}, "value": {"type": "number", "minimum": 0, "maximum": 1}}}}},
                        {"type": "object", "required": ["add"], "additionalProperties": false, "properties": {"add": {"type": "object", "required": ["text"], "additionalProperties": false, "properties": {"text": {"type": "string"}}}}}
                    ]
                }
            },
            "expect": {
                "type": "array",
                "items": {"type": "object", "required": ["line", "text"], "additionalProperties": false, "properties": {"line": {"type": "integer", "minimum": 0}, "text": {"type": "string"}}}
            }
        }
    })
}

fn tree_change_schema() -> Value {
    let node = |extra: Value| {
        let mut required = vec![json!("spec"), json!("node")];
        let mut properties = json!({
            "spec": {"type": "integer", "minimum": 1, "description": "The spec's ID (get_tree)."},
            "node": {"type": "integer", "minimum": 0, "description": "The node's ID (find_tree_nodes)."}
        });
        if let (Some(p), Some(e)) = (properties.as_object_mut(), extra.as_object()) {
            for (k, v) in e {
                p.insert(k.clone(), v.clone());
                required.push(json!(k));
            }
        }
        json!({"type": "object", "required": required, "additionalProperties": false, "properties": properties})
    };
    json!({
        "description": "app-api.v1 TreeChange: one passive tree edit, as the tree editor makes it. allocate-node allocates the node and the shortest path to it; deallocate-node removes the node and the nodes it leaves unconnected; set-mastery-effect chooses a mastery's effect (get_tree_node lists them). Nodes the editor keeps but does not edit are never removed.",
        "oneOf": [
            {"type": "object", "required": ["allocate-node"], "additionalProperties": false, "properties": {"allocate-node": node(json!({}))}},
            {"type": "object", "required": ["deallocate-node"], "additionalProperties": false, "properties": {"deallocate-node": node(json!({}))}},
            {"type": "object", "required": ["set-mastery-effect"], "additionalProperties": false, "properties": {"set-mastery-effect": node(json!({"effect": {"type": "integer", "minimum": 0, "description": "The effect's ID."}}))}}
        ]
    })
}

fn empty() -> Value {
    json!({"type": "object", "properties": {}, "additionalProperties": false})
}

/// The tool list (`tools/list`'s `tools`), the committed contract.
#[must_use]
pub fn list() -> Value {
    json!([
        {
            "name": "get_build",
            "title": "Get the open build",
            "description": "The open build: class, level, main skill, data version, line counts, items, and lines with warnings (app-api.v1 BuildView), with the snapshot version and the calculation's status (CalcStatus). Read-only.",
            "inputSchema": empty(),
            "annotations": {"readOnlyHint": true, "openWorldHint": false}
        },
        {
            "name": "list_fields",
            "title": "List calculated fields",
            "description": "Every field of the latest calculation: name, value (absent when not computed), and state (complete, partial, unsupported, indeterminate), with the version it is of. Read-only.",
            "inputSchema": empty(),
            "annotations": {"readOnlyHint": true, "openWorldHint": false}
        },
        {
            "name": "get_breakdown",
            "title": "Explain a field",
            "description": "One field of the latest calculation with its provenance: the field result and every graph node it depends on (calc result document v1 FieldResult and Node, nodes keyed by index). Contributions name their source (an item line, a passive, a gem, the config). Read-only.",
            "inputSchema": {"type": "object", "required": ["field"], "additionalProperties": false, "properties": {"field": {"type": "string", "description": "The field's name, as list_fields gives it (life, resistance.fire, hit-dps)."}}},
            "annotations": {"readOnlyHint": true, "openWorldHint": false}
        },
        {
            "name": "list_items",
            "title": "List items",
            "description": "The build's items: ID, name, base, rarity, slot, and number of lines with warnings (app-api.v1 ItemSummary). Read-only.",
            "inputSchema": empty(),
            "annotations": {"readOnlyHint": true, "openWorldHint": false}
        },
        {
            "name": "get_item",
            "title": "Get an item",
            "description": "One item with every line: index, text, status (matched, adapted, warning, excluded), warning code, and the game stat IDs it resolved to (app-api.v1 ItemView). Read-only.",
            "inputSchema": {"type": "object", "required": ["item"], "additionalProperties": false, "properties": {"item": {"type": "integer", "minimum": 0}}},
            "annotations": {"readOnlyHint": true, "openWorldHint": false}
        },
        {
            "name": "list_warnings",
            "title": "List lines with warnings",
            "description": "Every item line the application could not read: kept, shown, and not calculated (app-api.v1 WarningView). Read-only.",
            "inputSchema": empty(),
            "annotations": {"readOnlyHint": true, "openWorldHint": false}
        },
        {
            "name": "preview_item_edit",
            "title": "Preview an item edit",
            "description": "The effect of an item edit, calculated by the application without applying it: the edited item and every field before and after (app-api.v1 ItemEditPreview). Read-only.",
            "inputSchema": {"type": "object", "required": ["edit"], "additionalProperties": false, "properties": {"edit": item_edit_schema()}},
            "annotations": {"readOnlyHint": true, "openWorldHint": false}
        },
        {
            "name": "request_item_edit",
            "title": "Request an item edit",
            "description": "Asks the user to apply an item edit (app-api.v1 ChangeRequest). Refused unless the user allows change requests (APP_REQUESTS_OFF). The request waits for the user's approval in the application unless they gave a standing grant; poll get_request. An applied edit is one undoable step in the build's history.",
            "inputSchema": {"type": "object", "required": ["edit"], "additionalProperties": false, "properties": {"edit": item_edit_schema(), "note": {"type": "string", "maxLength": 500, "description": "Why, shown to the user."}}},
            "annotations": {"readOnlyHint": false, "destructiveHint": false, "idempotentHint": false, "openWorldHint": false}
        },
        {
            "name": "get_tree",
            "title": "Get the passive tree",
            "description": "The active passive spec (app-api.v1 TreeView: spec ID, class, ascendancy, allocated nodes, mastery effects, socketed jewels, kept nodes, points, and why it cannot be edited, if it cannot), each allocated node's name and kind, and the build's specs. Read-only.",
            "inputSchema": empty(),
            "annotations": {"readOnlyHint": true, "openWorldHint": false}
        },
        {
            "name": "find_tree_nodes",
            "title": "Find passive tree nodes",
            "description": "Nodes of the loaded data's tree whose name or stat lines contain the text (any case): ID, name, kind, ascendancy, lines, and whether the active spec allocates it; at most `limit` (default 25, at most 100), ascending by ID. Read-only.",
            "inputSchema": {"type": "object", "required": ["text"], "additionalProperties": false, "properties": {"text": {"type": "string", "minLength": 1, "maxLength": 200}, "kind": {"type": "string", "enum": ["normal", "notable", "keystone", "mastery", "jewel-socket"], "description": "Only nodes of this kind."}, "limit": {"type": "integer", "minimum": 1, "maximum": 100}}},
            "annotations": {"readOnlyHint": true, "openWorldHint": false}
        },
        {
            "name": "get_tree_node",
            "title": "Get a passive tree node",
            "description": "One node of the loaded data's tree (app-api.v1 TreeNodeView), whether the active spec allocates it, its neighbours' IDs and names, and, for a mastery, its effects and the chosen one. Read-only.",
            "inputSchema": {"type": "object", "required": ["node"], "additionalProperties": false, "properties": {"node": {"type": "integer", "minimum": 0}}},
            "annotations": {"readOnlyHint": true, "openWorldHint": false}
        },
        {
            "name": "preview_tree_change",
            "title": "Preview a passive tree change",
            "description": "The effect of a passive tree change, calculated by the application without applying it: the nodes it allocates (the path) or removes, and every field before and after (app-api.v1 TreePreview). A change the editor refuses fails with APP_TREE. Read-only.",
            "inputSchema": {"type": "object", "required": ["change"], "additionalProperties": false, "properties": {"change": tree_change_schema()}},
            "annotations": {"readOnlyHint": true, "openWorldHint": false}
        },
        {
            "name": "request_tree_change",
            "title": "Request a passive tree change",
            "description": "Asks the user to apply a passive tree change (app-api.v1 ChangeRequest with `tree`). Refused unless the user allows change requests (APP_REQUESTS_OFF). The request waits for the user's approval in the application unless they gave a standing grant; poll get_request. An applied change is one undoable step in the build's history.",
            "inputSchema": {"type": "object", "required": ["change"], "additionalProperties": false, "properties": {"change": tree_change_schema(), "note": {"type": "string", "maxLength": 500, "description": "Why, shown to the user."}}},
            "annotations": {"readOnlyHint": false, "destructiveHint": false, "idempotentHint": false, "openWorldHint": false}
        },
        {
            "name": "get_request",
            "title": "Get a change request",
            "description": "A change request's status: pending, applied, declined, or failed with its code (app-api.v1 ChangeRequest). Read-only.",
            "inputSchema": {"type": "object", "required": ["id"], "additionalProperties": false, "properties": {"id": {"type": "integer", "minimum": 0}}},
            "annotations": {"readOnlyHint": true, "openWorldHint": false}
        }
    ])
}

/// A tool call's failure: the code goes to the client as a tool error.
pub enum CallError {
    /// The arguments do not fit the tool (a JSON-RPC invalid-params error).
    Params(String),
    /// The tool ran and failed (a result with `isError`).
    Tool(AppError),
}

fn params<T: serde::de::DeserializeOwned>(args: &Value, key: &str) -> Result<T, CallError> {
    serde_json::from_value(args.get(key).cloned().unwrap_or(Value::Null))
        .map_err(|e| CallError::Params(format!("{key}: {e}")))
}

fn to_value(v: &impl serde::Serialize) -> Value {
    serde_json::to_value(v).unwrap_or(Value::Null)
}

fn no_build() -> CallError {
    CallError::Tool(AppError::new(loom_app::Code::NoBuild, "no build is open"))
}

fn no_tree() -> CallError {
    CallError::Tool(AppError::new(
        loom_app::Code::NoData,
        "the game data has no passive tree",
    ))
}

/// The active spec and the loaded data's tree.
fn tree_state(app: &App) -> Result<(api::BuildView, std::sync::Arc<api::TreeLayout>), CallError> {
    let build = app.snapshot().build.ok_or_else(no_build)?;
    let layout = app.tree_layout().map_err(CallError::Tool)?;
    Ok((build, layout))
}

fn kind_name(kind: &api::TreeNodeKind) -> Value {
    to_value(kind)
}

fn calc_document(app: &App) -> Result<(u64, Value), CallError> {
    let (version, doc) = app.calc_document().ok_or_else(|| {
        CallError::Tool(AppError::new(
            loom_app::Code::NoBuild,
            "no calculation has finished",
        ))
    })?;
    let doc: Value =
        serde_json::from_str(&doc).map_err(|e| CallError::Tool(AppError::layer("DTO_JSON", e)))?;
    Ok((version, doc))
}

/// Runs a tool.
///
/// # Errors
/// [`CallError`].
pub fn call(app: &App, client: &str, name: &str, args: &Value) -> Result<Value, CallError> {
    if !args.is_object() && !args.is_null() {
        return Err(CallError::Params("arguments must be an object".to_owned()));
    }
    match name {
        "get_build" => {
            let snap = app.snapshot();
            let build = snap.build.ok_or_else(no_build)?;
            Ok(
                json!({"version": snap.version, "build": to_value(&build), "calc": to_value(&snap.calc)}),
            )
        }
        "list_fields" => {
            let (version, doc) = calc_document(app)?;
            let fields: Vec<Value> = doc["result"]["fields"]
                .as_array()
                .map(|fs| {
                    fs.iter()
                        .map(|f| json!({"field": f["field"], "value": f.get("value").cloned().unwrap_or(Value::Null), "state": f["state"]}))
                        .collect()
                })
                .unwrap_or_default();
            Ok(json!({"version": version, "state": doc["result"]["state"], "fields": fields}))
        }
        "get_breakdown" => {
            let field: String = params(args, "field")?;
            let (version, doc) = calc_document(app)?;
            let result = &doc["result"];
            let f = result["fields"]
                .as_array()
                .and_then(|fs| fs.iter().find(|f| f["field"] == field.as_str()))
                .cloned()
                .ok_or_else(|| CallError::Params(format!("no field {field}")))?;
            let graph = result["graph"].as_array().cloned().unwrap_or_default();
            let mut nodes = serde_json::Map::new();
            let mut stack: Vec<u64> = f["node"].as_u64().into_iter().collect();
            while let Some(i) = stack.pop() {
                let key = i.to_string();
                if nodes.contains_key(&key) || nodes.len() >= 5000 {
                    continue;
                }
                let Some(node) = usize::try_from(i).ok().and_then(|i| graph.get(i)) else {
                    continue;
                };
                if let Some(children) = node["children"].as_array() {
                    stack.extend(children.iter().filter_map(Value::as_u64));
                }
                nodes.insert(key, node.clone());
            }
            Ok(json!({"version": version, "field": f, "nodes": nodes}))
        }
        "list_items" => {
            let build = app.snapshot().build.ok_or_else(no_build)?;
            Ok(json!({"items": to_value(&build.items)}))
        }
        "get_item" => {
            let id: u32 = params(args, "item")?;
            app.item(id).map(|i| to_value(&i)).map_err(CallError::Tool)
        }
        "list_warnings" => {
            let build = app.snapshot().build.ok_or_else(no_build)?;
            Ok(json!({"warnings": to_value(&build.warnings)}))
        }
        "preview_item_edit" => {
            let edit: api::ItemEdit = params(args, "edit")?;
            app.preview(&edit)
                .map(|p| to_value(&p))
                .map_err(CallError::Tool)
        }
        "request_item_edit" => {
            let edit: api::ItemEdit = params(args, "edit")?;
            let note: Option<String> = params(args, "note")?;
            app.request_change(client, edit, note)
                .map(|r| to_value(&r))
                .map_err(CallError::Tool)
        }
        "get_tree" => {
            let (build, layout) = tree_state(app)?;
            let tree = build.tree.as_ref().ok_or_else(no_tree)?;
            let allocated: Vec<Value> = tree
                .nodes
                .iter()
                .filter_map(|id| layout.nodes.iter().find(|n| n.id == *id))
                .map(|n| json!({"id": n.id, "name": n.name, "kind": kind_name(&n.kind)}))
                .collect();
            Ok(json!({
                "tree-version": layout.tree_version,
                "tree": to_value(tree),
                "allocated": allocated,
                "specs": to_value(&build.specs),
            }))
        }
        "find_tree_nodes" => {
            let text: String = params(args, "text")?;
            let kind: Option<String> = params(args, "kind")?;
            let limit: Option<usize> = params(args, "limit")?;
            let (build, layout) = tree_state(app)?;
            let allocated = build
                .tree
                .as_ref()
                .map(|t| t.nodes.as_slice())
                .unwrap_or_default();
            let needle = text.to_lowercase();
            let nodes: Vec<Value> = layout
                .nodes
                .iter()
                .filter(|n| n.modelled && !n.cluster_area)
                .filter(|n| kind.as_deref().is_none_or(|k| kind_name(&n.kind) == k))
                .filter(|n| {
                    n.name.to_lowercase().contains(&needle)
                        || n.lines.iter().any(|l| l.to_lowercase().contains(&needle))
                })
                .take(limit.unwrap_or(25).clamp(1, 100))
                .map(|n| {
                    json!({
                        "id": n.id,
                        "name": n.name,
                        "kind": kind_name(&n.kind),
                        "ascendancy": n.ascendancy,
                        "lines": n.lines,
                        "allocated": allocated.contains(&n.id),
                    })
                })
                .collect();
            Ok(json!({"nodes": nodes}))
        }
        "get_tree_node" => {
            let id: u32 = params(args, "node")?;
            let (build, layout) = tree_state(app)?;
            let node = layout
                .nodes
                .iter()
                .find(|n| n.id == id)
                .ok_or_else(|| CallError::Params(format!("no node {id}")))?;
            let tree = build.tree.as_ref();
            let neighbours: Vec<Value> = layout
                .nodes
                .iter()
                .filter(|n| n.id != id && (node.links.contains(&n.id) || n.links.contains(&id)))
                .map(|n| json!({"id": n.id, "name": n.name}))
                .collect();
            let effects = node.mastery_group.as_ref().and_then(|g| {
                layout
                    .masteries
                    .iter()
                    .find(|m| &m.id == g)
                    .map(|m| to_value(&m.effects))
            });
            let chosen = tree
                .and_then(|t| t.masteries.iter().find(|m| m.node == id))
                .map(|m| m.effect);
            Ok(json!({
                "node": to_value(node),
                "allocated": tree.is_some_and(|t| t.nodes.contains(&id)),
                "neighbours": neighbours,
                "mastery-effects": effects,
                "chosen-effect": chosen,
            }))
        }
        "preview_tree_change" => {
            let change: api::TreeChange = params(args, "change")?;
            app.preview_tree_change(&change)
                .map(|p| to_value(&p))
                .map_err(CallError::Tool)
        }
        "request_tree_change" => {
            let change: api::TreeChange = params(args, "change")?;
            let note: Option<String> = params(args, "note")?;
            app.request_tree_change(client, change, note)
                .map(|r| to_value(&r))
                .map_err(CallError::Tool)
        }
        "get_request" => {
            let id: u64 = params(args, "id")?;
            app.request(id).map(|r| to_value(&r)).ok_or_else(|| {
                CallError::Tool(AppError::new(
                    loom_app::Code::Request,
                    format!("no request {id}"),
                ))
            })
        }
        other => Err(CallError::Params(format!("no tool {other}"))),
    }
}
