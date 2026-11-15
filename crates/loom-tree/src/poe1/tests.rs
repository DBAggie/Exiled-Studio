//! The rules on a small tree (a test per command, T-g's kept nodes in each).
//!
//! ```text
//!        50(Asc1 start) - 51 - 52 (choice)
//!        |                  \ 53 (choice)
//! 1(A) - 10 - 11 - 12 - 13 - 30 - 2(B)
//!   \  - 20 - 21 /         |
//!   60(Asc2 start) - 61    mastery 40 (linked from 12); mastery 41 (linked from 21)
//! 55: an Asc1 node linked to 30 (the Ascendant's "Path of the ..." kind).
//! 70: a bloodline node (not modelled); 71: a node the tree lacks.
//! 80: a jewel socket linked to 13; 81: a nested expansion socket in a
//! group with 82, a proxy linked to 80; 83: a proxy alone, linked to 81.
//! ```

use std::collections::{BTreeMap, BTreeSet};

use loom_data::bundle::passives::PassiveTables;
use serde_json::json;

use super::*;
use crate::graph::Tree;

fn passive(node: u32, extra: &serde_json::Value) -> serde_json::Value {
    let mut v = json!({"id": format!("p{node}"), "node": node, "name": format!("N{node}")});
    if let (Some(o), Some(e)) = (v.as_object_mut(), extra.as_object()) {
        o.extend(e.clone());
    }
    v
}

fn node(id: u32, to: &[u32]) -> serde_json::Value {
    json!({"node": id, "orbit": 0, "orbit_index": 0,
        "connections": to.iter().map(|n| json!({"node": n})).collect::<Vec<_>>()})
}

fn de<T: serde::de::DeserializeOwned>(v: serde_json::Value) -> T {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("fixture: {e}"))
}

fn tree() -> Tree {
    let chars = |id: &str, n: u32| {
        json!({"id": id, "name": id, "class_number": n, "base_life": 0, "base_mana": 0,
            "strength": 0, "dexterity": 0, "intelligence": 0})
    };
    let asc = |id: &str, ch: &str, n: u32| json!({"id": id, "name": id, "character": ch, "class_number": n});
    let tables = PassiveTables {
        characters: de(json!([chars("A", 1), chars("B", 2)])),
        ascendancies: de(json!([asc("Asc1", "A", 1), asc("Asc2", "A", 2)])),
        object_templates: Vec::new(),
        passives: de(json!([
            passive(1, &json!({"characters": ["A"]})),
            passive(2, &json!({"characters": ["B"]})),
            passive(10, &json!({})),
            passive(11, &json!({})),
            passive(12, &json!({"notable": true})),
            passive(13, &json!({})),
            passive(20, &json!({})),
            passive(21, &json!({})),
            passive(30, &json!({})),
            passive(40, &json!({"mastery_group": "life"})),
            passive(41, &json!({"mastery_group": "life"})),
            passive(50, &json!({"ascendancy": "Asc1", "ascendancy_start": true})),
            passive(51, &json!({"ascendancy": "Asc1", "notable": true})),
            passive(
                52,
                &json!({"ascendancy": "Asc1", "multiple_choice_option": true})
            ),
            passive(
                53,
                &json!({"ascendancy": "Asc1", "multiple_choice_option": true})
            ),
            passive(55, &json!({"ascendancy": "Asc1"})),
            passive(60, &json!({"ascendancy": "Asc2", "ascendancy_start": true})),
            passive(61, &json!({"ascendancy": "Asc2"})),
            passive(70, &json!({"ascendancy": "Blood"})),
            passive(80, &json!({"jewel_socket": true})),
            passive(81, &json!({"jewel_socket": true, "expansion": true})),
            passive(82, &json!({})),
            passive(83, &json!({})),
        ])),
        mastery_groups: de(json!([{"id": "life", "effects": ["e100", "e101"]}])),
        mastery_effects: de(json!([{"id": "e100", "hash": 100}, {"id": "e101", "hash": 101}])),
        tree: de(json!({
            "tree": "Default", "graph": "g", "orbit_slots": [1], "roots": [],
            "groups": [
                {"x": 0, "y": 0, "nodes": [
                    node(1, &[10, 20, 50, 60]), node(2, &[30]), node(10, &[11]),
                    node(11, &[12]), node(20, &[21]), node(30, &[13])]},
                {"x": 0, "y": 0, "nodes": [node(12, &[13, 21, 40]), node(13, &[]), node(40, &[])]},
                {"x": 0, "y": 0, "nodes": [node(21, &[41]), node(41, &[])]},
                {"x": 0, "y": 0, "nodes": [
                    node(50, &[51, 55]), node(51, &[52, 53]), node(55, &[30]), node(52, &[]), node(53, &[]),
                    node(60, &[61]), node(61, &[]), node(70, &[])]},
                {"x": 0, "y": 0, "nodes": [node(80, &[13])]},
                {"x": 0, "y": 0, "nodes": [node(81, &[82]), node(82, &[80])]},
                {"x": 0, "y": 0, "nodes": [node(83, &[81])]},
            ]
        })),
    };
    Tree::from_tables(&tables, None)
}

