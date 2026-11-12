# Phase 3: attack hits (P3-10)

Plan step 10 (`dev/active/phase-3-calc-slice/phase-3-calc-slice-plan.md`): the main skill's hit when it is an attack with the weapon in the main hand, against the enemy of step 11. The chosen attack is Smite (Q2), with PoB's two skill parts (Melee Hit, Area Hit). Decisions: context file, "Decisions in P3-10".

## What is computed (`crates/loom-calc/src/poe1/hit.rs`)

Six slice fields, with PoB v2.67.2's formulas, order of operations, and rounding points (`Modules/CalcOffence.lua`, `calcs.offence`; each step cited in the module):

| Field (PoB) | Formula |
|---|---|
| `HitChance` | accuracy `max(0, floor(base x (1 + inc / 100) x more))`, the base `accuracy_rating_per_level x (level - 1)` + 2 per dexterity + the stats; `max(min(round(accuracy / (accuracy + (evasion / 5) ^ 0.9) x 125), 100), 5)` against the enemy's evasion rounded (`calcs.hitChance`); the enemy blocks nothing modelled |
| `AverageDamage` | per damage type: base `weapon x base multiplier + added x damage effectiveness`; `round(base x (1 + inc / 100) x more + converted)`; each pass `min / 2 + max / 2` of the damage times the share the type keeps (times the crit multiplier on the crit pass), times the enemy's effect (resistance less penetration; for physical `1 - min(armour / (armour + hit x 5) x 100, 75) / 100`, per pass); `hit x (1 - crit / 100) + crit hit x crit / 100`, times the hit chance / 100 |
| `CritChance` | `min((weapon crit + added) x (1 + inc / 100) x more, 100)`, at least 0, times the hit chance / 100 |
| `CritMultiplier` | as for spells (P3-9b) |
| `Speed` | `min(1 / (1 / (weapon rate x (1 + attack speed multiplier / 100)) / round((1 + inc / 100) x more, 2)), 1 / 0.033)` |
| `TotalDPS` (`HitDps`) | `AverageDamage x Speed x 1 x 1` |

- **The weapon** (`Classes/Item.lua`, the weapon branch): its base's physical damage, attack time, and critical strike chance (`weapon_types`), its quality, and its local lines act on the weapon (`Poe1Quantity::Weapon`): physical `round((base + flat) x (1 + inc / 100) x (1 + quality / 100))`, elemental `round(flat x (1 + inc / 100))`, chaos the flat; attack rate `round(round(1000 / attack time, 2) x (1 + local attack speed / 100), 2)`; crit `round((base + local) x (1 + local inc / 100), 2)`. "+# to Accuracy Rating" on the weapon is the character's accuracy for its attacks.
- **Skill parts** are PoB's (`hit::SKILL_PARTS`; the game data has none): Smite's Melee Hit is melee, its Area Hit area and not melee; the gem's `skillPart`, default 1, at most the number of parts.
- **Strength** gives a melee hit's physical damage `floor(Str / 5)`% increased (`Modules/CalcPerform.lua`), also on the part converted to lightning.
- **Conversion**: the skill's own (Smite: 50% of physical to lightning). The converted damage takes both types' modifiers and is rounded before the target type's own damage is added; the physical part keeps half.
- **Smite's aura** adds 12 to 220 lightning damage at level 20 (the game's resolved values; PoB applies it as an aura buff, unscaled with no aura or buff effect stat, which are unmapped).
- **Which attacks:** an attack (not channelled, triggered, a totem, trap, mine, minion, or brand), without a cooldown, with a weapon in the main hand of a class the skill lists and no weapon in the off hand. Unarmed and dual wielding leave the hit fields not computed. An attack other than Smite (`hit::VETTED_ATTACKS`), or one without PoB's parts, is computed but `Partial` (`unmodelled-skill`, `pob-skill-data`). `HitSpeed` stays not computed (PoB leaves it absent).
- **The enemy's armour** is now modelled for every physical hit, spells included (P3-9b's `physical-hit-armour` reason is gone); its cap is the monster template's `maximum_physical_damage_reduction_%` (75).
- **Not modelled, each a reason:** other conversion and gain, luck, double damage, action speed, damage taken, the enemy's block, the hit chance modifier (their stats are unmapped), a weapon with "No Physical Damage" (`unmodelled-item`, `no-physical-damage`), the off hand's weapon (`base-properties`).

New in calc result v1 (additive rule; schema and TypeScript regenerated): graph node `power` (`base ^ exponent`, `math::pow`, PoB's Lua `^`), on the pure-Rust `libm` crate (musl's `pow`; the same bits on Windows, Linux, and wasm32, a test writes them).

## Open question Q11: a weapon's attack rate

