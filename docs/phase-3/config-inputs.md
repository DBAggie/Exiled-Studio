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
- `<untyped>`: type PoB's `Placeholder` values (PoB applies a count input's placeholder when the input is unset) and custom modifier blocks.
- Inputs with their own effects, as the in-scope builds need them: charges, rage, fortification, shock, exposure (P3-12's next steps).
