//! The migration framework, exercised by test-only chains (Phase 2 plan,
//! decision 14: Phase 2 ships only v1).

use serde_json::{Value, json};

use super::*;
use crate::dto::{self, ReadError};

/// The v1 golden document of `dto::v1`'s tests.
const POE1: &str = include_str!("../dto/v1/testdata/poe1.json");

/// The synthetic v0: v1's shape, except that the data bundle's Steam build
/// was called `build_id`.
fn v0_json() -> String {
    let mut value: Value = serde_json::from_str(POE1).unwrap();
    value["version"] = json!(0);
    let data = value["build"]["data"].as_object_mut().unwrap();
    let steam_build = data.remove("steam_build").unwrap();
    data.insert("build_id".to_owned(), steam_build);
    serde_json::to_string_pretty(&value).unwrap()
}

/// The test-only step v0 to v1: renames `build.data.build_id`.
fn v0_to_v1(document: &mut Value, changes: &mut Vec<String>) -> Result<(), String> {
    let Some(data) = document
        .pointer_mut("/build/data")
        .and_then(Value::as_object_mut)
    else {
        // A build without a recorded bundle has nothing to rename.
        return Ok(());
    };
    if let Some(id) = data.remove("build_id") {
        data.insert("steam_build".to_owned(), id);
        changes.push("build.data.build_id renamed to steam_build".to_owned());
    } else if data.contains_key("steam_build") {
        return Err("build.data has steam_build, which v0 did not".to_owned());
    }
    Ok(())
}

fn v0_chain() -> Migrations {
    Migrations::new(
        vec![Migration {
            from: 0,
            apply: v0_to_v1,
        }],
        1,
    )
    .unwrap()
}

fn read_code(json: &str, migrations: &Migrations) -> &'static str {
    dto::read_with(json, migrations)
        .err()
        .map_or("ok", |e| e.code())
}

#[test]
fn a_v0_document_should_migrate_to_the_v1_build_with_its_report() {
    let loaded = dto::read_with(&v0_json(), &v0_chain()).unwrap();
    assert_eq!(loaded.build, dto::v1::from_json(POE1).unwrap());
    assert_eq!(
        loaded.report,
        MigrationReport {
            from: 0,
            to: 1,
            steps: vec![StepReport {
                from: 0,
                to: 1,
                changes: vec!["build.data.build_id renamed to steam_build".to_owned()],
            }],
        }
    );
    // Written back, the migrated build is the current document.
    assert_eq!(dto::write(&loaded.build).unwrap(), POE1);
}

#[test]
fn a_current_document_should_read_with_an_empty_report() {
    for migrations in [Migrations::shipped(), v0_chain()] {
        let loaded = dto::read_with(POE1, &migrations).unwrap();
        assert!(loaded.report.is_empty());
        assert_eq!((loaded.report.from, loaded.report.to), (1, 1));
    }
    assert!(dto::read(POE1).unwrap().report.is_empty());
}

#[test]
fn a_newer_document_should_give_migrate_too_new_before_it_is_read() {
    // Version 2 with a shape no reader knows: refused on the version alone.
    let json = r#"{"format": "loom-build", "version": 2, "build": {"made_up": []}}"#;
    assert_eq!(read_code(json, &Migrations::shipped()), "MIGRATE_TOO_NEW");
    assert_eq!(read_code(json, &v0_chain()), "MIGRATE_TOO_NEW");
    let big = r#"{"format": "loom-build", "version": 18446744073709551615}"#;
    assert_eq!(read_code(big, &Migrations::shipped()), "MIGRATE_TOO_NEW");
}

#[test]
fn a_version_no_step_reads_should_give_migrate_too_old() {
    // The shipped chain has no v0 step.
    assert_eq!(
        read_code(&v0_json(), &Migrations::shipped()),
        "MIGRATE_TOO_OLD"
    );
}