fn spec(class: u32, ascendancy: u32, nodes: &[u32]) -> Spec {
    Spec {
        class,
        ascendancy,
        nodes: nodes.iter().copied().collect(),
        masteries: BTreeMap::new(),
        leaps: Vec::new(),
    }
}

fn set(nodes: &[u32]) -> BTreeSet<u32> {
    nodes.iter().copied().collect()
}

#[test]
fn the_tree_should_mark_kinds_classes_and_what_it_models() {
    let t = tree();
    assert_eq!(t.classes.len(), 2);
    assert_eq!(t.class(1).map(|c| c.start), Some(1));
    let asc: Vec<_> = t.classes[0]
        .ascendancies
        .iter()
        .map(|a| (a.number, a.start))
        .collect();
    assert_eq!(asc, [(1, Some(50)), (2, Some(60))]);
    assert_eq!(t.nodes[&1].kind, Kind::ClassStart);
    assert_eq!(t.nodes[&50].kind, Kind::AscendancyStart);
    assert_eq!(t.nodes[&40].kind, Kind::Mastery);
    assert_eq!(t.nodes[&12].kind, Kind::Notable);
    assert!(t.nodes[&40].modelled && t.nodes[&41].modelled && t.nodes[&61].modelled);
    assert!(
        !t.nodes[&70].modelled,
        "a bloodline node is kept, not modelled"
    );
    assert_eq!(
        t.nodes[&13].neighbours,
        [12, 30, 80],
        "links are undirected"
    );
    assert_eq!(t.mastery_effects["life"], [100, 101]);
}

#[test]
fn allocate_should_take_the_lexicographically_smallest_shortest_path_and_keep_kept_nodes() {
    let t = tree();
    let s = spec(1, 0, &[1, 70, 71]);
    let a = allocate(&t, &s, 12).expect("reachable");
    assert_eq!(a.path, [10, 11, 12], "10-11-12 before 20-21-12");
    assert_eq!(a.spec.nodes, set(&[1, 10, 11, 12, 70, 71]));
    let b = allocate(&t, &a.spec, 13).expect("reachable");
    assert_eq!(b.path, [13]);
    let c = allocate(&t, &spec(1, 0, &[1, 20]), 13).expect("reachable");
    assert_eq!(c.path, [21, 12, 13], "from the nearest allocated node");
}

#[test]
fn allocate_should_refuse_starts_masteries_kept_and_other_ascendancies() {
    let t = tree();
    let s = spec(1, 1, &[1, 50]);
    assert_eq!(allocate(&t, &s, 2), Err(EditError::Start(2)));
    assert_eq!(allocate(&t, &s, 40), Err(EditError::Mastery(40)));
    assert_eq!(allocate(&t, &s, 70), Err(EditError::NotModelled(70)));
    assert_eq!(allocate(&t, &s, 99), Err(EditError::UnknownNode(99)));
    assert_eq!(allocate(&t, &s, 1), Err(EditError::Start(1)));
    assert_eq!(allocate(&t, &s, 61), Err(EditError::Unreachable(61)));
    let a = allocate(&t, &s, 51).expect("our ascendancy");
    assert_eq!(a.path, [51]);
    assert_eq!(
        allocate(&t, &spec(1, 0, &[1]), 51),
        Err(EditError::Unreachable(51)),
        "no ascendancy chosen"
    );
}

#[test]
fn a_choice_option_should_replace_the_other() {
    let t = tree();
    let s = spec(1, 1, &[1, 50, 51, 52, 70]);
    let a = allocate(&t, &s, 53).expect("reachable");
    assert_eq!(a.spec.nodes, set(&[1, 50, 51, 53, 70]));
    assert_eq!(
        points(&t, &a.spec),
        Points {
            passive: 0,
            ascendancy: 2
        },
        "the bloodline node counts as PoB counts it"
    );
}

