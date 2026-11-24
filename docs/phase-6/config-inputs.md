# P6-4a: config inputs

Phase 6, step P6-4 is split in two:
- **P6-4a (this document):** the config inputs left `unmodelled-config`. They need no game data.
- **P6-4b:** flasks. They need an extraction root, a bundle file, a validator, and a `data-parity` table (plan decision 8).

The recount is from the P6-2d evidence (`local/p6/p6-2d`, public and corpus). 103 distinct inputs were reasons in 42 builds. Each input's `apply` in PoB v2.67.2 (`Modules/ConfigOptions.lua`) was read. Every name it sets was then searched for in every calc module (`CalcSetup`, `CalcPerform`, `CalcOffence`, `CalcDefence`, `CalcActiveSkill`, `CalcTriggers`, `CalcMirages`, `Calcs`, `CalcTools`, `ModStore`, `ModDB`, `ModList`), as in P3-12g.

## What changed

1. **Tailwind and Adrenaline** (`poe1::buffs`). The checks now give PoB's effects (`CalcPerform.lua` 733 to 746):
   - Tailwind: 8% increased action speed;
   - Adrenaline: 100% increased damage, and 25% increased attack speed and cast speed.

   The effect stats are unmapped (`TAILWIND_DEPENDS_ON`, and `buff_effect_on_self_+%` in `DEPENDS_ON`), so they stay reasons and the constants hold. Adrenaline's movement speed and physical damage reduction are not slice fields.
2. **59 inputs that only set names** (`config::NAMED_ONLY`, now 89 entries). An input's `apply` sets `Condition:` or `Multiplier:` names that only modifiers read. A mapped modifier reads a named condition only through a settled state, and the store makes an unsettled one `unmodelled-condition`. The inputs fall in three groups:
   - **No calc module reads the names:**
     - the corpse, warcry, totem, mine, channelling, blocking, hit, kill, and impale conditions and counts;
     - the enemy's Burning, Hindered, Taunted, Champion's Intimidate, and presence time;
     - Focused, Gale Force, Soul Eater (its speed needs `CanHaveSoulEater`, which only modifiers set), Resonance, Otherworldly Pressure, Infusion, Magmatic Strikes, Plague Bearer, and Pride's maximum effect (Pride is not a modelled gem);
     - Perforate, Raise Spiders, Meat Shield, Arcane Cloak, and Cyclone;
     - the nearby rare count.
   - **The calc modules also set them from the build, and read them only through modifiers:** `condList` in `CalcPerform.lua` 242 to 276, the enemy's `Cursed` (3303), `UsingFlask` from active flasks (1735), and the summoned minion count (1459; read at 1469 only to set `OnlyMinion`, a condition).
   - **Read only outside the slice:**
     - `LowLife`: a flask's instant recovery, and the input itself in the defence estimate's life pools (`CalcDefence.lua` 2216, 2238);
     - Lifetap: flask recovery;
     - Elusive: avoidance and movement speed; Nightblade's crit needs a support that is not modelled;
     - the enemy's Debilitated and Malignant Madness: the enemy's damage and speed;
     - `KilledRecently`: trigger skills;
     - the player's `Ignited`;
     - the enemy's Withered stacks: they add chaos damage taken only with `WitherEffectStack`, which only unmapped stats and gems that are not modelled give (`CalcPerform.lua` 812 to 815).
3. **Two inputs only the defence estimate reads** (`config::ENEMY_DAMAGE`): `DisableEHPGainOnBlock` (`CalcDefence.lua` 2760, 2793) and `enemyDamageType` (1643).
4. **Four inputs PoB v2.67.2 ignores** (`config::INERT`): `brandAttachedToEnemy`, `buffFortify`, `enemyAwakeningLevel`, and `igniteMode`. No option defines them and no module reads them. Other versions of PoB save them. `buffFortify` is written only by `ConfigTab:ImportCalcSettings` from an old calcs tab.

## Left as reasons, for later steps

