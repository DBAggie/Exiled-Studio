//! The fixture manifest (`tools/oracle/fixtures/fixtures.json`, Spike C
//! plan, step 3): the public fixtures by tier, each pinned and with the
//! outcome it must have. `oracle-run` takes the manifest in place of
//! fixture paths; a fixture that differs from the manifest is refused
//! before anything runs.
//!
//! Pins: our own fixtures by the SHA-256 of their bytes (the repository's
//! `.gitattributes` keeps them LF on every platform). PoB's by their git
//! blob id in the pinned commit (`git rev-parse HEAD:<path>`): PoB's own
//! `.gitattributes` (`* text=auto`) checks them out with CRLF on Windows
//! and LF on Linux, so their bytes differ per platform while the blob does
//! not. The clean-checkout check ties the working file to the blob.
//!
//! Tiers: `pob-test` (PoB's own `spec/TestBuilds/3.13` builds, read from
//! the pinned checkout, never copied), `synthetic` (our own XML next to
//! the manifest), `discrepancy` (a synthetic fixture that changes one
//! input of its `pair`, for `oracle-compare`), `community` (public
//! community builds in `community/`, credited in `community/SOURCES.md`
//! under ADR-007 Amendment 1; a creator can have one removed, so nothing
//! may depend on a community fixture by name outside this manifest),
//! `mechanic` (our own small builds in `mechanic/`, one per mechanic the
//! engine supports, each naming the slice fields it `targets`; Phase 3
//! plan, decision 10; the first come with P3-6). The maintainer's corpus
//! is not listed: it stays on the workstation (ADR-007).

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::oracle::{self, FailureClass};

/// The outcome a fixture must have.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expect {
    /// Status `ok`.
    Ok,
    /// Status `partial`.
    Partial,
    /// Status `failed` with this class.
    Fails(FailureClass),
}

impl Expect {
    /// `ok`, `partial`, or a class name a single fixture can fail with.
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "ok" => Some(Self::Ok),
            "partial" => Some(Self::Partial),
            _ => FailureClass::ALL
                .into_iter()
                .filter(|c| {
                    !matches!(
                        c,
                        FailureClass::Nondeterminism
                            | FailureClass::ExpectedDifference
                            | FailureClass::UnexpectedDifference
                    )
                })
                .find(|c| c.name() == text)
                .map(Self::Fails),
        }
    }

    /// The name the manifest and the report use.
    pub fn name(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Partial => "partial",
            Self::Fails(class) => class.name(),
        }
    }

    /// Whether a fixture's status and class meet this expectation.
    pub fn met(self, status: &str, class: Option<FailureClass>) -> bool {
        match self {
            Self::Ok => status == "ok",
            Self::Partial => status == "partial",
            Self::Fails(expected) => status == "failed" && class == Some(expected),
        }
    }
}

/// `tools/oracle/fixtures/fixtures.json`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// Manifest format; 1.
    pub format: u32,
    /// The fixtures.
    pub fixtures: Vec<Entry>,
}

/// One fixture.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    /// Fixture id: the file name of `path`, unique.
    pub id: String,
    /// `pob-test`, `synthetic`, `discrepancy`, or `community`.
    pub tier: Tier,
    /// Where `path` is relative to.
    pub source: Source,
    /// Relative `/`-separated path.
    pub path: String,
    /// SHA-256 of the fixture's bytes (`local` fixtures).
    #[serde(default)]
    pub sha256: Option<String>,
    /// Git blob id in the pinned PoB commit (`pob` fixtures).
    #[serde(default)]
    pub blob: Option<String>,
    /// `ok`, `partial`, or the failure class (see [`Expect::parse`]).
    pub expect: String,
    /// For `discrepancy`: the fixture it changes one input of.
    #[serde(default)]
    pub pair: Option<String>,
    /// Free tags (`old-tree`: PoB converts the tree on load).
    #[serde(default)]
    pub tags: Vec<String>,
    /// For `mechanic`: the slice fields (`docs/phase-3/slice-fields.json`
    /// ids) the fixture isolates; `calc-parity --manifest` requires each
    /// to match or be registered (Phase 3 gate item 2).
    #[serde(default)]
    pub targets: Vec<String>,
    /// What the fixture is for.
    pub description: String,
}

