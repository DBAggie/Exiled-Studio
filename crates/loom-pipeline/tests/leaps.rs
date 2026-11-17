//! [`loom_pipeline::leaps`] (passive tree plan, T-f): which socketed jewels
//! let nodes in their radius be allocated without a path, read from their
//! selected, resolved lines' stats, on the synthetic tree bundle
//! (`crates/loom-data/tests/fixtures/make_bundles.py`: jewel socket 11028,
//! keystone 11040, radius rows Small and Medium). The lines are given their
//! resolutions here: the synthetic stat descriptions have no leap text.

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
use loom_domain::{
    Build, ItemsEntry, LineKind, LineResolution, Resolved, StatId, StatLine, StatValue,
};
use loom_tree::Tree;
use loom_tree::poe1::radius::{Centre, Leap, Ring};

const SOCKET: u32 = 11028;
const KEYSTONE: u32 = 11040;
const LEAP: &str = "local_unique_jewel_nearby_disconnected_passives_can_be_allocated";
const LEAP_KEYSTONES: &str =
    "local_unique_jewel_nearby_disconnected_keystone_passives_can_be_allocated";
const ESCAPE: &str =
    "local_unique_jewel_disconnected_passives_can_be_allocated_around_keystone_hash";
const RING: &str = "local_jewel_variable_ring_radius_value";

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

fn tree() -> Tree {
    let dir =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../loom-data/tests/fixtures/bundle-poe1-tree");
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
    Tree::from_bundle(&Bundle::load(&src).unwrap()).unwrap()
}

/// A build with a jewel of `lines` (after its `Radius:` line, when given)
/// in `socket`; each modifier line then takes the resolution `resolve`
/// gives its index among the modifier lines.
fn build(
    socket: u32,
    radius: Option<&str>,
    lines: &[&str],
    resolve: impl Fn(usize) -> (bool, LineResolution),
) -> Build {
    let radius = radius.map_or_else(String::new, |r| format!("Radius: {r}\n"));
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<PathOfBuilding>
	<Build level="1" targetVersion="3_0" className="Test Brute" ascendClassName="None" mainSocketGroup="1" bandit="None" pantheonMajorGod="None" pantheonMinorGod="None"/>
	<Tree activeSpec="1">
		<Spec treeVersion="3_29" classId="1" ascendClassId="0" nodes="1000">
			<Sockets>
				<Socket nodeId="{socket}" itemId="1"/>
			</Sockets>
		</Spec>
	</Tree>
	<Skills/>
	<Items activeItemSet="1" useSecondWeaponSet="false">
		<Item id="1">
Rarity: UNIQUE
Loom Test Leap
Cobalt Jewel
{radius}Implicits: 0
{lines}
		</Item>
		<ItemSet id="1" useSecondWeaponSet="false"/>
	</Items>
</PathOfBuilding>
"#,
        lines = lines.join("\n"),
    );
    let mut build = loom_pipeline::read_pob(&xml).unwrap();
    let mut index = 0;
    for entry in build.items.entries.typed_mut() {
        let ItemsEntry::Item(item) = entry else {
            continue;
        };
        for line in &mut item.body.lines {
            if let LineKind::Modifier(m) = &mut line.kind {
                (m.selected, m.resolution) = resolve(index);
                index += 1;
            }
        }
    }
    assert_eq!(index, lines.len(), "every line is a modifier line");
    build
}

fn stats(stats: &[(&str, StatValue)]) -> LineResolution {
    LineResolution::Resolved {
        result: Resolved::Stats(
            stats
                .iter()
                .map(|(s, v)| StatLine {
                    stat: StatId::new(*s),
                    value: v.clone(),
                })
                .collect(),
        ),
        adapted: None,
    }
}

const fn one(v: i64) -> StatValue {
    StatValue::Exact { min: v, max: v }
}

fn leaps_of(tree: &Tree, build: &Build) -> Vec<Leap> {
    let spec = loom_calc::poe1::collect::active_spec(build).unwrap();
    loom_pipeline::leaps(build, spec, tree)
}

/// The Small row's radius (800) and the Medium row's ring (1100 to 1400),
/// in tree units (times 1.2).
const SMALL: Ring = Ring {
    inner: 0,
    outer: 960,
};
const MEDIUM_RING: Ring = Ring {
    inner: 1320,
    outer: 1680,
};

