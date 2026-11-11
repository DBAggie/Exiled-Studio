//! Loads the synthetic PoE1 and PoE2 bundles (`tests/fixtures/`, written by
//! `make_bundles.py`) and checks the loader's error codes on mutations
//! (Phase 2 plan, step 3; decision 10).

#![expect(
    clippy::disallowed_methods,
    reason = "a test reads its own fixture files into a memory source; never part of a core build"
)]
#![expect(
    clippy::unwrap_used,
    reason = "a test helper edits fixed fixture JSON; a failure should panic"
)]

use std::fs;
use std::path::{Path, PathBuf};

use loom_data::bundle::{self, Bundle, MemorySource};
use loom_data::stat_desc::{Lookup, LookupHandler};
use loom_domain::Game;

fn files(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            files(&path, out)?;
        } else {
            out.push(path);
        }
    }
    Ok(())
}

/// The fixture bundle `name` as a memory source.
fn source(name: &str) -> std::io::Result<MemorySource> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    let mut paths = Vec::new();
    files(&root, &mut paths)?;
    let mut src = MemorySource::new();
    for p in paths {
        let rel = p
            .strip_prefix(&root)
            .map_err(std::io::Error::other)?
            .to_string_lossy()
            .replace('\\', "/");
        src.insert(rel, fs::read(&p)?);
    }
    Ok(src)
}

fn load(src: &MemorySource) -> Result<Bundle, &'static str> {
    Bundle::load(src).map_err(|e| e.code())
}

/// Replaces the manifest with `edit` applied to its JSON.
fn edit_manifest(src: &mut MemorySource, edit: impl FnOnce(&mut serde_json::Value)) {
    let mut m: serde_json::Value =
        serde_json::from_slice(&bundle::BundleSource::read(src, bundle::MANIFEST).unwrap())
            .unwrap();
    edit(&mut m);
    src.insert(bundle::MANIFEST, serde_json::to_vec(&m).unwrap());
}

#[test]
fn poe1_bundle_should_load_with_its_poe1_section() {
    let b = load(&source("bundle-poe1").unwrap()).unwrap();
    let poe1 = b.poe1.as_ref().unwrap();
    assert_eq!(
        (
            b.game,
            b.stats.len(),
            b.mods.len(),
            b.base_items.len(),
            poe1.grafts.as_ref().map(|g| g.bases[0].gem.as_str()),
            poe1.quality_types.as_ref().map(Vec::len),
        ),
        (Game::Poe1, 11, 3, 3, Some("Test/Gems/Bolt"), Some(1))
    );
}

#[test]
fn poe2_bundle_should_load_into_the_same_types_without_a_poe1_section() {
    let b = load(&source("bundle-poe2").unwrap()).unwrap();
    assert_eq!(
        (b.game, b.poe1.is_none(), b.mods.len()),
        (Game::Poe2, true, 3)
    );
}

#[test]
fn bundles_should_carry_valid_item_rules() {
    for name in ["bundle-poe1", "bundle-poe2"] {
        let b = load(&source(name).unwrap()).unwrap();
        assert_eq!(b.item_rules.mods.validate(), Ok(()), "{name}");
    }
}

#[test]
fn bundle_should_load_every_listed_stat_description_file() {
    let b = load(&source("bundle-poe2").unwrap()).unwrap();
    let scope = b
        .stat_descriptions
        .scope("Metadata/StatDescriptions/charm_stat_descriptions.txt");
    assert!(scope.is_ok(), "{scope:?}");
}

#[test]
fn bundle_should_answer_lookups_from_its_tables() {
    let b = load(&source("bundle-poe1").unwrap()).unwrap();
    assert_eq!(
        (
            b.name(LookupHandler::PassiveHash, 1),
            b.entries(LookupHandler::PassiveHash).len(),
            b.entries(LookupHandler::PassiveKeystoneIndex).len(),
        ),
        (Some("Test Node".to_owned()), 1, 0)
    );
}

