//! Atomic save and migrating load on a scratch folder, with injected
//! failures (Phase 2 plan, step 9).

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;

use loom_domain::Game;
use serde_json::{Value, json};

use super::*;
use loom_serialization::migrate::Migration;

/// The v1 golden document of `loom-serialization`'s DTO tests.
const POE1: &str = include_str!("../../loom-serialization/src/dto/v1/testdata/poe1.json");

struct ScratchDir(PathBuf);

impl ScratchDir {
    fn new(test: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("loom-persistence-{test}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    /// Every file in the folder with its bytes.
    fn listing(&self) -> BTreeMap<String, Vec<u8>> {
        std::fs::read_dir(&self.0)
            .unwrap()
            .map(|e| {
                let e = e.unwrap();
                (
                    e.file_name().to_string_lossy().into_owned(),
                    std::fs::read(e.path()).unwrap(),
                )
            })
            .collect()
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn poe1() -> Build {
    dto::v1::from_json(POE1).unwrap()
}

/// `std::fs` with injected failures, recording every call that changes
/// the disk.
#[derive(Default)]
struct Probe {
    /// `write_new` writes the whole file, then fails (a failed flush).
    fail_after_write: bool,
    /// `rename` fails.
    fail_rename: bool,
    /// `remove` fails.
    fail_remove: bool,
    /// The next `write_new` calls that report the name as taken.
    collisions: Cell<u32>,
    /// `link_new` first writes this to the target (another program
    /// creating it while the save runs).
    race: Option<&'static str>,
    /// `link_new` fails as on a file system without hard links.
    no_links: bool,
    changes: RefCell<Vec<String>>,
}

impl Probe {
    fn changes(&self) -> Vec<String> {
        self.changes.borrow().clone()
    }

    fn record(&self, what: &str, path: &Path) {
        let name = path.file_name().unwrap().to_string_lossy();
        self.changes.borrow_mut().push(format!("{what} {name}"));
    }
}

impl FileOps for Probe {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        StdFiles.read(path)
    }

    fn exists(&self, path: &Path) -> io::Result<bool> {
        StdFiles.exists(path)
    }

    fn write_new(&self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        self.record("write", path);
        if self.collisions.get() > 0 {
            self.collisions.set(self.collisions.get() - 1);
            return Err(io::ErrorKind::AlreadyExists.into());
        }
        StdFiles.write_new(path, bytes)?;
        if self.fail_after_write {
            return Err(io::Error::other("injected: flush failed"));
        }
        Ok(())
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.record("rename", from);
        if self.fail_rename {
            return Err(io::Error::other("injected: rename failed"));
        }
        StdFiles.rename(from, to)
    }

    fn link_new(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.record("link", from);
        if let Some(bytes) = self.race {
            std::fs::write(to, bytes)?;
        }
        if self.no_links {
            return Err(io::ErrorKind::Unsupported.into());
        }
        StdFiles.link_new(from, to)
    }

    fn remove(&self, path: &Path) -> io::Result<()> {
        self.record("remove", path);
        if self.fail_remove {
            return Err(io::Error::other("injected: remove failed"));
        }
        StdFiles.remove(path)
    }
}

fn store(probe: &Probe) -> Store<&Probe> {
    Store::with(probe, Migrations::shipped())
}

fn code<T>(result: Result<T, Error>) -> &'static str {
    result.err().map_or("ok", |e| e.code())
}

const REPLACE: SaveOptions<'static> = SaveOptions {
    overwrite: Overwrite::BuildDocument,
    imported_from: None,
};

#[test]
fn a_saved_build_should_load_back_with_the_canonical_bytes() {
    let dir = ScratchDir::new("round-trip");
    let target = dir.path("build.json");
    let probe = Probe::default();
    store(&probe)
        .save(&poe1(), &target, &SaveOptions::default())
        .unwrap();
    // Only the target is left: the temporary file was linked to it and
    // removed.
    assert_eq!(
        dir.listing(),
        BTreeMap::from([("build.json".to_owned(), POE1.as_bytes().to_vec())])
    );
    let changes = probe.changes();
    assert_eq!(changes.len(), 3, "{changes:?}");
    assert!(changes[0].starts_with("write .build.json.") && changes[0].ends_with(".loom-tmp"));
    assert!(changes[1].starts_with("link .build.json."));
    assert!(changes[2].starts_with("remove .build.json."));

    let loaded = Store::new().load(&target).unwrap();
    assert_eq!(loaded.build, poe1());
    assert!(loaded.report.is_empty());
}

#[test]
fn an_empty_build_should_round_trip_through_a_file() {
    let dir = ScratchDir::new("empty");
    let target = dir.path("empty.json");
    for game in [Game::Poe1, Game::Poe2] {
        let build = Build::new(game);
        Store::new().save(&build, &target, &REPLACE).unwrap();
        assert_eq!(Store::new().load(&target).unwrap().build, build);
    }
}

#[test]
fn an_existing_target_should_be_kept_unless_replacing_is_asked() {
    let dir = ScratchDir::new("exists");
    let target = dir.path("build.json");
    std::fs::write(&target, POE1).unwrap();
    let empty = Build::new(Game::Poe2);
    let probe = Probe::default();
    assert_eq!(
        code(store(&probe).save(&empty, &target, &SaveOptions::default())),
        "PERSIST_EXISTS"
    );
    assert!(probe.changes().is_empty());
    assert_eq!(std::fs::read_to_string(&target).unwrap(), POE1);

    store(&probe).save(&empty, &target, &REPLACE).unwrap();
    assert_eq!(Store::new().load(&target).unwrap().build, empty);
    assert_eq!(dir.listing().len(), 1);
}

#[test]
fn a_target_that_is_no_build_document_should_never_be_replaced() {
    let dir = ScratchDir::new("foreign");
    let cases: [(&str, &[u8], &str); 8] = [
        (
            "pob.xml",
            b"<PathOfBuilding></PathOfBuilding>",
            "PERSIST_NOT_A_BUILD",
        ),
        (
            "other.json",
            br#"{"format": "other", "version": 1}"#,
            "PERSIST_NOT_A_BUILD",
        ),
        ("bytes.bin", b"\xFF\xFE\x00", "PERSIST_ENCODING"),
        (
            "newer.json",
            br#"{"format": "loom-build", "version": 2}"#,
            "MIGRATE_TOO_NEW",
        ),
        ("bom.json", b"\xEF\xBB\xBF{}", "PERSIST_ENCODING"),
        (
            "damaged.json",
            br#"{"format": "loom-build", "version": 1, "build": {"game": "#,
            "PERSIST_NOT_A_BUILD",
        ),
        (
            "bad-body.json",
            br#"{"format": "loom-build", "version": 1, "build": {"made_up": 1}}"#,
            "DTO_JSON",
        ),
        (
            "old.json",
            br#"{"format": "loom-build", "version": 0}"#,
            "MIGRATE_TOO_OLD",
        ),
    ];
    for (name, bytes, _) in cases {
        std::fs::write(dir.path(name), bytes).unwrap();
    }
    let before = dir.listing();
    let probe = Probe::default();
    for (name, _, expected) in cases {
        assert_eq!(
            code(store(&probe).save(&poe1(), &dir.path(name), &REPLACE)),
            expected,
            "{name}"
        );
    }
    assert!(probe.changes().is_empty(), "{:?}", probe.changes());
    assert_eq!(dir.listing(), before);
}

#[test]
fn save_should_never_target_the_import_source() {
    let dir = ScratchDir::new("import-source");
    let source = dir.path("imported.xml");
    std::fs::write(&source, "<PathOfBuilding/>").unwrap();
    std::fs::create_dir(dir.path("sub")).unwrap();
    let options = SaveOptions {
        overwrite: Overwrite::BuildDocument,
        imported_from: Some(&source),
    };
    let probe = Probe::default();
    // The same file under other spellings.
    for target in [
        source.clone(),
        dir.0.join(".").join("imported.xml"),
        dir.0.join("sub").join("..").join("imported.xml"),
    ] {
        assert_eq!(
            code(store(&probe).save(&poe1(), &target, &options)),
            "PERSIST_IMPORT_SOURCE",
            "{}",
            target.display()
        );
    }
    assert!(probe.changes().is_empty());
    assert_eq!(
        std::fs::read_to_string(&source).unwrap(),
        "<PathOfBuilding/>"
    );
    // Another file in the same folder is fine.
    store(&probe)
        .save(&poe1(), &dir.path("imported.json"), &options)
        .unwrap();
}

#[cfg(windows)]
#[test]
fn the_import_source_should_be_matched_in_any_letter_case_on_windows() {
    let dir = ScratchDir::new("import-case");
    let source = dir.path("Imported.xml");
    std::fs::write(&source, "<PathOfBuilding/>").unwrap();
    let options = SaveOptions {
        overwrite: Overwrite::BuildDocument,
        imported_from: Some(&source),
    };
    assert_eq!(
        code(Store::new().save(&poe1(), &dir.path("IMPORTED.XML"), &options)),
        "PERSIST_IMPORT_SOURCE"
    );
}

/// The plan's injected-writer check: the temporary file is fully written,
/// then the write reports a failure. The target keeps its old bytes (or
/// stays absent) and the temporary file is removed.
#[test]
fn a_write_failing_after_the_temporary_file_is_written_should_leave_the_target() {
    let dir = ScratchDir::new("write-fails");
    let existing = dir.path("existing.json");
    std::fs::write(&existing, POE1).unwrap();
    let before = dir.listing();
    let probe = Probe {
        fail_after_write: true,
        ..Probe::default()
    };
    let empty = Build::new(Game::Poe1);
    for target in [existing, dir.path("new.json")] {
        let err = store(&probe).save(&empty, &target, &REPLACE).unwrap_err();
        assert_eq!(err.code(), "PERSIST_WRITE");
        assert!(matches!(
            err,
            Error::Persist(PersistError::Write {
                leftover: false,
                ..
            })
        ));
        assert_eq!(dir.listing(), before, "{}", target.display());
    }
    let changes = probe.changes();
    assert!(
        changes.iter().all(|c| !c.starts_with("rename")),
        "{changes:?}"
    );
    assert_eq!(
        changes.iter().filter(|c| c.starts_with("remove")).count(),
        2
    );
}

#[test]
fn a_failing_rename_should_leave_the_target_and_remove_the_temporary_file() {
    let dir = ScratchDir::new("rename-fails");
    let target = dir.path("build.json");
    std::fs::write(&target, POE1).unwrap();
    let before = dir.listing();
    let probe = Probe {
        fail_rename: true,
        ..Probe::default()
    };
    let err = store(&probe)
        .save(&Build::new(Game::Poe2), &target, &REPLACE)
        .unwrap_err();
    assert_eq!(err.code(), "PERSIST_RENAME");
    assert!(matches!(
        err,
        Error::Persist(PersistError::Rename {
            leftover: false,
            ..
        })
    ));
    assert_eq!(dir.listing(), before);
}

#[test]
fn a_temporary_file_that_cannot_be_removed_should_be_reported() {
    let dir = ScratchDir::new("leftover");
    let target = dir.path("build.json");
    std::fs::write(&target, POE1).unwrap();
    let probe = Probe {
        fail_rename: true,
        fail_remove: true,
        ..Probe::default()
    };
    let empty = Build::new(Game::Poe2);
    let err = store(&probe).save(&empty, &target, &REPLACE).unwrap_err();
    assert!(matches!(
        err,
        Error::Persist(PersistError::Rename { leftover: true, .. })
    ));
    assert!(err.to_string().contains("could not be removed"), "{err}");
    // The target keeps its bytes; the leftover is the temporary file.
    let listing = dir.listing();
    assert_eq!(listing.len(), 2);
    assert_eq!(listing["build.json"], POE1.as_bytes());
    let (name, bytes) = listing.iter().find(|(n, _)| *n != "build.json").unwrap();
    assert!(name.ends_with(".loom-tmp"), "{name}");
    assert_eq!(bytes, dto::write(&empty).unwrap().as_bytes());
}

#[test]
fn a_taken_temporary_name_should_be_skipped() {
    let dir = ScratchDir::new("collision");
    let target = dir.path("build.json");
    let probe = Probe {
        collisions: Cell::new(2),
        ..Probe::default()
    };
    store(&probe).save(&poe1(), &target, &REPLACE).unwrap();
    let changes = probe.changes();
    // Three names tried, then link and remove.
    assert_eq!(changes.len(), 5, "{changes:?}");
    assert_eq!(dir.listing().len(), 1);
}

#[test]
fn names_all_taken_should_give_persist_write() {
    let dir = ScratchDir::new("all-taken");
    let probe = Probe {
        collisions: Cell::new(u32::MAX),
        ..Probe::default()
    };
    let err = store(&probe)
        .save(&poe1(), &dir.path("build.json"), &REPLACE)
        .unwrap_err();
    assert!(matches!(
        err,
        Error::Persist(PersistError::Write {
            leftover: false,
            ..
        })
    ));
    assert_eq!(probe.changes().len(), 100);
    assert!(dir.listing().is_empty());
}

#[test]
fn a_missing_folder_should_give_persist_write() {
    let dir = ScratchDir::new("no-folder");
    let err = Store::new()
        .save(&poe1(), &dir.path("missing").join("build.json"), &REPLACE)
        .unwrap_err();
    assert_eq!(err.code(), "PERSIST_WRITE");
    assert!(dir.listing().is_empty());
}

/// Another program creates the target while the save runs: the link
/// refuses it, so the other file is kept (`Overwrite::Never` is atomic).
#[test]
fn a_target_appearing_during_the_save_should_be_kept() {
    let dir = ScratchDir::new("race");
    let target = dir.path("build.json");
    for overwrite in [Overwrite::Never, Overwrite::BuildDocument] {
        let _ = std::fs::remove_file(&target);
        let probe = Probe {
            race: Some("theirs"),
            ..Probe::default()
        };
        let options = SaveOptions {
            overwrite,
            imported_from: None,
        };
        assert_eq!(
            code(store(&probe).save(&poe1(), &target, &options)),
            "PERSIST_EXISTS"
        );
        assert_eq!(
            dir.listing(),
            BTreeMap::from([("build.json".to_owned(), b"theirs".to_vec())])
        );
    }
}

/// Without hard links the save checks again and renames.
#[test]
fn a_file_system_without_hard_links_should_fall_back_to_rename() {
    let dir = ScratchDir::new("no-links");
    let target = dir.path("build.json");
    let probe = Probe {
        no_links: true,
        ..Probe::default()
    };
    store(&probe)
        .save(&poe1(), &target, &SaveOptions::default())
        .unwrap();
    assert_eq!(std::fs::read_to_string(&target).unwrap(), POE1);
    assert_eq!(dir.listing().len(), 1);
    assert!(
        probe.changes()[2].starts_with("rename "),
        "{:?}",
        probe.changes()
    );
    // The second check still sees a target that appeared.
    std::fs::remove_file(&target).unwrap();
    let probe = Probe {
        no_links: true,
        race: Some("theirs"),
        ..Probe::default()
    };
    assert_eq!(
        code(store(&probe).save(&poe1(), &target, &SaveOptions::default())),
        "PERSIST_EXISTS"
    );
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "theirs");
    assert_eq!(dir.listing().len(), 1);
}

