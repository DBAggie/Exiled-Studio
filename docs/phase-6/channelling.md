# P6-5: channelling and Vaal skills

**Channelled skills.** PoB v2.67.2 computes a channelled skill's hit and DoT as any skill's. Three things differ:
1. its rate is not capped at the server tick rate (`CalcOffence.lua` 2366);
2. its stages and its release come from skill data (`hitTimeMultiplier`, `hitTimeOverride`) and from stat map entries tagged with a stage multiplier or a part;
3. it sets `Condition:Channelling` (`CalcPerform.lua` 255).

The engine now computes channelled skills: `Channel` is no longer in `hit::NOT_SELF_CAST`, `NOT_PLAIN_ATTACK`, or the DoT's `NOT_COMPUTED_KINDS`, and `hit::rate` leaves a channelled rate uncapped.
- **Stages and release:** read through PoB's data (base mods, tagged stat map entries), so the vetting rule (`hit::pob_vetted`, `docs/phase-6/skill-data.md`) leaves such a skill `Partial` until a step ports them.
- **`Channelling`:** left unsettled, so a modifier that reads it is a reason (the config input `multiplierChannelling` names it, `config::NAMED_ONLY`).

**Vaal skills.** A Vaal gem grants two skills. With the Vaal one as the main skill, the engine computes it as any other skill: the skill model picks the main active skill, and soul costs reach no field. The P6-6a sweep held 13 Vaal skills; P6-6b's rule moved 11 of them closer to PoB, none further. Nothing more to port here: a Vaal skill's own data acts through the same vetting.

**Evidence:**
- **The sweep's 49 new channelled builds:**
  - against the oracle, no unexpected difference;
  - every hit is `Partial` (stages, unmapped skill stats, parts not computed), so none claims a wrong `Complete`;
  - 7 equal PoB, and Cyclone differs only through the rate (Q11).
- **Public `calc-parity`:** 153 builds, 99 expected differences, 0 unexpected; 12 fields of the Cyclone build moved from not computed to `Partial`.
- **Corpus:** no unexpected difference.
- **Determinism:** both runs were byte-identical.

**The P6-6c review** (code-architecture-reviewer), applied here: `docs/phase-6/skill-data.md`, "The P6-6c review, applied in P6-5".

**Left:** porting the stages (Blade Flurry, Scorching Ray, Divine Ire...), the release skills, and settling `Channelling`. Each is a reason now and belongs to the mop-up batches, measured by the sweep.