PoB rounds a weapon's base attack rate to two decimals in its data exporter (`AttackRateBase = round(1000 / attack time, 2)`: 1.45 for a 690 ms axe, against 1000 / 690 = 1.4493) and the rate with local attack speed again (`Classes/Item.lua`). The game data carries the attack time in milliseconds; the game's tooltip shows two decimals, which says nothing about its calculation. The engine follows PoB until the game's own rounding is settled (context file, Q11). No register entry: nothing differs from the oracle.

A GGG developer's answer (Mark_GGG, 2020-01-29, [mirror](https://devtrackers.gg/pathofexile/p/bc0f1173-questions-thread-january-29-2020)) says weapons store an attack time in milliseconds, and that with increased attack speed the time is "rounded down to integer value" (625 / 1.23 = 508 ms), attacks per second `1000 / ms`, rounded to two decimals only for the tooltip. Where the skill's multiplier, global increases, and more multipliers enter before the floor is not stated. Maintainer decision (2026-11-12, on the recommendation): keep PoB's model until an in-game check settles it. The check, on a weapon tooltip: Siege Axe (667 ms) with 15% increased attack speed shows 1.73 by PoB's model and 1.72 by the floored-ms model (the weapon of `smite-weapon-mods-templar.xml`); Siege Axe 5% (1.58 or 1.57), Jade Hatchet 10% (1.60 or 1.59), Rusted Sword 8% (1.67 or 1.68). If the game shows the floored values, the rate switches and each affected fixture and field gets a `game-truth` register entry.

## Mechanic fixtures

10 new fixtures in `tools/oracle/fixtures/mechanic/` (original, a level 90 Templar unless named), each with the 17 character and defence fields and the 6 hit fields (`HitChance` with them) as targets:

| Fixture | Checks |
|---|---|
| `smite-templar.xml` | Smite 20 Melee Hit with a normal Siege Axe: weapon damage, rate, crit; accuracy against the default enemy's evasion (22% hit chance); conversion; the aura's added lightning; the enemy's armour; strength's melee physical damage |
| `smite-area-hit-templar.xml` | Smite 20 (20% quality) Area Hit: 20% less area damage less its quality; area and not melee |
| `smite-weapon-mods-templar.xml` | a rare Siege Axe (20% quality): local added physical and lightning damage, increased physical damage, attack speed, crit chance, accuracy |
| `smite-melee-physical-templar.xml` | Melee Physical Damage: more melee physical damage (also on the converted part), less attack speed |
| `smite-melee-physical-area-hit-templar.xml` | the same on the Area Hit: no melee more |
| `smite-elemental-attacks-templar.xml` | Elemental Damage with Attacks and Added Lightning Damage |
| `smite-items-templar.xml` | jewellery: added physical damage to attacks, accuracy and increased accuracy, increased physical, melee, attack, and lightning damage, attack speed |
| `smite-crit-items-templar.xml` | an amulet: global crit chance and multiplier on an attack |
| `smite-two-hand-level10-templar.xml` | a level 40 Templar, Smite 10 with a normal Double Axe (10% quality) |
| `smite-enemy-templar.xml` | an enemy configured with level 70, 20,000 armour, 3,000 evasion, 40% lightning resistance |

## Evidence (2026-11-12, Windows)

All runs from clean `b791360` on the P3-1d bundle (`local/p3/p3-1d/bundle-poe1-a`); outputs in `local/p3/p3-10/evidence/`; summaries in `docs/phase-3/attack-hits/`.

