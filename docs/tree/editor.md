# Passive tree editor

Passive tree plan (`dev/active/passive-tree-and-phase-5/`), the cut-line steps on the bundle's own tree version: T1 (crate and layout), T2 (rules), T5a (app commands), T6 (screen), and T7 (export of edited specs). Built on branch `feat/ux-sprint-2` at the maintainer's request (2026-11-15: "Implement the passive skill tree"). Layout evidence: [layout.md](layout.md).

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
- `poe1::url`: PoB's `EncodeURL` (version 6). Nodes are written in ascending order, while PoB's order is hash order; the site reads either. Cluster node numbers are written less 65536. Masteries are written as effect then node. Starts are left out. When a list exceeds 255 entries, the spec has no URL.

Tests: a unit test per command on a small tree, each with a kept node and a node the tree lacks, and tests for a node not connected on import and an ascendancy path node (`src/poe1/tests.rs`). Against the real 3_29 tree, maintainer-run: `tests/real_bundle.rs`.

## Application (`loom_app::tree`)

The spec's class and ascendancy are PoB's `classId` and `ascendClassId` attributes. They stay untyped in the domain and are edited in place. This deviates from plan T-h, which types them. Untyped, they already round-trip byte for byte, and the editor needs no build document change. Typing them is left for the step that needs them (version conversion, T4 and T5b). An edited spec gets:
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

- **Canvas**, drawn in our own shapes (ADR-000): circles for passives, a ring for notables and keystones, octagons for keystones and class starts, diamonds for sockets, a hollow circle for masteries. Allocated nodes and links are in the accent colour. Hovering shows the allocation path in blue and the nodes a removal would drop in red. Kept nodes are faded. Search matches are ringed. Arcs are drawn along an orbit when both ends share a group and an orbit. Masteries' links and class-to-ascendancy links are not drawn, as in PoB. You can drag to pan, zoom with the wheel or the buttons (zooming keeps the point under the pointer still), and Fit the whole tree. The colours are tokens (`--c-tree-*`).
- **Hover card** (DOM): the node's name, kind, and lines; a mastery's chosen effect; how many nodes a click allocates or removes; and the fields that change (the application's preview).
- **Click** allocates the node and its path or removes the node. A mastery opens its effect picker, where effects taken by a mastery of the same kind are disabled and the mastery can be removed. An allocated jewel socket opens its jewel picker: the build's jewels (cluster jewels aside), Empty socket, and Remove socket.
- **Spec bar**: New tree, Copy tree, Rename tree, and Delete tree (which asks first).
- **Bar**: class and ascendancy pickers (a class change asks first when nodes would be lost), points, search with a count, Tree or List view, and Reset (which asks first). A note counts the kept nodes.
- **List view**, the non-visual tree, does everything the canvas does: allocated, search results, keystones, notables, masteries, and the chosen ascendancy, each node with its lines and an Allocate, Remove, Choose-an-effect, or Choose-jewel button. It is the only view when the bundle has no layout.

The fake API carries a small synthetic tree with the same rules (`ui/src/api/fakeTree.ts`). `ui/src/test/tree.test.tsx` covers allocation, removal with orphans, undo, the mastery picker, the jewel picker, the ascendancy, the class change and its question, reset, search, a locked spec, the spec commands, and axe.

## Seen in the running application (2026-11-15)

The real app (`loom-tauri` with `e2e`, WebView2, 1440 x 900), the bundle `local/tree/bundle-poe1-tree-a`, and the community build Ice Nova Elementalist (campaign, lab 1): the whole tree fits and draws in our shapes, with the allocated path in the accent colour, the ascendancies outside the main tree at GGG's positions, and the kept groups (bloodlines) faded ([screens/01-tree-fit.png](screens/01-tree-fit.png)). Zoomed in, the arcs follow their orbits, and the hover card shows a node's name, kind, lines, and, for a kept node, that it is kept ([screens/02-tree-hover.png](screens/02-tree-hover.png): "Worship the Blightheart", a keystone the editor does not model, which GGG's layout places just below the Witch's start). One cosmetic point: at these zooms the class start reads as a filled dot rather than an octagon.

## Not in this step

- **Sprites** (T-a): the canvas draws our own shapes, and the bundle does not carry the export's sprite sheets yet.
- **Cluster jewels** (T3), **older tree versions** and conversion (T4, T5b). `socket-jewel` refuses cluster jewels.
- **The calculation's resolved passive set** (T2's `loom-pipeline` part): the calculation reads the spec's nodes as before. Class starts now appear in edited specs, as PoB writes them, and they have no stats.
- **Intuitive Leap, Thread of Hope, Impossible Escape** radii (T-f): not applied. A node allocated through one stays through every edit but a reset, because no edit disconnects it, but the editor does not offer such nodes.
- **Gate records for other versions and cluster jewels.** The 3_29 records for layout, allocation, and read-back are in [parity.md](parity.md).
