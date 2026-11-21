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
