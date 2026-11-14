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
