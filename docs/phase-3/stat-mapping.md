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

### Evidence (P3-8g)

Maintainer-run, 2026-11-13, Windows, from the worktree on a clean tree, `local/p3/p3-8g/evidence.sh` at tool commit `4cfece7`. No fixture changes: P3-8f's oracle run is the oracle side.

| Run | Result |
|---|---|
| `calc-run`, public, twice | Byte-identical twice, and **byte-identical to P3-8f's** (113 documents). Calculation time 78 ms (P3-8f: 229 ms) |
| `calc-parity`, public, with P3-8f's oracle run | The same counts as P3-8f: 33 registered differences, 0 unexpected, 1579 fields match |
| `calc-run`, corpus (21, `--anonymous`) | Byte-identical to P3-8f's. Calculation time 39 ms (P3-8f: 139 ms) |

Tests: 1213 workspace tests (the engine tests' synthetic ruleset now settles its conditions through `Ruleset::states` and queries through `Calc::context`; a direct test of `Calc::context` and of the default), clippy, fmt, wasm32. Review: `code-architecture-reviewer`, no blockers; doc fixes and the direct tests applied.

## P3-8h: skill keywords

Lines such as "20% increased Cast Speed with Cold Skills" need the skill's **keyword**, not a damage type. PoB v2.67.2 sets `KeywordFlag.Lightning`, `.Cold`, `.Fire`, `.Chaos`, `.Physical` from the skill's types after the supports added theirs (`Modules/CalcActiveSkill.lua` 391 to 405), and never per damage pass (`Modules/CalcOffence.lua` adds only the damage over time keywords per type). So Ice Nova is a cold skill whatever its hit deals, and Smite a lightning skill although half its hit is physical.

**The engine.** `Flags` (shared core) gains five skill keywords, `SKILL_PHYSICAL` to `SKILL_CHAOS` and `SKILL_ELEMENTAL`, apart from the damage type flags, which describe one part of a hit. `poe1::skill::keyword_flags` gives them from the type names (`KEYWORD_TYPES`, the game's `ActiveSkillType` IDs, which PoB's `SkillType` names match: a real-bundle test checks all five), and the spell and attack hit contexts carry the main skill's. When ailments or damage over time are computed, their contexts must take them too (PoB builds those configs from the skill's keyword flags).

| Stats | PoB (`Modules/ModParser.lua`) | The engine (hit rows) |
|---|---|---|
| `cast_speed_for_<chaos\|cold\|fire\|lightning>_skills_+%` | "cast speed" (`Speed` INC, `ModFlag.Cast`, 754) with "with cold skills" (`KeywordFlag.Cold`, 1048 to 1052) | `Speed` INC, cast and the keyword |
| `cast_speed_for_elemental_skills_+%` | "with elemental skills": any of the three keywords (1051) | `Speed` INC, cast, any elemental keyword |
| `<chaos\|cold\|fire\|lightning>_critical_strike_chance_+%`, `..._multiplier_+` | `CritChance` INC, `CritMultiplier` BASE (745, 748) with the keyword | the same, with the keyword |
| `<physical\|fire\|cold\|lightning\|chaos>_damage_with_attack_skills_+%`, `..._spell_skills_+%` | `ColdDamage` INC (701 and above) with `KeywordFlag.Attack` (978) or `.Spell` (971) | `Damage` INC with the damage type and the attack or spell flag |

`SPELL` stands for both `ModFlag.Spell` and `KeywordFlag.Spell`, as `ATTACK` already did; PoB leaves the spell keyword off only for the skills its data marks `cast` (Bear Trap, Vaal Detonate Dead, three heralds, one minion skill), none a vetted spell.

**Left:** "with Brand Skills" and "with Minion Skills" (PoB reads them as a `SkillType` tag; Brand is a PoE1-only word, so it is no shared flag): unmapped, reasons. No mapped stat reads the physical keyword yet.

### Mechanic fixtures

