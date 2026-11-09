//! `load-statdesc <extract>`: loads every stat-description file of an
//! extract through `loom-data` and reports counts (Spike B, PR B1).
//!
//! Checks, each a problem that makes the run fail:
//! - the manifest format is supported, and no stat-description file failed
//!   in the extract;
//! - every `ok` file loads, its `source` is the manifest key, and its
//!   description and English-variant counts equal the manifest's;
//! - handler use counts equal the manifest's `stat_descriptions.handlers`;
//! - every file's include chain builds (no missing file, no cycle).
//!
//! A render smoke test then renders every variant of every description
//! (each hand) once, with values its own conditions accept, and counts the
//! outcomes. It shows the renderer runs on all real data and how many
//! variants need a lookup table or have an unknown handler; it is not a
//! correctness check (PR B2b's `round-trip` and `render-check` are).

use std::collections::BTreeMap;

use loom_data::stat_desc::{
    Condition, Hand, Handler, NoLookup, StatDescFile, StatDescManifest, StatDescriptions,
    StatValue, VariantFilter, render_description,
};
use serde::Serialize;

/// Counts and problems of one run. Field order is the JSON order.
#[derive(Debug, Default, Serialize)]
pub struct Summary {
    /// `poe1` or `poe2`.
    pub game: String,
    /// Steam build of the extract.
    pub steam_build_id: Option<String>,
    /// Manifest format.
    pub manifest_format: u64,
    /// Files by manifest status and kind (`ok/descriptions`, ...).
    pub files_by_status: BTreeMap<String, usize>,
    /// Files loaded.
    pub files_loaded: usize,
    /// Description blocks.
    pub descriptions: usize,
    /// Handed descriptions.
    pub handed_descriptions: usize,
    /// English variants.
    pub variants: usize,
    /// Variants by flag.
    pub variant_flags: BTreeMap<String, usize>,
    /// Handler uses by kind.
    pub handlers_by_kind: BTreeMap<String, usize>,
    /// Handler uses by name, for every kind but numeric PoB-cited and
    /// display (lookup, name-derived, unknown, malformed listed by name).
    pub handlers_listed: BTreeMap<String, BTreeMap<String, usize>>,
    /// Placeholder uses by format as written.
    pub placeholder_formats: BTreeMap<String, usize>,
    /// Variants whose text did not parse, by error.
    pub template_errors: BTreeMap<String, usize>,
    /// Variant flags without known meaning, by flag.
    pub unknown_flags: BTreeMap<String, usize>,
    /// Largest number of includes of one file.
    pub max_includes_per_file: usize,
    /// Scopes built (one per file) and their longest chain.
    pub scopes_built: usize,
    /// Longest include chain, in files.
    pub max_chain_length: usize,
    /// Render smoke test: outcomes by name.
    pub render_smoke: BTreeMap<String, usize>,
    /// Everything that failed a check. Empty on success.
    pub problems: Vec<String>,
}

