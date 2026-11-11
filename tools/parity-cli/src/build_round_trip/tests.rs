use std::path::{Path, PathBuf};

use loom_domain::Completeness;

use super::mutate::{Kind, mutations};
use super::*;

const POE1_XML: &str = include_str!("../../testdata/build-round-trip/synthetic-poe1.xml");

fn testdata(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("testdata/build-round-trip")
        .join(name)
}

fn fixture_bundle(name: &str) -> loom_data::bundle::Bundle {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/loom-data/tests/fixtures")
        .join(name);
    crate::load_bundle(&dir).unwrap()
}

fn input(file: &str, format: Format, bytes: &[u8]) -> Input {
    Input {
        path: format!("t/{file}"),
        file: file.to_owned(),
        format,
        bytes: bytes.to_vec(),
    }
}

fn options<'a, 'r, 'd, 'b>(
    data: Option<Data<'r, 'd, 'b>>,
    expect: &'a BTreeSet<String>,
) -> Options<'a, 'r, 'd, 'b> {
    Options {
        data,
        anonymous: false,
        expect_import_error: expect,
        tool: Tool::default(),
    }
}

/// Runs inputs without a bundle.
fn plain(inputs: &[Input], expect: &[&str]) -> (Summary, Report) {
    let expect: BTreeSet<String> = expect.iter().map(|s| (*s).to_owned()).collect();
    run(inputs, &options(None, &expect))
}

/// The synthetic PoE2 document: the synthetic PoE1 build's items in a
/// PoE2 build, unresolved (PoB's format is PoE1's, so a PoE2 build comes
/// as a document).
fn synthetic_poe2() -> String {
    let poe1 = import(POE1_XML).unwrap();
    let mut b = Build::new(Game::Poe2);
    b.items = poe1.items;
    let mut json = to_json(&b).unwrap();
    json.push('\n');
    json
}

const PASS: Checks = Checks {
    a_xml_equal: Status::Pass,
    b_builds_equal: Status::Pass,
    c_json_stable: Status::Pass,
    d_source_lines: Status::Pass,
};

fn checks_eq(a: Checks, b: Checks) -> bool {
    a.all() == b.all()
}

/// Gate 3b (moved from `loom-parser`'s `resolve_build` tests, maintainer
/// decision 2026-11-10): the synthetic PoE1 build (PoB XML) and PoE2 build
/// (a document) resolve with their synthetic bundles, and round-trip
/// through the JSON document with the same bytes on a second run; the
/// PoE1 one also through PoB XML.
#[test]
fn synthetic_builds_should_round_trip_resolved() {
    let poe2_json = std::fs::read(testdata("synthetic-poe2.json")).unwrap();
    for (bundle, input, a) in [
        (
            "bundle-poe1",
            input("synthetic-poe1.xml", Format::PobXml, POE1_XML.as_bytes()),
            Status::Pass,
        ),
        (
            "bundle-poe2",
            input("synthetic-poe2.json", Format::BuildDocument, &poe2_json),
            Status::NotApplicable,
        ),
    ] {
        let bundle = fixture_bundle(bundle);
        let data = BundleData::new(&bundle);
        let resolver = Resolver::new(&data).unwrap();
        let expect = BTreeSet::new();
        let opts = options(
            Some(Data {
                resolver: &resolver,
                data: &data,
            }),
            &expect,
        );
        let (summary, report) = run(std::slice::from_ref(&input), &opts);
        assert!(report.builds.is_empty(), "{:#?}", report.builds);
        let row = &summary.per_build[0];
        assert!(row.ok, "{}", input.file);
        assert!(
            checks_eq(
                row.checks,
                Checks {
                    a_xml_equal: a,
                    ..PASS
                }
            ),
            "{:?}",
            row.checks
        );
        assert_eq!(row.completeness, Some(Completeness::Partial.name()));
        assert!(row.modifier_lines.get("resolved").copied().unwrap_or(0) > 0);
        assert_eq!(row.modifier_lines.get("pending"), None);
        assert_eq!(
            summary.inputs.bundle.as_ref().map(|b| b.game.as_str()),
            Some(bundle.game.name())
        );
    }
}

/// The PoE1 build has every outcome of the default scope through the XML
/// adapter (`resolve_build.rs` checks each outcome's content).
#[test]
fn synthetic_poe1_should_have_every_outcome() {
    let bundle = fixture_bundle("bundle-poe1");
    let data = BundleData::new(&bundle);
    let resolver = Resolver::new(&data).unwrap();
    let expect = BTreeSet::new();
    let opts = options(
        Some(Data {
            resolver: &resolver,
            data: &data,
        }),
        &expect,
    );
    let (summary, _) = run(
        &[input("s.xml", Format::PobXml, POE1_XML.as_bytes())],
        &opts,
    );
    let lines: Vec<(&str, usize)> = summary
        .modifier_lines
        .iter()
        .map(|(k, n)| (*k, *n))
        .collect();
    assert_eq!(
        lines,
        [
            ("continues", 2),
            ("excluded", 1),
            ("resolved", 9),
            ("resolved-adapted", 2),
            ("unresolved", 6)
        ]
    );
}