#[test]
fn a_leap_line_should_give_a_leap_around_its_socket_in_the_named_radius() {
    let tree = tree();
    let b = build(SOCKET, Some("Small"), &["leap"], |_| {
        (true, stats(&[(LEAP, one(1))]))
    });
    assert_eq!(
        leaps_of(&tree, &b),
        [Leap {
            socket: SOCKET,
            ring: SMALL,
            centre: Centre::Socket,
            keystones_only: false,
        }]
    );
}

#[test]
fn the_keystone_only_line_should_give_a_keystone_only_leap_unless_a_full_leap_is_there() {
    let tree = tree();
    let only = build(SOCKET, Some("Small"), &["keystones"], |_| {
        (true, stats(&[(LEAP_KEYSTONES, one(1))]))
    });
    assert_eq!(
        leaps_of(&tree, &only)
            .iter()
            .map(|l| l.keystones_only)
            .collect::<Vec<_>>(),
        [true]
    );
    let both = build(SOCKET, Some("Small"), &["leap", "keystones"], |i| {
        (true, stats(&[([LEAP, LEAP_KEYSTONES][i], one(1))]))
    });
    assert_eq!(
        leaps_of(&tree, &both)
            .iter()
            .map(|l| l.keystones_only)
            .collect::<Vec<_>>(),
        [false]
    );
}

#[test]
fn the_ring_stat_should_take_its_table_row_over_the_radius_line() {
    let tree = tree();
    let b = build(SOCKET, Some("Small"), &["leap", "ring"], |i| {
        (true, stats(&[[(LEAP, one(1)), (RING, one(2))][i].clone()]))
    });
    assert_eq!(
        leaps_of(&tree, &b)
            .iter()
            .map(|l| l.ring)
            .collect::<Vec<_>>(),
        [MEDIUM_RING]
    );
}

#[test]
fn an_escape_line_should_give_a_leap_around_each_keystone_it_names() {
    let tree = tree();
    let b = build(SOCKET, Some("Small"), &["escape"], |_| {
        (
            true,
            stats(&[(ESCAPE, StatValue::Lookup(vec![KEYSTONE.into(), -1]))]),
        )
    });
    assert_eq!(
        leaps_of(&tree, &b),
        [Leap {
            socket: SOCKET,
            ring: SMALL,
            centre: Centre::Keystone(KEYSTONE),
            keystones_only: false,
        }]
    );
}

#[test]
fn nothing_settled_should_give_no_leap() {
    let tree = tree();
    let leap = |_| (true, stats(&[(LEAP, one(1))]));
    // No radius line and no ring: no radius, so no leap (never a guess).
    assert!(leaps_of(&tree, &build(SOCKET, None, &["leap"], leap)).is_empty());
    // A radius name the table does not have.
    assert!(leaps_of(&tree, &build(SOCKET, Some("Huge"), &["leap"], leap)).is_empty());
    // A ring row the table does not have, and no radius line.
    let no_row = build(SOCKET, None, &["leap", "ring"], |i| {
        (true, stats(&[[(LEAP, one(1)), (RING, one(9))][i].clone()]))
    });
    assert!(leaps_of(&tree, &no_row).is_empty());
    // A line not selected (a variant the item does not use).
    let unselected = build(SOCKET, Some("Small"), &["leap"], |_| {
        (false, stats(&[(LEAP, one(1))]))
    });
    assert!(leaps_of(&tree, &unselected).is_empty());
    // A line that did not resolve (PoB's older wording) gives nothing.
    let pending = build(SOCKET, Some("Small"), &["leap"], |_| {
        (true, LineResolution::Pending)
    });
    assert!(leaps_of(&tree, &pending).is_empty());
    // A ranged value does not settle the escape's keystone.
    let ranged = build(SOCKET, Some("Small"), &["escape"], |_| {
        (
            true,
            stats(&[(
                ESCAPE,
                StatValue::Ranged {
                    min: i64::from(KEYSTONE),
                    max: i64::from(KEYSTONE) + 1,
                },
            )]),
        )
    });
    assert!(leaps_of(&tree, &ranged).is_empty());
}
