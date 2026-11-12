# Phase 3: spell hits (P3-9b)

Plan step 9b (`dev/active/phase-3-calc-slice/phase-3-calc-slice-plan.md`): the main skill's hit when it is a self-cast spell, against the enemy of step 11. The chosen spell is Ice Nova (Q2). Decisions: context file, "Decisions in P3-9b".

## What is computed (`crates/loom-calc/src/poe1/hit.rs`)

Five slice fields, with PoB v2.67.2's formulas, order of operations, and rounding points (`Modules/CalcOffence.lua`, `calcs.offence`; each step cited in the module):

| Field (PoB) | Formula |
|---|---|
| `AverageDamage` | per damage type (physical, lightning, cold, fire, chaos): base `skill x base multiplier + added x damage effectiveness`; `round(base x (1 + inc / 100) x more)`; each pass (non-crit, crit times the multiplier) `min / 2 + max / 2` times `1 - (resistance - penetration) / 100`; `hit x (1 - crit / 100) + crit hit x crit / 100`, times the hit chance (100) / 100 |
| `CritChance` | `min((skill base + added) x (1 + inc / 100) x more, 100)`, at least 0, times the hit chance / 100; the skill's base is PoB's `critChance` (P3-2's rule) |
| `CritMultiplier` | `1 + max(0, round(sum / 100, 2))`, the sum from the character template's 150 less 100 and the stats |
| `Speed` | `min(1 / (cast time / round((1 + inc / 100) x more, 2)), 1 / 0.033)` |
| `TotalDPS` (`HitDps`) | `AverageDamage x Speed x 1 x 1` (PoB's DPS and quantity multipliers) |

- **More multipliers** follow PoB's rule (`ModList:MoreInternal`): per list (the skill's gems, then the character) and per mod name, the product rounded to two decimals. Found by the first run: Controlled Destruction's 80% less crit gave 1.4999999999999993 against PoB's 1.5.
- **The enemy's resistance:** its configured or placeholder value (P3-11), capped at its maximum, 75 (the monster template's `base_maximum_all_resistances_%`) or the configured resistance when above it, at most 90, and at least -200; less the hit's penetration.
- **Which skills:** a spell and a hit (`Damage` or `Projectile`), not an attack, channelled, triggered, a totem, trap, mine, minion, or brand, without a cooldown, with a cast time, in an enabled group. Every other main skill leaves the five fields not computed. A spell other than Ice Nova (`hit::VETTED_SPELLS`, the spells the oracle has checked) is computed but `Partial` (`unmodelled-skill`, `pob-skill-data`): PoB also computes a skill from its own skill data (parts, cast time overrides, skill functions, hand-written flags), which the game data does not carry; a later PR adds a spell with its fixtures. `HitChance` (attacks) and `HitSpeed` (absent for a self-cast spell in PoB) stay not computed.
- **Base damage** is the game's resolved value (`BaseResolvedValues`), not PoB's recomputation from the level requirement (below).
- **The main spell's gems** (its active gem and the supports that apply) act through their stats on the skill alone; a gem stat with any other effect is unmapped. Every other enabled gem stays `unmodelled-gem` on every field.
- **Not modelled, each a reason:** conversion and gain (their stats are unmapped), luck, double damage, action speed, damage taken, the enemy's block, physical hits against armour (`unmodelled-skill`, `physical-hit-armour`), an `enemyIsBoss` input (`unmodelled-config`; the Uber boss takes 70% less damage). The default enemy's other modifiers (rare or unique, Pinnacle boss, warcry and enemy power, ailment threshold) reach no mapped stat: a test checks that no mapped row reads a named condition.

New in calc result v1 (additive rule; schema and TypeScript regenerated): graph nodes `difference` and `quotient` (PoB divides where a product with the reciprocal rounds differently), and the reason `unmodelled-skill`.

## Base damage: the game's resolved values

PoB recomputes a skill's effectiveness-scaled stats from the level requirement (`calcLib.buildSkillInstanceStats`); the game data stores the result (`BaseResolvedValues`), and the bundle does not carry level requirements. `tools/parity-cli/scripts/check-skill-resolved.py` compares the two on the P3-1d bundle and PoB v2.67.2's skill data (`docs/phase-3/spell-hits/resolved-vs-pob.json`):

| Rows (interpolation 3) | Equal | Different |
|---|---|---|
| 12,144 | 12,016 | 128 (79 skills) |

Every Ice Nova level is equal. The differences, mostly monster and minion skills: the game keeps a tiny value at 1 where PoB rounds to 0 (Smite levels 1 and 3, Added Lightning Damage level 1, Prismatic Burst level 1), values within 1e-5 of .5 the game rounds up (Tempest Shield level 7: 152.49999 gives the game 153, PoB 152), and large per-minute DoT values differing by 1. The engine takes the game's value (ADR-001); a fixture that reaches one of these rows gets a `pob-derivation` register entry (Smite levels 1 and 3: P3-10).

## Mechanic fixtures

11 new fixtures in `tools/oracle/fixtures/mechanic/` (original, a level 90 Witch unless named), each with the 17 character and defence fields and the 5 hit fields as targets:

