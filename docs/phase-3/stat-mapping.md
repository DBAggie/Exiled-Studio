# Phase 3: stat-to-effect mapping (step 8)

Plan step 8 (`dev/active/phase-3-calc-slice/phase-3-calc-slice-plan.md`): map the stats the in-scope builds reach, most frequent first, each with a test; per PR, the inventory's unmapped count for the in-scope builds falls, no in-scope build gains an unregistered difference, and the coverage summary is committed. Coverage summaries: `docs/phase-3/stat-mapping/`.

## P3-8a: stats that reach no slice field

Review (`code-architecture-reviewer`): every stat traced through PoB's `ModParser.lua`, `ModCache.lua`, `SkillStatMap.lua` and the calc modules; two left the table (fortification's maximum, the self crit multiplier), the table is checked against the local piece rows too, and the real converters are named at the table.

A stat that cannot change any of the 24 slice fields (attributes, life, mana, energy shield, armour, evasion, resistances, block, spell block, spell suppression, unreserved life and mana, and the main skill's average damage, speed, hit speed, hit chance, crit chance and multiplier, and hit DPS), whatever the build's config, is known: `StatMap::NoEffect`. It then no longer makes every computed field `partial`, and no value changes. The table is `NO_EFFECT` in `crates/loom-calc/src/poe1/mapping.rs`, each stat with a category:

| Category | Stats |
|---|---|
| `regeneration` | life, mana, and energy shield regeneration (rate, per level, per charge, per fortification, per spell block, after exposure, near rare enemies), Zealot's Oath's swap, consecrated ground's ES recovery |
| `leech` | energy shield leech from spell damage; cannot have life leeched from |
| `movement-speed` | `base_movement_velocity_+%` |
| `minion-defence` | minion and golem life, regeneration, golem immunity and revival |
| `freeze`, `ailment-duration` | chance to freeze, freeze proliferation, freeze, cold and lightning ailment durations |
| `stun`, `interruption` | stun duration, avoid interruption while casting |
| `damage-taken`, `ailment-on-self` | physical damage taken as fire or lightning, ailment duration on you, cannot be shocked |
| `charge-gain-chance` | chances to gain power charges (the count used comes from the config) |
| `fortification` | fortify chance and duration |
| `ground-creation-chance`, `ground-duration`, `rage-duration` | consecrated ground on kill, its lingering, rage loss delay |
| `crafting`, `sockets` | multiple crafted mods, abyss socket count |

Left out on purpose (they stay `unmapped`, so `partial`, until a step maps them): anything that can change a hit or its rate through the enemy (shock and its effect, chill, exposure, intimidate, consecrated ground's effect, curses, penetration), charge counts and maximums, rage, area of effect, gem levels, minion damage and speed (keystones redirect them to you), mana reservation efficiency, action speed, light radius (converters exist), staff- or shield-conditioned block and damage, fortification's maximum (PoB scales suppression, armour, evasion, and attack speed per fortification), and `base_self_critical_strike_multiplier_-%` (PoB's skill stat map also reads it as the enemy's `SelfCritMultiplier`, which reaches the crit multiplier). A stat that makes a `NO_EFFECT` row matter is itself unmapped; mapping such a converter later must revisit the rows it redirects.

### Evidence (2026-11-12, Windows; tool commit `8b11a17`, clean)

| Run | Builds | Result |
|---|---|---|
| `calc-run`, public, twice | 60 | 59 documents (`malformed.xml` fails import, as expected); field states equal to P3-7's (425 complete, 68 partial, 510 indeterminate, 413 not computed): the real builds stay `partial` or `indeterminate` through gems, config, and older trees; byte-identical twice |
| `calc-run`, corpus, `--anonymous`, twice | 21 | all `indeterminate` (older trees), as before; byte-identical |
| `calc-parity`, public, against P3-7's oracle run (same manifest), with the overlay run | 60 | No difference; 425 fields and 178 breakdown terms match; every mechanic target met. Exit 5 (`incomplete`), as in P3-7 |
| `calc-parity`, corpus | 21 | No difference; exit 5 (all indeterminate) |

Unmapped stats (`docs/phase-3/stat-mapping/p3-8a-*-coverage.json`, counts only):

| Group | Builds | Before (distinct, occurrences) | After |
|---|---|---|---|
| In-scope (Ice Nova, Smite, Winter Orb) | 8 | 153, 688 | 113, 572 |
| All public | 59 | 1054, 4221 | 1014, 3856 |
| Corpus | 21 | 559, 2491 | 535, 2305 |

Tests (`crates/loom-calc/src/poe1/mapping.rs`, `tests/poe1.rs`): the table is sorted and disjoint from the mapped and local piece rows; each row is `NoEffect` through `Poe1Mapping` and an armour piece's `PieceMapping`; nine stats that can reach the slice stay unmapped; a build with regeneration and movement speed stays `Complete`.