#[test]
fn load_should_refuse_an_unknown_format() {
    let mut src = source("bundle-poe2").unwrap();
    edit_manifest(&mut src, |m| m["format"] = 2.into());
    assert_eq!(load(&src).err(), Some("BUNDLE_FORMAT"));
}

#[test]
fn load_should_refuse_an_unknown_game() {
    let mut src = source("bundle-poe2").unwrap();
    edit_manifest(&mut src, |m| m["game"] = "poe3".into());
    assert_eq!(load(&src).err(), Some("BUNDLE_GAME"));
}

#[test]
fn load_should_refuse_a_manifest_without_a_required_file() {
    let mut src = source("bundle-poe2").unwrap();
    edit_manifest(&mut src, |m| {
        let files = m["files"].as_array_mut().unwrap();
        files.retain(|f| f["path"] != "data/mods.json");
    });
    assert_eq!(load(&src).err(), Some("BUNDLE_MISSING_FILE"));
}

fn add_listed(src: &mut MemorySource, path: &str) {
    let p = path.to_owned();
    edit_manifest(src, move |m| {
        m["files"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"path": p, "sha256": "0", "size": 0}));
    });
}

#[test]
fn load_should_refuse_a_path_that_leaves_the_bundle() {
    for bad in [
        "../secret.json",
        "/abs.json",
        "data\\x.json",
        "c:/x.json",
        "data//x.json",
    ] {
        let mut src = source("bundle-poe2").unwrap();
        add_listed(&mut src, bad);
        assert_eq!(load(&src).err(), Some("BUNDLE_BAD_PATH"), "{bad}");
    }
}

#[test]
fn load_should_refuse_a_path_listed_twice() {
    let mut src = source("bundle-poe2").unwrap();
    add_listed(&mut src, "data/stats.json");
    assert_eq!(load(&src).err(), Some("BUNDLE_DUPLICATE_PATH"));
}

#[test]
fn load_should_refuse_a_poe1_section_in_a_poe2_bundle() {
    let mut src = source("bundle-poe2").unwrap();
    add_listed(&mut src, "poe1/grafts.json");
    assert_eq!(load(&src).err(), Some("BUNDLE_GAME_SECTION"));
}

#[test]
fn load_should_refuse_a_listed_file_the_source_lacks() {
    let mut src = source("bundle-poe2").unwrap();
    src.remove("data/stats.json");
    assert_eq!(load(&src).err(), Some("BUNDLE_SOURCE_NOT_FOUND"));
}

#[test]
fn load_should_refuse_an_unknown_field() {
    let mut src = source("bundle-poe2").unwrap();
    src.insert("data/stats.json", r#"[{"id": "a", "colour": "red"}]"#);
    assert_eq!(load(&src).err(), Some("BUNDLE_JSON"));
}

#[test]
fn load_should_refuse_a_broken_stat_description_file() {
    let mut src = source("bundle-poe2").unwrap();
    src.insert("stat_descriptions/stat_descriptions.json", "{}");
    assert_eq!(load(&src).err(), Some("BUNDLE_STAT_DESCRIPTION"));
}

#[test]
fn scope_root_should_find_one_file_by_name_in_any_case() {
    let mut b = load(&source("bundle-poe1").unwrap()).unwrap();
    assert_eq!(
        (b.scope_root("STAT_DESCRIPTIONS"), b.scope_root("absent"),),
        (
            Ok("Metadata/StatDescriptions/stat_descriptions.txt"),
            Err(bundle::ScopeRootError::None),
        )
    );
    let mut twin = b
        .stat_descriptions
        .get("Metadata/StatDescriptions/stat_descriptions.txt")
        .unwrap()
        .clone();
    twin.source = "Metadata/Other/stat_descriptions.csd".to_owned();
    b.stat_descriptions.insert(twin);
    assert_eq!(
        b.scope_root("stat_descriptions"),
        Err(bundle::ScopeRootError::Several(2))
    );
}
