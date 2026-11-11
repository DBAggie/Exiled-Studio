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

/// `src` with `edit` applied to the JSON of `path`, rehashed.
fn edited(name: &str, path: &str, edit: impl FnOnce(&mut Value)) -> MemorySource {
    let mut src = source(name);
    let mut v = json_of(&src, path);
    edit(&mut v);
    put(&mut src, path, &v);
    rehash(&mut src);
    src
}

/// Each reference the skill tables hold, pointed at nothing, gives
/// `BUNDLE_DANGLING_REF` alone (Phase 3 plan, step 1a).
#[test]
fn dangling_skill_references_should_each_give_their_code() {
    type Edit = fn(&mut Value);
    let cases: [(&str, &str, Edit); 22] = [
        ("flag stat", "data/skill_types.json", |v| {
            v[0]["flag_stat"] = "nope".into();
        }),
        ("active skill type", "data/active_skills.json", |v| {
            v[0]["types"] = json!(["Nope"]);
        }),
        ("minion type", "data/active_skills.json", |v| {
            v[0]["minion_types"] = json!(["Nope"]);
        }),
        ("weapon class", "data/active_skills.json", |v| {
            v[0]["weapon_classes"] = json!(["Nope"]);
        }),
        ("active skill", "data/granted_effects.json", |v| {
            v[0]["active_skill"] = "nope".into();
        }),
        ("allowed type", "data/granted_effects.json", |v| {
            v[2]["allowed_types"] = json!(["Nope"]);
        }),
        ("added type", "data/granted_effects.json", |v| {
            v[2]["added_types"] = json!(["Nope"]);
        }),
        ("excluded type", "data/granted_effects.json", |v| {
            v[2]["excluded_types"] = json!(["Nope"]);
        }),
        ("added minion type", "data/granted_effects.json", |v| {
            v[2]["added_minion_types"] = json!(["Nope"]);
        }),
        ("cost type", "data/granted_effects.json", |v| {
            v[0]["levels"][0]["costs"][0]["resource"] = "Nope".into();
        }),
        ("additional effect", "data/gem_effects.json", |v| {
            v[0]["additional_granted_effects"] = json!(["Nope"]);
        }),
        ("stat set", "data/granted_effects.json", |v| {
            v[0]["stat_sets"] = json!(["Nope"]);
        }),
        ("quality stat", "data/granted_effects.json", |v| {
            v[0]["quality"][0]["stat"] = "nope".into();
        }),
        ("implicit stat", "data/stat_sets.json", |v| {
            v[0]["implicit_stats"] = json!(["nope"]);
        }),
        ("constant stat", "data/stat_sets.json", |v| {
            v[0]["constant_stats"][0]["stat"] = "nope".into();
        }),
        ("float stat", "data/stat_sets.json", |v| {
            v[0]["levels"][0]["float_stats"][0]["stat"] = "nope".into();
        }),
        ("level stat", "data/stat_sets.json", |v| {
            v[0]["levels"][0]["stats"][0]["stat"] = "nope".into();
        }),
        ("level flag", "data/stat_sets.json", |v| {
            v[0]["levels"][0]["flags"] = json!(["nope"]);
        }),
        ("level effect", "data/stat_sets.json", |v| {
            v[0]["levels"][0]["granted_effects"] = json!(["Nope"]);
        }),
        ("gem base", "data/gems.json", |v| {
            v[0]["base_item"] = "Nope".into();
        }),
        ("gem effect", "data/gems.json", |v| {
            v[0]["effects"] = json!(["Nope"]);
        }),
        ("effect tag", "data/gem_effects.json", |v| {
            v[0]["tags"] = json!(["nope"]);
        }),
    ];
    for (what, path, edit) in cases {
        let src = edited("bundle-poe1", path, edit);
        assert_eq!(codes(&src), ["BUNDLE_DANGLING_REF"], "{what}");
    }
}

#[test]
fn empty_skill_id_should_give_its_code() {
    let src = edited("bundle-poe2", "data/gem_tags.json", |v| {
        v.as_array_mut()
            .unwrap()
            .push(json!({"id": "", "name": "Nameless"}));
    });
    assert_eq!(codes(&src), ["BUNDLE_EMPTY_ID"]);
}

