Last Updated: 2026-11-09 (PR B2b: multi-line, lookups, round-trip, render-check)

# Spike B: text to stat-ID resolution (plan)

## Summary
Spike B in `docs/plan/ACTION-PLAN.md` (Phase 1, second spike). Prove that item modifier text can be resolved to `(stat_id, values)` by reverse-matching the game's stat-description tables that Spike A extracts (ADR-015), without PoB's text patterns (`ModParser.lua`). The corpus is the maintainer's PoE1 builds. Spike C (oracle harness) and Phase 2 (PoB XML import through stat resolution) build on this.

## Current state (2026-11-09)
- **PR B0 merged** (PR #13, `c71fa9c`). **PR B1 merged** (PR #14, `cb9b24d`): `loom-data::stat_desc` (model, loader, `Scope`, markup stripping, renderer) and `loom-parity-cli load-statdesc`; both extracts load with 0 problems (context file, "PR B1"). **PR B1x merged** (PR #15, `186b72f`): the six lookup tables are `extract` roots (PoE1 all six, PoE2 four; context file, "PR B1x"). **PR B2a merged** (PR #17, `7fa4c65`): `loom-parser::resolve` (index, tokenizer, verification) and `loom-data::stat_desc::invert` (context file, "PR B2a"). **PR B2b open, in review**: multi-line and lookup matching, `parity-cli round-trip` and `render-check` (context file, "PR B2b"). Next: PR B3.
- Spike A is done (gate met, PR #12 merged as `6f6bf83`). `loom-extractor extract` writes the stat-description files as JSON: `stat_descriptions/<file>.json` with `descriptions[]` (`stats`, `variants[]` with `conditions`, `flags`, `handlers`, `placeholders`, `text`), `includes`, `no_description`. PoE1 build 26530219: 44 files, 17,739 descriptions, 26,794 English variants, 0 unparsed lines. Reading notes: Spike A context file (`dev/completed/spike-a-bundles2-extraction/`), "For Spike B".
- Before B1/B2a, `loom-parser` and `loom-data` were empty core crates (wasm32; their `clippy.toml` bans fs and env, **also in tests**, since clippy runs with `--all-targets`). `tools/parity-cli` is an empty skeleton (depends on `loom-domain`, `loom-serialization`).
- Corpus: `D:\Documents\Path of Building\Builds` holds **21** PoB XML files (maintainer, 2026-11-09: 21 is correct; ADR-007 and the ACTION-PLAN say 22, corrected in B4b). All are `targetVersion="3_0"`. Survey on 2026-11-09 (throwaway scripts, not committed):
  - 1,055 items: 561 rare, 257 magic, 8 normal, 222 unique, 7 relic. About 3,740 non-unique implicit and explicit lines, so the 2% budget is about 75 lines.
  - Line prefixes: `{variant}` 1,611, `{range}` 1,511, `{tags}` 831, `{crafted}` 746, `{custom}` 54, `{eater}` 42, `{exarch}` 38, `{fractured}` 31, `{mutated}` 11. Legacy `<ModRange id range>` child elements: 5,431 (plan review count). `Catalyst` lines: 123. `Has Alt Variant`: 20.
  - **Lookup-handler lines in non-unique items: 117** (cluster `Added Small Passive Skills grant` 56, anoint `Allocates ...` 35, `Socketed Gems are Supported by` 26). Without lookup handlers the gate cannot be met. (B2b correction: the 56 cluster lines are ordinary descriptions, no lookup; 61 corpus lines need one.)
  - All 257 magic items have only one name line (affixes plus base), no separate base line.

## Target state
- `loom-data`: a stat-description model (descriptions, variants, conditions, handlers, placeholders, include chains) loaded from the extractor's JSON bytes, plus a forward renderer `(stats, values) -> lines`.
- `loom-parser`: a resolver `line(s) -> Resolution` that reverse-matches the model and returns stat IDs with values or value intervals, or an `Unresolved` with a category. Pure functions over `&str` and the model; wasm32; no fs.
- `loom-parser::pob_item`: reads one PoB item into typed lines (rarity, name and base, `Implicits: n`, prefixes, variants, ranges, flag lines). Import parsing per ADR-015.
- `tools/parity-cli` subcommands, maintainer-run, the only code that reads disk: `load-statdesc <extract>`, `round-trip <extract>`, `render-check <extract> <pob-clone>`, `resolve-corpus <builds-dir> <extract> --out <dir>` (`report.json`, `summary.json`, exit 1 while the gate is not met).
- Spike report `docs/spikes/spike-b-resolution.md`; committed summaries in `docs/spikes/spike-b/`.

## Design constraints and decisions
- **Crate ownership (resolves the handoff question):** the resolver is in `loom-parser`. AGENTS.md gives it "text -> stat-ID resolution, import parsing"; ADR-015 calls text an import adapter. It must be core because Phase 4 resolves pasted item text in the app. The model is in `loom-data` (canonical game data; breakdowns and item display render text from it too). `pob_item` is in `loom-parser`, not `loom-serialization`: AGENTS.md lets `loom-serialization` depend only on `loom-domain`, and reading item text needs the model and resolver. The XML layer of the PoB import stays Phase 2 work in `loom-serialization`.
- **Disk access only in `tools/parity-cli`** (ADR-016, `clippy.toml`). Core crates take `&[u8]`/`&str`. Every extract-backed or corpus-backed check is a `parity-cli` subcommand (the Spike A pattern: install tests lived in `tools/extractor`). `parity-cli`'s description widens to "oracle and corpus harnesses"; it gains `loom-data`, `loom-parser`, `serde_json`, and an XML reader (candidate `roxmltree`, MIT/Apache-2.0, through `cargo deny`).
- **Model status:** spike-level. Phase 2 designs the canonical schema and bundle format and may replace it. The loader refuses an unknown manifest format.
- **PoB use (ADR-005, ADR-006):** PoB data and code are used only as a cross-check and as cited evidence, never as an input to resolution. Every snippet is cited at the pinned tag, `v2.67.2:<path>` (read with `git show v2.67.2:<path>` in the legacy repo, read only, or a clone in `local/pob/`). The legacy working tree is past the tag (`v2.67.2-122-g3dbca4463`) and is not cited.
- **Include-chain precedence (resolves the handoff question, as a hypothesis):** a file's own descriptions win over its includes; nearer includes win over farther ones. This is **PoB-observed** (`v2.67.2:src/Modules/StatDescriber.lua` `getScope`: `t_insert(scope.scopeList, 1, scope)`, first scope with the stat wins), not game evidence. B2 reports how many corpus lines depend on it (their stat set is described in more than one file of the chain). A non-zero number is divergence-register material once Spike C defines the format.
- **Scope by mod domain:** the stat-description file for an item line is chosen from game data, `BaseItemTypes.ModDomain` of the item's base: tincture and graft domains use `tincture_stat_descriptions` and `graft_stat_descriptions`, every other domain uses `stat_descriptions`. This is a per-game data table in the resolver's input, with its evidence in the report (PoE2 gets its own table). `v2.67.2:src/Export/Scripts/mods.lua` loads tincture and graft files for all mods and splits output by `mod.Domain`, which is consistent. Lines that resolve only in another file get `data-gap/other-scope` with the file named. Every file is never searched blindly.
- **Normalization:** markup is stripped from template text before indexing (`<tag>{...}`, `[Ref|Text]`, `<<Icon>>`, `{{ }}`; Spike A report section 12 leaves this to Spike B). Numbers in a line become value tokens; literal digits in template text (1,836 of 15,773 non-`gem_quality` PoE1 `stat_descriptions` variants, e.g. `Level 20`) are tokenized the same way and checked as literals in verification. Range text `(a-b)` is one value token. Range text that crosses zero (`(-10-10)% increased`) is tried with both sign variants. Variants flagged `gem_quality` (PoE2 also `table_only`) stay out of the item index (PoB `matchLimit` skips them).
- **Index and candidates:** keyed by normalized template, valued by (description, variant). Candidates are deduplicated by description, so increased/reduced pairs of one description are not ambiguity (451 templates are shared by 950 variants in `stat_descriptions` alone). Two-line variants (463 texts with `\n` in `stat_descriptions`) match consecutive lines.
- **Numeric handlers:** inverted before the condition check (conditions apply to raw stat values). Lossy handlers give an interval; rounding follows the handler's own semantics, with PoB's `applySpecial` (`v2.67.2:src/Modules/StatDescriber.lua`) as cited reference for non-obvious cases (`divide_by_ten_1dp`, `per_minute_to_per_second`, `divide_by_one_hundred_and_negate` round to 1 dp). The handler list comes from the extract manifest (`stat_descriptions.handlers`); an unknown handler is `parser-gap/unsupported-handler`, never ignored.
- **Lookup handlers** render a key into another table's name. Inverse = name to key, which needs the table.

  | Handler | Corpus use | Needs (vendored schema, checked in B1) | Extracted? |
  |---|---|---|---|
  | `tree_expansion_jewel_passive` | none (B2b: the cluster `Added Small Passive Skills grant: ...` lines B0 counted here, 56, are ordinary descriptions of `local_affliction_jewel_small_nodes_grant_*` stats without a lookup) | `PassiveTreeExpansionSkills` (`PassiveSkillsKey`, `PassiveTreeExpansionJewelSizesKey`), `PassiveTreeExpansionJewels`, `PassiveTreeExpansionJewelSizes` | yes (B1x); not supported (B2b: no key form known) |
  | `passive_hash` | anoint `Allocates <notable>` (35) | `PassiveSkills.PassiveSkillGraphId` -> `Name` (B2b: hypothesis; PoB's export prints the value as an unsigned 16-bit number) | yes |
  | `display_indexable_support` | `Socketed Gems are Supported by ...` (26, with the next row) | `IndexableSupportGems` (`Index`, `Name`) (B2b: key = `Index`; Mods range `1-172` = row count refutes a 0-based row index) | yes (B1x) |
  | `display_indexable_non_active_support` (PoE1) | same | `IndexableNonActiveSupportGems` (`Index`, `Name`) (B2b: `1-164`, same evidence) | yes (B1x) |
  | `display_indexable_skill` (PoE1) | `+n to Level of all <skill> Gems` | `IndexableSkillGems` (`Index`, `Name1`, `Name2`) (B2b: key = `Index`, `1-287`; writes `Name1`, a hypothesis) | yes (B1x) |
  | `mod_value_to_item_class` | none found | `ItemClasses`? (B2b: refuted for row order: Mods values amulets 0, belts 15 are rows 4 and 21; not supported) | yes |
  | `affliction_reward_type` | none found | `AfflictionRewardTypeVisuals` (`AfflictionRewardTypes` enum, `Name`) | no |
  | `weapon_tree_unique_base_type_name` (PoE1) | none found | `WeaponPassiveTreeUniqueBaseTypes` -> `BaseItemTypes.Name` (hypothesis) | no |
  | `tempest_mod_text` | none found | no table found in the schema | - |
  | `mages_legacy_index` (PoE2) | n/a (PoE1 gate) | `UniqueMagesLegacy` (`DisplayText`) | no |
  | `passive_keystone_index` (PoE2) | n/a | `PassiveKeystoneList` (`DisplayText`) | no |
  | `specific_skill` (PoE2) | n/a | `SkillGemsForUniqueStat` (`Index`, `SkillGems`) (hypothesis) | no |
  | `ultimatum_wager_type_hash` (PoE2) | n/a | `UltimatumWagerTypes` (`HASH16`, `DisplayText`) | no |

  B1x extracted the six corpus-relevant tables (PoE2 has no file for `IndexableNonActiveSupportGems` or `IndexableSkillGems`, so they are PoE1 roots only; in PoE2 build 26476662 the other four hold no rows except `PassiveTreeExpansionJewelSizes`). The rows marked "no" are left out: no corpus line uses them; a later extractor PR adds one when a check needs it. A lookup handler whose table is missing gives `parser-gap/unsupported-handler`, not a data gap.
- **Values:** a ranged line (`{range:r}` or `<ModRange>`, text `(a-b)`) resolves to the interval `[a, b]`, with `r` and any catalyst quality recorded as data. Spike B does **not** compute the rolled value: `v2.67.2:src/Modules/ItemTools.lua` `itemLib.applyRange` adds per-range ranges, the antonym rewrite of negative increased to reduced, `data.modScalability` precision, and catalyst scaling (`Item.lua` `getCatalystScalar`). Reproducing it is Phase 2 import work. The gate counts stat IDs; `mod-backed` compares intervals.
- **Base of an item:** rare, normal, and unique items have a base line. Magic items do not: the base is the longest `BaseItemTypes.Name` contained in the name line, after removing `Superior ` and `Synthesised `. No match or a tie gives `special-case/unknown-base` for all the item's lines (unresolved, never excluded).
- **Ambiguity (B4a, defined after B3 measures the real ambiguous set):** a candidate is kept if some `Mods` row has the item's domain, a generation type that fits the line's section (implicit, explicit, enchant), and a stat set equal to the candidate's. Spawn tags are only a tie-breaker, and their use is reported (bench crafts, essences, influence, eldritch, veiled, and corruption mods do not spawn through the base's tags). A line that stays ambiguous is unresolved, `parser-gap/ambiguous`, with candidates listed. Any extra table (crafting bench, essences) comes through an extractor PR first.
- **No silent drops (ADR-015):** every line read from an item ends in exactly one of: resolved, excluded (named rule, counted), or unresolved (category). The report holds all three.
- **Untrusted input:** no panics on malformed builds or extract JSON (property tests on the item reader and the resolver); `indexing_slicing` lint in `loom-parser` and `loom-data`.
- **Determinism:** `BTreeMap`/sorted output; two harness runs byte-identical.
- **Scrubbing (ADR-007):** builds are keyed by a hash of the file content, never the file name. Item names and `{custom}` line text are never written to committed files (counts, or a hash of the text). Mod lines from game text are not personal data. Nothing else from the corpus is committed. Unit tests use synthetic lines and descriptions.
- **PoE2:** the gate is PoE1 only, but the model and resolver carry no PoE1 assumptions (ADR-001). The round-trip check also runs on the PoE2 extract.

## Steps

### 1. Stat-description model and renderer in `loom-data` (M)
- Types for files, descriptions, variants, conditions (`range`, `!`), flags, numeric and lookup handlers (`Handler::Unknown(name)` kept and counted), placeholders (index and every format found in A5), include chain; serde loader from bytes; `Scope` with the precedence above; markup stripping.
- Forward renderer `(stat values) -> lines`, with lookup handlers through an injected lookup trait (tables come from the host).
- **Accept:** unit tests over synthetic descriptions per construct (conditions, each numeric handler, placeholder formats, markup, multi-line, include precedence); `parity-cli load-statdesc` run by the maintainer on PoE1 and PoE2 with counts (descriptions, variants, handlers by kind, unknown handlers); wasm32 build.
- **Depends on:** Spike A output.

### 1x. Extractor roots for lookup tables (S-M)
- **Done in B1x** (context file, "PR B1x"). Tables confirmed in B1 (table above). Add the corpus-relevant ones as `extract` roots: `PassiveTreeExpansionSkills`, `PassiveTreeExpansionJewels`, `PassiveTreeExpansionJewelSizes`, `IndexableSupportGems`, `IndexableNonActiveSupportGems`, `IndexableSkillGems` (others optional, recorded if left out); determinism re-run on both games; manifests replaced in `docs/spikes/spike-a/`.
- **Accept:** extractor unit tests unchanged or extended; determinism holds; the tables are `ok` in the manifest.
- **Depends on:** nothing in Spike B; must merge before B2b.

### 2a. Reverse matcher, single-line and numeric handlers (`loom-parser`) (M)
- Template index, normalization, value tokens, numeric handler inversion, conditions, fill of stats without a placeholder (a value satisfying their condition, `0` when allowed), candidates deduplicated by description.
- **Accept:** unit tests per construct and per numeric handler inverse; property test: resolve never panics on arbitrary text.
- **Depends on:** 1.

### 2b. Multi-line, lookup handlers, round-trip and render checks (M-L)
- **Done in B2b** (context file, "PR B2b"; maintainer kept it one PR, 2026-11-09). Multi-line variants have up to 8 lines (PoE1), so matching takes any number of lines, not two.
- Two-line variants; lookup-handler inversion through the lookup trait.
- `parity-cli round-trip <extract>`: for every non-`gem_quality` variant, render with values inside its conditions (both ends and a middle value of each range, deterministic), resolve, and require the same description and values (or an interval containing them). Failures listed by category (`ambiguous-template`, `lossy-handler`, `unsupported-handler`, other with a reason).
- `parity-cli render-check <extract> <pob-clone>`: an **independent** check, because the round trip tests the renderer against itself. Render each `Mods` row's min and max with the B1 renderer and compare with PoB's exported mod text for the same `Id` (`Data/ModItem.lua` and siblings in a `local/pob/` clone; cross-check only). Differences categorized like A6a (`patch-delta`, `pob-rounding`, `pob-filtered`, `uncategorized`).
- Report how many corpus-relevant stat sets depend on include precedence.
- **Accept:** unit tests; both checks run by the maintainer on PoE1 (round-trip also PoE2) with counts and every failure categorized.
- **Depends on:** 2a, 1x.

### 3. PoB item reader and corpus harness (M)
- `loom-parser::pob_item` per `v2.67.2:src/Classes/Item.lua` `ParseRaw` (cited by snippet): rarity, name and base lines (magic base rule above), `Implicits: n`, prefixes (`{range}`, `{variant}`, `{tags}`, `{crafted}`, `{fractured}`, `{custom}`, `{eater}`, `{exarch}`, `{mutated}`, `{enchant}`; any other `{...}` counted and listed), `Selected Variant`, `Has Alt Variant`, `Selected Alt Variant` (and `Two`, `Three`), catalyst lines, flag lines by exact shape (`Corrupted`, `Mirrored`, `Split`, `Fractured Item`, `Synthesised Item`, `Unidentified`, `Veiled Prefix`, `Veiled Suffix`). Lines of a non-selected variant are excluded with the rule `not-selected-variant`.
- `parity-cli resolve-corpus`: reads the builds (all text nodes of each `<Item>`, since `<ModRange>` children make mixed content; `ModRange` values per `v2.67.2:src/Classes/ItemsTab.lua`), every `<Item>` in the build (each occurrence counts; distinct lines reported too), resolves each line in its scope, writes `report.json` and `summary.json`.
- Before the run: record the PoE1 Steam build; re-extract if it changed; record the extract manifest SHA-256 and `tool_dirty: false`. Copy summaries into `docs/` only after all runs (Spike A pitfall).
- Ambiguity is not filtered yet and categories are provisional; this PR records the first rate and the measured ambiguous set.
- **Accept:** unit tests on synthetic item texts per construct (unique with variants and alt variants, rare with crafted and fractured lines, magic item base match and tie, `ModRange` mixed content); property test: the reader never panics; corpus summary in the PR; two runs byte-identical; `cargo deny` passes with the XML dependency.
- **Depends on:** 2b.

### 4a. Disambiguation and `mod-backed` (M)
- The Mods filter defined above, from the B3 ambiguous set. `mod-backed` uses the same definition: a resolved non-unique line is mod-backed if some `Mods` row with the item's domain, a fitting generation type, an equal stat set, and an overlapping value interval exists. Non-backed resolved lines are listed with a reason.
- **Accept:** unit tests on synthetic Mods rows; corpus run with ambiguous and mod-backed counts.
- **Depends on:** 3.

### 4b. Categories, gate, report (M)
- Fixed category list, first match wins. **Each category needs positive evidence; a line without it is `uncategorized`.**
  - `unique`: item rarity unique or relic. Not in the gate denominator. Its unresolved lines also carry a second-level category from this list (for the per-area fallback decision).
  - `special-case/pob-custom`: the line has PoB's `{custom}` prefix (user-written). Excluded, counted, text not committed.
  - `special-case/not-a-modifier`: the line equals one of the flag-line shapes listed in step 3 (rule names the `Item.lua` snippet). Excluded, counted. No catch-all.
  - `special-case/unknown-base`: magic base rule found no single base.
  - `data-gap/other-scope`: resolves only in another stat-description file; file named.
  - `data-gap/legacy-text`: the line's text (numbers normalized) exists in PoB v2.67.2's mod text data but not in the current descriptions (league drift; PoB data used as cross-check only).
  - `data-gap/no-description`: no variant in any extracted file has literal words that cover the line's word sequence (numbers removed). Evidence is that search, recorded.
  - `parser-gap/ambiguous`: candidates remain after the filter; listed.
  - `parser-gap/near-miss`: a variant with the same literal words exists but values or conditions fail; variant named.
  - `parser-gap/unsupported-handler`: the matching variant uses a handler the resolver cannot invert, or a lookup table is missing.
  - `uncategorized`: anything else. Must be 0.
- Any exclusion rule beyond `not-selected-variant`, `special-case/pob-custom`, `special-case/not-a-modifier` needs maintainer approval in the PR.
- Spike report, gate verdict, ACTION-PLAN gate status and corpus count (21), handoff.
- **Accept:** unit test per category rule; summary committed; every unresolved line categorized; gate verdict.
- **Depends on:** 4a.

## Gate (objective, from the ACTION-PLAN)
Computed by `resolve-corpus` from `summary.json`:
1. **Resolution:** resolved / (non-unique implicit + explicit lines) >= 98%, where non-unique = rarity Normal, Magic, Rare; every line occurrence in every item of the 21 builds counts. Excluded lines (`not-selected-variant`, `special-case/pob-custom`, `special-case/not-a-modifier`) are counted separately, and the summary states the excluded share. Ambiguous and `unknown-base` lines count as unresolved. The distinct-line rate is reported next to the occurrence rate.
2. **Categorized:** `uncategorized = 0` over all lines, unique included, with positive evidence per category.
3. **Determinism:** two runs give byte-identical `report.json` and `summary.json`.
4. **Recorded inputs:** the summary records the extract manifest SHA-256 and Steam build, one content hash per build, the tool commit and dirty flag.

**Correctness (maintainer decision 2026-11-09: reported, not gated):** the `mod-backed` share of resolved non-unique lines is reported, and every non-backed line is listed with a reason. If it is below 100%, the verdict states the rate as "stat-ID resolved, not verified correct".

## Fallback
ADR-015: hybrid model. Areas whose lines fail (per category) get curated text patterns, decided per area by an ADR-015 amendment that the maintainer approves. The categorized report is the input for that decision.

## Risks and mitigations
| Risk | Mitigation |
|---|---|
| Lookup-handler lines (117 in non-unique items) exceed the 2% budget | Lookup-handler design, B1x extractor roots, B2b round trip on them. |
| Residual categories mask parser bugs | Positive evidence per category, otherwise `uncategorized`. |
| Renderer and resolver share a wrong handler semantic | Independent `render-check` against PoB v2.67.2 mod text. |
| Old-league text in the builds no longer matches current descriptions | `data-gap/legacy-text` with PoB evidence; near-miss separates drift from parser bugs; the maintainer can re-import builds. |
| Same text maps to several stats (local vs global) | Mods filter after measuring; leftovers count as unresolved. |
| A high rate with wrong matches | `mod-backed` with listed exceptions; verdict wording. |
| Magic-item base not found, so wrong scope or filter | Longest-name rule with its own unresolved category. |
| Catalyst-scaled or antonym-rewritten values misreported | Interval values; exact `applyRange` deferred to Phase 2. |
| Lossy handlers make values non-invertible | Intervals; the round trip measures it on every description. |
| PoB item text constructs the survey missed | Unknown `{...}` prefixes and unknown line shapes are counted and listed. |
| Personal data leaks from the corpus | Build hashes, no item names, no custom text in committed files. |
| Legacy checkout drifts from the pinned oracle | Cite `v2.67.2:` paths only. |
| Include precedence copied from PoB is wrong for the game | Hypothesis; dependent lines counted and reported. |
| The `loom-data` model hardens before Phase 2 | Marked spike-level in its module docs. |

## Planned PR breakdown
Each PR is one session and updates the handoff and this task folder (`/dev-docs-update`).

| PR | Content | Evidence |
|---|---|---|
| B0 | This plan; handoff update (A6b merge, Spike A docs to `dev/completed/`). | Tests: None (docs). Parity: None. Coverage: none. |
| B1 | Step 1: model, loader, renderer in `loom-data`; `parity-cli load-statdesc`. | Unit tests per construct; load counts on PoE1/PoE2; wasm32 build. |
| B1x | Step 1x: extractor roots for lookup tables. | Determinism on both games; manifests replaced. |
| B2a | Step 2a: single-line matcher, numeric handlers. | Unit and property tests. |
| B2b | Step 2b: multi-line, lookup handlers, `round-trip`, `render-check`. | Round-trip and render-check counts, failures categorized. |
| B3 | Step 3: PoB item reader, `resolve-corpus`, first rate. | Unit and property tests; corpus summary; determinism; `cargo deny`. |
| B4a | Step 4a: disambiguation, `mod-backed`. | Unit tests; ambiguous and mod-backed counts. |
| B4b | Step 4b: categories, gate verdict, spike report, ACTION-PLAN status. | Unit tests per rule; committed summary; gate pass/fail. |

Reviewed by `plan-reviewer` on 2026-11-09; all blockers, should-fix items, and nits are applied above (summary in the context file).
