//! The adapter's output document (protocol 1): the hand-written schema
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
pub const FIELDS: [&str; 9] = [
    "protocol",
    "stage",
    "error",
    "partial",
    "modules",
    "settings",
    "breakdown_source",
    "breakdowns",
    "main_output",
];

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
    /// The whole document with sorted keys.
    pub canonical: Value,
}

/// Parses and checks `bytes` against protocol 1 for `adapter`.
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
        canonical: canonical(&value),
    })
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
            protocol: 1,
            breakdowns: vec!["Life".to_owned(), "TotalDPS".to_owned()],
            opaque_keys: vec!["sourceGem".to_owned()],
        }
    }

    fn done() -> Value {
        json!({
            "protocol": 1,
            "stage": "done",
            "error": null,
            "partial": [],
            "modules": ["lua-utf8", "xml"],
            "settings": {"path": "/Settings.xml", "found": false},
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
            "protocol": 1,
            "stage": stage,
            "error": {"source": "pob", "message": "bad"},
            "partial": [],
            "modules": [],
            "settings": {"path": "/Settings.xml", "found": false},
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
            ("protocol", Box::new(|v| v["protocol"] = 2.into())),
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
        assert_eq!(schema["properties"]["protocol"]["const"], 1);
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
