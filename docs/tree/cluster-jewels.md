# Cluster jewels (passive tree plan, T3)

Cluster jewels add nodes to the tree when they are socketed in a cluster socket. This step reads the game's tables for them, builds the added nodes as PoB v2.67.2 does, and gives the calculation those nodes' stats. It also handles PoB's two numberings for cluster nodes. The plan's item is T-i (`dev/active/passive-tree-and-phase-5/passive-tree-and-phase-5-plan.md`).

## The data

`poe1/cluster_jewels.json` is a PoE1 section file (`loom_data::bundle::poe1::ClusterJewels`). `loom-parity-cli bundle` writes it from five tables (`tools/parity-cli/src/cluster_jewels.rs`), reading them as PoB's `Export/Scripts/cluster.lua` does:

| Field | Table | What |
|---|---|---|
| `sizes` | `PassiveTreeExpansionJewelSizes` | each size's name, numbered by its row |
| `jewels` | `PassiveTreeExpansionJewels` | each base's size, node counts, and the ring places its sockets, notables, and small passives take |
| `skills` | `PassiveTreeExpansionSkills` | the small passives a jewel can grant (a jewel names one by its row), with their mastery, tag, and size |
| `specials` | `PassiveTreeExpansionSpecialSkills` | the notables and keystones, each with the stat that adds it; `order` is the stat's row (PoB's `notableSortOrder`); `keystone` comes from the passive |
| `slots` | `PassiveJewelSlots` | each cluster socket's size, index, parent (the slot it replaces), position proxy, and start index by size |
| `grants` | the stat descriptions | each "Added Small Passive Skills also grant: ..." stat paired with the stat it gives each small passive |

The extractor now also roots `PassiveTreeExpansionSpecialSkills` (`TREE_TABLES_POE1`). An extract made before this step lacks it, so its bundle has no cluster file and cluster nodes stay a reason. The validator checks every passive, stat, and base the file names, duplicate rows, and index ranges (`cluster_domain`).

**Grants.** The game has 42 "also grant" stats (`local_affliction_jewel_small_nodes_grant_*`). Each one's line is "Added Small Passive Skills also grant: " followed by the text of a global stat. The bundle step pairs it with the one non-local, single-stat description in the same file that has a variant with that text. The match ignores case and reminder handlers, and the other handlers must be the same. That pairs 39 of the 42 in build 26530219's descriptions. The other three word the text apart: Evasion against Evasion Rating, Critical Strike Multiplier against Global Critical Strike Multiplier, and minion life regeneration with its decimals. They are listed in `GRANT_PAIRS`, each with the modifier PoB reads from the rest of the line. A grant stat that pairs neither way is left out, and a jewel with it is not built.

## The subgraphs

`loom_tree::poe1::cluster` ports PoB's `BuildClusterJewelGraphs` and `BuildSubgraph` (`Classes/PassiveSpec.lua` 1723 to 2271):
- A `Jewel` is what a jewel's lines say, read by `loom_pipeline::cluster_jewels` from the resolved stat IDs (ADR-015):
  - the node count, the socket counts, the small passive's row, nothingness, and the small passives' increased effect;
  - each notable or keystone stat;
  - each grant.
- `subgraphs` builds a subgraph for each large socket holding a valid jewel, then recursively for the jewels in the sockets it adds. The node numbers, places, ring links, and medium-jewel place rules are PoB's (the module docs list them).
- `expand` puts the subgraphs on a copy of the tree: the cluster areas' own links are cut, the nodes are placed around the proxy's group, and a socket the jewel adds stands in for the tree's nested socket. Allocation, deallocation, and `connected_to_start` then walk it like the rest of the tree.
- When PoB would stop with an error (no socket for a place, a notable the data lacks, no entrance), nothing is built. The jewel's allocated nodes then stay a `cluster-jewel-node` reason.

## The calculation

`loom_pipeline::resolve_passives` builds the socketed jewels' subgraphs and resolves the spec on the expanded tree:
- An allocated node that a jewel adds gets its state as any node does, and its stats go in `ResolvedPassives::cluster`.
- An allocated cluster node that no jewel adds stays `ClusterNode`, a reason.

`loom-calc` ingests a connected cluster node's stats. A small passive's stats are scaled by the jewel's increased effect, as PoB scales the node's mods (`CalcSetup.lua` 135 to 141, `ScaleAddList`, through `skill_modifiers::scale`; `Ingest::stats_scaled`). The jewel's own cluster stats have no effect of their own (`NO_EFFECT_PREFIXES`): they act through the nodes.

## The two numberings

