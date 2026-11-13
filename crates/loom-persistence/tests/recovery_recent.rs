//! Recovery snapshots and the recent list (Phase 4 plan, decision 8) on a
//! scratch folder.

#![expect(clippy::unwrap_used, reason = "tests; a failure should panic")]

use std::path::PathBuf;

use loom_domain::{Build, Game};
use loom_persistence::Store;
use loom_persistence::recent::{Entry, Recent};
use loom_persistence::recovery::{Header, Recovery, valid_id};

struct Scratch(PathBuf);

impl Scratch {
    fn new(test: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("loom-persistence-it-{test}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn header(at: u64) -> Header {
    Header {
        saved_at: at,
        name: "Test".to_owned(),
        path: None,
        origin: Some("C:/builds/test.xml".to_owned()),
    }
}

#[test]
fn recovery_should_offer_an_ended_instances_snapshot_and_restore_it() {
    let dir = Scratch::new("restore");
    let store = Store::new();
    let build = Build::new(Game::Poe1);
    {
        let first = Recovery::open(&dir.0, "first-1").unwrap();
        first.write(&store, &header(5), &build).unwrap();
        // While the first instance runs, a second one sees nothing.
        let second = Recovery::open(&dir.0, "second-1").unwrap();
        assert!(second.list(&store).is_empty());
        assert!(second.read(&store, "first-1").is_err());
        assert!(second.discard("first-1").is_err());
    }
    // The first instance ended without clearing: its snapshot is offered.
    let third = Recovery::open(&dir.0, "third-1").unwrap();
    let found = third.list(&store);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].id, "first-1");
    assert_eq!(found[0].header, header(5));
    let (h, restored) = third.read(&store, "first-1").unwrap();
    assert_eq!(h, header(5));
    assert_eq!(restored, build);
    third.discard("first-1").unwrap();
    assert!(third.list(&store).is_empty());
}

#[test]
fn recovery_clear_should_leave_nothing_to_offer() {
    let dir = Scratch::new("clear");
    let store = Store::new();
    {
        let first = Recovery::open(&dir.0, "a").unwrap();
        first
            .write(&store, &header(1), &Build::new(Game::Poe1))
            .unwrap();
        first.clear();
    }
    let second = Recovery::open(&dir.0, "b").unwrap();
    assert!(second.list(&store).is_empty());
}

#[test]
fn recovery_should_refuse_ids_that_could_leave_the_folder() {
    assert!(valid_id("abc-123"));
    for bad in ["", "../x", "a/b", "a\\b", "a.json", &"x".repeat(65)] {
        assert!(!valid_id(bad), "{bad}");
    }
    let dir = Scratch::new("ids");
    let store = Store::new();
    let rec = Recovery::open(&dir.0, "me").unwrap();
    assert_eq!(
        rec.read(&store, "../escape").unwrap_err().code(),
        "PERSIST_PATH"
    );
    assert!(Recovery::open(&dir.0, "../x").is_err());
}

#[test]
fn recovery_should_skip_files_that_are_not_snapshots() {
    let dir = Scratch::new("junk");
    let store = Store::new();
    std::fs::write(dir.0.join("junk.json"), "{\"format\":\"other\"}").unwrap();
    let rec = Recovery::open(&dir.0, "me").unwrap();
    assert!(rec.list(&store).is_empty());
}

#[test]
fn recent_should_keep_newest_first_without_duplicates_and_find_by_id() {
    let dir = Scratch::new("recent");
    let store = Store::new();
    let recent = Recent::at(&dir.0.join("sub").join("recent.json"));
    assert!(recent.entries(&store).is_empty());
    let entry = |path: &str, at| Entry {
        path: path.to_owned(),
        name: path.to_owned(),
        at,
    };
    for i in 0..12 {
        recent
            .touch(&store, entry(&format!("b{i}.json"), i))
            .unwrap();
    }
    recent.touch(&store, entry("b5.json", 99)).unwrap();
    let all = recent.entries(&store);
    assert_eq!(all.len(), 10);
    assert_eq!(all[0].path, "b5.json");
    assert_eq!(all.iter().filter(|e| e.path == "b5.json").count(), 1);
    let id = all[1].id();
    assert_eq!(recent.find(&store, &id).unwrap().path, all[1].path);
    recent.remove(&store, &id).unwrap();
    assert!(recent.find(&store, &id).is_none());
    assert!(recent.find(&store, "../../etc/passwd").is_none());
}

#[test]
fn recent_should_read_as_empty_when_the_cache_is_broken() {
    let dir = Scratch::new("broken");
    let store = Store::new();
    let file = dir.0.join("recent.json");
    std::fs::write(&file, "not json").unwrap();
    assert!(Recent::at(&file).entries(&store).is_empty());
}
