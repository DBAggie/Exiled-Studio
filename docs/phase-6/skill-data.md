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
- **`full-dps-vaal`** (`poe1::full_dps`): PoB's `getActiveSkillCount` (`Calcs.lua` 150 to 174) counts a Vaal gem's first effect when `enableGlobal1` is on (the default) and its second when `enableGlobal2` is (`skill::vaal_full_dps_enabled`; the game's `IsVaalGem`, the bundle's `gems.json` `vaal`). A Vaal skill that summons minions keeps the reason: PoB's full DPS of Vaal Summon Skeletons (417) is not its minion's `TotalDPS` (115) times the count, for a cause not found (open items follow-ups).

**Evidence** (`local/p6/s1c`, bundle `b-ct`, against `local/p6/n` at `dev` `6deeb5da`):

- public `Complete` 38,757 to 38,869 (+112: 7 Shield Crush and 3 Vortex fixtures, the trap and Vaal Righteous Fire builds); no value moved;
- public `calc-parity`: 682 expected differences, 0 unexpected; corpus: 0 unexpected, no state changed;
- both runs byte-identical;
- `reservation-becomes-cost` is in no build any more.

**Not reached in S1** (open items follow-ups, "Planned for the session, may not be reached"): totems' `totem-data`, Static Strike's part 1, the trigger builds (S3).
