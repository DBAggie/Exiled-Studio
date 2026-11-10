//! The adapter's output document (protocol 2): the hand-written schema
//! check and the canonical form `oracle-run` hashes and compares. The
//! contract is documented in `tools/oracle/schema/oracle-output.schema.json`;
//! this module is what enforces it (Spike C plan: no JSON-schema crate).
//!
//! Data trees (`main_output`, each breakdown) hold JSON values as the
//! adapter writes Lua tables: objects, arrays, strings, numbers, booleans,
//! and one-key placeholder objects for values that are not data. `null`
//! never occurs inside a tree (Lua tables cannot hold `nil`).

use serde_json::{Map, Value};

use crate::oracle::Adapter;

/// Where PoB's breakdowns are read from (the Calcs tab's environment).
pub const BREAKDOWN_SOURCE: &str = "calcsTab.calcsEnv.player.breakdown";

/// The top-level fields, in the order the schema lists them.
pub const FIELDS: [&str; 11] = [
    "protocol",
    "stage",
    "error",
    "partial",
    "modules",
    "settings",
    "shims",
    "probe",
    "breakdown_source",
    "breakdowns",
    "main_output",
];

/// The fields of one shim record.
const SHIM_FIELDS: [&str; 4] = ["name", "replaces", "installed", "detail"];

/// The fields of one recorded `Inflate` call.
const INFLATE_FIELDS: [&str; 5] = [
    "input_bytes",
    "output_bytes",
    "adler32",
    "trailing_bytes",
    "error",
];

/// What the shims did in one run, summed from the document's `probe`.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct Probe {
    /// `Inflate` calls.
    pub inflate_calls: usize,
    /// `Inflate` calls that failed (no zlib, or a zlib error).
    pub inflate_errors: usize,
    /// Bytes `Inflate` returned, over all calls.
    pub inflated_bytes: u64,
    /// Opens for writing the write guard refused.
    pub writes_refused: usize,
    /// Shims that are not installed (by name).
    pub not_installed: Vec<String>,
}

/// The stage a run reached; every stage but `Done` is where it failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// Starting PoB through `HeadlessWrapper.lua`.
    Startup,
    /// Reading and parsing the fixture.
    Import,
    /// Loading the build (PoB calculates while loading).
    Calculate,
    /// Writing PoB's tables as data.
    Normalize,
    /// Every stage passed.
    Done,
}

impl Stage {
    fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "startup" => Self::Startup,
            "import" => Self::Import,
            "calculate" => Self::Calculate,
            "normalize" => Self::Normalize,
            "done" => Self::Done,
            _ => return None,
        })
    }

    /// The name the document uses.
    pub fn name(self) -> &'static str {
        match self {
            Self::Startup => "startup",
            Self::Import => "import",
            Self::Calculate => "calculate",
            Self::Normalize => "normalize",
            Self::Done => "done",
        }
    }
}

/// A checked document.
#[derive(Debug)]
pub struct Document {
    /// Stage reached.
    pub stage: Stage,
    /// `(source, message)` of the failure; `None` when `stage` is `Done`.
    pub error: Option<(String, String)>,
    /// Reasons the result is partial (empty: complete).
    pub partial: Vec<String>,
    /// Modules PoB loaded through `require`.
    pub modules: Vec<String>,
    /// Whether a PoB settings file was found; `None` before startup ended.
    pub settings_found: Option<bool>,
    /// What the shims did.
    pub probe: Probe,
    /// The whole document with sorted keys.
    pub canonical: Value,
}

