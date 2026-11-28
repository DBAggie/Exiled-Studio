# P6-6a and P6-6b: the skill sweep, skill stats, and PoB's skill data

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
   2. no other PoB file names them: every `.lua` under `src` but the stat descriptions and the skills' constant stat lists (P6-6b: the first run scanned only `Modules` and `Classes`; the whole tree gives the same list). No name PoB builds at runtime (`"..._" ..`) matches one either;
   3. no item mod, mastery effect, passive, or base item of the bundle carries them (PoB reads those lines by their text). `cruelty_effect_+%` is on a mastery ("30% increased Effect of Cruelty"), which PoB's parser leaves unsupported (`Data/ModCache.lua` 4930, leftover text): it stays in the list.

   They are `NoEffect`. A test keeps the list sorted and apart from the mapped rows.
2. **Seven more, checked by hand the same way:** `NO_EFFECT` "display", for example `skill_override_pvp_scaling_time_ms`.
3. **Two more, following earlier precedent:** `active_skill_base_radius_+` ("skill-radius", as the skill's own base radius) and `active_skill_area_of_effect_+%_final` ("area", as `base_skill_area_of_effect_+%`). These two are not "read into no output": PoB reads them into its radius outputs (`CalcOffence.lua` 346 to 371, 1370). They follow the Phase 3 precedent that no slice field reads them. Revisit both when radius becomes a field.
4. **Mapped:**
   - `skill_physical_damage_%_to_convert_to_fire` and `_cold` (`SkillConversion`, `SkillStatMap.lua` 962 and 965, as the lightning row);
   - `global_always_hit`: through a gem only (P6-6b, the P6-6a review), the `CannotBeEvaded` flag, which `CalcOffence.lua` 2162 reads as it reads `skill("cannotBeEvaded")`. An item's "Hits can't be Evaded" is tagged with its hand in PoB (`ModCache.lua` 9458), so on an item or a passive the stat stays a reason.

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

## P6-6b: PoB's skill data as a rule

**Hit.** `hit::pob_part` and `hit::pob_vetted` replace `SKILL_PARTS` and the vetted lists (`VETTED_SPELLS`, `VETTED_ATTACKS`).
- **A spell's or attack's part and flags** come from PoB's generated skill table: the base flags `area`, `melee`, `projectile`, each overridden by the part's own field. The part is `m_min(#parts, skillPart or 1)`. Part 0 of a skill with parts is not computed, as before.
- **Spells now take the projectile flag.** Before, spells had none, so projectile modifiers missed projectile spells.
- **A skill is vetted** when the engine reads all of PoB's hand-written data:
  - no skill function;
  - no base mod but untagged display `SkillData` (the radius and its labels);
  - a part with no field but its name and flags.

  Otherwise the hit keeps `pob-skill-data`. Without PoB data, the game's flags are used and the hit is `Partial`.

**DoT.** The same check, run on the sweep, found four skills whose DoT was `Complete` but differed from PoB (P6-1). Three of them PoB v2.67.2 does not know (`CreepingFrostAltY`, `ExsanguinateAltY`, `VortexAltY`): PoB computes no DoT for them, the engine did. Exsanguinate has base mods the engine does not read (its 3 stages, tagged mods). The DoT is now `Partial` (`pob-skill-data`) when PoB has no data for the skill, or has a base mod other than the display data and the DoT's own keys and flags.

**Evidence** (the sweep, the oracle outputs of P6-6a):
- 12 builds closer to PoB, none further;
- **no `Complete` field differs from PoB except through the weapon's rate (Q11)**: Speed, hit DPS, and one ignite that scales with the rate;
- complete fields: 620 to 667.

Public `calc-parity` (153 builds, 99 expected differences, 0 unexpected) and the corpus are unchanged; both runs are byte-identical.

**The P6-6a review (code-architecture-reviewer), applied:**
- **`global_always_hit`** acts through a gem only (`mapping::ALWAYS_HIT_STAT` in `SkillMapping`). Before, the gem path dropped it and the item path applied it without PoB's per-hand tag.
- **The classifier** scans the whole PoB tree and the mastery effects and base items. The same list results, and `cruelty_effect_+%` is kept by hand (PoB leaves its mastery text unsupported).
- **Tests:** the 148 stats through both mappings, the fire and cold rows, the flag scale.
- **The flask merge reason** names its slot.
- **The radius rows' wording** now gives the Phase 3 precedent.

## Still open

- Shared skill stats PoB maps to outputs not computed yet: projectiles, chains, pierce, `showAverage`.
- Each skill's own stat map: P6-11's batches, measured by the sweep.
- Base mods and skill functions the engine does not port: reasons until a step ports them.

## P6-6c: the P6-6b review

The review (code-architecture-reviewer) found two ways the P6-6b rule let a wrong value through as `Complete`. Neither was in the sweep's builds: the sweep's evidence covers its own builds only.

1. **Melee and projectile.** PoB's data gives some vetted skills both flags (22 after the stat map rule below, the default attack `Melee` among them; none a spell). PoB keeps one of them, by the main-hand weapon (`CalcActiveSkill.lua` 302 to 308): a melee weapon clears projectile, a bow or wand clears melee. It then applies melee in preference to projectile (361 to 366). The engine applied both; now it follows PoB's rule.
2. **Tagged stat map entries.** PoB reads some stats of a skill's stat map under a tag:
   - Conflagration's DoT only in part 2;
   - Animosity's and Resentment's DoT per stack;
   - Shattering Steel of Ammunition's damage per distance.

   The generated table now carries each skill's `own_stats`: the stats with a tagged mod acting on the skill itself (none `GlobalEffect`, nor PoB's `notMinionStat` marker). A skill with any is not vetted, for the hit and for the DoT. This is conservative: a skill can be unvetted for a stat its level lacks. The table is regenerated from export format 11. It lists 430 skills, and Smite, Ice Nova, and Ethereal Knives have none.