- `ice-nova-skill-keywords-witch.xml`: cast speed with cold (20%) and elemental (10%) skills apply, with fire skills (30%) not; +25% crit multiplier with cold skills applies, +60% with lightning skills not; 40% crit chance with cold skills; cold damage with spell skills applies, with attack skills not. Every target equals PoB's.
- `smite-skill-keywords-templar.xml`: crit chance and multiplier with lightning skills apply, with cold skills not; cast speed with lightning skills not (an attack); physical and lightning damage with attack skills apply, fire damage with attack skills has no fire to scale, lightning damage with spell skills not. Average damage, crit chance and multiplier equal PoB's; speed and DPS differ by Q11's floored attack time alone (the same 1.27436 as `smite-intimidate-blind-templar.xml`): DIV-0034, DIV-0035 (`game-truth`).

### Review

`code-architecture-reviewer`: no blockers; the keyword and damage type separation checked against PoB (only the damage over time keywords are added per type). Applied: direct tests of `keyword_flags` (each type name, several, a type a support adds, the real bundle's names), `Flags::skill_keyword` for `Flags::skill`, notes on future ailment contexts, the skills PoB marks `cast`, and the unread physical keyword. The cited `ModParser.lua` lines were checked.

### Evidence (P3-8h)

Maintainer-run, 2026-11-13, Windows, from the main checkout on a clean tree, `local/p3/p3-8h/evidence.sh` at tool commit `ceaa1e5`; the review's `9f03533` changes tests, comments, and one name only. P3-12i's bundle and overlay are used. Summaries are in `docs/phase-3/stat-mapping/p3-8h-*`.

| Run | Result |
|---|---|
| `oracle-run`, the whole manifest | 115 fixtures, every expectation met, 3/3 identical runs each |
| `calc-run`, public, twice | 115 documents; 1648 complete, 334 partial, 442 unsupported, 312 indeterminate. Byte-identical twice |
| `calc-parity`, public | 35 registered differences (two new), 0 unexpected; 1622 fields match. **Every mechanic target met: 1614 of 1614 over 78 fixtures** |
| `calc-run` and `calc-parity`, corpus (21, `--anonymous`), twice | Byte-identical; no difference |

**Against P3-8g.** No field state changes. 13 values change in 5 public builds, all toward PoB's: the three in-scope Ice Nova Elementalist builds' speed (cast speed with cold skills) and DPS, the mapping build's crit multiplier (4.08 to 4.25, PoB 4.48) and average damage; Shock Nova's speed, Elemental Hit's crit multiplier. 25 reasons gone in 12 public builds (among them every `cold_critical_strike_multiplier_+`, `cast_speed_for_cold_skills_+%`, and the Smite act10 build's four `fire_damage_with_attack_skills_+%`), none added; corpus 5 gone, none added, no value change.

Tests: 1218 workspace tests, clippy, fmt, wasm32, the real-bundle test.

## P3-8i: gem levels

All three in-scope Ice Nova builds raise their gems' levels: "+1 to Level of all Cold Skill Gems" (a passive), "+N to Level of all Cold Spell Skill Gems" (an item), and in the mapping build also spell, all, and critical support gem levels. Until P3-8i each was `unmapped-stat`, and every gem kept the level the build writes.

### What PoB v2.67.2 does

- `Modules/ModParser.lua` 3280 to 3290 reads "+N to level of all <words> gems" as a `GemProperty` LIST modifier `{ keywordList = <the words>, key = "level", value = N }`. A phrase that is one gem's name gives `keyword` instead (3282).
- `Modules/CalcSetup.lua` tabulates the `GemProperty` modifiers per group (1641, 1769), after the passives, items, and config (693) are in the modifier database. `applyGemMods` (397 to 429) adds `value` to the level of every support (1732) and active effect (1804) whose gem matches every keyword, by `calcLib.gemIsType` (`Modules/CalcTools.lua` 164 to 176): `skill` is a gem with `grants_active_skill` that is not a support; `non-exceptional` is a gem without `exceptional`; any other word is a gem tag.
- The gem is the gem instance's `gemData`, which only a gem given by `gemId` has. `data.gemForSkill` is keyed by the granted effect's table, not its ID (`Modules/Data.lua` 1094), so a skill an item grants (only `skillId`, `Classes/SkillsTab.lua` 1182) has none and gains no level.
- `calcLib.validateGemLevel` then clamps a level the effect has no row for.

### The engine

