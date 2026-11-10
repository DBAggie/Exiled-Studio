# Spike B: text to stat-ID resolution (report)

Status: **gate not met** (2026-11-10, PR B4b). Every line is categorized; 95.1% of non-unique lines resolve, against 98% asked. The fallback decision (ADR-015, per area) is the maintainer's; section 6 gives the areas and the arithmetic.

Plan, decisions, and per-PR details: `dev/active/spike-b-text-stat-resolution/` (plan, context, tasks). Committed summary: `docs/spikes/spike-b/resolve-corpus-poe1-26530219.json`.

## 1. Question and gate
Can item modifier text be resolved to `(stat_id, values)` by reverse-matching the game's stat-description files (ADR-015), without PoB's text patterns? Gate (ACTION-PLAN, Spike B; plan "Gate"):
1. resolved / non-unique implicit and explicit lines >= 98%, every line occurrence of the 21 builds counted, excluded lines counted apart;
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
4. Ambiguous lines go through the `Mods` filter (B4a, B4y): domain and generation type per section, stat set, base implicits, spawn tags, values; then `unset-fit` and `numbered-stat` (section 4).
5. Every line not resolved gets one category with its evidence (`tools/parity-cli/src/categorize.rs`, section 5).

## 4. Anoints (plan step 4d)
The 35 non-unique (and 7 unique) `Allocates <notable>` lines matched six descriptions: `mod_granted_passive_hash`, `_2` .. `_5`, and `local_weapon_passive_tree_granted_passive_hash`. No row backs any of them: the five `GrantedPassive*` rows carry the hash at (0, 0), set when the anoint is applied. No game table says which of the five an anoint uses (B4c survey: `BlightCraftingResults` names only the passive). Two filter steps, approved by the maintainer on 2026-11-10:
- `unset-fit`: when no candidate is backed, keep the candidates that have a fitting row carrying their stat at (0, 0). The weapon-tree stat's rows are `WEAPON_TREE`, which do not fit the enchant section, so it is dropped.
- `numbered-stat`: candidates whose stats are equal up to a numbered suffix (`_2` .. `_9`) are one reading; the lowest numbers are kept, the others listed as dropped alternatives. PoB does not pick a stat either (`allocates (.+)` gives one `GrantedPassive` list mod, `v2.67.2:src/Modules/ModParser.lua`).

Result: all 42 lines resolve (`resolved/unset-fit+numbered-stat`). Their `mod-backed` reason is `row-value-unset`, by design.

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
**Rate:** non-unique **3,248 / 3,414 = 95.1%** (distinct 92.4%); 166 unresolved, the gate allows 68 (2%). Excluded 524 lines (13.3% of non-unique lines; table). `uncategorized`: **0** (unique included). Steps: B3 79.1%, B4a 85.4%, B4y 86.6%, B4c 88.0%, anoints 88.9% (3,248 / 3,652), B4b 95.1% (author notes out of the denominator).

| Non-unique lines | Lines |
|---|---:|
| resolved | 3,248 |
| `parser-gap/ambiguous` | 72 |
| `data-gap/legacy-text` | 48 |
| `special-case/item-property` | 15 |
| `parser-gap/client-string` | 15 |
| `parser-gap/near-miss` | 11 |
| `parser-gap/split-section` | 4 |
| `parser-gap/only-shadowed` | 1 |
| excluded `special-case/author-note` | 238 |
| excluded `special-case/not-a-modifier` | 230 |
| excluded `special-case/pob-custom` | 54 |
| excluded `not-selected-variant` | 2 |

Unique lines (not in the rate): 818 resolved; ambiguous 36, legacy-text 29, near-miss 32 (old unique wording: `Passives in radius are Conquered by the Vaal`, `Gain Adrenaline for 3 seconds on kill`), item-property 8, author-note 15, not-a-modifier 36, not-selected-variant 1,415.