PoB renumbered cluster nodes when it fixed its cluster hashes. A spec with `clusterHashFormatVersion="2"` uses the current numbers. A spec without the attribute that lists its nodes is legacy (format 1), as PoB reads it (`PassiveSpec.lua` 95). For a legacy spec, PoB works out each node's old number while it builds the subgraphs, then moves the allocations and socketed jewels to the current numbers (1651 to 1720, 1807 to 1884). Here:
- `subgraphs_for(tree, jewels, legacy)` builds the same subgraphs, plus each subgraph's map from old numbers to current ones (`Subgraph::legacy`). Two parts make up the map: the sockets in the legacy proxy group (`BuildLegacyProxyGroup`) and the ring slots on the proxy's own orbit (`BuildLegacyClusterOrbitMappings`). A nested socket's jewel is also looked up under its old number, as `GetSocketedJewel` does. Where two places share a slot, PoB keeps whichever its table walk sees last, in no set order; here the later place wins.
- `resolve_passives` applies the map without changing the build, so `calc-run` reads a legacy build as PoB does. Each state is given under the spec's own numbers.
- `loom_pipeline::migrate_cluster_hashes` rewrites each legacy spec once and marks it 2 (ADR-009): its nodes, without repeats, and its sockets. The app calls it on import. It skips a spec of another tree version than the data's, and a tree without cluster data (PoB then renumbers the spec when it reads the export).
- The PoB export writes `clusterHashFormatVersion="2"` for a spec whose nodes were made or edited here, so PoB does not read it as legacy (T7). A spec read from PoB keeps its own attributes.

## Checks

In CI, on the synthetic tree bundle (`make_bundles.py`, `cluster_jewels`: made-up tables, passives, bases, and line texts):
- `loom-tree/tests/cluster.rs`:
  - the large, medium, and small subgraphs' numbers, places, and links, worked by hand from PoB's rules;
  - a keystone jewel, invalid jewels, nothingness;
  - allocation on the expanded tree;
  - the legacy map, worked by hand for a proxy on a 16-slot orbit.
- `loom-pipeline/tests/cluster.rs`:
  - a PoB build with a large cluster jewel imported, read, allocated, resolved, and calculated with no node left a reason;
  - the nodes without their jewel;
  - the same allocation saved by an older PoB, which resolves the same and migrates to the current numbers.
- `parity-cli`: the converter on made-up tables, the grant pairing, and the `cluster_jewels` table of `data-parity`.
- `data-parity` adds the `cluster_jewels` table. It compares the bundle with PoB's `data.clusterJewels` (export format 9, `tools/oracle/data/pob_data.lua`): each jewel's layout, each small passive's tag and name, each notable's order, the keystones, and each proxy's start indices. PoB's side was run here on the pinned oracle and reads 9 jewels, 300 notables, and 8 keystones.

Maintainer-run, on the main workstation:
- The extraction with `PassiveTreeExpansionSpecialSkills`, then `bundle`.
- `data-parity` at export format 9 (`cluster_jewels` 0 differences expected, else each one explained).
- The corpus builds with cluster jewels through `calc-parity`, to check the subgraph numbers against PoB's.

## In the editor (T5b)

Done on the same branch: `socket-jewel` takes a cluster jewel in a socket of its size, and the editor works on the tree with the spec's cluster jewels expanded. The screen draws the added nodes, and the jewel picker lists the jewels that fit ([editor.md](editor.md), "Cluster sockets").

## The cluster tables on build 26530219 (2026-11-19)

**The data:** extract `local/extract/poe1/26530219-ot2` roots `PassiveTreeExpansionSpecialSkills`. Its index SHA-256 is the evidence bundles' (`d23c7899…`), so it is build 26530219. Its `bundle` writes `poe1/cluster_jewels.json`.

**Evidence bundle** `local/p6/packs/b-cl`: `b-gl` (data schema 5) with only that file added. `bundle-check` reports no findings; the bundle is local and unsigned.

**Data parity:**
- Table `cluster_jewels` compares 479 rows with 1 difference (before, 353 rows of PoB's with none in the bundle).
- The one: two notables are named Evil Eye. PoB's exporter keys `notableSortOrder` by name, so Lua keeps the last row (11455); the bundle keeps each notable's own (11454). Rule `cluster-notable-order-duplicate-name`, `pob-derivation`.
- `uncategorized` falls from 353 to 0: the 353 were the cluster rows the bundle lacked.

**Calculation** (`local/p6/cl/`, against `local/p6/d5/` on `b-gl`):
- **Public `calc-parity`:** 237 builds, 292 expected, 0 unexpected. 286 values moved, 255 toward PoB and 31 away; 13 are now equal to PoB.
- **Corpus:** 0 unexpected; 170 moved, 153 toward and 17 away.
- **Every "away" move checked** shows an earlier gap the cluster nodes now uncover. In Generals Perforate Zerker, intelligence and mana now equal PoB's (96, 672). Its unreserved mana moved from 139 to 141 against PoB's 138, because its reserved mana was already 4 below PoB's. The other away moves are a few points of attributes or life in builds already above PoB's.

**Maintainer:** the next signed bundle ships `poe1/cluster_jewels.json` from this extract (the full data run, `docs/phase-6/timeless.md`).