#[test]
fn deallocate_should_remove_orphans_and_their_masteries_but_keep_kept_nodes() {
    let t = tree();
    let mut s = spec(1, 0, &[1, 10, 11, 12, 13, 40, 70, 71]);
    s.masteries.insert(40, 100);
    let r = deallocate(&t, &s, 10).expect("allocated");
    assert_eq!(r.removed, [10, 11, 12, 13, 40]);
    assert_eq!(r.spec.nodes, set(&[1, 70, 71]));
    assert!(r.spec.masteries.is_empty());
    // Another route keeps the rest.
    let s = spec(1, 0, &[1, 10, 11, 12, 20, 21, 70]);
    let r = deallocate(&t, &s, 11).expect("allocated");
    assert_eq!(r.removed, [11]);
    assert_eq!(deallocate(&t, &s, 1), Err(EditError::Start(1)));
    assert_eq!(deallocate(&t, &s, 13), Err(EditError::NotAllocated(13)));
    assert_eq!(deallocate(&t, &s, 70), Err(EditError::NotModelled(70)));
}

#[test]
fn set_mastery_should_need_a_connected_group_and_a_free_effect() {
    let t = tree();
    let s = spec(1, 0, &[1, 10, 11, 12, 20, 21, 70]);
    let m = set_mastery(&t, &s, 40, 100).expect("12 is connected");
    assert_eq!(m.masteries.get(&40), Some(&100));
    assert!(m.nodes.contains(&40) && m.nodes.contains(&70));
    assert_eq!(
        set_mastery(&t, &m, 41, 100),
        Err(EditError::EffectTaken {
            effect: 100,
            by: 40
        })
    );
    assert!(set_mastery(&t, &m, 41, 101).is_ok());
    assert_eq!(
        set_mastery(&t, &m, 40, 7),
        Err(EditError::UnknownEffect {
            node: 40,
            effect: 7
        })
    );
    assert_eq!(set_mastery(&t, &s, 12, 100), Err(EditError::NotMastery(12)));
    assert_eq!(
        set_mastery(&t, &spec(1, 0, &[1]), 40, 100),
        Err(EditError::Unreachable(40))
    );
    // Changing an effect keeps the mastery.
    let again = set_mastery(&t, &m, 40, 101).expect("its own effect may change");
    assert_eq!(again.masteries.get(&40), Some(&101));
}

#[test]
fn select_class_should_keep_what_still_connects_and_drop_the_ascendancy() {
    let t = tree();
    let s = spec(1, 1, &[1, 50, 51, 10, 11, 12, 13, 30, 20, 70, 71]);
    let r = select_class(&t, &s, 2).expect("a class");
    assert_eq!(r.spec.class, 2);
    assert_eq!(r.spec.ascendancy, 0);
    assert_eq!(r.spec.nodes, set(&[2, 10, 11, 12, 13, 30, 70, 71]));
    assert_eq!(r.removed, [20, 51], "20 hung from class A's start");
    assert_eq!(select_class(&t, &s, 9), Err(EditError::UnknownClass(9)));
}

#[test]
fn select_ascendancy_should_swap_the_start_and_drop_the_old_nodes() {
    let t = tree();
    let s = spec(1, 1, &[1, 50, 51, 10, 70]);
    let r = select_ascendancy(&t, &s, 2).expect("an ascendancy");
    assert_eq!(r.spec.nodes, set(&[1, 60, 10, 70]));
    assert_eq!(r.removed, [51]);
    let none = select_ascendancy(&t, &r.spec, 0).expect("none");
    assert_eq!(none.spec.nodes, set(&[1, 10, 70]));
    assert_eq!(
        select_ascendancy(&t, &s, 3),
        Err(EditError::UnknownAscendancy(3))
    );
}

#[test]
fn reset_should_leave_only_the_starts() {
    let t = tree();
    let mut s = spec(1, 1, &[1, 50, 51, 10, 70, 71]);
    s.masteries.insert(40, 100);
    let r = reset(&t, &s);
    assert_eq!(r.nodes, set(&[1, 50]));
    assert!(r.masteries.is_empty());
}

#[test]
fn points_should_count_as_pob() {
    let t = tree();
    let s = spec(1, 1, &[1, 50, 51, 52, 10, 40, 71]);
    assert_eq!(
        points(&t, &s),
        Points {
            passive: 3,
            ascendancy: 1
        }
    );
}

