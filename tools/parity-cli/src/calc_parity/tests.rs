//! Tests of calc-parity: scripted oracle runs and successor runs (results
//! built with the engine's graph and written by the real DTO writer), one
//! per outcome (Phase 3 plan, step 5, "Accept"); the breakdown reader on
//! every breakdown the public fixtures produce.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use loom_calc::{
    CalcResult, Field, FieldResult, FieldState, Graph, Node, NodeIndex, Op, Quantity, Reasons,
    Source, Term, UnsupportedReason,
};
use loom_domain::Game;
use serde_json::{Value, json};

use super::breakdown::{self, Format};
use super::*;
use crate::calc_run::{
    BuildRecord, BundleRecord, OverlayRecord, Report as RunReport, Step, ToolRecord,
};

const SLICE: &[u8] = include_bytes!("../../../../docs/phase-3/slice-fields.json");
const LINES: &[u8] = include_bytes!("../../testdata/calc-parity/breakdown-lines.json");

/// A fresh temporary folder for one test.
fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("loom-calc-parity-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A canonical oracle output document.
fn oracle_doc(main: Value, breakdowns: Value) -> Value {
    json!({
        "protocol": 4, "stage": "done", "error": null, "partial": false,
        "modules": {}, "settings": {}, "shims": [], "probe": {},
        "breakdown_source": "calcsTab.calcsEnv.player.breakdown",
        "breakdowns": breakdowns, "main_output": main
    })
}

/// One oracle fixture: id, output (`None`: failed with the class), and
/// the manifest's expectation.
struct OracleFixture {
    id: &'static str,
    output: Option<Value>,
    class: Option<&'static str>,
    expect: Option<(&'static str, bool)>,
}

impl OracleFixture {
    fn ok(id: &'static str, output: Value) -> Self {
        Self {
            id,
            output: Some(output),
            class: None,
            expect: None,
        }
    }
}

const SHA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn write_oracle(dir: &Path, fixtures: &[OracleFixture]) {
    std::fs::create_dir_all(dir.join("outputs")).unwrap();
    let list: Vec<Value> = fixtures
        .iter()
        .map(|f| {
            if let Some(o) = &f.output {
                std::fs::write(
                    dir.join("outputs").join(format!("{}.json", f.id)),
                    serde_json::to_string_pretty(o).unwrap(),
                )
                .unwrap();
            }
            json!({
                "id": f.id,
                "sha256": SHA,
                "status": if f.output.is_some() { "ok" } else { "failed" },
                "class": f.class,
                "expect": f.expect.map(|e| e.0),
                "met": f.expect.map(|e| e.1),
            })
        })
        .collect();
    let report = json!({"platform": "test", "tool": {"commit": "t"}, "fixtures": list});
    std::fs::write(dir.join("report.json"), report.to_string()).unwrap();
}

/// A successor build: a result, or where it stopped.
type SuccessorBuild = (&'static str, Result<CalcResult, Step>);

fn write_successor(dir: &Path, overlay: bool, builds: Vec<SuccessorBuild>) {
    std::fs::create_dir_all(dir.join("documents")).unwrap();
    let records = builds
        .into_iter()
        .map(|(id, r)| match r {
            Ok(result) => {
                let json = calc_dto::to_json(&result).unwrap();
                std::fs::write(dir.join("documents").join(format!("{id}.json")), &json).unwrap();
                let doc = calc_dto::from_json(&json).unwrap();
                BuildRecord {
                    id: id.to_owned(),
                    sha256: SHA.to_owned(),
                    status: "ok".to_owned(),
                    step: None,
                    code: None,
                    document_sha256: Some(oracle::sha256_hex(json.as_bytes())),
                    state: Some(calc_run::state_name(&doc.result.state)),
                    fields: None,
                }
            }
            Err(step) => BuildRecord {
                id: id.to_owned(),
                sha256: SHA.to_owned(),
                status: "failed".to_owned(),
                step: Some(step),
                code: Some("TEST_CODE".to_owned()),
                document_sha256: None,
                state: None,
                fields: None,
            },
        })
        .collect();
    let report = RunReport {
        format: calc_run::FORMAT,
        tool: ToolRecord::default(),
        bundle: BundleRecord {
            game: "poe1".to_owned(),
            steam_build: Some(1),
            data_schema: 2,
            manifest_sha256: SHA.to_owned(),
        },
        overlay: overlay.then(|| OverlayRecord {
            sha256: SHA.to_owned(),
            rows: BTreeMap::new(),
        }),
        scope: None,
        anonymous: false,
        builds: records,
    };
    std::fs::write(dir.join("report.json"), calc_run::pretty(&report).unwrap()).unwrap();
}

/// `base * (1 + increased / 100)` as the engine's chain builds it, with
/// the chain's terms; returns the result node and its value.
fn chain(graph: &mut Graph, quantity: Quantity, base: f64, increased: f64) -> (NodeIndex, f64) {
    let c = graph
        .push(
            Node::Contribution {
                source: Source::Character,
                quantity,
                op: Op::Base,
                value: base,
            },
            base,
        )
        .unwrap();
    let b = graph
        .derive(Node::Sum {
            term: Term::Base,
            terms: vec![c],
        })
        .unwrap();
    let i = graph
        .push(
            Node::Contribution {
                source: Source::Rule("test"),
                quantity,
                op: Op::Increased,
                value: increased,
            },
            increased,
        )
        .unwrap();
    let s = graph
        .derive(Node::Sum {
            term: Term::Step("increased-sum"),
            terms: vec![i],
        })
        .unwrap();
    let pf = graph
        .derive(Node::PercentFactor {
            term: Term::Increased,
            of: s,
        })
        .unwrap();
    let m = graph
        .derive(Node::Product {
            term: Term::More,
            factors: vec![],
        })
        .unwrap();
    let r = graph
        .derive(Node::Product {
            term: Term::Result,
            factors: vec![b, pf, m],
        })
        .unwrap();
    (r, graph.value(r).unwrap())
}

/// A PoE1 result: Life complete (`base`, `increased`), Mana partial,
/// every other slice field not computed.
fn result(base: f64, increased: f64) -> (CalcResult, f64) {
    let mut graph = Graph::new();
    let (life, value) = chain(&mut graph, Quantity::Life, base, increased);
    let (mana, mana_value) = chain(&mut graph, Quantity::Mana, 500.0, 0.0);
    let mut fields: BTreeMap<Field, FieldResult> = loom_calc::poe1::SLICE
        .iter()
        .map(|f| (*f, FieldResult::unsupported(UnsupportedReason::NotComputed)))
        .collect();
    fields.insert(
        Field::Life,
        FieldResult {
            value: Some(value),
            node: Some(life),
            state: FieldState::Complete,
        },
    );
    fields.insert(
        Field::Mana,
        FieldResult {
            value: Some(mana_value),
            node: Some(mana),
            state: FieldState::Partial(
                [UnsupportedReason::DependsOnNotComputed {
                    field: Field::Strength,
                }]
                .into(),
            ),
        },
    );
    let state = CalcResult::worst(&fields);
    (
        CalcResult {
            game: Game::Poe1,
            fields,
            graph,
            state,
            reasons: Reasons::new(),
        },
        value,
    )
}

/// The oracle output matching `result(2000, 35)` (Life 2700).
fn life_output(life: Value, lines: Value) -> Value {
    oracle_doc(
        json!({"Life": life, "Mana": 500}),
        json!({"Life": lines, "Mana": null}),
    )
}

fn life_lines(total: &str) -> Value {
    json!([
        "2000 ^8(base)",
        "x 1.35 ^8(increased/reduced)",
        format!("= {total}")
    ])
}

struct Case {
    dir: PathBuf,
    request: Request,
}

fn case(name: &str, oracle: &[OracleFixture], successor: Vec<SuccessorBuild>) -> Case {
    let dir = temp(name);
    write_oracle(&dir.join("oracle"), oracle);
    write_successor(&dir.join("successor"), false, successor);
    let slice = dir.join("slice.json");
    std::fs::write(&slice, SLICE).unwrap();
    Case {
        request: Request {
            oracle: dir.join("oracle"),
            successor: dir.join("successor"),
            slice,
            register: None,
            overlay_run: None,
            manifest: None,
            out: dir.join("out"),
        },
        dir,
    }
}

impl Case {
    fn with_register(mut self, entries: Value) -> Self {
        let path = self.dir.join("register.json");
        std::fs::write(&path, json!({"format": 1, "entries": entries}).to_string()).unwrap();
        self.request.register = Some(path);
        self
    }

    fn with_overlay(mut self, builds: Vec<SuccessorBuild>) -> Self {
        let dir = self.dir.join("overlay");
        write_successor(&dir, true, builds);
        self.request.overlay_run = Some(dir);
        self
    }

    fn compare(&self) -> Report {
        compare(&self.request).unwrap()
    }
}

impl Drop for Case {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn entry(id: &str, field: &str, oracle: Value, successor: Value, kind: &str) -> Value {
    json!({
        "id": id, "mechanic": "test", "fixture": "a.xml", "field": field,
        "comparison": {"type": "exact"}, "oracle_value": oracle, "successor_value": successor,
        "kind": kind, "evidence": ["test"], "test": "calc_parity::tests", "added": "2026-11-11",
        "status": "accepted"
    })
}

fn field<'r>(r: &'r Report, id: &str) -> &'r FieldComparison {
    r.builds[0].fields.iter().find(|f| f.field == id).unwrap()
}

fn life_breakdown(r: &Report) -> &BreakdownComparison {
    r.builds[0]
        .breakdowns
        .iter()
        .find(|b| b.field == "Life")
        .unwrap()
}

#[test]
fn a_complete_field_equal_to_the_oracle_should_match_with_its_breakdown() {
    let (r, life) = result(2000.0, 35.0);
    assert_eq!(Format::G.print(life), "2700");
    let c = case(
        "match",
        &[OracleFixture::ok(
            "a.xml",
            life_output(json!(life), life_lines("2700")),
        )],
        vec![("a.xml", Ok(r))],
    );
    let report = c.compare();
    assert_eq!(report.status, "no_difference");
    assert_eq!(report.class, None);
    let life_field = field(&report, "Life");
    assert_eq!(life_field.outcome, Outcome::Match);
    assert_eq!(life_field.pointer, "/main_output/Life");
    let b = life_breakdown(&report);
    assert_eq!(b.outcome, BreakdownOutcome::Match);
    let terms: Vec<(&str, Option<&str>)> = b
        .terms
        .iter()
        .map(|t| (t.term.as_str(), t.successor.as_deref()))
        .collect();
    assert_eq!(
        terms,
        [
            ("base", Some("2000")),
            ("increased", Some("1.35")),
            ("more", Some("1.00")),
            ("total", Some("2700"))
        ]
    );
    // The optional more term: PoB prints none, ours is 1.
    assert!(b.terms[2].oracle.is_none() && b.terms[2].equal);
    // Every other field is listed, not passed.
    assert_eq!(report.counts.fields["match"], 1);
    assert_eq!(report.counts.fields["partial"], 1);
    assert_eq!(report.counts.fields["not-computed"], 22);
    // Mana: partial, not compared, with its reasons.
    let mana = field(&report, "Mana");
    assert_eq!(mana.outcome, Outcome::Partial);
    assert_eq!(mana.reasons, ["depends-on-not-computed"]);
    assert_eq!(mana.class, None);
    let str_field = field(&report, "Str");
    assert_eq!(str_field.outcome, Outcome::NotComputed);
    assert_eq!(str_field.reasons, ["not-computed"]);
}

#[test]
fn a_field_within_its_rule_should_pass_and_say_so() {
    let (r, life) = result(2000.0, 35.0);
    let c = case(
        "tolerance",
        &[OracleFixture::ok(
            "a.xml",
            life_output(json!(life + 0.5), life_lines("2700.5")),
        )],
        vec![("a.xml", Ok(r))],
    );
    let mut slice: Value = serde_json::from_slice(SLICE).unwrap();
    for f in slice["fields"].as_array_mut().unwrap() {
        if f["id"] == "Life" {
            f["comparison"] = json!({"rule": "abs", "tolerance": 1, "reason": "test"});
        }
    }
    std::fs::write(&c.request.slice, slice.to_string()).unwrap();
    let report = c.compare();
    assert_eq!(field(&report, "Life").outcome, Outcome::WithinTolerance);
    // The breakdown is compared at its printed decimals, without the rule.
    let total = &life_breakdown(&report).terms[3];
    assert_eq!(
        (total.oracle.as_deref(), total.successor.as_deref()),
        (Some("2700.5"), Some("2700"))
    );
    assert_eq!(total.class, Some(FailureClass::UnexpectedDifference));
}

#[test]
fn differences_should_be_registered_or_unexpected() {
    let (r, life) = result(2000.0, 35.0);
    let oracle = [OracleFixture::ok(
        "a.xml",
        life_output(json!(life + 1.0), life_lines("2701")),
    )];
    // Unregistered.
    let c = case("unregistered", &oracle, vec![("a.xml", Ok(r.clone()))]);
    let report = c.compare();
    assert_eq!(report.status, "unexpected");
    assert_eq!(report.class, Some(FailureClass::UnexpectedDifference));
    assert_eq!(field(&report, "Life").outcome, Outcome::Difference);
    assert_eq!(report.counts.unexpected_differences, 2);
    drop(c);
    // Both pointers registered: expected.
    let c = case("registered", &oracle, vec![("a.xml", Ok(r))]).with_register(json!([
        entry(
            "DIV-0001",
            "/main_output/Life",
            json!(2701),
            json!(2700),
            "game-truth"
        ),
        entry(
            "DIV-0002",
            "/breakdowns/Life/2",
            json!("2701"),
            json!("2700"),
            "game-truth"
        ),
    ]));
    let report = c.compare();
    assert_eq!(report.status, "expected");
    assert_eq!(report.class, Some(FailureClass::ExpectedDifference));
    assert_eq!(field(&report, "Life").entry.as_deref(), Some("DIV-0001"));
    assert_eq!(report.counts.expected_differences, 2);
    assert!(report.unused_entries.is_empty());
}

#[test]
fn an_entry_that_covers_nothing_should_be_listed_unused() {
    let (r, life) = result(2000.0, 35.0);
    let c = case(
        "unused",
        &[OracleFixture::ok(
            "a.xml",
            life_output(json!(life), life_lines("2700")),
        )],
        vec![("a.xml", Ok(r))],
    )
    .with_register(json!([entry(
        "DIV-0001",
        "/main_output/Life",
        json!(2701),
        json!(2700),
        "game-truth"
    )]));
    let report = c.compare();
    assert_eq!(report.unused_entries, ["DIV-0001"]);
}

#[test]
fn a_term_should_be_compared_at_its_printed_decimals() {
    // 1 + 35.49 / 100 prints as 1.35, as PoB's line does.
    let (r, life) = result(2000.0, 35.49);
    let printed = Format::G.print(life);
    assert_eq!(printed, "2709.8");
    let c = case(
        "decimals",
        &[OracleFixture::ok(
            "a.xml",
            life_output(json!(life), life_lines(&printed)),
        )],
        vec![("a.xml", Ok(r.clone()))],
    );
    assert_eq!(
        life_breakdown(&c.compare()).outcome,
        BreakdownOutcome::Match
    );
    drop(c);
    // PoB printing 1.36 is a term difference, at its line.
    let lines = json!([
        "2000 ^8(base)",
        "x 1.36 ^8(increased/reduced)",
        format!("= {printed}")
    ]);
    let c = case(
        "term-mismatch",
        &[OracleFixture::ok("a.xml", life_output(json!(life), lines))],
        vec![("a.xml", Ok(r))],
    );
    let report = c.compare();
    let b = life_breakdown(&report);
    assert_eq!(b.outcome, BreakdownOutcome::Difference);
    let inc = &b.terms[1];
    assert_eq!(inc.pointer, "/breakdowns/Life/1");
    assert_eq!(
        (inc.oracle.as_deref(), inc.successor.as_deref()),
        (Some("1.36"), Some("1.35"))
    );
    assert_eq!(inc.class, Some(FailureClass::UnexpectedDifference));
    assert_eq!(field(&report, "Life").outcome, Outcome::Match);
    assert_eq!(report.class, Some(FailureClass::UnexpectedDifference));
}

#[test]
fn a_missing_breakdown_should_pass_only_where_pob_writes_none() {
    // PoB writes no Life breakdown without increased or more; ours has
    // an increased factor of 1.35, which PoB would print.
    let (r, life) = result(2000.0, 35.0);
    let c = case(
        "absent",
        &[OracleFixture::ok(
            "a.xml",
            life_output(json!(life), Value::Null),
        )],
        vec![("a.xml", Ok(r))],
    );
    let report = c.compare();
    let b = life_breakdown(&report);
    assert_eq!(b.outcome, BreakdownOutcome::Difference);
    assert_eq!(b.terms[0].term, "increased");
    assert_eq!(b.terms[0].pointer, "/breakdowns/Life/increased");
    drop(c);
    let (r, life) = result(2000.0, 0.0);
    let c = case(
        "absent-ok",
        &[OracleFixture::ok(
            "a.xml",
            life_output(json!(life), Value::Null),
        )],
        vec![("a.xml", Ok(r))],
    );
    assert_eq!(
        life_breakdown(&c.compare()).outcome,
        BreakdownOutcome::Absent
    );
}

#[test]
fn a_complete_field_the_oracle_lacks_should_be_a_difference() {
    let (r, _) = result(2000.0, 35.0);
    let c = case(
        "oracle-absent",
        &[OracleFixture::ok(
            "a.xml",
            oracle_doc(json!({"Mana": 500}), json!({})),
        )],
        vec![("a.xml", Ok(r))],
    );
    let report = c.compare();
    let life = field(&report, "Life");
    assert_eq!(life.outcome, Outcome::Difference);
    assert!(life.oracle.is_none());
    assert_eq!(life.class, Some(FailureClass::UnexpectedDifference));
}

#[test]
fn a_side_without_output_should_carry_its_class() {
    let (r, life) = result(2000.0, 35.0);
    let c = case(
        "failed",
        &[
            OracleFixture {
                id: "a.xml",
                output: None,
                class: Some("ORACLE_CALCULATION"),
                expect: None,
            },
            OracleFixture::ok("b.xml", life_output(json!(life), life_lines("2700"))),
            OracleFixture {
                id: "c.xml",
                output: None,
                class: Some("ORACLE_IMPORT"),
                expect: Some(("ORACLE_IMPORT", true)),
            },
            OracleFixture::ok("d.xml", life_output(json!(life), life_lines("2700"))),
        ],
        vec![
            ("a.xml", Ok(r.clone())),
            ("b.xml", Err(Step::Document)),
            ("c.xml", Err(Step::Read)),
            ("d.xml", Err(Step::Read)),
        ],
    );
    let report = c.compare();
    let by_id = |id: &str| report.builds.iter().find(|b| b.id == id).unwrap();
    assert_eq!(by_id("a.xml").status, "not_compared");
    assert_eq!(by_id("a.xml").class, Some(FailureClass::Calculation));
    assert_eq!(by_id("b.xml").class, Some(FailureClass::Protocol));
    assert_eq!(by_id("c.xml").status, "expected_failure");
    assert_eq!(by_id("c.xml").class, None);
    // The successor failing to import a build the oracle ran.
    assert_eq!(by_id("d.xml").class, Some(FailureClass::Import));
    assert_eq!(report.status, "failed");
    assert_eq!(report.class, Some(FailureClass::Protocol));
    assert_eq!(report.counts.not_compared, 3);
    assert_eq!(report.counts.expected_failures, 1);
}

#[test]
fn a_data_version_entry_should_need_the_overlay_run_to_match() {
    let (r, life) = result(2000.0, 35.0);
    let (overlay_matches, _) = result(2000.0, 35.05); // 2701
    let oracle = [OracleFixture::ok(
        "a.xml",
        life_output(json!(2701), life_lines("2701")),
    )];
    let register = json!([
        entry(
            "DIV-0001",
            "/main_output/Life",
            json!(2701),
            json!(life),
            "data-version"
        ),
        entry(
            "DIV-0002",
            "/breakdowns/Life/2",
            json!("2701"),
            json!("2700"),
            "data-version"
        ),
    ]);
    // Without an overlay run: refused.
    let c =
        case("dv-none", &oracle, vec![("a.xml", Ok(r.clone()))]).with_register(register.clone());
    let report = c.compare();
    let f = field(&report, "Life");
    assert_eq!(f.class, Some(FailureClass::UnexpectedDifference));
    assert!(
        f.note.as_deref().unwrap().contains("no --overlay-run"),
        "{f:?}"
    );
    assert_eq!(report.unused_entries, ["DIV-0001", "DIV-0002"]);
    drop(c);
    // The overlay run differs from the oracle too: refused.
    let c = case("dv-differs", &oracle, vec![("a.xml", Ok(r.clone()))])
        .with_register(register.clone())
        .with_overlay(vec![("a.xml", Ok(r.clone()))]);
    let report = c.compare();
    let f = field(&report, "Life");
    assert_eq!(f.class, Some(FailureClass::UnexpectedDifference));
    assert!(
        f.note.as_deref().unwrap().contains("that run differs"),
        "{f:?}"
    );
    drop(c);
    // The overlay run matches the oracle: accepted.
    assert_eq!(
        Format::G.print(
            overlay_matches
                .graph
                .value(overlay_matches.fields[&Field::Life].node.unwrap())
                .unwrap()
        ),
        "2701"
    );
    let c = case("dv-matches", &oracle, vec![("a.xml", Ok(r))])
        .with_register(register)
        .with_overlay(vec![("a.xml", Ok(overlay_matches))]);
    let report = c.compare();
    // The breakdown's total matches at its decimals; the field itself
    // must equal exactly, which 2000 * 1.3505 does not.
    let total = &life_breakdown(&report).terms[3];
    assert_eq!(total.entry.as_deref(), Some("DIV-0002"));
    assert_eq!(total.class, Some(FailureClass::ExpectedDifference));
}

#[test]
fn overlay_and_plain_runs_should_not_be_swapped() {
    let (r, life) = result(2000.0, 35.0);
    let c = case(
        "swap",
        &[OracleFixture::ok(
            "a.xml",
            life_output(json!(life), life_lines("2700")),
        )],
        vec![("a.xml", Ok(r.clone()))],
    );
    // A plain run as the overlay run.
    let mut request = c.request.clone();
    request.overlay_run = Some(c.request.successor.clone());
    assert!(
        compare(&request)
            .unwrap_err()
            .contains("not a data-overlay run")
    );
    // An overlay run as the run compared.
    write_successor(&c.dir.join("ov"), true, vec![("a.xml", Ok(r))]);
    request.overlay_run = None;
    request.successor = c.dir.join("ov");
    assert!(
        compare(&request)
            .unwrap_err()
            .contains("a data-overlay run")
    );
}

#[test]
fn breakdowns_should_not_be_compared_when_pobs_views_differ() {
    let (r, life) = result(2000.0, 35.0);
    let output = oracle_doc(
        json!({"Life": life, "Mana": 500, "TotalDPS": 115_856.632}),
        json!({
            "Life": life_lines("2700"),
            "TotalDPS": ["21163.3 ^8(average hit)", "x 3.97 ^8(cast rate)", "= 84002.1"]
        }),
    );
    let c = case(
        "views",
        &[OracleFixture::ok("a.xml", output)],
        vec![("a.xml", Ok(r))],
    );
    let report = c.compare();
    let b = &report.builds[0];
    assert_eq!(
        b.views_differ,
        ["TotalDPS: breakdown total 84002.1, main output 115856.6"]
    );
    let life = life_breakdown(&report);
    assert_eq!(life.outcome, BreakdownOutcome::NotCompared);
    assert_eq!(
        life.reason.as_deref(),
        Some("PoB's CALCS and main views differ")
    );
    assert_eq!(report.counts.views_differ, 1);
    assert_eq!(report.status, "incomplete");
    assert_eq!(report.exit_code(), INCOMPLETE_EXIT);
    assert!(
        b_incomplete(&report)
            .iter()
            .any(|i| i.starts_with("breakdown Life not compared"))
    );
}

#[test]
fn an_unknown_breakdown_line_should_fail_the_build() {
    let (r, life) = result(2000.0, 35.0);
    let lines = json!([
        "2000 ^8(base)",
        "x 1.35 ^8(increased/reduced)",
        "+ 5 ^8(flat)",
        "= 2700"
    ]);
    let c = case(
        "normalization",
        &[OracleFixture::ok("a.xml", life_output(json!(life), lines))],
        vec![("a.xml", Ok(r))],
    );
    let report = c.compare();
    assert_eq!(report.builds[0].status, "failed");
    assert_eq!(report.class, Some(FailureClass::Normalization));
    assert!(
        report.builds[0]
            .reason
            .as_deref()
            .unwrap()
            .contains("breakdown Life")
    );
    // A known line of another shape's term is refused too.
    let (r, life) = result(2000.0, 35.0);
    let lines = json!(["2000 ^8(base)", "x 3.97 ^8(cast rate)", "= 2700"]);
    let c = case(
        "foreign-term",
        &[OracleFixture::ok("a.xml", life_output(json!(life), lines))],
        vec![("a.xml", Ok(r))],
    );
    let report = c.compare();
    assert_eq!(report.class, Some(FailureClass::Normalization));
    assert!(
        report.builds[0]
            .reason
            .as_deref()
            .unwrap()
            .contains("not in the slice file")
    );
}

#[test]
fn a_mechanic_fixture_should_meet_its_targets() {
    let (r, life) = result(2000.0, 35.0);
    let c = case(
        "mechanic",
        &[OracleFixture::ok(
            "m.xml",
            life_output(json!(life), life_lines("2700")),
        )],
        vec![("m.xml", Ok(r))],
    );
    let manifest = c.dir.join("fixtures.json");
    let write_manifest = |targets: Value| {
        let m = json!({"format": 1, "fixtures": [{
            "id": "m.xml", "tier": "mechanic", "source": "local", "path": "mechanic/m.xml",
            "sha256": SHA, "expect": "ok", "targets": targets, "description": "test"
        }]});
        std::fs::write(&manifest, m.to_string()).unwrap();
    };
    let mut request = c.request.clone();
    request.manifest = Some(manifest.clone());
    write_manifest(json!(["Life"]));
    let report = compare(&request).unwrap();
    assert_eq!(report.status, "no_difference");
    assert!(report.builds[0].targets[0].met);
    // Str is not computed: the target is not met.
    write_manifest(json!(["Life", "Str"]));
    let report = compare(&request).unwrap();
    assert_eq!(report.class, Some(FailureClass::UnexpectedDifference));
    assert!(!report.builds[0].targets[1].met);
    write_manifest(json!(["Nope"]));
    assert!(compare(&request).unwrap_err().contains("not a slice field"));
}

#[test]
fn builds_on_one_side_should_be_unexpected() {
    let (r, life) = result(2000.0, 35.0);
    let c = case(
        "unpaired",
        &[OracleFixture::ok(
            "a.xml",
            life_output(json!(life), life_lines("2700")),
        )],
        vec![("a.xml", Ok(r.clone())), ("b.xml", Ok(r))],
    );
    let report = c.compare();
    assert_eq!(report.unpaired, [("b.xml".to_owned(), "successor")]);
    assert_eq!(report.class, Some(FailureClass::UnexpectedDifference));
}

#[test]
fn a_poe2_result_should_list_every_field_as_not_computed() {
    let c = case(
        "poe2",
        &[OracleFixture::ok(
            "a.xml",
            life_output(json!(2700), life_lines("2700")),
        )],
        vec![("a.xml", Ok(CalcResult::game_unsupported(Game::Poe2)))],
    );
    let report = c.compare();
    assert_eq!(report.counts.fields["not-computed"], 24);
    assert_eq!(field(&report, "Life").reasons, ["not-in-result"]);
    assert_eq!(report.status, "incomplete");
    assert_eq!(b_incomplete(&report), ["no slice field is complete"]);
    assert_eq!(report.exit_code(), INCOMPLETE_EXIT);
}

#[test]
fn run_should_write_the_report_and_refuse_an_existing_folder() {
    let (r, life) = result(2000.0, 35.0);
    let c = case(
        "write",
        &[OracleFixture::ok(
            "a.xml",
            life_output(json!(life), life_lines("2700")),
        )],
        vec![("a.xml", Ok(r))],
    );
    let report = run(&c.request).unwrap();
    let written: Value =
        serde_json::from_slice(&std::fs::read(c.request.out.join("report.json")).unwrap()).unwrap();
    assert_eq!(written["status"], "no_difference");
    assert_eq!(written["slice"]["revision"], 2);
    assert!(report.summary_markdown().starts_with("# calc-parity"));
    assert!(run(&c.request).unwrap_err().contains("already exists"));
}

#[test]
fn a_document_changed_after_the_run_should_be_refused() {
    let (r, life) = result(2000.0, 35.0);
    let c = case(
        "tampered",
        &[OracleFixture::ok(
            "a.xml",
            life_output(json!(life), life_lines("2700")),
        )],
        vec![("a.xml", Ok(r))],
    );
    let doc = c.request.successor.join("documents").join("a.xml.json");
    let mut text = std::fs::read_to_string(&doc).unwrap();
    text.push(' ');
    std::fs::write(&doc, text).unwrap();
    assert!(
        compare(&c.request)
            .unwrap_err()
            .contains("differs from the report")
    );
}

/// Every breakdown the oracle wrote for the public fixtures reads with
/// its slice shape's reader (Phase 3 plan, step 5: the parser on every
/// line shape the public fixtures produce).
#[test]
fn every_public_fixture_breakdown_should_read() {
    let slice = SliceFile::read(SLICE).unwrap();
    let data: Value = serde_json::from_slice(LINES).unwrap();
    assert!(data["fixtures"].as_array().unwrap().len() >= 30);
    let mut terms_seen: BTreeMap<String, std::collections::BTreeSet<&str>> = BTreeMap::new();
    let mut rows = 0;
    for f in &slice.fields {
        let Some(spec) = &f.breakdown else { continue };
        let entry = &data["breakdowns"][&spec.pob_key];
        assert!(
            entry.is_object(),
            "{}: no recorded breakdowns",
            spec.pob_key
        );
        for line in entry["lines"].as_array().into_iter().flatten() {
            let one = json!([line]);
            let parsed = match spec.shape {
                Shape::Chain => breakdown::parse_chain(&one),
                Shape::MinMaxTotal => breakdown::parse_min_max_total(&one),
                Shape::Slots => panic!("{}: lines for a slots breakdown", spec.pob_key),
            };
            let p = parsed.unwrap_or_else(|e| panic!("{}: {e}", spec.pob_key));
            assert!(
                spec.terms.iter().any(|t| t.term == p[0].term),
                "{}: {line} reads as {}, not a term of the slice file",
                spec.pob_key,
                p[0].term
            );
            terms_seen
                .entry(spec.pob_key.clone())
                .or_default()
                .insert(p[0].term);
        }
        for slots in entry["slots"].as_array().into_iter().flatten() {
            assert_eq!(spec.shape, Shape::Slots, "{}", spec.pob_key);
            rows += breakdown::parse_slots(slots)
                .unwrap_or_else(|e| panic!("{}: {e}", spec.pob_key))
                .len();
        }
    }
    let seen = |key: &str| -> Vec<&str> { terms_seen[key].iter().copied().collect() };
    assert_eq!(seen("Life"), ["base", "increased", "more", "total"]);
    assert_eq!(seen("FireResist"), ["max", "min", "total"]);
    assert_eq!(
        seen("TotalDPS"),
        [
            "average",
            "dps-multiplier",
            "quantity-multiplier",
            "rate",
            "total"
        ]
    );
    assert!(rows > 20, "{rows}");
    // A null breakdown is recorded for the chain fields PoB leaves out.
    assert_eq!(data["breakdowns"]["Life"]["null"], true);
}

/// The first build's `incomplete` list.
fn b_incomplete(r: &Report) -> Vec<&str> {
    r.builds[0].incomplete.iter().map(String::as_str).collect()
}

/// Sets a build's SHA-256 in a calc-run report.
fn set_build_sha(dir: &Path, id: &str, sha: &str) {
    let path = dir.join("report.json");
    let mut v: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    for b in v["builds"].as_array_mut().unwrap() {
        if b["id"] == id {
            b["sha256"] = sha.into();
        }
    }
    std::fs::write(&path, v.to_string()).unwrap();
}

#[test]
fn builds_read_from_different_bytes_should_not_be_compared() {
    let (r, life) = result(2000.0, 35.0);
    let c = case(
        "input-differs",
        &[OracleFixture::ok(
            "a.xml",
            life_output(json!(life), life_lines("2700")),
        )],
        vec![("a.xml", Ok(r))],
    );
    set_build_sha(&c.request.successor, "a.xml", &"b".repeat(64));
    let report = c.compare();
    let b = &report.builds[0];
    assert!(b.input_differs);
    assert_eq!(b.status, "not_compared");
    assert_eq!(b.class, Some(FailureClass::Protocol));
    assert!(b.fields.is_empty());
    assert_eq!(report.status, "failed");
}

#[test]
fn an_overlay_run_of_other_builds_should_be_refused() {
    let (r, life) = result(2000.0, 35.0);
    let c = case(
        "overlay-bytes",
        &[OracleFixture::ok(
            "a.xml",
            life_output(json!(life), life_lines("2700")),
        )],
        vec![("a.xml", Ok(r.clone()))],
    )
    .with_overlay(vec![("a.xml", Ok(r))]);
    let overlay = c.request.overlay_run.clone().unwrap();
    // The same ids from other bytes.
    set_build_sha(&overlay, "a.xml", &"b".repeat(64));
    let err = compare(&c.request).unwrap_err();
    assert!(err.contains("not the same builds"), "{err}");
    // Another tool commit.
    set_build_sha(&overlay, "a.xml", SHA);
    let path = overlay.join("report.json");
    let mut v: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    v["tool"]["commit"] = "other".into();
    std::fs::write(&path, v.to_string()).unwrap();
    assert!(compare(&c.request).unwrap_err().contains("tool commit"));
}

#[test]
fn a_mechanic_fixture_missing_from_the_runs_should_fail() {
    let (r, life) = result(2000.0, 35.0);
    let c = case(
        "mechanic-missing",
        &[OracleFixture::ok(
            "a.xml",
            life_output(json!(life), life_lines("2700")),
        )],
        vec![("a.xml", Ok(r))],
    );
    let manifest = c.dir.join("fixtures.json");
    let m = json!({"format": 1, "fixtures": [{
        "id": "m.xml", "tier": "mechanic", "source": "local", "path": "mechanic/m.xml",
        "sha256": SHA, "expect": "ok", "targets": ["Life"], "description": "test"
    }]});
    std::fs::write(&manifest, m.to_string()).unwrap();
    let mut request = c.request.clone();
    request.manifest = Some(manifest);
    let report = compare(&request).unwrap();
    assert_eq!(report.missing_mechanic, ["m.xml"]);
    assert_eq!(report.class, Some(FailureClass::UnexpectedDifference));
}

#[test]
fn a_required_term_missing_on_both_sides_should_differ() {
    // Our Life without a base node, PoB without a base line: not a pass.
    let mut graph = Graph::new();
    let c0 = graph
        .push(
            Node::Constant {
                label: "flat",
                value: 2000.0,
            },
            2000.0,
        )
        .unwrap();
    let r = graph
        .derive(Node::Product {
            term: Term::Result,
            factors: vec![c0],
        })
        .unwrap();
    let mut fields: BTreeMap<Field, FieldResult> = loom_calc::poe1::SLICE
        .iter()
        .map(|f| (*f, FieldResult::unsupported(UnsupportedReason::NotComputed)))
        .collect();
    fields.insert(
        Field::Life,
        FieldResult {
            value: Some(2000.0),
            node: Some(r),
            state: FieldState::Complete,
        },
    );
    let state = CalcResult::worst(&fields);
    let result = CalcResult {
        game: Game::Poe1,
        fields,
        graph,
        state,
        reasons: Reasons::new(),
    };
    let c = case(
        "required-term",
        &[OracleFixture::ok(
            "a.xml",
            life_output(
                json!(2000),
                json!(["x 1.00 ^8(increased/reduced)", "= 2000"]),
            ),
        )],
        vec![("a.xml", Ok(result))],
    );
    let report = c.compare();
    let b = life_breakdown(&report);
    let base = b.terms.iter().find(|t| t.term == "base").unwrap();
    assert!(base.oracle.is_none() && base.successor.is_none());
    assert!(!base.equal);
    assert_eq!(b.outcome, BreakdownOutcome::Difference);
}

#[test]
fn a_term_search_should_not_enter_another_fields_result() {
    // Mana derived from Life's result: Mana's own base is the one found.
    let mut graph = Graph::new();
    let (life, _) = chain(&mut graph, Quantity::Life, 2000.0, 35.0);
    let c = graph
        .push(
            Node::Constant {
                label: "half",
                value: 0.5,
            },
            0.5,
        )
        .unwrap();
    let mana = graph
        .derive(Node::Product {
            term: Term::Result,
            factors: vec![life, c],
        })
        .unwrap();
    let doc = {
        let mut fields: BTreeMap<Field, FieldResult> = BTreeMap::new();
        for (f, n) in [(Field::Life, life), (Field::Mana, mana)] {
            fields.insert(
                f,
                FieldResult {
                    value: graph.value(n),
                    node: Some(n),
                    state: FieldState::Complete,
                },
            );
        }
        let result = CalcResult {
            game: Game::Poe1,
            state: CalcResult::worst(&fields),
            fields,
            graph,
            reasons: Reasons::new(),
        };
        calc_dto::from_json(&calc_dto::to_json(&result).unwrap()).unwrap()
    };
    let ours = Ours { doc: &doc };
    let mana_node = u32::try_from(mana.get()).unwrap();
    assert_eq!(ours.find(mana_node, "base"), None);
    let life_node = u32::try_from(life.get()).unwrap();
    assert!(ours.find(life_node, "base").is_some());
}

#[test]
fn fixed_decimals_should_round_exact_ties_to_even() {
    // Rust's formatting rounds the exact binary value, ties to even, as
    // glibc's printf does; 0.125 and 2.5 are exact ties.
    assert_eq!(Format::Fixed(2).print(0.125), "0.12");
    assert_eq!(Format::Fixed(0).print(2.5), "2");
    assert_eq!(Format::Fixed(0).print(3.5), "4");
}