/// Runs the checks. `read` returns the bytes of a path relative to the
/// extract directory.
pub fn run(
    manifest: &[u8],
    read: impl Fn(&str) -> Result<Vec<u8>, String>,
) -> Result<Summary, String> {
    let manifest = StatDescManifest::from_json(manifest)
        .map_err(|e| format!("manifest.json: {} ({e})", e.code()))?;
    let mut s = Summary {
        game: manifest.game.clone(),
        steam_build_id: manifest.steam_build_id.clone(),
        manifest_format: manifest.format,
        ..Summary::default()
    };
    let mut set = StatDescriptions::new();
    for (path, entry) in &manifest.files {
        *s.files_by_status
            .entry(format!("{}/{}", entry.status, entry.kind))
            .or_default() += 1;
        if entry.status == "failed" {
            s.problems.push(format!("{path}: failed in the extract"));
        }
        if entry.status != "ok" {
            continue;
        }
        let Some(output) = &entry.output else {
            s.problems.push(format!("{path}: ok but no output path"));
            continue;
        };
        let file = match read(output).and_then(|bytes| {
            StatDescFile::from_json(&bytes).map_err(|e| format!("{} ({e})", e.code()))
        }) {
            Ok(file) => file,
            Err(e) => {
                s.problems.push(format!("{output}: {e}"));
                continue;
            }
        };
        if file.source != *path {
            s.problems
                .push(format!("{output}: source {} is not {path}", file.source));
        }
        let variants: usize = file.descriptions.iter().map(|d| d.variants.len()).sum();
        if (file.descriptions.len(), variants) != (entry.descriptions, entry.english_variants) {
            s.problems.push(format!(
                "{output}: {} descriptions / {variants} variants, manifest {} / {}",
                file.descriptions.len(),
                entry.descriptions,
                entry.english_variants
            ));
        }
        count_file(&mut s, &file);
        s.files_loaded += 1;
        let source = file.source.clone();
        if set.insert(file).is_some() {
            s.problems
                .push(format!("{source}: loaded twice (same path, any case)"));
        }
    }
    let mut handler_names: BTreeMap<String, usize> = BTreeMap::new();
    for (_, file) in set.files() {
        for v in file.descriptions.iter().flat_map(|d| &d.variants) {
            for h in &v.handlers {
                *handler_names.entry(h.name().to_owned()).or_default() += 1;
            }
        }
    }
    if handler_names != manifest.handlers {
        s.problems
            .push("handler counts differ from the manifest".to_owned());
    }
    for (_, file) in set.files() {
        match set.scope(&file.source) {
            Ok(scope) => {
                s.scopes_built += 1;
                s.max_chain_length = s.max_chain_length.max(scope.chain().len());
            }
            Err(e) => s
                .problems
                .push(format!("{}: {} ({e})", file.source, e.code())),
        }
    }
    Ok(s)
}

fn bump(map: &mut BTreeMap<String, usize>, key: &str) {
    *map.entry(key.to_owned()).or_default() += 1;
}

fn count_file(s: &mut Summary, file: &StatDescFile) {
    s.max_includes_per_file = s.max_includes_per_file.max(file.includes.len());
    s.descriptions += file.descriptions.len();
    for d in &file.descriptions {
        if d.off_hand_stats.is_some() {
            s.handed_descriptions += 1;
        }
        for v in &d.variants {
            s.variants += 1;
            if v.gem_quality {
                bump(&mut s.variant_flags, "gem_quality");
            }
            if v.table_only {
                bump(&mut s.variant_flags, "table_only");
            }
            for flag in &v.unknown_flags {
                bump(&mut s.unknown_flags, flag);
            }
            for h in &v.handlers {
                let kind = h.kind().name();
                bump(&mut s.handlers_by_kind, kind);
                if matches!(
                    h,
                    Handler::Lookup { .. }
                        | Handler::Unknown { .. }
                        | Handler::Malformed { .. }
                        | Handler::Numeric { .. }
                ) && kind != "numeric-pob-cited"
                {
                    bump(
                        s.handlers_listed.entry(kind.to_owned()).or_default(),
                        h.name(),
                    );
                }
            }
            match &v.template {
                Ok(t) => {
                    for (_, written) in t.placeholders() {
                        bump(&mut s.placeholder_formats, written);
                    }
                }
                Err(e) => bump(&mut s.template_errors, e.name()),
            }
        }
        smoke(s, d);
    }
}

/// A value that `condition` accepts.
fn accepted(condition: Condition) -> i64 {
    match condition {
        Condition::Range { min: Some(v), .. } | Condition::Range { max: Some(v), .. } => v,
        Condition::Range { .. } => 1,
        Condition::Not(v) => {
            if v == 1 {
                2
            } else {
                1
            }
        }
    }
}