#[test]
fn synthetic_poe2_document_should_be_fresh() {
    let committed = std::fs::read_to_string(testdata("synthetic-poe2.json")).unwrap();
    assert_eq!(
        committed.replace("\r\n", "\n"),
        synthetic_poe2(),
        "run the ignored write_synthetic_poe2 test"
    );
}

#[test]
#[ignore = "writes testdata/build-round-trip/synthetic-poe2.json"]
fn write_synthetic_poe2() {
    std::fs::write(testdata("synthetic-poe2.json"), synthetic_poe2()).unwrap();
}

#[test]
fn unresolved_build_should_pass_without_a_bundle() {
    let (summary, report) = plain(&[input("s.xml", Format::PobXml, POE1_XML.as_bytes())], &[]);
    assert!(report.builds.is_empty(), "{:#?}", report.builds);
    let row = &summary.per_build[0];
    assert!(checks_eq(row.checks, PASS));
    assert_eq!(row.completeness, None);
    assert_eq!(row.items, 4);
    // Every mutation kind ran and survived.
    for kind in Kind::ALL {
        let m = row.mutations.get(kind.name()).copied().unwrap_or_default();
        assert!(m.tried > 0 && m.tried == m.survived, "{kind:?}: {m:?}");
    }
}

#[test]
fn expected_import_error_should_pass_and_an_import_should_not() {
    let broken = input("broken.xml", Format::PobXml, b"<PathOfBuilding><Build>");
    let good = input("good.xml", Format::PobXml, POE1_XML.as_bytes());
    let (summary, report) = plain(&[broken, good], &["broken.xml", "good.xml"]);
    assert_eq!((summary.passed, summary.failed), (1, 1));
    assert_eq!(summary.per_build[0].read_error, Some("IMPORT_XML"));
    assert_eq!(
        report.builds[0].failures,
        ["expected an import error; the file imported"]
    );
}

#[test]
fn unexpected_import_error_should_fail() {
    let other_root = input("x.xml", Format::PobXml, b"<Other/>");
    let not_utf8 = input("y.xml", Format::PobXml, &[0xff, 0xfe]);
    let (summary, report) = plain(&[other_root, not_utf8], &[]);
    assert_eq!(summary.failed, 2);
    assert_eq!(summary.per_build[0].read_error, Some("IMPORT_ROOT"));
    assert_eq!(summary.per_build[1].read_error, Some("NOT_UTF8"));
    assert_eq!(report.builds[0].failures, ["read: IMPORT_ROOT"]);
}

#[test]
fn identical_files_should_count_once_and_anonymous_names_should_be_hashes() {
    let a = input("a.xml", Format::PobXml, POE1_XML.as_bytes());
    let b = input("b.xml", Format::PobXml, POE1_XML.as_bytes());
    let expect = BTreeSet::new();
    let opts = Options {
        anonymous: true,
        ..options(None, &expect)
    };
    let (summary, _) = run(&[a, b], &opts);
    assert_eq!((summary.builds, summary.duplicates), (1, 1));
    let hash = hex(&Sha256::digest(POE1_XML.as_bytes()));
    assert_eq!(summary.per_build[0].name, format!("sha256:{hash}"));
    let json = serde_json::to_string(&summary).unwrap();
    assert!(!json.contains("Loom Test") && !json.contains("a.xml"));
}

/// A document that cannot be written (nesting past the DTO's limit) is a
/// failure, not a pass.
#[test]
fn unwritable_build_should_fail() {
    let deep = format!(
        "<PathOfBuilding>{}{}</PathOfBuilding>",
        "<Deep>".repeat(200),
        "</Deep>".repeat(200)
    );
    let (summary, report) = plain(&[input("d.xml", Format::PobXml, deep.as_bytes())], &[]);
    assert_eq!(summary.failed, 1);
    assert!(
        report.builds[0]
            .failures
            .iter()
            .any(|f| f == "write A: DTO_TOO_DEEP"),
        "{:?}",
        report.builds[0].failures
    );
    assert_eq!(summary.per_build[0].checks.b_builds_equal, Status::NotRun);
}

/// A resolved document read without its bundle: C (imported from PoB XML)
/// has pending lines, so (b) fails. Resolutions are compared.
#[test]
fn resolutions_should_be_compared() {
    let bundle = fixture_bundle("bundle-poe1");
    let data = BundleData::new(&bundle);
    let resolver = Resolver::new(&data).unwrap();
    let mut b = import(POE1_XML).unwrap();
    resolve(
        &mut b,
        Some(&Data {
            resolver: &resolver,
            data: &data,
        }),
    )
    .unwrap();
    let json = to_json(&b).unwrap();
    let (summary, report) = plain(
        &[input("r.json", Format::BuildDocument, json.as_bytes())],
        &[],
    );
    let row = &summary.per_build[0];
    assert_eq!(row.checks.b_builds_equal, Status::Fail);
    assert_eq!(row.checks.a_xml_equal, Status::NotApplicable);
    assert_eq!(row.checks.d_source_lines, Status::Pass);
    assert_eq!(report.builds[0].failures, ["(b) C differs from A"]);
}

