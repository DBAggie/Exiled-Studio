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

**The model** (`loom_domain::ConfigInput`, `kind`). A numeric `Placeholder` is a config entry of kind `Placeholder`, beside the inputs in source order; the PoB reader types it and the writer writes it back. PoB loads a `string` placeholder as the input's value (`Load`, line 876), so that form, and one without a number, stays untyped (and is listed by `kept_whole`). Build document v1 gains an optional `kind` on a config entry, left out for a set value (`"kind": "placeholder"`); schema and TypeScript regenerated. Documents are strict, so a reader from before this change refuses a document with a placeholder (`DTO_JSON`, not `MIGRATE_TOO_NEW`) rather than dropping it. This is the first change to build v1 since Phase 2, made additive rather than as a v2 with a migration, on the rule calc result v1 already follows (`dto::calc::v1`): additive changes until the Phase 3 gate, every reader being in this repository and nothing released. Accepted by the maintainer 2026-11-12 (keep v1, no migration).

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

**Not done.** PoB's `CustomModifierBlock` (custom modifiers, newer builds) stays untyped: `<untyped>` remains in 9 public builds, all from such a block: empty in four (Smite act1 among them), real modifiers in five (Smite act2, act5, and act10: "Perfect Forest Warrior", "Onslaught", "Culling Strike"; two others). Typing the blocks, with an empty block as no effect and each line as a reason until it is modelled, is the next config step. (Corrected in P3-12c: Smite act1's active set 5 holds the same three lines as act2, act5, and act10. Done in P3-12c, below, in full: the lines are resolved and applied.)

## P3-12c: custom modifier blocks

**Why.** PoB's custom modifiers (Configuration tab, "Custom Modifiers") are text the player writes, applied to the character as modifiers. PoB saves them per config set as `<CustomModifierBlock title enabled>` with the text inside (`Classes/ConfigTab.lua`, `Save`, line 944), and applies every line of every enabled block with text (`BuildModList`, lines 1106 to 1128): each line goes through `StripEscapes` and a trim of Lua's `%s`, then `modLib.parseMod`; a line it parses is added to the config's modifiers. The model kept the blocks untyped, so every build that has one had the reason `unmodelled-config <untyped>`, whatever the blocks held. The maintainer asked for the blocks in full (2026-11-12): the lines act, as in PoB.

