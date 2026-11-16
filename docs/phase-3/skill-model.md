# Phase 3: skill model (P3-9a)

Plan step 9a (`dev/completed/phase-3-calc-slice/phase-3-calc-slice-plan.md`): the main skill, its supports, and each gem instance's stats at its level and quality, from the game data's skill tables. No slice field changes yet: the hit fields (`AverageDamage`, `Speed`, `CritChance`, ...) are computed from this model in P3-9b (spells) and P3-10 (attacks), and enabled gems stay `unmodelled-gem` on the character fields until auras and buffs (P3-12). Decisions: context file, "Decisions in P3-9a".

## What is modelled (`crates/loom-calc/src/poe1/skill.rs`)

| Part | Rule (PoB v2.67.2) | Source |
|---|---|---|
| Main skill | the active skill set; `mainSocketGroup` and `mainActiveSkill` clamped; each enabled gem's granted effects (its own, then additional ones) that are not supports; an effect PoB may count only with its global effect on (`enableGlobal<n>`) is tried both ways, and differing picks are `main-skill-uncertain` | `Modules/CalcSetup.lua`, `Classes/SkillsTab.lua` |
| Supports | in group order, each once: not when the skill cannot be supported, a gems-only support on an item-granted skill, or its excluded types match; else when its allowed types match (or are empty); applied supports add their types | `Modules/CalcActiveSkill.lua`, `calcLib.canGrantedEffectSupportActiveSkill`, `calcLib.doesTypeExpressionMatch` |
| Gem level | the gem's level plus the build's global gem level modifiers its tags match (P3-8i, `poe1::gem_levels`, PoB's `applyGemMods`; `docs/phase-3/stat-mapping.md`, "P3-8i"), kept when the effect has it, else clamped to `1..=#levels`; a level PoB would find only by its later fallbacks (natural maximum, any level) is a problem | `calcLib.validateGemLevel` |
| Instance stats | quality stats `modf(per 1% x quality)` with the per-1% value as PoB's exporter writes it (`per_mille / 1000`, a double), plus the level's integer stats, the stat set's implicit stats as 1 each (PoB's `level[index] or 1`; Ice Nova lists `is_area_damage` among its stats), plus the constant stats, summed per stat; only the first stat set (PoB exports only it) | `calcLib.buildSkillInstanceStats`, `Data/Skills/*.lua` |
| Problems | a level stat with linear interpolation (code 2), several stat sets (flagged; only the first counts), a missing stat set, no level (`level` is `None`) | — |
| Not modelled | modifiers that raise gem quality, and gem levels that depend on the socket or name a gem (global gem levels: P3-8i): their stats are unmapped, so the fields that read the skill will be `Partial` while they are in the build | — |

The main-skill rule is the one `calc-survey` used since P3-3; it moved into the engine and the survey now calls it, so the survey and the calculation pick the same skill. Effectiveness-scaled float stats (base damage, interpolation 3) need a power function on the output path and belong to P3-9b, with the audited math module.

`CalcData` reads skill gems, gem effects, granted effects, active skills, and stat sets (required methods: an implementor without skill tables returns `None`, and the skill reads as unknown). The parity tool's `OverlayData` refuses a skill the overlay has a row for, at read time (maintainer decision 2026-11-12): the skill reads as missing in an overlay run, so its fields are indeterminate there, never computed from the game's row under the overlay's name. P3-2's overlay has 19 such rows, all monster, item-granted, or unique effects (`AvengingFlame`, `MinionInstability`, ...), none a skill the in-scope builds use.

## Evidence (2026-11-12, Windows)

| Run | Result |
|---|---|
| `calc-survey`, public fixtures (60 builds) and the corpus (21, `--anonymous`), with the tool before (`8b11a17`) and after (`89a7504`, again at `ab9ab17`) | `report.json` and `summary.json` byte-identical before and after, for both: the moved rule picks the same main skill, supports, and types for every build. Exit codes as before (public 0: Ice Nova 3 and Smite 4 in scope; corpus 1, no in-scope build on its own, as P3-3b recorded) |
| Ice Nova and Smite at level 20, quality 20 in the real bundle against PoB v2.67.2's own skill data (`Data/Skills/act_int.lua` `IceNova`, `act_str.lua` `Smite`), maintainer-run (`LOOM_BUNDLE_DIR=<bundle> cargo test -p loom-calc --test skill -- --ignored`, from clean `ab9ab17`) | **Equal**: Ice Nova radius 33 (0.15 x 20 + 4 + 26), freeze 25, `is_area_damage` 1; Smite's 13 stats (area damage -10 = 10 - 20, the quality display 20, shock 19, radii 21 and 42, constants, three implicit flags) |
| Workspace tests | 1066 pass; clippy, fmt |

The oracle output records no main-skill ID (`probe` holds only the CALCS alignment), so the main skill is checked against PoB through the hit fields in P3-9b; the instance stats are checked above against PoB's own skill data.

Tests: `crates/loom-calc/tests/skill.rs` (9, over the synthetic bundle: the main group's active gem and supports; clamping and missing gems; an item-granted skill; quality, level, and constant stats; level clamping and no level; the double-product truncation PoB's `math.modf` gives, 0.29 x 100 = 28; interpolated stats and several or missing stat sets; instances at each gem's level and quality; postfix type expressions; only the first stat set counting; the real-bundle check above, ignored in CI); the overlay's per-skill refusal (`calc_run`); the survey's own tests on the moved rule.
