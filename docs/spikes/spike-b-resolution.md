# Spike B: text to stat-ID resolution (report)

Status: **gate met** (2026-11-10, PR #27 (B5b), once merged; PR #28 (B5c) and PR #30 (B5g) raise the rate). 98.9% of non-unique lines resolve after B5g (98.5% with B5b) (98% asked) and every line is categorized. The maintainer decided the areas on 2026-11-10 (section 7a): B5a (PR #26) settled area 1 (local/global) from game data (section 4a); B5b excluded legacy text from the rate (section 6) and corrected ADR-007's count. The maintainer also asked for areas 3 to 5 (client-composed lines, item properties, the rest); those PRs follow and raise the rate further.

Plan, decisions, and per-PR details: `dev/active/spike-b-text-stat-resolution/` (plan, context, tasks). Committed summary: `docs/spikes/spike-b/resolve-corpus-poe1-26530219.json`.

## 1. Question and gate
Can item modifier text be resolved to `(stat_id, values)` by reverse-matching the game's stat-description files (ADR-015), without PoB's text patterns? Gate (ACTION-PLAN, Spike B; plan "Gate"):
1. resolved / non-unique implicit and explicit lines >= 98%, every line occurrence of the 21 builds counted, excluded lines counted apart (excluded by maintainer decision: PoB custom lines, flag and class-requirement lines, unselected variants, author notes, legacy text);
2. every unresolved line categorized with positive evidence, `uncategorized = 0`, uniques included;
3. two runs byte-identical;
4. inputs recorded (extract manifest, Steam build, build hashes, tool commit and clean tree).

## 2. Inputs
- Corpus: 21 PoB builds (`D:\Documents\Path of Building\Builds`, the maintainer's; ADR-007 and the ACTION-PLAN said 22, the maintainer confirmed 21 on 2026-11-09), 1,055 items (561 rare, 257 magic, 8 normal, 222 unique, 7 relic). Builds are named by content hash only.
- Extract: PoE1 Steam build 26530219, `local/determinism/poe1/run1` (tool commit `674837f`, manifest SHA-256 in the summary).
- PoB text, legacy-text evidence only (ADR-005): `git archive <tag> src/Data` of the legacy repository (read only) at six tags, one per year of PoB data, from the oldest era whose text occurs in the corpus to the pinned oracle: `v2.4.0` (2021-07, `82bc211b`, the last data before 3.15's flask rework), `v2.21.1` (2022-08, `6b901f65`), `v2.35.3` (2023-12, `0524c2bd`), `v2.48.0` (2024-08, `d7402e21`), `v2.57.0` (2025-10, `d2a84ab3`), `v2.67.2` (pinned, `b32759ab`). The summary records each source's files and a SHA-256. Measured: going from two tags (`v2.4.0`, `v2.67.2`) to six moved 6 lines (2 texts) from `author-note` to `legacy-text` and nothing else.

## 3. Method
1. `loom-parser::pob_item` reads each item as PoB's `ParseRaw` does (B3); flag, influence, and class-requirement lines are not modifiers.
2. Scope by `BaseItemTypes.ModDomain` (tincture and graft files for their domains, `stat_descriptions` otherwise).
3. `loom-parser::resolve` reverse-matches each run of lines of one section against the scope's templates, with numeric handler inversion, conditions, multi-line templates, and lookup handlers (B2a, B2b, B4c).
4. Ambiguous lines go through the `Mods` filter (B4a, B4y, B5a): domain and generation type per section, stat set, base implicits, local stats (section 4a), spawn tags, values, spawning rows with the value; then `unset-fit` and `numbered-stat` (section 4).
5. Every line not resolved gets one category with its evidence (`tools/parity-cli/src/categorize.rs`, section 5).

## 4. Anoints (plan step 4d)
The 35 non-unique (and 7 unique) `Allocates <notable>` lines matched six descriptions: `mod_granted_passive_hash`, `_2` .. `_5`, and `local_weapon_passive_tree_granted_passive_hash`. No row backs any of them: the five `GrantedPassive*` rows carry the hash at (0, 0), set when the anoint is applied. No game table says which of the five an anoint uses (B4c survey: `BlightCraftingResults` names only the passive). Two filter steps, approved by the maintainer on 2026-11-10:
- `unset-fit`: when no candidate is backed, keep the candidates that have a fitting row carrying their stat at (0, 0). The weapon-tree stat's rows are `WEAPON_TREE`, which do not fit the enchant section, so it is dropped.
- `numbered-stat`: candidates whose stats are equal up to a numbered suffix (`_2` .. `_9`) are one reading; the lowest numbers are kept, the others listed as dropped alternatives. PoB does not pick a stat either (`allocates (.+)` gives one `GrantedPassive` list mod, `v2.67.2:src/Modules/ModParser.lua`).

Result: all 42 lines resolve (`resolved/unset-fit+numbered-stat`). Their `mod-backed` reason is `row-value-unset`, by design.

## 4a. Local and global pairs (B5a)
B4b left 72 non-unique lines ambiguous: 55 because a legacy essence row (essence-only, named by no `Essences` column) backed one reading and stopped `spawn-tags`, 14 `{crafted}` lines (bench crafts do not spawn), 2 several-backed, 1 influence guard. Three filter changes, all from game data (`tools/parity-cli/src/mod_filter.rs`, module docs):
- **`local-stat`** (explicit lines of non-unique items, before `spawn-tags`): `Stats.IsLocal` and `Stats.IsWeaponLocal` mark local stats. When the kept readings are a local and a global one, a base tagged `weapon` (for `IsWeaponLocal` stats) or `armour` (for the other local stats) keeps the local reading, and any other base keeps the global one. The rule never drops a local reading that a fitting row spawns on the base (note `local-stat-spawns`), so flask-local and jewel-local stats are not overruled. Spawn data agrees: on PoE1 26530219 no prefix or suffix row of the global stat of these pairs spawns on the tags where the local stat applies. The only global `% increased Armour` rows on `body_armour` are eldritch implicits, and `local-stat` does not cover implicits.
- **Legacy essence rows by `ModType`**: such a row counts as spawning where the rows of its `ModType` that are not essence-only spawn, and nowhere when those spawn nowhere. Rows count only when they share the `ModType`, domain, and generation type (review fix). 100 of the 125 legacy essence rows have such rows (52 of them spawn somewhere); the other 25 keep the `essence-tags-unknown` guard. Example: `ChanceToDodgeEssence4`..`7` share `ChanceToSuppressSpellsOld` (stat `base_spell_suppression_chance_150%_of_value`) with rows that no longer spawn, so `+12% chance to Suppress Spell Damage` on a helmet reads as the current `base_spell_suppression_chance_%`.
- **`spawn-values`** (after `values`, only when `spawn-tags` ran): keep the readings backed by one row that both spawns on the base and holds the value. On boots, the 150% stat spawns only through the Delve row `DelveArmourDodgeAndSpellDodge_` (3-4), which does not hold the value.

**Check against earlier steps:** `local-stat` now decides 107 lines that `spawn-tags` (93) or `values` (14) had already resolved. 106 get the same stat. One changes: `+52 to maximum Energy Shield` on a PoB-crafted Crystal Belt. `values` had read it as `local_energy_shield`, because 52 is inside `LocalIncreasedEnergyShield8` and outside every global row. Belts have no ES property, so the global reading is right; the line is now `mod-backed` `values-outside`. No unique line changed.

**Result:** 67 of the 72 lines resolve (`local-stat` 45, `spawn-tags` 17, `spawn-values` 5). The 5 left are real ambiguities in the data: two `{crafted}` `+1 to maximum number of Skeletons` lines (the legacy bench row `IntMasterMaximumSkeletonsCrafted` and its `Updated` row back the two stats), `0.2% of Fire Damage Leeched as Life` (an eldritch implicit or a legacy corruption row; the item has no eldritch flag), `10% increased Mana Reservation Efficiency of Skills` (an implicit with two backed stats), and `Projectiles Pierce an additional Target` on a bow (the influence guard, as in B4b).

## 4b. Client-composed eldritch lines (B5c)
Eldritch implicits with `local_influence_mod_requires_unique_monster_presence` show as `While a Unique Enemy is in your Presence, {0}` (`ClientStrings` `InfluenceStatConditionPresenceUniqueMonster`) wrapped around the line of the mod's other stat; no stat description holds the whole text. `loom-parser::resolve::compose` reads such a line through the wrapper: the whole line must have no indexed key, the remainder must resolve, and the wrapper's stat is added to every reading with value 1. The wrappers and their stats are `categorize::COMPOSITIONS` (the Pinnacle wrapper `InfluenceStatConditionPresenceCelestialBoss` with `..._celestial_boss_presence` too; 0 lines in the corpus). The pairing of id and stat is written in code and checked: the run fails if a paired stat is not in `Stats`, a wrapper id missing from `ClientStrings` is listed (`compositions_missing`), and the summary's `composition_rows` counts each stat's `Mods` rows and their eldritch share (PoE1 26530219: 1,362 of 1,362 for both stats). A line is composed only when the plain resolver left it as one `NoTemplate` line, so a multi-line template is never split. The Pinnacle wrapper has 0 lines in the corpus; its pairing rests on the row check only.

**Result:** the 9 non-unique presence lines resolve, and all 9 are `mod-backed` `equal` (each by a `*EldritchImplicitUniquePresence*` row: `CurseEffectFlammability...2`, `GlobalCooldownRecovery...4`, and so on), which confirms the pairing. No other line of the corpus changed. The 6 graft `Uses level {1} {0}` lines stayed `parser-gap/client-string` here; B5x extracted the graft tables and B5g resolves 4 of them (section 4c).

## 4c. Graft skill lines (B5g)
A graft's implicit `Uses level {1} {0}` (`ClientStrings` `BrequelGraftSkillString`) names the skill the graft grants; no stat stands for it. `tools/parity-cli/src/grafts.rs` reads it from the tables B5x extracted: the graft base's metadata id ends in a `BrequelGrafts.Id` (`TulGraft4`), whose `SkillGem` row's base item names the skill (`Preserving Stillness`); `BrequelGraftGrantedSkillLevels` gives the levels (1 to 30). A line resolves when the item's base is a graft, the level is a whole number in that range, and the name equals the graft's skill. Such a line is `resolved` with kind `granted-skill` (detail `graft=`, `skill=`, `level=`): it counts as resolved in the rate, and the summary counts these lines apart (`resolved_kinds`). It has no stat ID and is not in `mod-backed`.

**Result:** 4 of the 6 graft lines resolve (Aegis Tulgraft, Hardening Uulgraft, Molten and Flamecaller Xophgraft, all level 30). The other 2 read `Uses level (1-30) Hardening Uulgraft` and `... Impaling Uulgraft`: a level range and the base's own name in place of the skill, which is PoB's placeholder, not game text. They stay `parser-gap/client-string`. No other line changed. The summary records what the rule read (`graft_inputs`: wrapper found, levels 1 to 30, no base name whose rows disagree), so a missing wrapper or level table would show.

## 5. Categories
First match wins; each rule needs evidence named on the line (`evidence` in `report.json`, `detail` in the summary). The word key compares text with case, numbers, and punctuation removed.

| Category | Rule and evidence | Excluded |
|---|---|---|
| `special-case/author-note` | Starts with a PoB colour escape (`^1`, `^x..`; no game text holds one: `colour_code_in_game_text: false`); or a PoB `Note:` spec line; or the word key equals no line of any stat description, allowlisted `ClientStrings` text, or PoB text at the six tags | yes (maintainer, 2026-11-10) |
| `special-case/pob-custom` | `{custom}` prefix | yes (plan) |
| `special-case/not-a-modifier` | Flag and influence lines; `Requires Class` lines (class kept on the item record) | yes (plan) |
| `excluded/not-selected-variant` | Line of a variant the item does not select | yes (plan) |
| `special-case/unknown-base` | The base was not found (none in the corpus) | no |
| `parser-gap/ambiguous` | Candidates left after the filter; the filter's reason | no |
| `parser-gap/only-shadowed` | Only matches shadowed by include precedence | no |
| `parser-gap/unsupported-handler` / `near-miss` | A template matched, no variant verified; rejection reasons | no |
| `parser-gap/split-section` | The line and the adjacent line of another section resolve as one multi-line description | no |
| `special-case/item-property` | A `ParseRaw` spec line with a number as value, or one of nine named `ClientStrings` `ItemDisplay*` texts (`Lasts {0} Seconds`, ...) | no (candidate, section 6) |
| `parser-gap/client-string` | A `ClientStrings` text the client composes a modifier with (`InfluenceStatConditionPresence*`, `BrequelGraftSkillString`); the remainder is not resolved | no |
| `parser-gap/near-miss` | Word key equals a variant line of the item's scope (a lookup placeholder matches one or more words) | no |
| `data-gap/other-scope` | Word key equals a variant line of another file only | no |
| `data-gap/legacy-text` | Word key equals PoB text at one of the tags, and no current game text | no |
| `uncategorized` | Anything else (also: no PoB source given) | no |

The plan's `data-gap/no-description` (no variant covers the line's words) is the third `author-note` evidence: with PoB's text at six tags checked as well, a line that matches no source is not modifier text of the game now or in PoB's data.

**Author notes, reviewed by hand** (238 non-unique lines: 105 colour code, 2 `Note:`, 131 no game text): build advice (`Use jeweler orbs at the bench ...`), wish lists (`% increased Spell Damage`, `Cast Speed`, `Double Multi`), passive and skill names (`Golem's Blood`, `Assassin's Mark`), and a lone `Abyss`. Two borderline texts: `+51 Dexterity` (2 lines, an author's edit of `+{0} to Dexterity`; PoB's lenient parser may read it) and `Graftblood: 0/18827` (2 lines, a graft property pasted from the game; no extracted table holds `Graftblood: `). Neither is game text, so the rule's evidence holds.

## 6. Results (PoE1 26530219)
**Rate:** non-unique **3,328 / 3,366 = 98.9%** (distinct 98.0%); 38 unresolved, the gate allows 67 (2%). Stat-only (lines resolved to a description's stat IDs, without the typed `kind` lines; `rate.stat_resolved`): 3,324 / 3,366 = 98.8%, which passes as well. Excluded 572 lines (14.5% of non-unique lines; table). `uncategorized`: **0** (unique included). Steps: B3 79.1%, B4a 85.4%, B4y 86.6%, B4c 88.0%, anoints 88.9% (3,248 / 3,652), B4b 95.1% (author notes out of the denominator), B5a 97.1% (3,315 / 3,414), B5b 98.5% (legacy text out of the denominator), B5c 98.8%, B5g 98.9% (4 granted-skill lines).

| Non-unique lines | Lines |
|---|---:|
| resolved | 3,328 (4 of them granted skills) |
| `special-case/item-property` | 15 |
| `parser-gap/client-string` | 2 |
| `parser-gap/near-miss` | 11 |
| `parser-gap/ambiguous` | 5 |
| `parser-gap/split-section` | 4 |
| `parser-gap/only-shadowed` | 1 |
| excluded `data-gap/legacy-text` | 48 |
| excluded `special-case/author-note` | 238 |
| excluded `special-case/not-a-modifier` | 230 |
| excluded `special-case/pob-custom` | 54 |
| excluded `not-selected-variant` | 2 |

Unique lines (not in the rate): 818 resolved; ambiguous 36, legacy-text 29, near-miss 32 (old unique wording: `Passives in radius are Conquered by the Vaal`, `Gain Adrenaline for 3 seconds on kill`), item-property 8, author-note 15, not-a-modifier 36, not-selected-variant 1,415. B5a changed none of them.

**Unresolved non-unique lines by area** (B4b numbering; decisions in section 7a):
1. **Local vs global ambiguity: 72, now 5** (B5a, section 4a).
2. **Legacy text, 48: excluded** (B5b). Pre-3.15 flask mods (`25% reduced Charges used`, `Immune to Curses during Flask effect`), 3.16-3.20 `reduced Effect of Curses on you during Flask Effect`, and single old texts; 25 distinct texts (counted from the summary's `categorized`, non-unique entries). By PoB source: `v2.4.0` and `v2.21.1` 30 lines, `v2.4.0` only 13, `v2.21.1` only 4, all six tags 1 (`85% increased Critical Strike Chance for Spells`: PoB v2.67.2 still carries the old mod; no current stat description has the text). By the rule's construction, every one has a PoB source and no current game text.
3. **Client-composed lines: 15, now 2** (B5c section 4b, B5g section 4c). The eldritch presence lines (9) and 4 graft lines resolve; 2 graft lines carry PoB's placeholder text.
4. **Item properties, 15.** `Lasts 6 Seconds`, `Currently has 40 Charges`, `Quality (Attribute Modifiers): +20%`, `Memory Strands: 63`: not modifiers.
5. **Author edits and old wording, 11 near-miss** (`+132 to Maximum Life`, `recover 5% of energy shield when you block`), **4 split-section** (cluster jewels whose `Implicits:` count is one short), **1 only-shadowed** (`24% increased effect of Offerings`, the B2b `patch-delta` text).

**Arithmetic** (allowance = 2% of the denominator, rounded down):
- B5a: 99 unresolved of 3,414, allowance 68: not met;
- B5b (area 2 excluded, decided 2026-11-10): 51 of 3,366, allowance 67: **met**;
- B5c (eldritch presence lines): 42 of 3,366;
- B5g (graft skill lines): 38 of 3,366.

**`mod-backed`** (reported, not gated): 3,138 / 3,324 = 94.4% (B5c: the 9 presence lines are backed; the denominator is the stat-resolved lines, so the 4 granted skills are not in it); resolver-only lines 2,797 / 2,940 = 95.1%. Not backed: `row-value-unset` 126 (anoints 35, cluster jewel nodes 51, others 40; the value is set outside the row by design), `values-outside` 49 (one more than B4b: the Crystal Belt line of section 4a), `no-fitting-row` 6, `no-row-with-stats` 5. So 60 resolved lines outside `row-value-unset` are "stat-ID resolved, not verified correct".

**Determinism and inputs:** two runs from clean commit `c437709` (the following commit changes docs only) gave byte-identical `summary.json` and `report.json`. The summary records the manifest SHA-256, the Steam build, 21 build hashes, the tool commit with `dirty: false`, the six PoB sources, the `ClientStrings` wrappers read (`compositions`), and the graft bases read (`graft_bases`, 16).

## 7. Verdict
**Gate met** (B5b): item 1 98.5% >= 98% (legacy text and author notes excluded by maintainer decision); items 2, 3, and 4 hold; `resolve-corpus` exits 0. Spike B's conclusion for ADR-015: reverse-matching the game's stat descriptions, with the `Mods` filter, resolves the corpus's current game text, local/global pairs included (B5a). No curated text-pattern fallback was needed. Text the game no longer has (legacy text) is categorized and counted apart: the exclusion is a rule of the gate's rate only. Per ADR-015 (`docs/adr/015-stat-id-modifier-model.md`: unresolved lines are quarantined and shown), when the resolver ships, these lines must still surface as quarantined items, never be dropped. Without the exclusion the rate is 97.1% (3,315 / 3,414). Client-composed lines (area 3) and item properties (area 4) are resolver work from game data, which the maintainer asked for next.

**Resolver output (B5g, for the maintainer):** a resolved line is not always a description's stat IDs and values. Since B5g a line can resolve to a typed result read from game data (`kind`: `granted-skill`; B5d adds item properties). Such lines count as resolved in the rate and are shown apart (`resolved_kinds`, `rate.stat_resolved`). The data model and calcs must not assume every resolved line is a stat ID; this is proposed as an ADR-015 clarification for the maintainer to accept, not applied to the ADR.

## 7a. Maintainer decisions (2026-11-10)
On B4b's areas (section 6):
1. Local vs global: the maintainer asked what resolving it needs. B5a needed no fallback: game data settles 67 of the 72 lines (section 4a). The 5 left stay `parser-gap/ambiguous`.
2. Legacy text: **excluded from the rate**, as author notes are. Done in B5b.
3. Client-composed lines: write the resolver code. Eldritch lines in B5c; grafts in B5g after the B5x extractor PR.
4. Item properties: attempt to resolve them. Later PR.
5. The rest: resolve them as recommended. Later PR.
6. ADR-007's corpus count: corrected to 21 in B5b.

## 8. Reproduce
```
git -C "D:\POE Dev\PathOfBuilding" archive <tag> src/Data | tar -x -C local/pob-<tag>   # each of the six tags
cargo run --release -p loom-parity-cli -- resolve-corpus "D:\Documents\Path of Building\Builds" local/determinism/poe1/run1 --out <dir> \
  --pob v2.4.0=local/pob-v2.4.0 --pob v2.21.1=local/pob-v2.21.1 --pob v2.35.3=local/pob-v2.35.3 \
  --pob v2.48.0=local/pob-v2.48.0 --pob v2.57.0=local/pob-v2.57.0 --pob v2.67.2=local/pob-v2.67.2
```
Exit 1 while the gate is not met. Unit tests: `cargo test -p loom-parity-cli` (one test per category rule in `categorize/tests.rs`; the anoint steps and the B5a steps (`local-stat`, legacy essence `ModType`, `spawn-values`) in `mod_filter/tests.rs`; corpus-level categories, class requirement, split-section, and gate inputs in `resolve_corpus/tests.rs`).

## 9. Divergence-register candidates (Spike C defines the format)
The B2b `patch-delta` texts (4 descriptions), `rendered-double-sign`, and the anoint choice (`numbered-stat` picks `mod_granted_passive_hash`; PoB keeps no stat).