Also:
- the DoT's `SkillData` keys and flags are explicit lists (`dot::DOT_SKILL_DATA_KEYS`, `DOT_FLAGS`), not prefixes;
- the `pob_vetted` doc no longer claims every hand-written datum is read;
- tests cover Conflagration, Animosity, and Frenzy's two flags.

**Evidence:**
- **Sweep:** no `Complete` field differs from PoB except through the weapon's rate (Q11). Complete fields: 667 to 661 (the conservative rule).
- **Public `calc-parity`:** unchanged (153 builds, 99 expected, 0 unexpected).
- **Corpus:** no unexpected difference.
- **Determinism:** both runs were byte-identical.

### The P6-6c review, applied in P6-5

- **Tests:** the weapon rule is a pure function (`hit::weapon_flags`) with its own test. The vetted example with both flags is the default attack (`Melee`); Frenzy has tagged stat map entries, so it is not vetted.
- **Melee over projectile** (`CalcActiveSkill.lua` 361 to 366) for spells and for the DoT's flags too. No vetted spell has both flags.
- **Limits stated:** a skill's own untagged stat map entries are taken as the plain reading. Their `div` or `mult` against the engine's mapping is not compared, and the sweep checks only its own builds.

## Open items S1 (2026-11-24): what the engine already reads

The open items survey (S0, `dev/active/open-items/open-items-inventory.md`) found builds whose only reason was `pob-skill-data` (or `reservation-becomes-cost`, `full-dps-vaal`) with every field equal to PoB. The vetting rule rejected PoB data the engine reads; three lists in `poe1::hit` now name that data, each entry cited and checked against the generated table by a test (`read_lists_should_vet_only_what_the_engine_reads`):