**The model.**
- `loom_domain::CustomModifierBlock` (on `ConfigSet::custom_modifiers`): title, enabled, the text as read, and its `lines` (`CustomModifierLine`: the modifier text and a `LineResolution`, as an item's modifier line). `Config::check_custom_lines`: a custom line never continues another line (`DOMAIN_CONTINUED_CUSTOM_LINE`; the build document refuses one both ways).
- The PoB reader types the blocks after a set's last other child, where PoB writes them; a block elsewhere, one whose `enabled` is not `true` or `false` (PoB reads a missing one as true, any other as false), or one with an element inside stays untyped in place and is listed by `kept_whole`. The lines are PoB's: `StripEscapes` (`HeadlessWrapper.lua` 75: `^` and a digit, then `^x` and six hex digits, two passes), the `%s` trim, empty lines left out. PoB's XML reader trims each text node and drops a whitespace-only one (`runtime/lua/xml.lua`), so an indented empty block has no text in PoB; ours keeps the whitespace for export and has no line, the same effect. The writer writes the text back unchanged.
- Resolution (`loom_parser::import::Resolver`, `resolve_custom_modifiers`): every pending line of every block, read alone in the bundle's default scope (character-wide: no item base) with the case-folding tolerance. With no base, no mod row can settle an ambiguous line, so it stays `ambiguous`; half of a description over two lines, read alone, has no template. A range stays a range (`unsettled-value`).
- The engine (`Ingest::custom_line`): each line of an enabled block of the active set acts through its resolution and the game's mapping, under the new source `custom-modifier` (block and line indexes); a line that did not resolve is `unresolved-line` (also `pending-line`, `excluded-line`) with that source, as an item line is. A disabled block does nothing.
- Build document v1 gains a set's optional `custom_modifiers` (with `lines` and their resolutions); calc result v1 gains the source `custom-modifier`. Both additive, on P3-12b's rule; schemas and TypeScript regenerated.
- Content directly under `<Config>` (an old layout; PoB loads it into set 1) was ignored by the engine with no reason; it is now `<untyped>`.

**What PoB reads that the game data does not.** PoB's parser knows its own shorthand, which is not a game text: "Onslaught" (PoB gives the Onslaught condition, 20% increased attack and cast speed), "Culling Strike", "Perfect Forest Warrior". Such a line does not resolve against the stat descriptions (or is ambiguous) and stays a reason. A game text can also render a stat the player did not mean: "100% more Spell Damage" resolves to Pain Attunement's `pain_attunement_keystone_spell_damage_+%_final` (its description has no condition text), which is unmapped, so a reason, where PoB adds generic more spell damage. Both are reasons, never silent values.

PoB's parser is its own grammar (`Modules/ModParser.lua`), and it drops a line it reads only in part (`mods and not extra`); ours reads game text through the stat descriptions. So a line can apply in one and not the other. Each such line is a reason here, never a value; no compared fixture differs (below), so the divergence register gains no entry.

**What changes in the builds.** No field value changes in any public or corpus document. In the 9 public builds whose blocks gave `<untyped>`, that reason is gone; lines that do not apply are named instead (39 reasons added in 15 builds, `p3-12c-public-vs-p3-12b.json`):
- The 4 in-scope Smite builds: "Perfect Forest Warrior" `unresolved-line` `no-template`, "Onslaught" `ambiguous`, and "Culling Strike", which resolves to `kill_enemy_on_hit_if_under_10%_life` and is `unmapped-stat`. Their state stays `Partial`.
- Two community builds' blocks (Facebreaker, Forged Frostbearer): 6 `unmapped-stat` (Pride, curses, damage taken as fire) and 15 `unresolved-line`.
- Four builds whose blocks are empty lose `<untyped>` and gain nothing.
- **The old config layout.** Six builds keep inputs directly under `<Config>`, PoB's layout before config sets (`Load` puts them into set 1). They now give `<untyped>`: PoB's five 3.13 test builds and `discrepancy-movement-speed.xml`. Their inputs were ignored before with no reason, a silent path. `discrepancy-movement-speed.xml` held only a legacy `customMods` line (movement speed, no slice field), so its 17 `Complete` fields become `Partial`. Modelling that layout is a follow-up.
- The 3 in-scope Ice Nova builds are unchanged: their custom modifiers are the legacy `customMods` input (follow-ups).

### Evidence

2026-11-12, Windows; tool commit `a8e034c`, clean. Inputs as P3-12b's: bundle `local/p3/p3-1d/bundle-poe1-a`, overlay `local/p3/p3-1d/parity-1/overlay.json`; a new oracle run for the new fixture. Summaries: `docs/phase-3/config-inputs/p3-12c-*`.

| Run | Builds | Result |
|---|---|---|
| `oracle-run`, the whole manifest | 87 | Every expectation met (86 `ok`, `malformed.xml` `ORACLE_IMPORT`), 3/3 identical runs each; the new fixture among them |
| `calc-run`, public, twice | 87 | 86 documents; 1008 complete (+5: +22 the new fixture, -17 `discrepancy-movement-speed.xml`), 351 partial, 393 unsupported, 312 indeterminate. Byte-identical twice |
| Documents against P3-12b's run | 86 | **No field value changes.** States: one build (above). Reasons: 9 gone, 39 added in 15 builds (above; `p3-12c-public-vs-p3-12b.json`) |
| `calc-run --data-overlay`, public | 87 | The same field states |
| `calc-parity`, public, with the overlay run, the register, and the manifest | 87 | 25 differences, all registered (DIV-0001 to DIV-0025, unchanged), 0 unexpected, no unused entry, no missing mechanic fixture; 990 fields match. **Every mechanic target met (991 of 991, 50 fixtures).** Exit 5 (`incomplete`), as before |
| `calc-run` and `calc-parity`, corpus (21, `--anonymous`), twice | 21 | All `indeterminate` (older trees), byte-identical, no difference; against P3-12b's run no value or state change; reasons: 9 builds gain `<untyped>` (the old layout), 1 an `unresolved-line`, 2 lose `<untyped>` (`p3-12c-corpus-poe1-26530219-vs-p3-12b.json`: kinds, sources, and stat IDs only, no text) |
| `build-round-trip`, public and corpus | 82, 21 | All pass (blocks typed, their text written back) |

**The new fixture** (`mechanic/ice-nova-custom-modifiers-witch.xml`): `ice-nova-witch.xml` with three blocks: an empty one; an enabled one of five game-text lines (`+30% to Cold Resistance`, `^2+20 to maximum Life^7`, `^xFF0000+20% to Global Critical Strike Multiplier`, `50% increased Spell Damage`, `10% increased Cast Speed`, with a blank line and a `^1` line between them); and a disabled one with three more. The engine's 22 compared fields are `Complete` and equal PoB's (`TotalDPS` 1739.93; the bare fixture 1039.35).

**The disabled block** (local only, `local/p3/p3-12c/negative/`): the same build with the third block enabled. PoB: life 1245, fire resistance -20, `TotalDPS` 2899.75. The engine matches on all 22 fields here too (44 of 44 over both builds). PoB applies a block only when enabled, and so does the engine.

**A probe of PoB-only lines** (local only, `local/p3/p3-12c/probe/`): ten lines in one block, PoB against the engine. The engine equals PoB on every field the resolved lines reach (life 1145, cold resistance 75, critical strike multiplier 1.7). It differs only where its lines are reasons: "Onslaught" (PoB +20% cast speed: `Speed` 1.857 against 1.571) and "100% more Spell Damage" (PoB doubles `AverageDamage`).

Tests (1141 workspace tests, 8 new):
- The PoB reader: blocks typed after the inputs with their lines, written back; other blocks kept whole; PoB's line rules (trim, Lua spaces, colour codes, two passes).
- The DTO golden with a block's lines and resolutions, and a continued custom line refused both ways.
- `resolve_build`: each outcome of a custom line on the synthetic bundle, idempotent, and unresolved lines make the build `Partial`.
- `tests/poe1.rs`: an enabled block's resolved line applies with its source in the graph, disabled and empty blocks do nothing, each unresolved state is its reason, and a ranged line is unsettled. Content under `<Config>` and a block kept whole are `<untyped>`.

Clippy, fmt, wasm32; schemas and TypeScript regenerated (the TypeScript type order moves; generated), UI type check.

Commands: as P3-12a's, with `<out>` = `local/p3/p3-12c/evidence` (script `local/p3/p3-12c/evidence.sh`), plus `build-round-trip` of `tools/oracle/fixtures`, `mechanic`, and `community` and of the corpus.

### Follow-ups

- **The legacy `customMods` input** (done in P3-12d, below; the 3 in-scope Ice Nova builds, several community builds). PoB moves it into a block when the set has no block with text (`Load`, line 876). roxmltree normalises a line break in an attribute value to a space, so the lines cannot be split from the domain's value. The reader must read the attribute's raw text. Our writer also writes a line break in an attribute as `&#10;`, which PoB's reader turns into nothing (`xml.lua` decodes only five named entities). Both are needed before those lines can act; the input stays `unmodelled-config` meanwhile.
- **The old config layout** (inputs directly under `<Config>`): type it as set 1, as PoB does, writing it back without a set.
- **PoB's shorthand** ("Onslaught", "Culling Strike"): the in-scope Smite builds need Onslaught's buff and culling strike modelled, from the buff side (P3-12) rather than as text.
- **Quarantine:** custom lines count in `completeness` but are not listed by `Build::quarantine` (per item line); a quarantine entry for config content would need a new shape.

## P3-12d: the legacy `customMods` input

**Why.** Before custom modifier blocks, PoB kept custom modifiers in one config input, `<Input name="customMods" string="...">`, its lines separated by raw line breaks inside the attribute. The 3 in-scope Ice Nova builds and 10 other community builds still hold them that way. PoB v2.67.2 still reads them (`Classes/ConfigTab.lua`, `Load`, 872 to 884): for each set, a non-empty `customMods` becomes the set's only block (title "Default", enabled, so PoB's modifier source is `Custom:Default`) when the set has no block, or one block whose text is empty, even a disabled one; otherwise PoB discards it. In every case PoB then clears the input, so `BuildModList`'s fallback for it (1129 on) never runs after a load. The input was `unmodelled-config customMods`.