#[test]
fn a_target_without_a_file_name_should_give_persist_path() {
    let probe = Probe::default();
    for target in ["", "..", "/"] {
        assert_eq!(
            code(store(&probe).save(&poe1(), Path::new(target), &REPLACE)),
            "PERSIST_PATH",
            "{target:?}"
        );
    }
    assert!(probe.changes().is_empty());
}

/// The synthetic v0 of `loom-serialization`'s migration tests: the data
/// bundle's Steam build was called `build_id`.
fn v0_to_v1(document: &mut Value, changes: &mut Vec<String>) -> Result<(), String> {
    let data = document
        .pointer_mut("/build/data")
        .and_then(Value::as_object_mut)
        .ok_or("no build.data")?;
    let id = data.remove("build_id").ok_or("no build.data.build_id")?;
    data.insert("steam_build".to_owned(), id);
    changes.push("build.data.build_id renamed to steam_build".to_owned());
    Ok(())
}

fn v0_json() -> String {
    let mut value: Value = serde_json::from_str(POE1).unwrap();
    value["version"] = json!(0);
    let data = value["build"]["data"].as_object_mut().unwrap();
    let id = data.remove("steam_build").unwrap();
    data.insert("build_id".to_owned(), id);
    serde_json::to_string_pretty(&value).unwrap()
}

