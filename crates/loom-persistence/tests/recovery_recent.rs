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
        summary: None,
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
fn a_snapshot_should_keep_what_its_build_is_and_read_without_it() {
    let dir = Scratch::new("summary");
    let store = Store::new();
    let build = Build::new(Game::Poe1);
    let with = Header {
        summary: Some(loom_persistence::recent::Summary {
            class: Some("Witch".to_owned()),
            ascendancy: Some("Necromancer".to_owned()),
            level: Some(92),
            skill: Some("Raise Spectre".to_owned()),
        }),
        ..header(7)
    };
    {
        let a = Recovery::open(&dir.0, "a-1").unwrap();
        a.write(&store, &with, &build).unwrap();
        let b = Recovery::open(&dir.0, "b-1").unwrap();
        b.write(&store, &header(8), &build).unwrap();
    }
    // Without a summary the file is as it was before the field: no key,
    // so an older application reads it, and this one reads an older file.
    let plain = std::fs::read_to_string(dir.0.join("b-1.json")).unwrap();
    assert!(!plain.contains("summary"), "{plain}");
    let c = Recovery::open(&dir.0, "c-1").unwrap();
    let found = c.list(&store);
    assert_eq!(found.len(), 2);
    assert_eq!(found[0].header, with);
    assert_eq!(found[1].header.summary, None);
    assert_eq!(c.read(&store, "a-1").unwrap().0, with);
    assert_eq!(c.read(&store, "b-1").unwrap().0, header(8));
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
        summary: None,
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

#[test]
fn recent_should_read_a_list_written_before_the_summary() {
    let dir = Scratch::new("recent-old");
    let store = Store::new();
    let file = dir.0.join("recent.json");
    std::fs::write(
        &file,
        r#"{"format":"loom-recent","version":1,"entries":[{"path":"a.json","name":"a","at":5}]}"#,
    )
    .unwrap();
    let recent = Recent::at(&file);
    let entries = recent.entries(&store);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].summary, None);
    // A summary is written and read back.
    let mut e = entries[0].clone();
    e.summary = Some(loom_persistence::recent::Summary {
        class: Some("Templar".to_owned()),
        ascendancy: Some("Inquisitor".to_owned()),
        level: Some(92),
        skill: Some("Smite".to_owned()),
    });
    recent.touch(&store, e.clone()).unwrap();
    assert_eq!(recent.entries(&store), vec![e]);
}

fn at_level(level: u32) -> Build {
    let mut build = Build::new(Game::Poe1);
    build.character.level = Some(loom_domain::Sourced::read(level, "test"));
    build
}

#[test]
fn recovery_history_should_keep_the_latest_steps_and_stop_at_a_broken_one() {
    use loom_persistence::recovery::{HISTORY_KEEP, StepRef};
    let dir = Scratch::new("history");
    let store = Store::new();
    let builds: Vec<Build> = (1..=30).map(at_level).collect();
    let entries: Vec<serde_json::Value> = (1..=30).map(|i| serde_json::json!({ "n": i })).collect();
    let steps = |range: std::ops::Range<usize>| -> Vec<StepRef<'_>> {
        range
            .map(|i| StepRef {
                id: i as u64,
                entry: &entries[i],
                build: &builds[i],
            })
            .collect()
    };
    let history = dir.0.join("first.history");
    {
        let first = Recovery::open(&dir.0, "first").unwrap();
        first.write(&store, &header(1), &builds[29]).unwrap();
        // Written in two autosaves: the second writes only its new steps
        // and drops the ones past the limit.
        first.write_history(&store, &steps(0..10)).unwrap();
        first.write_history(&store, &steps(0..29)).unwrap();
        let files = std::fs::read_dir(&history).unwrap().count();
        assert_eq!(files, HISTORY_KEEP + 1, "the kept steps and the index");
    }
    let second = Recovery::open(&dir.0, "second").unwrap();
    let read = second.read_history(&store, "first");
    assert_eq!(read.len(), HISTORY_KEEP);
    assert_eq!(read[0].build, builds[4], "oldest kept first");
    assert_eq!(read[HISTORY_KEEP - 1].build, builds[28]);
    assert_eq!(read[HISTORY_KEEP - 1].entry, entries[28]);
    // A step that does not read cuts the history there: the newer steps
    // stay, the older ones go (undo never skips a state).
    std::fs::write(history.join("20.json"), "not a document").unwrap();
    let read = second.read_history(&store, "first");
    assert_eq!(read.len(), 8);
    assert_eq!(read[0].build, builds[21]);
    // Discard removes it with the snapshot.
    second.discard("first").unwrap();
    assert!(!history.exists());
    assert!(second.read_history(&store, "first").is_empty());
}
