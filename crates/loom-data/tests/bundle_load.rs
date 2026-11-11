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

use loom_data::bundle::{self, Bundle, MemorySource, skills};
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
        (Game::Poe1, 22, 3, 4, Some("Test/Gems/Bolt"), Some(1))
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
    edit_manifest(&mut src, |m| m["format"] = 9.into());
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

#[test]
fn format_2_bundles_should_load_their_skill_tables_in_each_game_shape() {
    let one = load(&source("bundle-poe1").unwrap()).unwrap();
    let two = load(&source("bundle-poe2").unwrap()).unwrap();
    let shape = |b: &Bundle| {
        let sk = b.skills.as_ref().unwrap();
        let bolt = &sk.stat_sets[0];
        let support = &sk.granted_effects[2];
        (
            (
                sk.skill_types.len(),
                sk.active_skills.len(),
                sk.granted_effects.len(),
                sk.stat_sets.len(),
                sk.gems.len(),
                sk.gem_effects.len(),
                sk.gem_tags.len(),
            ),
            // f32 values read back exactly.
            (bolt.base_effectiveness, bolt.levels[1].float_stats[0].value),
            bolt.levels[0].damage_effectiveness,
            bolt.damage_incremental_effectiveness,
            support.levels[0].reservations[0].resource,
        )
    };
    assert_eq!(
        (shape(&one), shape(&two)),
        (
            (
                (3, 2, 3, 3, 2, 2, 2),
                (1.5, 6.25),
                Some(0),
                None,
                skills::Resource::Mana
            ),
            (
                (3, 2, 3, 3, 2, 2, 2),
                (1.5, 6.25),
                None,
                Some(0.01),
                skills::Resource::Spirit
            ),
        )
    );
}

/// The bundle as format 1: no skill files, `format` 1.
fn as_format_1(src: &mut MemorySource) {
    for path in bundle::calc_files() {
        src.remove(path);
    }
    edit_manifest(src, |m| {
        m["format"] = 1.into();
        m["files"]
            .as_array_mut()
            .unwrap()
            .retain(|f| !loom_data::bundle::calc_files().any(|c| f["path"] == c));
    });
}

#[test]
fn format_1_bundle_should_still_load_for_import_without_skill_tables() {
    for name in ["bundle-poe1", "bundle-poe2"] {
        let mut src = source(name).unwrap();
        as_format_1(&mut src);
        let b = load(&src).unwrap();
        assert_eq!(
            (b.manifest.format, b.skills.is_none(), b.mods.len()),
            (1, true, 3),
            "{name}"
        );
    }
}

#[test]
fn load_should_refuse_a_skill_file_in_a_format_1_bundle() {
    let mut src = source("bundle-poe2").unwrap();
    as_format_1(&mut src);
    add_listed(&mut src, skills::GEMS);
    assert_eq!(load(&src).err(), Some("BUNDLE_FORMAT_FILE"));
}

#[test]
fn load_should_refuse_a_format_2_bundle_without_a_skill_file() {
    for path in bundle::calc_files() {
        let mut src = source("bundle-poe1").unwrap();
        edit_manifest(&mut src, |m| {
            m["files"]
                .as_array_mut()
                .unwrap()
                .retain(|f| f["path"] != path);
        });
        assert_eq!(load(&src).err(), Some("BUNDLE_MISSING_FILE"), "{path}");
    }
}

#[test]
fn load_should_refuse_an_unknown_field_in_a_skill_row() {
    let mut src = source("bundle-poe2").unwrap();
    src.insert(skills::GEM_TAGS, r#"[{"id": "a", "colour": "red"}]"#);
    assert_eq!(load(&src).err(), Some("BUNDLE_JSON"));
}

fn stat_set(base: f32, damage: Option<f32>) -> skills::StatSet {
    skills::StatSet {
        id: "x".to_owned(),
        implicit_stats: Vec::new(),
        constant_stats: Vec::new(),
        base_effectiveness: base,
        incremental_effectiveness: base,
        damage_incremental_effectiveness: damage,
        levels: Vec::new(),
    }
}

/// `f32` fields are written as their shortest text and read back to the
/// same bits, over a spread of bit patterns (normal, subnormal, large).
#[test]
fn f32_fields_should_round_trip_through_their_shortest_text() {
    let json = serde_json::to_string(&stat_set(2.921, Some(0.0495))).unwrap();
    assert!(
        json.contains("\"base_effectiveness\":2.921,") && json.contains(":0.0495,"),
        "{json}"
    );
    let mut bits: u32 = 1;
    while bits < 0x7f80_0000 {
        let v = f32::from_bits(bits);
        let set = stat_set(v, Some(-v));
        let back: skills::StatSet =
            serde_json::from_str(&serde_json::to_string(&set).unwrap()).unwrap();
        assert_eq!(
            (
                back.base_effectiveness.to_bits(),
                back.damage_incremental_effectiveness.map(f32::to_bits)
            ),
            (bits, Some((-v).to_bits())),
            "{v:e}"
        );
        bits = bits.saturating_add(0x0001_3579);
    }
}

#[test]
fn format_2_bundles_should_load_their_passive_tables_in_each_game_shape() {
    let shape = |name: &str| {
        let b = load(&source(name).unwrap()).unwrap();
        let p = b.passives.unwrap();
        (
            (
                p.characters.len(),
                p.ascendancies.len(),
                p.object_templates.len(),
                p.passives.len(),
                p.mastery_groups.len(),
                p.mastery_effects.len(),
                p.tree.groups.len(),
            ),
            p.characters[0].template.clone(),
            p.object_templates[0].values.len(),
            (p.tree.groups[0].x, p.tree.groups[0].y),
            p.tree.groups[0].nodes[1].connections[0].orbit,
        )
    };
    assert_eq!(
        (shape("bundle-poe1"), shape("bundle-poe2")),
        (
            (
                (2, 1, 2, 5, 1, 1, 2),
                Some("test/chars/str".to_owned()),
                0,
                (-1.5, 2.25),
                Some(-2)
            ),
            ((2, 1, 1, 5, 1, 1, 2), None, 1, (-1.5, 2.25), Some(-2)),
        )
    );
}
