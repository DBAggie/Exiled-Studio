# P6-1: skill damage over time

Phase 6 plan (`dev/active/phase-6-mechanics/`), step P6-1. The main skill's own damage over time, as PoB v2.67.2 computes it (`Modules/CalcOffence.lua` 5647 to 5720), and what it needed: a field emitted only when it applies, PoB's hand-written skill data as a table, and two fixes that let older builds and renamed gems reach the calculation.

## The field

`dot-dps` (PoB's `TotalDot`; slice revision 3, `applies: present`). Our result has it only when the main skill has damage over time of its own. `calc-parity` compares it when we have it or PoB's value is not 0, and leaves it out otherwise, so a build without the mechanic counts no such field (plan decision 2).

## What is computed (`loom_calc::poe1::dot`)

1. **The DoT config:** the skill's flags with `Dot`. `Area`, `Projectile`, `Spell`, `Attack` and `Hit` stay only when the skill's DoT is one, by either:
   - PoB's skill data: `dotIsArea` and kin, the hand-written base mods;
   - the skill's own stats: `spell_damage_modifiers_apply_to_skill_dot` gives `dotIsSpell`, `projectile_damage_modifiers_apply_to_skill_dot` gives `dotIsProjectile` (`SkillStatMap.lua` 161, 164).

   Each type adds its DoT keyword (`KeywordFlag.FireDot`; new flags `DOT`, `FIRE_DOT`, ... in `loom_calc::Flags`, now 64 bits).
2. **Base per type:**
   - the game's `base_<type>_damage_to_deal_per_minute / 60`;
   - Righteous Fire: `Life x 0.7 + EnergyShield x 0.7` when unreserved life is above 1;
   - Vaal Righteous Fire: `(Life + EnergyShield) x percentSacrificed x RFMultiplier`, in PoB's order (its `preDamageFunc`s).
3. **Per type:** `base x (1 + inc / 100) x more x (1 + mult / 100) x effMult`.
   - `inc` and `more`: `Damage` in the DoT config, each more rounded per name, as a hit's.
   - `mult`: the damage over time multipliers (new quantity `DotMultiplier`).
   - `effMult`: `(1 - resist / 100) x (1 + taken inc / 100) x taken more`, where the resistance has no penetration (`calcResistForType`).
4. **`TotalDot`:** the sum, at most `DotDpsCap`.

Left `NotComputed` (the field is present, unsupported):
- stacking DoTs (`DotCanStack`);
- burning, caustic and Corrupting Blood ground;
- the kinds the hit leaves out (triggered, totem, trap, mine, minion, brand, channelled).

An aura's DoT is a reason (`aura-dot`).

## Stats mapped

| Stat | PoB | Effect |
|---|---|---|
| `damage_over_time_+%` | `SkillStatMap.lua` 752 | `Damage` INC, `DOT` |
| `burn_damage_+%` | 755 | `Damage` INC, `FIRE` and `FIRE_DOT` |
| `dot_multiplier_+` | 1172 | `DotMultiplier` BASE |
| `fire_dot_multiplier_+`, `cold_`, `chaos_` | 1175, 1181, 1178 | `DotMultiplier` BASE of that type |
| `physical_dot_multiplier_+` | `ModParser.lua` 739 | `DotMultiplier` BASE, physical |

The skill's DoT stats themselves are read from the gem instance (`dot::DOT_SKILL_STATS`: no effect through the mapping). Righteous Fire's own buff and degeneration also have no effect when it is the main skill: its burn is not a spell, and the slice has no degeneration.

Righteous Fire is now an "other gem" buff (`poe1::gems`): its spell damage more applies while unreserved life is at least 2. This is PoB's only threshold on it; new state `LIFE_THRESHOLD`, checked by `data-parity`'s `gem_effects` table. This is also how Vaal Righteous Fire's second skill is modelled.

## PoB's skill data as a table (plan decision 5)

`tools/oracle/data/pob_data.lua` export format 10 adds each skill's `base_flags`, `parts`, and skill function names. `loom-parity-cli pob-skills <pob-data.json> --out crates/loom-calc/src/poe1/pob_skills.rs` writes the table: 1,231 skills with hand-written data, generated, not edited, and byte-identical when regenerated.

`loom_calc::poe1::pob_skill` reads it:
- base flags;
- parts;
- `SkillData` keys (`dotIsArea`);
- flag base mods (`dotIsBurningGround`);
- the names of skill functions, which are ported by hand per skill.

PoB is MIT licensed.

## Two fixes found on the way

- **PoB's skills layout before skill sets.** `<Skill>` elements directly under `<Skills>` (older builds, 28 of the public fixtures) were kept untyped, so no skill was calculated. They are now read as one flat set 1, as PoB loads them (`Classes/SkillsTab.lua` 523 to 531), and written back the same way.
  - The domain gains `SkillsLayout` (`Sets`, `Flat`) with a layout check (`DOMAIN_FLAT_SKILLS_LAYOUT`).
  - Build v1 gains an optional `skills.layout` (additive; left out for `sets`), as `config.layout` did in P3-12e.
- **A gem the data does not have, or has renamed:**
  - **A missing gem:** PoB skips a gem without `gemData` (`CalcSetup.lua` 1649). We now skip it too, as a reason (`unmodelled-gem`), instead of failing the whole group.
  - **A renamed `skillId`** (OccVortex's `FrostBoltNova`, now `Vortex`): PoB finds the gem by its name (`FindSkillGem`). `skill::gem_variant` now does the same within the gem's own variants. The default variant's name is its active skill's.

## Evidence

Mechanic fixtures, each equal to PoB on every target (`tools/oracle/fixtures/mechanic/`):

| Fixture | Content |
|---|---|
| `vortex-dot-witch.xml` | Vortex alone |
| `vortex-dot-modifiers-witch.xml` | Increased DoT and cold damage, spell damage reaching the DoT, the DoT and cold DoT multipliers |
| `righteous-fire-marauder.xml` | Righteous Fire as the main skill, burning damage, the fire DoT multiplier |
| `vaal-righteous-fire-marauder.xml` | Vaal Righteous Fire as the main skill, the gem's Righteous Fire as a buff |

Before-and-after on the evidence bundle with tree packs (`local/p6/base-packs` against `local/p6/p6-1`), and `calc-parity` against the close-out oracle plus the new fixtures' PoB outputs (`local/p6/oracle`):

| Set | Before | After |
|---|---|---|
| Public `calc-parity` | 136 builds; 2,223 match; 0 unexpected | 140 builds; 2,295 match (the 72 fixture targets); 0 unexpected |
| Corpus | 141 unsupported, 170 indeterminate | 111 unsupported, 204 indeterminate |

In the corpus, more main skills now calculate: missing and renamed gems no longer stop their group. Corpus fields stay `indeterminate` or `partial` on their other reasons.

Community builds with a DoT skill (Essence Drain, Vaal Righteous Fire, Vortex) now have `dot-dps`, `partial` on their stats that are not mapped yet (`chaos_damage_+%` and others: P6-11).
