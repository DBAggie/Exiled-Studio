//! The passives' buffs (`loom_calc::poe1::passive_buffs`) against the real
//! bundle. Maintainer-run (the bundle is not in the repository):
//! `LOOM_BUNDLE_DIR=<bundle> cargo test -p loom-calc --test passive_buffs --
//! --ignored`. A bundle below data schema 6 carries no buffs: the test then
//! checks nothing.

#[expect(dead_code, reason = "only the real bundle's reader is used here")]
mod common;

use loom_calc::poe1::passive_buffs::{BUFF_NODES, PASSIVE_BUFFS};

/// [`BUFF_NODES`] is every node with buffs and no stats, and each such
/// node's buffs are listed in [`PASSIVE_BUFFS`] or are the reasons the
/// module names (an unlisted buff with stats).
#[test]
#[ignore = "needs the real PoE1 bundle (LOOM_BUNDLE_DIR)"]
#[expect(
    clippy::disallowed_methods,
    reason = "a maintainer-run test reads the bundle named by the environment"
)]
fn the_buff_nodes_should_be_the_bundles() {
    let dir = std::env::var("LOOM_BUNDLE_DIR").expect("LOOM_BUNDLE_DIR");
    let data = common::bundle_at(std::path::Path::new(&dir));
    if data.manifest.data_schema < loom_data::bundle::PASSIVE_BUFF_SCHEMA {
        return;
    }
    let passives = &data.passives.as_ref().expect("passives").passives;
    let mut nodes: Vec<u32> = passives
        .iter()
        .filter(|p| p.stats.is_empty() && p.buffs.iter().any(|b| !b.stats.is_empty()))
        .map(|p| u32::try_from(p.node).expect("a node number"))
        .collect();
    nodes.sort_unstable();
    assert_eq!(nodes, BUFF_NODES);
    // The four 3_29 ascendancy auras are listed.
    for node in [61355, 27096, 40059, 19641] {
        let p = passives.iter().find(|p| p.node == node).expect("the node");
        for b in &p.buffs {
            assert!(
                PASSIVE_BUFFS.iter().any(|(d, _)| *d == b.definition),
                "{}: {}",
                p.name,
                b.definition
            );
        }
    }
}