- **`READ_OWN_STATS`:** tagged stat map entries the engine reads with their tag. Shield Crush's added damage per 15 of the shield's armour and evasion (`SkillStatMap.lua` 1675 to 1697, `hit::shield_added`, P6-11hd) and its damage by distance (`MeleeProximity` ramp `{1, 0}`, `collect::melee_proximity`, P6-11ht).
- **`READ_PART_BASE_MODS`:** a base mod tagged with one part, which the engine does not read: that part keeps the reason, the others are vetted. Shield Crush's `dpsMultiplier` 2 in part 2 (`act_str.lua` 9887); the engine's `dpsMultiplier` is 1.
- **`READ_FUNCTIONS`:** a skill function that changes nothing while the level has no cooldown (`pob_vetted_with`, called by the spell path with the level's cooldown). Vortex's `preDamageFunc` sets `hitTimeOverride = output.Cooldown` (`act_int.lua` 20542), read only when set (`CalcOffence.lua` 2403). The engine maps no added cooldown, so without the level's cooldown PoB's is nil.
- **The DoT's flags** (`dotIsArea` and kin, `dot::DOT_SKILL_DATA_KEYS`) no longer unvet a hit: PoB reads them only in the DoT's config (`CalcOffence.lua` 5657 to 5669), and the DoT vets itself.

Two other reasons, narrowed the same way:

- **`reservation-becomes-cost`** (`poe1::costs`) only when a reservation reaches the cost: PoB adds the active level's flat or percent mana reservation, or a support level's `manaReservationPercent` (`CalcOffence.lua` 1693 to 1700, `CalcActiveSkill.lua` 516). Trap and totem supports add the type to skills that reserve nothing.
- **`full-dps-vaal`** (`poe1::full_dps`): PoB's `getActiveSkillCount` (`Calcs.lua` 150 to 174) counts a Vaal gem's first effect when `enableGlobal1` is on (the default) and its second when `enableGlobal2` is (`skill::vaal_full_dps_enabled`; the game's `IsVaalGem`, the bundle's `gems.json` `vaal`). A Vaal skill that summons minions kept the reason until the agent list found the cause (2026-11-26): PoB's full DPS of Vaal Summon Skeletons of Mages counts another minion than its main output, a `pob-bug` (DIV-0783; `docs/phase-6/minions.md`, "Vaal Summon Skeletons' full DPS"). It now counts its own minion.

**Evidence** (`local/p6/s1c`, bundle `b-ct`, against `local/p6/n` at `dev` `6deeb5da`):

- public `Complete` 38,757 to 38,869 (+112: 7 Shield Crush and 3 Vortex fixtures, the trap and Vaal Righteous Fire builds); no value moved;
- public `calc-parity`: 682 expected differences, 0 unexpected; corpus: 0 unexpected, no state changed;
- both runs byte-identical;
- `reservation-becomes-cost` is in no build any more.

**Totems** (S1 part 2): `totem-data` narrowed to what PoB reads and the engine does not (`docs/phase-6/totems.md`, "Open items S1"); public `Complete` 38,869 to 38,922, the Ballista fixtures' Q11 entries DIV-0684 to DIV-0693.

**Not reached in S1** (open items follow-ups, "Planned for the session, may not be reached"): Static Strike's part 1, the trigger builds (S3).


## The agent list (2026-11-26): the item-triggered skills' PoB data

After S7, `pob-skill-data` was the reason that blocked the most fields in the public set (172 fields in 27 builds). It was the only reason on the fields of six of S3 part 7's item-triggered skills, each equal to PoB's (`local/p6/tools/skill_data_skills.py` names the skills). What failed the vetting rule in each:
- Molten Burst: the base mod `CannotSplit`.
- Rain of Arrows: the base mod `OneShotProj`.
- Unseen Strike: the stat map entry `skill_has_trigger_from_unique_item`.
- Bone Nova and Icicle Burst: the stat map entry `spell_uncastable_if_triggerable`.
- Icicle Burst also has the base mod `showAverage`.
- Spark has a skill function and parts.

Two generic lists in `poe1::hit` now name what the engine reads of that data:

- **`TRIGGER_OWN_STATS`:** `spell_uncastable_if_triggerable` and `skill_has_trigger_from_unique_item` only set `skillData.triggered` and `triggeredByUnique` (`SkillStatMap.lua` 191 to 203). PoB reads these as a triggered skill:
  - its rate comes from the trigger (`CalcOffence.lua` 2205 to 2227);
  - it does not set "attacked recently" or "cast a spell recently" (`CalcPerform.lua` 240);
  - it is never a trigger's source (`CalcTriggers.lua` 41).

  The engine computes such a skill through `poe1::triggers`, and `conditions::main_skill_recently` already reads the trigger kind.
- **`READ_BASE_FLAGS`:**
  - `CannotSplit` is read only for the split count (`CalcOffence.lua` 1069), which no slice field holds.
  - `OneShotProj` is read only with `SequentialProjectiles` (769). Its one mapped source, Barrage's `projectiles_barrage`, already guards the projectile count (`mapping::NO_EFFECT_DEPENDS_ON`).

Icicle Burst stays partial. PoB turns `showAverage` off only when a trigger sets the rate (2227), and the vetting does not know the trigger; the attack path does not read the flag. Spark stays partial for its function and parts. (Both cleared by open items S1, below.)

The test `read_lists_should_vet_only_what_the_engine_reads` checks that each listed entry is in PoB's data, that the four skills are vetted, and that Icicle Burst is not.

**Evidence** (`local/p6/ag1`, `b-ct`, against S7 part 9's run):
- **Public:** 0 unexpected; `Complete` 48,804 to 48,837: 33 fields of four fixtures (`trigger-ngamahus-flame-molten-burst-templar.xml`, `trigger-lioneyes-paws-rain-of-arrows-ranger.xml`, `trigger-hidden-blade-unseen-strike-ranger.xml`, `trigger-uul-netols-bone-nova-templar.xml`). No value moved, and no other document changed.
- **Bone Nova:** the fixture's rate is S3 part 7's Q11 difference (0.21499 against PoB's 0.21505; Smite's attack rate in floored ms fed to PoB's rotation gives each side's value to the bit). Its fields are now `Complete`, so they are registered: DIV-0761 to DIV-0765, Q11.
- **Corpus:** 0 unexpected; documents byte-identical.
- **Tests:** the workspace tests, the real-bundle tests, clippy, rustfmt, and the wasm32 check pass.

**The Holy Relic's Nova** (the agent list's first item) was looked at and left. No public or corpus build uses Summon Holy Relic, so it would clear only its own fixture. It needs:
- the minion skill's level picked by level requirement (`CalcActiveSkill.lua` 895 to 902; the engine builds a minion skill at level 1);
- the gem's Holy Relic's Boon, a buff behind the config check `summonHolyRelicEnableHolyRelicBoon`;
- PoB's `nova` trigger with the minion as its actor (`CalcTriggers.lua` 1167): the character's fastest attack, times its chance to hit, capped by the Nova's cooldown, gives the minion's rate (the kept fixture: 0.2806, Smite's 1.275 at a 22% chance to hit).

## Open items S1 (2026-11-26): Icicle Burst's `showAverage` and Spark's part 1

The two skills S3 part 7 left partial (above), each the only reason on its fixture's fields:
- **Icicle Burst's `showAverage`** (an untagged `SkillData` base mod). PoB clears it when a trigger sets the skill's rate (`CalcOffence.lua` 2223 to 2227: `skillData.showAverage = false`). Otherwise PoB reads it in many outputs: the combined DPS, leech, poison and bleed stacks, impale (333, 3926, 4014, 4767, 5535, 5983, 6131).
  - `hit::pob_vetted_triggered` vets a skill with that base mod read (`SHOW_AVERAGE_KEY`).
  - The attack path and the spell path use it only when a trigger's rate is set (`triggers::Outcome::Rate`), and never for Cast on Death, which keeps `showAverage`.
  - A self-cast skill with its own `showAverage` keeps `pob-skill-data`, as before: Essence Drain, Hexblast, Earthquake, and the others in PoB's data.
  - The attack path now adds the reason after the trigger's outcome; `SpellSkill::vetted_triggered` carries the spell's.
- **Spark's `preDamageFunc`** (`act_int.lua` 16721 to 16727) multiplies the DPS by the hits per duration in part 2, "Maximum Hits", only. In part 1, "1 Hit", it changes nothing. `hit::NOOP_PART_FUNCTIONS` names (skill, function, part) entries a part does not run; Spark's part 2 keeps the reason.

Tests: `read_lists_should_vet_only_what_the_engine_reads` (Icicle Burst vetted only when triggered), `spark_should_be_vetted_in_its_first_part_only`.

**Evidence** (`local/p6/s1-a`, `b-pb`, against `local/p6/s9a`):
- **Public:** 0 unexpected; `Complete` 49,371 to 49,386; no value moved.
  - `trigger-mjolner-spark-templar.xml`: seven fields `Complete`, each equal to PoB's.
  - `trigger-camerias-icicle-burst-templar.xml`: eight fields `Complete`. Its trigger rate is S3 part 7's Q11 difference (Smite's attack rate in floored ms fed to PoB's rotation, as Bone Nova's), now registered: DIV-0850 to DIV-0854.
  - Both fixtures retargeted: 86 and 83 targets.
- **Corpus:** 0 unexpected; no document changed.
- **Tests:** the workspace tests, the real-bundle tests, clippy, rustfmt, and the wasm32 check.

**Cyclone** (open items S1, 2026-11-26). Its `initialFunc` adds `Multiplier:AdditionalMeleeRange` (`act_dex.lua` 4701 to 4714). Only its own stat `cyclone_area_of_effect_+%_per_additional_melee_range` reads that multiplier (`AreaOfEffect` INC per it), and `AreaOfEffect` feeds only radius outputs, already a no-effect row (P6-11dl).
- `hit::NO_SLICE_FUNCTIONS` and `NO_SLICE_OWN_STATS` name Cyclone's and Vaal Cyclone's.
- The claim is checked for these two skills only: `AreaOfEffect` reaches damage for skills whose damage scales with their area, which Cyclone's does not. Another skill's area-only data goes on these lists only after the same check (review of S1).
- Cyclone of Tumult's stage stats are not among them; it stays partial.
- Test: `cyclone_should_be_vetted_by_its_area_only_data` (each entry in PoB's data, each stat a no-effect row).

**Evidence** (`local/p6/s1-b`, against `local/p6/s1-a`):
- **Public:** 0 unexpected; `Complete` 49,386 to 49,388: the `FullDPS` of `trigger-cwc-channelling-cyclone-templar.xml` and `trigger-cwc-ice-nova-templar.xml`, which add Cyclone's hit DPS (Cast while Channelling's source).
- **Q11:** the two `FullDPS` (1,953.648 against PoB's 1,953.658) are DIV-0855 and DIV-0856.
  - Ice Nova's part equals PoB's.
  - Cyclone's attack rate is the weapon's floored attack time: a copy of the fixture with Cyclone as the main skill gives `Speed` 4.4978 against PoB's 4.5, and `TotalDPS` 21.0882 against 21.0987, the whole difference.
- **Unchanged:** no value moved; the other Cyclone builds (`Dual Wield Cospris CoC.xml`, `cyclone-shockwave-slayer.xml`) keep their other reasons.
- **Corpus:** no document changed.

**Ground Slam** (open items S1, 2026-11-26). Its own stat `groundslam_damage_to_close_targets_+%_final` is `Damage` MORE with `ModFlag.Hit` and `Condition:AtCloseRange` (`act_str.lua` 2086 to 2088).
- It is now a state row of the mapping under `conditions::AT_CLOSE_RANGE`, which the config's `conditionAtCloseRange` settles (`ConfigOptions.lua` 1765), and is listed in `hit::READ_OWN_STATS`.
- Two original fixtures, `ground-slam-close-range-templar.xml` (the check on) and `ground-slam-templar.xml` (off), each equal to PoB's but the Q11 attack rate (DIV-0857 to DIV-0866); 83 targets each.

**Evidence** (`local/p6/s1-c`, against `local/p6/s1-b`):
- **Public:** 0 unexpected; `Complete` 49,388 to 49,564 (the two fixtures).
- **Unchanged:** no other value moved. The two builds with Ground Slam (`boneshatter-juggernaut.xml`, `smite-inquisitor-act1.xml`) keep their other reasons.
- **Corpus:** no document changed.

**Socket colour quality under a socket rebuild** (open items S1, 2026-11-26; `unmodelled-skill` `socket-colour-quality`, 588 public fields in four builds). A gem's quality from a socket of its colour (P3-12i, `docs/phase-3/other-gems.md`) was not known on an item with a line that rebuilds its sockets. `poe1::sockets` now reads those lines as PoB does (`Item.lua` 2591 to 2634):
- **"Has no Sockets"** removes them all; PoB checks it first.
- **"Has # Sockets"** keeps the `Sockets` line's first # sockets in order. PoB fills a missing one with the item's default colour, which the engine does not read, so a count above the line's sockets stays not known. "Has 1 Socket" shows no number: its value is the description's one, read from the hidden value.
- **"Has # Abyssal Sockets"** (without a count line) keeps the line's other sockets in order and appends the abyssal ones, so a line whose abyssal sockets are its last ones, as many as stated, is kept as it is; any other line stays not known.
- **The builds:** the Unset Rings of `animate-guardian-guardian.xml`, `forged-frostbearer-spectre-necromancer.xml`, and `Mirage Archer Toxic Rain.xml` ("Has 1 Socket" with one socket on the line), and `flicker-strike-berserker.xml`'s weapon ("G-G-G-G-G A" with one abyssal socket).
- **Test:** `socket_count_lines_should_rebuild_as_pob_does` (and `socket_lines_should_count_only_when_part_of_the_item`, each rebuilding stat's reading).

**Evidence** (`local/p6/s1-d`, against `local/p6/s1-c`):
- **Public:** 0 unexpected; every `socket-colour-quality` reason is gone; `Complete` unchanged (49,564), as the four builds have other reasons.
- **Moved:** one value, toward PoB: `animate-guardian-guardian.xml`'s `FullDPS` 2,715,141 to 2,715,214 (PoB 12,186,392). Three other documents changed in their reasons only.
- **Corpus:** no document changed.