/// Parses and checks `bytes` against the protocol of `adapter`.
pub fn parse(bytes: &[u8], adapter: &Adapter) -> Result<Document, String> {
    let value: Value = serde_json::from_slice(bytes).map_err(|e| format!("not JSON: {e}"))?;
    let Value::Object(top) = &value else {
        return Err("document: not an object".to_owned());
    };
    for key in top.keys() {
        if !FIELDS.contains(&key.as_str()) {
            return Err(format!("document: unknown field {key}"));
        }
    }
    let field = |name: &str| {
        top.get(name)
            .ok_or_else(|| format!("document: missing field {name}"))
    };
    if field("protocol")?.as_u64() != Some(u64::from(adapter.protocol)) {
        return Err(format!("protocol: expected {}", adapter.protocol));
    }
    let stage = field("stage")?
        .as_str()
        .and_then(Stage::parse)
        .ok_or_else(|| "stage: not a known stage".to_owned())?;
    let error = match (field("error")?, stage) {
        (Value::Null, Stage::Done) => None,
        (Value::Null, _) => return Err("error: missing for a failed stage".to_owned()),
        (_, Stage::Done) => return Err("error: present although the stage is done".to_owned()),
        (Value::Object(error), _) => Some(check_error(error)?),
        _ => return Err("error: not an object".to_owned()),
    };
    let partial = string_list(field("partial")?, "partial")?;
    let modules = string_list(field("modules")?, "modules")?;
    if !modules.windows(2).all(|w| w[0] < w[1]) {
        return Err("modules: not sorted and unique".to_owned());
    }
    let settings_found = match (field("settings")?, stage) {
        (Value::Null, Stage::Startup) => None,
        (Value::Null, _) => return Err("settings: missing after startup".to_owned()),
        (Value::Object(settings), _) => Some(check_settings(settings)?),
        _ => return Err("settings: not an object".to_owned()),
    };
    let mut probe = check_probe(field("probe")?)?;
    probe.not_installed = check_shims(field("shims")?, adapter)?;
    // A result that depends on a failed shim is partial, never ok.
    if probe.inflate_errors > 0 && partial.is_empty() {
        return Err("partial: empty although an Inflate call failed".to_owned());
    }
    if field("breakdown_source")?.as_str() != Some(BREAKDOWN_SOURCE) {
        return Err(format!("breakdown_source: expected {BREAKDOWN_SOURCE}"));
    }
    let main_output = field("main_output")?;
    let breakdowns = field("breakdowns")?;
    if stage == Stage::Done {
        let Value::Object(_) = main_output else {
            return Err("main_output: not an object".to_owned());
        };
        check_tree(main_output, "main_output", adapter)?;
        let Value::Object(breakdowns) = breakdowns else {
            return Err("breakdowns: not an object".to_owned());
        };
        let names: Vec<&String> = breakdowns.keys().collect();
        let mut expected: Vec<&String> = adapter.breakdowns.iter().collect();
        expected.sort();
        let mut found = names.clone();
        found.sort();
        if found != expected {
            return Err(format!(
                "breakdowns: expected the names {expected:?}, found {found:?}"
            ));
        }
        for (name, tree) in breakdowns {
            if !tree.is_null() {
                check_tree(tree, &format!("breakdowns.{name}"), adapter)?;
            }
        }
    } else if !main_output.is_null() || !breakdowns.is_null() {
        return Err("main_output and breakdowns: present although the stage failed".to_owned());
    }
    Ok(Document {
        stage,
        error,
        partial,
        modules,
        settings_found,
        probe,
        canonical: canonical(&value),
    })
}

/// Checks the shim records: exactly the manifest's shims, in its order.
/// Returns the names of those not installed.
fn check_shims(value: &Value, adapter: &Adapter) -> Result<Vec<String>, String> {
    let Value::Array(records) = value else {
        return Err("shims: not an array".to_owned());
    };
    let mut names = Vec::new();
    let mut not_installed = Vec::new();
    for record in records {
        let Value::Object(record) = record else {
            return Err("shims: a record is not an object".to_owned());
        };
        exact_keys(record, &SHIM_FIELDS, "shims")?;
        let name = record["name"]
            .as_str()
            .ok_or_else(|| "shims.name: not a string".to_owned())?;
        if !record["replaces"].is_string() {
            return Err(format!("shims.{name}.replaces: not a string"));
        }
        if !(record["detail"].is_string() || record["detail"].is_null()) {
            return Err(format!("shims.{name}.detail: not a string or null"));
        }
        let installed = record["installed"]
            .as_bool()
            .ok_or_else(|| format!("shims.{name}.installed: not a boolean"))?;
        if !installed {
            not_installed.push(name.to_owned());
        }
        names.push(name);
    }
    if names != adapter.shims {
        return Err(format!(
            "shims: expected {:?}, found {names:?}",
            adapter.shims
        ));
    }
    Ok(not_installed)
}

