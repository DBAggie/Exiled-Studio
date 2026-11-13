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

## P3-12g: the pantheon's soul stats

The pantheon (`docs/phase-3/config-inputs.md`, "P3-12g") applies its soul stats through this mapping. All 41 (12 gods, each stat on one soul) are `NoEffect` rows, read as PoB v2.67.2 reads their lines (`Modules/ModParser.lua`): avoidance, stun, ailments and curses on you, damage taken by you and its reduction, regeneration and flasks, movement speed, the enemy's regeneration and attack speed. `NO_EFFECT` holds 96 rows.

Three of them reach a slice field only through a config input, which must stay a reason: `NO_EFFECT_DEPENDS_ON_INPUTS` (`loom_calc::poe1::mapping`) lists `conditionBlinded` (for `cannot_be_blinded`), `conditionChilled` and `conditionChilledEffect` (for `chill_effectiveness_on_self_+%`), and the thirteen `playerCursedWith*` inputs (for `curse_effect_on_self_+%`); a test sets each and expects `unmodelled-config`. `additional_chaos_resistance_against_damage_over_time_%` is `ChaosResist` with `ModFlag.Dot` in PoB, outside the shown resistance.

Evidence: the maintainer-run real-bundle test (every soul stat of the bundle is a `NoEffect` row); four Ice Nova mechanic fixtures with eight of the twelve gods, each equal to PoB on every target, and PoB's fields equal to the fixture without a god. The guards that list `NoEffect` rows naming charges or damage taken list the souls' rows.

## P3-8d: charge modifiers

The Ice Nova builds' passives give power charges: "+1 to Maximum Power Charges", "+1 to Minimum Power Charges", and effects per charge (increased damage, spell damage, crit chance, crit multiplier). Until P3-8d each was `unmapped-stat`, and the charges (P3-12e) counted the character's constant maximum only.

### What PoB v2.67.2 does

`doActorCharges` (`Modules/CalcPerform.lua` 947 to 1023) counts each kind:
- the maximum is `<Kind>ChargesMax` BASE, the character's constant plus modifiers, at least 0 (or its OVERRIDE);
- the minimum is `<Kind>ChargesMin` BASE, at least 0;
- the count is `max(use ? (override or maximum) : 0, min(maximum, minimum))`, so a minimum gives charges without the `use<Kind>Charges` check.

A per-charge modifier is a mod with the `{ type = "Multiplier", var = "<Kind>Charge" }` tag (`Modules/ModParser.lua`), so it acts times the count.

### The engine

- **`Poe1Quantity::Charge(kind, value)`** (calc result v1 names such as `poe1.charge.power.maximum` and `poe1.charge.power.per.crit-multiplier`, additive): a modifier's `Base` effect, which only `charges` reads.
- **`charges::MODIFIERS`**, each read as PoB reads its game text. `Poe1Mapping` asks this table before its rows.

  | Stats | What they change |
  |---|---|
  | `max_power_charges`, `max_frenzy_charges`, `max_endurance_charges` | The maximum |
  | `base_minimum_*_charges` | The minimum |
  | `damage_+%_per_power_charge`, `damage_+%_per_frenzy_charge`, `damage_+%_per_endurance_charge` | `Damage` INC per charge |
  | `spell_damage_+%_per_power_charge` | Spell `Damage` INC per charge |
  | `critical_strike_chance_+%_per_power_charge` | `CritChance` INC per charge. This is also the character's constant, which still applies; the two add, as PoB's two mods do |
  | `critical_strike_multiplier_+_per_power_charge` | `CritMultiplier` BASE per charge |

- **`charges`** runs after every other source and counts as PoB does. The constant effects take the `use` input as their source, or the first minimum modifier's source when only a minimum gives charges. Each per-charge modifier keeps its own source.
- Everything else that reads or changes a count stays a reason: "at maximum" flags, conversions to absorption, affliction, or brutal charges, "maximum frenzy charges equal maximum power charges", and per-charge stats not in the table. The module's guard test still holds for every row outside the table.

### Mechanic fixtures

| Fixture | What it isolates |
|---|---|
| `ice-nova-power-charge-modifiers-witch.xml` | Power charges used, +1 maximum (4 charges), damage, spell damage, crit chance, and crit multiplier per charge |
| `ice-nova-minimum-power-charges-witch.xml` | No `use` check; +2 minimum (2 charges) and spell damage per charge |

