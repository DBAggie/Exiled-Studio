# Phase 3: config inputs (step 12)

Plan step 12 (`dev/active/phase-3-calc-slice/phase-3-calc-slice-plan.md`): each config input is modelled, listed as not affecting the slice (with a reason and a test), or makes the fields it can reach `Partial`. Summaries: `docs/phase-3/config-inputs/`.

Before this step the engine modelled two kinds of input (`resistancePenalty`, P3-6; the enemy model's numeric inputs, P3-11). Every other input of the active config set was an `unmodelled-config` reason on every computed field, whatever it did.

## P3-12a: inputs that only set named conditions

**The rule** (`crates/loom-calc/src/poe1/config.rs`, `NAMED_ONLY`). In PoB v2.67.2 most config inputs only add a named condition or multiplier (`Condition:BeenHitRecently`, `Multiplier:NearbyEnemies`; `Modules/ConfigOptions.lua`), which only modifiers read. Such an input changes no computed field, because every stat the engine ingests is one of three kinds:

- mapped: no mapping reads a named condition (`mapping.rs` tests the global and hit rows; local and weapon rows are built with none);
- `NoEffect`: it changes no slice field whatever the config (P3-8a's criterion, `stat-mapping.md`);
- a reason (`unmapped-stat`, `unmodelled-gem`, ...), which already makes every computed field `Partial`.

A later mapping that reads a named condition is still never silent: the engine sets no named condition from the config, so the store leaves such an effect out with `unmodelled-condition` (`crates/loom-calc/src/condition.rs`).

An input is listed only if PoB's calc modules (`CalcSetup.lua`, `CalcPerform.lua`, `CalcOffence.lua`, `CalcDefence.lua`) read its names only through modifiers or flags a modifier sets, or add from them only effects outside the slice. The 16 inputs are those the in-scope builds use, plus their neighbours in the public fixtures; the fixtures use more inputs of the same kind (`conditionKilledRecently`, `multiplierBeenHitRecently`, ...), each a candidate for the table once checked the same way:

| Input | Sets (PoB `ConfigOptions.lua` line) | What PoB's calc modules do with it |
|---|---|---|
| `OverloadedIntensity`, `intensifyIntensity` | `Multiplier:OverloadedIntensity` (517), `Multiplier:Intensity` (514) | Nothing; the intensity gems' skill stats read them |
| `bloodSandStance` | `Condition:SandStance` for Sand (654) | `CalcSetup.lua` 652 adds `Condition:BloodStance` otherwise; the stance skills' stats read them |
| `conditionBeenHitRecently`, `conditionBlockedRecently`, `conditionChilledEnemyRecently`, `conditionShockedEnemyRecently` | The condition of that name (1375, 1421, 1339, 1351) | Only through modifiers or flags a modifier sets (`CalcDefence.lua` 707, 723: maximum block if not blocked recently, from a stat) |
| `conditionCritRecently` | `Condition:CritRecently`, `Condition:SkillCritRecently` (1251) | Nothing |
| `conditionFullLife` | `Condition:FullLife` (174) | `CalcPerform.lua` 86 sets it for Chaos Inoculation; never read |
| `conditionConvergence` | `Condition:Convergence`, itself conditioned on `Condition:CanGainConvergence` (1441) | `CalcSetup.lua` 129 and `CalcPerform.lua` 789 turn it into more elemental damage, but only with `CanGainConvergence`, which only a modifier sets ("Gain Convergence when you Hit a Unique Enemy", `ModParser.lua` 2680) |
| `conditionEnemyChilled`, `conditionEnemyChilledEffect` | The enemy's `Condition:Chilled`, `Condition:ChilledConfig` (1899); `ChillVal`, `DesiredChillVal` (never read) (1907) | `CalcPerform.lua` 3527: less action speed for the enemy (no slice field); damage taken only with a modifier's flag (`ChillEffectIncDamageTaken` and others, `ModParser.lua` 3877) or Bonechill (a gem) |
| `conditionEnemyOnConsecratedGround` | The enemy's `Condition:OnConsecratedGround` (2012) | `CalcPerform.lua` 3688: damage taken from `DamageTakenConsecratedGround`, which only a modifier gives |
| `conditionOnConsecratedGround` | `Condition:OnConsecratedGround`, for minions too (1148) | `CalcPerform.lua` 804, 3171: life regeneration and less curse effect on you (no slice field); accuracy only from `ConsecratedGroundAlsoAccuracy` (a modifier) |
| `conditionStationary` | `Multiplier:StationarySeconds`; `Condition:Stationary` above 0 (158) | Nothing |
| `multiplierNearbyEnemies` | `Multiplier:NearbyEnemies`, `Condition:OnlyOneNearbyEnemy` (1234) | Nothing |

**What the rule depends on** (`DEPENDS_ON`). Each modifier path above starts at a game stat, and the rule holds only while that stat is a reason. The 11 stats: block if not blocked recently (4), consecrated ground's enemy damage taken and accuracy, chill's damage taken (`enemies_chilled_by_hits_take_damage_increased_by_chill_effect`, `keystone_cold_purist`), `gain_convergence_on_hitting_unique_enemy`, and Bonechill's two stats. Bonechill is the sharp case: PoB's effect is the gem's skill data (`supportBonechill`, `Data/Skills/sup_int.lua` 560; `CalcPerform.lua` 3536, cold damage taken equal to the configured chill) and its stat map ignores the two stats, so they look like candidates for `NoEffect`; classifying them so would make `conditionEnemyChilled` silent. A test asserts each of the 11 is `Unmapped` through every mapping: mapping one must revisit `NAMED_ONLY`.

**Still reasons** (their own effects; later P3-12 steps): charges (`use*Charges`), `multiplierRage` (more attack damage), `buffFortification`, `conditionEnemyShocked` and `conditionShockEffect` (damage taken), `conditionEnemyColdExposure`, `summonLightningGolemEnableWrath` (a minion's aura), `customMods`, `bandit`, the pantheon, `enemyIsBoss`, and `<untyped>`: PoB's `Placeholder` values and custom modifier blocks, which the model keeps untyped. PoB applies a count input's placeholder when the input is unset (`Classes/ConfigTab.lua`, `BuildModList`), so placeholders can change a result; typing them is a follow-up.

**What changes:** reasons only. No value, state, or graph changes in any document (below). In the 7 in-scope builds, the config reasons fall from 18 (Ice Nova) and 19 (Smite) to 10 each; every field stays `Partial` through the remaining reasons (other gems, unmapped stats, choices, and the config inputs above).

| Build | Config reasons before | After |
|---|---|---|
| `ice-nova-elementalist-*` (3) | 18 | 10: `<untyped>`, `bandit`, `conditionEnemyColdExposure`, `conditionEnemyShocked`, `customMods`, the pantheon (2), `summonLightningGolemEnableWrath`, `useEnduranceCharges`, `usePowerCharges` |
| `smite-inquisitor-act*` (4) | 19 | 10: `<untyped>`, `buffFortification`, `conditionEnemyShocked`, `conditionShockEffect`, `multiplierRage`, the pantheon (2), the three charges |

### Evidence

2026-11-12, Windows; tool commit `23a1a5c`, clean (the runs before it, on the uncommitted tree, gave byte-identical documents). Inputs as P3-8b's and the Q11 switch's: bundle `local/p3/p3-1d/bundle-poe1-a`, overlay `local/p3/p3-1d/parity-1/overlay.json`; a new oracle run for the new fixture.

| Run | Builds | Result |
|---|---|---|
| `oracle-run`, the whole manifest | 83 | Every expectation met (82 `ok`, `malformed.xml` `ORACLE_IMPORT`), 3/3 identical runs each; the new fixture among them |
| `calc-run`, public, twice | 83 | 82 documents (`malformed.xml` fails import, as expected); 936 complete (+22, the new fixture), 334 partial, 386 unsupported, 312 indeterminate. Byte-identical twice |
| Documents against the Q11 switch's run | 82 | **No field value, state, or graph changes.** Reasons only: 107 `unmodelled-config` reasons gone in 26 builds, none added |
| `calc-run --data-overlay`, public | 83 | The same field states |
| `calc-parity`, public, with the overlay run, the register, and the manifest | 83 | 25 differences, all registered (DIV-0001 to DIV-0025, unchanged), 0 unexpected, no unused entry; 918 fields match (+22). **Every mechanic target met (902 of 902, 46 fixtures).** Exit 5 (`incomplete`), as before: 34 real builds have fields that are not `Complete` |
| `calc-run` and `calc-parity`, corpus (21, `--anonymous`), twice | 21 | All `indeterminate` (older trees), byte-identical, no difference |

**The new fixture** (`mechanic/ice-nova-config-conditions-witch.xml`): `ice-nova-witch.xml` with all 16 inputs set. PoB's values equal the bare fixture's and the engine's fields are all `Complete` and match. It shows PoB's `apply` of each input does nothing on a build with no modifier, which is the positive half only: on a bare build no modifier can read the names, so it cannot show that the argument above holds.

**The negative control** (local only, `local/p3/p3-12a/negative/`; two builds: Ice Nova with Bonechill, without and with `conditionEnemyChilled` and a 30% chill). PoB: `TotalDPS` 1184.97 without the chill, 1540.47 with it (+30%, Bonechill's cold damage taken). The engine: both builds `Partial`, through Bonechill's stats (`support_chills_also_grant_cold_damage_taken_+%_equal_to_slow_amount` and its chilling-area twin, `support_bonechill_cold_damage_+%_final`, `chill_effect_+%`), with the same value (1039.35, no Bonechill); the chilled build has no config reason. The input that changes PoB's hit is never a silent value: the stat it needs is a reason. It is not a mechanic fixture, since its targets cannot be `Complete` until Bonechill is modelled.

Tests (1126 workspace tests, 5 new): `poe1::config` (the table sorted and unique; the listed inputs alone; no overlap with the modelled inputs; each `DEPENDS_ON` stat `Unmapped` through the global, skill, piece, and weapon mappings); `tests/poe1.rs`, every listed input with a true, false, number (3 and 0), and string value gives the bare build's states and values, all together too, and an input with its own effects (charges, shock, fortification) stays the only reason beside one. Clippy, fmt, wasm32.

Commands (`<out>` a new folder):

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

### Follow-ups

- More inputs of the same kind (the public fixtures' other `condition*` and `multiplier*` inputs), each checked as above.
- `<untyped>`: type PoB's `Placeholder` values (PoB applies a count input's placeholder when the input is unset) and custom modifier blocks. Placeholders: done in P3-12b (below).
- Inputs with their own effects, as the in-scope builds need them: charges, rage, fortification, shock, exposure (P3-12's next steps).

## P3-12b: placeholders

**Why.** PoB saves a placeholder (`<Placeholder name="ShockStacks" number="1"/>`) for every option that has one, and applies a numeric option's placeholder when its input is unset, or 0 unless the option is `countAllowZero` (`Classes/ConfigTab.lua`, `BuildModList`, line 1147), through the option's own `apply`; its calc modules also read some placeholders directly (`env.configPlaceholder`: the enemy's damage, the distances). The model kept placeholders untyped, so every build PoB saved since placeholders exist had the reason `unmodelled-config <untyped>`, whatever they held.

**The model** (`loom_domain::ConfigInput`, `kind`). A numeric `Placeholder` is a config entry of kind `Placeholder`, beside the inputs in source order; the PoB reader types it and the writer writes it back. PoB loads a `string` placeholder as the input's value (`Load`, line 876), so that form, and one without a number, stays untyped (and is listed by `kept_whole`). Build document v1 gains an optional `kind` on a config entry, left out for a set value (`"kind": "placeholder"`); schema and TypeScript regenerated. Documents are strict, so a reader from before this change refuses a document with a placeholder (`DTO_JSON`, not `MIGRATE_TOO_NEW`) rather than dropping it. This is the first change to build v1 since Phase 2, made additive rather than as a v2 with a migration, on the rule calc result v1 already follows (`dto::calc::v1`): additive changes until the Phase 3 gate, every reader being in this repository and nothing released. **For the maintainer to confirm** (context file, "Decisions in P3-12b").

**The rule** (`crates/loom-calc/src/poe1/config.rs`). A placeholder is classified as its input, because PoB applies it through the same `apply`:

| Placeholders | Class | Why (PoB v2.67.2) |
|---|---|---|
| `ScorchStacks`, `ShockStacks` | `NAMED_ONLY` | The enemy's `Multiplier:ScorchStacks`, `ShockStacks` (`ConfigOptions.lua` 1892, 1951); `CalcPerform.lua` 3550, 3566 read them only with `ScorchCanStack` or `ShockCanStack`, which only a modifier sets (`ModParser.lua` 3045, 3047) |
| `aspectOfLunarisLunarProtection`, `aspectOfLunarisLunarResistance`, `aspectOfSolarisSunscaldStacks`, `configSpectralTigerCount`, `multiplierHallowingFlameStacksRemovedByAlly`, `sigilOfPowerStages` | `NAMED_ONLY` | Multipliers (262, 265, 269, 644, 1994, 631) the calc modules never read; the skills' stat maps and modifiers do (Hallowing Flame: `ModParser.lua` 2898) |
| `multiplierEnemyHallowingFlame` | `NAMED_ONLY` | The enemy's `Multiplier:HallowingFlame` (1991); never read (`CalcPerform.lua` 3440 reads the condition, and only with a modifier's `CanInflictHallowingFlame`) |
| `multiplierWitheredStackCountSelf` | `NAMED_ONLY` | `Multiplier:WitheredStack` (1231); `CalcPerform.lua` 852: chaos damage taken by you (no slice field), only with `CanBeWithered` |
| `meleeDistance`, `projectileDistance` | `NAMED_ONLY` | No `apply` (1789, 1790); `CalcActiveSkill.lua` 457 makes the input or placeholder the skill's `skillDist`, which only modifiers' `DistanceRamp` and `MeleeProximity` tags read (`ModStore.lua` 599, 619); `CalcOffence.lua` 1083, 1086 add such modifiers for `PointBlank` and `FarShot`, modifiers' flags |
| `enemyLevel`, the four resistances, `enemyArmour`, `enemyEvasion` | `enemy::RESET_PLACEHOLDERS` | `enemyIsBoss`, earlier in PoB's list, sets them on every run for each kind it knows (2087 to 2219); the enemy model derives them (P3-11). For an unknown kind PoB would read the saved ones, and the enemy model reports `UnknownBossKind` |
| `enemy{Physical,Lightning,Cold,Fire,Chaos}Damage`, `enemy{Lightning,Cold,Fire}Pen`, `enemyPhysicalOverwhelm`, `enemyCritChance`, `enemyCritDamage`, `enemySpeed`, `enemyDamageRollRange` | `ENEMY_DAMAGE` (inputs too) | No `apply` (2352 to 2376); only `calcs.buildDefenceEstimations` reads them (`CalcDefence.lua` 1737 to 1757, 2965, 3552: damage taken, effective life, maximum hit; no slice field), and `presetBossSkills` reads the roll range to set the damage placeholders (2286). `enemyPhysicalOverwhelm` is here, not reset: Pinnacle and Uber leave it |

Any other placeholder is `unmodelled-config` under its input's name. The enemy model and the resistance penalty read set inputs only. A placeholder's reason has the same form as its input's (`unmodelled-config` and the name), so a report does not tell them apart.

**What the rule depends on.** `DEPENDS_ON` gains 18 stats, each asserted `Unmapped` through every mapping and (maintainer-run, `tests/enemy.rs`) present in the real bundle: shock and scorch stacking (`raider_shocks_you_apply_can_stack_up_to_50_times`, `can_apply_additional_scorch`), and the distance modifiers: the projectile and arrow ramps (6), `slayer_damage_+%_final_from_distance`, `keystone_point_blank`, the Point Blank support on an item, `player_far_shot`, `gain_player_far_shot_while_do_not_have_iron_reflexes`, and four skill stats (Lancing Steel, Shattering Steel, Shield Crush, Close Combat support); and the Hallowing Flame modifier that reads `multiplierHallowingFlameStacksRemovedByAlly` (`physical_damage_%_to_add_as_lightning_damage_per_hallowing_flame_consumed_by_ally_up_to_80%`, `ModParser.lua` 2898). Freezing Pulse and Eye of Winter add their distance modifiers in their own skill code, outside any stat; neither skill is modelled.

**What changes:** reasons only. No value, state, or graph changes in any document. In the 3 in-scope Ice Nova builds `<untyped>` is gone (9 config reasons left: `bandit`, `conditionEnemyColdExposure`, `conditionEnemyShocked`, `customMods`, the pantheon (2), `summonLightningGolemEnableWrath`, two charges); the 4 Smite builds keep it through their custom modifier block (below).

### Evidence

2026-11-12, Windows; tool commit `803e7cf`, clean (the run on the uncommitted tree before it gave byte-identical documents). Inputs as P3-12a's: bundle `local/p3/p3-1d/bundle-poe1-a`, overlay `local/p3/p3-1d/parity-1/overlay.json`; a new oracle run for the new fixture. Summaries: `docs/phase-3/config-inputs/p3-12b-*`.

| Run | Builds | Result |
|---|---|---|
| `oracle-run`, the whole manifest | 86 | Every expectation met (85 `ok`, `malformed.xml` `ORACLE_IMPORT`), 3/3 identical runs each; the new fixture among them |
| `calc-run`, public, twice | 86 | 85 documents; 1003 complete (+22, the new fixture), 334 partial, 391 unsupported, 312 indeterminate. Byte-identical twice |
| Documents against P3-8c's run | 85 | **No field value, state, or graph changes.** Reasons only: 22 gone in 19 builds (`<untyped>` 19, `projectileDistance` 2, `meleeDistance` 1, inputs now `NAMED_ONLY`), none added (`p3-12b-public-reasons-vs-p3-8c.json`) |
| `calc-run --data-overlay`, public | 86 | The same field states |
| `calc-parity`, public, with the overlay run, the register, and the manifest | 86 | 25 differences, all registered (DIV-0001 to DIV-0025, unchanged), 0 unexpected, no unused entry, no missing mechanic fixture; 985 fields match (+22). **Every mechanic target met (969 of 969, 49 fixtures).** Exit 5 (`incomplete`), as before |
| `calc-run` and `calc-parity`, corpus (21, `--anonymous`), twice | 21 | All `indeterminate` (older trees), byte-identical, no difference; against P3-8c's run no value or state change, 12 reasons gone in 11 builds (`<untyped>` 10, `meleeDistance` 2), none added |
| `build-round-trip`, public and corpus | 81, 21 | All pass (the reader now types placeholders; export and re-import equal) |

**The new fixture** (`mechanic/ice-nova-config-placeholders-witch.xml`): `ice-nova-witch.xml` with a saved placeholder for each of the 31 classified inputs at a value PoB would show if it used it (the enemy's cold resistance 0, level 1, armour 0; shock stacks 5; distances 5 and 90), and two enemy damage inputs. PoB's values equal the bare fixture's on all 24 compared fields (`TotalDPS` 1039.35 in both): PoB really resets the enemy's placeholders, and the rest change nothing here. The engine's fields are `Complete` and match, with no config reason. As for P3-12a, this is the positive half: on a bare build no modifier reads the names.

**The other boss kinds** (local only, `local/p3/p3-12b/kinds/`; the fixture runs only the default Pinnacle, whose resets differ from the others'). The same build with `enemyIsBoss` `None` and `Uber`, each bare and with the fixture's placeholders and inputs (oracle 3/3 each): every slice key of PoB's `main_output` is equal between bare and placeholders (`TotalDPS` 2078.71 for None, 311.81 for Uber). The only keys that differ are PoB's damage-taken estimates (`TotalEHP`, `PhysicalTakenHit`, `enemySkillTime`, `totalEnemyDamage`, and 9 others), from the two enemy damage inputs: `ENEMY_DAMAGE` reaches PoB's output, but no slice field. Not a mechanic fixture: an `enemyIsBoss` input stays a reason, so the engine's fields are `Partial`.

Tests (1133 workspace tests, 5 new): the PoB reader (a numeric placeholder typed beside the inputs, written back; a string or non-numeric one kept whole and listed); the DTO golden with a placeholder; `poe1::config` (`ENEMY_DAMAGE` sorted and apart from the other tables; `DEPENDS_ON` with the 16 new stats `Unmapped` through every mapping); `tests/poe1.rs`, every classified placeholder at 0, 1, and 99 gives the bare build's states and values, enemy damage inputs are quiet, and any other placeholder (`conditionShockEffect`, a `resistancePenalty` placeholder) is the only reason under its name; `tests/enemy.rs`, saved placeholders at 1 leave each boss kind's enemy unchanged. Clippy, fmt, wasm32; schema and TypeScript regenerated, UI type check.

Commands: as P3-12a's (above), with `<out>` = `local/p3/p3-12b/evidence`, plus `build-round-trip` (`AGENTS.md`).

**Not done.** PoB's `CustomModifierBlock` (custom modifiers, newer builds) stays untyped: `<untyped>` remains in 9 public builds, all from such a block: empty in four (Smite act1 among them), real modifiers in five (Smite act2, act5, and act10: "Perfect Forest Warrior", "Onslaught", "Culling Strike"; two others). Typing the blocks, with an empty block as no effect and each line as a reason until it is modelled, is the next config step.