/// Checks the probe and sums it up.
fn check_probe(value: &Value) -> Result<Probe, String> {
    let Value::Object(probe) = value else {
        return Err("probe: not an object".to_owned());
    };
    exact_keys(probe, &["inflate", "writes_refused"], "probe")?;
    let writes_refused = string_list(&probe["writes_refused"], "probe.writes_refused")?.len();
    let Value::Array(calls) = &probe["inflate"] else {
        return Err("probe.inflate: not an array".to_owned());
    };
    let mut summary = Probe {
        inflate_calls: calls.len(),
        writes_refused,
        ..Probe::default()
    };
    for (i, call) in calls.iter().enumerate() {
        let path = format!("probe.inflate[{}]", i + 1);
        let Value::Object(call) = call else {
            return Err(format!("{path}: not an object"));
        };
        exact_keys(call, &INFLATE_FIELDS, &path)?;
        if call["input_bytes"].as_u64().is_none() {
            return Err(format!("{path}.input_bytes: not a count"));
        }
        let results = ["output_bytes", "adler32", "trailing_bytes"];
        match &call["error"] {
            Value::String(_) => {
                if results.iter().any(|k| !call[*k].is_null()) {
                    return Err(format!("{path}: results present with an error"));
                }
                summary.inflate_errors += 1;
            }
            Value::Null => {
                if results.iter().any(|k| call[*k].as_u64().is_none()) {
                    return Err(format!("{path}: results missing without an error"));
                }
                if call["adler32"]
                    .as_u64()
                    .is_some_and(|a| a > u64::from(u32::MAX))
                {
                    return Err(format!("{path}.adler32: not a 32-bit value"));
                }
                summary.inflated_bytes += call["output_bytes"].as_u64().unwrap_or(0);
            }
            _ => return Err(format!("{path}.error: not a string or null")),
        }
    }
    Ok(summary)
}

/// `map` has exactly the keys `expected`.
fn exact_keys(map: &Map<String, Value>, expected: &[&str], path: &str) -> Result<(), String> {
    if map.len() != expected.len() || !expected.iter().all(|k| map.contains_key(*k)) {
        return Err(format!("{path}: expected exactly the keys {expected:?}"));
    }
    Ok(())
}

fn check_error(error: &Map<String, Value>) -> Result<(String, String), String> {
    if error.len() != 2 {
        return Err("error: expected exactly source and message".to_owned());
    }
    let source = error.get("source").and_then(Value::as_str);
    let message = error.get("message").and_then(Value::as_str);
    match (source, message) {
        (Some(source @ ("lua" | "prompt" | "pob" | "adapter")), Some(message)) => {
            Ok((source.to_owned(), message.to_owned()))
        }
        _ => Err("error: source must be lua, prompt, pob, or adapter; message a string".to_owned()),
    }
}

fn check_settings(settings: &Map<String, Value>) -> Result<bool, String> {
    if settings.len() != 2 || !settings.get("path").is_some_and(Value::is_string) {
        return Err("settings: expected exactly path and found".to_owned());
    }
    settings
        .get("found")
        .and_then(Value::as_bool)
        .ok_or_else(|| "settings.found: not a boolean".to_owned())
}

fn string_list(value: &Value, name: &str) -> Result<Vec<String>, String> {
    let Value::Array(items) = value else {
        return Err(format!("{name}: not an array"));
    };
    items
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("{name}: not a string list"))
        })
        .collect()
}