/// Fixture tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Tier {
    /// PoB's own test builds, from the pinned checkout.
    PobTest,
    /// Our own fixtures.
    Synthetic,
    /// Our own fixture that changes one input of its pair.
    Discrepancy,
    /// A public community build in `community/` (ADR-007 Amendment 1).
    Community,
    /// Our own small build in `mechanic/` isolating one mechanic the
    /// engine supports, with the slice fields it targets (Phase 3 plan,
    /// decision 10).
    Mechanic,
}

/// Where a fixture's path is relative to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    /// `<oracle-dir>/pob` (the checkout `oracle-verify` checks).
    Pob,
    /// The manifest's own directory.
    Local,
}

/// The blob id of `path` in the `HEAD` commit of the checkout `pob`.
pub fn git_blob(pob: &Path, path: &str) -> Result<String, String> {
    crate::oracle_verify::git(pob, &["rev-parse", &format!("HEAD:{path}")])
}

/// A fixture resolved to a file that matches its pin.
#[derive(Debug, Clone)]
pub struct Resolved {
    /// Fixture id.
    pub id: String,
    /// The file.
    pub path: PathBuf,
    /// SHA-256 of its bytes on this machine.
    pub sha256: String,
    /// The outcome it must have.
    pub expect: Expect,
}

impl Manifest {
    /// Parses and checks a manifest.
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let manifest: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        if manifest.format != 1 {
            return Err(format!(
                "unsupported fixture manifest format {}",
                manifest.format
            ));
        }
        if manifest.fixtures.is_empty() {
            return Err("fixtures: empty".to_owned());
        }
        for (i, entry) in manifest.fixtures.iter().enumerate() {
            let at = format!("fixtures[{}] ({})", i + 1, entry.id);
            let parts: Vec<&str> = entry.path.split('/').collect();
            let plain =
                |p: &&str| !p.is_empty() && *p != "." && *p != ".." && !p.contains(['\\', ':']);
            if !parts.iter().all(plain) {
                return Err(format!(
                    "{at}: path must be relative, with / and no . or .. parts"
                ));
            }
            if parts.last() != Some(&entry.id.as_str()) {
                return Err(format!("{at}: id must be the file name of the path"));
            }
            if manifest.fixtures[..i].iter().any(|e| e.id == entry.id) {
                return Err(format!("{at}: duplicate id"));
            }
            let hex = |text: &str, len: usize| {
                text.len() == len && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
            };
            match (entry.source, &entry.sha256, &entry.blob) {
                (Source::Local, Some(sha256), None) if hex(sha256, 64) => {}
                (Source::Pob, None, Some(blob)) if hex(blob, 40) => {}
                (Source::Local, _, _) => {
                    return Err(format!(
                        "{at}: a local fixture needs sha256 (64 lowercase hex digits) and no blob"
                    ));
                }
                (Source::Pob, _, _) => {
                    return Err(format!(
                        "{at}: a pob fixture needs blob (40 lowercase hex digits) and no sha256"
                    ));
                }
            }
            if Expect::parse(&entry.expect).is_none() {
                return Err(format!(
                    "{at}: expect must be ok, partial, or a class a single fixture can fail with"
                ));
            }
            let tag = |t: &String| {
                !t.is_empty()
                    && t.bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            };
            if !entry.tags.iter().all(tag) || !entry.tags.windows(2).all(|w| w[0] < w[1]) {
                return Err(format!(
                    "{at}: tags must be lowercase-kebab, sorted, unique"
                ));
            }
            if entry.description.trim().is_empty() {
                return Err(format!("{at}: description is empty"));
            }
            match (entry.tier, &entry.pair) {
                (Tier::Discrepancy, Some(pair)) => {
                    if pair == &entry.id || !manifest.fixtures.iter().any(|e| &e.id == pair) {
                        return Err(format!("{at}: pair must name another fixture"));
                    }
                }
                (Tier::Discrepancy, None) => {
                    return Err(format!("{at}: a discrepancy fixture needs a pair"));
                }
                (_, Some(_)) => return Err(format!("{at}: only discrepancy fixtures have a pair")),
                (_, None) => {}
            }
            if (entry.tier == Tier::PobTest) != (entry.source == Source::Pob) {
                return Err(format!(
                    "{at}: pob-test fixtures, and only they, come from pob"
                ));
            }
            if (entry.tier == Tier::Community) != entry.path.starts_with("community/") {
                return Err(format!(
                    "{at}: community fixtures, and only they, live in community/"
                ));
            }
            if (entry.tier == Tier::Mechanic) != entry.path.starts_with("mechanic/") {
                return Err(format!(
                    "{at}: mechanic fixtures, and only they, live in mechanic/"
                ));
            }
            let unique = entry.targets.windows(2).all(|w| w[0] < w[1]);
            match (entry.tier, entry.targets.is_empty()) {
                (Tier::Mechanic, false) if unique => {}
                (Tier::Mechanic, _) => {
                    return Err(format!(
                        "{at}: a mechanic fixture needs targets, sorted and unique"
                    ));
                }
                (_, false) => return Err(format!("{at}: only mechanic fixtures have targets")),
                (_, true) => {}
            }
        }
        Ok(manifest)
    }

    /// The fixtures as files: `pob` paths under `<oracle_dir>/pob`, `local`
    /// ones under `local_dir` (the manifest's directory). `Err` if a file
    /// cannot be read or differs from its pin. `blob_of` returns the blob
    /// id of a path in the PoB checkout's `HEAD`.
    pub fn resolve(
        &self,
        local_dir: &Path,
        oracle_dir: &Path,
        blob_of: impl Fn(&Path, &str) -> Result<String, String>,
    ) -> Result<Vec<Resolved>, String> {
        let pob = oracle_dir.join("pob");
        self.fixtures
            .iter()
            .map(|entry| {
                let base = match entry.source {
                    Source::Pob => pob.clone(),
                    Source::Local => local_dir.to_path_buf(),
                };
                let path = entry.path.split('/').fold(base, |p, part| p.join(part));
                let bytes =
                    std::fs::read(&path).map_err(|e| format!("fixture {}: {e}", entry.id))?;
                let sha256 = oracle::sha256_hex(&bytes);
                let (found, pinned) = match (&entry.sha256, &entry.blob) {
                    (Some(pinned), _) => (sha256.clone(), pinned),
                    (None, Some(pinned)) => (
                        blob_of(&pob, &entry.path)
                            .map_err(|e| format!("fixture {}: {e}", entry.id))?,
                        pinned,
                    ),
                    (None, None) => return Err(format!("fixture {}: no pin", entry.id)),
                };
                if &found != pinned {
                    return Err(format!(
                        "fixture {}: {found} differs from the manifest's {pinned}",
                        entry.id
                    ));
                }
                Ok(Resolved {
                    id: entry.id.clone(),
                    path,
                    sha256,
                    expect: Expect::parse(&entry.expect)
                        .ok_or_else(|| format!("fixture {}: bad expect", entry.id))?,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    const COMMITTED: &[u8] = include_bytes!("../../oracle/fixtures/fixtures.json");

    fn fixtures_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../oracle/fixtures")
    }

    #[test]
    fn committed_manifest_should_parse_and_its_local_fixtures_match() {
        let manifest = Manifest::parse(COMMITTED).unwrap();
        let tiers = |t: Tier| manifest.fixtures.iter().filter(|e| e.tier == t).count();
        assert_eq!(tiers(Tier::PobTest), 5);
        assert!(tiers(Tier::Synthetic) >= 3);
        assert_eq!(tiers(Tier::Discrepancy), 1);
        assert!(tiers(Tier::Community) >= 1);
        // The first mechanic fixtures come with P3-6.
        let mechanic = tiers(Tier::Mechanic);
        // Local fixtures resolve without an oracle (their hashes are checked
        // here, in every cargo test); PoB's are checked by oracle-run.
        let local = Manifest {
            format: 1,
            fixtures: manifest
                .fixtures
                .into_iter()
                .filter(|e| e.source == Source::Local)
                .collect(),
        };
        let resolved = local
            .resolve(&fixtures_dir(), Path::new("no-oracle"), no_git)
            .unwrap();
        assert!(
            resolved
                .iter()
                .any(|r| r.expect == Expect::Fails(FailureClass::Import))
        );
        // Every local XML fixture is listed (community builds included).
        let xml_in = |dir: PathBuf| -> Vec<String> {
            std::fs::read_dir(dir)
                .unwrap()
                .filter_map(|e| e.ok()?.file_name().into_string().ok())
                .filter(|n| n.ends_with(".xml"))
                .collect()
        };
        let mut on_disk = xml_in(fixtures_dir());
        on_disk.extend(xml_in(fixtures_dir().join("community")));
        let mechanic_dir = fixtures_dir().join("mechanic");
        let mechanic_on_disk = if mechanic_dir.is_dir() {
            xml_in(mechanic_dir)
        } else {
            Vec::new()
        };
        assert_eq!(mechanic_on_disk.len(), mechanic);
        on_disk.extend(mechanic_on_disk);
        on_disk.sort();
        let mut listed: Vec<String> = resolved.iter().map(|r| r.id.clone()).collect();
        listed.sort();
        assert_eq!(listed, on_disk);
    }

    fn no_git(_: &Path, _: &str) -> Result<String, String> {
        Err("no git in this test".to_owned())
    }

    fn manifest_with(change: impl FnOnce(&mut Value)) -> Result<Manifest, String> {
        let mut value: Value = serde_json::from_slice(COMMITTED).unwrap();
        change(&mut value);
        Manifest::parse(value.to_string().as_bytes())
    }

    fn last(value: &mut Value) -> &mut Value {
        value["fixtures"]
            .as_array_mut()
            .unwrap()
            .last_mut()
            .unwrap()
    }

    /// A change to the committed manifest's JSON.
    type Change = Box<dyn FnOnce(&mut Value)>;

    #[test]
    fn parse_should_reject_bad_entries() {
        let cases: Vec<(Change, &str)> = vec![
            (Box::new(|v| last(v)["path"] = "../x.xml".into()), "path"),
            (Box::new(|v| last(v)["path"] = "C:/x.xml".into()), "path"),
            (
                Box::new(|v| last(v)["id"] = "other.xml".into()),
                "file name",
            ),
            (Box::new(|v| last(v)["sha256"] = "AB".into()), "sha256"),
            (
                Box::new(|v| last(v)["blob"] = "a".repeat(40).into()),
                "no blob",
            ),
            (
                Box::new(|v| v["fixtures"][0]["blob"] = "a".repeat(64).into()),
                "blob (40",
            ),
            (
                Box::new(|v| v["fixtures"][0]["sha256"] = "a".repeat(64).into()),
                "no sha256",
            ),
            (
                Box::new(|v| last(v)["expect"] = "ORACLE_NONDETERMINISM".into()),
                "expect",
            ),
            (
                Box::new(|v| last(v)["description"] = " ".into()),
                "description",
            ),
            (Box::new(|v| last(v)["tags"] = json!(["Old Tree"])), "tags"),
            (Box::new(|v| last(v)["tags"] = json!(["b", "a"])), "tags"),
            (Box::new(|v| last(v)["extra"] = 1.into()), "unknown field"),
            (Box::new(|v| last(v)["format"] = 1.into()), "unknown field"),
            (Box::new(|v| v["format"] = 2.into()), "format"),
            (Box::new(|v| v["fixtures"] = json!([])), "empty"),
        ];
        for (change, expected) in cases {
            let err = manifest_with(change).unwrap_err();
            assert!(err.contains(expected), "{expected}: {err}");
        }
    }

    #[test]
    fn parse_should_check_pairs_tiers_and_duplicates() {
        let discrepancy = |v: &mut Value| -> usize {
            v["fixtures"]
                .as_array()
                .unwrap()
                .iter()
                .position(|e| e["tier"] == "discrepancy")
                .unwrap()
        };
        let err = manifest_with(|v| {
            let i = discrepancy(v);
            v["fixtures"][i]["pair"] = "missing.xml".into();
        })
        .unwrap_err();
        assert!(err.contains("pair must name"), "{err}");
        let err = manifest_with(|v| {
            let i = discrepancy(v);
            v["fixtures"][i].as_object_mut().unwrap().remove("pair");
        })
        .unwrap_err();
        assert!(err.contains("needs a pair"), "{err}");
        let err =
            manifest_with(|v| v["fixtures"][0]["pair"] = "minimal-level1.xml".into()).unwrap_err();
        assert!(err.contains("only discrepancy"), "{err}");
        let err = manifest_with(|v| {
            let first = v["fixtures"][0].as_object_mut().unwrap();
            first.insert("source".to_owned(), "local".into());
            first.remove("blob");
            first.insert("sha256".to_owned(), "a".repeat(64).into());
        })
        .unwrap_err();
        assert!(err.contains("come from pob"), "{err}");
        let err = manifest_with(|v| {
            let first = v["fixtures"][0].clone();
            v["fixtures"].as_array_mut().unwrap().push(first);
        })
        .unwrap_err();
        assert!(err.contains("duplicate id"), "{err}");
    }

    #[test]
    fn parse_should_keep_community_fixtures_in_their_folder() {
        let community = |v: &mut Value| -> usize {
            v["fixtures"]
                .as_array()
                .unwrap()
                .iter()
                .position(|e| e["tier"] == "community")
                .unwrap()
        };
        // A community build outside community/.
        let err = manifest_with(|v| {
            let i = community(v);
            let id = v["fixtures"][i]["id"].clone();
            v["fixtures"][i]["path"] = id;
        })
        .unwrap_err();
        assert!(err.contains("live in community/"), "{err}");
        // Another tier inside community/.
        let err = manifest_with(|v| {
            let i = community(v);
            v["fixtures"][i]["tier"] = "synthetic".into();
        })
        .unwrap_err();
        assert!(err.contains("live in community/"), "{err}");
    }

    /// A mechanic entry appended to the committed manifest.
    fn mechanic(change: impl FnOnce(&mut Value)) -> Result<Manifest, String> {
        manifest_with(|v| {
            let mut entry = json!({
                "id": "bare-witch.xml",
                "tier": "mechanic",
                "source": "local",
                "path": "mechanic/bare-witch.xml",
                "sha256": "a".repeat(64),
                "expect": "ok",
                "targets": ["Int", "Life", "Mana"],
                "description": "a level 1 Witch without items"
            });
            change(&mut entry);
            v["fixtures"].as_array_mut().unwrap().push(entry);
        })
    }

    #[test]
    fn parse_should_take_mechanic_fixtures_with_targets() {
        let manifest = mechanic(|_| {}).unwrap();
        let last = manifest.fixtures.last().unwrap();
        assert_eq!(last.tier, Tier::Mechanic);
        assert_eq!(last.targets, ["Int", "Life", "Mana"]);
        let cases: Vec<(Change, &str)> = vec![
            (
                Box::new(|e| e["path"] = "bare-witch.xml".into()),
                "live in mechanic/",
            ),
            (
                Box::new(|e| e["tier"] = "synthetic".into()),
                "live in mechanic/",
            ),
            (Box::new(|e| e["targets"] = json!([])), "needs targets"),
            (
                Box::new(|e| e["targets"] = json!(["Life", "Int"])),
                "sorted and unique",
            ),
            (
                Box::new(|e| e["targets"] = json!(["Life", "Life"])),
                "sorted and unique",
            ),
        ];
        for (change, expected) in cases {
            let err = mechanic(change).unwrap_err();
            assert!(err.contains(expected), "{expected}: {err}");
        }
        // Targets on another tier.
        let err = manifest_with(last_targets).unwrap_err();
        assert!(err.contains("only mechanic fixtures"), "{err}");
    }

    fn last_targets(v: &mut Value) {
        last(v)["targets"] = json!(["Life"]);
    }

    #[test]
    fn resolve_should_refuse_changed_bytes() {
        let mut manifest = Manifest::parse(COMMITTED).unwrap();
        manifest.fixtures.retain(|e| e.source == Source::Local);
        manifest.fixtures[0].sha256 = Some("0".repeat(64));
        let err = manifest
            .resolve(&fixtures_dir(), Path::new("no-oracle"), no_git)
            .unwrap_err();
        assert!(err.contains("differs from the manifest"), "{err}");
    }

    #[test]
    fn resolve_should_check_pob_fixtures_by_blob() {
        let temp = std::env::temp_dir().join(format!("loom-fixtures-{}", std::process::id()));
        let file = temp.join("pob").join("spec").join("a.xml");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, "<PathOfBuilding/>\r\n").unwrap();
        let manifest = |pinned: String| Manifest {
            format: 1,
            fixtures: vec![Entry {
                id: "a.xml".to_owned(),
                tier: Tier::PobTest,
                source: Source::Pob,
                path: "spec/a.xml".to_owned(),
                sha256: None,
                blob: Some(pinned),
                expect: "ok".to_owned(),
                pair: None,
                tags: Vec::new(),
                targets: Vec::new(),
                description: "test".to_owned(),
            }],
        };
        let asked = std::cell::RefCell::new(Vec::new());
        let blob_of = |pob: &Path, path: &str| {
            asked
                .borrow_mut()
                .push((pob.to_path_buf(), path.to_owned()));
            Ok("b".repeat(40))
        };
        let resolved = manifest("b".repeat(40))
            .resolve(Path::new("x"), &temp, blob_of)
            .unwrap();
        assert_eq!(resolved[0].path, file);
        // The SHA-256 of this machine's bytes, not the blob, is reported.
        assert_eq!(
            resolved[0].sha256,
            oracle::sha256_hex(b"<PathOfBuilding/>\r\n")
        );
        assert_eq!(
            asked.borrow()[0],
            (temp.join("pob"), "spec/a.xml".to_owned())
        );
        let err = manifest("c".repeat(40))
            .resolve(Path::new("x"), &temp, blob_of)
            .unwrap_err();
        assert!(err.contains("differs from the manifest"), "{err}");
        std::fs::remove_dir_all(&temp).unwrap();
    }

    #[test]
    fn expect_should_parse_and_match_outcomes() {
        assert_eq!(Expect::parse("ok"), Some(Expect::Ok));
        assert_eq!(
            Expect::parse("ORACLE_IMPORT"),
            Some(Expect::Fails(FailureClass::Import))
        );
        assert_eq!(Expect::parse("ORACLE_UNEXPECTED_DIFFERENCE"), None);
        assert_eq!(Expect::parse("failed"), None);
        let import = Expect::Fails(FailureClass::Import);
        assert!(import.met("failed", Some(FailureClass::Import)));
        assert!(!import.met("failed", Some(FailureClass::Calculation)));
        assert!(!import.met("ok", None));
        assert!(Expect::Ok.met("ok", None));
        assert!(!Expect::Ok.met("partial", None));
        assert!(Expect::Partial.met("partial", None));
        for e in [Expect::Ok, Expect::Partial, import] {
            assert_eq!(Expect::parse(e.name()), Some(e));
        }
    }
}