**Unresolved non-unique lines by area** (the fallback input):
1. **Local vs global ambiguity, 72.** 55 are backed on both sides and spawn tags are skipped because a legacy essence-only row (`ChanceToDodgeEssence4-7`, `IncreasedAttackSpeedEssenceJewellery4-7`, ...) that no `Essences` column names backs one side; 14 are `{crafted}` lines backed by legacy bench rows without class restrictions (bench crafts do not spawn); 2 several-backed; 1 influence guard. The game data cannot tell them apart. Options: a curated rule per stat pair (ADR-015 fallback), or reading the item's other context.
2. **Legacy text, 48.** Pre-3.15 flask mods (`25% reduced Charges used`, `Immune to Curses during Flask effect`, 40 lines from `v2.4.0`), 3.16-3.20 `reduced Effect of Curses on you during Flask Effect` (4), and single old texts. The game no longer has these texts. Options: re-import the builds, or curated legacy patterns (ADR-015 quarantine).
3. **Client-composed lines, 15.** Eldritch `While a Unique Enemy is in your Presence, ...` (9) and graft `Uses level 30 <skill>` (6). Resolvable from game data (`ClientStrings` plus `local_influence_mod_requires_unique_monster_presence`, and the graft mod's granted skill); a resolver feature, not a fallback.
4. **Item properties, 15.** `Lasts 6 Seconds`, `Currently has 40 Charges`, `Quality (Attribute Modifiers): +20%`, `Memory Strands: 63`: not modifiers. Excluding them is a new exclusion rule and needs the maintainer.
5. **Author edits and old wording, 11 near-miss** (`+132 to Maximum Life`, `recover 5% of energy shield when you block`), **4 split-section** (cluster jewels whose `Implicits:` count is one short), **1 only-shadowed** (`24% increased effect of Offerings`, the B2b `patch-delta` text).

**Arithmetic** (allowance = 2% of the denominator, rounded down):
- areas 3 and 4 only (+15 resolved, −15 from the denominator): 136 unresolved of 3,399, allowance 67: not met;
- areas 2, 3, and 4 (legacy text excluded too): 88 of 3,351, allowance 67: not met;
- area 1 with areas 3 and 4: 64 of 3,399, allowance 67: met;
- area 1 with area 2: 46 of 3,366, allowance 67: met.

Every combination that meets the gate needs a decision on area 1.

**`mod-backed`** (reported, not gated): 3,063 / 3,248 = 94.3%; resolver-only lines 2,797 / 2,940 = 95.1%. Not backed: `row-value-unset` 126 (anoints 35, cluster jewel nodes 51, others 40; the value is set outside the row by design), `values-outside` 48, `no-fitting-row` 6, `no-row-with-stats` 5. So the resolved lines are "stat-ID resolved, not verified correct" for the 59 lines outside `row-value-unset`.

**Determinism and inputs:** two runs from clean commit `39bd52d` (code `cc98f59`) gave byte-identical `summary.json` and `report.json`; the summary records the manifest SHA-256, the Steam build, 21 build hashes, the tool commit with `dirty: false`, and the six PoB sources.

## 7. Verdict
Gate items 2, 3, and 4 hold. Item 1 does not: 95.1% < 98%. Spike B's conclusion for ADR-015: reverse-matching game data resolves the corpus's current game text; what it cannot resolve is (1) local/global pairs whose game rows cannot be told apart, (2) text the game no longer has, and (3) a few client-composed lines that game data can still cover. Areas 1 and 2 are the fallback candidates. The next step is the maintainer's decision (handoff, Next action).

## 8. Reproduce
```
git -C "D:\POE Dev\PathOfBuilding" archive <tag> src/Data | tar -x -C local/pob-<tag>   # each of the six tags
cargo run --release -p loom-parity-cli -- resolve-corpus "D:\Documents\Path of Building\Builds" local/determinism/poe1/run1 --out <dir> \
  --pob v2.4.0=local/pob-v2.4.0 --pob v2.21.1=local/pob-v2.21.1 --pob v2.35.3=local/pob-v2.35.3 \
  --pob v2.48.0=local/pob-v2.48.0 --pob v2.57.0=local/pob-v2.57.0 --pob v2.67.2=local/pob-v2.67.2
```
Exit 1 while the gate is not met. Unit tests: `cargo test -p loom-parity-cli` (one test per category rule in `categorize/tests.rs`; the anoint steps in `mod_filter/tests.rs`; corpus-level categories, class requirement, split-section, and gate inputs in `resolve_corpus/tests.rs`).

## 9. Divergence-register candidates (Spike C defines the format)
The B2b `patch-delta` texts (4 descriptions), `rendered-double-sign`, and the anoint choice (`numbered-stat` picks `mod_granted_passive_hash`; PoB keeps no stat).
