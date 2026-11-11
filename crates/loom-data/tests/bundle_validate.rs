//! The validator's mutation set (Phase 2 plan, step 4; gate item 4): each
//! case changes one thing in a synthetic bundle and must give exactly its
//! expected code.

#![expect(
    clippy::disallowed_methods,
    reason = "a test reads its own fixture files into a memory source; never part of a core build"
)]
#![expect(
    clippy::unwrap_used,
    reason = "test helpers edit fixed fixture JSON; a failure should panic"
)]

use std::fs;
use std::path::{Path, PathBuf};

use loom_data::bundle::validate::validate;
use loom_data::bundle::{BundleSource, MANIFEST, MemorySource};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

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

fn source(name: &str) -> MemorySource {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    let mut paths = Vec::new();
    files(&root, &mut paths);
    let mut src = MemorySource::new();
    for p in paths {
        let rel = p
            .strip_prefix(&root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        if rel.ends_with(".py") {
            continue;
        }
        src.insert(rel, fs::read(&p).unwrap());
    }
    src
}

fn json_of(src: &MemorySource, path: &str) -> Value {
    serde_json::from_slice(&src.read(path).unwrap()).unwrap()
}

fn put(src: &mut MemorySource, path: &str, v: &Value) {
    src.insert(path, serde_json::to_vec_pretty(v).unwrap());
}

/// Recomputes every listed file's SHA-256 and size, so a content change
/// reaches the checks after the structural ones.
fn rehash(src: &mut MemorySource) {
    let mut m = json_of(src, MANIFEST);
    for f in m["files"].as_array_mut().unwrap() {
        let bytes = src.read(f["path"].as_str().unwrap()).unwrap();
        let sha: String = Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        f["sha256"] = sha.into();
        f["size"] = bytes.len().into();
    }
    put(src, MANIFEST, &m);
}

fn list(src: &mut MemorySource, path: &str) {
    let mut m = json_of(src, MANIFEST);
    m["files"]
        .as_array_mut()
        .unwrap()
        .push(json!({"path": path, "sha256": "", "size": 0}));
    put(src, MANIFEST, &m);
}

fn codes(src: &MemorySource) -> Vec<&'static str> {
    validate(src).codes()
}

#[test]
fn synthetic_bundles_should_be_valid() {
    for name in ["bundle-poe1", "bundle-poe2"] {
        assert_eq!(validate(&source(name)).findings, [], "{name}");
    }
}

#[test]
fn missing_file_should_give_its_code() {
    let mut src = source("bundle-poe2");
    src.remove("data/mods.json");
    assert_eq!(codes(&src), ["BUNDLE_SOURCE_NOT_FOUND"]);
}

#[test]
fn changed_byte_should_give_a_hash_mismatch() {
    let mut src = source("bundle-poe2");
    let mut bytes = src.read("data/stats.json").unwrap();
    let at = bytes.iter().position(|b| *b == b'l').unwrap();
    bytes[at] = b'L';
    src.insert("data/stats.json", bytes);
    assert_eq!(codes(&src), ["BUNDLE_HASH_MISMATCH"]);
}

#[test]
fn extra_file_should_be_unlisted() {
    let mut src = source("bundle-poe2");
    src.insert("data/extra.json", "[]");
    assert_eq!(codes(&src), ["BUNDLE_UNLISTED_FILE"]);
}

#[test]
fn dangling_reference_should_give_its_code() {
    let mut src = source("bundle-poe2");
    let mut bases = json_of(&src, "data/base_items.json");
    bases[0]["implicits"] = json!(["NoSuchMod"]);
    put(&mut src, "data/base_items.json", &bases);
    rehash(&mut src);
    assert_eq!(codes(&src), ["BUNDLE_DANGLING_REF"]);
}

#[test]
fn duplicate_id_should_give_its_code() {
    let mut src = source("bundle-poe2");
    let mut stats = json_of(&src, "data/stats.json");
    let first = stats[0].clone();
    stats.as_array_mut().unwrap().push(first);
    put(&mut src, "data/stats.json", &stats);
    rehash(&mut src);
    assert_eq!(codes(&src), ["BUNDLE_DUPLICATE_ID"]);
}

#[test]
fn wrong_game_tag_should_refuse_the_poe1_section() {
    let mut src = source("bundle-poe1");
    let mut m = json_of(&src, MANIFEST);
    m["game"] = "poe2".into();
    put(&mut src, MANIFEST, &m);
    assert_eq!(codes(&src), ["BUNDLE_GAME_SECTION"]);
}

#[test]
fn poe1_section_in_a_poe2_bundle_should_be_refused() {
    let mut src = source("bundle-poe2");
    src.insert("poe1/grafts.json", r#"{"bases": [], "levels": []}"#);
    list(&mut src, "poe1/grafts.json");
    rehash(&mut src);
    assert_eq!(codes(&src), ["BUNDLE_GAME_SECTION"]);
}

#[test]
fn unknown_format_should_give_its_code() {
    let mut src = source("bundle-poe2");
    let mut m = json_of(&src, MANIFEST);
    m["format"] = 9.into();
    put(&mut src, MANIFEST, &m);
    assert_eq!(codes(&src), ["BUNDLE_FORMAT"]);
}

#[test]
fn signatures_should_be_empty_in_format_1() {
    let mut src = source("bundle-poe2");
    let mut m = json_of(&src, MANIFEST);
    m["signatures"] = json!([{"by": "someone"}]);
    put(&mut src, MANIFEST, &m);
    assert_eq!(codes(&src), ["BUNDLE_SIGNATURES"]);
}

#[test]
fn client_text_the_rules_name_should_exist() {
    let mut src = source("bundle-poe2");
    put(&mut src, "data/client_texts.json", &json!([]));
    rehash(&mut src);
    assert_eq!(codes(&src), ["BUNDLE_MISSING_CLIENT_TEXT"]);
}

#[test]
fn scope_the_rules_name_should_build() {
    let mut src = source("bundle-poe2");
    let path = "stat_descriptions/charm_stat_descriptions.json";
    src.remove(path);
    let mut m = json_of(&src, MANIFEST);
    m["files"]
        .as_array_mut()
        .unwrap()
        .retain(|f| f["path"] != path);
    put(&mut src, MANIFEST, &m);
    assert_eq!(codes(&src), ["BUNDLE_SCOPE"]);
}

#[test]
fn item_rules_that_do_not_validate_should_give_their_code() {
    let mut src = source("bundle-poe2");
    let mut rules = json_of(&src, "data/item_rules.json");
    rules["mods"]["fits"] = json!([]);
    put(&mut src, "data/item_rules.json", &rules);
    rehash(&mut src);
    assert_eq!(codes(&src), ["BUNDLE_ITEM_RULES"]);
}