**Reading the value as PoB does.** PoB's XML reader (`runtime/lua/xml.lua`, `parseAttribs`, `decodeContent`) takes an attribute's raw text between the quotes, so line breaks, tabs, and carriage returns stay, and decodes `&lt;`, `&gt;`, `&amp;`, `&apos;`, `&quot;`; any other reference (`&#10;`) becomes nothing. XML (roxmltree) turns a raw line break into a space, which joined the lines. The PoB reader now reads every config input's `string` value from the raw text (`Attribute::range_value`), decoded as PoB does, guarded against roxmltree's value. It keeps the element whole, listed by `kept_whole`, in two cases:
- A raw `>`. XML allows it, but PoB's tag pattern ends there, so PoB reads no value.
- Any other reference. PoB drops it without a trace; here it stays a reason (ADR-015).

The writer writes such a value as PoB does: only `& < > " '` escaped, line breaks, tabs, and carriage returns raw. Before, it wrote `&#10;`, which PoB reads as nothing. A carriage return stays in the value (a CRLF file gives `\r\n`; PoB on Windows reads files in text mode and sees `\n`); the lines are the same either way (Lua's `%s` trim). In the public fixtures and the corpus only `customMods` values change under this reading (23 inputs in 12 public builds, 14 in 5 corpus builds); no input holds another reference or a raw `>` (`local/p3/p3-12d/string_scan.py`). Known limit: an element kept whole for a raw `>` or a reference is written back as XML read it (`&gt;`, the decoded character), which PoB reads, so export and re-import of that one element differ.

**The model.** `ConfigInput.lines` (build document v1: an input's optional `lines`, additive on P3-12b's rule): the PoB reader derives a `customMods` string's lines with P3-12c's rules (`StripEscapes`, the `%s` trim, empty lines left out); the resolver resolves them as a block's (default scope, no base). `Config::check_custom_lines` also refuses a continued input line (`DOMAIN_CONTINUED_CUSTOM_LINE`). `Build::completeness` counts them, whether they apply or not (as a disabled block's lines): it says what the import read.