- **`loom_calc::poe1::gem_levels`.** `LEVEL_STATS` lists the 23 global gem level stats with their keywords: the words of each English description between "Level of all" and "Gems", lower-cased, as PoB's pattern reads them. A real-bundle test checks every row against its description, checks that every word is a gem tag or one of `gemIsType`'s words, and checks that no phrase is a gem's name. `is_type` is `gemIsType` without the name comparisons.
- **`Poe1Quantity::GemLevel(stat)`** (calc result v1 name `poe1.gem-level.<stat>`, additive): the mapping gives each row one unconditional `Base` effect.
- **`GemLevels`** sums these effects. Effects from a gem are left out, so a gem's own stat cannot raise the levels its stats were read at. `skill::main_skill` and `skill::group_skills` add each gem's matching total to its level (`GemEffectRef::level`, with the bonus kept apart as `bonus`). The gem's tags are its gem effect's (`GemEffects.Tags`, which PoB's exporter writes as `gem.tags`). So every reader of a gem's level sees the level PoB uses: the instance's stats, the stat set's level row (base damage, crit chance), the costs and reservation, and a support's cost multiplier. This holds for the main skill and for every other group's buffs.
- **Order.** `collect` now adds the gems after the config, so custom lines count. The hit phase reads the same totals from the finished store, because the steps after the gems (charges, damage taken, buffs) add none. With the reorder alone, every document without a gem level stat stayed byte-identical.
- **Left as reasons:** gem levels that depend on the socket ("+1 to Level of Socketed Gems" and its kin), a gem named in a line ("+1 to Level of all Raise Spectre Gems"), conditional lines, `supported_*_gem_level_+` (a support's own stat on what it supports), and gem quality modifiers.
- **Not traced:** the `GemLevel` effects feed no graph node. A gem's level reaches the result only through the values it changes, so a document does not show why a gem is above the level the build writes. `GemEffectRef::bonus` holds the number. A "gem level" leaf with its sources is a follow-up.
- **Unverified against the game:** a level past the effect's last row is clamped to it (PoB's rule). The fixtures stay inside the rows.

### Mechanic fixtures

| Fixture | What it isolates |
|---|---|
| `ice-nova-gem-levels-witch.xml` | Ice Nova with Increased Critical Strikes and Added Cold Damage, Herald of Ice in another group. Lines for cold, spell, herald, and fire skill gems and for critical and non-exceptional support gems: Ice Nova gains 2 levels, Increased Critical Strikes 3, Added Cold Damage 1, Herald of Ice 3 (it has the `spell` tag). Every target equals PoB's |
| `smite-gem-levels-templar.xml` | Smite with Melee Physical Damage, Precision in another group. Lines for strength, aura, dexterity, melee, and physical skill gems and for non-exceptional support gems: Smite (it has the `aura` tag) gains 3, Melee Physical Damage 1, Precision 3. Average damage, crit chance and multiplier, hit chance, and unreserved mana equal PoB's. Speed and DPS differ by Q11's floored attack time alone (1000 / 667 against PoB's 1.50, times the same 0.85 and 0.9): DIV-0036 to DIV-0038 (`game-truth`) |

### Review

`code-architecture-reviewer` found no blockers and checked `is_type`, the support and active handling, the item-granted case, the ordering, and the clamp against PoB. Applied:
- effects from a gem are left out of the totals, and the two read points are documented;
- a whole-number check on the sum;
- the real-bundle test checks that no phrase is a gem name;
- a test for a gem's own level stat;
- doc wrapping.

Recorded rather than changed: the provenance gap ("Not traced") and the clamp past the last row.

### Evidence (P3-8i)

Maintainer-run on 2026-11-13, on Windows, from the main checkout on a clean tree: `local/p3/p3-8i/evidence.sh` at tool commit `7a7c173`. The review's `31cd0b5` leaves every calc document byte-identical: public and corpus were re-run and compared. P3-12i's bundle and overlay are used. Summaries are in `docs/phase-3/stat-mapping/p3-8i-*`.

| Run | Result |
|---|---|
| `oracle-run`, the whole manifest | 117 fixtures, every expectation met, 3/3 identical runs each |
| `calc-run`, public, twice | 117 documents; 1693 complete, 334 partial, 445 unsupported, 312 indeterminate. Byte-identical twice |
| `calc-parity`, public | 38 registered differences (DIV-0036 to DIV-0038 new), 0 unexpected; 1665 fields match. **Every mechanic target met: 1659 of 1659 over 80 fixtures** |
| `calc-run` and `calc-parity`, corpus (21, `--anonymous`), twice | Byte-identical; no difference |

**Against P3-8h** (`p3-8i-public-vs-p3-8h.json`, `p3-8i-corpus-poe1-26530219-vs-p3-8h.json`). No field state changes. Values changed in 4 public builds, every one toward PoB's:

| Build | Value | Before | After | PoB |
|---|---|---|---|---|
| Ice Nova campaign (lab 1) | average damage | 4024 | 4953 | 31434 |
| Ice Nova campaign | average damage | 9459 | 11642 | 142914 |
| Ice Nova mapping | average damage | 23523 | 62402 | 2335701 |
| Ice Nova mapping | crit chance | 77.14 | 78.00 | 100 |
| Ice Nova mapping | crit multiplier | 4.25 | 4.31 | 4.48 |
| Ice Nova mapping | energy shield | 1674 | 1733 | 2255 |
| Lacerate | average damage | 180.5 | 188.2 | 24757 |

Each build's hit DPS follows its average damage. The mapping build's energy shield and crit values rose because its aura and Increased Critical Damage gems gained levels too.

Reasons:
- **Public:** 31 gone in 13 builds (every global gem level stat), none added.
- **Corpus:** 13 gone in 6 builds, none added. One build's average damage and DPS moved toward PoB's.

Tests: 1226 workspace tests, clippy, fmt, wasm32, and the real-bundle tests.

## P3-8j: minion modifiers that also affect you

The in-scope Smite act5 and act10 builds take Spiritual Command ("Increases and Reductions to Minion Attack Speed also affect you") and Spiritual Aid ("... Minion Damage also affect you"), with five nodes of minion attack and cast speed and Redemption's minion damage. These were the in-scope builds' most frequent unmapped stats after golems: 24 occurrences. Until P3-8j each was `unmapped-stat`, and `minion_damage_+%` was `NoEffect`, guarded by the redirects in `NO_EFFECT_DEPENDS_ON`.

### What PoB v2.67.2 does

- `Modules/ModParser.lua` reads "Minions deal #% increased Damage", "Minions have #% increased Attack Speed", "... Cast Speed", and "... Attack and Cast Speed" as `MinionModifier` LIST modifiers: `Damage` INC; `Speed` INC with `ModFlag.Attack`, with `ModFlag.Cast`, and with no flag. `Data/SkillStatMap.lua` 1900 to 1931 does the same for a gem's own stats.
- It reads the redirects as flags (3455 to 3462): `MinionDamageAppliesToPlayer`, `MinionAttackSpeedAppliesToPlayer`, and `MinionCastSpeedAppliesToPlayer`. "... at #% of their value" also sets `ImprovedMinionDamageAppliesToPlayer` MAX #.
- `Modules/CalcOffence.lua` 613 to 643 runs for the skill being computed. With a flag set, every `MinionModifier` of the skill's modifier list becomes the player's at the multiplier (the `Improved...` MAX, else 100%), keeping its source and tags. `Damage` INC stays `Damage` INC with its flags. `Speed` INC with no flag or the Attack (Cast) flag becomes `Speed` INC with `ModFlag.Attack` (`ModFlag.Cast`). Nothing is floored. Only the life redirect floors (`CalcPerform.lua` 1566), and no stat here reaches it.

### The engine

- **`loom_calc::poe1::minions`.** `STATS` maps the four minion stats to `Poe1Quantity::Minion(kind)` (`Increased`; calc result v1 name `poe1.minion.<kind>`, additive). It maps the four redirects (damage, damage at 150%, attack speed, cast speed) to `Poe1Quantity::MinionToYou(kind, percent)` (`poe1.minion-to-you.<kind>.<percent>`). A redirect is a `Flag` with its percent in the quantity, because a redirect line has no value: its stat value came through as 0 in the first trial.
- **`minions()`** runs last in `collect`. For each kind with a redirect, the largest percent counts (PoB's flag with its MAX), and each minion effect of that kind becomes the character's (`Damage` INC, or `Speed` INC for attacks or casts) at that percent, with the minion effect's source and conditions. A minion effect without a redirect changes no slice field, because no minion is computed.
- **A gem's minion stats.** Only the main skill's gems are read through `SkillMapping`, so their minion stats are the entries of PoB's `skillModList` and follow the redirects too (the review's finding). An example is Minion Damage supporting Absolution with Spiritual Aid. Other groups' gems never reach this mapping.
- **Left as reasons** (`minions::DEPENDS_ON`, a test checks each is `Unmapped`): the Dominating Blow and Absolution redirect, the skill stat that applies minion damage to all damage, "Modifiers to Minion Damage also affect you" (PoB does not read it, so applying it would be a divergence), and `additive_minion_damage_modifiers_apply_to_you_at_%_value` (no description). Other minion stats PoB reads as `Damage` or `Speed` (minion melee damage, damage on full life) and minion modifiers that name a skill (golems, spectres) are also left as reasons. A real-bundle test checks each row's text and that every "Minion Damage / Attack Speed / Cast Speed also affect" description is a row or in `DEPENDS_ON`.
- `minion_damage_+%` leaves `NO_EFFECT`, and its guards leave `NO_EFFECT_DEPENDS_ON`.

### Mechanic fixtures

| Fixture | What it isolates |
|---|---|
| `ice-nova-minion-modifiers-witch.xml` | Ice Nova with minion damage, cast speed, and attack speed, and the redirects of minion damage (also at 150%: the larger counts) and of minion cast speed. Ice Nova gains 30% increased damage and 10% increased cast speed; the minion attack speed reaches nothing. Every target equals PoB's |
| `smite-minion-modifiers-templar.xml` | Smite with a normal Siege Axe, minion damage, attack, cast, and attack and cast speed, and the redirects of minion damage and of minion attack speed. Smite gains 20% increased damage and 16% increased attack speed; the minion cast speed reaches nothing. Average damage, crit, hit chance, and unreserved mana equal PoB's. Speed and DPS differ by Q11's floored attack time alone (1000 / 667 against PoB's 1.50, times the same 0.85 and 1.16): DIV-0039 to DIV-0041 (`game-truth`) |

### Review

`code-architecture-reviewer`: no blockers. One important finding: the first commit gave a gem's own minion stats no effect, but PoB converts the skill's own `MinionModifier` entries too. That would have been a silent drop. Applied in `373b6dc`: those stats are minion effects that `minions()` converts. Also applied: tests for a gem's minion damage and for other minion damage stats staying unmapped, a stale comment, and the module docs. The reviewer confirmed that applying the conversion globally differs from PoB's per-skill list only for modifiers tagged with a skill, which are all unmapped, and that the tag conversion (`getConvertedModTags`) changes none of the mapped stats. Recorded rather than changed: DIV-0039 to DIV-0041 name the Q11 test, which is the mechanic they register.

### Evidence (P3-8j)

Maintainer-run on 2026-11-13, on Windows, from the main checkout on a clean tree: `local/p3/p3-8j/evidence.sh` at tool commit `70e9864`. The review's `373b6dc` leaves every calc document byte-identical: public and corpus were re-run and compared. P3-12i's bundle and overlay are used. Summaries are in `docs/phase-3/stat-mapping/p3-8j-*`.

| Run | Result |
|---|---|
| `oracle-run`, the whole manifest | 119 fixtures, every expectation met, 3/3 identical runs each |
| `calc-run`, public, twice | 119 documents; 1738 complete, 334 partial, 448 unsupported, 312 indeterminate. Byte-identical twice |
| `calc-parity`, public | 41 registered differences (DIV-0039 to DIV-0041 new), 0 unexpected; 1708 fields match. **Every mechanic target met: 1704 of 1704 over 82 fixtures** |
| `calc-run` and `calc-parity`, corpus (21, `--anonymous`), twice | Byte-identical; no difference |

**Against P3-8i** (`p3-8j-public-vs-p3-8i.json`, `p3-8j-corpus-poe1-26530219-vs-p3-8i.json`). No field state changes. Values changed in 2 public builds, every one toward PoB's:

| Build | Value | Before | After | PoB |
|---|---|---|---|---|
| Smite act5 | average damage | 1461 | 2029 | 15434 |
| Smite act5 | speed | 1.997 | 2.278 | 2.748 |
| Smite act5 | hit DPS | 2917 | 4622 | 42417 |
| Smite act10 | average damage | 1977 | 2733 | 31852 |
| Smite act10 | speed | 2.097 | 2.378 | 2.869 |
| Smite act10 | hit DPS | 4146 | 6501 | 91370 |

Reasons:
- **Public:** 74 gone in 5 builds (the two Smite builds, and three minion builds whose minion stats and redirects are no longer reasons), none added.
- **Corpus:** 23 gone in 3 builds, none added. One build's average damage and DPS rose (Spiritual Aid).

Tests: 1239 workspace tests, clippy, fmt, wasm32, and the real-bundle tests.

## P3-8k: the enemy's exposure and the guaranteed shock

The four in-scope Elementalist builds (the three Ice Nova builds and Winter Orb) take Shaper of Storms ("Hits always Shock", "Shocks from your Hits always increase Damage taken by at least 25%", "All Damage can Shock"), Mastermind of Discord ("Exposure you inflict applies an extra -25% to the affected Resistance"), the lightning ailment effect nodes, and the mastery "10% Chance to Inflict Cold Exposure on Hit with Cold Damage". These were the in-scope builds' most frequent unmapped stats after golems: 20 occurrences. Until P3-8k each was `unmapped-stat`, and the three exposure checks were `NAMED_ONLY`, guarded by their stats in `config::DEPENDS_ON`. The Ice Nova campaign and mapping builds set the shocked and cold exposure checks in their active config sets, so their damage moves toward PoB's. Winter Orb sets the cold exposure check too, but its damage is not computed (Winter Orb is not a modelled skill), so it only loses reasons.

### What PoB v2.67.2 does

Line numbers are the pinned checkout's (`local/oracle/pob`). Some earlier docs cite the newer legacy checkout's numbers.

- **Exposure.** The checks `conditionEnemyFireExposure`, `conditionEnemyColdExposure`, and `conditionEnemyLightningExposure` (`Modules/ConfigOptions.lua` 1947 to 1955) give the enemy `<Type>Exposure` BASE -10. Each is conditioned on the player's `CanApply<Type>Exposure`. `calcs.perform` (`Modules/CalcPerform.lua` 375 to 385) sets that condition when the main skill's or the character's `<Type>ExposureChance` sums above 0. The chance's value is read nowhere else. The chance comes from modifiers ("#% Chance to Inflict Cold Exposure on Hit with Cold Damage", `ModParser.lua` 4194; "Cold Skills have a #% chance to apply Cold Exposure on Hit", 4191 to 4193; "Inflict Fire, Cold, and Lightning Exposure on Hit", 4200) and from the `elementalArmyExposureType` input (`ConfigOptions.lua` 413 to 420). PoB applies a config input whatever its `ifSkill` (`Classes/ConfigTab.lua` 1083 to 1101). Then, per type (3563 to 3589), unless Elemental Equilibrium is on and the enemy was hit by that type, PoB takes the strongest exposure on the enemy, adds each `ExtraExposure` and `Extra<Type>Exposure` BASE of the character, and makes the result the enemy's `<Type>Resist` BASE. That value is capped by `ExposureMin`'s OVERRIDE. With no override, `modDB:Override` returns no value, so `m_min(min, ...)` is `m_min(min)`. An oracle probe on the pinned runtime confirmed it: Ice Nova's damage against the default enemy's 50% cold resistance rose by the ratio of a 40% and a 15% resistance, with no error.
- **The guaranteed shock** (3499 to 3546). The shock is `floor(min(max(override, ShockVal), max))`. `ShockVal` is the configured effect (only while the shocked check is on). For each `ShockMinimum` (3503 to 3530), `effect = value x calcLib.mod(modDB, nil, "EnemyShockEffect") x calcLib.mod(enemyDB, nil, "SelfShockEffect")`; the minimums add up into `minimum`, and `override` keeps the largest of any effect and that sum. A minimum does not set the enemy's `Condition:Shocked` (only `ShockOverride` does), so it raises the damage taken only with the check on. "#% increased Effect of Lightning Ailments", "... of Non-Damaging Ailments", and "... of Shock" are `EnemyShockEffect` INC (`ModParser.lua` 772 to 786). The flag `ChecksHighestDamage` makes PoB read the main skill's list instead (3517).
- "Hits always Shock" (`EnemyShockChance` BASE 100) and "All Damage can Shock" (`<Type>CanShock` flags) feed only the shock chance and the shock PoB shows (`CalcOffence.lua` 4108, 5208 on). They never feed the enemy's shock in the hit.

### The engine

- **`loom_calc::poe1::exposure`** (new). `STATS` maps the chances to `Poe1Quantity::ExposureChance(type)` and the extras to `Poe1Quantity::ExtraExposure(None | Some(type))`; each is `Base` and unconditional. The calc result v1 names are `poe1.exposure-chance.<type>` and `poe1.extra-exposure[.<type>]`, additive. `exposure()` runs once every source is in. It turns Elemental Army's input into the chance of its type. Then, for each check that is on, when the chance of its type sums above 0, it gives the enemy `Poe1Quantity::EnemyResistance(type)` (`poe1.enemy-resistance.<type>`) `Base` -10, with the check's source, and one more per extra of every type or of that type, with the extra's source. The check alone, without a chance, gives nothing, as in PoB. A check that is not a boolean, or an Elemental Army value outside the list, is a reason.
- **The hit** (`hit::effective_resistance`) adds the `EnemyResistance` sum to the enemy's base resistance before the maximum, only when there is one, so a hit without exposure keeps its graph.
- **`exposure::DEPENDS_ON`** holds everything that would make the sum differ from PoB's. These stay reasons: other exposures (skills', "Nearby Enemies have Fire Exposure", Hydrosphere, Wave of Conviction, a flask, at maximum rage, on suppressing), the minimum override, Elemental Equilibrium and its effect, other chances (a gem's or minion's, socketed, devotion, curse-cast, graft), and the fire and lightning "with Fire (Lightning) Damage" chances. PoB reads those two through its generic parser with a `FireHasDamage` condition (`ModParser.lua` 1081 to 1083), not as the cold one's own pattern. The exposure stats P3-12f put in `config::DEPENDS_ON` moved here; the rows are no longer reasons.
- **The shock** (`poe1::damage_taken`). `SHOCK_STATS` maps the three effect stats to `Poe1Quantity::ShockEffect` (`Increased`, `poe1.shock-effect`) and the minimum to `Poe1Quantity::ShockMinimum` (`Base`, `poe1.shock-minimum`). With the shocked check on, `shock()` computes PoB's value from the configured effect and the unconditional minimums, in order, each times `1 + inc / 100`. No row is `More`, so PoB's rounded more factor is 1, and no enemy modifier gives `SelfShockEffect`. The effect's source is the configured input, or the first minimum's when the minimums are stronger. The shock's other sources are no graph node, a documented gap like the charges' count. `SHOCK_DEPENDS_ON` holds Pure Elementalist's more effect, the Elementalist's "more Effect of Lightning Ailments ... if the highest Damage Type is Lightning" (its `ChecksHighestDamage` flag), conditional effects, the maximum and its override, and stacking shocks.
- **`NoEffect`**: "Hits always Shock" and "All Damage can Shock" (`ailment-chance`).
- `collect` now runs `exposure()` and `damage_taken()` after the buffs and the minion redirects, once every source is in. Only the enemy's quantities move, and nothing else reads them.
- A gem's shock effect or exposure stat stays a reason through the gem mapping, as before. PoB scales a guaranteed shock by the character's `EnemyShockEffect`, not the skill's (without `ChecksHighestDamage`).
- A maintainer-run test (`tests/exposure_shock.rs`) checks each row's text in the real bundle and that every `DEPENDS_ON` stat exists. It also checks that every stat whose text names exposure, shock, or the effect of lightning or non-damaging ailments is a row, `Unmapped`, or a listed `NoEffect` row. The exposure debuff's own description file is skipped: it words the resistance stats as exposure.

### Mechanic fixtures

| Fixture | What it isolates |
|---|---|
| `ice-nova-exposure-witch.xml` | Ice Nova against an enemy exposed to cold and to fire, with a cold exposure chance and the extras -25% (every type), -5% (cold), and -7% (fire). The enemy's cold resistance is 50 - 10 - 25 - 5 = 10; the fire check has no fire chance. Every target equals PoB's |
| `ice-nova-guaranteed-shock-witch.xml` | Ice Nova against a shocked enemy (10% configured), with a 25% minimum shock, 30% increased effect of lightning ailments, 20% of non-damaging ailments, and "Hits always Shock". The shock is 25 x 1.5 = 37.5, floored to 37. Every target equals PoB's. "All Damage can Shock" is left out: as a custom line it is `only-shadowed` (the game's only description of the stat is shadowed in its file), while passive nodes carry the stat by ID |
| `smite-exposure-shock-templar.xml` | Smite with a normal Siege Axe against an enemy exposed to lightning and shocked (10%), Elemental Army's type set to lightning (the chance, whatever the skill), an extra -25%, a 25% minimum shock, and 20% increased effect of shock. The lightning damage meets 15% resistance, and the shock is 30. Average damage, crit, hit chance, and unreserved mana equal PoB's. Speed and DPS differ by Q11's floored attack time alone (1000 / 667 against PoB's 1.50, times the same 0.85): DIV-0042 to DIV-0044 (`game-truth`) |