#[test]
fn a_failing_step_should_give_migrate_step() {
    // A v0 document with v1's field name: the step refuses it.
    let json = POE1.replacen("\"version\": 1,", "\"version\": 0,", 1);
    let err = dto::read_with(&json, &v0_chain()).unwrap_err();
    assert_eq!(err.code(), "MIGRATE_STEP");
    assert!(err.to_string().contains("version 0 to 1"), "{err}");
}

#[test]
fn a_step_that_leaves_no_object_should_give_migrate_step() {
    fn replace(document: &mut Value, _: &mut Vec<String>) -> Result<(), String> {
        *document = json!([]);
        Ok(())
    }
    let chain = Migrations::new(
        vec![Migration {
            from: 0,
            apply: replace,
        }],
        1,
    )
    .unwrap();
    assert_eq!(read_code(&v0_json(), &chain), "MIGRATE_STEP");
}

#[test]
fn steps_should_run_in_order_and_each_set_the_version() {
    fn note(document: &mut Value, changes: &mut Vec<String>) -> Result<(), String> {
        changes.push(format!("saw version {}", document["version"]));
        Ok(())
    }
    let chain = Migrations::new(
        (3..6).map(|from| Migration { from, apply: note }).collect(),
        6,
    )
    .unwrap();
    let doc = json!({"format": "loom-build", "version": 4});
    let (doc, report) = chain.run(doc, 4).unwrap();
    assert_eq!(doc["version"], json!(6));
    let seen: Vec<_> = report.steps.iter().map(|s| s.changes.join("")).collect();
    assert_eq!(seen, ["saw version 4", "saw version 5"]);
    assert_eq!((report.from, report.to), (4, 6));
    assert_eq!(chain.oldest(), 3);
}

#[test]
fn a_broken_chain_should_give_migrate_chain() {
    let step = |from| Migration {
        from,
        apply: v0_to_v1,
    };
    let codes: Vec<_> = [
        Migrations::new(vec![step(0), step(2)], 3),
        Migrations::new(vec![step(0)], 2),
        Migrations::new(vec![step(1), step(0)], 2),
        Migrations::new(vec![step(u64::MAX)], 0),
    ]
    .iter()
    .map(|r| r.as_ref().err().map_or("ok", MigrateError::code))
    .collect();
    assert_eq!(codes, ["MIGRATE_CHAIN"; 4]);
    assert!(Migrations::new(Vec::new(), 7).is_ok());
}

#[test]
fn a_document_without_a_build_header_should_give_the_dto_code() {
    let cases = [
        ("not json", "DTO_JSON"),
        ("[]", "DTO_JSON"),
        (r#"{"version": 1}"#, "DTO_JSON"),
        (r#"{"format": "other", "version": 1}"#, "DTO_FORMAT"),
        (r#"{"format": "loom-build", "version": "1"}"#, "DTO_JSON"),
        (r#"{"format": "loom-build", "version": -1}"#, "DTO_JSON"),
        (r#"{"format": "loom-build", "version": 1.5}"#, "DTO_JSON"),
    ];
    for (json, code) in cases {
        let err = dto::read(json).unwrap_err();
        assert!(matches!(err, ReadError::Dto(_)), "{json}");
        assert_eq!(err.code(), code, "{json}");
    }
}

#[test]
fn a_current_but_malformed_document_should_give_its_dto_code() {
    let json = POE1.replacen("\"game\": \"poe1\"", "\"game\": \"poe9\"", 1);
    assert_eq!(read_code(&json, &Migrations::shipped()), "DTO_JSON");
}

#[test]
fn the_report_should_serialize_for_hosts() {
    let loaded = dto::read_with(&v0_json(), &v0_chain()).unwrap();
    assert_eq!(
        serde_json::to_value(&loaded.report).unwrap(),
        json!({
            "from": 0,
            "to": 1,
            "steps": [{"from": 0, "to": 1, "changes": ["build.data.build_id renamed to steam_build"]}]
        })
    );
}
