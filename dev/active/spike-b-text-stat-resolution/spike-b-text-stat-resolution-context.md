Last Updated: 2026-11-09 (PR B0: plan, plan-reviewer findings applied)

# Spike B: context

## Key files
- `docs/plan/ACTION-PLAN.md`, section "Spike B": goal, gate, fallback.
- `docs/adr/015-stat-id-modifier-model.md`: stat-ID first, text is an import adapter, quarantine.
- `docs/adr/016-wasm-compatible-core.md` and `crates/*/clippy.toml`: core crates do no fs or env, including in tests.
- `docs/adr/007-corpus-and-coverage.md`: corpus seeded from the maintainer's builds, scrubbed.
- `docs/adr/005-build-and-data-compatibility.md`: PoB data is a cross-check only.
- `docs/adr/001-migration-and-platform.md`: no PoE1-only assumptions in shared types.
- `dev/completed/spike-a-bundles2-extraction/spike-a-bundles2-extraction-context.md`, "For Spike B": how to read `stat_descriptions/**/*.json`.
- `docs/spikes/spike-a-extraction.md`, sections 12 (stat-description grammar; markup left to Spike B) and 13 (A5 results).
- `tools/extractor/src/statdesc.rs`: writer of the JSON the `loom-data` loader reads.
- `crates/loom-data`, `crates/loom-parser`, `tools/parity-cli`: empty skeletons today.

## Inputs on the maintainer workstation
- Corpus: `D:\Documents\Path of Building\Builds` (21 XML files). Never commit anything from it unscrubbed.
- Extract: `local/determinism/poe1/run1` (PoE1 build 26530219, A6b tool commit) and `local/determinism/poe2/run1` (PoE2 26476662). Handler counts are in each `manifest.json`, `stat_descriptions.handlers`. The extract has no `PassiveTreeExpansion*`, `CraftingBenchOptions`, or `Essences` tables.
- PoB references, cited at the pinned tag only (`git show v2.67.2:<path>` in `D:\POE Dev\PathOfBuilding`, read only; its working tree is at `v2.67.2-122-g3dbca4463` and must not be cited):
  - `src/Modules/StatDescriber.lua`: `getScope` (precedence), `matchLimit` (conditions; skips `gem_quality`), `applySpecial` (handler rounding).
  - `src/Modules/ItemTools.lua`: `itemLib.applyRange` (ranges, antonyms, `modScalability`, catalyst scalar). Not reproduced in Spike B.
  - `src/Classes/Item.lua`: `ParseRaw` (flag lines, variants and alt variants, catalyst), `getCatalystScalar`.
  - `src/Classes/ItemsTab.lua`: `<ModRange>` handling at load.
  - `src/Export/Scripts/mods.lua`: `loadStatFile("tincture_stat_descriptions.txt", "graft_stat_descriptions.txt")`, output split by `mod.Domain`.

## Decisions
- 2026-11-09 (B0): resolver in `loom-parser`; model in `loom-data`; `pob_item` in `loom-parser` (`loom-serialization` cannot depend on `loom-data`); all disk-reading checks are `tools/parity-cli` subcommands; scope chosen by `BaseItemTypes.ModDomain`; include precedence as PoB's `scopeList`, adopted as a hypothesis and measured; ranged values as intervals (exact `applyRange` is Phase 2); ambiguous and unknown-base lines count as unresolved; each line occurrence counts, distinct rate reported.
- 2026-11-09, maintainer: the corpus has 21 builds (correct); ADR-007 and the ACTION-PLAN text "22" get corrected in B4b (ADR-007 wording via the maintainer).
- 2026-11-09, maintainer: `mod-backed` is reported evidence, not a gate threshold.
- 2026-11-09, maintainer: completed task docs go to `dev/completed/`; Spike A moved there in B0.

## plan-reviewer (2026-11-09) summary
Verdict: proceed with changes. All applied in this plan:
- Blockers: (C1) lookup handlers missing, and they decide the gate (B0 measured 117 non-unique lines); added the lookup-handler design, the B1x extractor PR, and B2b. (C2) `no-description` was a residual bucket; every category now needs positive evidence, `data-gap/legacy-text` added. (C3) extract-backed tests cannot live in core crates (clippy bans fs and env in tests); moved to `parity-cli` subcommands. (C4) `applyRange` was misdescribed (scalability, antonyms, catalysts, `<ModRange>`); ranged values are now intervals. (C5) magic items have no base line; longest-name rule and `unknown-base` added.
- ADR items: cite `v2.67.2:` only; precedence marked PoB-observed; PoB used as cross-check only, stated.
- Should-fix: scope by `ModDomain` (not item class); independent `render-check`; normalization details (markup, literal digits, `gem_quality`, dedupe by description, two-line variants); Mods filter without spawn-tag rejection, same definition for `mod-backed`; exclusion bounds and distinct rate; B2 and B4 split.
- Nits: `pob_item` placement reason, alt variants, second-level category for unique lines, item-set mention removed, `parity-cli` deps and description, handoff wording, Steam build check before B3.

## Open questions for the maintainer
- None now. Exclusion rules beyond the three named ones, if B4b finds a need, come to the maintainer in that PR.

## Facts found while planning
- PoB stores ranged lines as `{range:r}` plus `(a-b)` text, or as `<ModRange id range>` children (legacy; mixed XML content).
- PoB writes enchants and some implicits with a `{crafted}` prefix in the implicit block (cluster jewel `Adds 8 Passive Skills`, flask `Used when Charges reach full`); they count as implicit lines.
- Relic items are uniques (PoB rarity `RELIC`).
- Survey scripts: XML lines are indented; strip each line before matching `Rarity:`.