### Review

`code-architecture-reviewer`: no blockers. It checked each PoB site (the gate, the checks, Elemental Army, the exposure and shock arithmetic, the enemy's resistance order in `CalcOffence.lua` 461 to 474, `ShockMinimum` not setting `Condition:Shocked`) and that the 26 stats moved from `config::DEPENDS_ON` are all rows or in `exposure::DEPENDS_ON`. Findings:
- **The chances are unconditional, looser than the game** ("Cold Skills have ...", "... with Cold Damage"). Kept as PoB reads them, with the reason in the module docs: the check states that the enemy is exposed, and the chance only gates whether the character can expose it at all. Exposure is a debuff on the enemy, so once any of the character's skills applies it, every hit meets it. No value the game gives differs, so there is no divergence entry.
- Applied: a provenance sentence in `exposure`'s docs (the chance is no graph node); the sign comment on the extras (PoB's `(%-?%d+)` does not parse "+5"; every such stat rolls negative); `config-inputs.md` points to this section.
- Recorded, not changed: the shared `unconditional` helpers in two modules (`charges` has a third), and further tests (a resistance floor with exposure, a placeholder check). The engine sums the exposure into the same `Resistance` path the floor already reads, and placeholder checks are ignored by the same `last_input` as before.

### Evidence (P3-8k)