#[test]
fn the_url_should_hold_nodes_clusters_and_masteries_without_starts() {
    let t = tree();
    let mut s = spec(1, 1, &[1, 50, 10, 40, 65537]);
    s.masteries.insert(40, 100);
    let bytes = [
        0, 0, 0, 6, 1, 0b1001, 2, 0, 10, 0, 40, 1, 0, 1, 1, 0, 100, 0, 40,
    ];
    let expected: String = url::encode(&t, &s, 2).expect("encodable");
    assert_eq!(
        expected,
        format!(
            "{}{}",
            url::PREFIX,
            base64_std(&bytes).replace('+', "-").replace('/', "_")
        )
    );
    let big = spec(1, 0, &(1000..1300).collect::<Vec<_>>());
    assert_eq!(url::encode(&t, &big, 0), None, "more than 255 nodes");
}

/// A plain base64 to check the encoder's against.
fn base64_std(bytes: &[u8]) -> String {
    const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut bits = String::new();
    for b in bytes {
        bits.push_str(&format!("{b:08b}"));
    }
    while !bits.len().is_multiple_of(6) {
        bits.push('0');
    }
    let mut out: String = bits
        .as_bytes()
        .chunks(6)
        .map(|c| {
            let v = c
                .iter()
                .fold(0usize, |acc, d| acc * 2 + usize::from(*d == b'1'));
            char::from(A[v])
        })
        .collect();
    while !out.len().is_multiple_of(4) {
        out.push('=');
    }
    out
}

#[test]
fn a_node_not_connected_on_import_should_survive_every_edit_but_reset() {
    let t = tree();
    // 13 was allocated through a radius jewel: nothing connects it.
    let s = spec(1, 1, &[1, 50, 10, 11, 13, 51]);
    let r = deallocate(&t, &s, 10).expect("allocated");
    assert_eq!(r.removed, [10, 11], "13 was never connected, so it stays");
    assert!(r.spec.nodes.contains(&13));
    let r = select_ascendancy(&t, &s, 2).expect("an ascendancy");
    assert!(r.spec.nodes.contains(&13) && !r.spec.nodes.contains(&51));
    let r = select_class(&t, &s, 2).expect("a class");
    assert!(r.spec.nodes.contains(&13));
    let a = allocate(&t, &spec(1, 1, &[1, 50, 51, 52, 13]), 53).expect("reachable");
    assert!(a.spec.nodes.contains(&13) && !a.spec.nodes.contains(&52));
    assert!(!reset(&t, &s).nodes.contains(&13));
}

#[test]
fn an_ascendancy_path_node_should_reach_the_main_tree() {
    let t = tree();
    let s = spec(1, 1, &[1, 50, 55]);
    let a = allocate(&t, &s, 13).expect("through 55 and 30");
    assert_eq!(a.path, [30, 13]);
    // 30 hangs from 55: removing 55 removes it.
    let r = deallocate(&t, &a.spec, 55).expect("allocated");
    assert_eq!(r.removed, [13, 30, 55]);
    // Without the ascendancy, 55 is not ours.
    assert_eq!(
        allocate(&t, &spec(1, 0, &[1]), 55),
        Err(EditError::Unreachable(55))
    );
}

#[test]
fn shortest_paths_should_be_counted_up_to_the_cap() {
    let t = tree();
    let s = spec(1, 0, &[1]);
    // 1-10-11 is the only shortest path to 11.
    assert_eq!(shortest_path_count(&t, &s, 11, 2), Ok(1));
    // 12: 1-10-11-12 and 1-20-21-12 tie; the rule takes the smaller list.
    assert_eq!(shortest_path_count(&t, &s, 12, 2), Ok(2));
    assert_eq!(shortest_path_count(&t, &s, 12, 1), Ok(1));
    assert_eq!(path(&t, &s, 12), Ok(vec![10, 11, 12]));
    assert_eq!(
        shortest_path_count(&t, &s, 1, 2),
        Err(EditError::Start(1)),
        "as path"
    );
}

