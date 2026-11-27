# Older tree versions (tree packs)

Passive tree plan (`dev/active/passive-tree-and-phase-5/`), step T4 and decision T-d. Built on branch `feat/tree-remaining` at the maintainer's request (2026-11-16: "complete tasks all in a single pr one by one until we have gotten through the remaining plan items").

A bundle carries the game's own tree in its passive tables. A build saved on an older tree (PoB's `treeVersion`, such as `3_28`) was calculated on that own tree before this step, with the reason `tree-version`, because a node's stats can change between versions. Now the bundle can also carry older versions as **tree packs**, and the calculation uses the spec's own version when the bundle has it.

## The pack

`trees/<version>.json` in the bundle (`loom_data::bundle::tree_packs`). It is optional, and the manifest lists each pack. A pack holds what changes between versions:

- the passives, with their stats as stat IDs;
- the graph: groups, nodes, links, and class starts;
- the masteries and their effects;
- the lines that did not read as stats (`unresolved`), kept as text with a reason (`no-template`, `ambiguous`, or `unsettled`);
- the export it came from: repository, tag, commit, and the SHA-256 of `data.json`.

Classes, ascendancies, object templates, the layout, jewel radii, and cluster jewel data are the bundle's own. The validator checks each pack:
- its version must be its file's (`BUNDLE_VALUE_RANGE`);
- every stat, ascendancy, character, mastery effect, linked node, and root it names must exist (`BUNDLE_DANGLING_REF`);
- node numbers must be unique (`BUNDLE_DUPLICATE_ID`).

## Making one (maintainer-run)

`tools/tree/manifest.json` is now format 2. `export` is still the bundle's own version (3.29.1, read by `tree-layout`). The new `versions` block pins one export commit for each older version:

| Version | Tag | Commit |
|---|---|---|
| 3_25 | 3.25.3d | `d355f6a16aadfa5d73e3b9a31ca9d7346fa84673` |
| 3_26 | 3.26.0 | `9785205dca9cb617ea5a7d77faefd3608693ce77` (the annotated tag's commit) |
| 3_27 | 3.27.0g | `765dda38b9dd21ec28429fd81dd2d17ad6390623` |
| 3_28 | 3.28.0 | `648e492b17d49d7213c63f4c1e0aa561617d5ad1` |

Each pin is the latest tag of its minor version on `grindinggear/skilltree-export`, as of 2026-11-16.

```sh
sh tools/tree/acquire.sh 3_28          # or: acquire.ps1 -Version 3_28; into local/tree-export-3_28/
loom-parity-cli tree-pack tools/tree/manifest.json 3_28 local/tree-export-3_28 <bundle-dir> --out <new-dir>
```

`tree-pack` writes a copy of the bundle with the pack added and the manifest updated, through the same copy step as `tree-layout`. It refuses to run in these cases:
- the checkout is not at the pinned commit;
- the export's orbits differ from the bundle's;
- the bundle does not validate before, or the copy does not validate after;
- more than 1% of the export's lines did not read (`MAX_UNREAD_PERCENT`; see "The 1% rule").

Run it once per version, each time on the previous run's output (3_28 first, then 3_27, 3_26, 3_25). A bundle with packs is signed like any other bundle. Packs are GGG data, so they live only in bundles and are never committed (ADR-005 Amendment 2).

## How the export is read

`tools/parity-cli/src/tree_pack.rs`:

- **Stats.** If the bundle has the same node and its stats print as the export's lines (through the bundle's `passive_skill_stat_descriptions`, in any order), the pack takes the bundle's stats unchanged. Otherwise `loom-parser` reads each line as stat IDs (ADR-015). Of several readings, the converter keeps the one whose stats are all the bundle's stats for the same node; failing that, the one whose stats the bundle's passives, mastery effects, or templates use. (The first rule reads "+20 to Dexterity and Intelligence" on 3.28's Versatility, which two descriptions print.) A line that does not read alone is read in its current wording when the review accepts one ("Rewordings"). A line is kept as text, never guessed, in these cases:
  - no description matches;
  - two readings remain;
  - a description has another variant with different values;
  - a shown value is not one number.
