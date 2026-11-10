//! `oracle-run` classification with a scripted launcher (no LuaJIT): each
//! run writes a given document (or none) and reports a given exit. The
//! real process path is tested in `oracle_process`; the fake Lua adapters
//! in `tools/oracle/fixtures/fake-adapters/` run through the real runtime
//! in the ignored test at the end.

use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{Value, json};

use super::*;
use crate::oracle::Manifest;

const COMMITTED: &[u8] = include_bytes!("../../../oracle/manifest.json");

fn adapter() -> Adapter {
    Manifest::parse(COMMITTED).unwrap().adapter
}

fn shims() -> Value {
    json!([
        {"name": "inflate", "replaces": "Inflate", "installed": true, "detail": "zlib 1.3.1"},
        {"name": "write_guard", "replaces": "io.open", "installed": true, "detail": null}
    ])
}

fn done_document() -> Value {
    let breakdowns: serde_json::Map<String, Value> = adapter()
        .breakdowns
        .iter()
        .map(|b| (b.clone(), Value::Null))
        .collect();
    json!({
        "protocol": 2,
        "stage": "done",
        "error": null,
        "partial": [],
        "modules": ["lua-utf8", "xml"],
        "settings": {"path": "/Settings.xml", "found": false},
        "shims": shims(),
        "probe": {"inflate": [], "writes_refused": []},
        "breakdown_source": "calcsTab.calcsEnv.player.breakdown",
        "breakdowns": breakdowns,
        "main_output": {"Life": 60, "Mana": 40.5}
    })
}

fn failed_document(stage: &str, message: &str) -> Value {
    json!({
        "protocol": 2,
        "stage": stage,
        "error": {"source": "lua", "message": message},
        "partial": [],
        "modules": [],
        "settings": if stage == "startup" { Value::Null } else { json!({"path": "/Settings.xml", "found": false}) },
        "shims": shims(),
        "probe": {"inflate": [], "writes_refused": []},
        "breakdown_source": "calcsTab.calcsEnv.player.breakdown",
        "breakdowns": null,
        "main_output": null
    })
}

/// What one scripted run does.
#[derive(Clone)]
enum Step {
    /// Writes the document, exits with the code.
    Write(Value, i32),
    /// Writes raw bytes, exits with the code.
    Raw(&'static str, i32),
    /// Writes nothing, exits with the code.
    Exit(i32),
    /// Hits the time limit.
    Timeout,
    /// Writes a complete document, then hits the time limit.
    TimeoutAfterWriting,
    /// The output path is a directory, so it cannot be read as a file.
    Unreadable,
    /// Cannot start.
    NoStart,
}

struct Scripted {
    steps: Vec<Step>,
    next: Cell<usize>,
}

impl Scripted {
    fn new(steps: Vec<Step>) -> Self {
        Self {
            steps,
            next: Cell::new(0),
        }
    }

