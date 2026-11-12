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

## P3-8c: the Smite builds' global stats

**Why these five.** After P3-12a, `smite-inquisitor-act1` had five unmapped stats left (13 occurrences), the fewest of the in-scope builds; all five also occur in the other Smite builds and two in the Ice Nova builds. With them done, that build has no `unmapped-stat` reason (it stays `Partial` through other gems, 10 config inputs, and the pantheon).

| Stat | Text | Read as (PoB v2.67.2) |
|---|---|---|
| `attack_and_cast_speed_+%` | "#% increased Attack and Cast Speed" | `Speed` INC, no flag (`Data/SkillStatMap.lua` 524): every rate the engine computes is an attack's or a cast's, so it adds to the same sum as `attack_speed_+%` and `base_cast_speed_+%` |
| `elemental_damage_with_attack_skills_+%` | "#% increased Elemental Damage with Attack Skills" | `ElementalDamage` INC, `KeywordFlag.Attack` (1593): `Damage` INC for an attack's elemental damage |
| `melee_critical_strike_chance_+%` | "#% increased Melee Critical Strike Chance" | `CritChance` INC, `ModFlag.Melee` (no stat map row; `ModParser.lua`'s "melee" flag word) |
| `chance_to_freeze_shock_ignite_%` | "#% chance to Freeze, Shock and Ignite" | `NoEffect` (`ailment-chance`): PoB's ailment chances (`CalcOffence.lua` 4262) set the chance on hit and the ailment's flags; the enemy is shocked only from the config or a `ShockBase`/`ShockOverride`/`ShockMinimum` source (`CalcPerform.lua` 3591), never from a chance |
| `minion_damage_+%` | "Minions deal #% increased Damage" | `NoEffect` (`minion-damage`): a `MinionModifier` list, read only by a minion's calculation; no minion is in the slice |

**What `minion_damage_+%` depends on** (`NO_EFFECT_DEPENDS_ON`). Minion damage reaches you only through `MinionDamageAppliesToPlayer` (`CalcOffence.lua` 613), set by six game stats ("Increases and Reductions to Minion Damage also affect you", its 150% and Dominating Blow forms, "Modifiers to Minion Damage also affect you", and two skill stats; `ModParser.lua` 3455 to 3459, `SkillStatMap.lua` 905). A test asserts each is `Unmapped` through every mapping: while one is in a build, the fields are `Partial`; mapping one must revisit the row. Four more stats in the stats table name the same redirect but have no description and no PoB row, so no line resolves to them (they are `Unmapped` too). Minion speed stays unmapped (P3-8a's rule).

### Evidence (2026-11-12, Windows; tool commit `5287654`, clean)

Inputs as P3-12a's: bundle `local/p3/p3-1d/bundle-poe1-a`, overlay `local/p3/p3-1d/parity-1/overlay.json`; commands as in `docs/phase-3/config-inputs.md` (out folder `local/p3/p3-8c/evidence`). The runs on the uncommitted tree gave byte-identical documents to the clean commit's.

| Run | Builds | Result |
|---|---|---|
| `oracle-run`, the whole manifest | 85 | Every expectation met (84 `ok`, `malformed.xml` `ORACLE_IMPORT`), 3/3 identical runs each; the two new fixtures among them |
| `calc-run`, public, twice | 85 | 84 documents; 981 complete (+45, the two new fixtures), 334 partial, 389 unsupported, 312 indeterminate. Byte-identical twice |
| Documents against P3-12a's run | 82 | **No field state changes.** 188 `unmapped-stat` reasons gone in 23 builds (the five stats), none added. Values change in 8 builds already `Partial` (the 4 Smite builds, 2 Ice Nova, Shock Nova, Elemental Hit), the new stats now counted |
| `calc-parity`, public, with the overlay run, the register, and the manifest | 85 | 25 differences, all registered (DIV-0001 to DIV-0025, unchanged), 0 unexpected, no unused entry, no missing mechanic fixture; 963 fields match (+45). Exit 5 (`incomplete`), as before |
| `calc-run` and `calc-parity`, corpus (21, `--anonymous`), twice | 21 | All `indeterminate` (older trees), byte-identical; reasons only (106 gone in 16 builds); no difference |

**The new fixtures** (`tools/oracle/fixtures/mechanic/`): `smite-global-attack-stats-templar.xml`, Smite on a Karui Maul (1000 ms, the one Smite weapon whose attack time divides 1000, so the game's floored time equals PoB's and no register entry is needed) with an amulet carrying all five lines; `ice-nova-attack-and-cast-speed-witch.xml`, Ice Nova with "12% increased Attack and Cast Speed" (PoB's rate 1.4286 to 1.6). Every field of both is `Complete` and matches PoB. A local check (the Smite fixture with each line removed, engine only) shows each mapped line changes the value PoB matches: without the speed line `Speed` 0.85 (0.935 with it), without melee crit `CritChance` 1.1 (1.54), without the elemental line `AverageDamage` 122.03 (152.18); without the minion and ailment lines nothing changes, so PoB's values with those lines equal the engine's without them.

Unmapped stats (`docs/phase-3/stat-mapping/p3-8c-*-coverage.json`, against P3-12a's run):

| Group | Builds | Before (distinct, occurrences) | After |
|---|---|---|---|
| In-scope (Ice Nova, Smite, Winter Orb) | 8 | 108, 363 | 103, 292 |
| All public | 82 | 1045, 3196 | 1040, 3008 |
| Corpus | 21 | 509, 1844 | 504, 1738 |

Tests (1128 workspace tests, 2 new, `mapping.rs`): the three rows as PoB reads them, the same through a gem's mapping, the two `NoEffect` rows; `NO_EFFECT_DEPENDS_ON` sorted, naming `NO_EFFECT` rows, each stat `Unmapped` through the game's, gem, piece, and weapon mappings. Clippy, fmt, wasm32. Reviewed by `code-architecture-reviewer`: no blockers; each reading confirmed in PoB; the parity evidence above and the comment on the four unreachable stats applied.