| Run | Builds | Result |
|---|---|---|
| `oracle-run`, the fixture manifest with the 10 new fixtures (`local/p3/p3-10/evidence/oracle`) | 81 | Exit 0: every fixture meets its expectation, 3/3 each |
| `calc-run`, public (fixtures, mechanic, community, PoB's 3.13 builds), twice | 81 | 80 documents, `malformed.xml` `IMPORT_XML` as expected (exit 1); 897 fields complete, 68 partial, 578 indeterminate, 377 not computed; the two runs' documents and `report.json` byte-identical |
| `calc-run --data-overlay` (`local/p3/p3-1d/parity-1/overlay.json`), public | 81 | The same field states (no overlay row reaches a hit field) |
| `calc-run`, corpus, `--anonymous`, twice | 21 | All `indeterminate` (older trees; 363 fields, 6 more than P3-9b: attacks now computed); byte-identical |
| `calc-parity`, public, with the overlay run, the register, and the manifest | 81 | **No difference.** 897 fields match (+230: the 10 new fixtures' 23 targets each); 346 breakdown terms match (`TotalDPS` among them), 89 absent where PoB writes none; **every mechanic target met (863 of 863, 44 fixtures)**. Exit 5 (`incomplete`), expected: 34 real builds have no complete field to compare |
| `calc-parity`, corpus, against P3-5's protocol-4 oracle run | 21 | No difference; exit 5 (all indeterminate) |

Against P3-9b: complete fields +230 (the new fixtures), 48 fields of real attack builds move from not computed to indeterminate, nothing else changes state.

The first run found every Melee Hit fixture 1 to 4% low on `AverageDamage` while both Area Hit fixtures matched: strength's `floor(Str / 5)`% increased melee physical damage, now modelled. Two fixtures were `Partial` on that run: an amulet line written "increased Physical Damage" resolved to the weapon-local stat (the game writes "Global Physical Damage" on jewellery; the fixture uses the game's text), and the weapon's canonical "increased Physical Damage" line carries `local_weapon_no_physical_damage` at 0, now mapped.

**The in-scope Smite builds** (`smite-inquisitor-act*`, Q8's derived setups) are `indeterminate` on the hit fields (unsettled item values), with partial reasons for unmapped stats, other gems (auras, buffs), config, and choices; never compared. Their Smite levels (5 and 13) are not the ones where the game's resolved added lightning differs from PoB's (levels 1 and 3), so no register entry is needed; the fixtures use levels 20 and 10.

**Spells unchanged:** the 11 Ice Nova mechanic fixtures' documents are byte-identical to P3-9b's (`local/p3/p3-9b/evidence/public`); the three community Ice Nova builds' field states and values are unchanged (their wand is now modelled and two attack stats mapped, so their reasons changed).

Tests (`crates/loom-calc/tests/attack.rs`, 11): a melee hit with conversion and strength's bonus; an area hit with the area more, part clamping and the default part; the hit chance against evasion; the enemy's armour per pass; only attacks with a usable main-hand weapon (none, dual wielding, a wrong class; a shield is fine); an attack the oracle has not checked `Partial` with the game's flags; the weapon's local lines and "No Physical Damage" as a reason; the second weapon set; an off hand of unknown base, part 0 and a non-numeric part; conversion above 100% scaled; another weapon class and a base without attack time (missing data, never an infinite rate). Mapping tests for the attack and weapon rows; `math::pow` bits; the graph's `power` node; calc result v1 golden files, schema, and TypeScript regenerated. 1106 workspace tests, clippy, fmt, wasm32, `cargo deny`. The review's fixes (an attack-time guard, tests, comments) leave every public document byte-identical to the run above.

Commands (`<out>` a new folder; as P3-9b's):

```sh
lpc=target/release/loom-parity-cli; b=local/p3/p3-1d/bundle-poe1-a; scope=docs/phase-3/calc-scope-poe1.json
public="tools/oracle/fixtures tools/oracle/fixtures/mechanic tools/oracle/fixtures/community local/oracle/pob/spec/TestBuilds/3.13"
$lpc oracle-run tools/oracle/manifest.json local/oracle --out <out>/oracle tools/oracle/fixtures/fixtures.json
$lpc calc-run $public --bundle $b --scope $scope --out <out>/public
$lpc calc-run $public --bundle $b --scope $scope --data-overlay local/p3/p3-1d/parity-1/overlay.json --out <out>/public-overlay
$lpc calc-parity <out>/oracle <out>/public --slice docs/phase-3/slice-fields.json --out <out>/parity --register docs/parity/divergence-register.json --overlay-run <out>/public-overlay --manifest tools/oracle/fixtures/fixtures.json
$lpc calc-run "<corpus>" --anonymous --bundle $b --scope $scope --out <out>/corpus
$lpc calc-parity local/p3/p3-5/evidence-p4/oracle-corpus <out>/corpus --slice docs/phase-3/slice-fields.json --out <out>/parity-corpus --register docs/parity/divergence-register.json
```

## Follow-ups (not in this PR)

- **Weapon bases in data parity.** The review compared the bundle's `weapon_types` with PoB v2.67.2's `Data/Bases/*.lua`: 360 of 362 bases equal (`round(1000 / ms, 2)` equals PoB's `AttackRateBase`, and the physical damage and crit are equal); Driftwood Wand (game 690 ms, 4 to 8; PoB 1.5, 5 to 9) and Crude Bow (game 741 ms, 4 to 10; PoB 1.4, 5 to 13) differ, PoB's data older than the game's. No fixture uses them. A `data-parity` table for weapon bases (and its overlay rows) makes such a fixture a known `data-version` difference instead of `uncategorized`; the 360 equal rates also support Q11's default (PoB's rounding is its exporter's, applied to the same millisecond values).
- **Supports' weapon restrictions.** PoB disables a skill whose supports exclude the weapon (`getWeaponFlags` over each support's `weaponTypes`). The bundle does not carry supports' restrictions; no mapped support has one, and a support with one has unmapped stats (`Partial`). A PR that maps such a support adds the data with it.
- **Fixtures not yet covered by the oracle:** a shield in the off hand with an attack, other weapon classes (sword, mace, staff, sceptre), a skill conversion above 100% (unit-tested only).
- **Gate for Smite on real builds:** the four in-scope Smite builds stay indeterminate (unsettled item values; other gems, config, choices); step 12 (buffs and config) and step 8 (mappings) bring them closer. This PR's acceptance rests on the 10 mechanic fixtures (230 targets).
