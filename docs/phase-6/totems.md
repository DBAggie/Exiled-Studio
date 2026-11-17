# P6-6d: totems

The plan's P6-6 covers totems, traps, mines, and brands. This step does totems; traps, mines, and brands, each with its own throwing, laying, or activation rate, are P6-6e.

PoB v2.67.2 computes a totem's skill as the skill's own hit and DoT, per totem: `TotalDPS` is one totem's. Two things differ from a self-cast skill:
1. **Rate.** The totem's action speed, `1 + TotemActionSpeed INC / 100`, replaces the player's (`CalcOffence.lua` 2346 to 2351). Tailwind gives totems action speed only with `Condition:TotemTailwind`.
2. **Bleed stacks** count every active totem (4327 to 4329).

**The engine:**
- `SummonsTotem` is no longer in `hit::NOT_SELF_CAST`, `NOT_PLAIN_ATTACK`, or the DoT's `NOT_COMPUTED_KINDS`.
- `hit::rate` applies the player's action speed to self-cast skills only. No totem action speed stat is mapped, so the totem's factor is 1, and every totem stat ("Totems deal ...", placement speed, active totem limit) is a reason.
- A totem's bleed is not modelled: bleed stacks per totem are a reason through the totem limit's unmapped stats.

**Evidence:**
- **The sweep's 14 totem builds** (one per totem gem effect) against the oracle:
  - no unexpected difference;
  - every hit is `Partial` (totem stats, PoB skill data);
  - 4 equal PoB (Holy Flame Totem, Shockwave Totem and a variant of each).
- **Public `calc-parity`:** 153 builds, 99 expected differences, 0 unexpected; 13 fields of the Holy Flame Totem build moved from not computed to `Partial`.
- **Corpus:** no unexpected difference.
- **Determinism:** both runs were byte-identical.

**The P6-5 review** (code-architecture-reviewer), applied here: `docs/phase-6/channelling.md`, "The P6-5 review".