    fn same(step: Step) -> Self {
        Self::new(vec![step; 3])
    }
}

fn finished(code: Option<i32>, timed_out: bool) -> Finished {
    Finished {
        code,
        timed_out,
        stdout: String::new(),
        stderr: "stderr text".to_owned(),
        elapsed: Duration::from_millis(5),
    }
}

impl Launcher for Scripted {
    fn launch(&self, _fixture: &Path, output: &Path) -> Result<Finished, String> {
        let n = self.next.get();
        self.next.set(n + 1);
        match &self.steps[n] {
            Step::Write(value, code) => {
                std::fs::write(output, value.to_string()).unwrap();
                Ok(finished(Some(*code), false))
            }
            Step::Raw(text, code) => {
                std::fs::write(output, text).unwrap();
                Ok(finished(Some(*code), false))
            }
            Step::Exit(code) => Ok(finished(Some(*code), false)),
            Step::Timeout => Ok(finished(None, true)),
            Step::TimeoutAfterWriting => {
                std::fs::write(output, done_document().to_string()).unwrap();
                Ok(finished(None, true))
            }
            Step::Unreadable => {
                std::fs::create_dir_all(output).unwrap();
                Ok(finished(Some(0), false))
            }
            Step::NoStart => Err("cannot start /x/bin/luajit: NotFound".to_owned()),
        }
    }
}

struct Temp(PathBuf);

impl Temp {
    fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("loom-oracle-run-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn fixture() -> Fixture {
    Fixture {
        id: "build.xml".to_owned(),
        path: PathBuf::from("build.xml"),
        sha256: "0".repeat(64),
        expect: None,
    }
}

fn run_scripted(name: &str, launcher: &Scripted) -> FixtureResult {
    let temp = Temp::new(name);
    let scrub = Scrub::new(&[(Path::new("/oracle/dir"), "<oracle>")]);
    let runs = launcher.steps.len();
    run_fixture(
        &fixture(),
        runs,
        &temp.0.join("runs"),
        &adapter(),
        launcher,
        &scrub,
    )
    .unwrap()
}

#[test]
fn run_fixture_should_report_ok_for_identical_runs() {
    let result = run_scripted("ok", &Scripted::same(Step::Write(done_document(), 0)));
    let report = &result.report;
    assert_eq!(report.status, "ok");
    assert_eq!(report.class, None);
    assert_eq!(report.determinism.identical, 3);
    assert!(report.outcome.output_sha256.is_some());
    let text = result.output.unwrap();
    assert_eq!(
        report.outcome.output_sha256.as_deref(),
        Some(oracle::sha256_hex(text.as_bytes()).as_str())
    );
    // Canonical: keys sorted, numbers in shortest form.
    assert!(text.find("\"breakdown_source\"").unwrap() < text.find("\"stage\"").unwrap());
}

#[test]
fn run_fixture_should_report_partial_without_a_class() {
    let mut document = done_document();
    document["shims"][0]["installed"] = false.into();
    document["shims"][0]["detail"] = "cannot load the zlib library".into();
    document["probe"]["inflate"] = json!([{"input_bytes": 9, "output_bytes": null,
        "adler32": null, "trailing_bytes": null, "error": "cannot load the zlib library"}]);
    document["partial"] = json!(["inflate: cannot load the zlib library"]);
    let result = run_scripted("partial", &Scripted::same(Step::Write(document, 0)));
    assert_eq!(result.report.status, "partial");
    assert_eq!(result.report.class, None);
    let probe = result.report.probe.unwrap();
    assert_eq!(probe.inflate_errors, 1);
    assert_eq!(probe.not_installed, ["inflate"]);
}

#[test]
fn run_fixture_should_report_the_first_runs_probe() {
    let mut document = done_document();
    document["probe"] = json!({
        "inflate": [{"input_bytes": 9, "output_bytes": 30, "adler32": 5, "trailing_bytes": 0, "error": null}],
        "writes_refused": ["./Data/TimelessJewelData/MilitantFaith.bin"]
    });
    let result = run_scripted("probe", &Scripted::same(Step::Write(document, 0)));
    assert_eq!(result.report.status, "ok");
    let probe = result.report.probe.as_ref().unwrap();
    assert_eq!(
        (
            probe.inflate_calls,
            probe.inflated_bytes,
            probe.writes_refused
        ),
        (1, 30, 1)
    );
    // No probe without a valid document.
    let failed = run_scripted("no-probe", &Scripted::same(Step::Exit(1)));
    assert!(failed.report.probe.is_none());
}

#[test]
fn classify_should_reject_a_failed_inflate_without_partial() {
    let mut document = done_document();
    document["probe"]["inflate"] = json!([{"input_bytes": 9, "output_bytes": null,
        "adler32": null, "trailing_bytes": null, "error": "inflate returned -3"}]);
    assert_eq!(
        class_of(Step::Write(document, 0)),
        Some(FailureClass::Protocol)
    );
}

#[test]
fn resolve_zlib_should_hash_a_path_inside_the_oracle_dir() {
    let temp = Temp::new("zlib-path");
    std::fs::create_dir_all(temp.0.join("pob/runtime")).unwrap();
    std::fs::write(temp.0.join("pob/runtime/zlib1.dll"), b"zlib").unwrap();
    let mut adapter = adapter();
    adapter
        .zlib
        .insert("test-x86_64".to_owned(), "pob/runtime/zlib1.dll".to_owned());
    let (record, argument) = resolve_zlib(&adapter, &temp.0, "test-x86_64").unwrap();
    assert_eq!(record.file.as_deref(), Some("pob/runtime/zlib1.dll"));
    assert_eq!(record.sha256, Some(oracle::sha256_hex(b"zlib")));
    assert!(Path::new(&argument).is_absolute());
    assert!(Path::new(&argument).ends_with("pob/runtime/zlib1.dll"));
    std::fs::remove_file(temp.0.join("pob/runtime/zlib1.dll")).unwrap();
    assert!(resolve_zlib(&adapter, &temp.0, "test-x86_64").is_err());
    assert!(resolve_zlib(&adapter, &temp.0, "other-x86_64").is_err());
}

#[test]
fn resolve_zlib_should_pass_a_library_name_unchanged() {
    let mut adapter = adapter();
    adapter
        .zlib
        .insert("test-x86_64".to_owned(), "libloom-none.so.1".to_owned());
    let (record, argument) = resolve_zlib(&adapter, Path::new("."), "test-x86_64").unwrap();
    assert_eq!(argument, OsString::from("libloom-none.so.1"));
    assert_eq!(record.file, None);
    assert_eq!(record.sha256, None);
}

#[test]
fn run_fixture_should_classify_differing_runs_as_nondeterminism() {
    let mut other = done_document();
    other["main_output"]["Life"] = 61.into();
    let launcher = Scripted::new(vec![
        Step::Write(done_document(), 0),
        Step::Write(other, 0),
        Step::Write(done_document(), 0),
    ]);
    let report = run_scripted("nondeterminism", &launcher).report;
    assert_eq!(report.status, "failed");
    assert_eq!(report.class, Some(FailureClass::Nondeterminism));
    assert_eq!(report.determinism.identical, 2);
    assert!(!report.runs[1].same_as_first);
    assert!(report.runs[1].outcome.is_some());
    assert!(report.runs[2].outcome.is_none());
}

#[test]
fn run_fixture_should_classify_a_failure_then_success_as_nondeterminism() {
    let launcher = Scripted::new(vec![Step::Timeout, Step::Write(done_document(), 0)]);
    let report = run_scripted("flip", &launcher).report;
    assert_eq!(report.class, Some(FailureClass::Nondeterminism));
}

#[test]
fn run_fixture_should_keep_a_repeated_failure_failed() {
    let launcher = Scripted::same(Step::Write(failed_document("import", "not XML"), 1));
    let report = run_scripted("repeated-failure", &launcher).report;
    assert_eq!(report.status, "failed");
    assert_eq!(report.class, Some(FailureClass::Import));
    assert_eq!(report.determinism.identical, 3);
    assert_eq!(report.outcome.stage, Some("import"));
    assert_eq!(report.runs[0].stderr_tail.as_deref(), Some("stderr text"));
}

/// The class of a single run.
fn class_of(step: Step) -> Option<FailureClass> {
    // Tests run in parallel: each call gets its own directory.
    static CALLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = CALLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    run_scripted(&format!("class-{n}"), &Scripted::new(vec![step]))
        .report
        .class
}

#[test]
fn classify_should_map_each_failure_to_its_class() {
    let mut settings = done_document();
    settings["settings"]["found"] = true.into();
    let mut network = done_document();
    network["modules"] = json!(["lua-utf8", "socket.http"]);
    let mut schema = done_document();
    schema["main_output"]["Life"] = Value::Null;
    let cases = [
        ("missing runtime", Step::NoStart, FailureClass::Environment),
        ("timeout", Step::Timeout, FailureClass::Timeout),
        (
            "timeout after a complete document",
            Step::TimeoutAfterWriting,
            FailureClass::Timeout,
        ),
        (
            "unreadable document",
            Step::Unreadable,
            FailureClass::Protocol,
        ),
        ("crash before output", Step::Exit(1), FailureClass::Launch),
        (
            "exit 0 without output",
            Step::Exit(0),
            FailureClass::Protocol,
        ),
        (
            "not JSON",
            Step::Raw("{\"protocol\":2,", 0),
            FailureClass::Protocol,
        ),
        (
            "two documents",
            Step::Raw("{}\n{}\n", 0),
            FailureClass::Protocol,
        ),
        (
            "schema violation",
            Step::Write(schema, 0),
            FailureClass::Protocol,
        ),
        (
            "done with exit 1",
            Step::Write(done_document(), 1),
            FailureClass::Protocol,
        ),
        (
            "failure with exit 0",
            Step::Write(failed_document("import", "x"), 0),
            FailureClass::Protocol,
        ),
        (
            "startup error",
            Step::Write(failed_document("startup", "promptMsg"), 1),
            FailureClass::Launch,
        ),
        (
            "malformed fixture",
            Step::Write(failed_document("import", "x"), 1),
            FailureClass::Import,
        ),
        (
            "calculation error",
            Step::Write(failed_document("calculate", "x"), 1),
            FailureClass::Calculation,
        ),
        (
            "normalization error",
            Step::Write(failed_document("normalize", "x"), 1),
            FailureClass::Normalization,
        ),
        (
            "settings file found",
            Step::Write(settings, 0),
            FailureClass::Environment,
        ),
        (
            "network module",
            Step::Write(network, 0),
            FailureClass::Environment,
        ),
    ];
    for (name, step, class) in cases {
        assert_eq!(class_of(step), Some(class), "{name}");
    }
}

#[test]
fn classify_should_accept_lcurl_stub_free_modules() {
    assert!(!is_network_module("lua-utf8"));
    assert!(!is_network_module("sha1.bit_ops"));
    assert!(is_network_module("lcurl.safe"));
    assert!(is_network_module("socket"));
}

#[test]
fn scrub_should_hide_addresses() {
    let scrub = Scrub::new(&[]);
    assert_eq!(
        scrub.apply("table: 0x01a2b3c4 [C]: at 0x7ff6853b4600, 0x end"),
        "table: 0x? [C]: at 0x?, 0x end"
    );
}

#[test]
fn run_fixture_should_record_streams_and_keep_tails_of_failures() {
    let launcher = Scripted::same(Step::Write(failed_document("import", "x"), 1));
    let report = run_scripted("streams", &launcher).report;
    let run = &report.runs[0];
    assert_eq!(run.stderr.bytes, "stderr text".len());
    assert_eq!(
        run.stderr.sha256.as_deref(),
        Some(oracle::sha256_hex(b"stderr text").as_str())
    );
    assert_eq!(run.stdout.bytes, 0);
    assert_eq!(run.stdout_tail.as_deref(), Some(""));
    let ok = run_scripted(
        "streams-ok",
        &Scripted::same(Step::Write(done_document(), 0)),
    )
    .report;
    assert!(ok.runs[0].stderr_tail.is_none());
    assert!(ok.runs[0].stderr.sha256.is_some());
}

#[test]
fn non_ascii_path_should_be_refused_on_windows_only() {
    let paths = [
        ("adapter", Path::new("C:/loom/oracle.lua")),
        ("fixture", Path::new("C:/builds/\u{65e5} build.xml")),
    ];
    let expected = if cfg!(windows) { Some("fixture") } else { None };
    assert_eq!(non_ascii_path(&paths), expected);
    assert_eq!(non_ascii_path(&paths[..1]), None);
}

#[test]
fn adapter_files_should_hash_every_lua_file_next_to_the_adapter() {
    let temp = Temp::new("adapter-files");
    std::fs::write(temp.0.join("oracle.lua"), "-- a").unwrap();
    std::fs::write(temp.0.join("json.lua"), "-- b").unwrap();
    std::fs::write(temp.0.join("notes.txt"), "x").unwrap();
    let files = adapter_files(&temp.0.join("oracle.lua")).unwrap();
    let names: Vec<&str> = files.keys().map(String::as_str).collect();
    assert_eq!(names, ["json.lua", "oracle.lua"]);
    assert_eq!(files["json.lua"], oracle::sha256_hex(b"-- b"));
    assert!(adapter_files(&temp.0.join("missing.lua")).is_err());
}

#[test]
fn scrub_should_replace_paths_with_either_separator() {
    let scrub = Scrub::new(&[
        (Path::new(r"D:\oracle"), "<oracle>"),
        (Path::new(r"D:\oracle\pob\x.xml"), "<fixture>"),
    ]);
    assert_eq!(
        scrub.apply(r"D:\oracle\pob\src\Main.lua:3: in D:/oracle/pob/x.xml"),
        "<oracle>/pob/src/Main.lua:3: in <fixture>"
    );
}

#[test]
fn outcome_should_not_depend_on_the_oracle_location() {
    let message = |dir: &str| {
        let document = failed_document(
            "calculate",
            &format!("{dir}/pob/src/Modules/Calcs.lua:9: boom"),
        );
        let scrub = Scrub::new(&[(Path::new(dir), "<oracle>")]);
        let started = Ok(finished(Some(1), false));
        classify(
            &started,
            Ok(Some(document.to_string().as_bytes())),
            &adapter(),
            &scrub,
        )
        .0
    };
    let windows = message(r"C:\Users\someone\loom\local\oracle");
    let linux = message("/home/someone/loom-oracle");
    assert_eq!(windows, linux);
    assert_eq!(
        windows.message.as_deref(),
        Some("lua: <oracle>/pob/src/Modules/Calcs.lua:9: boom")
    );
}

#[test]
fn collect_fixtures_should_read_xml_files_of_a_directory_in_order() {
    let temp = Temp::new("collect");
    std::fs::write(temp.0.join("b build.xml"), "<b/>").unwrap();
    std::fs::write(temp.0.join("Ä build.xml"), "<a/>").unwrap();
    std::fs::write(temp.0.join("notes.txt"), "x").unwrap();
    let fixtures = collect_fixtures(std::slice::from_ref(&temp.0), &temp.0, false).unwrap();
    let ids: Vec<&str> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(ids, ["b build.xml", "Ä build.xml"]);
    assert_eq!(fixtures[0].sha256, oracle::sha256_hex(b"<b/>"));
    assert!(fixtures.iter().all(|f| f.expect.is_none()));
    // Anonymous: ids are content hashes, so no file name is reported.
    let fixtures = collect_fixtures(std::slice::from_ref(&temp.0), &temp.0, true).unwrap();
    assert_eq!(fixtures[0].id, oracle::sha256_hex(b"<b/>"));
    assert_eq!(fixtures[1].id, oracle::sha256_hex(b"<a/>"));
}

#[test]
fn collect_fixtures_should_read_a_fixture_manifest() {
    let temp = Temp::new("collect-manifest");
    std::fs::write(temp.0.join("a.xml"), "<a/>").unwrap();
    let manifest = json!({"format": 1, "fixtures": [{
        "id": "a.xml", "tier": "synthetic", "source": "local", "path": "a.xml",
        "sha256": oracle::sha256_hex(b"<a/>"), "expect": "ORACLE_IMPORT",
        "description": "test"
    }]});
    let path = temp.0.join("fixtures.json");
    std::fs::write(&path, manifest.to_string()).unwrap();
    let fixtures = collect_fixtures(std::slice::from_ref(&path), &temp.0, false).unwrap();
    assert_eq!(fixtures[0].id, "a.xml");
    assert_eq!(
        fixtures[0].expect,
        Some(Expect::Fails(FailureClass::Import))
    );
    std::fs::write(temp.0.join("a.xml"), "<b/>").unwrap();
    let err = collect_fixtures(&[path], &temp.0, false).unwrap_err();
    assert!(err.contains("differs from the manifest"), "{err}");
}

#[test]
fn finish_should_not_count_fixtures_that_met_their_expectation() {
    let temp = Temp::new("finish-expect");
    let request = request(&temp, temp.0.join("no-oracle"));
    let mut report = run(&request).unwrap();
    report.oracle.class = None;
    let scripted = |name: &str, step: Step, expect: Option<Expect>| {
        let temp = Temp::new(name);
        let scrub = Scrub::new(&[]);
        let fixture = Fixture {
            expect,
            ..fixture()
        };
        let launcher = Scripted::same(step);
        run_fixture(
            &fixture,
            2,
            &temp.0.join("runs"),
            &adapter(),
            &launcher,
            &scrub,
        )
        .unwrap()
        .report
    };
    let import = Some(Expect::Fails(FailureClass::Import));
    let failed_import = failed_document("import", "pob: bad xml");
    // An expected failure is met: the report is ok, exit 0.
    report.fixtures = vec![
        scripted(
            "expect-ok",
            Step::Write(done_document(), 0),
            Some(Expect::Ok),
        ),
        scripted(
            "expect-import",
            Step::Write(failed_import.clone(), 1),
            import,
        ),
    ];
    assert_eq!(report.fixtures[1].met, Some(true));
    let report = report.finish();
    assert_eq!(
        (report.status, report.class, report.exit_code()),
        ("ok", None, 0)
    );
    // An expected failure that ran ok is unmet: exit 4.
    let mut report = report;
    report.fixtures = vec![scripted("unmet", Step::Write(done_document(), 0), import)];
    let report = report.finish();
    assert_eq!((report.status, report.class), ("unmet", None));
    assert_eq!(report.exit_code(), EXIT_UNMET);
    assert!(
        report
            .summary_markdown()
            .contains("ORACLE_IMPORT (**not met**)")
    );
    // A fixture expected ok that fails fails the report with its class.
    let mut report = report;
    report.fixtures = vec![scripted(
        "fails",
        Step::Write(failed_import, 1),
        Some(Expect::Ok),
    )];
    let report = report.finish();
    assert_eq!(report.class, Some(FailureClass::Import));
    assert_eq!(report.exit_code(), 15);
}

#[test]
fn collect_fixtures_should_reject_duplicate_names_and_empty_input() {
    let temp = Temp::new("duplicates");
    std::fs::create_dir_all(temp.0.join("a")).unwrap();
    std::fs::create_dir_all(temp.0.join("b")).unwrap();
    std::fs::write(temp.0.join("a/x.xml"), "<a/>").unwrap();
    std::fs::write(temp.0.join("b/x.xml"), "<b/>").unwrap();
    let err = collect_fixtures(&[temp.0.join("a"), temp.0.join("b")], &temp.0, false).unwrap_err();
    assert!(err.contains("two fixtures are named"), "{err}");
    std::fs::write(temp.0.join("b/x.xml"), "<a/>").unwrap();
    let err = collect_fixtures(&[temp.0.join("a"), temp.0.join("b")], &temp.0, true).unwrap_err();
    assert!(err.contains("same bytes"), "{err}");
    std::fs::create_dir_all(temp.0.join("empty")).unwrap();
    assert!(collect_fixtures(&[temp.0.join("empty")], &temp.0, false).is_err());
}

#[test]
fn tail_should_keep_the_last_characters() {
    assert_eq!(tail("abcdef", 3), "def");
    assert_eq!(tail("ab", 3), "ab");
    assert_eq!(tail("ääää", 2), "ää");
}

fn request(temp: &Temp, oracle_dir: PathBuf) -> Request {
    let manifest = temp.0.join("manifest.json");
    std::fs::write(&manifest, COMMITTED).unwrap();
    let fixture = temp.0.join("build.xml");
    std::fs::write(&fixture, "<PathOfBuilding/>").unwrap();
    let adapter = temp.0.join("oracle.lua");
    std::fs::write(&adapter, "-- adapter").unwrap();
    Request {
        manifest,
        oracle_dir,
        adapter,
        fixtures: vec![fixture],
        out: temp.0.join("out"),
        options: Options {
            runs: 3,
            timeout: Duration::from_secs(5),
        },
        tool: Tool {
            commit: None,
            dirty: None,
        },
        anonymous: false,
    }
}

#[test]
fn run_should_not_launch_on_an_unverified_oracle() {
    let temp = Temp::new("unverified");
    let request = request(&temp, temp.0.join("no-oracle"));
    let report = run(&request).unwrap();
    assert_eq!(report.status, "failed");
    assert_eq!(report.class, Some(FailureClass::Acquisition));
    assert!(report.fixtures.is_empty());
    assert!(report.adapter.is_none());
    let written: Value =
        serde_json::from_slice(&std::fs::read(temp.0.join("out/report.json")).unwrap()).unwrap();
    assert_eq!(written["class"], "ORACLE_ACQUISITION");
    assert_eq!(written["oracle"]["status"], "failed");
    assert!(temp.0.join("out/summary.md").exists());
}

#[test]
fn run_should_refuse_an_existing_output_directory_and_zero_runs() {
    let temp = Temp::new("refuse");
    let mut request = request(&temp, temp.0.join("no-oracle"));
    std::fs::create_dir_all(&request.out).unwrap();
    assert!(run(&request).unwrap_err().contains("already exists"));
    request.out = temp.0.join("out2");
    request.options.runs = 0;
    assert!(run(&request).is_err());
}

#[test]
fn finish_should_fail_on_a_dirty_checkout_after_the_runs() {
    let temp = Temp::new("finish");
    let request = request(&temp, temp.0.join("no-oracle"));
    let mut report = run(&request).unwrap();
    report.oracle.class = None;
    report.after.push(Check {
        name: "pob.clean".to_owned(),
        class: FailureClass::Acquisition,
        ok: false,
        expected: "clean".to_owned(),
        actual: "1 entries: !! src/Data/x.bin".to_owned(),
    });
    let report = report.finish();
    assert_eq!(report.class, Some(FailureClass::Acquisition));
    assert!(report.summary_markdown().contains("!! src/Data/x.bin"));
}

#[test]
fn finish_should_report_partial_unless_something_failed() {
    let temp = Temp::new("finish-partial");
    let request = request(&temp, temp.0.join("no-oracle"));
    let mut report = run(&request).unwrap();
    report.oracle.class = None;
    let mut document = done_document();
    document["probe"]["inflate"] = json!([{"input_bytes": 9, "output_bytes": null,
        "adler32": null, "trailing_bytes": null, "error": "no zlib"}]);
    document["partial"] = json!(["inflate: no zlib"]);
    let partial = run_scripted(
        "finish-partial-fixture",
        &Scripted::same(Step::Write(document, 0)),
    );
    let ok = run_scripted(
        "finish-ok-fixture",
        &Scripted::same(Step::Write(done_document(), 0)),
    );
    report.fixtures = vec![ok.report, partial.report];
    let report = report.finish();
    assert_eq!((report.status, report.class), ("partial", None));
    let failed = run_scripted("finish-failed-fixture", &Scripted::same(Step::Exit(1)));
    let mut report = report;
    report.fixtures.push(failed.report);
    let report = report.finish();
    assert_eq!(report.status, "failed");
    assert_eq!(report.class, Some(FailureClass::Launch));
}

#[test]
fn summary_should_list_fixtures_skipped_after_a_dirty_checkout() {
    let temp = Temp::new("skipped");
    let request = request(&temp, temp.0.join("no-oracle"));
    let mut report = run(&request).unwrap();
    report.skipped = vec!["b.xml".to_owned(), "c.xml".to_owned()];
    assert!(
        report
            .summary_markdown()
            .contains("Not run (the checkout changed): b.xml, c.xml")
    );
}

#[test]
fn adapter_arguments_should_name_breakdowns_then_opaque_keys() {
    let arguments = adapter_arguments(&adapter());
    assert_eq!(
        arguments.first().map(String::as_str),
        Some("breakdown=Armour")
    );
    assert_eq!(
        arguments.last().map(String::as_str),
        Some("opaque=sourceGem")
    );
}

/// The oracle directory for the tests that need the real runtime.
fn oracle_dir() -> PathBuf {
    PathBuf::from(std::env::var("LOOM_ORACLE_DIR").unwrap())
}

fn repo(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(path)
}

/// The fake Lua adapters, and the real adapter on the synthetic fixtures,
/// through the real runtime. Needs an acquired oracle
/// (`tools/oracle/acquire.ps1` or `acquire.sh`); set `LOOM_ORACLE_DIR` to
/// it (for example `local/oracle`) and run with `--ignored`.
#[test]
#[ignore = "needs an acquired oracle in LOOM_ORACLE_DIR"]
fn adapters_should_get_their_class_on_the_real_runtime() {
    let fakes = repo("oracle/fixtures/fake-adapters");
    let minimal = repo("oracle/fixtures/minimal-level1.xml");
    let malformed = repo("oracle/fixtures/malformed.xml");
    let timeless = repo("oracle/fixtures/timeless-militant-faith.xml");
    let real = repo("oracle/adapter/oracle.lua");
    let cases = [
        (
            fakes.join("timeout.lua"),
            &minimal,
            Some(FailureClass::Timeout),
        ),
        (
            fakes.join("crash.lua"),
            &minimal,
            Some(FailureClass::Launch),
        ),
        (
            fakes.join("no-output.lua"),
            &minimal,
            Some(FailureClass::Protocol),
        ),
        (
            fakes.join("wrong-protocol.lua"),
            &minimal,
            Some(FailureClass::Protocol),
        ),
        (
            fakes.join("startup-error.lua"),
            &minimal,
            Some(FailureClass::Launch),
        ),
        (
            fakes.join("flaky.lua"),
            &minimal,
            Some(FailureClass::Nondeterminism),
        ),
        (fakes.join("environment.lua"), &minimal, None),
        (real.clone(), &minimal, None),
        (real.clone(), &malformed, Some(FailureClass::Import)),
        (real.clone(), &timeless, None),
    ];
    for (adapter, fixture, class) in cases {
        let name = format!(
            "{}-{}",
            adapter.file_name().unwrap().to_string_lossy(),
            fixture.file_name().unwrap().to_string_lossy()
        );
        let temp = Temp::new(&name);
        // The fakes need little time; the real adapter starts PoB.
        let timeout = if adapter == real { 120 } else { 3 };
        let request = Request {
            manifest: repo("oracle/manifest.json"),
            oracle_dir: oracle_dir(),
            adapter,
            fixtures: vec![fixture.clone()],
            out: temp.0.join("out"),
            options: Options {
                runs: 2,
                timeout: Duration::from_secs(timeout),
            },
            tool: Tool {
                commit: None,
                dirty: None,
            },
            anonymous: false,
        };
        let report = run(&request).unwrap();
        assert_eq!(report.oracle.class, None, "{name}: oracle not verified");
        assert_eq!(
            report.fixtures[0].class, class,
            "{name}: {:?}",
            report.fixtures[0]
        );
        let identical = if class == Some(FailureClass::Nondeterminism) {
            1
        } else {
            2
        };
        assert_eq!(
            report.fixtures[0].determinism.identical, identical,
            "{name}"
        );
        assert!(
            report.after.iter().all(|c| c.ok),
            "{name}: checkout changed"
        );
        if fixture == &timeless {
            // The jewel data was inflated, and its .bin cache refused.
            let probe = report.fixtures[0].probe.as_ref().unwrap();
            assert_eq!(probe.inflate_calls, 1, "{name}: {probe:?}");
            assert_eq!(probe.inflate_errors, 0, "{name}: {probe:?}");
            assert!(probe.inflated_bytes > 1_000_000, "{name}: {probe:?}");
            assert_eq!(probe.writes_refused, 1, "{name}: {probe:?}");
            assert_eq!(report.fixtures[0].status, "ok");
        }
    }
}

/// Spec case 8 on the real runtime: with a zlib entry that is not a
/// library (PoB's own LICENSE.md, a file in the verified checkout), the
/// inflate shim is not installed, and a build that needs jewel data is
/// `partial`; a build that does not is still `ok`. Same requirements as
/// above.
#[test]
#[ignore = "needs an acquired oracle in LOOM_ORACLE_DIR"]
fn real_adapter_should_be_partial_when_zlib_does_not_load() {
    let temp = Temp::new("no-zlib");
    let mut manifest: Value =
        serde_json::from_slice(&std::fs::read(repo("oracle/manifest.json")).unwrap()).unwrap();
    manifest["adapter"]["zlib"][oracle::platform()] = "pob/LICENSE.md".into();
    let manifest_path = temp.0.join("manifest.json");
    std::fs::write(&manifest_path, manifest.to_string()).unwrap();
    let request = Request {
        manifest: manifest_path,
        oracle_dir: oracle_dir(),
        adapter: repo("oracle/adapter/oracle.lua"),
        fixtures: vec![
            repo("oracle/fixtures/minimal-level1.xml"),
            repo("oracle/fixtures/timeless-militant-faith.xml"),
        ],
        out: temp.0.join("out"),
        options: Options {
            runs: 2,
            timeout: Duration::from_secs(120),
        },
        tool: Tool {
            commit: None,
            dirty: None,
        },
        anonymous: false,
    };
    let report = run(&request).unwrap();
    assert_eq!(report.oracle.class, None, "oracle not verified");
    let [minimal, timeless] = &report.fixtures[..] else {
        panic!("{:?}", report.fixtures);
    };
    assert_eq!(minimal.status, "ok", "{minimal:?}");
    assert_eq!(timeless.status, "partial", "{timeless:?}");
    assert_eq!(timeless.class, None);
    assert_eq!(timeless.determinism.identical, 2);
    let probe = timeless.probe.as_ref().unwrap();
    assert_eq!(probe.not_installed, ["inflate"]);
    assert_eq!((probe.inflate_calls, probe.inflate_errors), (1, 1));
    assert_eq!(report.status, "partial");
    assert_eq!(report.class, None);
    assert!(report.after.iter().all(|c| c.ok), "checkout changed");
}

/// The shims' Lua tests (`tools/oracle/fixtures/lua-tests/shims.lua`) on
/// the real runtime with the manifest's zlib. Same requirements as above.
#[test]
#[ignore = "needs an acquired oracle in LOOM_ORACLE_DIR"]
fn shims_should_pass_their_lua_tests() {
    let platform = oracle::platform();
    let oracle_dir = std::path::absolute(oracle_dir()).unwrap();
    let luajit = oracle_dir.join(oracle::runtime_files(&platform).unwrap()[0]);
    let (_, zlib) = resolve_zlib(&adapter(), &oracle_dir, &platform).unwrap();
    let args = [
        repo("oracle/fixtures/lua-tests/shims.lua").into_os_string(),
        repo("oracle/adapter/shims.lua").into_os_string(),
        zlib,
    ];
    let finished = oracle_process::run(
        &luajit,
        &args,
        &std::env::temp_dir(),
        &[],
        Duration::from_secs(60),
    )
    .unwrap();
    assert_eq!(
        finished.code,
        Some(0),
        "{}{}",
        finished.stdout,
        finished.stderr
    );
    assert!(finished.stdout.starts_with("ok "), "{}", finished.stdout);
}

/// The JSON writer's Lua tests (`tools/oracle/fixtures/lua-tests/`) on the
/// real runtime. Same requirements as above.
#[test]
#[ignore = "needs an acquired oracle in LOOM_ORACLE_DIR"]
fn json_writer_should_pass_its_lua_tests() {
    let platform = oracle::platform();
    let luajit = oracle_dir().join(oracle::runtime_files(&platform).unwrap()[0]);
    let args = [
        repo("oracle/fixtures/lua-tests/json_writer.lua").into_os_string(),
        repo("oracle/adapter/json.lua").into_os_string(),
    ];
    let finished = oracle_process::run(
        &std::path::absolute(luajit).unwrap(),
        &args,
        &std::env::temp_dir(),
        &[],
        Duration::from_secs(60),
    )
    .unwrap();
    assert_eq!(
        finished.code,
        Some(0),
        "{}{}",
        finished.stdout,
        finished.stderr
    );
    assert!(finished.stdout.starts_with("ok "), "{}", finished.stdout);
}

/// The intentional discrepancy on the real runtime: `oracle-compare` of
/// the pair's real outputs reports exactly the two movement-speed fields,
/// unexpected without a register and expected with the test register.
#[test]
#[ignore = "needs an acquired oracle in LOOM_ORACLE_DIR"]
fn discrepancy_pair_should_differ_in_exactly_the_registered_fields() {
    use crate::oracle_compare;
    let temp = Temp::new("discrepancy-pair");
    let request = Request {
        manifest: repo("oracle/manifest.json"),
        oracle_dir: oracle_dir(),
        adapter: repo("oracle/adapter/oracle.lua"),
        fixtures: vec![
            repo("oracle/fixtures/minimal-level1.xml"),
            repo("oracle/fixtures/discrepancy-movement-speed.xml"),
        ],
        out: temp.0.join("out"),
        options: Options {
            runs: 2,
            timeout: Duration::from_secs(120),
        },
        tool: Tool {
            commit: None,
            dirty: None,
        },
        anonymous: false,
    };
    let report = run(&request).unwrap();
    assert_eq!((report.status, report.class), ("ok", None), "{report:?}");
    let outputs = temp.0.join("out/outputs");
    let compare = |register: Option<PathBuf>, out: &str| {
        oracle_compare::compare(&oracle_compare::Request {
            left: outputs.join("minimal-level1.xml.json"),
            right: outputs.join("discrepancy-movement-speed.xml.json"),
            register,
            fixture: None,
            out: temp.0.join(out),
        })
        .unwrap()
    };
    let unregistered = compare(None, "compare-1");
    assert_eq!(unregistered.class, Some(FailureClass::UnexpectedDifference));
    let fields: Vec<&str> = unregistered.fixtures[0]
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
    let registered = compare(
        Some(repo("oracle/fixtures/discrepancy-register.json")),
        "compare-2",
    );
    assert_eq!(registered.class, Some(FailureClass::ExpectedDifference));
    assert!(registered.unused_entries.is_empty());
}
