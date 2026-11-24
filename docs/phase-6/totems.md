# P6-6d and P6-6e: totems, traps, mines, brands

The plan's P6-6 covers totems, traps, mines, and brands. This step does totems; traps, mines, and brands, each with its own throwing, laying, or activation rate, are P6-6e.

PoB v2.67.2 computes a totem's skill as the skill's own hit and DoT, per totem: `TotalDPS` is one totem's. Two things differ from a self-cast skill:
1. **Rate.** The totem's action speed, `1 + TotemActionSpeed INC / 100`, replaces the player's (`CalcOffence.lua` 2346 to 2351). Tailwind gives totems action speed only with `Condition:TotemTailwind`.
2. **Bleed stacks** count every active totem (4327 to 4329).

**The engine:**
- `SummonsTotem` is no longer in `hit::NOT_SELF_CAST`, `NOT_PLAIN_ATTACK`, or the DoT's `NOT_COMPUTED_KINDS`.
- `hit::rate` applies the player's action speed to self-cast skills only. No totem action speed stat is mapped, so the totem's factor is 1, and every totem stat ("Totems deal ...", placement speed, active totem limit) is a reason.
- **Bleed:** PoB multiplies bleed stacks by the active totems. At the default of one totem this changes nothing; extra totems come from unmapped stats, which are reasons.
- **Not modelled,** so a totem's hit and DoT carry the reason `totem-data` (`hit::TOTEM_REASON`; the P6-6d review):
  - placement time and duration;
  - Siege Ballista's `totemFireOnce` and its ring override's DPS multiplier;
  - `KeywordFlag.Totem`: there is no totem keyword in the engine, so "Totems deal ..." lines are reasons;
  - the conditions PoB sets for a totem main skill (`HaveTotem`, `SummonedTotemRecently`, `TotemsHitRecently`).

  Without this reason, some vetted totem skills would be `Partial` only because their totem stats are unmapped. Mapping one of those stats would then make the skill a wrong `Complete`; a test lists them.
- **Action speed:** no fixture checks it on a totem against PoB; the reason keeps every totem `Partial` until one does.

**Evidence:**
- **The sweep's 14 totem builds** (one per totem gem effect) against the oracle:
  - no unexpected difference;
  - every hit is `Partial` (totem stats, PoB skill data);
  - 4 equal PoB (Holy Flame Totem, Shockwave Totem and a variant of each).
- **Public `calc-parity`:** 153 builds, 99 expected differences, 0 unexpected; 13 fields of the Holy Flame Totem build moved from not computed to `Partial`.
- **Corpus:** no unexpected difference.
- **Determinism:** both runs were byte-identical.

**The P6-5 review** (code-architecture-reviewer), applied here: `docs/phase-6/channelling.md`, "The P6-5 review".

## P6-6e: traps, mines, brands; the P6-6d review

**Traps and mines.** PoB replaces a trap's or mine's skill time with its throwing or laying time (`skillData.timeOverride`, `CalcOffence.lua` 1223 to 1337, 2208):
- the base is `TrapThrowingTime` 0.6 s or `MineLayingTime` 0.3 s (`CalcSetup.lua` 55, 56);
- the speed is scaled by trap or mine throwing speed and by action speed, and capped at the server tick rate;
- the time is divided by the throw count;
- cast speed does not reach it.

The engine now computes a trap's or mine's **spell** this way (`hit::cast_rate`), with one thrown and no throwing speed (none is mapped). The hit keeps the reason `trap-mine-data` (`hit::TRAP_MINE_REASON`): throw counts, trap and mine speed, cooldowns, and `KeywordFlag.Trap` and `Mine` mods are not read. A trap's or mine's **attack** and DoT stay not computed.

The rate ignores trap and mine throwing speed: the Trap and Mine supports' speed at higher gem levels, Advanced Traps, and tree and gear speed. Those stats are unmapped, so a build with them is `Partial` through `unmapped-stat` as well as the reason; the sweep's equal builds hold only at the base values. (P6-6e review, applied in P6-7a.)

**Brands** stay not computed: their activation rate, attached count, and duration are their own model.

**Sweep:** 33 trap and mine builds against the oracle.
- No unexpected difference, and every hit is `Partial`.
- 14 equal PoB.
- The rate equals PoB's (1.67 per second for traps, 3.33 for mines) except where a variant has its own throwing time.

**Public and corpus:**
- public `calc-parity`: 153 builds, 99 expected differences, 0 unexpected;
- corpus: no unexpected difference;
- both runs byte-identical.

**The P6-6d review** (code-architecture-reviewer), applied:
- the reason `totem-data` (above), with a test listing the vetted totem skills;
- a test that the DoT's excluded kinds equal the attack's;
- the module docs' lists brought up to date.

## Open items S1: `totem-data` narrowed (2026-11-24)

The engine has read most of what P6-6d listed since then: the totem keyword (`Flags::KEYWORD_TOTEM`, `KeywordFlag.Totem` lines mapped), the conditions PoB sets and leaves unset for a totem main skill (`conditions::summoned_totem`, `skill_used_by_you`, `main_skill_recently`; `CalcPerform.lua` 240 to 270), and the totem limit (`stacks::totems`). The totem's action speed (`CalcOffence.lua` 2346 to 2351) comes only from unmapped stats, each a reason. The reason `totem-data` now stays only where PoB reads totem data the engine does not:

- every field of Siege Ballista of Trarthus (`hit::TOTEM_DATA_SKILLS`): its `totemFireOnce` and ring `dpsMultiplier` (`act_dex.lua` 14127 to 14133; `CalcOffence.lua` 2352 to 2355);
- a totem's bleed, whose stacks PoB multiplies by the active totems (4327 to 4330);
- a totem's damage over time (`poe1::dot`, unchanged: `DotCanStackAsTotems`, 5784);
- a minion's totem skill (unchanged).

**Leech.** A totem's, trap's, or mine's hit leeches only through PoB's `DamageLifeLeechToPlayer` (3528 to 3530), which no mapped stat gives; the engine now sets no leech field for such a hit (`hit::set_ailments`). Before, the totem's reason hid the player's leech the engine computed for it.

PoB's other totem paths reach no slice field: the player's auras copied to the totem as `Totem` defence mods (`CalcPerform.lua` 2466 to 2490, 3371), and the totem's level and life (`CalcActiveSkill.lua` 433, 607).

**Evidence** (`local/p6/s1d`, against S1 part 1's `local/p6/s1c`):

- public `Complete` 38,869 to 38,922 (+53: the 5 Ice Nova spell totem fixtures and the 2 Ballista fixtures); no value moved;
- the 2 Ballista fixtures differ from PoB only by the Short Bow's rate in floored ms (Q11: 1000 / 667 against PoB's 1.50, ratio 0.99950 in the speed, hit DPS, ignite, full and combined DPS): DIV-0684 to DIV-0693;
- public `calc-parity`: 692 expected, 0 unexpected; corpus: 0 unexpected, no state changed; both runs byte-identical.
