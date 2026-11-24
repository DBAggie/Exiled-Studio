//! The Abyss jewels' reader (`loom_data::bundle::timeless::abyss`, P6-10a)
//! on the shipped results files: PoB's own example (Kurgal seed 533 at
//! socket 61419, `spec/System/TestAbyssTimelessJewel_spec.lua`) is the
//! real file's record, and Zorath's walk to its ascendancy section takes
//! about 30 ms in a release build. Maintainer-run (the bundle is not in the
//! repository): `LOOM_BUNDLE_DIR=<bundle> cargo test -p loom-data --test
//! timeless_abyss_files -- --ignored --nocapture`.

#![expect(
    clippy::disallowed_methods,
    reason = "a maintainer-run test reads the bundle named by the environment"
)]
#![expect(
    clippy::disallowed_types,
    reason = "a maintainer-run test times the reader; never part of a core build"
)]
#![expect(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "a maintainer-run test over fixed files; a failure should panic"
)]

use std::collections::BTreeSet;
use std::path::PathBuf;

use loom_data::bundle::timeless::abyss::{AbyssPart, read_path, read_socket};
use loom_data::bundle::timeless::{TIMELESS_LUTS, TimelessLuts};

fn dir() -> PathBuf {
    PathBuf::from(std::env::var("LOOM_BUNDLE_DIR").expect("LOOM_BUNDLE_DIR"))
}

fn luts() -> TimelessLuts {
    serde_json::from_slice(&std::fs::read(dir().join(TIMELESS_LUTS)).unwrap()).unwrap()
}

#[test]
#[ignore = "needs a bundle with the timeless results (LOOM_BUNDLE_DIR)"]
fn kurgal_533_at_61419_should_be_pobs_example() {
    let luts = luts();
    let jewel = luts.jewel(9).expect("Kurgal");
    let bytes = std::fs::read(dir().join(&jewel.file)).unwrap();
    let got = read_socket(&luts, jewel, &bytes, 533, 61419);
    let part = |kind, global, rolls: &[i64]| AbyssPart {
        kind,
        global,
        rolls: rolls.to_vec(),
    };
    assert_eq!(got.get(&15117), Some(&vec![part(1, 622, &[8])]));
    assert_eq!(got.get(&4367), Some(&vec![part(1, 627, &[6])]));
    assert_eq!(got.get(&20528), Some(&vec![part(2, 207, &[-15])]));
    assert_eq!(got.get(&21958), Some(&vec![part(2, 211, &[12, 18])]));
    assert!(!got.contains_key(&53279));
}

#[test]
#[ignore = "needs a bundle with the timeless results (LOOM_BUNDLE_DIR)"]
fn zorath_should_read_a_path_and_every_ascendancy_quickly() {
    let luts = luts();
    let jewel = luts.jewel(11).expect("Zorath");
    let bytes = std::fs::read(dir().join(&jewel.file)).unwrap();
    let path: BTreeSet<u32> = [8948, 21678, 27659, 32210, 37671, 48778, 57226].into();
    let start = std::time::Instant::now();
    let got = read_path(&luts, jewel, &bytes, 7300, &path);
    let took = start.elapsed();
    println!("Zorath read_path: {took:?}, {} nodes", got.len());
    // The fixture `timeless-abyss-zorath-ice-nova-witch.xml`: PoB changes
    // these seven path nodes; each of the 21 ascendancies adds its pick.
    for node in &path {
        assert!(got.contains_key(node), "{node}");
    }
    assert!(got.len() > path.len());
    // About 30 ms in a release build; an unoptimised one is ~30 times slower.
    let bound = if cfg!(debug_assertions) { 10.0 } else { 0.5 };
    assert!(took.as_secs_f64() < bound, "{took:?}");
}