- **Passive points.** A node's passive points are its `skill_points`, not a stat. The export prints them as a line ("Grants 1 Passive Skill Point"), and that line is read into the field.
- **IDs.**
  - A node keeps the bundle's passive ID when the bundle has the same node number. Otherwise its ID is `export<node>`.
  - A node's `expansion` flag is the bundle's when the bundle has the node. Otherwise it is derived: the node is a cluster socket, a position proxy, or has no group.
  - A mastery keeps the bundle's group ID, with the export's effect list.
  - Effects keep the bundle's IDs by hash. Otherwise their ID is `export<hash>`.
- **Ascendancies.** The export's `ascendancyName` is an ID: Raider is the ascendancy shown as Warden. The converter matches it against the bundle's IDs. The bundle has no row for alternate and bloodline ascendancies (Warlock, Primalist, the old Warden), so their nodes get no ascendancy, as in the bundle's own passives. The tree screen draws them faded and never allocates them (T-g).
- **Graph.** Groups follow their numbers' order. Links come from the export's `out` lists, and the root's `out` list gives the class starts.

## Rewordings

3.29 reworded lines that older trees print. For example, "15% increased Global Accuracy Rating" is now "15% increased Accuracy Rating". The bundle's descriptions have only the current wording, so the old one does not read.

`tree-pack` learns rewordings from GGG's own data. Take a node or mastery effect that both the export and the bundle have. If its lines differ by exactly one line each way, and that pair has the same numbers, the pair is a candidate: old template to current template, with numbers as `#`. A template mapped two ways is dropped.

Same numbers do not prove the same stat. GGG sometimes changes a node's mechanic and keeps its values. The candidates for 3_25 to 3_28 included "#% increased Damage over Time" to "#% increased Spell Damage", and "#% reduced Mana Cost of Curse Skills" to "#% increased Mana Cost Efficiency of Curse Skills". So a candidate is applied only when the reviewed table `tools/tree/rewordings.json` accepts it. The table sits beside the manifest and has two lists:

- **Accepted** (12): changes of wording alone. These are "Global" dropped (five lines), a capital, a typo ("Lighting"), "Your" added, "also" added, "a maximum of" added, and one rephrased totem line.
- **Rejected** (13), each with its reason: another stat, a wider or narrower scope (critical strikes to all damage, every leech to life leech, elemental ailments to all ailments), or another condition ("no Endurance Charges" to "Minimum Endurance Charges").

A candidate in neither list is printed in the run's summary as `unreviewed`, and its lines stay unread until someone reviews it. The summary also lists the rewordings it applied (`reworded`).

A rewording applies only to a line that does not read alone. That includes one line of a longer description. 3.25 prints "Auras from your Skills have 8% increased Effect on you for" and "each Herald affecting you, up to 40%" as two lines of one description; once the second line is reworded, the two read together.

**Loose pairs** (the agent list, S9, 2026-11-26). A node whose lines differ in more than one line gives no one-to-one pair, so a rewording there was never learned: Counterweight's "+30% to Global Critical Strike Multiplier while wielding a Staff" sits beside its reworded Critical Strike Chance line. `tree-pack` now also offers every pair of a differing old line and a differing current line with the same numbers on such a node. A loose pair is applied only when the review accepts it, and is never listed as unreviewed: GGG's data does not say which current line an old one became. The review accepted one, "#% to Global Critical Strike Multiplier while wielding a Staff" to "#% to Critical Strike Multiplier while wielding a Staff" ("Global" dropped; four staff nodes, 4977, 22702, 39761, 53957). Each version then has four fewer unread lines (3_28: 30 to 26, 3_27: 38 to 34, 3_26: 44 to 40, 3_25: 50 to 46). **Approved by the maintainer 2026-11-26.**

The other loose pairs of 3_25 to 3_28, left unaccepted, are other mechanics, as their texts show. Examples: "#% chance to refresh Ignite Duration on Critical Strike" against "#% increased Ignite Duration on you", "Unaffected by Poison" against "Removes all Energy Shield", and the Amethyst against the Bismuth flask line. The pairs already rejected ("All Damage can Ignite", "All Damage can Shock", "Damage from your Critical Strikes cannot be Reflected") come up as loose pairs too and stay rejected.