/// Checks a data tree: no `null`; `$` keys only as a known placeholder
/// (the only key of its object); `#` keys only as numbers.
fn check_tree(value: &Value, path: &str, adapter: &Adapter) -> Result<(), String> {
    match value {
        Value::Null => Err(format!("{path}: null inside a data tree")),
        Value::Bool(_) | Value::Number(_) | Value::String(_) => Ok(()),
        Value::Array(items) => items
            .iter()
            .enumerate()
            .try_for_each(|(i, item)| check_tree(item, &format!("{path}[{}]", i + 1), adapter)),
        Value::Object(map) => {
            if let Some((key, inner)) = map.iter().find(|(k, _)| k.starts_with('$')) {
                if map.len() != 1 {
                    return Err(format!("{path}: placeholder {key} with other keys"));
                }
                return check_placeholder(key, inner, path, adapter);
            }
            for (key, inner) in map {
                if let Some(number) = key.strip_prefix('#')
                    && !is_number_key(number)
                {
                    return Err(format!("{path}: key {key} is not a number key"));
                }
                check_tree(inner, &format!("{path}.{key}"), adapter)?;
            }
            Ok(())
        }
    }
}

fn is_number_key(text: &str) -> bool {
    matches!(text, "inf" | "-inf")
        || (!text.is_empty() && text.parse::<f64>().is_ok_and(f64::is_finite))
}

fn check_placeholder(
    key: &str,
    inner: &Value,
    path: &str,
    adapter: &Adapter,
) -> Result<(), String> {
    let ok = match key {
        "$number" => matches!(inner.as_str(), Some("inf" | "-inf" | "nan")),
        "$object" | "$cycle" | "$userdata" => inner.is_string(),
        "$opaque" => inner
            .as_str()
            .is_some_and(|k| adapter.opaque_keys.iter().any(|o| o == k)),
        "$function" => inner == &Value::Bool(true),
        _ => false,
    };
    if ok {
        Ok(())
    } else {
        Err(format!("{path}: invalid placeholder {key}: {inner}"))
    }
}

