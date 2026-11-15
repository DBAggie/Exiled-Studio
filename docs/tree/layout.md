# Passive tree layout: evidence

Passive tree plan, T-b and T1 (`dev/active/passive-tree-and-phase-5/`); ADR-005 Amendment 2.

## What the layout is

A node's position is its group's centre plus its orbit's radius in the direction of its slot's angle, measured clockwise from up: `(x + r sin θ, y − r cos θ)`. The game files our extractor reads give the groups' centres and each node's orbit and slot (`data/passive_tree.json`). They do not give the radii or the angles. Those come from GGG's passive tree export and are carried in the bundle's optional file `data/passive_tree_layout.json` (`loom_data::bundle::tree_layout::TreeLayout`).

| Constant | Source |
|---|---|
| `orbit_radii` `[0, 82, 162, 335, 493, 662, 846]` | the export's `data.json`, `constants.orbitRadii` |
| slots per orbit `[1, 6, 16, 16, 40, 72, 72]` | the export's `constants.skillsPerOrbit`, which must equal the bundle's `orbit_slots` (the run refuses otherwise) |
| 16-slot angles | the export README's table (3.17.0 "Notable Changes"): every 30° and 45° |
| 40-slot angles | not documented by GGG; the same rule as the documented table: the multiples of 360°/(slots − 4) and the four diagonals, so every 10° and 45° |
| other orbits | evenly spaced |

The layout file records its source: the repository, the tag, the commit, the SHA-256 of `README.md` and `data.json`, and the angle rule in words.

## How it is made

```sh
powershell -ExecutionPolicy Bypass -File tools/tree/acquire.ps1     # or tools/tree/acquire.sh
cargo run --release -p loom-parity-cli -- tree-layout tools/tree/manifest.json local/tree-export <bundle> --out <new bundle>
```

`tools/tree/manifest.json` pins the export at tag `3.29.1`, commit `8bd138b32ea2631455cac5935bfab089f826094f`. `acquire` clones or fetches it into `local/tree-export/` and checks the commit. `tree-layout` refuses to run unless:
- the checkout's `HEAD` is the pinned commit;
- `skillsPerOrbit` equals the bundle's orbit slots, with one radius per orbit;
- the README's 16-slot table equals the rule's angles (`tree_layout::orbit_angles(16)`);
- the bundle validates before, and the copy validates after.

The output is a copy of the bundle with the layout file added and the manifest's file list updated. Nothing else changes, so no calculation changes. The file is optional (`BUNDLE_TREE_LAYOUT` checks it when listed). Without it, the tree connects, calculates, and is listed but not drawn. This deviates from the plan, which put the file in data schema 4 with a schema bump. An optional listed file needs no bump, and bundles made before it stay valid.

## Evidence (maintainer workstation, 2026-11-15)

Bundle: schema 4, Steam build 26530219 (tree version 3_29), copied with the layout to `local/tree/bundle-poe1-tree-a`. Layout file SHA-256 `edff8ff4…`, manifest `65952be5…`.

- **The export and the bundle agree.** Every one of the bundle's 2,987 nodes is in the export at 3.29.1 (3,389 nodes; the rest are nodes the game's graph does not carry), with the same group, orbit, and slot. The group centres are equal up to the export's rounding: the export writes them rounded, some to one decimal (up to 0.05 off), while the game's psg, which the bundle carries, has them in full. The bundle keeps the psg's values.
- **The angles equal PoB's.** PoB v2.67.2's `PassiveTree.lua` `CalcOrbitAngles` (lines 1015-1027) lists the same 16 angles (citing the same README) and the same 40 angles (0 to 350 by 10, with 45, 135, 225, 315), and spaces other orbits evenly. PoB reads the radii from the same export's constants. So loom's positions equal PoB's, with PoB used as a cross-check only.
- **The rules reach the whole tree.** `cargo test -p loom-tree --test real_bundle -- --ignored`: every node is placed. 2,732 nodes are modelled (48 keystones, 615 notables, 315 masteries, 21 jewel sockets) and 255 are kept but not edited, 78 of them cluster areas (the 36 nested cluster jewel sockets among them; [parity.md](parity.md)). Every keystone is reachable from each of the 7 classes, and deallocating a path's first step removes it all. Every ascendancy notable is reachable once its ascendancy is chosen.
- **The layout DTO prints every line.** `cargo test -p loom-app --test tree_workflow -- --ignored`: 4,272 node lines, each written from `passive_skill_stat_descriptions`. Eight stats have no description in the game data and are not declared hidden, so they are shown as their ID and value instead of being dropped (ADR-015): `has_trigger_when_ward_breaks`, `track_ward_broken_in_the_past_2_seconds`, `grants_a_crab_arm`, `unique_insanity_do_weird_things` (three nodes), `can_see_monster_type_beasts`, and `permanenet_mercenary_damage_+%_final_and_minion_damage_+%_final_from_luminary` (the game's spelling). The layout is 913 KB as JSON and is made once per data load.

The per-node comparison with PoB's own tree (plan gate 1) is in [parity.md](parity.md), with its record. Main-tree nodes are within 0.01 and every edge set is equal. Ascendancies differ by one offset each, because PoB moves them, and some nodes differ by the export's rounding.

## Sprites (T-a, 2026-11-15)

The same `tree-layout` run adds the tree's sprites (ADR-005 Amendment 2; `loom_data::bundle::tree_sprites`):
- **The sheets**: ten of the export's sprite categories at zoom `0.3835` (the fourth of five levels: the passives' icons allocated and not, the frames, which hold the jewel sockets' too, and the masteries' icons unconnected, connected, and chosen) are nine files, copied byte for byte from the export's `assets/` into `trees/assets/`, listed and hashed in the manifest. They are never committed; they ship only inside bundles.
- **The index** `data/passive_tree_sprites.json`, ours: each category's sprites (name, place on its sheet) as the export's `sprites` gives them for that zoom, and each tree node's icons from the export's `nodes`, for the nodes the bundle's tree has. Its source block adds each sheet's SHA-256 to the layout's. `BUNDLE_TREE_SPRITES` checks it when listed: every sheet listed, every sprite on its sheet, every node icon on a sheet, rows in order.

Evidence: from the latest rules bundle (Steam build 26530219) to `local/tree/bundle-poe1-tree-b`: 4.2 MB of sheets, 2,987 nodes with art; the bundle validates. The summary's `icons_without_sprite` is 315: the 315 modelled masteries' plain `icon`, which is on the export's `mastery` sheet, not copied. The canvas draws a mastery from its `inactiveIcon` and `activeIcon`, all of which the copied sheets have, so no drawn icon is missing (tree data run, 2026-11-15: the same 315 on the rules bundle and on the bundle with radii). In the running application the tree draws with the sprites ([editor.md](editor.md), "Seen in the running application").