**The rule** (`loom_calc::poe1::collect::legacy_custom_mods`). The last `customMods` input of the active set (PoB's inputs are a table) acts as PoB's migration says: its lines act through `Ingest::custom_line` under the new calc result v1 source `config-input-line` (input, line; additive), when the set has no block or one block with whitespace-only text. Otherwise PoB discards the input: no effect, no reason. `false` is PoB's empty text (nothing). It stays `unmodelled-config customMods` when PoB's outcome is not known:
- the value is a number or `true` (PoB then fails on the block's text);
- the set has untyped content, which can hold another input or block;
- content outside any set when the set is set 1 (PoB loads that content there).

Lines on any other input (a build document can carry them) are `unmodelled-config` under its name.

**What changes in the builds** (`p3-12d-public-vs-p3-12c.json`). In 13 public builds `unmodelled-config customMods` is gone and the lines that do not act are named instead (52 reasons: `unresolved-line` `no-template` or `ambiguous`, and `unmapped-stat`). The 3 in-scope Ice Nova builds: "+180% to all resistances" resolves and applies; "Ice Nova Buffs" and "Mana Infused Staff" (labels), "+1.5% to critical strike chance" and "20% more damage" (ambiguous without a base), the two "more block chance" lines (PoB's own grammar), and "30% less mana cost of skills" (Hierophant's `hierophant_mana_cost_+%_final`, unmapped) stay reasons. Their state stays `Partial`.

**Values.** 5 builds' resistances change, through "+N% to all resistances" lines, all in `Partial` fields:
- 19 of the 20 changed values now equal PoB's, against none before (cold resistance -7 became 75, PoB 75).
- Shock Nova's lightning resistance is 76, PoB 82: a higher maximum from content the engine does not model (the field stays `Partial`).
- No `Complete` field changes. The corpus: 3 builds, 3 `customMods` reasons gone, 19 line reasons added, no value or state change.

`discrepancy-movement-speed.xml` (the old layout) stays `<untyped>` (next step).

### Evidence

2026-11-12, Windows; tool commit `565c890`, clean. Inputs as P3-12c's; a new oracle run for the new fixture. Summaries: `docs/phase-3/config-inputs/p3-12d-*`.

| Run | Builds | Result |
|---|---|---|
| `oracle-run`, the whole manifest | 88 | Every expectation met (87 `ok`, `malformed.xml` `ORACLE_IMPORT`), 3/3 identical runs each |
| `calc-run`, public, twice | 88 | 87 documents; 1030 complete (+22, the new fixture), 351 partial, 395 unsupported, 312 indeterminate. Byte-identical twice |
| Documents against P3-12c's run | 87 | No state change. Values: 5 builds' resistances (above). Reasons: 13 gone, 52 added in 13 builds |
| `calc-run --data-overlay`, public | 88 | The same field states |
| `calc-parity`, public, with the overlay run, the register, and the manifest | 88 | 25 differences, all registered (DIV-0001 to DIV-0025, unchanged), 0 unexpected, no unused entry, no missing mechanic fixture; 1012 fields match (+22). **Every mechanic target met (1013 of 1013, 51 fixtures).** Exit 5 (`incomplete`), as before |
| `calc-run` and `calc-parity`, corpus (21, `--anonymous`), twice | 21 | Byte-identical, no difference; no value or state change; reasons as above (`p3-12d-corpus-poe1-26530219-vs-p3-12c.json`: kinds, sources, and stat IDs only) |
| `build-round-trip`, public (fixtures, community, mechanic) and corpus | 83, 21 | All pass |

**The new fixture** (`mechanic/ice-nova-legacy-custom-mods-witch.xml`):
- `ice-nova-witch.xml` with two `customMods` inputs. The first, `+100 to maximum Life`, is superseded. The second has five game-text lines with raw line breaks, colour codes, a tab, and blank lines.
- The set's only block is empty and disabled.
- PoB: life 1155 (+30, so the first input does not count), cold resistance -35, critical strike multiplier 1.65, `TotalDPS` 1647.38 (bare: 1039.35).
- The engine's 22 compared fields are `Complete` and equal PoB's.

**Negative controls** (local only, `local/p3/p3-12d/negative/`), the fixture changed three ways:
- A block with text: PoB discards the input, and only the block's `+10 to maximum Life` acts (life 1135).
- Two empty blocks: PoB discards it (life 1125, the bare value).
- The second input's first line break written as `&#10;`: PoB joins the first two lines and applies neither.

The engine matches PoB on all 44 fields of the first two. On the third its fields are `Partial`: the input is kept whole (`<untyped>`).

Tests (1150 workspace tests, 9 new):
- The PoB reader:
  - raw line breaks kept, the lines read, written back raw;
  - other strings without lines;
  - the five entities in either quote, and `&amp;#10;`;
  - another reference, or a raw `>`, keeps the element whole;
  - CRLF and LF give the same lines;
  - a numeric legacy value has no lines.
- The DTO golden with an input's lines, and a continued input line refused both ways.
- `resolve_build` resolves input lines.
- `tests/poe1.rs`, every branch of the rule:
  - no block, and one empty block (enabled or not), apply the lines;
  - a block with text, or two blocks, give nothing;
  - the last input counts;
  - empty text and `false` do nothing;
  - unresolved lines are reasons under `config-input-line`;
  - a number, `true`, or untyped content are the reason;
  - another set does not act;
  - lines on another input are a reason.

Clippy, fmt; schemas and TypeScript regenerated, UI type check.

Commands: as P3-12c's, with `<out>` = `local/p3/p3-12d/evidence` (script `local/p3/p3-12d/evidence.sh`), plus `build-round-trip` of `mechanic`.

### Follow-ups

- **The old config layout** (inputs directly under `<Config>`, PoB's set 1; `discrepancy-movement-speed.xml`, PoB's 3.13 test builds). There, PoB's `NewConfigSet(1)` already holds one empty default block, so one block read in that layout makes two and the migration never applies (plan review).
- **A block's text and references:** a block's text is element text, read by XML's rule (`&#10;` a line break), while PoB reads any reference other than the five as nothing; and the writer writes a carriage return in text as `&#13;`. Not reached by any fixture; align with PoB's reading (or keep such a block whole) with the old layout.
- PoB's shorthand and grammar ("Onslaught", "more" lines, labels): reasons until the buffs are modelled.
