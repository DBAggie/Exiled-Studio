# Passive tree parity with PoB

Passive tree plan, gate item 1 (layout), gate item 2 (allocation, on the bundle's own tree version), and T7's read-back. PoB is the pinned oracle (v2.67.2), run headless on the oracle runtime. These are maintainer-run records: they need the real bundle and the oracle checkout, which CI does not have.

## How it runs

```sh
tools/tree/parity.sh [bundle-dir]      # default local/tree/bundle-poe1-tree-a
```

The script verifies the oracle and runs three comparisons. It writes the records to `docs/tree/records/` and its working files to `local/tree/pob/`, and exits 0 when every record matches. PoB's side comes from `tools/oracle/data/pob_tree.lua`, which follows the pattern of `pob_data.lua`: it starts PoB with its own `HeadlessWrapper.lua`, writes only to its output (the oracle's write guard; any write it refuses is listed in the output), and with `zlib=` installs the oracle's inflate shim. Loom's side is `loom-parity-cli tree-parity`. The plan named an adapter protocol 5 for this. Instead, the script stays outside the adapter and its protocol, as `pob_data.lua` does, because it reads trees and specs, not calculations.

| Mode | PoB | loom |
|---|---|---|
| `layout` | `PassiveTree` as PoB builds it: each node's position (`ProcessNode`), links (`linkedId`), class and ascendancy starts, and the nodes its tree file has that the class removed | `loom_tree::Tree` from the bundle with the layout file |
| `alloc` | for each request: `ImportFromNodeList` with the class, ascendancy, and allocated set, then the target's `path` or `depends`, the allocated set after the import, and `CountAllocNodes` | `poe1::path`, `poe1::deallocate`, `poe1::points` on the same spec |
| `readback` | PoB loads a build loom exported (`loadBuildFromXML`), and each spec's class, ascendancy, nodes, mastery effects, and socketed jewels | the same file read by loom's PoB XML reader |

`alloc-requests` makes the requests from a seed: a class, perhaps an ascendancy, up to five allocations by loom's rules, then one target, either allocated (a deallocation case) or not. Because PoB starts from the same connected spec, its import keeps the allocated set unchanged, and the record checks that.

## Records (2026-11-15, bundle manifest `65952be5…`, tree 3_29)

**Layout** (`records/layout.json`): every one of the bundle's 2,987 nodes is accounted for. Their links are equal (2,943 nodes; the other 44 are below), and the 28 starts are equal (7 classes, 21 ascendancies).
- **2,884 nodes within 0.01** of PoB's position. Every main-tree node is in this group except two cluster jewel proxies.
- **59 within the export's rounding (up to 0.06).** GGG's export, which PoB's `tree.lua` carries, writes group centres rounded: 116 of 3_29's groups have one decimal. The game's psg, which the bundle carries, has them in full. These 59 are ascendancy, bloodline, and proxy nodes.
- **Ascendancies are compared under one offset each.** PoB's `fix_ascendancy_positions.py` moves every group of an ascendancy by one offset, to a point of PoB's choosing, because GGG's ascendancy positions "look scrambled". Loom draws GGG's positions, which sit outside the main tree, at least 564 units from its nearest node. The record lists each offset.
- **44 nodes PoB removes**: the legacy alternate ascendancies Primalist (11), Warden (15), and Warlock (18), which PoB's `PassiveTree.lua` drops. The bundle carries them, and loom keeps them as nodes it does not edit.
- PoB has 409 nodes the bundle lacks. These include cluster jewels' generated nodes and the seven ascendancy nodes PoB's script adds.

Gate item 1 asked for every node within 0.01. This record meets that for the main tree. Elsewhere it allows 0.1, because the export rounds, and it names each such node.

**Allocation** (`records/alloc.json`): 2,000 requests, seed 20261115, no differences.
- 1,364 allocations: 766 with a unique shortest path, equal node for node; 363 with tied paths, of equal length (loom's tie rule, plan T-f); 235 unreachable in both.
- 547 deallocations with equal removed sets. Requests whose target is a start are not counted, because loom never removes a start.
- 2,000 point counts equal (passive and ascendancy).

**Read-back** (`records/readback-*.json`): three community builds on 3_29 were edited through the application's commands by `crates/loom-app/tests/tree_workflow.rs` (`edited_builds_should_be_exported_for_pob_to_read_back`). The edits allocate the two nearest keystones, choose a mastery effect, remove the first keystone's path, switch to another ascendancy, allocate its nearest notable, and move a socketed jewel (not a cluster jewel) to an allocated empty socket (`SocketJewel`) where the build has one. The builds were then exported, and PoB read every spec with the same nodes, mastery effects, jewels, class, and ascendancy as loom, plus the build's class and ascendancy names: ice-nova 4 of 4 specs, Facebreaker 11 of 11, Animate Guardian 1 of 1. Facebreaker Luminary has 11 specs with cluster jewels, a bloodline, and a timeless jewel; loading it needs the inflate shim.

## What the comparison found

The first allocation run showed that loom allocated cluster jewels' position proxies and nested sockets, which PoB's spec leaves out (`PassiveSpec:Init`: proxies, proxy groups, sockets with a parent). `loom_tree::Node::cluster_area` now marks them: the nested expansion sockets, the groups holding one, and stat-less non-socket nodes linked to one. This gives exactly PoB's 78 nodes on 3_29. They are not modelled, not drawn, and kept in a spec. Allocating a cluster jewel's nodes is T3.

## Not covered

- Masteries in `alloc`: the requests carry none, so mastery choice is checked only by the read-back.
- Radius jewels (Intuitive Leap and the like, T-f): modelled ([editor.md](editor.md)), but not in `alloc`'s requests, and the bundles here lack the game's radius table. PoB's read-back of the Essence Drain build keeps the three nodes its jewel holds, as loom's rule does with PoB's radii. The radii themselves are checked by the data-parity table `jewel_radii` once a bundle has them.
- Other tree versions (T4) and cluster subgraphs (T3).
- CI cannot re-run the records, because their inputs are local. `tree_parity`'s test `the_committed_tree_records_should_match_and_add_up` checks that each committed record has no differences, that its counts account for every node, request, and spec, and that its hashes are SHA-256.
