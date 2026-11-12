# Phase 3: character, attributes, life, mana, energy shield (P3-6)

Plan step 6 (`dev/active/phase-3-calc-slice/phase-3-calc-slice-plan.md`): the engine's first computed fields. The mechanic fixtures match PoB v2.67.2 on every target and every breakdown PoB writes. Decisions: context file, "Decisions in P3-6".

## What is computed

Eight of the 24 slice fields: `Str`, `Dex`, `Int`, `Life`, `Mana`, `EnergyShield`, `LifeUnreserved`, `ManaUnreserved`. The other 16 stay `not-computed`. The formulas follow PoB's (`crates/loom-calc/src/poe1/phases.rs`, with each source):

| Field | Formula (PoB) | Source |
|---|---|---|
| Attributes | `max(round(base x ((1 + inc/100) x more)), 0)` | `Modules/CalcPerform.lua`, attributes pass |
| Life | base `38 + life_per_level x level + floor(Str/2)`; `max(round(base x (1 + inc/100) x more), 1)` | `CalcSetup.lua`, `CalcPerform.lua`, `CalcDefence.lua` |
| Mana | base `34 + mana_per_level x level + floor(Int/2)`; `round(base x ((1 + inc/100) x more))` | same |
| Energy shield | rows summed; the global row `base x ((1 + inc/100) x more)` only when its base is above 0, with `floor(Int/10)`% increased; `max(round(total), 0)` | `CalcDefence.lua` |
| Unreserved | the pool, when above 0 (nothing is reserved until gems, P3-9a, P3-12) | `CalcPerform.lua`, reservation |

The level is clamped to 1..100. Class base values come from the characters table; per-level growth comes from `metadata/characters/character`. The breakdown terms are the graph nodes `slice-fields.json` revision 2 names (`base`, `increased`, `more`, `result`; ES rows `step.slot.global`).

## What is collected, and what is a reason

`poe1/collect.rs` collects the class and level, the active spec's allocated and connected nodes, the mastery effects of connected masteries, the active item set's slots of the weapon set in use, and jewels in allocated sockets. The 24 stat mappings in `poe1/mapping.rs` cover life, mana and ES (flat and increased), global defences, and attributes as items and the passive tree write them. Everything else is a reason on every computed field, never a silent value:

