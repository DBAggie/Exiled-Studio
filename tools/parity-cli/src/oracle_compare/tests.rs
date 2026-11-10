//! Tests of `oracle-compare`: field differences, register classification
//! (the intentional discrepancy pair), and run-directory pairing.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::*;

const DISCREPANCY_REGISTER: &str = "../oracle/fixtures/discrepancy-register.json";

/// A temporary directory removed on drop.
struct Temp(PathBuf);

impl Temp {
    fn new(name: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("loom-oracle-compare-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn crate_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

/// An output document like the adapter's for `minimal-level1.xml`, cut
/// down to the fields the tests use.
fn document(movement_speed: f64) -> Value {
    json!({
        "protocol": oracle::PROTOCOL,
        "stage": "done",
        "error": null,
        "partial": [],
        "modules": ["lua-utf8"],
        "settings": {"path": "/Settings.xml", "found": false},
        "shims": [],
        "probe": {"inflate": [], "writes_refused": []},
        "breakdown_source": "calcsTab.calcsEnv.player.breakdown",
        "breakdowns": {"Life": ["60 (base)"], "TotalDPS": null},
        "main_output": {
            "Life": 60,
            "MovementSpeedMod": movement_speed,
            "EffectiveMovementSpeedMod": movement_speed,
            "MainHand": {"Accuracy": 40}
        }
    })
}

fn write(path: &Path, value: &Value) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, serde_json::to_string_pretty(value).unwrap() + "\n").unwrap();
}

fn files_request(temp: &Temp, left: &Value, right: &Value, register: Option<PathBuf>) -> Request {
    let left_path = temp.0.join("minimal-level1.xml.json");
    let right_path = temp.0.join("discrepancy-movement-speed.xml.json");
    write(&left_path, left);
    write(&right_path, right);
    Request {
        left: left_path,
        right: right_path,
        register,
        fixture: None,
        out: temp.0.join("out"),
    }
}

#[test]
fn diff_should_report_changed_and_one_sided_fields() {
    let left = document(1.0);
    let mut right = document(1.0);
    right["main_output"]["Life"] = json!(72);
    right["main_output"]["New"] = json!(true);
    right["main_output"]["MainHand"]
        .as_object_mut()
        .unwrap()
        .remove("Accuracy");
    right["main_output"]["a/b"] = json!(1);
    right["breakdowns"]["Life"] = json!(["60 (base)", "x"]);
    // Not compared: the run's environment.
    right["modules"] = json!(["lua-utf8", "xml"]);
    right["shims"] = json!([{"name": "inflate"}]);
    let raw = diff(&left, &right);
    let fields: Vec<(&str, bool, bool)> = raw
        .iter()
        .map(|(f, l, r)| (f.as_str(), l.is_some(), r.is_some()))
        .collect();
    assert_eq!(
        fields,
        [
            ("/main_output/Life", true, true),
            ("/main_output/MainHand/Accuracy", true, false),
            ("/main_output/New", false, true),
            ("/main_output/a~1b", false, true),
            ("/breakdowns/Life/1", false, true),
        ]
    );
}

#[test]
fn diff_should_compare_numbers_as_doubles_and_types_strictly() {
    let left = document(1.0);
    let mut right = document(1.0);
    right["main_output"]["Life"] = json!(60.0);
    assert!(diff(&left, &right).is_empty());
    right["main_output"]["Life"] = json!("60");
    right["main_output"]["MainHand"] = json!(40);
    let raw = diff(&left, &right);
    assert_eq!(raw.len(), 2, "{raw:?}");
    assert_eq!(raw[0].0, "/main_output/Life");
    assert_eq!(raw[1].0, "/main_output/MainHand");
}

#[test]
fn compare_should_classify_the_discrepancy_pair() {
    // Without a register: both fields are unexpected (exit 20).
    let temp = Temp::new("pair");
    let request = files_request(&temp, &document(1.0), &document(1.1), None);
    let report = compare(&request).unwrap();
    assert_eq!(report.status, "unexpected");
    assert_eq!(report.class, Some(FailureClass::UnexpectedDifference));
    assert_eq!(report.class.unwrap().exit_code(), 20);
    let fixture = &report.fixtures[0];
    assert_eq!(fixture.id, "discrepancy-movement-speed.xml");
    let fields: Vec<&str> = fixture
        .differences
        .iter()
        .map(|d| d.field.as_str())
        .collect();
    assert_eq!(
        fields,
        [
            "/main_output/EffectiveMovementSpeedMod",
            "/main_output/MovementSpeedMod"
        ]
    );
    assert_eq!(report.counts.unexpected_differences, 2);

    // With the test register: both are expected (exit 19), and every
    // entry was used.
    let temp = Temp::new("pair-register");
    let request = files_request(
        &temp,
        &document(1.0),
        &document(1.1),
        Some(crate_path(DISCREPANCY_REGISTER)),
    );
    let report = compare(&request).unwrap();
    assert_eq!(report.status, "expected", "{}", report.summary_markdown());
    assert_eq!(report.class, Some(FailureClass::ExpectedDifference));
    assert_eq!(report.class.unwrap().exit_code(), 19);
    let entries: Vec<Option<&str>> = report.fixtures[0]
        .differences
        .iter()
        .map(|d| d.entry.as_deref())
        .collect();
    assert_eq!(entries, [Some("DIV-0002"), Some("DIV-0001")]);
    assert!(report.unused_entries.is_empty());
    assert_eq!(report.register.as_ref().unwrap().accepted, 2);
}

#[test]
fn compare_should_not_cover_other_values_or_fixtures() {
    // 1 -> 1.2: neither the exact entry nor the 0.125 tolerance covers it.
    let temp = Temp::new("pair-other-value");
    let request = files_request(
        &temp,
        &document(1.0),
        &document(1.2),
        Some(crate_path(DISCREPANCY_REGISTER)),
    );
    let report = compare(&request).unwrap();
    assert_eq!(report.status, "unexpected");
    let notes: Vec<&str> = report.fixtures[0]
        .differences
        .iter()
        .map(|d| d.note.as_deref().unwrap_or("-"))
        .collect();
    assert!(
        notes[0].starts_with("DIV-0002: 1 and 1.2 are further apart"),
        "{notes:?}"
    );
    assert!(
        notes[1].starts_with("DIV-0001 registers 1 -> 1.1"),
        "{notes:?}"
    );
    // Within the tolerance but not the exact value: one of each.
    let temp = Temp::new("pair-tolerance");
    let request = files_request(
        &temp,
        &document(1.0),
        &document(1.05),
        Some(crate_path(DISCREPANCY_REGISTER)),
    );
    let report = compare(&request).unwrap();
    let classes: Vec<FailureClass> = report.fixtures[0]
        .differences
        .iter()
        .map(|d| d.class)
        .collect();
    assert_eq!(
        classes,
        [
            FailureClass::ExpectedDifference,
            FailureClass::UnexpectedDifference
        ]
    );
    // Another fixture id: no entry applies.
    let temp = Temp::new("pair-other-fixture");
    let mut request = files_request(
        &temp,
        &document(1.0),
        &document(1.1),
        Some(crate_path(DISCREPANCY_REGISTER)),
    );
    request.fixture = Some("other.xml".to_owned());
    let report = compare(&request).unwrap();
    assert_eq!(report.status, "unexpected");
    assert!(
        report.fixtures[0]
            .differences
            .iter()
            .all(|d| d.entry.is_none() && d.note.is_none())
    );
    // The entries name a fixture that was not compared: not unused.
    assert!(report.unused_entries.is_empty());
}

#[test]
fn compare_should_list_unused_entries_and_identical_outputs() {
    let temp = Temp::new("identical");
    let request = files_request(
        &temp,
        &document(1.0),
        &document(1.0),
        Some(crate_path(DISCREPANCY_REGISTER)),
    );
    let report = compare(&request).unwrap();
    assert_eq!((report.status, report.class), ("identical", None));
    assert_eq!(report.unused_entries, ["DIV-0001", "DIV-0002"]);
    assert!(
        report
            .summary_markdown()
            .contains("covered nothing: DIV-0001, DIV-0002")
    );
}

#[test]
fn compare_should_refuse_bad_inputs() {
    let temp = Temp::new("bad");
    let mut request = files_request(&temp, &document(1.0), &json!({"protocol": 1}), None);
    assert!(compare(&request).unwrap_err().contains("protocol"));
    request.right = temp.0.clone();
    assert!(compare(&request).unwrap_err().contains("not one of each"));
    let invalid = temp.0.join("register.json");
    std::fs::write(&invalid, r#"{"format": 1, "entries": [{}]}"#).unwrap();
    let request = files_request(&temp, &document(1.0), &document(1.0), Some(invalid));
    assert!(compare(&request).unwrap_err().contains("invalid register"));
    std::fs::create_dir_all(&request.out).unwrap();
    assert!(run(&request).unwrap_err().contains("already exists"));
}

/// A fixture of [`run_dir`]: id, status, class, output.
type RunEntry = (
    &'static str,
    &'static str,
    Option<&'static str>,
    Option<Value>,
);

/// Writes an oracle-run directory with `fixtures` as `(id, status,
/// class, output)`.
fn run_dir(dir: &Path, platform: &str, fixtures: &[RunEntry]) {
    let list: Vec<Value> = fixtures
        .iter()
        .map(|(id, status, class, _)| {
            json!({"id": id, "sha256": "a".repeat(64), "status": status, "class": class})
        })
        .collect();
    write(
        &dir.join("report.json"),
        &json!({"format": 1, "platform": platform, "tool": {"commit": "abc"}, "fixtures": list}),
    );
    for (id, _, _, output) in fixtures {
        if let Some(output) = output {
            write(&dir.join("outputs").join(format!("{id}.json")), output);
        }
    }
}

#[test]
fn compare_should_pair_run_directories_by_fixture() {
    let temp = Temp::new("runs");
    let (left, right) = (temp.0.join("windows"), temp.0.join("linux"));
    run_dir(
        &left,
        "windows-x86_64",
        &[
            ("a.xml", "ok", None, Some(document(1.0))),
            ("b.xml", "ok", None, Some(document(1.0))),
            ("c.xml", "ok", None, Some(document(1.0))),
            ("left-only.xml", "ok", None, Some(document(1.0))),
        ],
    );
    run_dir(
        &right,
        "linux-x86_64",
        &[
            ("a.xml", "ok", None, Some(document(1.0))),
            ("b.xml", "ok", None, Some(document(1.1))),
            ("c.xml", "failed", Some("ORACLE_CALCULATION"), None),
            ("right-only.xml", "ok", None, Some(document(1.0))),
        ],
    );
    let request = Request {
        left,
        right,
        register: None,
        fixture: None,
        out: temp.0.join("out"),
    };
    let report = run(&request).unwrap();
    assert_eq!(report.mode, "runs");
    assert_eq!(report.left.platform.as_deref(), Some("windows-x86_64"));
    let statuses: Vec<(&str, &str)> = report
        .fixtures
        .iter()
        .map(|f| (f.id.as_str(), f.status))
        .collect();
    assert_eq!(
        statuses,
        [
            ("a.xml", "identical"),
            ("b.xml", "unexpected"),
            ("c.xml", "not_compared")
        ]
    );
    // The failure is never read as a difference, and its class wins.
    assert_eq!(report.fixtures[2].class, Some(FailureClass::Calculation));
    assert!(report.fixtures[2].differences.is_empty());
    assert_eq!(report.class, Some(FailureClass::Calculation));
    assert_eq!(report.status, "failed");
    assert_eq!(
        report.unpaired,
        [
            ("left-only.xml".to_owned(), "left"),
            ("right-only.xml".to_owned(), "right")
        ]
    );
    assert_eq!(report.counts.not_compared, 1);
    let written: Value =
        serde_json::from_slice(&std::fs::read(temp.0.join("out/report.json")).unwrap()).unwrap();
    assert_eq!(written["class"], "ORACLE_CALCULATION");
    let summary = std::fs::read_to_string(temp.0.join("out/summary.md")).unwrap();
    assert!(
        summary.contains("no output on the right side (ORACLE_CALCULATION)"),
        "{summary}"
    );
    assert!(summary.contains("left-only.xml (left)"), "{summary}");
    // --fixture is for two files only.
    let request = Request {
        fixture: Some("a.xml".to_owned()),
        out: temp.0.join("out2"),
        ..request
    };
    assert!(compare(&request).unwrap_err().contains("--fixture"));
}

#[test]
fn compare_should_report_unpaired_fixtures_as_unexpected() {
    let temp = Temp::new("unpaired");
    let (left, right) = (temp.0.join("l"), temp.0.join("r"));
    run_dir(&left, "p", &[("a.xml", "ok", None, Some(document(1.0)))]);
    run_dir(
        &right,
        "p",
        &[
            ("a.xml", "ok", None, Some(document(1.0))),
            ("b.xml", "ok", None, Some(document(1.0))),
        ],
    );
    let report = compare(&Request {
        left,
        right,
        register: None,
        fixture: None,
        out: temp.0.join("out"),
    })
    .unwrap();
    assert_eq!(report.class, Some(FailureClass::UnexpectedDifference));
    assert_eq!(report.counts.identical, 1);
}

#[test]
fn summary_should_show_each_difference_with_its_entry() {
    let temp = Temp::new("summary");
    let request = files_request(
        &temp,
        &document(1.0),
        &document(1.1),
        Some(crate_path(DISCREPANCY_REGISTER)),
    );
    let summary = compare(&request).unwrap().summary_markdown();
    assert!(
        summary.contains(
            "| `/main_output/MovementSpeedMod` | 1.0 | 1.1 | ORACLE_EXPECTED_DIFFERENCE | DIV-0001 |"
        ),
        "{summary}"
    );
}

#[test]
fn not_compared_roots_should_cover_every_other_document_field() {
    let mut roots: Vec<&str> = COMPARED
        .iter()
        .copied()
        .chain(NOT_COMPARED.iter().map(|(f, _)| *f))
        .collect();
    roots.sort_unstable();
    let mut fields = crate::oracle_output::FIELDS.to_vec();
    fields.sort_unstable();
    assert_eq!(roots, fields);
}

fn runs_request(temp: &Temp, left: PathBuf, right: PathBuf, register: Option<PathBuf>) -> Request {
    Request {
        left,
        right,
        register,
        fixture: None,
        out: temp.0.join("out"),
    }
}

/// Review blocker: an expected difference (19) must not hide an
/// unexpected one (20) or an unpaired fixture; a run failure still wins.
#[test]
fn report_class_should_not_let_expected_hide_unexpected() {
    let register = Some(crate_path(DISCREPANCY_REGISTER));
    let id = "discrepancy-movement-speed.xml";
    let cases: [(&str, &[RunEntry], FailureClass); 3] = [
        (
            "expected-and-unexpected",
            &[("other.xml", "ok", None, Some(document(1.1)))],
            FailureClass::UnexpectedDifference,
        ),
        (
            "expected-and-unpaired",
            &[],
            FailureClass::UnexpectedDifference,
        ),
        (
            "expected-and-failed",
            &[("other.xml", "failed", Some("ORACLE_TIMEOUT"), None)],
            FailureClass::Timeout,
        ),
    ];
    for (name, extra, class) in cases {
        let temp = Temp::new(name);
        let (left, right) = (temp.0.join("l"), temp.0.join("r"));
        let mut l = vec![(id, "ok", None, Some(document(1.0)))];
        let mut r = vec![(id, "ok", None, Some(document(1.1)))];
        l.push(("other.xml", "ok", None, Some(document(1.0))));
        r.extend(extra.iter().cloned());
        run_dir(&left, "p", &l);
        run_dir(&right, "p", &r);
        let report = compare(&runs_request(&temp, left, right, register.clone())).unwrap();
        assert_eq!(report.fixtures[0].status, "expected", "{name}");
        assert_eq!(report.class, Some(class), "{name}");
        assert_ne!(report.status, "expected", "{name}");
    }
}

#[test]
fn compare_should_match_failures_both_sides_expected() {
    let temp = Temp::new("expected-failure");
    let (left, right) = (temp.0.join("l"), temp.0.join("r"));
    let fixture = |met: bool| {
        json!({"id": "malformed.xml", "sha256": "a".repeat(64), "status": "failed",
            "class": "ORACLE_IMPORT", "expect": "ORACLE_IMPORT", "met": met})
    };
    for (dir, met) in [(&left, true), (&right, true)] {
        write(
            &dir.join("report.json"),
            &json!({"platform": "p", "tool": {}, "fixtures": [fixture(met)]}),
        );
    }
    let report = compare(&runs_request(&temp, left.clone(), right.clone(), None)).unwrap();
    assert_eq!(report.fixtures[0].status, "expected_failure");
    assert_eq!((report.status, report.class), ("identical", None));
    assert_eq!(report.counts.expected_failures, 1);
    // Unmet on a side: not compared, with both sides' classes named.
    write(
        &right.join("report.json"),
        &json!({"platform": "p", "tool": {}, "fixtures": [fixture(false)]}),
    );
    let report = compare(&runs_request(&temp, left, right, None)).unwrap();
    assert_eq!(report.fixtures[0].status, "not_compared");
    assert_eq!(report.class, Some(FailureClass::Import));
    let reason = report.fixtures[0].reason.as_deref().unwrap();
    assert!(
        reason.contains("left ORACLE_IMPORT, right ORACLE_IMPORT"),
        "{reason}"
    );
}

#[test]
fn compare_should_refuse_truncated_outputs() {
    let temp = Temp::new("truncated");
    let request = files_request(
        &temp,
        &json!({"protocol": 2}),
        &json!({"protocol": 2}),
        None,
    );
    assert!(
        compare(&request)
            .unwrap_err()
            .contains("exactly the fields")
    );
    let mut failed = document(1.0);
    failed["stage"] = json!("calculate");
    let request = files_request(&temp, &failed, &document(1.0), None);
    assert!(compare(&request).unwrap_err().contains("complete run"));
}

#[test]
fn compare_should_flag_differing_inputs_and_refuse_path_ids() {
    let temp = Temp::new("inputs");
    let (left, right) = (temp.0.join("l"), temp.0.join("r"));
    run_dir(&left, "p", &[("a.xml", "ok", None, Some(document(1.0)))]);
    run_dir(&right, "p", &[("a.xml", "ok", None, Some(document(1.0)))]);
    let mut report: Value =
        serde_json::from_slice(&std::fs::read(right.join("report.json")).unwrap()).unwrap();
    report["fixtures"][0]["sha256"] = json!("b".repeat(64));
    write(&right.join("report.json"), &report);
    let compared = compare(&runs_request(&temp, left.clone(), right.clone(), None)).unwrap();
    assert!(compared.fixtures[0].input_differs);
    assert_eq!(compared.fixtures[0].status, "identical");
    report["fixtures"][0]["id"] = json!("../a.xml");
    write(&right.join("report.json"), &report);
    let err = compare(&runs_request(&temp, left, right, None)).unwrap_err();
    assert!(err.contains("not a file name"), "{err}");
}

#[test]
fn compare_should_cover_an_absent_field_with_a_null_entry() {
    let temp = Temp::new("absent");
    let mut register: Value =
        serde_json::from_slice(&std::fs::read(crate_path(DISCREPANCY_REGISTER)).unwrap()).unwrap();
    register["entries"] = json!([{
        "id": "DIV-0003", "mechanic": "m", "fixture": "discrepancy-movement-speed.xml",
        "field": "/main_output/New", "comparison": {"type": "exact"},
        "oracle_value": null, "successor_value": true, "kind": "harness",
        "evidence": ["e"], "test": "t", "added": "2026-11-10", "status": "accepted"
    }]);
    let path = temp.0.join("register.json");
    write(&path, &register);
    let mut right = document(1.0);
    right["main_output"]["New"] = json!(true);
    let report = compare(&files_request(&temp, &document(1.0), &right, Some(path))).unwrap();
    let difference = &report.fixtures[0].differences[0];
    assert_eq!(difference.change, "only_right");
    assert_eq!(difference.entry.as_deref(), Some("DIV-0003"));
    assert_eq!(report.class, Some(FailureClass::ExpectedDifference));
}
