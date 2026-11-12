//! Tests of calc-run on the synthetic bundles and builds CI has.

use std::path::Path;

use serde_json::json;

use super::*;

fn repo(path: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn input(path: &str) -> Input {
    let full = repo(path);
    let bytes = std::fs::read(&full).unwrap();
    let file = full.file_name().unwrap().to_string_lossy().into_owned();
    let format = if file.ends_with(".json") {
        Format::BuildDocument
    } else {
        Format::PobXml
    };
    Input {
        path: path.to_owned(),
        file,
        format,
        bytes,
    }
}

fn bytes_input(file: &str, format: Format, bytes: &[u8]) -> Input {
    Input {
        path: file.to_owned(),
        file: file.to_owned(),
        format,
        bytes: bytes.to_vec(),
    }
}

const POE1_BUILD: &str = "tools/parity-cli/testdata/build-round-trip/synthetic-poe1.xml";
const POE2_BUILD: &str = "tools/parity-cli/testdata/build-round-trip/synthetic-poe2.json";

/// Runs `inputs` with a synthetic bundle (`poe1` or `poe2`).
fn run_with(
    game: &str,
    inputs: &[Input],
    anonymous: bool,
    overlay: Option<&[u8]>,
) -> Result<Outputs, String> {
    let dir = repo(&format!("crates/loom-data/tests/fixtures/bundle-{game}"));
    let bundle = crate::load_bundle(&dir).unwrap();
    let manifest_sha256 = sha256_hex(&std::fs::read(dir.join("bundle.json")).unwrap());
    let data = BundleData::new(&bundle);
    let resolver = Resolver::new(&data).unwrap();
    let overlay = overlay
        .map(|bytes| OverlayData::read(bytes, &bundle, &manifest_sha256))
        .transpose()?;
    let request = Request {
        bundle: &bundle,
        resolver: &resolver,
        data: &data,
        bundle_record: BundleRecord {
            game: game.to_owned(),
            steam_build: bundle.manifest.source.steam_build,
            data_schema: bundle.manifest.data_schema,
            manifest_sha256,
        },
        overlay: overlay.as_ref().map(|o| {
            (
                o,
                OverlayRecord {
                    sha256: "o".to_owned(),
                    rows: o.counts(),
                },
            )
        }),
        scope: None,
        anonymous,
        tool: ToolRecord::default(),
    };
    run(inputs, &request)
}

#[test]
fn a_poe1_build_should_give_a_document_with_every_slice_field() {
    let out = run_with("poe1", &[input(POE1_BUILD)], false, None).unwrap();
    let b = &out.report.builds[0];
    assert_eq!(b.id, "synthetic-poe1.xml");
    assert_eq!(b.status, "ok");
    // P3-6 and P3-7 compute 17 fields; the synthetic bundle has no
    // Scion, so they are indeterminate (missing-data); 7 not computed.
    assert_eq!(b.state.as_deref(), Some("indeterminate"));
    assert_eq!(
        b.fields,
        Some(StateCounts {
            unsupported: 7,
            indeterminate: 17,
            ..StateCounts::default()
        })
    );
    let (id, json) = &out.documents[0];
    assert_eq!(id, "synthetic-poe1.xml");
    assert_eq!(
        b.document_sha256.as_deref(),
        Some(sha256_hex(json.as_bytes()).as_str())
    );
    let doc = calc_dto::from_json(json).unwrap();
    assert_eq!(doc.result.fields.len(), 24);
    assert_eq!(out.summary.field_reasons.get("not-computed"), Some(&7));
    assert_eq!(out.summary.field_reasons.get("missing-data"), Some(&17));
    assert_eq!(out.summary.build_states.get("indeterminate"), Some(&1));
    assert_eq!(out.summary.timing.builds, 1);
}

#[test]
fn two_runs_should_give_the_same_documents_and_report() {
    let inputs = [input(POE1_BUILD)];
    let a = run_with("poe1", &inputs, false, None).unwrap();
    let b = run_with("poe1", &inputs, false, None).unwrap();
    assert_eq!(a.documents, b.documents);
    assert_eq!(pretty(&a.report).unwrap(), pretty(&b.report).unwrap());
}

#[test]
fn a_poe2_build_should_give_an_unsupported_result() {
    let out = run_with("poe2", &[input(POE2_BUILD)], false, None).unwrap();
    let doc = calc_dto::from_json(&out.documents[0].1).unwrap();
    assert!(doc.result.fields.is_empty());
    assert_eq!(out.report.builds[0].state.as_deref(), Some("unsupported"));
}

#[test]
fn failures_should_name_their_step_and_code() {
    let inputs = [
        bytes_input("bad.xml", Format::PobXml, b"<PathOfBuilding"),
        bytes_input("latin1.xml", Format::PobXml, &[0xff, 0xfe]),
        // A PoE2 build with the PoE1 bundle does not resolve.
        input(POE2_BUILD),
    ];
    let out = run_with("poe1", &inputs, false, None).unwrap();
    let steps: Vec<(Option<Step>, &str)> = out
        .report
        .builds
        .iter()
        .map(|b| (b.step, b.code.as_deref().unwrap_or("")))
        .collect();
    assert_eq!(steps[0], (Some(Step::Read), "IMPORT_XML"));
    assert_eq!(steps[1], (Some(Step::Read), "IMPORT_ENCODING"));
    assert_eq!(steps[2].0, Some(Step::Resolve));
    assert!(out.documents.is_empty());
    assert_eq!(out.summary.failed, 3);
    assert_eq!(out.summary.failures.values().sum::<usize>(), 3);
}

#[test]
fn ids_should_be_file_names_or_hashes_and_unique() {
    let a = input(POE1_BUILD);
    let out = run_with("poe1", std::slice::from_ref(&a), true, None).unwrap();
    assert_eq!(out.report.builds[0].id, sha256_hex(&a.bytes));
    assert!(out.report.anonymous);
    let err = run_with("poe1", &[a.clone(), a.clone()], false, None)
        .err()
        .unwrap();
    assert!(err.contains("two builds are named"), "{err}");
    let err = run_with("poe1", &[a.clone(), a], true, None).err().unwrap();
    assert!(err.contains("same bytes"), "{err}");
}

/// An overlay for the synthetic PoE1 bundle.
fn overlay(change: impl FnOnce(&mut Value)) -> Vec<u8> {
    let dir = repo("crates/loom-data/tests/fixtures/bundle-poe1");
    let bundle = crate::load_bundle(&dir).unwrap();
    let mut v = json!({
        "format": 1,
        "inputs": {
            "game": "poe1",
            "steam_build": bundle.manifest.source.steam_build,
            "bundle_manifest_sha256": sha256_hex(&std::fs::read(dir.join("bundle.json")).unwrap()),
            "data_schema": bundle.manifest.data_schema,
            "pob_export_sha256": "x", "pob_version": "2.67.2", "tree_version": "3_29", "rules_sha256": "x"
        },
        "rows": [
            {"table": "skills", "key": "A", "field": "level.1.stat.x", "pob": 0, "bundle": 1, "category": "pob-derivation", "rule": "r"},
            {"table": "monster_levels", "key": "1", "field": "armour", "pob": 5, "bundle": 6, "category": "pob-derivation", "rule": "r"}
        ]
    });
    change(&mut v);
    v.to_string().into_bytes()
}

#[test]
fn an_overlay_should_hold_pobs_values_for_this_bundle_only() {
    let dir = repo("crates/loom-data/tests/fixtures/bundle-poe1");
    let bundle = crate::load_bundle(&dir).unwrap();
    let sha = sha256_hex(&std::fs::read(dir.join("bundle.json")).unwrap());
    let o = OverlayData::read(&overlay(|_| {}), &bundle, &sha).unwrap();
    assert_eq!(
        o.pob_value("skills", "A", "level.1.stat.x"),
        Some(&json!(0))
    );
    assert_eq!(o.pob_value("skills", "B", "level.1.stat.x"), None);
    assert_eq!(o.counts().get("monster_levels"), Some(&1));
    // It calculates with the bundle's data version.
    assert_eq!(o.data_version(), bundle.data_version());
    let refused = |change: Box<dyn FnOnce(&mut Value)>| {
        OverlayData::read(&overlay(change), &bundle, &sha)
            .err()
            .unwrap()
    };
    assert!(
        refused(Box::new(
            |v| v["inputs"]["bundle_manifest_sha256"] = "b".into()
        ))
        .contains("not this bundle")
    );
    assert!(refused(Box::new(|v| v["inputs"]["game"] = "poe2".into())).contains("not this bundle"));
    assert!(refused(Box::new(|v| v["format"] = 2.into())).contains("format"));
    assert!(
        refused(Box::new(|v| {
            let first = v["rows"][0].clone();
            v["rows"].as_array_mut().unwrap().push(first);
        }))
        .contains("twice")
    );
    // A run with the overlay records it; rows of tables the engine does
    // not read change nothing.
    let plain = run_with("poe1", &[input(POE1_BUILD)], false, None).unwrap();
    let with = run_with("poe1", &[input(POE1_BUILD)], false, Some(&overlay(|_| {}))).unwrap();
    assert_eq!(plain.documents, with.documents);
    assert_eq!(with.report.overlay.unwrap().rows.len(), 2);
}

#[test]
fn an_overlay_should_refuse_a_skill_it_has_a_row_for() {
    let dir = repo("crates/loom-data/tests/fixtures/bundle-poe1");
    let bundle = crate::load_bundle(&dir).unwrap();
    let sha = sha256_hex(&std::fs::read(dir.join("bundle.json")).unwrap());
    let o = OverlayData::read(
        &overlay(|v| {
            v["rows"].as_array_mut().unwrap().push(json!(
                {"table": "skills", "key": "TestBolt", "field": "types", "pob": "x", "bundle": "y", "category": "representation", "rule": "r"}
            ));
        }),
        &bundle,
        &sha,
    )
    .unwrap();
    // The skill with a row reads as missing (granted effect and stat set);
    // the others, its gem, and its active skill row read as the bundle's.
    assert!(o.granted_effect("TestBolt").is_none());
    assert!(o.stat_set("TestBolt").is_none());
    assert!(bundle.granted_effect("TestBolt").is_some());
    assert!(o.granted_effect("TestStrike").is_some());
    assert!(o.skill_gem("Test/Gems/Bolt").is_some());
    assert!(o.gem_effect("TestBolt").is_some());
    assert!(o.active_skill("test_bolt").is_some());
}

/// A row for the overlay.
fn row(table: &str, key: &str, field: &str, pob: Value) -> Value {
    json!({"table": table, "key": key, "field": field, "pob": pob, "bundle": 0, "category": "patch-delta", "rule": "r"})
}

#[test]
fn an_overlay_should_give_pobs_values_to_the_engines_reads() {
    let dir = repo("crates/loom-data/tests/fixtures/bundle-poe1");
    let bundle = crate::load_bundle(&dir).unwrap();
    let sha = sha256_hex(&std::fs::read(dir.join("bundle.json")).unwrap());
    let with = |rows: Vec<Value>| {
        overlay(move |v| {
            for r in rows {
                v["rows"].as_array_mut().unwrap().push(r);
            }
        })
    };
    // An overlay row replaces a bundle value; it never adds a row. The
    // synthetic bundle's character template is not the game's, so its
    // constants are not read.
    let o = OverlayData::read(
        &with(vec![row(
            "character_constants",
            "life_per_level",
            "value",
            json!(10),
        )]),
        &bundle,
        &sha,
    )
    .unwrap();
    assert_eq!(o.character_constant("life_per_level"), None);
    // With the game's template, the overlay's value is the one read.
    let mut bundle = bundle;
    bundle
        .passives
        .as_mut()
        .unwrap()
        .object_templates
        .iter_mut()
        .find(|t| t.id == "test/chars/character")
        .unwrap()
        .id = loom_calc::CHARACTER_TEMPLATE.to_owned();
    assert_eq!(bundle.character_constant("test_life_per_level"), Some(12));
    let o = OverlayData::read(
        &with(vec![
            row("characters", "Test Sage", "intelligence", json!(40)),
            row(
                "character_constants",
                "test_life_per_level",
                "value",
                json!(10),
            ),
        ]),
        &bundle,
        &sha,
    )
    .unwrap();
    let ours = bundle.character("Test Sage").unwrap();
    let pob = o.character("Test Sage").unwrap();
    assert_eq!(pob.intelligence, 40);
    assert_eq!(
        (pob.strength, pob.base_life),
        (ours.strength, ours.base_life)
    );
    assert_eq!(o.character_constant("test_life_per_level"), Some(10));
    assert_eq!(o.character("Nobody"), None);
    assert_eq!(o.passive_stats(101), bundle.passive_stats(101));
    assert_eq!(
        o.mastery_effect_stats(4242),
        bundle.mastery_effect_stats(4242)
    );
    // Rows the engine reads but cannot apply, or not as integers: refused.
    for (r, message) in [
        (
            row("passives", "101", "stats", json!(["+10 to Life"])),
            "cannot be applied",
        ),
        (
            row("mastery_effects", "4242", "stats", json!([])),
            "cannot be applied",
        ),
        (
            row("characters", "Test Sage", "strength", json!(1.5)),
            "not an integer",
        ),
    ] {
        let err = OverlayData::read(&with(vec![r]), &bundle, &sha)
            .err()
            .unwrap();
        assert!(err.contains(message), "{err}");
    }
    // A passive row the engine does not read changes nothing.
    assert!(
        OverlayData::read(
            &with(vec![row("passives", "101", "ascendancy", json!("X"))]),
            &bundle,
            &sha
        )
        .is_ok()
    );
}

#[test]
fn an_overlay_run_should_calculate_with_pobs_values() {
    // The synthetic build as a Test Sage (the synthetic bundle's class).
    let mut sage = input(POE1_BUILD);
    sage.bytes = String::from_utf8(sage.bytes)
        .unwrap()
        .replace("className=\"Scion\"", "className=\"Test Sage\"")
        .into_bytes();
    let doc = |out: &Outputs| calc_dto::from_json(&out.documents[0].1).unwrap();
    let int = |out: &Outputs| {
        let d = doc(out);
        let f = d
            .result
            .fields
            .iter()
            .find(|f| f.field == "intelligence")
            .unwrap();
        serde_json::to_value(f.value.as_ref().unwrap()).unwrap()
    };
    let plain = run_with("poe1", std::slice::from_ref(&sage), false, None).unwrap();
    let rows = overlay(|v| {
        v["rows"].as_array_mut().unwrap().push(row(
            "characters",
            "Test Sage",
            "intelligence",
            json!(40),
        ));
    });
    let with = run_with("poe1", &[sage], false, Some(&rows)).unwrap();
    assert_eq!(int(&plain), json!(32.0));
    assert_eq!(int(&with), json!(40.0));
}

#[test]
fn write_and_read_should_round_trip_and_check_hashes() {
    let out = run_with("poe1", &[input(POE1_BUILD)], false, None).unwrap();
    let dir = std::env::temp_dir().join(format!("loom-calc-run-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    write(&dir, &out).unwrap();
    assert!(write(&dir, &out).unwrap_err().contains("already exists"));
    let (report, _) = read_report(&dir).unwrap();
    assert_eq!(report, out.report);
    let doc = read_document(&dir, &report.builds[0]).unwrap();
    assert_eq!(doc.result.fields.len(), 24);
    for name in ["summary.json", "timing.json"] {
        assert!(dir.join(name).is_file(), "{name}");
    }
    let mut changed = report.builds[0].clone();
    changed.document_sha256 = Some("0".repeat(64));
    assert!(
        read_document(&dir, &changed)
            .unwrap_err()
            .contains("differs")
    );
    let mut bad = report;
    bad.builds[0].id = "../x".to_owned();
    std::fs::write(dir.join("report.json"), pretty(&bad).unwrap()).unwrap();
    assert!(read_report(&dir).unwrap_err().contains("not a file name"));
    std::fs::remove_dir_all(&dir).unwrap();
}