| Fixture | Checks |
|---|---|
| `ice-nova-witch.xml` | Ice Nova 20 alone: base damage, crit, cast rate, the default enemy's 50% cold resistance |
| `ice-nova-level10-witch.xml` | level 40 Witch, Ice Nova 10 with 20% quality: another level's base damage; quality adds only area |
| `ice-nova-added-cold-witch.xml` | Added Cold Damage: added damage times damage effectiveness 2.5 |
| `ice-nova-controlled-destruction-witch.xml` | 39% more spell damage, 80% less crit chance (PoB's more rounding) |
| `ice-nova-increased-critical-strikes-witch.xml` | additional base crit (hundredths) and increased crit |
| `ice-nova-cold-penetration-witch.xml` | penetration of the enemy's resistance |
| `ice-nova-faster-casting-witch.xml` | increased cast speed (rounded modifier) |
| `ice-nova-increased-critical-damage-witch.xml` | crit multiplier |
| `ice-nova-added-damage-items-witch.xml` | jewellery: added cold damage to spells; increased spell, cold, elemental, and generic damage; cast speed |
| `ice-nova-crit-items-witch.xml` | jewellery: spell crit chance, global crit chance and multiplier |
| `ice-nova-enemy-witch.xml` | an enemy configured with 80% cold resistance (raising its maximum) and level 70 |

## Evidence (2026-11-12, Windows)

All runs from clean `a6b234b` on the P3-1d bundle (`local/p3/p3-1d/bundle-poe1-a`); outputs in `local/p3/p3-9b/evidence/`; summaries in `docs/phase-3/spell-hits/`.

| Run | Builds | Result |
|---|---|---|
| `oracle-run`, the fixture manifest with the 11 new fixtures (`local/p3/p3-9b/oracle-public`) | 71 | Exit 0: every fixture meets its expectation, 3/3 each |
| `calc-run`, public (fixtures, mechanic, community, PoB's 3.13 builds), twice | 71 | 70 documents, `malformed.xml` `IMPORT_XML` as expected (exit 1); 667 fields complete, 68 partial, 530 indeterminate, 415 not computed; the two runs' documents and `report.json` byte-identical |
| `calc-run --data-overlay` (`local/p3/p3-1d/parity-1/overlay.json`), public | 71 | The same field states (no overlay row reaches a hit field) |
| `calc-run`, corpus, `--anonymous`, twice | 21 | All `indeterminate` (older trees); byte-identical |
| `calc-parity`, public, with the overlay run, the register, and the manifest | 71 | **No difference.** 667 fields match (+242: the 11 new fixtures' 22 targets each); 266 breakdown terms match (`TotalDPS` among them), 69 absent where PoB writes none; **every mechanic target met (633 of 633, 34 fixtures)**. Exit 5 (`incomplete`), expected: 34 real builds have no complete field to compare |
| `calc-parity`, corpus, against P3-5's protocol-4 oracle run | 21 | No difference; exit 5 (all indeterminate) |
| `check-skill-resolved.py` (above) | 12,144 rows | 12,016 equal, every Ice Nova level among them |

The first run found one difference, Controlled Destruction's crit chance (1.4999999999999993 against PoB's 1.5): PoB rounds each more factor to two decimals, now modelled. A fixture line written "Critical Strike Chance for Spells" did not resolve (the game writes "Spell Critical Strike Chance"); the fixture uses the game's text.

**Known limit, not reached by any fixture:** the store sums effects in its canonical order (by source); PoB sums the skill's list, then the character's. Sums of several non-integer values (base crit in hundredths from more than one source) may differ in the last bit; the same holds within one more factor's product of three or more values, which the two-decimal rounding hides except near a half.

Tests (`crates/loom-calc/tests/hit.rs`, 12): a spell alone against the default enemy; added damage x effectiveness and more damage; PoB's more rounding; more and increased by mod name; penetration; a configured resistance raising the maximum and the 90 cap; only self-cast spells without cooldown; an unvetted spell `Partial`; the game's resolved value over PoB's recomputation; physical damage; other gems and unmapped gem stats as reasons; an enemy input of another type and missing constants. Mapping tests for every hit row kind and `SkillMapping`. 1091 workspace tests, clippy, fmt, wasm32; calc result v1 golden files, schema, and TypeScript regenerated.

Commands (`<out>` a new folder):

```sh
lpc=target/release/loom-parity-cli; b=local/p3/p3-1d/bundle-poe1-a; scope=docs/phase-3/calc-scope-poe1.json
public="tools/oracle/fixtures tools/oracle/fixtures/mechanic tools/oracle/fixtures/community local/oracle/pob/spec/TestBuilds/3.13"
$lpc oracle-run tools/oracle/manifest.json local/oracle --out <out>/oracle tools/oracle/fixtures/fixtures.json
$lpc calc-run $public --bundle $b --scope $scope --out <out>/public
$lpc calc-run $public --bundle $b --scope $scope --data-overlay local/p3/p3-1d/parity-1/overlay.json --out <out>/public-overlay
$lpc calc-parity <out>/oracle <out>/public --slice docs/phase-3/slice-fields.json --out <out>/parity --register docs/parity/divergence-register.json --overlay-run <out>/public-overlay --manifest tools/oracle/fixtures/fixtures.json
python -I tools/parity-cli/scripts/check-skill-resolved.py local/pob-v2.67.2/src/Data $b/data
```

## The in-scope builds

Ice Nova's three in-scope builds (`ice-nova-elementalist-*`, Q8's derived setups) have their hit fields `indeterminate`, never compared: items with unsettled values (`unsettled-value`), and partial reasons for unmapped stats, the config's many conditions and custom mods (`unmodelled-config`), Alira and the pantheon (`unmodelled-choice`), weapons (`unmodelled-item`), and the other gems (Vaal Ice Nova's second effect, auras, golems: `unmodelled-gem`). The Accept allows this ("or `Partial` with its reason"); step 12 (buffs and config) and step 8 (mappings) bring them closer. No difference is hidden: `calc-parity` compares only complete fields.

**The register:** empty. The 128 rows where the game's resolved value differs from PoB's are not reached by any compared fixture; the register records an observed difference per fixture and field (`docs/parity/README.md`), so the first fixture that reaches one adds a `game-truth` entry with the data row as evidence (Smite levels 1 and 3: P3-10 uses level 20).
