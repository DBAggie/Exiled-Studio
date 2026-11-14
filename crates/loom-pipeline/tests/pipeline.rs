//! The pipeline over the synthetic PoE1 bundle and build (made-up rows;
//! `crates/loom-data/tests/fixtures/make_bundles.py`,
//! `tools/parity-cli/testdata/build-round-trip/synthetic-poe1.xml`).

#![expect(
    clippy::disallowed_methods,
    reason = "a test reads the fixtures into memory; never part of a core build"
)]
#![expect(
    clippy::unwrap_used,
    reason = "a test over fixed fixtures; a failure should panic"
)]

use std::fs;
use std::path::{Path, PathBuf};

use loom_data::bundle::{Bundle, MemorySource, validate};
use loom_parser::import::{BundleData, Resolver};
use loom_pipeline::{PobFormat, Reresolved, Step};

fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files(&path, out);
        } else {
            out.push(path);
        }
    }
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn bundle() -> Bundle {
    let dir = root().join("crates/loom-data/tests/fixtures/bundle-poe1");
    let mut paths = Vec::new();
    files(&dir, &mut paths);
    let mut src = MemorySource::new();
    for p in paths {
        let rel = p
            .strip_prefix(&dir)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        src.insert(rel, fs::read(&p).unwrap());
    }
    assert!(validate::validate(&src).is_valid());
    Bundle::load(&src).unwrap()
}

fn synthetic() -> String {
    fs::read_to_string(root().join("tools/parity-cli/testdata/build-round-trip/synthetic-poe1.xml"))
        .unwrap()
}

#[test]
fn pob_format_should_tell_xml_from_share_codes() {
    assert_eq!(loom_pipeline::pob_format("  <?xml"), PobFormat::Xml);
    assert_eq!(loom_pipeline::pob_format("\u{feff}<Path"), PobFormat::Xml);
    assert_eq!(loom_pipeline::pob_format("eNrtXW1z"), PobFormat::ShareCode);
}

#[test]
fn read_pob_should_read_a_share_code_as_its_xml() {
    let xml = synthetic();
    let code = loom_serialization::legacy::share_code::encode(&xml);
    assert_eq!(
        loom_pipeline::read_pob(&code).unwrap(),
        loom_pipeline::read_pob(&xml).unwrap()
    );
}

#[test]
fn read_pob_should_give_the_adapter_code_on_failure() {
    let e = loom_pipeline::read_pob("<not a build").unwrap_err();
    assert_eq!(e.step, Step::Read);
    assert!(e.code.starts_with("IMPORT_"), "{}", e.code);
    let e = loom_pipeline::read_pob("!!! not base64").unwrap_err();
    assert_eq!(e.step, Step::Read);
}

#[test]
fn pipeline_should_calculate_the_synthetic_build_deterministically() {
    let bundle = bundle();
    let data = BundleData::new(&bundle);
    let resolver = Resolver::new(&data).unwrap();
    let run = || {
        let mut build = loom_pipeline::read_pob(&synthetic()).unwrap();
        let ends = loom_pipeline::resolve(&resolver, &data, &mut build).unwrap();
        let options = loom_pipeline::options(&build, &data, &ends, None);
        let result = loom_pipeline::calculate(&build, &bundle, &options, &|| false).unwrap();
        loom_pipeline::document(&result).unwrap()
    };
    let first = run();
    assert_eq!(first, run());
    // calc-run's document for the same input, written before the pipeline
    // moved here (Phase 4 plan, decision 0): the move changed nothing.
    let golden = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/testdata/synthetic-poe1.calc.json"),
    )
    .unwrap()
    .replace("\r\n", "\n");
    assert_eq!(first, golden);
}

#[test]
fn calculate_should_stop_when_cancelled() {
    let bundle = bundle();
    let data = BundleData::new(&bundle);
    let resolver = Resolver::new(&data).unwrap();
    let mut build = loom_pipeline::read_pob(&synthetic()).unwrap();
    let ends = loom_pipeline::resolve(&resolver, &data, &mut build).unwrap();
    let options = loom_pipeline::options(&build, &data, &ends, None);
    let e = loom_pipeline::calculate(&build, &bundle, &options, &|| true).unwrap_err();
    assert_eq!(e.step, Step::Calculate);
    assert_eq!(e.code, "CALC_CANCELLED");
}

#[test]
fn reresolve_should_recover_the_imports_ends_from_a_document() {
    let bundle = bundle();
    let data = BundleData::new(&bundle);
    let resolver = Resolver::new(&data).unwrap();
    let mut build = loom_pipeline::read_pob(&synthetic()).unwrap();
    let ends = loom_pipeline::resolve(&resolver, &data, &mut build).unwrap();
    let json = loom_serialization::dto::write(&build).unwrap();
    let read = loom_pipeline::read_document(&json).unwrap();
    assert_eq!(read, build);
    match loom_pipeline::reresolve(&resolver, &data, &read).unwrap() {
        Reresolved::Same(again) => assert_eq!(again, ends),
        Reresolved::Changed => panic!("the same bundle resolved differently"),
    }
}

#[test]
fn reresolve_should_report_changed_resolutions_and_keep_the_build() {
    let bundle = bundle();
    let data = BundleData::new(&bundle);
    let resolver = Resolver::new(&data).unwrap();
    let mut build = loom_pipeline::read_pob(&synthetic()).unwrap();
    loom_pipeline::resolve(&resolver, &data, &mut build).unwrap();
    // A stored resolution another resolver could have written.
    let mut stored = build.clone();
    for entry in stored.items.entries.typed_mut() {
        if let loom_domain::ItemsEntry::Item(item) = entry {
            for line in &mut item.body.lines {
                if let loom_domain::LineKind::Modifier(m) = &mut line.kind
                    && matches!(m.resolution, loom_domain::LineResolution::Resolved { .. })
                {
                    m.resolution = loom_domain::LineResolution::Unresolved(
                        loom_domain::UnresolvedReason::NoTemplate,
                    );
                }
            }
        }
    }
    let before = stored.clone();
    assert!(matches!(
        loom_pipeline::reresolve(&resolver, &data, &stored).unwrap(),
        Reresolved::Changed
    ));
    assert_eq!(stored, before);
}

#[test]
fn reset_resolutions_should_keep_exclusions() {
    let bundle = bundle();
    let data = BundleData::new(&bundle);
    let resolver = Resolver::new(&data).unwrap();
    let mut build = loom_pipeline::read_pob(&synthetic()).unwrap();
    loom_pipeline::resolve(&resolver, &data, &mut build).unwrap();
    loom_pipeline::reset_resolutions(&mut build);
    let mut excluded = 0;
    for item in build.items.items() {
        for (_, m) in item.body.modifiers() {
            match m.resolution {
                loom_domain::LineResolution::Pending => {}
                loom_domain::LineResolution::Excluded(_) => excluded += 1,
                ref other => panic!("{other:?}"),
            }
        }
    }
    // `{custom}+15 to maximum Life`.
    assert!(excluded >= 1);
}