**For the maintainer: the ignite and shock rejections.** They were rejected as "a narrower scope (hits only); may be another stat". Two facts bear on that. PoB v2.67.2 parses "All Damage can Ignite" and "All Damage with Hits can Ignite" to the same four flags (`ModParser.lua` 2794 to 2805, and the shock pair at 2812 to 2823). And the current stat's ID, `all_damage_can_ignite` on Shaper of Flames (node 53123), still reads the old wording. Accepting them would read Shaper of Flames and Shaper of Storms in full on 3_25 to 3_28. In the corpus, node 53123's unread line blocks 421 fields in 5 builds. It is a review decision, so it is not changed here.

**The packs and the current bundle.** The shipped packs were made on 2026-11-16 against `bundle-poe1-tree-a`. Converting them again against the current evidence bundle (`b-pb`, build 26530219, data schema 6), with the old table, reads about 20 more nodes per version in full: lines the earlier descriptions did not print, such as Shaper of Storms' "Always Shock". On the public fixtures that alone moves 6 values to PoB's exactly: the movement speed of three builds. In the corpus, 4 values move toward PoB and 2 away, both in one indeterminate build whose full DPS was already above PoB's; it gains an ignite its passives give. The next data run's packs are made this way (`tree-pack` against the run's bundle).

**Evidence** (the S9 agent, 2026-11-26): the four packs rebuilt chained, 3_28 to 3_25, against `b-pb`, once with the old table (`local/p6/packs/b-s9r`, run `local/p6/s9-r`) and once with the new code and table (`b-s9`, run `local/p6/s9-1`); `unreviewed` empty in all eight conversions. Against `b-s9r`, the new packs differ only in the four staff nodes.
- **Public:** 0 unexpected; `Complete` unchanged (49,371). Six values move, all toward PoB, all in `cyclone-shockwave-slayer.xml` (tree 3_28, Counterweight): its crit multiplier 3.42 to 4.02, equal to PoB's; its average damage, hit DPS, impale, combined and full DPS closer (full DPS 42.9 to 50.5 million, PoB 68.4 million; partial on its other reasons).
- **Corpus:** 0 unexpected; no document changed.
- **Tests:** `tree_pack::tests::loose_pairs_should_apply_only_when_accepted`, the loose case in `convert_should_read_old_wordings_only_as_the_review_accepts`, the pairs in `rewordings_should_pair_one_changed_line_with_equal_numbers`; the workspace tests, the real-bundle tests, clippy, rustfmt, and wasm32.

## The 1% rule

The plan's fallback (decision T-d) says: "A tree version with more than 1% of its node lines unresolved ships without that version; its builds stay `indeterminate` (`tree-version`)". `tree-pack` enforces it. It counts the export's node lines and its mastery effects' lines, each effect once. When more than 1% did not read, it refuses to write the pack.

`--accept-unread` writes the pack anyway, and the summary marks it `over_threshold`. That flag is for the maintainer, who decides on a version over the line.

## The four versions

These were made against the real bundle (`bundle-poe1-tree-a`, build 26530219), chained 3_28 to 3_25, on 2026-11-16. The result's manifest SHA-256 is `4814e3ac…`.

| Version | Passives | Mastery effects | Groups | Lines | Unread | Share | Rewordings applied |
|---|---|---|---|---|---|---|---|
| 3_28 | 3,337 | 353 | 776 | 5,376 | 30 | 0.56% | 6 |
| 3_27 | 3,280 | 353 | 748 | 5,284 | 38 | 0.72% | 6 |
| 3_26 | 3,152 | 354 | 718 | 5,107 | 44 | 0.86% | 6 |
| 3_25 | 3,135 | 354 | 715 | 5,093 | 50 | 0.98% | 7 |

All four ship under the 1% rule. None needs `--accept-unread`, and no candidate is unreviewed. The lines left are mechanics the current descriptions do not have, such as "Nearby Enemies have 10% reduced Cooldown Recovery Rate", plus the lines of the rejected rewordings.

How the count came down for 3_28:
- The first conversion left 1,088 lines unread.
- Taking the bundle's stats when they print the same lines, and keeping the reading whose stats the bundle uses, brought it to 103.
- Reading the passive-point line into its field brought it to 77. That is 1.43%, over the line; the other three versions were over too, at 1.59% to 1.87%.
- Using the node's own stats as the first tie-break, and applying the accepted rewordings, brought it to 30.

## In the calculation

The host picks the tree with `loom_pipeline::pick_tree`:
- If the active spec names another version than the data's and the bundle has that version's pack, the host uses the pack's tree (`Tree::from_pack`) and passes the pack's version as `CalcOptions::tree_version`.
- Otherwise the host uses the data's own tree.

The app (`GameData::trees`) and `calc-run` (`Request::trees`) build the pack trees once, with `loom_pipeline::pack_trees`. Each pack tree borrows the radii and cluster tables of the data's own tree, which the caller has already built.

`resolve_passives` puts the pack's stats in `ResolvedPassives::pack` (`PackStats`). With a pack, the engine reads nodes and mastery effects from it, not from the passives table. This applies everywhere the engine reads them: the passives, granted notables, their kind (notable or ascendancy, to choose between two of one name), and the Life Mastery check (`collect::node_stats`, `collect::node_kind`, `collect::effect_stats`; tested by `a_granted_passive_should_read_the_tree_packs_stats`). Each line the pack could not read gives its node the reason `unmodelled-passive` with `unresolved-tree-line`.

A spec of a version with no pack keeps the data's tree and the engine's `tree-version` reason, as before.

**The public builds on older trees** (T8). Twelve community fixtures save trees 3_25 to 3_28: one on 3_27, the rest on 3_28. Each ran through `calc-run` with the scope file, then was compared with PoB through `calc-parity`:

| | Without packs | With the four packs |
|---|---|---|
| Field states (288 fields) | 233 `indeterminate` (`tree-version`), 55 `unsupported` | 233 `partial`, 55 `unsupported` |
| Fields equal to PoB's value | 82 | 82 |
| Builds with an `unresolved-tree-line` reason | none | 5 |

Two values change, both on Shock Nova Hierophant (tree 3_27): mana and unreserved mana, from 6,919 to 6,905. Node 31819 is "5% increased maximum Mana" on 3.27, in both GGG's export and PoB's own 3_27 tree, and 6% on 3.29. So the pack's value is the right one for that tree. PoB's 8,373 differs for reasons outside the tree: the field is partial in both runs.

The tie-break mattered here. Before the node's own stats came first, rf-chieftain-league-start lost 20 Dexterity and 20 Intelligence on an ambiguous line. Its values now equal the run without packs.

## Parity with PoB

`tree-parity layout` and `tree-parity alloc` read the version from PoB's output and compare against the pack of that version when the bundle has one. `alloc-requests` takes the version too. `tools/oracle/data/pob_tree.lua alloc` takes `tree=<version>`, as `layout` already did.

All four packs match PoB, with no differences. The run used the chained bundle above, with PoB at the oracle pin. The records are `docs/tree/records/layout-<version>.json` and `alloc-<version>.json`, written by `TREES="3_25 3_26 3_27 3_28" tools/tree/parity.sh`. Details are in `docs/tree/parity.md`, "Older versions".

## Tests

- `crates/loom-pipeline/tests/tree_packs.rs`, on the synthetic tree bundle, whose `trees/3_28.json` is made up by `make_bundles.py`'s `tree_pack`:
  - the pack builds a tree with the bundle's classes;
  - a 3_28 spec is calculated on the pack with its unread line as a reason;
  - a spec of the data's version, or of a version with no pack, keeps the data's tree.
- `crates/loom-data/tests/bundle_validate.rs`: each bad reference in a pack gives its code.
- `tools/parity-cli/src/tree_pack.rs`:
  - the conversion of a made-up export, covering every rule above;
  - templates, and the learned rewordings: reordered lines, changed numbers, two changed lines, conflicts;
  - accepted, rejected, and unreviewed rewordings in a conversion;
  - the line count.
- `tools/parity-cli/src/tree_parity.rs`: the committed version records match and add up, as the 3_29 ones do.

## Not covered yet

- The older versions' own stat descriptions. Lines the rewordings do not cover stay reasons.
- Editing an older spec on its own tree. The editor converts it to the data's version instead (T5b; `docs/tree/editor.md`, "Converting a tree").
- Packs for versions before 3_25.
- The signed bundle with packs. The maintainer makes it, with the commands above and then the release's bundle signing.
