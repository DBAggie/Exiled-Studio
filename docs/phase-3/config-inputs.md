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

**Still reasons** (their own effects; later P3-12 steps; charges and fortification since P3-12e, below): charges (`use*Charges`), `multiplierRage` (more attack damage), `buffFortification`, `conditionEnemyShocked` and `conditionShockEffect` (damage taken), `conditionEnemyColdExposure`, `summonLightningGolemEnableWrath` (a minion's aura), `customMods`, `bandit`, the pantheon, `enemyIsBoss`, and `<untyped>`: PoB's `Placeholder` values and custom modifier blocks, which the model keeps untyped. PoB applies a count input's placeholder when the input is unset (`Classes/ConfigTab.lua`, `BuildModList`), so placeholders can change a result; typing them is a follow-up.

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

- **The old config layout** (inputs directly under `<Config>`, PoB's set 1; `discrepancy-movement-speed.xml`, PoB's 3.13 test builds). There, PoB's `NewConfigSet(1)` already holds one empty default block, so one block read in that layout makes two and the migration never applies (plan review). Done in P3-12e (below).
- **A block's text and references** (done in P3-12e: such a block is kept whole): a block's text is element text, read by XML's rule (`&#10;` a line break), while PoB reads any reference other than the five as nothing; and the writer writes a carriage return in text as `&#13;`. Not reached by any fixture; align with PoB's reading (or keep such a block whole) with the old layout.
- PoB's shorthand and grammar ("Onslaught", "more" lines, labels): reasons until the buffs are modelled.

## P3-12e: the old config layout, block text, charges, fortification

Several steps in one PR (maintainer, 2026-11-12: as many of the remaining Phase 3 tasks as possible). Shock, rage, and exposure were looked at and left for later steps (Follow-ups).

### The old config layout

**Why.** Before config sets, PoB saved inputs, placeholders, and blocks directly under `<Config>`. PoB still loads every such child into set 1 (`Classes/ConfigTab.lua`, `Load`, lines 888 to 902), creating the set with `NewConfigSet(1)`, which already holds one empty enabled block (line 1281); the blocks read are appended after it. Since P3-12c that content was one `<untyped>` reason.

**The model.**
- `loom_domain::ConfigLayout` (`Sets`, `Flat`) on `Config`. A flat config holds one set: ID 1, no title, no attributes, nothing untyped beside it. `Config::check_layout` enforces this (`DOMAIN_FLAT_CONFIG_LAYOUT`); the build document refuses a broken one both ways.
- Build document v1 gains an optional `layout` (`"flat"`; left out for `sets`). It is additive, on P3-12b's rule. Schema and TypeScript are regenerated.
- The PoB reader types the layout when `<Config>` has an element child and no `ConfigSet` child. A comment alone does not count. A mix of both layouts keeps its loose children untyped, because PoB's result then depends on their order.
- The writer writes the set's content back without a set element. A flat config that breaks the check (only a build never read through a document can) is written with set elements, so nothing is dropped.
- `kept_whole` lists such content as `Config/Input` and the like.

**The rule** (`loom_calc::poe1::collect::legacy_custom_mods`). In the flat set, PoB's default empty block comes before the blocks read:
- With no block read, the legacy `customMods` lines apply, as before.
- With one block read, even an empty one, the set has two blocks. PoB discards the input, and only that block's lines act.

### Block text PoB reads otherwise

PoB reads a block's text with its own XML reader (`runtime/lua/xml.lua`):
- it trims the text;
- it decodes the five named entities and reads any other reference as nothing;
- it keeps a CDATA section as a separate node.

XML decodes `&#10;` as a line break and a CDATA section as text. A block whose raw content holds either is now kept whole, as P3-12d does for attribute values: it is a reason, and the text is never silently changed.

### Charges

`loom_calc::poe1::charges`, following PoB v2.67.2:
- **Inputs.** `usePowerCharges`, `useFrenzyCharges`, and `useEnduranceCharges` (`ConfigOptions.lua` 900 to 917) turn each kind on.
- **Count.** It is `override<Kind>Charges` when set (the input, else its placeholder; 0 is unset), else the maximum. The maximum is the character constant `max_<kind>_charges` (3). The count is at least 0 (`doActorCharges`, `CalcPerform.lua` 980 to 1120). PoB does not clamp an override to the maximum or round a fractional one; neither does the engine.
- **Effects per charge**, from the character constants (`CalcSetup.lua` 625 to 630):

| Charge | Effect per charge | Engine effect |
|---|---|---|
| Power | 50% increased critical strike chance (`critical_strike_chance_+%_per_power_charge`) | One `CritChance` increased effect, 50 x count |
| Frenzy | 4% increased attack speed, 4% increased cast speed, 4% more damage (`base_attack_speed_+%_per_frenzy_charge`, `base_cast_speed_+%_per_frenzy_charge`, `object_inherent_damage_+%_final_per_frenzy_charge`) | `Speed` increased for attacks and casts, `Damage` more (one effect: 4 x count, as PoB's one modifier per charge kind) |
| Endurance | 4% physical and elemental damage reduction | None: damage taken is not a slice field |

- **Not known, so a reason** under the `use` input: a `use` value that is not a boolean, an override that is not a finite number, or lines on either input. PoB reads any value other than `false` as on; the engine is conservative.
- **Source.** The `use` input is each effect's source, whichever input set the count.
- **Why nothing else is needed.** Everything else that reads or changes a count is a modifier, so a stat: a different maximum or minimum, the "at maximum" flags, conversions to absorption, affliction, or brutal charges, and effects per charge. A test checks that no mapped row names a charge. The `NoEffect` rows that do (six: chances to gain a charge, regeneration per charge, shock immunity at maximum) are listed, each checked to reach no slice field. A second test checks that the maxima stay `Unmapped` through every mapping.

### Fortification

`buffFortification` and `overrideFortification` join `NAMED_ONLY`:
- They set `Condition:Fortified` and `FortificationStacks` (`ConfigOptions.lua` 1039, 1042).
- `CalcPerform.lua` (659 to 680) then sets the stacks, less damage taken when hit per stack (not a slice field), the `HaveMaximumFortification` flag, and one more `BuffOnSelf`; line 3889 exports the stacks to a party.
- Every other reader is a modifier.
- A test checks that no mapped row names fortification. The three `NoEffect` rows that do (a chance to fortify, its duration, regeneration per stack) are listed, and `base_max_fortification` stays `Unmapped`.

### What changes

- **Public builds.** Values change in 10 builds, all in `Partial` fields, and each moved toward PoB:

| Builds | Change |
|---|---|
| The 3 Ice Nova builds | Crit chance: power charges |
| The 4 Smite builds | Crit chance, attack rate, damage: power and frenzy charges |
| Elemental Hit, Kinetic Blast, Shock Nova | Crit chance; attack rate and damage where frenzy charges are used |

- **`discrepancy-movement-speed.xml`.** It is read in the old layout, and its `customMods` line ("10% increased Movement Speed", no slice field) applies. Its 17 character fields go from `Partial` to `Complete`.
- **Reasons.** 68 are gone and 42 added in 33 builds (`p3-12e-public-vs-p3-12d.json`):
  - gone: the charges (54), `buffFortification` (8), `<untyped>` (6);
  - added: the individual inputs that the old layout's `<untyped>` used to hide (`enemyIsBoss`, `buffOnslaught`, `conditionLeeching`, and others).
- **The 7 in-scope builds.** Their config reasons fall to 6 for Ice Nova (`bandit`, `conditionEnemyColdExposure`, `conditionEnemyShocked`, the pantheon (2), `summonLightningGolemEnableWrath`) and to 5 for Smite (`conditionEnemyShocked`, `conditionShockEffect`, `multiplierRage`, the pantheon (2)). Every field stays `Partial`, through other gems, unmapped stats, and these inputs.
- **Corpus.** Two builds' values change: one build's crit chance and attack rate from charges, and one old-layout build's chaos resistance from 16 to 75, PoB's value, once its old-layout config is read. Reasons: 29 gone, 104 added in 17 builds, mostly inputs and lines the old layout's `<untyped>` used to hide.

### Evidence

2026-11-12, Windows. Tool commit `f33bba2`, clean. Inputs are P3-12d's (bundle `local/p3/p3-1d/bundle-poe1-a`, overlay `local/p3/p3-1d/parity-1/overlay.json`), with a new oracle run for the three new fixtures. Summaries are in `docs/phase-3/config-inputs/p3-12e-*`.

| Run | Builds | Result |
|---|---|---|
| `oracle-run`, the whole manifest | 91 | Every expectation met, 3/3 identical runs each |
| `calc-run`, public, twice | 91 | 90 documents; 1114 complete, 334 partial, 400 unsupported, 312 indeterminate. Byte-identical twice |
| Documents against P3-12d's run | 87 | As above |
| `calc-run --data-overlay`, public | 91 | The same field states |
| `calc-parity`, public, with the overlay run, the register, and the manifest | 91 | 28 differences, all registered (DIV-0001 to DIV-0028; 3 new, below), 0 unexpected, no unused entry, no missing mechanic fixture; 1094 fields match. **Every mechanic target met (1080 of 1080, 54 fixtures).** Exit 5 (`incomplete`), as before |
| `calc-run` and `calc-parity`, corpus (21, `--anonymous`), twice | 21 | Byte-identical, no difference (exit 5) |
| `build-round-trip`: public (fixtures, community, mechanic, PoB's 3.13 test builds) and corpus | All | All pass |

**The new fixtures and the register.**

| Fixture | What it holds | Result |
|---|---|---|
| `ice-nova-old-config-layout-witch.xml` | The old layout: an enemy level placeholder and a three-line `customMods` | 22 of 22 equal to PoB (life 1155, cold resistance -35, average damage 1018.57) |
| `ice-nova-charges-witch.xml` | The three charges at their maximum | 22 of 22 equal to PoB (crit chance 18.75, cast rate 1.6, average damage 859.14) |
| `smite-charges-templar.xml` | One power charge (the override's placeholder), two frenzy charges, one endurance charge, fortified with 10 stacks | Crit, damage, and the rest equal to PoB |

On `smite-charges-templar.xml` the attack rate, `TotalDPS`, and its breakdown total differ. It is Q11's `game-truth` difference: a weapon's attack time in floored milliseconds (1000 / 667 = 1.4993 against PoB's 1.50), times the same 1.08 on both sides. These are registered as DIV-0026 to DIV-0028, with the evidence of DIV-0001 to DIV-0025 (`docs/phase-3/attack-hits.md`, "The switch"). No other divergence is claimed: the per-charge values are the game's character constants, which PoB also reads.

**Negative controls** (local only, `local/p3/p3-12e/negative/`). The old-layout fixture was run with one block "+10 to maximum Life" (PoB: life 1135) and with one empty disabled block (PoB: 1125, the bare value). In both PoB discards the input, and the engine matches PoB on every field compared.

**Tests** (1160 workspace tests):
- The PoB reader: the flat layout read and written back; mixed layouts; a comment alone; references and CDATA in a block; a broken flat build exported with set elements.
- `Config::check_layout`: ID, title, attributes, two sets, untyped content. The document refuses a broken flat config on write and on read.
- `tests/poe1.rs`: the migration in the flat layout.
- `tests/hit.rs`: power and frenzy counts (maximum, override, placeholder, 0, above the maximum, fractional, negative, last wins); `use` off; endurance; reasons for bad values and for lines on a charge input.
- The charge and fortification guards.
- Clippy, fmt, wasm32. Schema and TypeScript regenerated; UI type check.

**Review** (`code-architecture-reviewer`): PoB's semantics confirmed (the flat set's default block, counts not clamped, a frenzy "more" valued 4 x count, the `Combat` condition in the default mode). Applied:
- lines on a charge input are a reason (they were skipped);
- the guards cover `NoEffect` rows and the maxima;
- a non-finite override is a reason;
- endurance looks up no constant;
- the writer's fallback for a broken flat config;
- a comment alone is not the flat layout;
- tests, and the domain doc.

**Commands:** as P3-12d's, with `<out>` = `local/p3/p3-12e/evidence` (script `local/p3/p3-12e/evidence.sh`), plus `build-round-trip` of `mechanic` and PoB's 3.13 test builds.

### Follow-ups

Shock, rage, cold exposure, Onslaught, culling strike, and the bandit are done in P3-12f (below).

- **Shock** (`conditionEnemyShocked`, `conditionShockEffect`, all 7 in-scope builds). PoB gives a shocked enemy increased damage taken equal to the shock's effect: the configured `ShockVal`, at most 50, floored to its precision (`CalcPerform.lua` 3545 to 3640). The engine has no enemy damage taken in the hit yet; that comes first.
- **Rage** (`multiplierRage`). It acts only with `Condition:CanGainRage` or `RageRegen`, which about 25 modifier texts set (`ModParser.lua` 2201 to 2585, 5517 to 5539). Each needs its stat ID before rage can join `NAMED_ONLY` or be modelled (more attack damage per rage).
- **Cold exposure** (`conditionEnemyColdExposure`). It needs `CanApplyColdExposure`, also set by modifiers; same approach.
- **A block's `enabled` attribute** written with a character reference: XML reads it, PoB reads the reference as nothing. This is contrived and not in any fixture; apply P3-12d's raw-attribute check if one appears.
- **Onslaught and culling strike** (the Smite builds' custom lines), the bandit, the pantheon, and the minion's aura.

## P3-12f: the enemy's damage taken, shock, the boss kind, delirium, the bandit, Onslaught, rage, exposure

Several steps in one PR (maintainer, 2026-11-12: as many of the remaining tasks as fit). The pantheon was looked at and left: its souls' stats are game data the bundle does not carry yet (Follow-ups).

### The enemy's damage taken in the hit

**PoB** (`Modules/CalcOffence.lua` 3515 to 3620, the effective mode, PoB's main view): each damage type's hit is multiplied by `effMult = (1 + takenInc / 100) x takenMore x (1 - effectiveResist / 100)`.
- `takenInc` is the enemy's `DamageTaken` and `<Type>DamageTaken` INC, plus `ElementalDamageTaken` for an elemental type and projectile or trap and mine damage taken for such skills.
- `takenMore` is their MORE product (`ModDB:More`: each name's product rounded to two decimals).
- For physical damage the "resistance" is the armour's reduction, computed on the hit before `effMult`.

**The engine.** One new quantity, `Poe1Quantity::EnemyDamageTaken` (PoB's `DamageTaken` on the enemy, every type). `hit::passes` multiplies each type's enemy effect by `(1 + inc / 100) x more` of it, in PoB's order, only when the store holds such an effect (a hit without one keeps its graph). Only `poe1::damage_taken` adds such effects. Damage taken of one type, elemental, projectile, or trap and mine damage taken come only from modifiers, so from stats: no mapping produces them (a test).

### Shock

**PoB.**
- `conditionEnemyShocked` (`ConfigOptions.lua` 1943) sets the enemy's `Condition:Shocked` and `Condition:ShockedConfig`.
- `conditionShockEffect` (1947), a count, sets the enemy's `ShockVal` BASE, only while `ShockedConfig`.
- `calcs.perform` (`CalcPerform.lua` 3545 to 3640): the shock is `floor(min(max(override, ShockVal), max))`. `override` is the strongest guaranteed shock (`ShockBase`, `ShockOverride`, `ShockMinimum`: modifiers); `max` is the `ShockMax` override, else 50 plus each active skill's `ShockMax` (`ailmentData.Shock`, `Modules/Data.lua` 427, precision 0). The enemy takes `DamageTaken` INC equal to it (per stack with `ShockCanStack`, a modifier's flag).
- Nothing happens when the enemy is already shocked, immune, or avoids shock: enemy modifiers only.
- The guard `ShockVal > 0 or modDB:Sum(...)` is always true in Lua (0 is true), so with no configured effect the shock is 0.

**The engine** (`loom_calc::poe1::damage_taken`): with the check on, the effect is the count (the input, else its placeholder; 0 unset), clamped to 0..50 and floored, as `EnemyDamageTaken` INC. A check that is not a boolean, or an effect that is not a finite number, is a reason under its input. A check's placeholder is not applied (PoB applies placeholders to counts only). Everything else that sets a shock is a modifier: no mapped row names shock or damage taken, and the `NoEffect` rows that do (two ailment chances, shock on you, damage taken by you as another type) are listed and checked (a test).

### The boss kind and delirium

- **`enemyIsBoss`.** The Uber kind gives the enemy `DamageTaken` MORE -70 (`ConfigOptions.lua` 2190). The kinds' other modifiers reach no slice field (conditions and multipliers modifiers read, warcry power, the ailment threshold; `poe1::enemy` module docs). The enemy model already modelled the kinds' placeholders (P3-11), so a string kind is no longer a reason.
  - PoB's loader spells a string kind its own way (`Classes/ConfigTab.lua` 853 to 855): lower case, each word capitalised, then "Uber Atziri" read as "Boss" and "Shaper" and "Sirus" as "Pinnacle". The enemy model does the same (`BossKind::of_pob`).
  - A kind PoB does not know stays the enemy model's reason on the hit (read as the default kind, never Uber); a number stays a reason.
  - The enemy model now reads the last input of a name, as PoB's input table does; it read the first (review). PoB never writes a name twice, so no fixture changes.
- **`deliriousPercentage`** (2222 to 2243): 20% to 100% give the enemy `DamageTaken` MORE -16 to -80, and `Damage` INC 6 to 30, the enemy's damage, which only PoB's damage-taken estimates read. "None" (`0`) gives nothing. Another value is a reason.
- With both, PoB multiplies the two in one name and rounds: `round(0.3 x 0.2, 2) = 0.06` (a test).

### The bandit

**PoB.** `calcs.initModDB` (`CalcSetup.lua` 663 to 672) reads `env.configInput.bandit`: Alira `ElementalResist` BASE 15, Kraityn 8% increased movement speed, Oak `Life` BASE 40, anything else one passive point. Kraityn and the passive point reach no slice field.

Which value PoB reads:
- PoB loads the `<Build>` element first (`buildMode:LoadDB`) and copies its `bandit` attribute into the config's first set (`Modules/Build.lua` 648 to 651).
- `ConfigTab:Load` then replaces every set; each new set holds the list's default, "None" (`Classes/ConfigTab.lua` 1290 to 1296).
- So with a `<Config>` element the attribute is never read, and a set without the input has no bandit.

**The engine** (`collect::choices`): with a typed config set, the active set's last `bandit` input acts (none: no bandit), and the attribute is ignored. A value outside the four is a reason under the input. With no set, the domain cannot tell a build without a `<Config>` element (the attribute counts) from an empty one (it does not), so an attribute other than "None" stays a reason. The pantheon is unchanged.

### Onslaught

**PoB.** `buffOnslaught` (`ConfigOptions.lua` 1010) sets `Condition:Onslaught`; `CalcSetup.lua` (97) turns it into the `Onslaught` flag. `CalcPerform.lua` (682 to 711) then gives `floor(20 x (1 + inc / 100))` increased attack, cast, and movement speed, `inc` the `OnslaughtEffect` and `BuffEffectOnSelf` sums (and an active Silver Flask's effect).

**The engine** (`loom_calc::poe1::buffs`): with the check on, `Speed` INC 20 for attacks and casts. The stats that change the effect (`buff_effect_on_self_+%`, `onslaught_effect_+%`, `map_monster_and_player_onslaught_effect_+%`) must stay reasons (a test, and the real-bundle check); an active flask is already a reason.

### Rage and exposure in `NAMED_ONLY`

(Since P3-8k the exposure checks have their effect, `poe1::exposure`, and their stats moved to `exposure::DEPENDS_ON` or became its rows: `docs/phase-3/stat-mapping.md`, "P3-8k". Rage stays as below.)

Both inputs act only with a condition that modifiers set, as `conditionConvergence` (P3-12a):

| Input | PoB's effect | Acts only with | Set by |
|---|---|---|---|
| `multiplierRage` | `Multiplier:RageStack` (line 1074), read in `CalcPerform.lua` 914 to 935 (rage's more attack damage and speed) | `Condition:CanGainRage` or `RageRegen` above 0 | Modifiers (`ModParser.lua` 2201 to 2585, 5517 to 5539) and skill data (Chain Hook, Vengeful Cry, a curse's stun rage, the Spirit of Akoya tattoo) |
| `conditionEnemyColdExposure`, `conditionEnemyFireExposure`, `conditionEnemyLightningExposure` | The enemy's `<Type>Exposure` BASE -10 (lines 1973 to 1981) | The player's `Condition:CanApply<Type>Exposure` (`CalcPerform.lua` 398 to 408): `<Type>ExposureChance` above 0 on the main skill or the character | Modifiers (`ModParser.lua` 850 to 855, 4246 to 4320, 4546) and `elementalArmyExposureType` (itself a reason) |

The stats behind those modifiers were found by their text in the bundle's stat descriptions (the game's own text) and matched to PoB's: 39 for rage (gaining rage on hit, kill, warcry, crit, flask use, when hit, from mana spent; regenerating it per life or mana regeneration; the skills' stats) and 28 for exposure (the chance and on-hit forms, socketed, devotion, curse-cast, graft, minion, and Summon Arbalists variants). They join `DEPENDS_ON`: each must stay `Unmapped` through every mapping (a test), and each is a stat of the real bundle (the maintainer-run test). A further test checks that no mapped row names rage or exposure; the two `NoEffect` rows that do (rage's loss delay; mana regeneration after inflicting exposure) are listed. PoB's skills that expose set the exposure itself, not the chance: they are other gems, reasons.

### Culling strike

The Smite builds' custom line "Culling Strike" resolves to `kill_enemy_on_hit_if_under_10%_life`. PoB reads it as `CullPercent` MAX 10 (`ModParser.lua` 5066), which reaches only `CullMultiplier`, and that only `CullingDPS` and `CombinedDPS` (`CalcOffence.lua` 3332 to 3339, 6289 to 6292). It is a `NoEffect` row (`culling`).

### What changes

- **Public builds.** Values change in 9 builds, all in `Partial` fields: 22 values, each closer to PoB's, 6 now equal (`p3-12f-public-vs-p3-12e.json`):

| Builds | Change |
|---|---|
| The 4 Smite builds | Average damage and DPS: a 20% shock (Smite act1: 760.3 against PoB's 919.6, was 524.3) |
| Shock Nova, Kinetic Blast | Average damage and DPS: a configured shock; Kinetic Blast also Onslaught and Alira |
| Elemental Hit | Attack rate and DPS: Onslaught |
| Winter Orb, Arakaali's spider, Kinetic Blast | Resistances: Alira (Winter Orb's three now equal PoB's) |

- **Reasons.** 85 are gone and none added, in 28 builds: `conditionEnemyShocked` (16), `conditionShockEffect` (11), `multiplierRage` (11), the bandit (9 inputs, 10 attributes), `buffOnslaught` (7), the exposures (11), `enemyIsBoss` (5), culling strike (5 lines).
- **The 7 in-scope builds.** Their config reasons fall to the pantheon (two inputs, two choices) for all seven, and the Lightning Golem's Wrath for Ice Nova. Every field stays `Partial`, through other gems, unmapped stats, and unresolved lines.
- **Corpus.** Four builds' values change, all 11 values closer to PoB's, 3 now equal: Alira's resistances in three builds, a shock in one. Reasons: 40 gone, none added, in 18 builds.

### Evidence

2026-11-13, Windows. Tool commit `6b8bb87` (the calc runs record the tree as dirty: only `CLAUDE.md`, `docs/plan/ACTION-PLAN.md`, and the task files had uncommitted edits; no code). Inputs are P3-12e's (bundle `local/p3/p3-1d/bundle-poe1-a`, overlay `local/p3/p3-1d/parity-1/overlay.json`), with a new oracle run for the three new fixtures. Summaries are in `docs/phase-3/config-inputs/p3-12f-*`.

| Run | Builds | Result |
|---|---|---|
| `oracle-run`, the whole manifest | 94 | Every expectation met, 3/3 identical runs each |
| `calc-run`, public, twice | 94 | 93 documents; 1181 complete, 334 partial, 405 unsupported, 312 indeterminate. Byte-identical twice |
| `calc-run --data-overlay`, public | 94 | The same field states |
| `calc-parity`, public, with the overlay run, the register, and the manifest | 94 | 30 differences, all registered (DIV-0001 to DIV-0030; 2 new, below), 0 unexpected, no unused entry, no missing mechanic fixture; 1159 fields match. **Every mechanic target met (1147 of 1147, 57 fixtures).** Exit 5 (`incomplete`), as before |
| `calc-run` and `calc-parity`, corpus (21, `--anonymous`), twice | 21 | Byte-identical, no difference (exit 5) |
| `build-round-trip`: public (fixtures, community, mechanic, PoB's 3.13 test builds) and corpus | 94, 21 | All pass |

**The new fixtures and the register.**

| Fixture | What it holds | Result |
|---|---|---|
| `ice-nova-shocked-uber-witch.xml` | A 35% shock, the Uber boss, 40% delirium, Alira | 22 of 22 equal to PoB |
| `smite-shocked-templar.xml` | A configured 70% shock (50% applied), 20% delirium, Oak, Onslaught | Every field but the attack rate and DPS equal to PoB |
| `ice-nova-onslaught-rage-exposure-witch.xml` | Onslaught, 30 rage, cold and fire exposure (no source of rage or exposure) | 22 of 22 equal to PoB: only Onslaught changes a field |

On `smite-shocked-templar.xml` the attack rate and `TotalDPS` differ: Q11's `game-truth` difference (1000 / 667 = 1.4993 against PoB's 1.50, times the same 0.85 and 1.2 on both sides), registered as DIV-0029 and DIV-0030 with the evidence of DIV-0001. The breakdown's displayed total rounds equal (164.8). No other divergence is claimed: shock's maximum and precision, Onslaught's 20, delirium's and the Uber boss's values, and the bandit's rewards are PoB's constants, as the game describes them.

**Tests** (1169 workspace tests): the damage taken in a hit (shock: cap, floor, placeholder, last input, negative; the Uber kind, its spellings, delirium, both together; reasons for bad values); Onslaught; the bandit (Alira, Oak, none, the attribute ignored beside a set, reasons); `BossKind::of_pob`; the guards (shock and damage taken, rage and exposure, Onslaught's stats). The maintainer-run real-bundle test checks every `DEPENDS_ON` stat, the new ones included. Clippy, fmt, wasm32.

**Review** (`code-architecture-reviewer`): no code blocker; every rule confirmed against PoB's Lua. Applied: the enemy model reads the last input of a name; PoB's boss-kind spelling; Onslaught on an attack (the Smite fixture); one shared `last_input` helper; docs. Left: the pantheon reads the build attribute even beside a set (conservative: a reason, documented); the guards match stat-ID words, as earlier steps (the real-bundle test is the net).

**Commands:** as P3-12e's, with `<out>` = `local/p3/p3-12f/evidence` (script `local/p3/p3-12f/evidence.sh`).

### Follow-ups

- **The pantheon** (all 7 in-scope builds; 24 public builds). PoB applies every soul of the chosen gods (`Modules/PantheonTools.lua`), each soul's modifier a stat with a value, from the game's pantheon tables (`Data/Pantheons.lua` is exported from them). The bundle needs a pantheon table first (extractor, converter, validator); then the souls' stats act through the mapping, and the choice is read from the config set as the bandit is.
- **The Lightning Golem's Wrath** (`summonLightningGolemEnableWrath`, the Ice Nova builds), with the other gems' auras and buffs.
- **Other config inputs** in the public builds: `conditionEnemyIgnited`, `conditionLeeching`, `conditionEnemyBlinded`, and the others listed by the calc runs.
- **A delirium number other than 0** is a reason; PoB ignores it (no branch matches). Conservative.

## P3-12g: the pantheon; Intimidate, Unnerve, Blind; ten inputs that only set names

Several steps in one PR (maintainer, 2026-11-13: as much of the remaining work as fits). The pantheon was the handoff's next step; the enemy conditions and the name-only inputs are the public builds' most frequent remaining config reasons after it. The Lightning Golem's Wrath was looked at and left: its aura is the golem's own skill, so it needs the golem gem modelled first (Follow-ups).

### The pantheon as game data

PoB v2.67.2 exports `Data/Pantheons.lua` from the game's `PantheonPanelLayout` (`src/Export/Scripts/pantheons.lua`). The bundle now carries the same table:

| Part | Where |
|---|---|
| `PantheonPanelLayout` among the extractor's PoE1 roots | `tools/extractor/src/extract.rs` (`CHOICE_TABLES_POE1`) |
| `poe1/pantheons.json` in the bundle's optional PoE1 section: `Pantheon { id, major, souls }`, each `PantheonSoul { name, stats }` | `loom_data::bundle::poe1` |
| The converter reads the table as PoB's exporter does: rows with `IsDisabled` left out, `GodName<n>` with `Effect<n>_StatsKeys` and `Effect<n>_Values` paired in order, a soul without stats left out; a soul whose lists differ in length, or whose key is no stat, fails the conversion | `tools/parity-cli/src/pantheons.rs` |
| The validator: every soul stat is a stat (`BUNDLE_DANGLING_REF`); IDs unique and not empty; every soul named, with a stat (`BUNDLE_VALUE_RANGE`). Shared code names no PoE1-only word: `Poe1Section::choice_stats` and `choice_domain` give the validator what to check | `loom_data::bundle::validate` |
| PoB's `data.pantheons` in the data export (format 3): each god's major mark, its souls' names, their mods' lines and values | `tools/oracle/data/pob_data.lua` |
| `data-parity` table `pantheons`: PoB keeps each soul's mod as the line its exporter rendered (`describeStats`), the bundle the stat; each of our stats is rendered with the game's stat descriptions into PoB's form, and the lines, values, soul names, and major marks are compared | `tools/parity-cli/src/data_parity/compare.rs` |

The section is optional (P2's rule for `poe1/`): a bundle without the file still loads, and a build that chooses a god is then `Indeterminate` (`missing-data`, table `pantheons`).

### The pantheon in the calc

**PoB.** `calcs.initModDB` (`Modules/CalcSetup.lua` 560 to 571) reads `env.configInput.pantheonMajorGod` and `pantheonMinorGod`; for a choice other than "None" it applies `pantheon.applySoulMod` (`Modules/PantheonTools.lua`): every soul of the god, upgrades included (PoB has no input for which upgrades are unlocked), each mod's line through the mod parser. Which value PoB reads is the bandit's rule (`Modules/Build.lua` 630 to 651).

**The engine** (`loom_calc::poe1::pantheon`):
- With a typed active set, each input's last value acts; "None" or no input, nothing. The god's soul stats act through the stat mapping, as an item's would, with the config input as their source (`Source::Config`).
- A name the table does not hold, or a value that is not a name, is a reason under the input (PoB would fail on it).
- With no set, an attribute other than "None" stays a reason (`unmodelled-choice`), as the bandit's; beside a set the attributes are ignored (P3-12f left the pantheon's attributes a reason even beside a set).
- `CalcData::poe1()` gives the PoE1-only tables (`Poe1Data`); it defaults to none, so data without them is `missing-data`, never a silent zero. The parity tool's overlay passes the bundle's through (the table equals PoB's).

**The soul stats.** All 41 (12 gods) join `NO_EFFECT`, each read as PoB reads its line: avoidance, stun, ailments and curses on you, damage taken by you and its reduction, regeneration and flasks, movement speed, the enemy's regeneration and attack speed. Three only matter through a config input, which must stay a reason (`NO_EFFECT_DEPENDS_ON_INPUTS`, a test sets each and expects `unmodelled-config`):
- "Cannot be Blinded": only Blind on you (`conditionBlinded`, 20% less accuracy and evasion, `CalcPerform.lua` 824 to 835);
- "#% reduced Effect of Curses on you": only the curses the `playerCursedWith*` inputs put on you (`CalcPerform.lua` 3114);
- "#% reduced Effect of Chill on you": only chill on you (`conditionChilled`, `conditionChilledEffect`).

"+#% Chaos Resistance against Damage Over Time" is `ChaosResist` BASE with `ModFlag.Dot` (`ModParser.lua` 1001): PoB's shown resistance sums without a flag, so it never counts there. PoB drops a line its parser does not read in full; the engine reads the stat instead, and since no soul stat reaches a slice field the two agree on every field. A maintainer-run test checks that every soul stat of the real bundle is a `NoEffect` row (41 of 41).

### Intimidate, Unnerve, and Blind

| Input | PoB | The engine |
|---|---|---|
| `conditionEnemyIntimidated` | The enemy's `Condition:Intimidated` (`ConfigOptions.lua` 1956); `calcs.initModDB` (`CalcSetup.lua` 64) gives every actor, the enemy too, `DamageTaken` INC 10 with `ModFlag.Attack` while intimidated | `EnemyDamageTaken` INC 10 for attacks |
| `conditionEnemyUnnerved` | `Condition:Unnerved` (1977); `DamageTaken` INC 10 with `ModFlag.Spell` (66) | `EnemyDamageTaken` INC 10 for spells |
| `conditionEnemyBlinded` | `Condition:Blinded` (1834), the `Blind` flag (`CalcSetup.lua` 74); `doActorMisc` (`CalcPerform.lua` 824 to 835, run for the enemy at 3559): `Accuracy` and `Evasion` MORE `floor(-20 x effect)`, `effect` 1 plus the enemy's `BlindEffect` and `BuffEffectOnSelf` INC | New `Poe1Quantity::EnemyEvasion` MORE -20; an attack's chance to hit reads the enemy's evasion as PoB's `calcLib.val` (base, INC, MORE), then rounds |

The party's variants need a party member's condition (no party is modelled). The enemy's accuracy reaches no slice field. "#% increased Blind Effect" (`blind_effect_+%`) is an `EnemyModifier` of `BlindEffect` (`ModParser.lua` 2238): it must stay a reason (`BLIND_DEPENDS_ON`, checked `Unmapped` through every mapping and a stat of the real bundle). A test checks that no mapped row, and no `NoEffect` row but "Cannot be Blinded" (on you), names Intimidate, Unnerve, or Blind: the stats that inflict them on hit are reasons, so the configured conditions are the only ones the engine applies. A check that is not a boolean is a reason under its input.

### Ten inputs that only set names

`NAMED_ONLY` gains, each with every reader of its names in the calc modules checked (`poe1::config` docs): `conditionCastSpellRecently`, `conditionUsedMinionSkillRecently` (each with `UsedSkillRecently`), `multiplierNonInstantSpellCastRecently`, `conditionLeeching` (no calc reader); `conditionKilledRecently` (only trigger skills read it, `CalcTriggers.lua`); the enemy's `conditionEnemyBleeding`, `conditionEnemyLowLife`, `conditionEnemyPoisoned` (no calc reader on the enemy); `conditionEnemyMaimed` (30% reduced movement speed, not a slice field); `conditionEnemyIgnited` (and `Burning`): Combustion reads it (`CalcPerform.lua` 2709 to 2716), only for a skill supported by the Combustion gem, whose stat `ignites_apply_fire_resistance_+` joins `DEPENDS_ON`, and Call the Pyre's trigger (a gem).

### What changes

- **Public builds** (`p3-12g-public-vs-p3-12f.json`). Ten values change in three builds, all in `Partial` fields, each closer to PoB's: Elemental Hit and Kinetic Blast (a blinded enemy: hit chance 64 to 70% and 85 to 90%, and their crit chance, average damage, and DPS), Shock Nova (an unnerved enemy: average damage and DPS 8% higher). No field changes state.
- **Reasons.** 159 are gone and none added, in 32 builds: every pantheon reason (`pantheonMajorGod`, `pantheonMinorGod`, and the attributes' `unmodelled-choice`: none is left in any public build), the ten name-only inputs, the three enemy conditions, and five `base_stun_recovery_+%` item lines (a soul stat, now a `NoEffect` row).
- **The 7 in-scope builds.** The four Smite builds have no config reason left; the three Ice Nova builds only the Lightning Golem's Wrath. Every field stays `Partial`, through other gems, unmapped stats, and unresolved lines.
- **Corpus.** No value changes; 108 reasons gone, none added, in 20 builds.

### Evidence

2026-11-13, Windows. Tool commit `2f084ad`, clean (the review's test-only commit `cfa5f43` came after the run). Summaries are in `docs/phase-3/config-inputs/p3-12g-*`; the bundle and data-parity records are replaced (`docs/phase-3/bundle-poe1-26530219.json`, `data-parity-poe1-26530219.json`).

| Run | Result |
|---|---|
| `check-determinism.sh` on PoE1 (clean `30e7d8a`) | Deterministic: 572 files identical in two runs; the new `PantheonPanelLayout.json` the only new table |
| `bundle` twice | Byte-identical. Against P3-1d's bundle, only `bundle.json` and the new `poe1/pantheons.json` (12 gods; 4 disabled rows and 16 souls without stats left out). Manifest SHA-256 `595dd9a7…` |
| `bundle-check` | Exit 0, no findings |
| `pob_data.lua` twice (export format 3) | Byte-identical; oracle checkout clean |
| `data-parity` twice | Byte-identical. **`pantheons`: 88 values compared, 0 differences** (each soul's rendered line equals PoB's). Every other table as P3-1d's: 534 differences, `uncategorized` 0, 314 overlay rows |
| `oracle-run`, the whole manifest | 100 fixtures, every expectation met, 3/3 identical runs each |
| `calc-run`, public, twice | 99 documents; 1314 complete, 334 partial, 416 unsupported, 312 indeterminate. Byte-identical twice |
| `calc-run --data-overlay` (this bundle's overlay) | The same field states |
| `calc-parity`, public, with the overlay run, the register, and the manifest | 33 differences, all registered (DIV-0001 to DIV-0033; 3 new, below), 0 unexpected, no unused entry; 1290 fields match. **Every mechanic target met (1280 of 1280, 63 fixtures).** Exit 5 (`incomplete`), as before |
| `calc-run` and `calc-parity`, corpus (21, `--anonymous`), twice | Byte-identical, no difference (exit 5) |
| `build-round-trip`: public and corpus | All pass |

**The new fixtures and the register.**

| Fixture | What it holds | Result |
|---|---|---|
| `ice-nova-pantheon-arakaali-shakari-witch.xml`, `-solaris-yugul-`, `-lunaris-garukhan-`, `-brine-king-gruthkul-` | The four major and four minor gods most likely to touch a slice field (chaos resistance against damage over time, damage taken and its reduction, curses, shock, and Blind on you) | 22 of 22 equal to PoB each; PoB's own fields equal `ice-nova-witch.xml`'s |
| `ice-nova-unnerve-intimidate-blind-witch.xml` | Unnerve, Intimidate, Blind, and the ten name-only inputs | 22 of 22 equal to PoB (Unnerve's 1.1) |
| `smite-intimidate-blind-templar.xml` | The same on Smite | Every field but the attack rate and DPS equal to PoB (hit chance 26%, PoB's) |

On `smite-intimidate-blind-templar.xml` the attack rate, `TotalDPS`, and its displayed breakdown total differ by Q11's `game-truth` difference (1000 / 667 = 1.4993 against PoB's 1.50, times the same 0.85), registered as DIV-0031 to DIV-0033 with DIV-0001's evidence. No other divergence is claimed: Intimidate's and Unnerve's 10%, Blind's 20%, and the souls' values are the game's, as PoB has them.

**Tests** (1179 workspace tests): the converter (pairing, disabled rows, souls without stats, a table it would misread); validator mutations; `data-parity`'s `pantheons` table; the pantheon in the calc (both gods, the source, "None", the last input, an unknown god, a value that is not a name, data without the table, no set); the inputs a `NoEffect` row relies on; Unnerve and Intimidate on a spell and an attack, Blind's hit chance, Blind beside Corrosion or Brine ground (reasons); the guards (no row names Intimidate, Unnerve, or Blind; the charge and damage-taken guards list the souls' rows). Maintainer-run real-bundle tests: every soul stat is a `NoEffect` row (41, unique), every `DEPENDS_ON` and `BLIND_DEPENDS_ON` stat is a stat. Clippy, fmt, wasm32.

**Review** (`code-architecture-reviewer`): no code blocker; every rule confirmed against PoB's Lua (the soul lines, the ten inputs' readers, Intimidate's and Unnerve's flags, Blind's place before the rounding). Applied: `EnemyEvasion`'s stable name tested; the soul-stat count as unique stats; Blind beside the other evasion inputs; the god looked up whatever its major mark (docs); docs and the bundle record. Left: the register's Q11 entries cite the attack-time test, as DIV-0001 to DIV-0030 do; a test for a pantheon attribute beside only untyped sets (their reason covers it, as the bandit's).

**Commands:** as P3-12f's, plus the bundle and data steps (P3-1d's), with `<out>` = `local/p3/p3-12g/evidence` (script `local/p3/p3-12g/evidence.sh`).

### Follow-ups

- **The Lightning Golem's Wrath** (`summonLightningGolemEnableWrath`, the three Ice Nova builds): the aura is the golem's own skill (`LightningGolemWrath`, `Data/Skills/minion.lua`), so the golem gem comes first. With it, **the other gems** (`unmodelled-gem`, the largest reason left in the in-scope builds: auras, heralds, golems, buffs, curses, warcries, totems, movement skills, reservation). PoB lets a non-main skill act through its buffs (`GlobalEffect` mods of its stat maps), reservation, minions, totems, chilling areas, warcries, and curses (`CalcPerform.lua` 1261 to 1440, 1928 to 2040); which gems are inert for the slice needs those stat maps per skill.
- **Other config inputs** in the public builds: `multiplierWitheredStackCount`, `buffLifetap`, `infusedChannellingInfusion`, `multiplierCorpseConsumedRecently`, `prideEffect`, `buffTailwind`, and the others the calc runs list.
- **Q12** (maintainer, in game): see the handoff.

## P3-12j: the "Onslaught" modifier

The four in-scope Smite builds' active custom block holds the line "Onslaught". PoB reads it as `Condition:Onslaught` (`Modules/ModParser.lua` 3973), the condition the `buffOnslaught` check also sets (P3-12f), without the check's combat tag. `calcs.initModDB` turns the condition into the `Onslaught` flag (`CalcSetup.lua` 97), and `calcs.perform` gives the buff once whatever the number of sources (`CalcPerform.lua` 656 to 684).

### The line

"Onslaught" is a game text. Two stats render it: `base_should_have_onslaught_from_stat` (the character's Onslaught: the Synthesis implicit, a unique, two monster mods) and `silver_flask_display_onslaught` (a Silver Flask's display stat, which no mod row carries). An item line is settled by the mod-row filter with the item's base. A custom line has no base, so P3-12c left it ambiguous.

**The rule** (`loom_parser::import`, `resolve_custom_line`): of an ambiguous custom line's readings, the one that is global (no stat local to an item) and backed by a mod row of an item domain (`Mods::backed_in`; the item domains are the base items' domains, so the monster and area domains are out) is the reading when it is the only one. Otherwise the line stays ambiguous. It is a prior, not PoB's parse: a player's custom line is read as something an item could carry. The first run took the weapon-local `local_critical_strike_chance` for "+1.5% to critical strike chance" in the Ice Nova builds' legacy lines (PoB: global base crit), still a reason but a misleading one; local readings are therefore excluded (`2909f75`), and that line stays ambiguous. A custom line is a modifier the player gives the character; a reading that no item can carry is a display or a monster stat. The synthetic bundle's accuracy line, read before as ambiguous, now resolves to the one reading that has a row; its evasion line, with no row for either reading, stays ambiguous (`crates/loom-parser/tests/resolve_build.rs`).

### The calc

- **`Poe1Quantity::Onslaught`** (calc result v1: a new quantity name, `poe1.onslaught`, additive), a flag. The stat's mapping row gives a `Flag` effect on it.
- **`poe1::buffs`** runs after every other source. Onslaught is on when the check is on or when any unconditional `Flag` effect on the quantity exists; it then adds `Speed` INC 20 for attacks and for casts once. The source is the config input, else the first flag's source. Two lines, or a line and the check, give it once, as PoB's flag does.
- `DEPENDS_ON` (Onslaught's effect stats) is unchanged; the guard test now allows the one flag stat.

### Mechanic fixtures

| Fixture | What it isolates |
|---|---|
| `ice-nova-onslaught-modifier-witch.xml` | The custom line "Onslaught" alone, the check off: 20% increased cast speed |
| `ice-nova-onslaught-twice-witch.xml` | The line in two blocks and the check on: the same, once (PoB's output for both fixtures is identical) |

The first run of both matched PoB on every target.

### Review

`code-architecture-reviewer`: no blockers; PoB's reading confirmed (the tagless condition, the flag once, the check's combat tag). Applied: the global check through the locality classifier (`local_kind`), the doc notes on the rule being a prior and on the breadth of the item domains, a comment and test that every Onslaught flag is unconditional, the test modules merged, a disabled block in the test. Not done: marking a reading the rule took as `adapted` (a domain change; the line's source and the fixtures show it), a fixture with two blocks and the check off (the unit test covers it).

### Evidence

Maintainer-run, 2026-11-13, Windows, from the worktree on a clean tree. `local/p3/p3-12j/evidence.sh` (tool commit `36c46b4`, before the rebase on #91's tip: the oracle run only, which has no engine code) and `local/p3/p3-12j/evidence-calc.sh` (tool commit `2909f75`: the calc runs and `calc-parity`; the review's `962ea42` changes no result). No game data changes: P3-12i's bundle and overlay. Summaries in `docs/phase-3/config-inputs/p3-12j-*`.

| Run | Result |
|---|---|
| `oracle-run`, the whole manifest | 108 fixtures, every expectation met, 3/3 identical runs each |
| `calc-run`, public, twice | 108 documents; 1493 complete, 334 partial, 429 unsupported, 312 indeterminate. Byte-identical twice |
| `calc-parity`, public, with the overlay run, the register, and the manifest | 33 differences, all registered (unchanged), 0 unexpected; 1469 fields match. **Every mechanic target met: 1459 of 1459 over 71 fixtures** |
| `calc-run` and `calc-parity`, corpus (21, `--anonymous`), twice | Byte-identical; no difference |

**Against P3-12i** (`p3-12j-public-vs-p3-12i.json`, `p3-12j-corpus-poe1-26530219-vs-p3-12i.json`). No field state changes. The four in-scope Smite builds' speed and hit DPS change, all eight values closer to PoB's: Smite act1's speed is 1.5956 against PoB's 1.5948 (the remaining gap is Q11's floored attack time, `game-truth`). Their "Onslaught" reason is gone (4); none added. Smite act1's only reason left is "Perfect Forest Warrior" (Q13). Corpus: 4 legacy custom lines in 3 builds now resolve (none was "Onslaught"); 8 values in 2 builds change, each closer to PoB's or equal (4 equal: the resistances and speed of one build).

Tests: 1207 workspace tests, clippy, fmt, wasm32.

## Phase 3 close-out

Config inputs the other gems now read (`docs/phase-3/other-gems.md`, "Phase 3 close-out"):
- `summonLightningGolemEnableWrath` (`ConfigOptions.lua` 681): Lightning Golem's Wrath aura when a Summon Lightning Golem is modelled (`poe1::gems::wrath`); any other value is a reason.
- `flameWallAddedDamage` (443): Flame Wall's added fire damage to projectiles (`BuffStat::WhenInput`).
- `bloodSandStance` (643) left `config::NAMED_ONLY`: Flesh and Stone's Blood Stance row applies unless the stance is Sand (`CalcSetup.lua` 538 sets Blood Stance otherwise).
- `conditionCritRecently` (1240) left `NAMED_ONLY` for `poe1::conditions`: "#% increased Cast Speed if you've dealt a Critical Strike Recently" reads `CritRecently`; `SkillCritRecently` stays a name only modifiers read.
- `multiplierNearbyAlly` (1110) stays named-only for every reader but Rallying Cry's exerted attacks: with a modelled Rallying Cry and allies above 0 it is `rallying-cry-allies` (`poe1::gems`).

The custom modifier lines (P3-12c, P3-12d) gained notes and patterns: `docs/phase-3/custom-lines.md` (ADR-015 Amendment 2).