The first run of both matched PoB on every target.

Not traced: when a modifier raises the maximum, the constant effects (the power charges' crit chance, the frenzy charges' speed and damage) name the `use` input as their source, not the modifier, and a per-charge modifier with no charges gives no effect, so no breakdown shows it. Both are labels, not values (PoB's are the same).

### Review

`code-architecture-reviewer`: no blockers. It confirmed the count rule, each row against PoB's parser, no double count of the crit-chance-per-charge constant and stat, the conversions and flags staying reasons, gem stats staying reasons, and the order. Applied: tests for every kind (a frenzy minimum without the check, an endurance per-charge effect), a negative maximum, crit multiplier per charge, and the table's disjointness from the mapping's other tables. Not changed: the provenance of the constant effects (noted above) and splitting `charges()`.

### Evidence

Maintainer-run, 2026-11-13, Windows, from the worktree on a clean tree. `local/p3/p3-8d/evidence.sh` ran at tool commit `05f03f6`; the review's tests (`553947f`) change no engine code. No game data changes: P3-12i's bundle and overlay. Summaries are in `docs/phase-3/stat-mapping/p3-8d-*`.

| Run | Result |
|---|---|
| `oracle-run`, the whole manifest | 110 fixtures, every expectation met, 3/3 identical runs each |
| `calc-run`, public, twice | 110 documents; 1537 complete, 334 partial, 433 unsupported, 312 indeterminate. Byte-identical twice |
| `calc-parity`, public | 33 registered differences (unchanged), 0 unexpected; 1513 fields match. **Every mechanic target met: 1503 of 1503 over 73 fixtures** |
| `calc-run` and `calc-parity`, corpus (21, `--anonymous`), twice | Byte-identical; no difference |

**Against P3-12j** (`p3-8d-public-vs-p3-12j.json`, `p3-8d-corpus-poe1-26530219-vs-p3-12j.json`). No field state changes. 16 values in 5 public builds change, every one closer to PoB's: the two in-scope Ice Nova Elementalist builds (campaign and mapping: average damage, crit chance, crit multiplier, hit DPS), Boneshatter, Elemental Hit, and Kinetic Blast. 102 reasons gone in 23 public builds, none added; corpus 38 gone, none added, no value change.

Tests: 1209 workspace tests, clippy, fmt, wasm32.

## P3-8e: area damage, spell crit multiplier, area and light radius

Six stats from the in-scope Ice Nova builds' passives and gems, each read as PoB v2.67.2 reads it:

| Stat | PoB | The engine |
|---|---|---|
| `area_damage_+%` | `Damage` INC with `ModFlag.Area` (`Data/SkillStatMap.lua` 779) | hit row, `Damage` INC, area |
| `base_spell_critical_strike_multiplier_+` ("+#% to Critical Strike Multiplier for Spell Damage") | `CritMultiplier` BASE with "for spell damage" (`ModFlag.Spell`; `Modules/ModParser.lua` 748, 968) | hit row, `CritMultiplier` BASE, spell |
| `base_skill_area_of_effect_+%`, `base_aura_area_of_effect_+%`, `spell_area_of_effect_+%` | `AreaOfEffect` INC (`SkillStatMap.lua` 573 to 577), which feeds only radius outputs (`Modules/CalcOffence.lua` 345 to 455, 1370) | `NoEffect` (`area`) |
| `light_radius_+%` | `LightRadius` INC; it reaches a slice field only through `LightRadiusAppliesToDamage` (`CalcOffence.lua` 746), `LightRadiusAppliesToAccuracy` (732), and Eclipse Solaris's fire damage per 1% light radius (`ModParser.lua` 2436) | `NoEffect` (`light-radius`), with those four stats as `NO_EFFECT_DEPENDS_ON` guards (they stay `Unmapped`: a test). The game's other light radius stats (to area of effect, with energy shield, while phasing) reach no slice field and stay `Unmapped` |

No slice field reads area of effect (`docs/phase-3/slice-fields.json`).

### Mechanic fixture

`ice-nova-area-spell-crit-witch.xml`: Ice Nova with custom lines for all six stats ("20% increased Area Damage", "+30% to Critical Strike Multiplier for Spell Damage", "15% increased Area of Effect", "Spell Skills have 10% increased Area of Effect", "10% increased Area of Effect of Aura Skills", "20% increased Light Radius"). Its average damage, hit DPS, and crit multiplier change; no other field does; no reason is left. The first run matched PoB on every target.

### Review

`code-architecture-reviewer`: the mapping is right; every PoB reader of area of effect and light radius was checked, and the guard list is complete for PoB. Applied: assertions for the two rows in the mapping tests, comments moved back to their rows, the guards' reasons in `NO_EFFECT_DEPENDS_ON`'s doc, and this section.

### Evidence

Maintainer-run, 2026-11-13, Windows, from the worktree on a clean tree. `local/p3/p3-8e/evidence.sh` ran at tool commit `0b9a946`; the review's `cc8dbf1` changes tests and comments only. P3-12i's bundle and overlay are used. Summaries are in `docs/phase-3/stat-mapping/p3-8e-*`.

| Run | Result |
|---|---|
| `oracle-run`, the whole manifest | 111 fixtures, every expectation met, 3/3 identical runs each |
| `calc-run`, public, twice | 111 documents; 1559 complete, 334 partial, 435 unsupported, 312 indeterminate. Byte-identical twice |
| `calc-parity`, public | 33 registered differences (unchanged), 0 unexpected; 1535 fields match. **Every mechanic target met: 1525 of 1525 over 74 fixtures** |
| `calc-run` and `calc-parity`, corpus (21, `--anonymous`), twice | Byte-identical; no difference |

**Against P3-8d.** No field state changes. 6 values change, in the two in-scope Ice Nova Elementalist builds, all closer to PoB's; the campaign build's crit multiplier now equals PoB's (2.4). 83 reasons gone in 22 public builds, none added; corpus 57 gone, none added, no value change.

Tests: 1209 workspace tests, clippy, fmt, wasm32.

## P3-8f: weapon conditions

Eight stats of the in-scope builds (and many public builds) apply "while wielding a Staff" or "while holding a Shield". PoB v2.67.2 reads those phrases as `{ type = "Condition", var = "UsingStaff" }` and `UsingShield` (`Modules/ModParser.lua` 1619, 1598), conditions `doActorAttribsConditions` sets from the equipped items (`Modules/CalcPerform.lua` 135 onward): `UsingShield` when "Weapon 2" is a shield, `Using<flag>` from the main hand's weapon type (`data.weaponTypeInfo`: a staff or warstaff is `Staff`).

### The engine

- **`loom_calc::poe1::states`** settles `UsingStaff` and `UsingShield` from the hands of the weapon set in use (`collect::hands`, which gains `main_hand` and `off_hand`: empty, a weapon of a class, a shield, another base, or unknown). A hand whose base is not known leaves its condition unsettled: an effect that needs it is `unmodelled-condition`. Every PoE1 query context (`hit.rs`, `phases.rs`) holds them (since P3-8g through `Ruleset::states` and `Calc::context`). The core's named conditions (`Conditions::with_state`) existed since P3-4; no effect used one before, so no earlier value can change.
- **`mapping::STATE_ROWS`**, each as PoB reads its text:

  | Stat | Effect |
  |---|---|
  | `defences_+%_while_wielding_staff` | energy shield, armour, evasion INC (PoB's `Defences`), staff |
  | `global_critical_strike_chance_+%_while_holding_staff` | `CritChance` INC, staff |
  | `global_critical_strike_multiplier_+_while_holding_staff` | `CritMultiplier` BASE, staff |
  | `shield_spell_block_%` | spell block BASE, shield |
  | `spell_block_with_staff_%` | spell block BASE, staff |
  | `spell_damage_+%_while_holding_shield` | spell `Damage` INC, shield |
  | `spell_staff_damage_+%` | spell `Damage` INC, staff |
  | `staff_block_%` | `BlockChance` BASE, staff |

- Not settled (their effects stay reasons): `OffHandIsEmpty`, `Unarmed`, the other weapon types, one- and two-handed, melee weapon, dual wielding, Necromantic Aegis. PoB also reads the off hand's weapon type; a staff cannot be held there.

### Mechanic fixtures

| Fixture | What it isolates |
|---|---|
| `ice-nova-staff-conditions-witch.xml` | A Gnarled Branch; spell damage, crit chance and multiplier, spell block, defences while wielding a staff |
| `ice-nova-shield-conditions-witch.xml` | A Carved Wand and a Twig Spirit Shield; spell damage and spell block while holding a shield, and staff lines that do nothing |

Both matched PoB on every target with no reason left (the first draft used Driftwood Wand, a name with two base rows, and the attack block line, whose text two stats share; both were replaced). Attack block while wielding a staff, warstaves, the swap set, an off-hand armour without block, and an unknown main hand are covered by `crates/loom-calc/tests/attack.rs`.

### Review

`code-architecture-reviewer`: no code blockers. It confirmed the conditions' sources (the set in use, Warstaff as Staff, Aegis needing a shield, an empty hand settled false), that no earlier value can change, and that the block, spell block, and defence phases read the conditional effects. Applied: the engine-level tests above, an assertion for every row, and module docs on what is not settled. Not changed: `Hands::off_hand_weapon` beside `off_hand` (derivable; left to avoid touching the attack path after the run). Follow-up: the hands are looked up for every query context, so the public run's calculation time rose from about 74 ms to 229 ms (113 builds); done in P3-8g (below).

### Evidence

Maintainer-run, 2026-11-13, Windows, from the worktree on a clean tree. `local/p3/p3-8f/evidence.sh` ran at tool commit `8d6b7ff`; the review's commit adds tests and docs only. P3-12i's bundle and overlay are used. Summaries are in `docs/phase-3/stat-mapping/p3-8f-*`.

| Run | Result |
|---|---|
| `oracle-run`, the whole manifest | 113 fixtures, every expectation met, 3/3 identical runs each |
| `calc-run`, public, twice | 113 documents; 1603 complete, 334 partial, 439 unsupported, 312 indeterminate. Byte-identical twice |
| `calc-parity`, public | 33 registered differences (unchanged), 0 unexpected; 1579 fields match. **Every mechanic target met: 1569 of 1569 over 76 fixtures** |
| `calc-run` and `calc-parity`, corpus (21, `--anonymous`), twice | Byte-identical; no difference |

**Against P3-8e.** No field state changes. 32 values in 14 public builds change, every one closer to PoB's or equal (1 equal): block and spell block in most of them; the Ice Nova mapping build's armour, evasion, energy shield, block, spell block, crit chance and multiplier, average damage, and DPS; the Ice Nova campaign build's damage and spell block. 124 reasons gone in 15 public builds, none added. Corpus: 6 values in 6 builds closer to PoB's or equal (spell block), 11 reasons gone, none added.

Tests: 1212 workspace tests, clippy, fmt, wasm32.

## P3-8g: the settled conditions once per calculation

P3-8f settled `UsingStaff` and `UsingShield` in every query context, looking the hands up each time; the public run's calculation time rose about threefold. The core now asks the game's rules once per calculation: `Ruleset::states` (default: none) gives the named conditions the build settles, `Calc` keeps them, and `Calc::context(flags)` builds every query context with them. PoE1's rules return `poe1::states::states`. The core stays game-neutral: it holds condition names and values, never PoE1's. A condition that could differ per query (a skill's own hand) would need a per-query rule; none does yet (both conditions are the character's).

### Evidence

Maintainer-run, 2026-11-13, Windows, from the worktree on a clean tree, `local/p3/p3-8g/evidence.sh` at tool commit `4cfece7`. No fixture changes: P3-8f's oracle run is the oracle side.

| Run | Result |
|---|---|
| `calc-run`, public, twice | Byte-identical twice, and **byte-identical to P3-8f's** (113 documents). Calculation time 78 ms (P3-8f: 229 ms) |
| `calc-parity`, public, with P3-8f's oracle run | The same counts as P3-8f: 33 registered differences, 0 unexpected, 1579 fields match |
| `calc-run`, corpus (21, `--anonymous`) | Byte-identical to P3-8f's. Calculation time 39 ms (P3-8f: 139 ms) |

Tests: 1213 workspace tests (the engine tests' synthetic ruleset now settles its conditions through `Ruleset::states` and queries through `Calc::context`; a direct test of `Calc::context` and of the default), clippy, fmt, wasm32. Review: `code-architecture-reviewer`, no blockers; doc fixes and the direct tests applied.
