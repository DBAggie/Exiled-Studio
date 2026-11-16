# Passive tree report

The tree half of the passive tree and Phase 5 plan (`dev/active/passive-tree-and-phase-5/`), checked against its gate (`docs/plan/ACTION-PLAN.md`, written by T0). This report covers the steps below the plan's cut line, done on branch `feat/tree-remaining` at the maintainer's request (2026-11-16: "complete tasks all in a single pr one by one until we have gotten through the remaining plan items"):

- T3, cluster jewels;
- T4, older tree versions;
- T5b, cluster sockets and conversion in the app;
- T7's cluster hash v2;
- T8, parity and this report;
- the close of T1's last item.

The steps above the cut line shipped in PRs #113 and #114 (`docs/tree/editor.md`). Phase 5's own report is `docs/phase-5/phase-5-report.md`.

## What was built

| Step | What | Document |
|---|---|---|
| T3 | The game's cluster jewel tables in the bundle; PoB's subgraphs (`loom_tree::poe1::cluster`); the added nodes in the calculation, small passives scaled by the jewel; PoB's two numberings, legacy specs migrated on import; the `cluster_jewels` data-parity table | [cluster-jewels.md](cluster-jewels.md) |
| T7 hash v2 | A spec edited here exports `clusterHashFormatVersion="2"` | [cluster-jewels.md](cluster-jewels.md), "The two numberings" |
| T4 | Tree packs 3_25 to 3_28 converted from GGG's export at pinned commits (`tree-pack`); lines read as stat IDs, older wordings through a reviewed table; the 1% rule enforced; the calculation picks the spec's own version | [tree-packs.md](tree-packs.md) |
| T5b | Cluster jewels socketed in the editor, their nodes drawn and edited; `convert-tree-version` with its preview; the jewel picker by size; the e2e's cluster steps | [editor.md](editor.md), "Cluster sockets" and "Converting a tree" |
| T8 | Layout, allocation, and conversion against PoB for every pack; the public builds on older trees; the B3 property test | [parity.md](parity.md), "Older versions" |
| T1 | Its last item, the layout as a required file of data schema 4, closed as decided: it stays optional, so released bundles stay valid | [layout.md](layout.md) |

## Gate

| # | Item | Status |
|---|---|---|
| 1 | Layout (MR, records re-checked in CI): every node within 0.01 of the export's position, edge sets equal, starts equal | **Met.** 3_29 (`records/layout.json`, PR #113): the main tree within 0.01, other nodes within the export's rounding, each named. 3_25 to 3_28 (`records/layout-<version>.json`): every node within 0.01, no difference. One link exists only in PoB's own 3_25 data (mastery 49391); the record accounts for it, and no allocation can use it. |
| 2 | Allocation (MR): paths, deallocations, points, and conversion drops equal PoB's | **Met.** 2,000 seeded requests on each of 3_29 and 3_25 to 3_28: unique paths equal node for node, tied paths of equal length, deallocations and points equal (`records/alloc*.json`). Conversion (`records/convert-*.json`): 8,000 seeded specs and the 65 public builds' own specs on older trees convert exactly as PoB's import does. |
| 3 | Cluster jewels (MR; CI on T1a): subgraph IDs equal PoB's; cluster fixtures' fields match PoB | **In CI: met.** The subgraphs' numbers, places, links, and the legacy map, worked from PoB's rules, on the synthetic bundle (`loom-tree/tests/cluster.rs`, `loom-pipeline/tests/cluster.rs`). **Maintainer-run: open.** The extraction with `PassiveTreeExpansionSpecialSkills`, `data-parity` at export format 9, and the corpus builds with cluster jewels through `calc-parity`. |
| 4 | Corpus (MR): no `tree:version` reason for a carried version; every such build calculates; differences registered; no edit removes a kept node (B3 property test in CI) | **Public builds: met.** The 12 community fixtures on 3_25 to 3_28 calculate on their packs: their 233 fields go from `indeterminate` (`tree-version`) to `partial`, with no version reason. Fields equal to PoB stay 82 of 288. The one value that moves is Shock Nova's mana, toward the 3_27 tree's own node (5% where 3_29 has 6%), so no divergence entry is needed ([tree-packs.md](tree-packs.md), "In the calculation"). **B3: met.** `no_edit_should_remove_a_kept_node` (`loom-tree/tests/synthetic_tree.rs`): 256 random sequences of every edit but reset keep every kept node. **Private corpus: open, maintainer-run** (below). |
| 5 | App (CI): tree workflow in vitest and e2e on Windows and Linux; axe clean; keyboard-only workflow | **Here: met for vitest and axe.** `tree.test.tsx` adds conversion; `treeCluster.test.ts` is new. The e2e's cluster steps run in CI's `e2e` job on both systems; this workstation did not run them. |

The plan's fallbacks:
- **No version ships over the 1% rule.** All four packs read at least 99% of their lines (0.56% to 0.98% unread), with 11 reviewed rewordings. The rule is enforced by `tree-pack`.
- **No cluster jewel kind is known to differ from PoB.** Its subgraphs follow PoB's rules. The corpus check against PoB is maintainer-run.

## Found on the way

- **Same numbers are not the same stat.** Rewordings learned from GGG's data included mechanic changes that keep their values: Damage over Time to Spell Damage, Mana Cost to Mana Cost Efficiency, critical-strike reflect to all reflect. So the converter applies only rewordings a person accepted (`tools/tree/rewordings.json`). It lists any new one as unreviewed, and leaves its lines unread.
- **A node's own stats settle an ambiguous line.** 3.28's "+20 to Dexterity and Intelligence" has two readings. Picking the one whose stats the bundle's node already has fixed 40 attribute points in one public build.
- **PoB never allocates a mastery without a choice.** It also drops one whose effect its tree does not know. Conversion now does the same.
- **The PoB script kept state between requests.** PoB's `ResetNodes` leaves ascendancy starts allocated, so a bloodline start from one public build appeared in the next. `pob_tree.lua` now clears the spec before each request.
- **PoB's 3_25 data has one link GGG's export does not** (mastery 49391 to 20953), at both 3.25 tags.

## Maintainer-run

1. **The signed bundle with packs.** Run `tools/tree/acquire` for 3_25 to 3_28, then `tree-pack` four times, from 3_28 down ([tree-packs.md](tree-packs.md), "Making one"). Then `bundle-check`, and sign as any bundle (`docs/release/signing.md`).
2. **`data-parity` per pack**, as the plan's T4 row asks.
3. **The private corpus on the packs.** `calc-run` the corpus with the scope file on the bundle with and without packs, and `calc-parity` both, as [tree-packs.md](tree-packs.md) did for the public builds. Expect:
   - no `tree-version` reason for builds on 3_25 to 3_28;
   - each changed value explained by its tree's own nodes or registered.
4. **T3's three items** (gate 3).
5. **`TREES="3_25 3_26 3_27 3_28" tools/tree/parity.sh <bundle>`** on the signed bundle, to re-record the layout, allocation, and conversion records against it (the committed ones name the bundle made here, manifest `4814e3ac…`).

## Not done

- **Timeless jewels and the other radius jewels** that transform nodes or count attributes. Their nodes allocate as usual and are not modelled.
- **Editing an older spec on its own tree.** It is calculated on its pack, and edited after conversion.
- **Packs before 3_25**, and the older versions' own stat descriptions.
