# P6-6a: the skill sweep and skill stats, batch 1

The plan's P6-6 generalizes PoB's skill data (parts, flags, the `pob-skill-data` vetting reason) before totems, traps, mines, and brands. P6-5 (channelling) needs those parts, so this step comes first (the order change is in the tasks file). Before generalizing, the sweep below measures what blocks the skills the engine computes.

## The skill sweep

**Builds:** `tools/oracle/sweep/make_sweep.py` writes one minimal PoB build per skill gem effect the engine computes as a plain hit, from the bundle's gem tables. Each skill gets one build per PoB part.
- **A self-cast spell:** a level 90 Witch with the gem at level 20.
- **A plain attack a one-hand axe can use:** a level 90 Templar with a Rusted Hatchet.

The builds are made up from the game's IDs. They are local evidence (`local/p6/sweep`), not committed: 376 builds on the flask evidence bundle. The oracle ran them in 28 minutes, 3 times each.

**Results by value**, comparing the hit fields whatever their state, before this step:
- 77 equal PoB;
- 45 differ only through the weapon's rate (Q11);
- 239 differ;
- 15 are not computed.

Every differing build carries reasons:
- skill stats with no mapping (1,565 on the hit fields);
- parts not computed (`several-stat-sets`);
- other gems.

So the hit is `Partial`, never a wrong `Complete`. Only Smite's parts were clean (vetted).

**What blocks most skills is their own stats, not PoB's hand-written data.** Removing the vetting reason alone would make no further skill `Complete`. This step therefore maps skill stats, and the vetting generalization follows in P6-6b.

## Skill stats, batch 1

1. **`poe1::skill_no_output`: 148 skill stats PoB reads into no output** (plan decision 9). `tools/oracle/sweep/skill_stats.py` lists, among the unmapped stats with a gem source (sweep, public, corpus), those that meet all three conditions:
   1. no stat map names them, neither `Data/SkillStatMap.lua` nor any skill's own `statMap` (PoB reads a gem's stats only through these, `Modules/CalcActiveSkill.lua`);
   2. no module or class names them, and no name PoB builds at runtime (`"..._" ..`) matches one;
   3. no item mod or passive of the bundle carries them (PoB reads item and passive lines by their text).

   They are `NoEffect`. A test keeps the list sorted and apart from the mapped rows.
2. **Seven more, checked by hand the same way:** `NO_EFFECT` "display", for example `skill_override_pvp_scaling_time_ms`.
3. **Two more, following earlier precedent:** `active_skill_base_radius_+` ("skill-radius", as the skill's own base radius) and `active_skill_area_of_effect_+%_final` ("area", as `base_skill_area_of_effect_+%`).
4. **Mapped:**
   - `skill_physical_damage_%_to_convert_to_fire` and `_cold` (`SkillConversion`, `SkillStatMap.lua` 962 and 965, as the lightning row);
   - `global_always_hit`: the `CannotBeEvaded` flag, which `CalcOffence.lua` 2162 reads as it reads `skill("cannotBeEvaded")`.

**Sweep after the batch:**
- 83 builds equal PoB (+6), and 57 differ only through the rate (+12);
- no build moved away from PoB;
- complete fields: 325 to 620.

**Public and corpus:**
- distinct unmapped stats: 1,060 to 1,031;
- public `calc-parity`: 153 builds, 99 expected differences, 0 unexpected, field counts unchanged (each build has other reasons);
- corpus: no unexpected difference;
- both runs byte-identical.

## Left for the next batches

Shared skill stats that PoB maps to outputs not computed yet (decision 9):
- projectile count (`base_number_of_projectiles`, 72 sweep builds);
- chains, pierce, secondary duration;
- `base_skill_show_average_damage_instead_of_dps` (PoB's `showAverage`, which changes what `TotalDPS` holds);
- each skill's own stat map entries.

The vetting reason (`pob-skill-data`) becomes a rule in P6-6b: PoB's flags and parts from the generated table, with skill functions and tagged base mods as reasons. The sweep is the evidence for each skill it unlocks.
