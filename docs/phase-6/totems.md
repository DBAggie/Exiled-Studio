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