fn smoke(s: &mut Summary, d: &loom_data::stat_desc::Description) {
    for hand in [Hand::Main, Hand::Off] {
        if d.stats_of(hand).is_none() {
            continue;
        }
        for (i, v) in d.variants.iter().enumerate() {
            let filter = VariantFilter {
                gem_quality: v.gem_quality,
                table_only: v.table_only,
            };
            let values: Vec<StatValue> = v
                .conditions
                .iter()
                .map(|c| StatValue::exact(accepted(*c)))
                .collect();
            let outcome = match render_description(d, hand, &values, &NoLookup, filter) {
                Ok((chosen, _)) if chosen == i => "rendered".to_owned(),
                Ok(_) => "rendered-earlier-variant".to_owned(),
                Err(reason) => reason.name().to_owned(),
            };
            bump(&mut s.render_smoke, &outcome);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = r#"{"format": 3, "game": "poe1", "steam_build_id": "1",
        "stat_descriptions": {"files": {
          "metadata/a.txt": {"kind": "descriptions", "status": "ok", "output": "a.json",
                             "descriptions": 1, "english_variants": 1},
          "metadata/f.txt": {"kind": "filters", "status": "skipped", "output": null,
                             "descriptions": 0, "english_variants": 0}},
        "handlers": {"negate": 1}}}"#;

    const FILE: &str = r#"{"descriptions": [{"handed":false,"line":1,"name":null,
        "off_hand_stats":null,"stats":["s"],"variants":[{"conditions":[{"kind":"range",
        "max":-1,"min":null}],"flags":[],"handlers":[{"name":"negate","stat":1}],"line":2,
        "placeholders":[{"format":"","index":0}],"text":"{0}% reduced X"}]}],
        "file_flags": [], "includes": [], "languages": {}, "no_description": [],
        "source": "metadata/a.txt"}"#;

    fn read(path: &str) -> Result<Vec<u8>, String> {
        (path == "a.json")
            .then(|| FILE.as_bytes().to_vec())
            .ok_or_else(|| format!("{path}: not found"))
    }

    #[test]
    fn run_should_load_count_and_smoke_render_a_consistent_extract() {
        let s = run(MANIFEST.as_bytes(), read).unwrap();
        assert!(s.problems.is_empty(), "{:?}", s.problems);
        assert_eq!((s.files_loaded, s.descriptions, s.variants), (1, 1, 1));
        assert_eq!(s.render_smoke.get("rendered"), Some(&1));
        assert_eq!(s.scopes_built, 1);
    }

    #[test]
    fn run_should_report_counts_that_differ_from_the_manifest() {
        let manifest = MANIFEST.replace(r#""negate": 1"#, r#""negate": 2"#);
        let s = run(manifest.as_bytes(), read).unwrap();
        assert_eq!(s.problems, vec!["handler counts differ from the manifest"]);
    }

    #[test]
    fn run_should_report_an_unreadable_file_and_a_missing_include() {
        let manifest = MANIFEST.replace(r#""output": "a.json""#, r#""output": "b.json""#);
        let s = run(manifest.as_bytes(), read).unwrap();
        assert!(s.problems.iter().any(|p| p.starts_with("b.json")));
        let read_with_include = |path: &str| {
            read(path).map(|_| {
                FILE.replace(r#""includes": []"#, r#""includes": ["x.txt"]"#)
                    .into_bytes()
            })
        };
        let s = run(MANIFEST.as_bytes(), read_with_include).unwrap();
        assert!(
            s.problems
                .iter()
                .any(|p| p.contains("DATA_STATDESC_SCOPE_MISSING"))
        );
    }

    #[test]
    fn run_should_refuse_an_unsupported_manifest_format() {
        let err = run(br#"{"format": 9}"#, read).unwrap_err();
        assert!(err.contains("DATA_STATDESC_FORMAT"));
    }

    #[test]
    fn accepted_should_satisfy_each_condition_form() {
        for c in [
            Condition::Range {
                min: Some(3),
                max: None,
            },
            Condition::Range {
                min: None,
                max: Some(-1),
            },
            Condition::Range {
                min: None,
                max: None,
            },
            Condition::Not(1),
            Condition::Not(0),
        ] {
            assert!(c.holds(accepted(c)), "{c:?}");
        }
    }
}