#[test]
fn an_older_file_should_load_migrated_with_its_report_and_stay_unchanged() {
    let dir = ScratchDir::new("migrate");
    let path = dir.path("old.json");
    std::fs::write(&path, v0_json()).unwrap();
    let before = dir.listing();
    let probe = Probe::default();
    let chain = Migrations::new(
        vec![Migration {
            from: 0,
            apply: v0_to_v1,
        }],
        1,
    )
    .unwrap();
    let loaded = Store::with(&probe, chain.clone()).load(&path).unwrap();
    assert_eq!(loaded.build, poe1());
    assert_eq!((loaded.report.from, loaded.report.to), (0, 1));
    assert_eq!(loaded.report.steps.len(), 1);
    assert!(probe.changes().is_empty());
    assert_eq!(dir.listing(), before);

    // Saving the migrated build over its v0 file is allowed and writes v1.
    Store::with(&probe, chain)
        .save(&loaded.build, &path, &REPLACE)
        .unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), POE1);
    // Without the step, the v0 file is refused.
    std::fs::write(&path, v0_json()).unwrap();
    assert_eq!(code(Store::new().load(&path)), "MIGRATE_TOO_OLD");
}

/// The plan's damaged-file check: each damage gives its layer's code, and
/// a load writes nothing.
#[test]
fn a_damaged_file_should_give_its_code_and_write_nothing() {
    let dir = ScratchDir::new("damaged");
    let with_bom = [b"\xEF\xBB\xBF".as_slice(), POE1.as_bytes()].concat();
    let cases: [(&str, &[u8], &str); 7] = [
        (
            "truncated.json",
            &POE1.as_bytes()[..POE1.len() / 2],
            "DTO_JSON",
        ),
        ("empty.json", b"", "DTO_JSON"),
        (
            "latin1.json",
            b"{\"format\": \"loom-build\xE9\"}",
            "PERSIST_ENCODING",
        ),
        ("bom.json", &with_bom, "PERSIST_ENCODING"),
        (
            "newer.json",
            br#"{"format": "loom-build", "version": 9, "x": 1}"#,
            "MIGRATE_TOO_NEW",
        ),
        (
            "other.json",
            br#"{"format": "other", "version": 1}"#,
            "DTO_FORMAT",
        ),
        ("pob.xml", b"<PathOfBuilding/>", "DTO_JSON"),
    ];
    for (name, bytes, _) in cases {
        std::fs::write(dir.path(name), bytes).unwrap();
    }
    let before = dir.listing();
    let probe = Probe::default();
    for (name, _, expected) in cases {
        assert_eq!(
            code(store(&probe).load(&dir.path(name))),
            expected,
            "{name}"
        );
    }
    assert_eq!(
        code(store(&probe).load(&dir.path("missing.json"))),
        "PERSIST_READ"
    );
    assert_eq!(code(store(&probe).load(&dir.0)), "PERSIST_READ");
    assert!(probe.changes().is_empty());
    assert_eq!(dir.listing(), before);
}