| Reason | When |
|---|---|
| `unmapped-stat` | a stat with no mapping row |
| `unmodelled-gem`, `unmodelled-config`, `unmodelled-choice` | an enabled gem; any config input; a bandit or pantheon choice other than `None` |
| `unmodelled-item` | an armour piece's or weapon's base properties (P3-7, P3-10), an active flask, an abyssal socket, an unknown slot |
| `unmodelled-passive` | a cluster jewel node (PoB's subgraph ids have bit 16 set; legacy ids are `expansion` sockets), or a node not connected to a start (PoB drops it on load; jewels that allocate without a path, such as Impossible Escape, are not modelled yet) |
| `unread-line` | an item line the import did not read |
| `unchecked-tree-version` | passives allocated and no data tree version given |
| `missing-data` (indeterminate) | a class, constant, passive, or mastery effect the data lacks |
| `tree-version` (indeterminate) | the build's tree version is not the data's (PoB converts old trees on load; we do not) |

`calc-run --scope docs/phase-3/calc-scope-poe1.json` gives the data's tree version (`3_29` for Steam build 26530219); the bundle carries no label.

## Mechanic fixtures

19 original builds in `tools/oracle/fixtures/mechanic/`, tier `mechanic`, each with the 8 computed fields as `targets`: a bare level 1 and level 90 character of each class; a Witch with passives (flat ES and mana, Intelligence, increased life and mana); a Scion with life and Strength from a passive; a Witch with Utmost Intellect and its group's Attributes mastery; a Witch with two rings, an amulet and a belt; a Witch with a Cobalt Jewel in an allocated socket. PoB's values (equal to ours on every field):

| Fixture | Str | Dex | Int | Life | Mana | ES |
|---|---|---|---|---|---|---|
| `bare-scion-level1` | 20 | 20 | 20 | 60 | 50 | 0 |
| `bare-scion-level90` | 20 | 20 | 20 | 1128 | 584 | 0 |
| `bare-marauder-level1` | 32 | 14 | 14 | 66 | 47 | 0 |
| `bare-marauder-level90` | 32 | 14 | 14 | 1134 | 581 | 0 |
| `bare-ranger-level1` | 14 | 32 | 14 | 57 | 47 | 0 |
| `bare-ranger-level90` | 14 | 32 | 14 | 1125 | 581 | 0 |
| `bare-witch-level1` | 14 | 14 | 32 | 57 | 56 | 0 |
| `bare-witch-level90` | 14 | 14 | 32 | 1125 | 590 | 0 |
| `bare-duelist-level1` | 23 | 23 | 14 | 61 | 47 | 0 |
| `bare-duelist-level90` | 23 | 23 | 14 | 1129 | 581 | 0 |
| `bare-templar-level1` | 23 | 14 | 23 | 61 | 51 | 0 |
| `bare-templar-level90` | 23 | 14 | 23 | 1129 | 585 | 0 |
| `bare-shadow-level1` | 14 | 23 | 23 | 57 | 51 | 0 |
| `bare-shadow-level90` | 14 | 23 | 23 | 1125 | 585 | 0 |
| `passives-witch` | 14 | 14 | 82 | 1271 | 781 | 37 |
| `passives-scion` | 25 | 20 | 20 | 1142 | 584 | 0 |
| `masteries-witch` | 15 | 15 | 183 | 1125 | 681 | 17 |
| `jewellery-witch` | 76 | 44 | 87 | 1281 | 642 | 36 |
| `jewel-witch` | 14 | 24 | 96 | 1125 | 689 | 16 |

Found while making them, both on the fixture side:
- A mastery whose group's notable was not allocated: PoB dropped it on load (30 life lower). The game does not allow it either. The fixture was corrected, and the engine now checks connectivity (above).
- The Moonstone Ring's implicit is cast speed in both the game data and PoB's; an older unique keeps an ES implicit. The fixture uses a Paua Ring instead.

## Evidence (2026-11-12, Windows)

Inputs: bundle `local/p3/p3-1c/evidence2/bundle-poe1-a` (Steam build 26530219, format 2); overlay `local/p3/p3-2/run4/overlay.json`; scope `docs/phase-3/calc-scope-poe1.json`. Outputs in `local/p3/p3-6/evidence/`, local; summaries in `docs/phase-3/character-fields/`.

| Run | Builds | Result |
|---|---|---|
| `oracle-run`, fixture manifest (clean `c83c905`) | 56 | Exit 0: every expectation met, 3/3 each |
| `calc-run`, public (fixtures, community, PoB's 3.13 builds), twice (clean `2b072a3`) | 56 | 55 documents, `malformed.xml` `IMPORT_XML` at `read` (exit 1, as expected); 168 fields complete, 32 partial, 240 indeterminate, 880 not computed; the two runs' documents and `report.json` byte-identical |
| `calc-run`, corpus, `--anonymous`, twice | 21 | 21 documents, all `indeterminate`: every corpus build uses an older tree (3_15 to 3_28); byte-identical |
| `calc-run --data-overlay`, public and corpus | 56, 21 | The overlay has no row these fields read (`characters`, `character_constants`, passive stats), so the same values |
| `calc-parity`, public, with the overlay run and the manifest | 56 | **No difference.** 168 fields match (the 19 mechanic fixtures and the 2 synthetic level 1 Scions, 8 each); breakdowns 24 match term by term, 39 absent where PoB writes none (no increased and no more); every mechanic target met (19 x 8). Exit 5 (`incomplete`): the 34 real builds have no complete field (gems, config, item properties), so nothing of theirs is compared |
| `calc-parity`, corpus, against P3-5's protocol-4 oracle run (adapter unchanged) | 21 | No difference; exit 5 (`incomplete`, every build indeterminate) |

Before the cluster-socket rule, every 3_29 public build with medium cluster jewels had its sockets flagged `unconnected-node`. Now one public build on the data's tree has unconnected nodes: three notables from Impossible Escape, which is not modelled.

Tests: `crates/loom-calc/tests/poe1.rs` (21: each class value, level clamp, passives, masteries, rings, jewels, connectivity, cluster nodes, unmapped stats, tree version, each reason kind, breakdown terms, an empty mana pool, determinism); `poe1/mapping.rs` (a test per row); `poe1/phases.rs` (`floor(n x 0.1)` equals integer division up to 100,000); `calc_run` (overlay reads, refusal of rows it cannot apply, an overlay run calculating with PoB's values); the manifest's mechanic count. CI's `round-trip` job checks the synthetic build's counts (16 not computed, 8 indeterminate: the synthetic bundle has no Scion).
