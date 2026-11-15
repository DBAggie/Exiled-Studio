# Passive tree editor

Passive tree plan (`dev/active/passive-tree-and-phase-5/`), the cut-line steps on the bundle's own tree version: T1 (crate and layout), T1a (synthetic fixture), T2 (rules and the calculation's resolved passive set), T5a (app commands, typed class and ascendancy), T6 (screen, sprites), and T7 (export of edited specs), with the radius jewels that allocate without a path (T-f). Built on branch `feat/ux-sprint-2` at the maintainer's request (2026-11-15: "Implement the passive skill tree"; then "complete the open items in the currently open branch and PR"). Layout and sprite evidence: [layout.md](layout.md).

## `loom-tree` (core, wasm32)

- `Tree::from_tables(passives, layout)`: the graph from the bundle's passive tables, with links made undirected. Each node has a kind (normal, notable, keystone, mastery, jewel socket, cluster socket, class start, ascendancy start), a group, an orbit and slot, its position when the bundle has the layout, its stats, whether it is a multiple-choice option, and whether the editor **models** it. Classes come with their starts and their ascendancies, numbered as PoB numbers them (`classId`, `ascendClassId`).
- **Modelled** nodes are those reachable from the class starts without entering an ascendancy, each ascendancy's nodes reached from its start, and the masteries linked to such nodes. The others are **kept**: bloodline and alternate ascendancies, cluster jewels' template nodes, and the cluster areas (`cluster_area`: nested expansion sockets, the groups holding one, and their position proxies, which PoB's `PassiveSpec:Init` also leaves out). That makes 255 kept on 3_29, 78 of them cluster areas; cluster areas are not drawn. Edits never allocate them and never remove them (T-g). A node the tree lacks is kept the same way.
- `poe1`: the rules, as PoB's `PassiveSpec.lua` where they are deterministic.
  - **Allocation** takes a shortest path from the allocated nodes connected to the class or ascendancy start. Class and ascendancy starts, masteries, and kept nodes are never on a path. Ascendancy nodes link only within their ascendancy, and only the spec's ascendancy is reachable. The one way out is an ascendancy node's own link into the main tree: 3_29's six Ascendant "Path of the …" nodes (for example 56722 to 57264 and 57226) are walked from the ascendancy into the main tree, never the other way, as PoB's `BuildPathFromNode` and `FindStartFromNode`.
  - **The tie rule is loom's (T-f).** Of several shortest paths, the one whose node list, from the allocated set to the target, is lexicographically smallest by node number. PoB's choice follows Lua hash order, which is not a rule.
  - **A mastery** hangs from the nodes linked to it: the export's and the game graph's links from its group's notables, as PoB's `linked`. It is allocated by choosing an effect, which needs a connected linked node. An effect is taken at most once per mastery group.
  - **Deallocation** removes the node and the modelled nodes the edit disconnects: those connected before it and not after. Kept nodes count as connected. A node that was not connected before the edit stays (for example one allocated through a radius jewel, which the editor does not model). The same applies to class and ascendancy changes and to replacing a multiple-choice option.
  - **Class and ascendancy changes** follow PoB's `SelectClass` and `SelectAscendClass`. The old starts go and the new ones come. A class change also resets the ascendancy and drops what no longer connects. An ascendancy change drops the old ascendancy's nodes.
  - **Reset** is PoB's `ResetNodes`: only the starts stay, so kept nodes go too, as T-g allows.
  - **Points** are counted as PoB's `CountAllocNodes`: starts are not counted, and an ascendancy's multiple-choice options are not counted. They are shown, never enforced.
  - **A multiple-choice option** replaces the other options of its parent.
  - **Radius jewels that allocate without a path** (T-f, `poe1::radius`): Intuitive Leap and Thread of Hope around their socket, Impossible Escape around a keystone (`Spec.leaps`, one `Leap` per socketed jewel: socket, ring, centre, keystones only). A node is in a radius when its distance from the centre is between the ring's inner and outer radius, both included, as PoB's `nodesInRadius` (the centre, masteries, cluster areas, and, around a keystone, sockets excluded). A leap lets a node in its radius be allocated alone (a path of one node), unless it is a start, a socket, or an ascendancy node, or the jewel takes only keystones and it is not one (PoB's `intuitiveLeapLikesAffecting`). Paths to other nodes are still found without leaps. Nodes a leap holds stay through deallocations, as PoB keeps them when it loads a build (`radius::held`, until nothing changes: a held socket's jewel holds too).
  - **Radii** are the game's `PassiveJewelRadii` (bundle `poe1/jewel_radii.json`, optional): a jewel's `Radius:` line names a row, and a ring jewel's ring value `k` is the table's `k`-th row's ring. The data-parity table `jewel_radii` checks both against PoB's `data.jewelRadius` (PoB export format 7). A bundle without the table has no radius, so no leap: the bundles on the maintainer's workstation predate it, and the next extraction run adds it (handoff, Next action).
- `poe1::url`: PoB's `EncodeURL` (version 6). Nodes are written in ascending order, while PoB's order is hash order; the site reads either. Cluster node numbers are written less 65536. Masteries are written as effect then node. Starts are left out. When a list exceeds 255 entries, the spec has no URL.

Tests: a unit test per command on a small tree, each with a kept node and a node the tree lacks, tests for a node not connected on import and an ascendancy path node, and the ring, a leap, and held nodes (`src/poe1/tests.rs`). On the synthetic tree bundle (T1a, in CI): every kind placed and modelled, allocation across the tree with a mastery and a multiple choice, the large cluster socket plain and the nested ones cluster sockets, and a radius jewel allocating a node near its socket alone (`tests/synthetic_tree.rs`). Against the real 3_29 tree, maintainer-run: `tests/real_bundle.rs`.

**The synthetic tree bundle** (`crates/loom-data/tests/fixtures/bundle-poe1-tree`, written by `make_bundles.py`'s `tree_bundle`): the CI fixture bundle with a larger made-up tree: Test Brute (two ascendancies) and Test Sage (one), each ascendancy with a multiple choice; three rows of nine groups, each a ring of six nodes, with notables, two masteries, a keystone, and two jewel sockets; a large cluster socket with a nested medium and small socket and their proxies; the layout constants, two jewel radii, and a made-up sprite sheet with its index. Every value is made up; only the shapes follow the game's.

## Calculation (`loom_pipeline`, T2)

The calculation reads the passives through a resolved set, `CalcOptions.passives` (`loom_calc::ResolvedPassives`), made by `loom_pipeline::resolve_passives` from the tree: each allocated node is connected (to the class start, `loom_tree::poe1::connected_to_start`, or held by a radius jewel), unconnected, a cluster socket, or a cluster jewel's node. The calculation counts connected nodes and masteries, and keeps the reasons it gave before (`unconnected-node`, `cluster-jewel-node`). `loom-calc` no longer reads passive links (its `passive_links` input is gone). `loom_pipeline::leaps` finds the build's leaping jewels in the spec's sockets by their stats: `local_unique_jewel_nearby_disconnected_passives_can_be_allocated` (and its keystones-only form) with the item's `Radius:` line, and `local_unique_jewel_disconnected_passives_can_be_allocated_around_keystone_hash` with `local_jewel_variable_ring_radius_value` for Impossible Escape. PoB's older one-line wording ("Passives in Radius can be Allocated…") does not resolve and stays an `unresolved-line` reason (ADR-015). `calc-run` and the app both pass the tree.

Evidence: on the latest rules bundle (no radii), the 133 public fixtures and the synthetic build give byte-identical documents before and after T2 and again after T-f. With PoB's 3_16 radii typed into a local, never committed copy of the layout bundle, only the community Essence Drain build changes: three nodes (27137, 36859, 49698) are held by its jewel, as PoB's read-back keeps them in the active spec ("ALTERNATIVE SETUP"). Its fields change (energy shield 2648 to 2761, armour 2092 to 2338, block 28 to 42); PoB's totals do not confirm them, because the build has other content the calculation does not model, so no field record is claimed.

## Application (`loom_app::tree`)

The spec's class and ascendancy are PoB's `classId` and `ascendClassId` attributes, typed by the domain's accessors (`PassiveSpec::class`, `ascendancy`, `secondary_ascendancy`, and their setters; plan T-h) over the attributes they are kept in and edited in place, so an unedited spec round-trips byte for byte and the build document is unchanged. An edited spec gets:
- its nodes and mastery effects written in ascending order, starts included, as PoB writes them;
- `classId` and `ascendClassId` set;
- its `<URL>` child, when it has one, regenerated inside the old text's whitespace, or removed when the format cannot hold the spec;
- when it is the active spec, the build's `className` and `ascendClassName` set to match (`None` for no ascendancy), as PoB's `Load` expects.

An unedited spec is not touched, so Phase 3's documents and the round-trip gate are unchanged.

When the spec has no `classId` (or no `ascendClassId`) and is the active spec, the class (or ascendancy) is taken from the build's names, as PoB's `Load` does.

**A spec is shown, not edited** (`TreeView.locked` gives the reason, and every command gives `APP_TREE`) when:
- its `treeVersion` and the loaded data's tree version are both known and differ. Converting versions is T4 and T5b. When the data's version is not known (a bundle `TREE_VERSIONS` lacks), the spec is edited, and the calculation reports `unchecked-tree-version` as before;
- its class is not one of the tree's.
The screen then shows the tree with a note and offers no edits.

**`socket-jewel`** puts a jewel of the build in an allocated jewel socket of the tree, or empties the socket. The item must be a `Jewel` or `AbyssJewel` by its base's item class, and not a cluster jewel: cluster jewels' sockets and generated nodes come with T3 and T5b. The spec's `<Sockets>` list gets the socket if it lacks it. Removing a socket's node leaves its jewel entry, as PoB does.

| API v1 (additive) | |
|---|---|
| Commands | `allocate-node`, `deallocate-node`, `set-mastery-effect`, `select-class`, `select-ascendancy`, `reset-tree`, `socket-jewel`, each with the spec's ID. Each is one undoable change (history kind `tree`); a refused edit gives `APP_TREE`. |
| Spec commands | `new-spec` (a copy of a spec, or a new one on the data's tree version with the active spec's class, ascendancy, and bloodline and only the starts), `rename-spec`, `delete-spec` (never the last; later specs are renumbered, and when the active spec goes, the one before it becomes active). None changes the active spec otherwise. History kind `tree-specs`. |
| `BuildView.tree` | the active spec as the editor sees it: class, ascendancy, nodes, masteries, socketed jewels, kept nodes, points, and `locked` |
| `TreeNodeView.cluster_area` | the node is a cluster area: not drawn, not listed |
| Shell `tree_layout` | the loaded data's tree: groups, nodes (kind, position, links, stat lines printed from `passive_skill_stat_descriptions`), classes, mastery effects. Made once per data load. |
| Shell `preview_tree` | what clicking a node would do: the path or the removed nodes, and each slice field before and after. Calculated off the writer, never committed. |

`GameData` builds the tree and its layout DTO when the bundle loads. A stat no description prints is shown as its ID and value (ADR-015), except one the game declares without a description.

Tests: `crates/loom-app/tests/tree_workflow.rs`. On the CI fixture bundle, every command is checked with undo back to the import byte for byte, along with the refusals for another version and an unknown class, the class and ascendancy taken from the build's names, the unknown node kept, the URL regenerated, the build's names following the spec, a preview not committing, and the export reading back. Against the real bundle, maintainer-run: the layout is placed and every line is printed.

## MCP (plan A-b, ADR-017)

Five tools join the contract (`schemas/mcp-tools.v1.json`, an additive diff):

| Tool | |
|---|---|
| `get_tree` | the active spec (`TreeView`), each allocated node's name and kind, the tree version, and the build's specs |
| `find_tree_nodes` | nodes whose name or lines contain a text, optionally of one kind; ID, name, kind, ascendancy, lines, allocated; at most 100 |
| `get_tree_node` | one node, whether it is allocated, its neighbours, and a mastery's effects and chosen effect |
| `preview_tree_change` | a `TreeChange`'s path or removed nodes and each field before and after, never committed |
| `request_tree_change` | a `TreeChange` through the approval flow, as `request_item_edit`: refused unless the user allows requests, waiting for approval unless there is a standing grant, polled with `get_request` |

`TreeChange` (app API v1, additive) is `allocate-node`, `deallocate-node`, or `set-mastery-effect`, each with the spec's ID, as the commands of those names. A change the editor refuses fails at once with `APP_TREE` and is not queued. `ChangeRequest` gains `tree` and `tree-preview`, and its `edit` becomes nullable (old documents stay valid). An applied tree request is one undoable step of history kind `tree-request-applied`. The AI connection screen shows a tree request by node name: the path it allocates, the nodes it removes, or the mastery's effect. Tests: `crates/loom-integrations/tests/mcp_contract.rs` (`tree_tools_should_read_the_tree_and_request_changes_the_user_approves`) and `ui/src/test/tree.test.tsx`.

## Screen (`ui/src/screens/Tree.tsx`, ADR-008 Amendment 1)

- **Sprites** (T-a, ADR-005 Amendment 2): when the bundle carries the export's sprite sheets, the canvas draws each passive's icon and frame (allocated or not), the jewel sockets' frames, and the masteries' icons (unconnected, connected when their group has an allocated node, chosen), at the export's zoom (a sprite covers its pixel size divided by the zoom in tree units). The preview's path and removals and search matches are rings over the art. Class and ascendancy starts stay our shapes. The shell's `tree_sprites` gives the sheets as `data:` URLs (the CSP already allows `data:` images; nothing is fetched); without sprites it answers `APP_NO_DATA` and the canvas draws our shapes. Screenshots with GGG's art are not committed.
- **Canvas**, without sprites drawn in our own shapes: circles for passives, a ring for notables and keystones, octagons for keystones and class starts, diamonds for sockets, a hollow circle for masteries. Allocated nodes and links are in the accent colour. Hovering shows the allocation path in blue and the nodes a removal would drop in red. Kept nodes are faded. Search matches are ringed. Arcs are drawn along an orbit when both ends share a group and an orbit. Masteries' links and class-to-ascendancy links are not drawn, as in PoB. You can drag to pan, zoom with the wheel or the buttons (zooming keeps the point under the pointer still), and Fit the whole tree. The colours are tokens (`--c-tree-*`).
- **Hover card** (DOM): the node's name, kind, and lines; a mastery's chosen effect; how many nodes a click allocates or removes; and the fields that change (the application's preview).
- **Click** allocates the node and its path or removes the node. A mastery opens its effect picker, where effects taken by a mastery of the same kind are disabled and the mastery can be removed. An allocated jewel socket opens its jewel picker: the build's jewels (cluster jewels aside), Empty socket, and Remove socket.
- **Spec bar**: New tree, Copy tree, Rename tree, and Delete tree (which asks first).
- **Bar**: class and ascendancy pickers (a class change asks first when nodes would be lost), points, search with a count, Tree or List view, and Reset (which asks first). A note counts the kept nodes.
- **List view**, the non-visual tree, does everything the canvas does: allocated, search results, keystones, notables, masteries, and the chosen ascendancy, each node with its lines and an Allocate, Remove, Choose-an-effect, or Choose-jewel button. It is the only view when the bundle has no layout.

The fake API carries a small synthetic tree with the same rules (`ui/src/api/fakeTree.ts`) and no sprites. `ui/src/test/tree.test.tsx` covers allocation, removal with orphans, undo, the mastery picker, the jewel picker, the ascendancy, the class change and its question, reset, search, a locked spec, the spec commands, and axe. The real-app workflow (`ui/e2e/workflow.mjs`, CI's `e2e` job) runs on the synthetic tree bundle: it imports the synthetic build as its Test Brute, allocates a notable from the List view, and checks the exported spec has it.

## Seen in the running application (2026-11-15)

The real app (`loom-tauri` with `e2e`, WebView2, 1440 x 900), the bundle `local/tree/bundle-poe1-tree-a`, and the community build Ice Nova Elementalist (campaign, lab 1): the whole tree fits and draws in our shapes, with the allocated path in the accent colour, the ascendancies outside the main tree at GGG's positions, and the kept groups (bloodlines) faded ([screens/01-tree-fit.png](screens/01-tree-fit.png)). Zoomed in, the arcs follow their orbits, and the hover card shows a node's name, kind, lines, and, for a kept node, that it is kept ([screens/02-tree-hover.png](screens/02-tree-hover.png): "Worship the Blightheart", a keystone the editor does not model, which GGG's layout places just below the Witch's start). At these zooms the class start read as a filled dot; it now keeps a minimum size and an outline, so it reads as an octagon.

With the sprites (2026-11-15, a bundle made by `tree-layout` from the latest rules bundle, `local/tree/bundle-poe1-tree-b`): the tree draws with GGG's icons and frames, masteries and sockets included, the allocated path in the export's allocated art, and the class start in our octagon. Screenshots are not committed (they show GGG's art).

## Not in this step

Below the plan's cut line (maintainer 2026-11-14: they may become their own PRs), each needing data this workstation does not have:
- **Cluster jewels** (T3, T5b; and T7's cluster hash v2): the expansion tables (`PassiveJewelSlots`, `PassiveTreeExpansionJewels`, `...Skills`, `...SpecialSkills`) need an extraction run (the extractor already roots `PassiveJewelSlots`), then the subgraphs, the hash formats, and the calculation. `socket-jewel` refuses cluster jewels.
- **Older tree versions** and conversion (T4, T5b): packs 3_25 to 3_28 from the export's older tags, converted with node text resolved to stat IDs.
- **Real radii**: the next extraction run adds `PassiveJewelRadii`; until then leaps apply only in bundles that have it (the synthetic tree bundle).
- **Timeless jewels and the other radius jewels** (transforming nodes, counting attributes): not modelled; their nodes are allocated as usual.
- **The corpus run and the tree report** (T8): the corpus is not on this workstation. The 3_29 records for layout, allocation, and read-back are in [parity.md](parity.md).
