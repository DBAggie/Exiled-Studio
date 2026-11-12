# Phase 3: enemy model (P3-11)

Plan step 11 (`dev/active/phase-3-calc-slice/phase-3-calc-slice-plan.md`): the enemy PoB v2.67.2's config describes. No slice field is the enemy's own; the hit fields read it (P3-9b spells, P3-10 attacks), where the oracle checks it through hit damage. Decisions: context file, "Decisions in P3-11".

## What is modelled (`crates/loom-calc/src/poe1/enemy.rs`)

PoB's config tab applies its options in list order (`Classes/ConfigTab.lua`, `BuildModList`). A list option applies its input, and list inputs start at their `defaultIndex`; a count option applies its input when set (and not 0 unless `countAllowZero`), else its placeholder. `enemyIsBoss` (default `Pinnacle`) sets the later options' placeholders:

| Kind | Level placeholder | Resistances (fire, cold, lightning, chaos) | Armour and evasion placeholders |
|---|---|---|---|
| `None` | none | none (no modifier) | the monster level's |
| `Boss` | none | 40, 40, 40, 25 | the monster level's |
| `Pinnacle` (default) | 84 | 50, 50, 50, 30 | `round(table x mean / 100)`, means over the 22 bosses |
| `Uber` | 85 | 50, 50, 50, 30 | the same over the 7 uber bosses |

- **Level** (`UpdateLevel`, `env.enemyLevel`): the `enemyLevel` input when above 0, else the kind's placeholder, else the character's level, capped at `MaxEnemyLevel` (85). The armour and evasion placeholders read the table at the enemy's level, at least the kind's (84 or 85).
- **Tables:** evasion is the game's `DefaultMonsterStats.Evasion`. Armour is PoB's derivation from the game constant `MonsterDamageReductionImprovement` (`Export/Scripts/miscdata.lua`, "The table here is wrong so we generate it instead"): `floor((10 + 2 x level) x (1 + c / 100)^level)`. The power is `math::pow_int`, repeated multiplication with only IEEE-exact operations, so every platform gives the same bits, and it equals PoB's table at all 100 levels.
- **Means** (`data.bossStats`): `100 + total / count` over P3-1d's boss list, from the bundle's monster types.
- **Inputs win over placeholders:** `enemyLevel`, `enemyFireResist`, `enemyColdResist`, `enemyLightningResist`, `enemyChaosResist`, `enemyArmour`, `enemyEvasion`. A build's saved placeholders are overwritten by the kind's.
- **Problems, never a silent value:** an `enemyIsBoss` value PoB does not list; an input of another type; a missing table row or constant.

The model's numeric inputs are modelled (no `unmodelled-config` reason). `enemyIsBoss` stays a reason: its kind also gives modifiers the engine does not ingest yet (the player's `WarcryPower` and `Multiplier:EnemyPower` +20, the enemy's `Condition:RareOrUnique`, `Condition:PinnacleBoss`, `AilmentThreshold` MORE, and the Uber boss's `DamageTaken` MORE -70, which changes hit damage). P3-9b models them before a hit field is `Complete`; the default kind's apply even without an input. The armour table is capped at PoB's 100 levels, and the character level is clamped to 1..100 as PoB's.

Not modelled here (the hits apply them): the enemy's maximum resistance, penetration, exposure, curses, damage taken, and the enemy's damage (`enemyPhysicalDamage` and the like; no slice field reads it).

## Evidence (2026-11-12, Windows)

| Run | Result |
|---|---|
| Real PoE1 bundle (P3-1d), maintainer-run (`LOOM_BUNDLE_DIR=<bundle> cargo test -p loom-calc --test enemy -- --ignored`, clean `46a8bf5`) | **Equal to PoB:** the default enemy is level 84 with armour 61,989, evasion 10,143, and resistances 50/50/50/30, as PoB v2.67.2 saved in 71 fixture builds (`<Placeholder>`); the armour table equals PoB's `data.monsterArmourTable` at all 100 levels, and no value before the floor lies within 1e-9 of a whole number (so C's `pow` and repeated multiplication cannot floor apart) |
| `calc-run`, public and corpus, twice each, on the P3-1d bundle (clean `46a8bf5`) | Byte-identical; field states as P3-7's |
| `calc-parity`, public, against P3-7's oracle run, with the overlay run (`local/p3/p3-1d/parity-1/overlay.json`) | No difference; 425 fields and 178 breakdown terms match; every mechanic target met. A model-only step: no field reads the enemy yet, so this shows only that nothing else changed |
| `calc-parity`, corpus | No difference (all indeterminate) |

Tests (`crates/loom-calc/tests/enemy.rs`, 5 and 1 maintainer-run): the Pinnacle default; each boss kind's placeholders; inputs over placeholders (0 resistance, a level above the cap, a level of 0); problems; PoB's armour table from the constant. `tests/poe1.rs`: enemy inputs modelled, and a bad value still a reason. 1075 workspace tests, clippy, fmt, wasm32.
