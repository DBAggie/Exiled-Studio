Last Updated: 2026-11-24

# Open items (tasks)

## Prepare (2026-11-24, done)
- [x] Record PR #167's merge into `dev` (`6deeb5da`) in the handoff and CLAUDE.md
- [x] Plan, context, tasks, and `maintainer-follow-ups.md`
- [x] `local/p6/tools/rank_reasons.py`

## Session start
- [ ] Check `dev`'s CI run at `6deeb5da` (oracle jobs); if a fixture fails, fix it first in this branch
- [x] Review the plan with the `plan-reviewer` agent; findings applied 2026-11-24 (agent-only register entries, scriptable gate with `open-items-inventory.json` and `inventory_check.py`, committed PoB instrumentation, `b-ct` checked before calling data missing, follow-ups completed, order by effort with S3, S4, S7 as stretch, fixture budget, the v2 variant check before each step)
- [x] Open the PR (`gh pr create --base dev`); write its number into the handoff and the context file, and push: PR #168

## Steps (S0 first; then by fields cleared per unit of effort; S3, S4, S7 stretch)
- [x] S0: survey and classify; `open-items-inventory.json` and `.md`; `local/p6/tools/inventory_check.py`; the fixture budget; class B and C items copied into `maintainer-follow-ups.md`. Found (2026-11-24): 344 distinct reasons, 275 A, 69 C (the 68 game-truth stats and the older item wordings of `no-template`), B only as gem parts (Skitterbots); fixture budget 75 for the session (about 14.8 s each on the Windows oracle job). **Order from S0** (`local/p6/tools/build_blockers.py`: builds blocked by one or two reasons): S1 first, since its existing fixtures clear without new ones (Shield Crush 7 builds and Vortex 3, `pob-skill-data` alone; 7 totem builds, `totem-data` and `reservation-becomes-cost` alone; the CwC builds; `full-dps-vaal`), then S8, S10, S9, S2
- [ ] S1: main skills without PoB skill data, and the related `unmodelled-skill` kinds
  - [x] S1 part 1 (`local/p6/s1c`): the vetting rule's read lists (Shield Crush, Vortex, the DoT's flags), `reservation-becomes-cost` only when a reservation reaches the cost, a Vaal gem's `enableGlobal` flags in the full DPS; public `Complete` 38,757 to 38,869, 0 unexpected (`docs/phase-6/skill-data.md`, "Open items S1")
  - [ ] the rest of S1: other `pob-skill-data` skills by builds (Cyclone, Static Strike part 1, Fire Trap, Thrown Shield, ...), `socket-colour-quality`, `sigil-of-power-stages`, `skill-chill-effect`, `minion-buffs`; totems and Static Strike are in the follow-ups as "may not be reached"
- [ ] S2: unmodelled support and active gems
- [ ] S3: triggers and Spellslinger
- [ ] S4: Blood and Sand, and PoB's full DPS pass
- [ ] S5: costs per second, warcry power, mirage warriors, Sacred Wisps
- [ ] S6: minion damage over time (spectres if their data is in the bundle)
- [ ] S7: ward, EHP's other pools, maximum hit, degeneration, a dual wielder's leech
- [ ] S8: config inputs; the close-out's closed reasons; repeats' `FINAL` and `FINAL_DPS`; the two guard-kept stats
- [ ] S9: resolver and item gaps
- [ ] S10: PoB's mana `PercentStat`
- [ ] S11: report section, handoff, CLAUDE.md, follow-ups file