#[test]
fn a_cluster_area_should_be_neither_modelled_nor_reached() {
    let t = tree();
    assert!(t.nodes[&80].modelled && !t.nodes[&80].cluster_area);
    for n in [81, 82, 83] {
        assert!(t.nodes[&n].cluster_area, "{n}");
        assert!(!t.nodes[&n].modelled, "{n}");
    }
    let s = spec(1, 0, &[1, 10, 11, 12, 13, 80]);
    assert_eq!(allocate(&t, &s, 81), Err(EditError::NotModelled(81)));
    assert_eq!(allocate(&t, &s, 82), Err(EditError::NotModelled(82)));
}

#[test]
fn connected_to_start_should_follow_every_link_from_any_allocated_start() {
    let t = tree();
    // 1 - 10 - 11 connect; 50 is a start too and 51 hangs from it; 2 is
    // another class's start, and 30 and 13 hang from it; 70 (kept), 71
    // (absent), and a cluster jewel's node do not count.
    let nodes = set(&[1, 10, 11, 13, 70, 71, 50, 51, 30, 2, 0x1_0005]);
    assert_eq!(
        connected_to_start(&t, &nodes),
        set(&[1, 10, 11, 50, 51, 30, 13, 2])
    );
    // No allocated start: nothing counts.
    assert!(connected_to_start(&t, &set(&[10, 11])).is_empty());
}

/// The test tree with positions: the socket 80 at the origin, 21 at 100,
/// 12 at 150, 30 at 200, the mastery 40 at 50, the keystone-free rest far.
fn placed() -> Tree {
    let mut t = tree();
    for n in t.nodes.values_mut() {
        n.position = Some((10_000.0, f64::from(n.id) * 1_000.0));
    }
    for (id, x) in [(80, 0.0), (21, 100.0), (12, 150.0), (30, 200.0), (40, 50.0)] {
        if let Some(n) = t.nodes.get_mut(&id) {
            n.position = Some((x, 0.0));
        }
    }
    t
}

fn leap(outer: i64) -> radius::Leap {
    radius::Leap {
        socket: 80,
        ring: radius::Ring { inner: 0, outer },
        centre: radius::Centre::Socket,
        keystones_only: false,
    }
}

#[test]
fn a_ring_should_hold_the_nodes_at_its_distances_but_not_its_centre_or_masteries() {
    let t = placed();
    let full = radius::Ring {
        inner: 0,
        outer: 150,
    };
    assert_eq!(radius::in_ring(&t, 80, full, true), set(&[21, 12]));
    let ring = radius::Ring {
        inner: 150,
        outer: 200,
    };
    assert_eq!(radius::in_ring(&t, 80, ring, true), set(&[12, 30]));
    // Around a keystone, sockets are never in the radius.
    assert!(!radius::in_ring(&t, 21, full, false).contains(&80));
    assert!(radius::in_ring(&t, 21, full, true).contains(&80));
}

#[test]
fn a_leap_should_allocate_alone_and_hold_while_its_socket_connects() {
    let t = placed();
    // Class B: 2 - 30 - 13 - 80 (the socket); 21 is two steps away.
    let mut s = spec(2, 0, &[2, 30, 13, 80]);
    assert_eq!(path(&t, &s, 21).unwrap(), vec![12, 21]);
    s.leaps.push(leap(120));
    assert_eq!(path(&t, &s, 21).unwrap(), vec![21]);
    assert_eq!(shortest_path_count(&t, &s, 21, 9).unwrap(), 1);
    let a = allocate(&t, &s, 21).unwrap();
    assert_eq!(a.path, vec![21]);
    assert!(connected(&t, &a.spec).contains(&21));
    // No path starts from a held node: 20 (next to 21 and A's start) is
    // out of reach.
    assert_eq!(path(&t, &a.spec, 20), Err(EditError::Unreachable(20)));
    // Out of the radius: a path as usual.
    let mut far = s.clone();
    far.leaps = vec![leap(50)];
    assert_eq!(path(&t, &far, 21).unwrap(), vec![12, 21]);
    // Losing the socket's connection drops what it held.
    let r = deallocate(&t, &a.spec, 13).unwrap();
    assert_eq!(r.removed, vec![13, 21, 80]);
}

#[test]
fn held_should_add_nodes_in_counted_sockets_radii_until_nothing_changes() {
    let t = placed();
    let allocated = set(&[1, 21, 30, 80]);
    let none = radius::held(&t, &[leap(250)], &allocated, &set(&[1]));
    assert!(none.is_empty(), "the socket does not count: {none:?}");
    assert_eq!(
        radius::held(&t, &[leap(250)], &allocated, &set(&[1, 80])),
        set(&[21, 30])
    );
}