Maintainer-run on 2026-11-13, on Windows, from the main checkout on a clean tree: `local/p3/p3-8k/evidence.sh` at tool commit `8c038c5`. The review's changes after it are comments and docs only. P3-12i's bundle and overlay are used. Summaries are in `docs/phase-3/stat-mapping/p3-8k-*`.

| Run | Result |
|---|---|
| `oracle-run`, the whole manifest | 122 fixtures, every expectation met, 3/3 identical runs each |
| `calc-run`, public, twice | 122 documents; 1805 complete, 334 partial, 453 unsupported, 312 indeterminate. Byte-identical twice |
| `calc-parity`, public | 44 registered differences (DIV-0042 to DIV-0044 new), 0 unexpected; 1773 fields match. **Every mechanic target met: 1771 of 1771 over 85 fixtures** |
| `calc-run` and `calc-parity`, corpus (21, `--anonymous`), twice | Byte-identical; no difference |

**Against P3-8j** (`p3-8k-public-vs-p3-8j.json`, `p3-8k-corpus-poe1-26530219-vs-p3-8j.json`). No field state changes. Values changed in 2 public builds, every one toward PoB's:

| Build | Value | Before | After | PoB |
|---|---|---|---|---|
| Ice Nova Elementalist campaign | average damage | 11642 | 24026 | 142914 |
| Ice Nova Elementalist campaign | hit DPS | 37255 | 76884 | 563490 |
| Ice Nova Elementalist mapping | average damage | 62402 | 147034 | 2335701 |
| Ice Nova Elementalist mapping | hit DPS | 186314 | 439002 | 11486312 |

Both builds remain far below PoB's values because of their other reasons (golems, Arcane Surge, Spell Boost, and the other gems).

Reasons:
- **Public:** 36 gone in 12 builds, none added. The builds are five Elementalist builds (Ice Nova campaign and mapping, Winter Orb, Lightning Trap, Penance Brand), builds with non-damaging ailment effect or a cold exposure chance, and two builds whose `elementalArmyExposureType` input was a reason.
- **Corpus:** 15 gone in 8 builds, none added. No value changed.

Tests: 1249 workspace tests, clippy, fmt, wasm32, and the real-bundle tests (10, `tests/exposure_shock.rs` among them).
