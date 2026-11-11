//! The generated schemas (Phase 2 plan, step 7): the committed files are
//! fresh, and the documents `dto::v1::to_json` writes validate against
//! the schema (draft 2020-12, with `boon`), while broken ones do not.

#![expect(
    clippy::unwrap_used,
    reason = "a test reads fixed files; a failure should panic"
)]

use std::fs;
use std::path::{Path, PathBuf};

use loom_serialization::dto::v1::to_json;
use loom_serialization::legacy::pob_xml::import;
use serde_json::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn committed_schemas_should_be_fresh() {
    for (name, text) in loom_schema_gen::schemas().unwrap() {
        let committed = fs::read_to_string(root().join("schemas").join(name)).unwrap();
        assert!(
            committed == text,
            "schemas/{name} is stale: run `cargo run -p loom-schema-gen`"
        );
    }
}

struct Validator {
    schemas: boon::Schemas,
    index: boon::SchemaIndex,
}

impl Validator {
    fn new() -> Self {
        let schema: Value = serde_json::from_str(&loom_schema_gen::build_v1().unwrap()).unwrap();
        let mut compiler = boon::Compiler::new();
        compiler
            .add_resource("build.v1.schema.json", schema)
            .unwrap();
        let mut schemas = boon::Schemas::new();
        let index = compiler
            .compile("build.v1.schema.json", &mut schemas)
            .unwrap();
        Self { schemas, index }
    }

    fn check(&self, json: &str) -> Result<(), String> {
        let value: Value = serde_json::from_str(json).unwrap();
        self.schemas
            .validate(&value, self.index)
            .map_err(|e| format!("{e:#}"))
    }
}

fn golden(name: &str) -> String {
    fs::read_to_string(
        root()
            .join("crates/loom-serialization/src/dto/v1/testdata")
            .join(name),
    )
    .unwrap()
}

#[test]
fn synthetic_documents_should_validate() {
    let v = Validator::new();
    for name in ["poe1.json", "poe2.json"] {
        v.check(&golden(name)).unwrap();
    }
    for game in [loom_domain::Game::Poe1, loom_domain::Game::Poe2] {
        v.check(&to_json(&loom_domain::Build::new(game)).unwrap())
            .unwrap();
    }
}

#[test]
fn public_fixture_documents_should_validate() {
    let v = Validator::new();
    let dir = root().join("tools/oracle/fixtures");
    let mut checked = 0;
    for d in [dir.clone(), dir.join("community")] {
        for e in fs::read_dir(d).unwrap() {
            let p = e.unwrap().path();
            if !p.extension().is_some_and(|x| x == "xml")
                || p.file_name().is_some_and(|n| n == "malformed.xml")
            {
                continue;
            }
            let build = import(&fs::read_to_string(&p).unwrap()).unwrap();
            if let Err(e) = v.check(&to_json(&build).unwrap()) {
                panic!("{}: {e}", p.display());
            }
            checked += 1;
        }
    }
    assert!(checked >= 24, "only {checked} fixtures");
}

#[test]
fn broken_documents_should_not_validate() {
    let v = Validator::new();
    let doc = golden("poe1.json");
    for (from, to) in [
        // An unknown field.
        ("\"game\": \"poe1\",", "\"game\": \"poe1\", \"x\": 1,"),
        // Another format or version.
        ("\"loom-build\"", "\"other\""),
        ("\"version\": 1,", "\"version\": 2,"),
        // An unknown variant.
        ("\"header\": \"rarity\"", "\"header\": \"made-up\""),
        // A malformed non-finite number.
        ("\"7ff0000000000000\"", "\"7FF0\""),
        // A missing required field.
        ("\"enabled\": true,", ""),
    ] {
        let changed = doc.replacen(from, to, 1);
        assert_ne!(changed, doc, "{from} not found");
        assert!(v.check(&changed).is_err(), "{from} -> {to} validated");
    }
}