#[test]
fn duplicate_skill_ids_should_give_their_code() {
    for path in [
        "data/skill_types.json",
        "data/cost_types.json",
        "data/active_skills.json",
        "data/granted_effects.json",
        "data/stat_sets.json",
        "data/gems.json",
        "data/gem_effects.json",
        "data/gem_tags.json",
    ] {
        let src = edited("bundle-poe2", path, |v| {
            let first = v[0].clone();
            v.as_array_mut().unwrap().push(first);
        });
        assert_eq!(codes(&src), ["BUNDLE_DUPLICATE_ID"], "{path}");
    }
}

#[test]
fn skill_levels_out_of_order_should_give_their_code() {
    for path in ["data/granted_effects.json", "data/stat_sets.json"] {
        let src = edited("bundle-poe2", path, |v| {
            v[0]["levels"].as_array_mut().unwrap().reverse();
        });
        assert_eq!(codes(&src), ["BUNDLE_SKILL_LEVELS"], "{path} reversed");
        let src = edited("bundle-poe2", path, |v| {
            let levels = v[0]["levels"].as_array_mut().unwrap();
            levels[1]["level"] = levels[0]["level"].clone();
        });
        assert_eq!(codes(&src), ["BUNDLE_SKILL_LEVELS"], "{path} repeated");
    }
}

/// The bundle as format 1 (no skill files), rehashed.
fn format_1(name: &str) -> MemorySource {
    let mut src = source(name);
    for p in loom_data::bundle::calc_files() {
        src.remove(p);
    }
    let mut m = json_of(&src, MANIFEST);
    m["format"] = 1.into();
    m["files"]
        .as_array_mut()
        .unwrap()
        .retain(|f| !loom_data::bundle::calc_files().any(|c| f["path"] == c));
    put(&mut src, MANIFEST, &m);
    src
}

#[test]
fn format_1_bundles_should_still_be_valid() {
    for name in ["bundle-poe1", "bundle-poe2"] {
        assert_eq!(validate(&format_1(name)).findings, [], "{name}");
    }
}

#[test]
fn skill_file_in_a_format_1_bundle_should_be_refused() {
    let mut src = format_1("bundle-poe2");
    src.insert("data/gem_tags.json", "[]");
    list(&mut src, "data/gem_tags.json");
    rehash(&mut src);
    assert_eq!(codes(&src), ["BUNDLE_FORMAT_FILE"]);
}

/// Each reference the character and passive tables hold, pointed at
/// nothing, gives `BUNDLE_DANGLING_REF` alone (Phase 3 plan, step 1b).
#[test]
fn dangling_passive_references_should_each_give_their_code() {
    type Edit = fn(&mut Value);
    let cases: [(&str, &str, Edit); 13] = [
        ("class template", "data/characters.json", |v| {
            v[0]["template"] = "nope".into();
        }),
        ("ascendancy class", "data/ascendancies.json", |v| {
            v[0]["character"] = "Nope".into();
        }),
        ("template parent", "data/object_templates.json", |v| {
            v[1]["extends"] = "nope".into();
        }),
        ("template stat", "data/object_templates.json", |v| {
            v[0]["stats"][0]["stat"] = "nope".into();
        }),
        ("passive stat", "data/passives.json", |v| {
            v[1]["stats"][0]["stat"] = "nope".into();
        }),
        ("passive mastery", "data/passives.json", |v| {
            v[3]["mastery_group"] = "Nope".into();
        }),
        ("passive ascendancy", "data/passives.json", |v| {
            v[4]["ascendancy"] = "Nope".into();
        }),
        ("passive class", "data/passives.json", |v| {
            v[0]["characters"] = json!(["Nope"]);
        }),
        ("group effect", "data/mastery_groups.json", |v| {
            v[0]["effects"] = json!(["Nope"]);
        }),
        ("effect stat", "data/mastery_effects.json", |v| {
            v[0]["stats"][0]["stat"] = "nope".into();
        }),
        ("tree node", "data/passive_tree.json", |v| {
            v["groups"][1]["nodes"][0]["node"] = 999.into();
        }),
        ("tree connection", "data/passive_tree.json", |v| {
            v["groups"][0]["nodes"][0]["connections"][0]["node"] = 999.into();
        }),
        ("tree root", "data/passive_tree.json", |v| {
            v["roots"] = json!([999]);
        }),
    ];
    for (what, path, edit) in cases {
        let src = edited("bundle-poe1", path, edit);
        assert_eq!(codes(&src), ["BUNDLE_DANGLING_REF"], "{what}");
    }
}