#[test]
fn changed_source_lines_should_fail_check_d() {
    let input_lines = xml_item_lines(POE1_XML);
    assert_eq!(input_lines.len(), 4);
    let mut b = import(POE1_XML).unwrap();
    let mut failures = Vec::new();
    assert!(same_lines(&input_lines, &b, "A", &mut failures));
    for entry in b.items.entries.typed_mut() {
        if let loom_domain::ItemsEntry::Item(item) = entry
            && item.id.0 == 2
        {
            item.body.lines[4].source = "changed".to_owned();
        }
        if let loom_domain::ItemsEntry::Item(item) = entry
            && item.id.0 == 3
        {
            item.body.lines.pop();
        }
    }
    assert!(!same_lines(&input_lines, &b, "A", &mut failures));
    assert_eq!(
        failures,
        [
            "(d) A item 2: line 4 differs",
            "(d) A item 3: 5 source lines in, 4 out"
        ]
    );
}

#[test]
fn mutation_writer_should_reproduce_the_document() {
    let set = mutations(POE1_XML).unwrap();
    // Every mutated copy differs from the source: the mutation is real.
    for m in &set {
        assert!(
            !xml_equal(&m.xml, POE1_XML).unwrap(),
            "{:?} at {}",
            m.kind,
            m.path
        );
        assert!(m.xml.contains("oom"), "{:?}", m.kind);
    }
    assert!(matches!(mutations("<a>"), Err(mutate::Error::Xml(_))));
}

#[test]
fn mutation_targets_should_be_each_path_once() {
    let xml = r#"<PathOfBuilding a="1"><Items><Item id="1">x</Item><Item id="2">y</Item></Items><Notes>n</Notes></PathOfBuilding>"#;
    let set = mutations(xml).unwrap();
    let got: Vec<(String, &str)> = set
        .iter()
        .map(|m| (m.path.clone(), m.kind.name()))
        .collect();
    let mut want = Vec::new();
    for (path, kinds) in [
        (
            "PathOfBuilding",
            &[
                Kind::Attribute,
                Kind::FirstChild,
                Kind::LastChild,
                Kind::CommentBefore,
                Kind::InstructionAfter,
            ][..],
        ),
        (
            "PathOfBuilding/Items",
            &[
                Kind::Attribute,
                Kind::FirstChild,
                Kind::LastChild,
                Kind::CommentBefore,
                Kind::InstructionAfter,
                Kind::ElementAfter,
            ][..],
        ),
        ("PathOfBuilding/Items/Item", &Kind::ALL[..]),
        (
            "PathOfBuilding/Notes",
            &[
                Kind::Attribute,
                Kind::FirstChild,
                Kind::LastChild,
                Kind::CommentBefore,
                Kind::InstructionAfter,
                Kind::ElementAfter,
            ][..],
        ),
    ] {
        for k in kinds {
            want.push((path.to_owned(), k.name()));
        }
    }
    assert_eq!(got, want);
    // The earlier item text is a second text child, before the body.
    let extra = set.iter().find(|m| m.kind == Kind::ExtraItemText).unwrap();
    assert!(
        extra
            .xml
            .contains("<Item id=\"1\">\nRarity: NORMAL\nLoom Mutation Earlier Text\n<LoomMutation")
    );
    let doc = roxmltree::Document::parse(&extra.xml).unwrap();
    let texts = doc
        .descendants()
        .find(|n| n.has_tag_name("Item"))
        .unwrap()
        .children()
        .filter(roxmltree::Node::is_text)
        .count();
    assert_eq!(texts, 2);
}

#[test]
fn mutation_values_should_be_escaped() {
    let set = mutations("<PathOfBuilding/>").unwrap();
    let attribute = set.iter().find(|m| m.kind == Kind::Attribute).unwrap();
    let doc = roxmltree::Document::parse(&attribute.xml).unwrap();
    assert_eq!(
        doc.root_element().attribute(mutate::ATTRIBUTE),
        Some("a < b & \"c\" 'd'\tline\nend")
    );
}

#[test]
fn arguments_should_parse() {
    let args = |s: &str| -> Vec<String> { s.split(' ').map(str::to_owned).collect() };
    let got = crate::round_trip_args(&args(
        "a b.xml --bundle d --out o --anonymous --expect-import-error m.xml",
    ))
    .unwrap();
    assert_eq!(got.inputs, ["a", "b.xml"]);
    assert_eq!(
        (got.bundle.as_deref(), got.out.as_deref(), got.anonymous),
        (Some("d"), Some("o"), true)
    );
    assert!(got.expect_import_error.contains("m.xml"));
    for bad in [
        "a",
        "--out o",
        "a --out",
        "a --out o --out p",
        "a --out o --x",
    ] {
        assert_eq!(crate::round_trip_args(&args(bad)), None, "{bad}");
    }
}