/// `value` with every object's keys sorted, whatever map type serde_json
/// was built with (another crate in the workspace may enable
/// `preserve_order`).
pub fn canonical(value: &Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.iter().map(canonical).collect()),
        Value::Object(map) => {
            let mut entries: Vec<(&String, &Value)> = map.iter().collect();
            entries.sort_by(|a, b| a.0.cmp(b.0));
            Value::Object(
                entries
                    .into_iter()
                    .map(|(k, v)| (k.clone(), canonical(v)))
                    .collect(),
            )
        }
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn adapter() -> Adapter {
        Adapter {
            protocol: 2,
            breakdowns: vec!["Life".to_owned(), "TotalDPS".to_owned()],
            opaque_keys: vec!["sourceGem".to_owned()],
            shims: vec!["inflate".to_owned(), "write_guard".to_owned()],
            zlib: std::collections::BTreeMap::new(),
        }
    }

    fn shims() -> Value {
        json!([
            {"name": "inflate", "replaces": "Inflate", "installed": true, "detail": "zlib 1.3.1"},
            {"name": "write_guard", "replaces": "io.open", "installed": true, "detail": null}
        ])
    }

    fn inflated() -> Value {
        json!({"input_bytes": 10, "output_bytes": 40, "adler32": 7, "trailing_bytes": 0, "error": null})
    }

    fn done() -> Value {
        json!({
            "protocol": 2,
            "stage": "done",
            "error": null,
            "partial": [],
            "modules": ["lua-utf8", "xml"],
            "settings": {"path": "/Settings.xml", "found": false},
            "shims": shims(),
            "probe": {"inflate": [inflated()], "writes_refused": ["./Data/x.bin"]},
            "breakdown_source": BREAKDOWN_SOURCE,
            "breakdowns": {"Life": ["1 (base)", "= 1"], "TotalDPS": null},
            "main_output": {
                "Life": 60,
                "Speed": 0.1,
                "Big": {"$number": "inf"},
                "Req": {"sourceGem": {"$opaque": "sourceGem"}, "item": {"$object": "Item"}},
                "#3": true,
                "List": [1, "a", {"$function": true}]
            }
        })
    }

    fn failed(stage: &str) -> Value {
        json!({
            "protocol": 2,
            "stage": stage,
            "error": {"source": "pob", "message": "bad"},
            "partial": [],
            "modules": [],
            "settings": {"path": "/Settings.xml", "found": false},
            "shims": shims(),
            "probe": {"inflate": [], "writes_refused": []},
            "breakdown_source": BREAKDOWN_SOURCE,
            "breakdowns": null,
            "main_output": null
        })
    }

    fn check(value: &Value) -> Result<Document, String> {
        parse(value.to_string().as_bytes(), &adapter())
    }

    #[test]
    fn parse_should_accept_complete_document() {
        let document = check(&done()).unwrap();
        assert_eq!(document.stage, Stage::Done);
        assert_eq!(document.settings_found, Some(false));
        assert_eq!(document.error, None);
        assert_eq!(
            document.probe,
            Probe {
                inflate_calls: 1,
                inflate_errors: 0,
                inflated_bytes: 40,
                writes_refused: 1,
                not_installed: vec![],
            }
        );
    }

    #[test]
    fn parse_should_accept_a_failed_inflate_in_a_partial_result() {
        let mut value = done();
        value["shims"][0]["installed"] = false.into();
        value["shims"][0]["detail"] = "cannot load the zlib library".into();
        value["probe"]["inflate"][0] = json!({"input_bytes": 10, "output_bytes": null,
            "adler32": null, "trailing_bytes": null, "error": "cannot load the zlib library"});
        value["partial"] = json!(["inflate: cannot load the zlib library"]);
        let probe = check(&value).unwrap().probe;
        assert_eq!(probe.inflate_errors, 1);
        assert_eq!(probe.not_installed, ["inflate"]);
        // The same without a partial reason breaks the contract.
        value["partial"] = json!([]);
        let err = check(&value).unwrap_err();
        assert!(err.starts_with("partial"), "{err}");
    }

    #[test]
    fn parse_should_accept_failed_stage_without_output() {
        let document = check(&failed("import")).unwrap();
        assert_eq!(document.stage, Stage::Import);
        assert_eq!(document.error, Some(("pob".to_owned(), "bad".to_owned())));
    }

    #[test]
    fn parse_should_accept_startup_failure_without_settings() {
        let mut value = failed("startup");
        value["settings"] = Value::Null;
        assert_eq!(check(&value).unwrap().settings_found, None);
    }

    #[test]
    fn parse_should_reject_documents_that_break_the_schema() {
        type Edit = Box<dyn Fn(&mut Value)>;
        let cases: Vec<(&str, Edit)> = vec![
            ("not JSON", Box::new(|v| *v = json!("text"))),
            ("unknown field", Box::new(|v| v["extra"] = 1.into())),
            (
                "missing field",
                Box::new(|v| {
                    v.as_object_mut().unwrap().remove("modules");
                }),
            ),
            ("protocol", Box::new(|v| v["protocol"] = 1.into())),
            (
                "shim missing",
                Box::new(|v| {
                    v["shims"].as_array_mut().unwrap().pop();
                }),
            ),
            (
                "shims out of order",
                Box::new(|v| v["shims"].as_array_mut().unwrap().reverse()),
            ),
            (
                "shim extra key",
                Box::new(|v| v["shims"][0]["library"] = "C:/zlib1.dll".into()),
            ),
            (
                "shim installed not boolean",
                Box::new(|v| v["shims"][1]["installed"] = "yes".into()),
            ),
            ("probe missing", Box::new(|v| v["probe"] = Value::Null)),
            (
                "probe extra key",
                Box::new(|v| v["probe"]["other"] = json!([])),
            ),
            (
                "inflate call without results",
                Box::new(|v| v["probe"]["inflate"][0]["output_bytes"] = Value::Null),
            ),
            (
                "inflate call with results and an error",
                Box::new(|v| v["probe"]["inflate"][0]["error"] = "x".into()),
            ),
            (
                "inflate negative size",
                Box::new(|v| v["probe"]["inflate"][0]["input_bytes"] = (-1).into()),
            ),
            (
                "adler32 above 32 bits",
                Box::new(|v| v["probe"]["inflate"][0]["adler32"] = 4_294_967_296_u64.into()),
            ),
            (
                "refused write not a string",
                Box::new(|v| v["probe"]["writes_refused"] = json!([1])),
            ),
            ("stage", Box::new(|v| v["stage"] = "later".into())),
            (
                "error on done",
                Box::new(|v| v["error"] = json!({"source": "lua", "message": "x"})),
            ),
            (
                "modules order",
                Box::new(|v| v["modules"] = json!(["xml", "lua-utf8"])),
            ),
            (
                "settings missing",
                Box::new(|v| v["settings"] = Value::Null),
            ),
            (
                "settings shape",
                Box::new(|v| v["settings"] = json!({"found": true})),
            ),
            (
                "breakdown source",
                Box::new(|v| v["breakdown_source"] = "elsewhere".into()),
            ),
            (
                "breakdown names",
                Box::new(|v| {
                    v["breakdowns"].as_object_mut().unwrap().remove("Life");
                }),
            ),
            (
                "null in tree",
                Box::new(|v| v["main_output"]["Life"] = Value::Null),
            ),
            (
                "unknown placeholder",
                Box::new(|v| v["main_output"]["Life"] = json!({"$table": 1})),
            ),
            (
                "placeholder with keys",
                Box::new(|v| v["main_output"]["Big"]["x"] = 1.into()),
            ),
            (
                "bad number tag",
                Box::new(|v| v["main_output"]["Big"] = json!({"$number": "1.#INF"})),
            ),
            (
                "unlisted opaque key",
                Box::new(|v| v["main_output"]["Req"]["sourceGem"] = json!({"$opaque": "item"})),
            ),
            (
                "bad number key",
                Box::new(|v| v["main_output"]["#x"] = 1.into()),
            ),
            (
                "main_output array",
                Box::new(|v| v["main_output"] = json!([1])),
            ),
        ];
        for (name, edit) in cases {
            let mut value = done();
            edit(&mut value);
            assert!(check(&value).is_err(), "{name} was accepted");
        }
    }

    #[test]
    fn parse_should_reject_output_after_failed_stage() {
        let mut value = failed("calculate");
        value["main_output"] = json!({"Life": 1});
        assert!(check(&value).is_err());
        let mut value = failed("calculate");
        value["error"] = Value::Null;
        assert!(check(&value).is_err());
        let mut value = failed("calculate");
        value["error"]["source"] = "shell".into();
        assert!(check(&value).is_err());
    }

    #[test]
    fn schema_file_should_list_the_checked_fields_and_constants() {
        let schema: Value = serde_json::from_slice(include_bytes!(
            "../../oracle/schema/oracle-output.schema.json"
        ))
        .unwrap();
        let required: Vec<&str> = schema["required"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert_eq!(required, FIELDS);
        assert_eq!(schema["properties"]["protocol"]["const"], 2);
        let keys = |value: &Value| -> Vec<String> {
            value["required"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap().to_owned())
                .collect()
        };
        assert_eq!(keys(&schema["$defs"]["shim"]), SHIM_FIELDS);
        assert_eq!(keys(&schema["$defs"]["inflate_call"]), INFLATE_FIELDS);
        assert_eq!(
            schema["properties"]["breakdown_source"]["const"],
            BREAKDOWN_SOURCE
        );
        let stages: Vec<&str> = schema["properties"]["stage"]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        for stage in &stages {
            assert_eq!(Stage::parse(stage).map(Stage::name), Some(*stage));
        }
        assert_eq!(stages.len(), 5);
    }

    #[test]
    fn canonical_should_sort_keys_and_shorten_numbers() {
        let text = r#"{"b":0.10000000000000001,"a":{"d":5,"c":-0}}"#;
        let value: Value = serde_json::from_str(text).unwrap();
        assert_eq!(
            serde_json::to_string(&canonical(&value)).unwrap(),
            r#"{"a":{"c":-0.0,"d":5},"b":0.1}"#
        );
    }

    #[test]
    fn canonical_should_not_depend_on_key_order_in_the_document() {
        let a: Value = serde_json::from_str(r#"{"x":1,"y":{"q":2,"p":3}}"#).unwrap();
        let b: Value = serde_json::from_str(r#"{"y":{"p":3,"q":2},"x":1}"#).unwrap();
        assert_eq!(
            serde_json::to_string(&canonical(&a)).unwrap(),
            serde_json::to_string(&canonical(&b)).unwrap()
        );
    }
}