| Inputs | Step |
|---|---|
| Minion inputs (Feeding Frenzy, minion charges, Onslaught, full life), `TotalSpectreLife`, `detonateDeadCorpseLife` | P6-7 |
| Brands (`ActiveBrands`, `BrandsAttachedToEnemy`), `TotemsSummoned`, `minesPerThrow`, Storm Rain's and Toxic Rain's overlaps | P6-6 |
| `waveOfConvictionExposureType`, `buffUnholyMight` (`multiplierWarcryPower` read since P6-11bs, `docs/phase-6/gems.md`) | P6-9 |
| `buffAlchemistsGenius` (flask effect), `conditionUsingFlask`'s flask source | P6-4b |
| The enemy's brittle, scorch, and sap (and their grounds), crushed, frozen, moving (bleed's more damage, `CalcSetup.lua` 537), Ensnare, covered in ash; Elemental Equilibrium's inputs; mana burn; `presetBossSkills`; `bandit` with a value PoB reads otherwise | P6-11 |

After this step, 36 distinct inputs remain reasons, in 26 builds.

**Guards** (P6-4a review, applied in P6-4b):
- A test checks that no name a `NAMED_ONLY` input sets is a condition the engine settles (the config checks, the weapon states, the thresholds, the state rows).
- Another test checks that the classified inputs give `NoEffect` through `collect::input_use`, as an input and as a placeholder.
- `config::DEFAULT_ON` lists the two `defaultState = true` options (Champion's Intimidate, Sacrifice Minion). PoB saves them only when off, so settling either must read absent as on.

## Evidence

**Fixtures:**

| Fixture | Result |
|---|---|
| `ice-nova-tailwind-adrenaline-witch.xml` | Tailwind and Adrenaline on a spell, with eleven quiet inputs set (name-only, defence-only, ignored). Equal to PoB on every field. |
| `smite-tailwind-adrenaline-templar.xml` | On an attack. Equal to PoB but for the weapon's rate (Q11, DIV-0094 to DIV-0096). |
| `smite-poison-chaos-templar.xml` | The P6-2c review's poison fix (`docs/phase-6/ailments.md`). Q11, DIV-0091 to DIV-0093. |

**Public `calc-parity`:** 152 builds, 2,551 match (+64), 96 expected differences, 0 unexpected.

**Corpus:** no unexpected difference.

Both runs were byte-identical.

## Open items S8 (2026-11-24): config inputs

The open items survey (S0) listed 28 `unmodelled-config` inputs. This step settles those whose PoB effect is fully known; each change is in `poe1::config`, `poe1::buffs`, or `poe1::damage_taken`, with PoB's lines in the code.

**No effect in a computed field** (each a `NoEffect` input now):

- `stormRainBeamOverlap`, `stormRainActiveArrows`, `toxicRainPodOverlap` (`config::SKILL_DATA_ONLY`): a `SkillData` value for the named skill (`SkillName` tag), read only by that skill's `preDamageFunc` as its `dpsMultiplier` (`act_dex.lua` 11747, 11851, 14933, 15025, 15114). Every such skill is unvetted for its hit, and Toxic Rain's for its DoT too (a test: `skill_data_only_skills_should_stay_unvetted`), so the value reaches no `Complete` field. The DoT vetting is now a function, `dot::pob_dot_vetted`.
- `ActiveBrands` (`NAMED_ONLY`): `Multiplier:ConfigActiveBrands`, which `CalcPerform.lua` 1327 to 1330 turns into `Multiplier:ActiveBrand` for an active brand skill; only "per Brand" modifiers read it (`ModParser.lua` 1342, 1343). Their stats, found by the text in the bundle's descriptions, join `DEPENDS_ON`: `cooldown_speed_+%_per_brand_up_to_40%`, `critical_strike_chance_+%_per_brand`, `sigil_recall_cooldown_speed_+%_per_brand_up_to_40%`.
- `buffAlchemistsGenius` (`NAMED_ONLY`): its condition holds only with `Condition:CanHaveAlchemistGenius` (`ConfigOptions.lua` 1044), which only "Gain Alchemist's Genius when you use a Flask" sets (`ModParser.lua` 5102 to 5107); its stat, `gain_alchemists_genius_on_flask_use_%`, joins `DEPENDS_ON`.

**Modelled:**

- **Unholy Might** (`buffUnholyMight`, `poe1::buffs`): `PhysicalDamageConvertToChaos` BASE `floor(100 x (1 + BuffEffectOnSelf / 100))` and `Condition:CanWither` (`CalcPerform.lua` 720 to 724), which enables Withered's chaos damage taken (2704 to 2707, 811 to 814; `poe1::damage_taken`, run after the buffs). `BuffEffectOnSelf` is unmapped, so 100.
- **A moving enemy's bleed** (`conditionEnemyMoving`, `poe1::damage_taken`): PoB's base modifier `Damage` MORE 200 with `KeywordFlag.Bleed` while the enemy is `Moving` (`CalcSetup.lua` 537). Its exception, `NoExtraBleedDamageToMovingEnemy`, comes only from unmapped stats (Crimson Dance, a unique's line).

**Fixtures** (2 of the session's 75):

- `ice-nova-s8-unholy-might-withered-witch.xml`: added physical spell damage converted to chaos, the enemy at 10 Withered stacks: every computed field equals PoB.
- `smite-s8-bleed-moving-enemy-templar.xml`: the bleed against a moving enemy equals PoB per hit; the rate differs only by the Rusted Hatchet's 667 ms (Q11), carried into the hit DPS, the bleed, the mana per second, and the full DPS: DIV-0694 to DIV-0699.

**Still reasons** (follow-ups or later steps): the minion inputs (`minionsConditionFullLife`, `minionBuffOnslaught`, the minion charge inputs; the minion actor's conditions), Feeding Frenzy (its minion damage and speed), `enemyConditionHitByLightningDamage` (Elemental Equilibrium's exposure, `CalcPerform.lua` 3562 to 3572, and the hit sets it, `CalcOffence.lua` 3609), `multiplierNearbyAlly` (Rallying Cry's allies), brittle (`conditionEnemyBrittle`, `conditionBrittleEffect`; scorch, sap, crush, and the grounds are part 2, below), `bandit` as a value the engine does not read, `presetBossSkills`, `TotalSpectreLife`, `detonateDeadCorpseLife`, `multiplierManaBurnStacks`, `EEIgnoreHitDamage`.

### Open items S8, part 2: scorch, sap, crush, the grounds, two stats

- **Scorch and sap** (`poe1::damage_taken::alt_ailments`): PoB's ailment table (`CalcPerform.lua` 3469 to 3546). The check sets the enemy's `Condition:<ailment>` and `<ailment>Config`; the effect input sets `<ailment>Val` while that holds (`ConfigOptions.lua` 1858 to 1865, 1932 to 1939). The ailment is `floor(min(max(override, Val), max))`, the maxima 30 and 20 (`Data.lua` 422, 424). Its `Base`, `Override`, and `Minimum` modifiers and the skills' maxima come only from unmapped stats. Scorch gives the enemy `ElementalResist` BASE `-num` (`Poe1Quantity::EnemyResistance` for fire, cold, and lightning), sap gives it `Damage` MORE `-num` (`EnemyDamage`).
- **Crushed** (`conditionEnemyCrushed`): an actor's `PhysicalDamageReduction` BASE -15 while crushed (`CalcSetup.lua` 109), the enemy's here (`EnemyPhysicalDamageReduction`).
- **The grounds** (`conditionEnemyOnBrittleGround`, `OnSappedGround`, `OnScorchedGround`) in `NAMED_ONLY`: each sets the ailment's condition and its ground's, but no `Val` (that needs the check's `Config` condition), so the table adds an effect of 0.
- **Brittle stays a reason**: its `SelfCritChance` BASE sits on the enemy and so reaches a minion's hits too (`CalcOffence.lua` 3015), which a character effect would not.
- **Two stats `NoEffect`**, the guard tests naming each: `chance_to_gain_adrenaline_for_2_seconds_on_leech_removed_by_filling_unreserved_life_%` (PoB parses its first line with text left over and applies no mod of such a line, `Item.lua` 2504, 2531; the joined line does not parse) and `travel_skills_cannot_be_exerted` (unparsed).

**Fixture** (3 of the session's 75): `ice-nova-s8-scorch-sap-crushed-witch.xml`, Ice Nova with added physical spell damage, the enemy scorched (20), sapped (15), and crushed: every computed field equals PoB, and each input is a source in the graph.

**Evidence** (`local/p6/s8c`, `s8d`): public `Complete` 39,155 with the fixture, 0 unexpected; corpus 0 unexpected, 3 values of the crushed build toward PoB, none away.