#[test]
fn duplicate_passive_ids_and_numbers_should_give_their_code() {
    for path in [
        "data/characters.json",
        "data/ascendancies.json",
        "data/object_templates.json",
        "data/mastery_groups.json",
        "data/mastery_effects.json",
    ] {
        let src = edited("bundle-poe2", path, |v| {
            let first = v[0].clone();
            v.as_array_mut().unwrap().push(first);
        });
        assert_eq!(codes(&src), ["BUNDLE_DUPLICATE_ID"], "{path}");
    }
    // A passive repeated: its ID and its node number.
    let src = edited("bundle-poe2", "data/passives.json", |v| {
        let first = v[0].clone();
        v.as_array_mut().unwrap().push(first);
    });
    assert_eq!(validate(&src).findings.len(), 2);
    assert_eq!(codes(&src), ["BUNDLE_DUPLICATE_ID"]);
    // Two passives on one node; a tree node twice; two effects, one hash.
    type Edit = fn(&mut Value);
    let numbers: [(&str, Edit); 6] = [
        ("data/passive_tree.json", |v| v["roots"] = json!([100, 100])),
        ("data/characters.json", |v| v[1]["class_number"] = 1.into()),
        ("data/ascendancies.json", |v| {
            let mut other = v[0].clone();
            other["id"] = "Other".into();
            v.as_array_mut().unwrap().push(other);
        }),
        ("data/passives.json", |v| {
            let mut other = v[0].clone();
            other["id"] = "other".into();
            v.as_array_mut().unwrap().push(other);
        }),
        ("data/passive_tree.json", |v| {
            v["groups"][1]["nodes"][0]["node"] = 100.into();
        }),
        ("data/mastery_effects.json", |v| {
            let mut other = v[0].clone();
            other["id"] = "Other".into();
            v.as_array_mut().unwrap().push(other);
        }),
    ];
    for (path, edit) in numbers {
        let src = edited("bundle-poe2", path, edit);
        assert_eq!(codes(&src), ["BUNDLE_DUPLICATE_ID"], "{path}");
    }
}

#[test]
fn empty_passive_id_should_give_its_code() {
    let src = edited("bundle-poe2", "data/passives.json", |v| {
        v[2]["id"] = "".into();
    });
    assert_eq!(codes(&src), ["BUNDLE_EMPTY_ID"]);
}

#[test]
fn template_chain_that_loops_should_give_its_code() {
    let src = edited("bundle-poe1", "data/object_templates.json", |v| {
        v[0]["extends"] = "test/chars/character".into();
    });
    assert_eq!(codes(&src), ["BUNDLE_TEMPLATE_CYCLE"], "itself");
    let src = edited("bundle-poe1", "data/object_templates.json", |v| {
        v[0]["extends"] = "test/chars/str".into();
    });
    let found = validate(&src).findings;
    assert_eq!(
        (codes(&src), found.len()),
        (vec!["BUNDLE_TEMPLATE_CYCLE"], 2),
        "two in a loop"
    );
}

#[test]
fn tree_node_outside_its_orbit_should_give_its_code() {
    type Edit = fn(&mut Value);
    let cases: [(&str, Edit); 3] = [
        ("orbit past the slots", |v| {
            v["groups"][0]["nodes"][1]["orbit"] = 2.into();
        }),
        ("negative orbit", |v| {
            v["groups"][0]["nodes"][1]["orbit"] = (-1).into();
        }),
        ("position past the orbit", |v| {
            v["groups"][0]["nodes"][1]["orbit_index"] = 6.into();
        }),
    ];
    for (what, edit) in cases {
        let src = edited("bundle-poe2", "data/passive_tree.json", edit);
        assert_eq!(codes(&src), ["BUNDLE_TREE_GEOMETRY"], "{what}");
    }
}
